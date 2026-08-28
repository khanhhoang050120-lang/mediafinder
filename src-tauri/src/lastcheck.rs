//! "Lần cuối cùng có người đi kiểm tra là khi nào" — khác với "lần cuối cùng
//! chỉ mục thay đổi".
//!
//! # Vì sao cần một mốc riêng
//!
//! Giao diện muốn trả lời một câu rất đơn giản: *chỉ mục này còn đáng tin
//! không*. Mốc duy nhất nó có trước đây là `built_at_unix` trong `index.bin`.
//! Nhưng mốc ấy chỉ được đóng dấu trong [`crate::index::persist::save`], mà
//! `run_incremental` **cố ý không ghi lại cache khi không có gì đổi** — ghi
//! 47 MB mỗi 15 phút chỉ để đẩy một con trỏ là khoảng 4,5 GB xuống SSD mỗi
//! ngày cho việc không làm gì.
//!
//! Hệ quả: một máy hoàn toàn khoẻ, tác vụ vừa chạy hai phút trước, nhưng vì
//! buổi tối không ai đụng vào tệp nào nên `built_at_unix` vẫn là mốc lúc chiều
//! — và giao diện tô vàng *"Ổ trong máy: 4 giờ trước"* ngay trên màn hình
//! "Không tìm thấy kết quả nào". Một cảnh báo sai đúng vào lúc người dùng đang
//! cố hiểu vì sao không ra kết quả là tệ hơn không cảnh báo gì: nó chỉ họ đi
//! sai hướng, và sau vài lần thì họ thôi đọc mọi cảnh báo.
//!
//! Hai câu hỏi khác nhau cần hai con số khác nhau:
//!
//! * `built_at_unix` — **chỉ mục thay đổi lần cuối lúc nào**. Đúng cho việc
//!   biết dữ liệu cũ tới đâu *khi có thay đổi bị bỏ lỡ*.
//! * mốc ở đây — **cỗ máy làm mới chạy lần cuối lúc nào**. Đúng cho việc biết
//!   nó còn sống hay đã chết. Đây là con số giao diện thật sự cần.
//!
//! # Vì sao là tệp JSON nhỏ, không nhét vào `index.bin`
//!
//! Đụng vào định dạng chỉ mục là phải nâng `SCHEMA_VERSION`, mà lời hứa ghi
//! ngay cạnh hằng số đó nói rõ chỉ được nâng ở bản major. Và một mốc ghi mỗi
//! 15 phút mà nằm trong tệp 47 MB thì lại đúng vào cái bẫy vừa tránh được.
//! Tệp này vài chục byte; mất nó thì cùng lắm giao diện nói "chưa rõ".
//!
//! Cùng lối với `netscan_mark.rs`, và mang sẵn `#[serde(default)]` vì lý do
//! đã trả giá một lần ở đó: thêm trường mới không được làm tệp cũ đọc ra
//! `None` trên hai mươi tới bốn mươi máy.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Dấu vết lượt kiểm tra gần nhất của tiến trình làm mới.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LastCheck {
    /// Unix seconds lúc lượt kiểm kết thúc.
    pub at_unix: i64,
    /// Lượt đó có đổi gì trong chỉ mục không. Giao diện chưa dùng tới, nhưng
    /// nó là thứ phân biệt "đã kiểm và không có gì mới" với "đã kiểm và có" —
    /// và đó chính là câu hỏi sinh ra module này.
    pub changed: bool,
}

/// Ghi đè thư mục dữ liệu trong kiểm thử.
static DIR_OVERRIDE: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

fn mark_path() -> Option<PathBuf> {
    if let Some(d) = DIR_OVERRIDE.lock().expect("lastcheck lock").clone() {
        return Some(d.join("lastcheck.json"));
    }
    let cache = crate::index::persist::cache_path().ok()?;
    Some(cache.parent()?.join("lastcheck.json"))
}

/// Đọc dấu vết. `None` khi chưa từng kiểm, hoặc tệp hỏng — cả hai dẫn tới cùng
/// một câu trả lời trung thực: "không biết".
pub fn load() -> Option<LastCheck> {
    let raw = std::fs::read_to_string(mark_path()?).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Đóng dấu một lượt kiểm vừa xong.
///
/// Gọi ở **mọi** lượt chạy xong xuôi, kể cả lượt không đổi gì — đó là toàn bộ
/// lý do module này tồn tại.
pub fn record(changed: bool) {
    let Some(path) = mark_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let at_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mark = LastCheck { at_unix, changed };
    match serde_json::to_string(&mark) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("không ghi được mốc lần kiểm gần nhất: {e}");
            }
        }
        Err(e) => tracing::warn!("không mã hoá được mốc lần kiểm gần nhất: {e}"),
    }
}

/// Lệnh cho giao diện.
#[tauri::command]
pub fn last_check() -> Option<LastCheck> {
    load()
}

#[cfg(test)]
mod tests {
    use super::*;

    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mf-lastcheck-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        *DIR_OVERRIDE.lock().unwrap() = Some(dir.clone());
        dir
    }

    #[test]
    fn chua_kiem_bao_gio_thi_khong_biet() {
        let _g = SERIAL.lock();
        let dir = sandbox("empty");
        assert_eq!(load(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Bất biến trung tâm: **lượt không đổi gì vẫn phải để lại dấu vết.**
    ///
    /// Đây đúng là ca mà `built_at_unix` không trả lời được, và là lý do
    /// module này tồn tại. Bỏ lời gọi `record` ở nhánh không-đổi trong
    /// `run_incremental` thì bài này vẫn xanh — nên có thêm một chốt đọc thẳng
    /// mã nguồn ở `tests/refresh_guards.rs`.
    #[test]
    fn luot_khong_doi_gi_van_de_lai_dau_vet() {
        let _g = SERIAL.lock();
        let dir = sandbox("nochange");
        record(false);
        let m = load().expect("luot khong doi gi phai van ghi moc");
        assert!(!m.changed);
        assert!(m.at_unix > 1_700_000_000, "dau thoi gian phai la that");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ghi_de_lan_sau_thay_lan_truoc() {
        let _g = SERIAL.lock();
        let dir = sandbox("overwrite");
        record(false);
        let dau = load().unwrap();
        record(true);
        let sau = load().unwrap();
        assert!(sau.changed, "lan sau phai thay lan truoc");
        assert!(sau.at_unix >= dau.at_unix);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tep_hong_thi_tra_loi_khong_biet_chu_khong_no() {
        let _g = SERIAL.lock();
        let dir = sandbox("corrupt");
        std::fs::write(dir.join("lastcheck.json"), b"{khong phai json").unwrap();
        assert_eq!(load(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Tệp do bản cũ ghi, thiếu trường, vẫn phải đọc được — cùng bài học với
    /// `NetScanMark`.
    #[test]
    fn tep_thieu_truong_van_doc_duoc() {
        let _g = SERIAL.lock();
        let dir = sandbox("forward");
        std::fs::write(dir.join("lastcheck.json"), br#"{"atUnix":1787890985}"#).unwrap();
        let m = load().expect("tep ban cu phai doc duoc");
        assert_eq!(m.at_unix, 1_787_890_985);
        assert!(!m.changed);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn hinh_dang_gui_sang_giao_dien_la_camel_case() {
        let json = serde_json::to_string(&LastCheck {
            at_unix: 5,
            changed: true,
        })
        .unwrap();
        assert_eq!(json, r#"{"atUnix":5,"changed":true}"#);
    }
}
