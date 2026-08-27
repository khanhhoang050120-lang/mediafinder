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

/// Nhịp thử lại khi hỏi máy chủ thất bại, tính bằng giây.
///
/// Ứng dụng khởi động cùng Windows *trước khi* mạng kịp kết nối, nên cú hỏi
/// đầu tiên gần như được sắp đặt để thất bại. Bản đầu của phần này hỏi đúng
/// một lần rồi thôi — nghĩa là máy nào bật lên là trượt thông báo cập nhật
/// cho tới lần khởi động kế tiếp, và với một ứng dụng ngồi ở khay hàng tuần
/// thì "lần kế tiếp" xa vô kể. Giãn dần rồi dừng ở mười phút: máy offline
/// thật sự chỉ tốn một yêu cầu mỗi mười phút, còn máy vừa nối mạng xong thì
/// biết tin trong vòng nửa phút.
const RETRY_DELAYS_SECS: &[u64] = &[30, 60, 120, 300, 600];

/// Hỏi lại sau chừng này khi lần trước ĐÃ trả lời được.
///
/// Một phiên app sống nhiều ngày ở khay; bản phát hành ra lúc nào không ai
/// hẹn trước. Một ngày một câu hỏi là đủ để không ai bị bỏ lại quá lâu.
const RECHECK_SECS: u64 = 24 * 60 * 60;

/// Delay cho lượt thử `n` (đếm từ 0), kẹp ở mức cuối bảng.
fn retry_delay_secs(attempt: usize) -> u64 {
    *RETRY_DELAYS_SECS
        .get(attempt)
        .unwrap_or(RETRY_DELAYS_SECS.last().expect("bảng nhịp không rỗng"))
}

/// Hỏi máy chủ xem có bản mới không, chạy nền — và hỏi cho tới khi có câu
/// trả lời.
///
/// Gọi lúc khởi động. Không chặn: mạng có thể chậm hoặc không có, và tìm kiếm
/// phải dùng được ngay dù việc này chưa xong. Lỗi không cản trở việc chính
/// của ứng dụng — nhưng cũng không được phép là dấu chấm hết: thất bại thì
/// thử lại theo [`RETRY_DELAYS_SECS`], trả lời được rồi thì một ngày hỏi lại
/// một lần cho phiên sống dài ngày ở khay.
pub fn check_in_background(app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("update-check".into())
        .spawn(move || {
            use tauri_plugin_updater::UpdaterExt;
            let mut failures = 0usize;
            loop {
                let updater = match app.updater() {
                    Ok(u) => u,
                    Err(e) => {
                        tracing::warn!("không dựng được updater: {e}");
                        return;
                    }
                };

                let wait = match tauri::async_runtime::block_on(updater.check()) {
                    Ok(found) => {
                        let version = found.map(|u| u.version.clone());
                        let had_news = version.is_some();
                        record(&app, version);
                        failures = 0;
                        if had_news {
                            // Cửa sổ có thể đang mở sẵn từ trước khi tin về —
                            // báo cho nó thay vì chờ lần mở kế tiếp.
                            use tauri::Emitter;
                            let _ = app.emit("update-available", ());
                        }
                        RECHECK_SECS
                    }
                    // Không có mạng là trường hợp thường gặp nhất ở đây, nhất
                    // là ngay sau đăng nhập. Ghi log và hẹn lượt sau.
                    Err(e) => {
                        let d = retry_delay_secs(failures);
                        failures += 1;
                        tracing::info!("không kiểm tra được cập nhật (thử lại sau {d}s): {e}");
                        d
                    }
                };
                std::thread::sleep(std::time::Duration::from_secs(wait));
            }
        })
        .expect("spawn update-check");
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
    fn nhip_thu_lai_gian_dan_roi_dung_o_muc_cuoi() {
        let _g = SERIAL.lock();
        assert_eq!(retry_delay_secs(0), 30);
        assert_eq!(retry_delay_secs(1), 60);
        assert_eq!(retry_delay_secs(4), 600);
        // Qua het bang thi ket o muc cuoi — may offline lau ngay khong duoc
        // phep leo thang vo han, cung khong duoc phep dung hoi.
        assert_eq!(retry_delay_secs(5), 600);
        assert_eq!(retry_delay_secs(500), 600);
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
