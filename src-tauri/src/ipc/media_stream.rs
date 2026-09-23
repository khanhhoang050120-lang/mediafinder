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

    // Video mà WebView2 không giải mã được (ProRes…) KHÔNG đi qua đây: chúng
    // được chuyển mã và giao từng mảnh qua các lệnh `preview_*`
    // ([`crate::media::ffphien`]). Bộ phục vụ URI của Tauri chỉ nhận một thân
    // đáp ứng đã hoàn chỉnh, nên qua đây thì phải chuyển mã xong mới gửi được
    // gì — đúng cái chờ mà người dùng phàn nàn.

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
/// # Vì sao đây là GỢI Ý, không phải cổng chặn
///
/// Định dạng tệp và bộ giải mã bên trong là hai chuyện khác nhau. `.mkv` chỉ
/// nói "đây là hộp Matroska", không nói bên trong là H.264 (WebView2 phát
/// được) hay HEVC/AV1 (thường thì không). Phần đuôi tệp **không** trả lời
/// được câu hỏi quan trọng, nên chỗ này không cố trả lời thay.
///
/// Việc phán xử thuộc về Chromium: nó mở container, đọc phần mô tả luồng, và
/// nếu không giải mã nổi thì raise `error` — lúc đó trang hiện "không xem
/// trước được định dạng này" kèm nút mở bằng ứng dụng mặc định. Đó là câu trả
/// lời trung thực, và nó tới sau khi đã THỬ.
///
/// # Vì sao `application/octet-stream` là câu trả lời sai
///
/// Bản trước để `.mkv` và `.avi` ở `application/octet-stream`, tin rằng
/// Chromium sẽ tự đoán container. Nó **không**: với `<video src>` trỏ vào một
/// scheme tuỳ biến, WebView2 tin `Content-Type` và từ chối ngay khi thấy
/// `octet-stream` — không mở tệp, không đọc luồng, không thử gì cả. Nên mọi
/// `.mkv` đều báo "không xem trước được", kể cả những tệp H.264 mà nó thừa
/// sức phát.
///
/// Khai một kiểu video thật thì tệ nhất cũng chỉ đưa ta về đúng chỗ cũ — bộ
/// giải mã từ chối và trang báo lỗi — còn tốt nhất là tệp phát được. Không có
/// chiều nào xấu hơn hiện trạng.
///
/// # `video/quicktime` là một cái bẫy riêng
///
/// `.mov` từng được khai là `video/quicktime`, thứ Chromium không nhận. Nhưng
/// `.mov` và `.mp4` dùng chung cấu trúc ISO base media, và đa số `.mov` của
/// máy ảnh chứa H.264 — khai là `video/mp4` thì chính những tệp đó phát được.
fn mime_for(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        // ISO base media: `.mov` đi chung với `.mp4` vì cùng một cấu trúc hộp,
        // và `video/quicktime` thì Chromium không nhận.
        "mp4" | "m4v" | "mov" => "video/mp4",
        "webm" => "video/webm",
        // Matroska. WebView2 phát được khi bên trong là H.264/VP9/AV1 — tức
        // phần lớn `.mkv` tải về. HEVC thì tuỳ máy có bộ giải mã phần cứng.
        "mkv" => "video/x-matroska",
        // Các container còn lại: khai đúng tên để Chromium THỬ. Cái nào nó
        // không mở nổi thì báo lỗi, đúng như khi chưa khai gì.
        "avi" => "video/x-msvideo",
        "ts" | "m2ts" | "mts" => "video/mp2t",
        "mpg" | "mpeg" | "m2v" | "mpv" => "video/mpeg",
        "3gp" => "video/3gpp",
        "ogv" => "video/ogg",
        "flv" => "video/x-flv",
        "wmv" | "asf" => "video/x-ms-wmv",

        "jpg" | "jpeg" | "jfif" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        // HEIC: Chromium trên Windows chưa giải mã được, nhưng khai đúng vẫn
        // hơn — khi nào WebView2 hỗ trợ thì tự chạy, không phải sửa lại đây.
        "heic" | "heif" => "image/heic",
        "tif" | "tiff" => "image/tiff",

        "mp3" => "audio/mpeg",
        "m4a" | "m4b" | "aac" => "audio/mp4",
        "wav" => "audio/wav",
        // `.opus` là Opus trong hộp Ogg. KHÔNG khai `audio/opus`: đo trên lõi
        // Edge (cùng lõi với WebView2), `canPlayType("audio/opus")` trả rỗng —
        // tức trình duyệt không nhận kiểu đó — còn `audio/ogg` thì nhận.
        "ogg" | "oga" | "opus" => "audio/ogg",
        "flac" => "audio/flac",
        "weba" => "audio/webm",
        "aiff" | "aif" => "audio/aiff",
        "wma" => "audio/x-ms-wma",
        "mid" | "midi" => "audio/midi",

        // Không nhận ra: để Chromium tự quyết thay vì khẳng định một điều
        // không biết.
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
    fn every_video_container_is_claimed_as_video() {
        // Bài kiểm thử cũ khoá đúng con bọ này lại: nó khẳng định .mkv và .avi
        // PHẢI là `application/octet-stream`, tin rằng Chromium sẽ tự sniff.
        // Nó **không** sniff — với một thẻ video trỏ vào scheme tuỳ biến thì
        // `octet-stream` bị từ chối thẳng, nên mọi .mkv đều báo "không xem
        // trước được" kể cả khi bên trong là H.264.
        //
        // Không kiểm chuỗi MIME cụ thể của từng đuôi: đó là chi tiết có thể
        // đổi. Kiểm cái bất biến thật — một tệp video không bao giờ được rời
        // khỏi đây dưới dạng `octet-stream`, vì đó là hình thức duy nhất bảo
        // đảm nó KHÔNG được thử.
        for name in [
            "a.mp4", "a.mkv", "a.mov", "a.avi", "a.webm", "a.m4v", "a.wmv", "a.flv", "a.mpg",
            "a.mpeg", "a.m2ts", "a.mts", "a.ts", "a.3gp", "a.ogv", "a.asf", "a.m2v", "a.mpv",
        ] {
            let got = mime_for(Path::new(name));
            assert!(
                got.starts_with("video/"),
                "{name} phải được khai là video, nhận được {got}"
            );
        }
    }

    #[test]
    fn quicktime_is_never_claimed() {
        // `.mov` từng mang `video/quicktime`, thứ Chromium không nhận — nên
        // một tệp H.264 hoàn toàn phát được vẫn hiện ra khung đen. Nó dùng
        // chung cấu trúc hộp với .mp4, nên khai như .mp4 là cách để nó THỬ.
        assert_eq!(mime_for(Path::new("a.mov")), "video/mp4");
        assert_eq!(mime_for(Path::new("a.MOV")), "video/mp4");
    }

    #[test]
    fn images_and_audio_keep_their_own_types() {
        assert_eq!(mime_for(Path::new("a.JPG")), "image/jpeg");
        assert_eq!(mime_for(Path::new("a.png")), "image/png");
        assert_eq!(mime_for(Path::new("a.mp3")), "audio/mpeg");
        assert_eq!(mime_for(Path::new("a.flac")), "audio/flac");
        // Hộp Ogg — kiểu `audio/opus` thì WebView2 không nhận.
        assert_eq!(mime_for(Path::new("a.opus")), "audio/ogg");
    }

    #[test]
    fn an_unknown_extension_is_left_unclaimed() {
        // Chỗ duy nhất `octet-stream` còn đúng: app không biết đây là gì, nên
        // nó không khẳng định gì cả.
        assert_eq!(
            mime_for(Path::new("khong-duoi")),
            "application/octet-stream"
        );
        assert_eq!(mime_for(Path::new("a.txt")), "application/octet-stream");
    }
}
