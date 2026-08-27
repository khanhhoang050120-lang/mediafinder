//! The `thumb://` URI scheme.
//!
//! Thumbnails reach the page as image URLs, not as data in a search response.
//! Returning them inline would mean base64 in the JSON of every keystroke —
//! a hundred results at 35 KB each is 3.5 MB of encoded text per search, all
//! of it discarded the moment the user types another letter.
//!
//! As URLs the browser does the work instead: it fetches only what is on
//! screen, caches what it has fetched, runs several requests in parallel, and
//! cancels the rest when a row scrolls away. None of that has to be written.
//!
//! URL shape: `thumb://localhost/{epoch}_{index}?s={pixels}`
//!
//! `epoch` guards against a rescan landing mid-scroll. Entry numbers are
//! positions in the index, so after a rebuild the same number means a
//! different file — and an in-flight request would quietly paint the wrong
//! picture next to the right name. A mismatched epoch is refused instead.
//!
//! The two numbers are joined with `_`, not `/`. Tauri's `convertFileSrc`
//! percent-encodes the path it is given, so a slash arrives here as `%2F` and
//! never splits: the first attempt at this produced `/2%2F84341` and every
//! thumbnail silently 400'd. Underscore is unreserved and passes through
//! untouched.

use tauri::http::{Request, Response, StatusCode};
use tauri::{AppHandle, Manager, UriSchemeResponder};

use crate::media::thumbnail::ThumbnailService;
use crate::state::AppState;

pub const SCHEME: &str = "thumb";

/// Largest thumbnail the UI may ask for.
///
/// The size is part of the cache key, so an unbounded value would let a
/// crafted URL fill memory with one enormous entry per pixel value.
const MAX_SIZE: u32 = 512;
const DEFAULT_SIZE: u32 = 192;

/// Serve one thumbnail request.
///
/// Runs the lookup on a blocking pool: resolving a cache miss can take tens of
/// milliseconds, and this is called on the thread driving the webview.
pub fn handle(app: &AppHandle, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let response = build(&app, &request);
        responder.respond(response);
    });
}

fn build(app: &AppHandle, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri();
    let Some((epoch, index)) = parse_entry(uri.path()) else {
        tracing::warn!("thumb: không phân tích được đường dẫn {:?}", uri.path());
        return error(StatusCode::BAD_REQUEST);
    };
    let size = parse_size(uri.query());

    let state = app.state::<AppState>();
    if state.index_epoch() != epoch {
        // The index was rebuilt while this request was in flight.
        return error(StatusCode::GONE);
    }

    let snapshot = state.snapshot();
    if index >= snapshot.len() {
        return error(StatusCode::NOT_FOUND);
    }
    let path = snapshot.full_path(index);
    // Release the snapshot before the slow part so a rebuild is not held up by
    // a thumbnail decode.
    drop(snapshot);

    match app
        .state::<ThumbnailService>()
        .get(index as u64, &path, size)
    {
        Ok(png) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/png")
            // Immutable: the epoch already changes whenever the meaning of an
            // index does, so a cached response can never go stale.
            .header("Cache-Control", "public, max-age=31536000, immutable")
            .body(png.as_ref().clone())
            .unwrap_or_else(|_| error(StatusCode::INTERNAL_SERVER_ERROR)),

        // Hai câu trả lời khác nhau cho hai tình huống khác nhau: 503 nghĩa
        // là "hỏi lại đi, đĩa đang bận", 404 nghĩa là "tệp này không có
        // thumbnail, hỏi lại vô ích". Giao diện thử lại một lỗi tạm thời và
        // buông một lỗi thật.
        Err(crate::media::thumbnail::ThumbError::Busy) => error(StatusCode::SERVICE_UNAVAILABLE),
        Err(_) => error(StatusCode::NOT_FOUND),
    }
}

fn error(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        // Một câu trả lời lỗi không bao giờ được nằm lại trong cache của
        // webview: "đang bận" mà bị cache thì lần hỏi lại nhận đúng câu từ
        // chối cũ mà đĩa không hề được hỏi.
        .header("Cache-Control", "no-store")
        .body(Vec::new())
        .expect("static response builds")
}

/// Parse `/{epoch}_{index}` out of the URL path.
///
/// Shared with [`super::media_stream`]: both schemes name an entry the same
/// way, and two copies of this would drift the moment one of them changed.
pub(crate) fn parse_entry(path: &str) -> Option<(u64, usize)> {
    let mut parts = path.trim_start_matches('/').split('_');
    let epoch = parts.next()?.parse().ok()?;
    let index = parts.next()?.parse().ok()?;
    // Anything further is not a URL this scheme issues.
    if parts.next().is_some() {
        return None;
    }
    Some((epoch, index))
}

/// Parse `s=` out of the query, clamped to something sane.
fn parse_size(query: Option<&str>) -> u32 {
    query
        .and_then(|q| {
            q.split('&')
                .find_map(|pair| pair.strip_prefix("s="))
                .and_then(|v| v.parse::<u32>().ok())
        })
        .unwrap_or(DEFAULT_SIZE)
        .clamp(16, MAX_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_well_formed_path() {
        assert_eq!(parse_entry("/7_1234"), Some((7, 1234)));
        assert_eq!(parse_entry("7_0"), Some((7, 0)));
    }

    #[test]
    fn a_slash_separator_is_rejected_because_it_never_arrives_intact() {
        // Regression guard. `convertFileSrc` percent-encodes the path, so a
        // slash reaches this function as `%2F` and splits nothing. Every
        // thumbnail 400'd silently until the separator changed to `_`.
        assert_eq!(parse_entry("/2%2F84341"), None);
        assert_eq!(parse_entry("/7/1234"), None);
    }

    #[test]
    fn rejects_paths_this_scheme_never_issues() {
        assert_eq!(parse_entry("/7"), None, "thiếu chỉ số");
        assert_eq!(parse_entry("/7_12_extra"), None, "thừa đoạn");
        assert_eq!(parse_entry("/abc_12"), None, "epoch không phải số");
        assert_eq!(parse_entry("/7_-1"), None, "chỉ số âm");
        assert_eq!(parse_entry("/"), None);
    }

    #[test]
    fn size_defaults_and_clamps() {
        assert_eq!(parse_size(None), DEFAULT_SIZE);
        assert_eq!(parse_size(Some("s=128")), 128);
        assert_eq!(parse_size(Some("x=1&s=64")), 64);

        // An unbounded size would let one URL allocate an arbitrarily large
        // image, and every distinct value would take its own cache slot.
        assert_eq!(parse_size(Some("s=999999")), MAX_SIZE);
        assert_eq!(parse_size(Some("s=0")), 16);
        assert_eq!(parse_size(Some("s=notanumber")), DEFAULT_SIZE);
    }
}
