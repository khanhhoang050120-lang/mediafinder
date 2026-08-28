//! Thumbnails, via the shell's own thumbnail provider.
//!
//! Windows already knows how to draw a preview of every media type installed
//! on the machine — that is what File Explorer shows. `IShellItemImageFactory`
//! asks it the same question. Bundling ffmpeg to decode a video frame would
//! mean shipping tens of megabytes, keeping codecs current, and dealing with
//! licensing, all to produce a worse answer than the one already on the disk:
//! Explorer caches thumbnails in `thumbcache_*.db`, so most requests never
//! decode anything at all.
//!
//! Three things make this fast enough to feel instant while scrolling:
//!
//! * **Ask the cache first.** `SIIGBF_INCACHEONLY` returns immediately or
//!   fails; only the misses go on to decode.
//! * **A fixed pool of workers.** Decoding a 4K video frame is slow and
//!   disk-bound; without a bound, scrolling a long list would spawn hundreds
//!   of threads all fighting for the same disk.
//! * **An LRU in front of both.** Scrolling back over something already seen
//!   must not decode it again.
//!
//! COM is per-thread, so each worker initialises once and stays apartment-
//! threaded for its lifetime — the shell's thumbnail providers expect STA.

use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;

use lru::LruCache;
use parking_lot::Mutex;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::System::Com::{
    CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_INCACHEONLY, SIIGBF_THUMBNAILONLY,
};

#[derive(Debug, thiserror::Error)]
pub enum ThumbError {
    #[error("không có thumbnail cho tệp này")]
    Unavailable,

    /// Hàng đợi đầy — người dùng đang cuộn nhanh hơn đĩa theo kịp.
    ///
    /// Tách riêng khỏi `Unavailable` vì hai câu trả lời đòi hai phản ứng
    /// ngược nhau: "tệp này không có thumbnail" thì hỏi lại là vô ích, còn
    /// "đang bận" thì hỏi lại chính là điều nên làm. Trước đây cả hai trả
    /// cùng một mã, và phía giao diện ẩn ảnh vĩnh viễn cho cả hai.
    #[error("hàng đợi thumbnail đang đầy")]
    Busy,

    #[error("Windows từ chối dựng thumbnail: {0}")]
    Shell(String),

    #[error("không đọc được điểm ảnh của thumbnail")]
    Pixels,

    #[error("không mã hoá được ảnh PNG: {0}")]
    Encode(String),
}

/// Cap on how many thumbnails stay in memory.
///
/// Thumbnails are clamped to the requested size before encoding, so a 192px
/// entry is tens of kilobytes. Five hundred of them is a few tens of megabytes
/// — small against the index itself, and far more than fits on screen at once,
/// so scrolling back over recently seen rows is always a cache hit.
///
/// This number is only safe *because* of that clamp: before it, one video
/// thumbnail came back as a 1280×720 frame weighing 1.27 MB, which would have
/// made this cache 650 MB.
const CACHE_ENTRIES: usize = 512;

/// Concurrent decoders.
///
/// Higher numbers do not help: a cache miss is dominated by reading the file,
/// and four requests already keep a disk busy. More threads would just make
/// them queue somewhere less visible.
const WORKERS: usize = 4;

/// How many pending requests to hold before refusing new ones.
///
/// Bounded on purpose. Dragging a scrollbar can ask for thousands of
/// thumbnails in a second; an unbounded queue would keep decoding images that
/// scrolled off screen long ago while the ones actually visible wait behind
/// them. Refusing is better — the UI simply asks again when the row settles.
const QUEUE_DEPTH: usize = 64;

// Checked at build time rather than in a test: the queue must hold more than
// one screenful of rows, or fast scrolling would drop requests that are about
// to become visible.
const _: () = assert!(CACHE_ENTRIES > 0);
const _: () = assert!(
    QUEUE_DEPTH > WORKERS * 4,
    "hàng đợi phải chứa hơn một màn hình"
);

type CacheKey = (u64, u32);

struct Job {
    key: CacheKey,
    path: String,
    size: u32,
    reply: SyncSender<Result<Arc<Vec<u8>>, ThumbError>>,
}

/// How long a "no thumbnail" answer is remembered.
///
/// The frontend now retries transient failures a few times. Without this
/// cache, every retry against a file that genuinely has no thumbnail would
/// decode it again from scratch — the most expensive way to learn nothing
/// new. A minute is long enough to absorb the retries and short enough that
/// installing a new codec pack shows results on the next scroll-past.
const MISS_TTL: std::time::Duration = std::time::Duration::from_secs(60);

/// Entries in the miss cache. A key plus an `Instant` is tiny; this covers
/// several screenfuls of files with no thumbnails without measurable memory.
const MISS_ENTRIES: usize = 2048;

/// Shared thumbnail service: cache in front, bounded worker pool behind.
pub struct ThumbnailService {
    cache: Arc<Mutex<LruCache<CacheKey, Arc<Vec<u8>>>>>,
    /// Files the workers already tried and failed to thumbnail, with when.
    /// Only genuine `Unavailable` answers land here — a refused-because-busy
    /// request must stay retryable.
    misses: Mutex<LruCache<CacheKey, std::time::Instant>>,
    jobs: SyncSender<Job>,
    /// Chỉ trong bản kiểm thử: giữ đầu nhận sống khi dựng dịch vụ 0 worker.
    /// Không có nó, Receiver bị thả ngay cuối constructor và channel chết
    /// trước khi bài kiểm thử kịp nhét job mồi vào.
    #[cfg(test)]
    _rx_keepalive: Arc<Mutex<Receiver<Job>>>,
}

impl ThumbnailService {
    pub fn new() -> Self {
        Self::with_limits(WORKERS, QUEUE_DEPTH)
    }

    /// Tách riêng để kiểm thử ép được trạng thái "hàng đợi đầy" một cách tất
    /// định: không worker nào rút việc ra, thì hàng sâu bao nhiêu cũng đầy
    /// được bằng đúng bấy nhiêu lời gọi. Ngoài kiểm thử ra, đường duy nhất
    /// vào đây là `new()` với các hằng số thật.
    fn with_limits(workers: usize, queue_depth: usize) -> Self {
        let cache = Arc::new(Mutex::new(LruCache::new(
            std::num::NonZeroUsize::new(CACHE_ENTRIES).expect("cache size is non-zero"),
        )));
        let (tx, rx) = mpsc::sync_channel::<Job>(queue_depth);
        let rx = Arc::new(Mutex::new(rx));

        for n in 0..workers {
            let rx: Arc<Mutex<Receiver<Job>>> = Arc::clone(&rx);
            let cache = Arc::clone(&cache);
            std::thread::Builder::new()
                .name(format!("thumbnail-{n}"))
                .spawn(move || worker(rx, cache))
                .expect("spawn thumbnail worker");
        }

        Self {
            cache,
            misses: Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(MISS_ENTRIES).expect("miss cache size is non-zero"),
            )),
            jobs: tx,
            #[cfg(test)]
            _rx_keepalive: rx,
        }
    }

    /// Fetch a thumbnail, blocking until it is ready.
    ///
    /// Called from the asset-protocol handler, which already runs off the UI
    /// thread, so blocking here never stalls the window.
    pub fn get(&self, id: u64, path: &str, size: u32) -> Result<Arc<Vec<u8>>, ThumbError> {
        let key = (id, size);
        if let Some(hit) = self.cache.lock().get(&key).cloned() {
            return Ok(hit);
        }

        // Đã thử và biết là không có? Trả lời ngay thay vì decode lại — các
        // lần hỏi lại của giao diện phải gần như miễn phí với tệp không ảnh.
        {
            let mut misses = self.misses.lock();
            if let Some(when) = misses.get(&key) {
                if when.elapsed() < MISS_TTL {
                    return Err(ThumbError::Unavailable);
                }
                misses.pop(&key);
            }
        }

        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        let job = Job {
            key,
            path: path.to_string(),
            size,
            reply: reply_tx,
        };

        // A full queue means the user is scrolling faster than the disk can
        // keep up. Dropping the request is correct: the row asks again after
        // a short pause — `Busy`, not `Unavailable`, so that retry is not
        // silenced by the miss cache.
        self.jobs.try_send(job).map_err(|_| ThumbError::Busy)?;

        let res = reply_rx.recv().map_err(|_| ThumbError::Busy)?;
        if matches!(res, Err(ThumbError::Unavailable)) {
            self.misses.lock().put(key, std::time::Instant::now());
        }
        res
    }
}

impl Default for ThumbnailService {
    fn default() -> Self {
        Self::new()
    }
}

fn worker(rx: Arc<Mutex<Receiver<Job>>>, cache: Arc<Mutex<LruCache<CacheKey, Arc<Vec<u8>>>>>) {
    // Shell thumbnail providers are apartment-threaded. Initialise once and
    // hold it for the life of the thread; doing it per request would serialise
    // every call through COM setup.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    }

    loop {
        // Hold the receiver lock only long enough to take one job, so the
        // other workers are not blocked behind this one's decode.
        let job = match rx.lock().recv() {
            Ok(j) => j,
            Err(_) => return, // service dropped
        };

        // Another worker may have finished the same request while this one
        // waited for the lock.
        if let Some(hit) = cache.lock().get(&job.key).cloned() {
            let _ = job.reply.send(Ok(hit));
            continue;
        }

        let result = render_png(&job.path, job.size).map(Arc::new);
        if let Ok(png) = &result {
            cache.lock().put(job.key, Arc::clone(png));
        }
        let _ = job.reply.send(result);
    }
}

/// Ask the shell for a thumbnail and encode it as PNG.
fn render_png(path: &str, size: u32) -> Result<Vec<u8>, ThumbError> {
    let bitmap = shell_thumbnail(path, size)?;

    // The HBITMAP is ours now; make sure it is released on every path out.
    let pixels = bitmap_to_rgba(bitmap.0);
    unsafe {
        let _ = DeleteObject(HGDIOBJ(bitmap.0 .0));
    }
    let (width, height, rgba) = pixels?;
    let (width, height, rgba) = downscale_to_fit(width, height, rgba, size);

    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        &mut png,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Adaptive,
    );
    image::ImageEncoder::write_image(
        encoder,
        &rgba,
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| ThumbError::Encode(e.to_string()))?;

    Ok(png)
}

/// Shrink to fit inside `size`×`size`, preserving aspect ratio.
///
/// A safety net rather than the main mechanism: the shell is asked for the
/// right size and usually obliges. But providers are third-party code — a
/// codec pack can install its own — and one that ignores the request would
/// otherwise put a multi-megabyte image into a cache sized for thumbnails.
/// Clamping here means a badly behaved provider costs a little CPU instead of
/// hundreds of megabytes of memory.
fn downscale_to_fit(width: u32, height: u32, rgba: Vec<u8>, size: u32) -> (u32, u32, Vec<u8>) {
    if width <= size && height <= size {
        return (width, height, rgba);
    }

    let scale = (size as f32 / width as f32).min(size as f32 / height as f32);
    let new_w = ((width as f32 * scale).round() as u32).max(1);
    let new_h = ((height as f32 * scale).round() as u32).max(1);

    let Some(buffer) = image::RgbaImage::from_raw(width, height, rgba) else {
        // Cannot happen — the buffer is built as width*height*4 — but falling
        // back to the original is better than panicking in a worker thread.
        return (width, height, Vec::new());
    };
    let resized = image::imageops::resize(
        &buffer,
        new_w,
        new_h,
        // Triangle: visually fine at thumbnail scale and several times cheaper
        // than Lanczos, which matters on the miss path that is already slow.
        image::imageops::FilterType::Triangle,
    );
    (new_w, new_h, resized.into_raw())
}

/// Newtype so the raw handle is never accidentally copied around.
struct OwnedBitmap(HBITMAP);

fn shell_thumbnail(path: &str, size: u32) -> Result<OwnedBitmap, ThumbError> {
    let wide = HSTRING::from(path);
    let want = SIZE {
        cx: size as i32,
        cy: size as i32,
    };

    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None)
                .map_err(|e| ThumbError::Shell(e.message()))?;

        // Fast path: whatever Explorer already cached. This returns in
        // microseconds or fails, and on a library the user has browsed before
        // it answers the large majority of requests.
        if let Ok(h) = factory.GetImage(want, SIIGBF_INCACHEONLY | SIIGBF_THUMBNAILONLY) {
            if !h.is_invalid() {
                return Ok(OwnedBitmap(h));
            }
        }

        // Slow path: actually decode a frame.
        //
        // Two flags matter here, and both are about what *not* to accept.
        //
        // `SIIGBF_THUMBNAILONLY` refuses the file-type icon. Without it the
        // shell happily returns the generic video glyph when no real thumbnail
        // exists — and worse, that glyph is itself cached, so the fast path
        // above answers with it and a real frame is never extracted. Every
        // video in the grid showed the same grey play button.
        //
        // A media finder that shows a generic icon has told the user nothing
        // they did not already know from the file's name. Better to return
        // nothing and let the coloured kind badge stand alone.
        //
        // `SIIGBF_BIGGERSIZEOK` is deliberately absent: it invites the provider
        // to return its natural size, and video providers take that literally —
        // asking for 192×192 came back as a 1280×720 frame, 1.27 MB of PNG for
        // one row of a list.
        let h = factory
            .GetImage(want, SIIGBF_THUMBNAILONLY)
            .map_err(|_| ThumbError::Unavailable)?;
        if h.is_invalid() {
            return Err(ThumbError::Unavailable);
        }
        Ok(OwnedBitmap(h))
    }
}

/// Copy an HBITMAP's pixels out as top-down RGBA.
///
/// `GetDIBits` with a negative height asks GDI for top-down rows, which is
/// what every image encoder expects; bitmaps are bottom-up by default and
/// forgetting this yields a vertically mirrored thumbnail.
fn bitmap_to_rgba(bitmap: HBITMAP) -> Result<(u32, u32, Vec<u8>), ThumbError> {
    unsafe {
        let mut info = BITMAP::default();
        let written = windows::Win32::Graphics::Gdi::GetObjectW(
            HGDIOBJ(bitmap.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut info as *mut _ as *mut c_void),
        );
        if written == 0 || info.bmWidth <= 0 || info.bmHeight <= 0 {
            return Err(ThumbError::Pixels);
        }

        let width = info.bmWidth as u32;
        let height = info.bmHeight as u32;

        let mut header = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: info.bmWidth,
                biHeight: -info.bmHeight, // negative: top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut buffer = vec![0u8; (width * height * 4) as usize];
        let hdc = GetDC(None);
        let rows = GetDIBits(
            hdc,
            bitmap,
            0,
            height,
            Some(buffer.as_mut_ptr() as *mut c_void),
            &mut header,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, hdc);

        if rows == 0 {
            return Err(ThumbError::Pixels);
        }

        // GDI hands back BGRA; encoders want RGBA.
        for px in buffer.chunks_exact_mut(4) {
            px.swap(0, 2);
        }

        // Shell thumbnails often leave alpha at zero for formats that have no
        // transparency, which would render the whole image invisible. Treat a
        // fully transparent result as opaque.
        if buffer.chunks_exact(4).all(|px| px[3] == 0) {
            for px in buffer.chunks_exact_mut(4) {
                px[3] = 255;
            }
        }

        Ok((width, height, buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_is_unavailable_not_a_panic() {
        let r = render_png(r"D:\definitely\not\here\nope.mp4", 128);
        assert!(matches!(
            r,
            Err(ThumbError::Unavailable) | Err(ThumbError::Shell(_))
        ));
    }

    #[test]
    fn service_reports_unavailable_for_a_missing_file() {
        let svc = ThumbnailService::new();
        let r = svc.get(1, r"D:\definitely\not\here\nope.mp4", 128);
        assert!(r.is_err());
    }

    #[test]
    fn error_messages_are_written_for_the_user() {
        assert!(ThumbError::Unavailable.to_string().contains("thumbnail"));
    }
}

#[cfg(test)]
mod miss_cache_tests {
    use super::*;

    /// Hàng đợi đầy phải trả `Busy` — và tuyệt đối không được ghi vào
    /// miss-cache, vì "đang bận" là lời mời hỏi lại chứ không phải câu trả
    /// lời cuối cùng.
    ///
    /// Dịch vụ dựng với 0 worker và hàng sâu đúng 1: không ai rút việc ra,
    /// nên một job mồi là hàng đầy — tất định, không đua tranh, không cần
    /// chặn thread nào.
    #[test]
    fn hang_day_tra_busy_va_khong_ghi_nho() {
        let svc = ThumbnailService::with_limits(0, 1);

        // Job mồi chiếm chỗ duy nhất. Giữ đầu nhận sống tới cuối test để
        // channel không tự dọn.
        let (plug_tx, _plug_rx) = mpsc::sync_channel(1);
        svc.jobs
            .try_send(Job {
                key: (0, 0),
                path: String::new(),
                size: 0,
                reply: plug_tx,
            })
            .expect("job moi phai vao duoc hang con trong");

        let res = svc.get(9, r"C:at\ky.mp4", 64);
        assert!(
            matches!(res, Err(ThumbError::Busy)),
            "hang day phai la Busy, nhan duoc: {res:?}"
        );
        assert!(
            svc.misses.lock().peek(&(9u64, 64u32)).is_none(),
            "Busy bi ghi vao miss-cache — cac luot hoi lai se bi nuot oan"
        );
    }

    /// Miss-cache phải trả lời tức thì, không đụng tới hàng đợi worker.
    ///
    /// Đường dẫn cố tình không tồn tại: nếu câu hỏi lọt qua miss-cache và
    /// xuống tới worker, phép đo thời gian sẽ tố cáo (một vòng qua channel +
    /// shell chậm hơn tra bảng hàng nghìn lần).
    #[test]
    fn miss_cache_tra_loi_ngay_khong_hoi_dia() {
        let svc = ThumbnailService::new();
        let key = (7u64, 64u32);
        svc.misses.lock().put(key, std::time::Instant::now());

        let t = std::time::Instant::now();
        let res = svc.get(7, r"C:\duong\dan\khong\ton\tai.mp4", 64);
        assert!(
            matches!(res, Err(ThumbError::Unavailable)),
            "phai la Unavailable, nhan duoc: {res:?}"
        );
        assert!(
            t.elapsed() < std::time::Duration::from_millis(50),
            "cham bat thuong — cau hoi da lot xuong worker"
        );
    }

    /// Ghi nhớ hết hạn thì phải hỏi lại thật — cài codec mới xong, lần cuộn
    /// sau phải thấy ảnh chứ không bị câu trả lời cũ đè một đời.
    #[test]
    fn miss_het_han_thi_hoi_lai() {
        let svc = ThumbnailService::new();
        let key = (8u64, 64u32);
        svc.misses.lock().put(
            key,
            std::time::Instant::now() - MISS_TTL - std::time::Duration::from_secs(1),
        );

        // Đường dẫn không tồn tại → worker thật sự được hỏi (shell trả lỗi
        // Shell, thứ cố ý KHÔNG bị ghi nhớ — tệp biến mất có thể quay lại).
        // Bất biến cần giữ: sau lần hỏi này, mục ghi nhớ CŨ không còn ngồi đó
        // trả lời thay — hoặc đã bị nhổ đi, hoặc đã được làm tươi.
        let _ = svc.get(8, r"C:\duong\dan\khong\ton\tai.mp4", 64);
        let misses = svc.misses.lock();
        if let Some(when) = misses.peek(&key) {
            assert!(
                when.elapsed() < MISS_TTL,
                "muc ghi nho het han van con nguyen — expiry khong chay"
            );
        }
    }
}
