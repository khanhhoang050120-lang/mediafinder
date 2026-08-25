//! The `media://` URI scheme — playing a result without leaving the app.
//!
//! A preview needs the file's bytes in the page, and there are two ways to get
//! them there. Reading the file in Rust and handing the page a data URL would
//! mean holding an entire video in memory and base64-ing it first; a two
//! gigabyte clip becomes 2,7 GB of text. So the bytes travel as a URL instead
//! and the browser fetches what it needs, when it needs it.
//!
//! URL shape: `media://localhost/{epoch}_{index}` — the same identity
//! [`super::protocol`] uses for thumbnails, and for the same reason: `epoch`
//! refuses a request issued against an index that has since been rebuilt.
//!
//! **No path ever appears in a URL.** The page can only ask for a position in
//! the index, so it can only ever reach a file the index already holds. Serving
//! by path would mean the webview could name any file on the machine.
//!
//! # Range requests are the whole point
//!
//! Chromium's media player does not download a video and then play it — it
//! asks for byte ranges, and asks again wherever the user drags the scrubber.
//! Without `206 Partial Content` the player has to take the file whole before
//! showing a frame, and seeking stops working entirely.
//!
//! Measured on this user's NAS (`F:`, gigabit):
//!
//! ```text
//! byte đầu tiên          66 ms
//! thông lượng            84,7 MB/s  (678 Mbps)
//! nhảy tới cuối tệp      18 ms
//! ```
//!
//! Far above any video bitrate, so a NAS file plays like a local one. That
//! measurement is why this feature was worth building at all.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use tauri::http::{Request, Response, StatusCode};
use tauri::{AppHandle, Manager, UriSchemeResponder};

use crate::state::AppState;

pub const SCHEME: &str = "media";

/// Most bytes served in one response.
///
/// A player asking for `bytes=0-` means "all of it", and answering literally
/// would pull a two gigabyte file into memory. Answering with less than was
/// asked for is allowed — the response says which range it actually carries,
/// and the player comes back for the rest.
const MAX_CHUNK: u64 = 8 * 1024 * 1024;

/// Largest file served in one piece to a request with no `Range` header.
///
/// Images are fetched whole by `<img>`, which never sends `Range`. A camera
/// RAW can be a hundred megabytes and that is still fine to read once; beyond
/// this the request is refused rather than silently truncated, because a
/// half-read image is a corrupt image.
const MAX_WHOLE: u64 = 256 * 1024 * 1024;

/// Serve one media request.
///
/// On a blocking pool: this reads from disk, and on a network drive the first
/// read costs tens of milliseconds. The thread driving the webview must not
/// wait for that.
pub fn handle(app: &AppHandle, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let response = build(&app, &request);
        responder.respond(response);
    });
}

fn build(app: &AppHandle, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Some((epoch, index)) = super::protocol::parse_entry(request.uri().path()) else {
        tracing::warn!("media: không phân tích được đường dẫn {:?}", request.uri());
        return error(StatusCode::BAD_REQUEST);
    };

    let state = app.state::<AppState>();
    if state.index_epoch() != epoch {
        return error(StatusCode::GONE);
    }
    let snapshot = state.snapshot();
    if index >= snapshot.len() {
        return error(StatusCode::NOT_FOUND);
    }
    let path = snapshot.full_path(index);
    // Released before the read: a rebuild must not wait on network I/O.
    drop(snapshot);

    let mime = mime_for(Path::new(&path));
    let Ok(mut file) = File::open(&path) else {
        // Deleted since the last scan. Ordinary, not an error worth logging at
        // every scroll.
        return error(StatusCode::NOT_FOUND);
    };
    let Ok(meta) = file.metadata() else {
        return error(StatusCode::NOT_FOUND);
    };
    let len = meta.len();
    if len == 0 {
        return error(StatusCode::NOT_FOUND);
    }

    let requested = request
        .headers()
        .get("range")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| parse_range(v, len));

    match requested {
        Some((start, end)) => {
            let mut buf = vec![0u8; (end - start + 1) as usize];
            if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut buf).is_err() {
                return error(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header("Content-Type", mime)
                .header("Accept-Ranges", "bytes")
                .header("Content-Range", format!("bytes {start}-{end}/{len}"))
                .header("Cache-Control", "no-store")
                .body(buf)
                .unwrap_or_else(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
        }
        None => {
            if len > MAX_WHOLE {
                // Refused rather than truncated: a half-read image is a broken
                // image, and the page cannot tell the difference.
                tracing::warn!("media: {path} quá lớn để gửi nguyên khối");
                return error(StatusCode::PAYLOAD_TOO_LARGE);
            }
            let mut buf = Vec::with_capacity(len as usize);
            if file.read_to_end(&mut buf).is_err() {
                return error(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", mime)
                .header("Accept-Ranges", "bytes")
                .header("Cache-Control", "no-store")
                .body(buf)
                .unwrap_or_else(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

/// Parse one `Range: bytes=start-end` header against a known file length.
///
/// Only a single range is handled. Multipart ranges exist in the standard and
/// no media player sends them; answering the first range of a multipart
/// request would be wrong, so anything with a comma is declined and the player
/// falls back to asking properly.
fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (from, to) = spec.split_once('-')?;

    let (start, end) = if from.is_empty() {
        // `bytes=-500` — the last 500 bytes. Players use this to read the
        // trailing index of an MP4 whose moov atom sits at the end.
        let n: u64 = to.trim().parse().ok()?;
        if n == 0 {
            return None;
        }
        (len.saturating_sub(n), len - 1)
    } else {
        let start: u64 = from.trim().parse().ok()?;
        let end = if to.trim().is_empty() {
            len - 1
        } else {
            to.trim().parse::<u64>().ok()?.min(len - 1)
        };
        (start, end)
    };

    if start > end || start >= len {
        return None;
    }
    // Capped, not refused: answering with less than was asked is legal, and it
    // keeps one request from pulling a whole film into memory.
    Some((start, end.min(start + MAX_CHUNK - 1)))
}

/// What the page should treat these bytes as.
///
/// Only formats the webview reliably decodes are claimed outright. Everything
/// else gets `application/octet-stream`, and what happens then was measured
/// rather than assumed: Chromium **sniffs the container** and plays it anyway
/// when it can. An `.mkv` holding H.264 played fine in this preview even
/// though nothing here called it a video.
///
/// So this is not a gate, it is a hint. Claiming a type the decoder then
/// refuses is the case worth avoiding — the player shows a black rectangle and
/// never reports an error. Leaving it unclaimed lets Chromium decide, and when
/// Chromium cannot, it raises `error` and the page shows "không xem trước
/// được định dạng này".
fn mime_for(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "mp3" => "audio/mpeg",
        "m4a" | "aac" => "audio/mp4",
        "wav" => "audio/wav",
        "ogg" | "opus" => "audio/ogg",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    }
}

fn error(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .body(Vec::new())
        .expect("static response builds")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_range_is_read_as_written() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=500-999", 1000), Some((500, 999)));
    }

    #[test]
    fn an_open_ended_range_runs_to_the_end_of_the_file() {
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
    }

    #[test]
    fn a_suffix_range_reads_the_tail() {
        // How a player finds the moov atom of an MP4 that was not written for
        // streaming: the index sits at the end of the file.
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=-5000", 1000), Some((0, 999)));
    }

    #[test]
    fn a_range_past_the_end_is_clamped_not_trusted() {
        assert_eq!(parse_range("bytes=0-99999", 1000), Some((0, 999)));
    }

    #[test]
    fn a_huge_request_is_capped_so_one_response_cannot_hold_a_whole_film() {
        let len = 4 * 1024 * 1024 * 1024;
        let (start, end) = parse_range("bytes=0-", len).expect("valid range");
        assert_eq!(start, 0);
        assert_eq!(end - start + 1, MAX_CHUNK);
    }

    #[test]
    fn nonsense_is_declined_rather_than_guessed_at() {
        assert_eq!(parse_range("bytes=500-100", 1000), None);
        assert_eq!(parse_range("bytes=2000-3000", 1000), None);
        assert_eq!(parse_range("items=0-10", 1000), None);
        assert_eq!(parse_range("bytes=abc", 1000), None);
        assert_eq!(parse_range("bytes=-0", 1000), None);
        // Multipart: declining is safer than answering only the first part.
        assert_eq!(parse_range("bytes=0-99,200-299", 1000), None);
    }

    #[test]
    fn only_formats_the_webview_can_decode_get_a_real_type() {
        assert_eq!(mime_for(Path::new("a.mp4")), "video/mp4");
        assert_eq!(mime_for(Path::new("a.JPG")), "image/jpeg");
        // MKV and AVI decode in almost nothing the webview ships with, so they
        // are deliberately not claimed as playable.
        assert_eq!(mime_for(Path::new("a.mkv")), "application/octet-stream");
        assert_eq!(mime_for(Path::new("a.avi")), "application/octet-stream");
        assert_eq!(
            mime_for(Path::new("khong-duoi")),
            "application/octet-stream"
        );
    }
}
