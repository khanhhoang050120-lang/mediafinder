//! Nhật ký chẩn đoán ra file — để bản đã cài cũng kể lại được chuyện gì xảy ra.
//!
//! Bài học trực tiếp từ đợt gỡ lỗi "không thấy thông báo cập nhật" (P20):
//! `tracing` chỉ ra stderr, mà bản cài đặt làm gì có console — nên toàn bộ
//! chẩn đoán trên máy người dùng là suy luận chay. Từ nay mọi dòng log đi
//! đồng thời hai ngả: stderr (cho phiên dev) và một file trong thư mục dữ
//! liệu (cho 20–40 máy ngoài kia). Menu khay có "Xem nhật ký" mở thẳng thư
//! mục này.
//!
//! Xoay theo dung lượng, không theo ngày — tránh kéo thêm dependency thời
//! gian chỉ để đặt tên file. Lúc khởi động, file chính vượt 5 MB thì dây
//! chuyền dịch xuống (`.1` → `.2` → … → `.5`, cái cuối rơi khỏi mép). Một
//! phiên chạy dài có thể vượt trần đôi chút — chấp nhận: log của app này
//! tính bằng kilobyte mỗi ngày.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

/// Trần dung lượng file chính trước khi dây chuyền dịch xuống.
const ROTATE_AT: u64 = 5 * 1024 * 1024;

/// Số file cũ giữ lại (`mediafinder.1.log` … `mediafinder.{KEEP}.log`).
const KEEP: u32 = 5;

/// Thư mục chứa nhật ký: cạnh cache, trong dữ liệu riêng của ứng dụng.
pub fn logs_dir() -> Option<PathBuf> {
    let cache = crate::index::persist::cache_path().ok()?;
    Some(cache.parent()?.join("logs"))
}

fn main_log_path(dir: &std::path::Path) -> PathBuf {
    dir.join("mediafinder.log")
}

/// Dây chuyền dịch xuống một nấc. Lỗi ở đây không đáng chặn khởi động —
/// cùng lắm là file log to hơn dự kiến.
fn rotate(dir: &std::path::Path) {
    let main = main_log_path(dir);
    let too_big = std::fs::metadata(&main)
        .map(|m| m.len() > ROTATE_AT)
        .unwrap_or(false);
    if !too_big {
        return;
    }
    let slot = |n: u32| dir.join(format!("mediafinder.{n}.log"));
    let _ = std::fs::remove_file(slot(KEEP));
    for n in (1..KEEP).rev() {
        let _ = std::fs::rename(slot(n), slot(n + 1));
    }
    let _ = std::fs::rename(&main, slot(1));
}

/// `File` bọc trong khoá, chia được cho nhiều thread của tracing.
///
/// Mỗi bản ghi log là một lần khoá–ghi–mở; tần suất log của app này thấp
/// tới mức tranh chấp không đo được, đổi lại không cần thêm crate nào.
#[derive(Clone)]
pub struct SharedFile(Arc<Mutex<File>>);

impl Write for SharedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("log lock").write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().expect("log lock").flush()
    }
}

impl<'a> MakeWriter<'a> for SharedFile {
    type Writer = SharedFile;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Mở (và xoay nếu cần) file log của phiên này.
fn open_log_file() -> Option<SharedFile> {
    let dir = logs_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    rotate(&dir);
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(main_log_path(&dir))
        .ok()?;
    Some(SharedFile(Arc::new(Mutex::new(file))))
}

/// Khởi tạo tracing: stderr + file, cùng một bộ lọc `RUST_LOG`.
///
/// Không mở được file (đĩa đầy, thư mục bị khoá…) thì lùi về stderr-đơn như
/// trước — thiếu nhật ký file là điều đáng tiếc, không phải lý do từ chối
/// chạy.
pub fn init() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("mediafinder=info,warn"));

    match open_log_file() {
        Some(file) => {
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().with_writer(std::io::stderr))
                .with(fmt::layer().with_ansi(false).with_writer(file))
                .try_init();
        }
        None => {
            let _ = fmt().with_env_filter(filter).try_init();
            tracing::warn!("không mở được file nhật ký — chỉ ghi ra stderr phiên này");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dây chuyền xoay: file to dịch xuống .1, .KEEP cũ rơi khỏi mép, file
    /// nhỏ thì đứng yên.
    #[test]
    fn xoay_theo_dung_luong() {
        let dir = std::env::temp_dir().join(format!("mf-diag-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // File nhỏ: không xoay.
        std::fs::write(main_log_path(&dir), b"nho").unwrap();
        rotate(&dir);
        assert!(main_log_path(&dir).exists(), "file nho khong duoc dich di");
        assert!(!dir.join("mediafinder.1.log").exists());

        // File vượt trần: dịch xuống .1; .KEEP cũ biến mất.
        std::fs::write(main_log_path(&dir), vec![b'x'; (ROTATE_AT + 1) as usize]).unwrap();
        std::fs::write(dir.join(format!("mediafinder.{KEEP}.log")), b"gia nhat").unwrap();
        rotate(&dir);
        assert!(!main_log_path(&dir).exists(), "file to phai duoc dich di");
        assert!(dir.join("mediafinder.1.log").exists());
        assert!(
            !dir.join(format!("mediafinder.{KEEP}.log")).exists()
                || std::fs::metadata(dir.join(format!("mediafinder.{KEEP}.log")))
                    .map(|m| m.len() != 8)
                    .unwrap_or(true),
            "ban gia nhat phai roi khoi mep"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
