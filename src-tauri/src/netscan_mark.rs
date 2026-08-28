//! Dấu vết lần quét ổ mạng gần nhất — để giao diện hỏi trước khi quét lại.
//!
//! Nút "+ ổ mạng" không mang trạng thái: bấm lần nào cũng chạy lại trọn cả
//! hai giai đoạn, tốn vài phút và tranh băng thông mạng. Người dùng bấm lần
//! hai vì tưởng nó là một nút khác, rồi ngồi chờ một việc mình không định
//! làm. Hộp thoại xác nhận cần nói được **lần trước quét lúc nào, ra bao
//! nhiêu tệp** — không có hai con số đó thì lời hỏi rỗng tuếch.
//!
//! Ghi ra một tệp JSON nhỏ cạnh cache chứ **không** nhét vào `index.bin`:
//! đụng vào định dạng chỉ mục là phải nâng `SCHEMA_VERSION`, mà lời hứa ghi
//! ngay cạnh hằng số đó nói rõ chỉ được nâng ở bản major. Một dấu vết tiện
//! nghi không đáng để bẻ gãy lời hứa ấy — và mất tệp này thì cùng lắm là hộp
//! thoại nói "chưa rõ lần trước", không ai mất dữ liệu gì.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Lần quét ổ mạng gần nhất đã hoàn tất.
///
/// **`#[serde(default)]` không phải trang trí.** `load()` là
/// `serde_json::from_str(&raw).ok()?`, nên một trường mới **bắt buộc** sẽ làm
/// mọi `netscan.json` đã nằm sẵn trên 20–40 máy đọc ra `None` — nghĩa là mốc
/// "ổ mạng quét lần cuối" biến mất khỏi giao diện đúng vào bản phát hành thêm
/// trường đó. Không nổ, nhưng hỏng đúng thứ vừa mới xây. Với `default`, tệp
/// cũ vẫn đọc được và trường mới nhận giá trị mặc định.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NetScanMark {
    /// Unix seconds lúc quét xong.
    pub at_unix: i64,
    /// Số tệp media tìm được trên ổ mạng.
    pub files: usize,
    /// Số ổ mạng đã đi qua.
    pub drives: usize,
    /// Lượt quét mất bao lâu — để hộp thoại nói được "mất khoảng 4 phút".
    pub seconds: f64,
}

/// Ghi đè thư mục dữ liệu trong kiểm thử.
static DIR_OVERRIDE: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

fn mark_path() -> Option<PathBuf> {
    if let Some(d) = DIR_OVERRIDE.lock().expect("netscan mark lock").clone() {
        return Some(d.join("netscan.json"));
    }
    let cache = crate::index::persist::cache_path().ok()?;
    Some(cache.parent()?.join("netscan.json"))
}

/// Đọc dấu vết. `None` khi chưa từng quét, hoặc tệp hỏng/mất — cả hai đều
/// dẫn tới cùng một câu trả lời trung thực: "không biết".
pub fn load() -> Option<NetScanMark> {
    let raw = std::fs::read_to_string(mark_path()?).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Ghi dấu vết sau một lượt quét ổ mạng **đã hoàn tất**.
///
/// Lượt bị huỷ giữa chừng không được ghi: nó không phải một câu trả lời, và
/// nói "đã quét lúc 14:32" về một lượt dừng dở là nói dối.
pub fn save(files: usize, drives: usize, seconds: f64) {
    let Some(path) = mark_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let at_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mark = NetScanMark {
        at_unix,
        files,
        drives,
        seconds,
    };
    match serde_json::to_string(&mark) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("không ghi được dấu vết quét ổ mạng: {e}");
            }
        }
        Err(e) => tracing::warn!("không mã hoá được dấu vết quét ổ mạng: {e}"),
    }
}

/// Ghi dấu vết cho một lượt quét vừa kết thúc — **điểm quyết định duy nhất**.
///
/// Tách khỏi `save` để chính quy tắc "chỉ lượt đi trọn mới được ghi" có một
/// chỗ trú ngụ mà kiểm thử gọi tới được. Nếu quy tắc nằm rải trong `if` ở
/// `lib.rs`, bài kiểm thử chỉ mô phỏng lại được nó — và một mô phỏng thì
/// không bao giờ đỏ khi bản thật bị sửa.
pub fn record_outcome(outcome: &crate::NetworkScanOutcome) {
    if outcome.cancelled || outcome.files == 0 {
        return;
    }
    save(outcome.files, outcome.drives, outcome.seconds);
}

#[cfg(test)]
mod tests {
    use super::*;

    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mf-netmark-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        *DIR_OVERRIDE.lock().unwrap() = Some(dir.clone());
        dir
    }

    #[test]
    fn chua_quet_bao_gio_thi_khong_biet() {
        let _g = SERIAL.lock();
        let dir = sandbox("empty");
        assert_eq!(load(), None, "chua co dau vet ma da tra loi gi do");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ghi_roi_doc_lai_dung_nhu_cu() {
        let _g = SERIAL.lock();
        let dir = sandbox("roundtrip");
        save(313_945, 2, 271.5);
        let m = load().expect("vua ghi ma doc khong ra");
        assert_eq!(m.files, 313_945);
        assert_eq!(m.drives, 2);
        assert!((m.seconds - 271.5).abs() < 0.01);
        assert!(m.at_unix > 1_700_000_000, "dau thoi gian phai la that");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Tệp của bản cũ, thiếu trường, vẫn phải đọc được.
    ///
    /// Đây là bài canh cho `#[serde(default)]`. Bỏ thuộc tính đó đi thì bài này
    /// đỏ — và nếu nó đỏ trên máy phát triển thì nghĩa là trên 20–40 máy ngoài
    /// kia, mọi `netscan.json` viết bởi bản trước sẽ đọc ra `None` ngay khi ai
    /// đó thêm một trường mới. Mốc "ổ mạng quét lần cuối" biến mất khỏi giao
    /// diện đúng vào bản phát hành thêm trường ấy: không nổ, nhưng hỏng lặng lẽ
    /// đúng thứ vừa xây.
    #[test]
    fn tep_thieu_truong_van_doc_duoc_thay_vi_mat_trang() {
        let _g = SERIAL.lock();
        let dir = sandbox("forward-compat");
        // Đúng hình dạng một tệp do bản cũ hơn ghi ra: chỉ có hai trường.
        std::fs::write(
            dir.join("netscan.json"),
            br#"{"atUnix":1787890985,"files":320528}"#,
        )
        .unwrap();
        let m = load().expect("tep cua ban cu phai van doc duoc");
        assert_eq!(m.at_unix, 1_787_890_985);
        assert_eq!(m.files, 320_528);
        assert_eq!(m.drives, 0, "truong thieu phai nhan gia tri mac dinh");
        assert_eq!(m.seconds, 0.0, "truong thieu phai nhan gia tri mac dinh");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tep_hong_thi_tra_loi_khong_biet_chu_khong_no() {
        let _g = SERIAL.lock();
        let dir = sandbox("corrupt");
        std::fs::write(dir.join("netscan.json"), b"{khong phai json").unwrap();
        assert_eq!(load(), None, "tep hong phai dan toi 'khong biet'");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Bất biến của điểm gọi: chỉ lượt đi trọn mới để lại dấu vết.
    ///
    /// Gọi thẳng `record_outcome` — chính hàm mà `lib.rs` dùng, chứ không mô
    /// phỏng lại điều kiện của nó: một lượt bị huỷ không phải câu trả lời, và nói
    /// "đã quét lúc 14:32" về nó là nói dối với người đang quyết định có nên
    /// bỏ ra vài phút nữa hay không.
    #[test]
    fn luot_bi_huy_khong_de_lai_dau_vet() {
        let _g = SERIAL.lock();
        let dir = sandbox("cancelled");

        let cancelled = crate::NetworkScanOutcome {
            drives: 2,
            files: 999,
            seconds: 30.0,
            cancelled: true,
        };
        record_outcome(&cancelled);
        assert_eq!(load(), None, "luot bi huy da de lai dau vet");

        // Lượt không tìm được gì cũng không phải một câu trả lời đáng khoe.
        record_outcome(&crate::NetworkScanOutcome {
            files: 0,
            cancelled: false,
            ..cancelled
        });
        assert_eq!(load(), None, "luot khong ra tep nao van ghi dau vet");

        record_outcome(&crate::NetworkScanOutcome {
            cancelled: false,
            ..cancelled
        });
        assert_eq!(
            load().map(|m| m.files),
            Some(999),
            "luot di tron phai duoc ghi"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ghi_de_lan_sau_thay_lan_truoc() {
        let _g = SERIAL.lock();
        let dir = sandbox("overwrite");
        save(100, 1, 10.0);
        save(200, 2, 20.0);
        let m = load().unwrap();
        assert_eq!(m.files, 200, "dau vet phai la lan gan nhat");
        let _ = std::fs::remove_dir_all(dir);
    }
}
