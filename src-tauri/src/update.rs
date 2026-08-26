//! Nói cho người dùng biết khi có bản mới.
//!
//! Bộ cài nặng hơn 200 MB và chỉ nằm trên trang Releases. Không có phần này
//! thì người dùng ở lại bản cũ cho tới khi tự nhớ ra mà đi tìm — nghĩa là gần
//! như không bao giờ.
//!
//! **Chỉ kiểm tra, không tự tải.** Việc tải hơn 200 MB là quyết định của người
//! đang trả tiền cho đường truyền đó, nên phần này dừng ở chỗ báo tin; frontend
//! hỏi, và chỉ tải khi người dùng đồng ý.
//!
//! Ứng dụng khởi động cùng Windows ở chế độ ẩn, nên lúc đăng nhập không có cửa
//! sổ nào để hiện thông báo. Vì vậy tin báo đi vào **tooltip của khay hệ
//! thống**, chỗ duy nhất nhìn thấy được khi chưa có cửa sổ, và chờ ở đó tới khi
//! người dùng mở ứng dụng lên.

use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use serde::Serialize;

/// Phiên bản mới nhất tìm được, nếu có. Frontend đọc qua lệnh
/// [`crate::ipc::commands::update_status`].
static AVAILABLE: Mutex<Option<String>> = Mutex::new(None);

/// Đã kiểm tra xong lần này chưa — để giao diện phân biệt "chưa biết" với
/// "đã hỏi rồi, không có gì mới".
static CHECKED: AtomicBool = AtomicBool::new(false);

/// Những gì giao diện cần biết về tình hình cập nhật.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UpdateStatus {
    /// Đã hỏi máy chủ xong chưa. Sai nghĩa là chưa có câu trả lời, không phải
    /// là không có bản mới.
    pub checked: bool,
    /// Phiên bản mới, ví dụ `"1.1.0"`. `None` khi đang chạy bản mới nhất.
    pub available: Option<String>,
    /// Phiên bản đang chạy, để giao diện nói "1.0.0 → 1.1.0".
    pub current: String,
}

/// Bản đang chạy, lấy từ `tauri.conf.json` lúc biên dịch.
pub fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Tình hình hiện tại. Rẻ — chỉ đọc thứ đã ghi sẵn, không gọi mạng.
pub fn status() -> UpdateStatus {
    UpdateStatus {
        checked: CHECKED.load(Ordering::Relaxed),
        available: AVAILABLE.lock().clone(),
        current: current_version(),
    }
}

/// Ghi lại kết quả một lần kiểm tra và cập nhật tooltip khay hệ thống.
///
/// Tách riêng khỏi phần gọi mạng để test được mà không cần máy chủ thật.
pub fn record(app: &tauri::AppHandle, version: Option<String>) {
    *AVAILABLE.lock() = version.clone();
    CHECKED.store(true, Ordering::Relaxed);

    if let Some(v) = version {
        tracing::info!("có bản mới: {v}");
        set_tray_tooltip(app, &format!("MediaFinder — có bản {v}, mở để cập nhật"));
    } else {
        tracing::info!("đang chạy bản mới nhất ({})", current_version());
    }
}

/// Đổi tooltip khay để người dùng thấy tin ngay cả khi chưa mở cửa sổ.
///
/// Thất bại ở đây không đáng để làm hỏng việc gì: tin vẫn còn trong
/// [`AVAILABLE`] và giao diện sẽ hiện khi cửa sổ mở ra.
fn set_tray_tooltip(app: &tauri::AppHandle, text: &str) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Err(e) = tray.set_tooltip(Some(text)) {
            tracing::warn!("không đổi được tooltip khay: {e}");
        }
    }
}

/// Hỏi máy chủ xem có bản mới không, chạy nền.
///
/// Gọi lúc khởi động. Không chặn: mạng có thể chậm hoặc không có, và tìm kiếm
/// phải dùng được ngay dù việc này chưa xong.
///
/// Mọi lỗi đều nuốt sau khi ghi log. Không kiểm tra được bản mới là chuyện
/// vặt — nó không được phép cản trở việc chính của ứng dụng, và người dùng
/// không làm gì được với một thông báo "không kết nối được máy chủ cập nhật".
pub fn check_in_background(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_updater::UpdaterExt;

        let updater = match app.updater() {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!("không dựng được updater: {e}");
                return;
            }
        };

        match updater.check().await {
            Ok(Some(update)) => record(&app, Some(update.version.clone())),
            Ok(None) => record(&app, None),
            // Không có mạng là trường hợp thường gặp nhất ở đây, và nó bình
            // thường — máy có thể đang offline. Ghi log rồi thôi.
            Err(e) => tracing::info!("không kiểm tra được cập nhật: {e}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trạng thái nằm trong biến tĩnh dùng chung, nên các test phải nối đuôi
    /// nhau thay vì chạy song song.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn reset() {
        *AVAILABLE.lock() = None;
        CHECKED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn chua_kiem_tra_thi_khong_phai_la_khong_co_ban_moi() {
        let _g = SERIAL.lock();
        reset();

        let s = status();
        assert!(!s.checked, "chưa hỏi máy chủ thì checked phải là false");
        assert_eq!(s.available, None);
    }

    #[test]
    fn phien_ban_hien_tai_lay_tu_cargo() {
        let _g = SERIAL.lock();
        // Đây là số hiệu mà `tauri.conf.json` và `Cargo.toml` phải giữ khớp
        // nhau; lệch là bản cập nhật sẽ so sánh sai.
        assert_eq!(current_version(), env!("CARGO_PKG_VERSION"));
        assert!(
            !current_version().is_empty(),
            "phiên bản rỗng thì updater không so sánh được"
        );
    }

    #[test]
    fn status_phan_anh_dung_thu_da_ghi() {
        let _g = SERIAL.lock();
        reset();

        // Không có bản mới: đã kiểm tra, nhưng không có gì.
        *AVAILABLE.lock() = None;
        CHECKED.store(true, Ordering::Relaxed);
        let s = status();
        assert!(s.checked);
        assert_eq!(s.available, None);

        // Có bản mới.
        *AVAILABLE.lock() = Some("9.9.9".into());
        let s = status();
        assert!(s.checked);
        assert_eq!(s.available.as_deref(), Some("9.9.9"));
        assert_eq!(s.current, current_version());

        reset();
    }
}
