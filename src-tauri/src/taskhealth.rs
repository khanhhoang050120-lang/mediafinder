//! Cỗ máy làm mới chỉ mục còn sống hay không — và nói ra khi nó chết.
//!
//! Mọi thứ trong ứng dụng đều ngầm giả định rằng tác vụ định kỳ đang chạy: chỉ
//! mục tự tươi, "Quét lại" không hỏi quyền, tệp mới tự hiện ra. BUG-024 vừa
//! chứng minh giả định đó **sai trên máy người dùng thật** — móc gỡ cài đặt
//! xoá luôn tác vụ khi có ai cài tay đè lên bản cũ, và không một dấu hiệu nào
//! trên màn hình nói cho họ biết.
//!
//! Hậu quả cụ thể, đã truy trong mã: trên máy đã mất tác vụ,
//! [`setup::upgrade_schedule_if_stale`] thoát ngay ở `!scheduled_task_exists()`,
//! và [`setup::ensure_scheduled_task`] chỉ được gọi bên trong `run_indexer` —
//! tức chỉ chạy trong tiến trình `--index` nâng quyền, mà tiến trình ấy lại do
//! chính tác vụ đã mất khởi động. Vòng lặp khép kín: **máy đó không còn đường
//! tự làm mới nào cả** cho tới khi có người bấm "Quét lại" và chấp nhận một
//! hộp thoại UAC. Cài bản mới lên cũng không gỡ được, vì không ai biết mà bấm.
//!
//! Nên module này chỉ làm đúng một việc: trả lời "tác vụ còn đó không", để
//! giao diện nói ra thành một câu và chỉ đường bấm. Nó **không** tự sửa —
//! tạo lại tác vụ cần quyền cao, và giành quyền mà người dùng không yêu cầu
//! là việc khác hẳn với việc nói cho họ biết.
//!
//! Cố ý **không** phân tích đầu ra `schtasks /Query /FO LIST /V` để lấy "lần
//! chạy gần nhất" và "kết quả gần nhất": tên các trường trong đầu ra đó được
//! bản địa hoá theo ngôn ngữ hiển thị của Windows, nên trên một máy chạy
//! Windows tiếng Việt phép so chuỗi sẽ trượt — và im lặng đúng trên những máy
//! cần chẩn đoán nhất. Câu hỏi "task có tồn tại không" thì trả lời được bằng
//! mã thoát, không cần đọc chữ nào.

use serde::{Deserialize, Serialize};

/// Tình trạng đường làm mới chỉ mục, gọn đủ để giao diện nói một câu.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TaskHealth {
    /// Tác vụ định kỳ có tồn tại trên máy này không.
    pub task_exists: bool,
}

/// Hỏi Windows xem tác vụ còn đó không.
///
/// Chỉ dựa vào mã thoát của `schtasks /Query`, nên miễn nhiễm với ngôn ngữ
/// hiển thị và với mã hoá đầu ra — hai thứ đã làm hỏng một chốt chặn trước đây
/// (xem `SCHEDULE_MARK` trong `setup.rs`).
pub fn check() -> TaskHealth {
    TaskHealth {
        task_exists: crate::setup::scheduled_task_exists(),
    }
}

/// Lệnh cho giao diện.
///
/// **Gọi thưa thôi.** Mỗi lượt sinh một tiến trình `schtasks.exe` (đo được
/// khoảng vài chục mili giây), nên chỗ gọi đúng là lúc mở cửa sổ và sau mỗi
/// lượt quét — tuyệt đối không phải mỗi lần gõ phím trong ô tìm kiếm.
#[tauri::command]
pub fn task_health() -> TaskHealth {
    check()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hình dạng gửi sang giao diện phải ổn định.
    ///
    /// Tên trường ở đây là hợp đồng với `src/lib/search.ts`; đổi nó mà quên
    /// phía kia thì giao diện đọc ra `undefined` và câu cảnh báo im lặng biến
    /// mất — đúng kiểu hỏng mà module này sinh ra để chống.
    #[test]
    fn hinh_dang_gui_sang_giao_dien_la_camel_case() {
        let json = serde_json::to_string(&TaskHealth { task_exists: true }).unwrap();
        assert_eq!(json, r#"{"taskExists":true}"#);
    }

    /// Tệp/JSON thiếu trường vẫn phải đọc được, cùng lý do như `NetScanMark`:
    /// thêm trường mới không được làm bản cũ mất trắng.
    #[test]
    fn thieu_truong_thi_nhan_mac_dinh_chu_khong_no() {
        let h: TaskHealth = serde_json::from_str("{}").unwrap();
        assert!(!h.task_exists, "mac dinh phai la 'khong biet co' = false");
    }

    /// Trên máy phát triển, `check()` phải chạy được và trả lời dứt khoát —
    /// không treo, không hoảng. Giá trị đúng/sai tuỳ máy nên không khẳng định.
    #[test]
    fn check_chay_duoc_tren_may_that() {
        let _ = check();
    }
}
