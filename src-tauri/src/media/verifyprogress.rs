//! Tiến độ của một lượt xác minh tầng 3 — để màn hình thôi đứng im.
//!
//! # Vì sao cần một module cho việc tưởng như vặt
//!
//! [`crate::media::verify::verify_paths`] chạy một mạch rồi mới trả kết quả.
//! Với một nhóm nằm hết trên ổ trong máy thì không sao — vài giây. Nhưng nhóm
//! trong ảnh người dùng gửi là **bốn tệp 11,2 GB**, một bản nằm trên ổ mạng:
//! xác minh nó phải kéo khoảng **45 GB**, trong đó hơn 11 GB qua NAS.
//!
//! Suốt quãng ấy giao diện chỉ hiện đúng ba chữ "đang xác minh…" đứng im. Người
//! dùng không phân biệt được *"đang đọc, cứ chờ"* với *"treo rồi"* — và cách
//! duy nhất để họ thử là bỏ đi rồi bấm lại, tức vứt bỏ toàn bộ phần đã đọc.
//!
//! # Vì sao là trạng thái dùng chung + poll, không phải sự kiện
//!
//! Vì tầng 2 đã làm đúng như vậy và nó chạy tốt: [`crate::media::dupes`] giữ
//! một chùm biến nguyên tử, giao diện hỏi mỗi 400 ms. Cùng một bài toán thì
//! dùng cùng một lời giải — thêm một cơ chế thứ hai (kênh sự kiện) chỉ để làm
//! việc y hệt là bắt người đọc mã sau này phải học hai thứ thay vì một.
//!
//! Poll còn có một tính chất mà sự kiện không có: cửa sổ đóng rồi mở lại vẫn
//! đọc được trạng thái hiện tại. Sự kiện phát ra lúc không ai nghe thì mất.
//!
//! # Đơn vị là BYTE, không phải số tệp
//!
//! Đếm theo tệp thì thanh tiến độ nhảy 0% → 25% → 50% và đứng im hàng phút
//! giữa mỗi bậc — với bốn tệp 11,2 GB nó tệ hơn không có gì, vì nó *hứa* một
//! độ mịn mà nó không có. Byte cho một con số nhích đều, và đó cũng là thứ
//! phản ánh đúng công việc thật: một tệp 11 GB nặng gấp trăm lần một tệp 100 MB.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;

/// Tiến độ một lượt xác minh, đọc được từ giao diện.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct VerifyProgress {
    /// Có lượt xác minh nào đang chạy không.
    pub running: bool,
    /// Tổng số byte phải đọc — tổng dung lượng mọi tệp trong nhóm.
    ///
    /// Lấy từ `metadata()` trước khi đọc, nên nó là con số thật chứ không
    /// phải ước lượng. Bằng 0 khi chưa đo được (tệp đã biến mất, NAS rớt).
    pub total_bytes: u64,
    /// Đã đọc xong bao nhiêu byte.
    pub done_bytes: u64,
    /// Tổng số tệp trong nhóm đang đối chiếu.
    ///
    /// KHÔNG còn `file_index` đi kèm: [`crate::media::verifyfast::doi_chieu`]
    /// đọc **song hành cả nhóm** — cùng một đoạn ở mọi tệp, rồi tách cụm —
    /// chứ không đi lần lượt từng tệp. Trong lối đọc đó không tồn tại "đang ở
    /// tệp thứ mấy", và cố báo một con số như vậy thì nó đứng im ở 0 suốt
    /// lượt. Đã lộ ra trên app thật: giao diện hiện "tệp 0/3".
    pub file_count: usize,
}

impl VerifyProgress {
    /// Phần trăm đã xong, làm tròn xuống, chặn trong 0..=100.
    ///
    /// Trả `None` khi chưa biết tổng — giao diện phải nói "đang đọc…" chứ
    /// không được vẽ một thanh 0% trông như đang đứng im.
    pub fn percent(&self) -> Option<u8> {
        if self.total_bytes == 0 {
            return None;
        }
        let p = self.done_bytes.saturating_mul(100) / self.total_bytes;
        Some(p.min(100) as u8)
    }
}

/// Trạng thái dùng chung giữa luồng xác minh và giao diện.
///
/// Clone rẻ: mọi trường đều là `Arc`. `DupeService` cũng dựng theo lối này.
#[derive(Clone, Default)]
pub struct VerifyState {
    running: Arc<AtomicBool>,
    total_bytes: Arc<AtomicU64>,
    done_bytes: Arc<AtomicU64>,
    file_count: Arc<AtomicU64>,
    /// Giương lên để xin luồng xác minh dừng giữa chừng.
    ///
    /// Một lượt 45 GB qua NAS chạy nhiều phút; người dùng đổi ý mà không có
    /// đường dừng thì đĩa cứ quay cho tới hết để ra một câu trả lời không ai
    /// còn muốn nghe. Cùng lý do `DupeService` có `stop`.
    stop: Arc<AtomicBool>,
}

impl VerifyState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bắt đầu một lượt. Trả `false` nếu đang có lượt khác chạy.
    ///
    /// Từ chối thay vì xếp hàng: hai lượt cùng đọc một ổ chỉ đổi tuần tự lấy
    /// tiếng lạch cạch, và giao diện chỉ hiện được một thanh tiến độ.
    pub fn begin(&self, file_count: usize) -> bool {
        if self.running.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.stop.store(false, Ordering::SeqCst);
        self.total_bytes.store(0, Ordering::Relaxed);
        self.done_bytes.store(0, Ordering::Relaxed);
        self.file_count.store(file_count as u64, Ordering::Relaxed);
        true
    }

    /// Đặt tổng số byte sau khi đã đo xong cả nhóm.
    pub fn set_total(&self, bytes: u64) {
        self.total_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Cộng thêm số byte vừa đọc được.
    pub fn add_done(&self, bytes: u64) {
        self.done_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn finish(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Xin dừng. Chỉ giương cờ rồi trả về ngay — luồng đang đọc sẽ thấy nó ở
    /// lần lặp kế tiếp.
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    pub fn cancelled(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    pub fn snapshot(&self) -> VerifyProgress {
        VerifyProgress {
            running: self.running.load(Ordering::SeqCst),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            done_bytes: self.done_bytes.load(Ordering::Relaxed),
            file_count: self.file_count.load(Ordering::Relaxed) as usize,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chua_do_duoc_tong_thi_khong_bao_phan_tram() {
        // Vẽ một thanh 0% khi chưa biết tổng là nói dối theo hướng tệ nhất:
        // trông y hệt một lượt đang treo.
        let p = VerifyProgress {
            total_bytes: 0,
            done_bytes: 0,
            ..Default::default()
        };
        assert_eq!(p.percent(), None);
    }

    #[test]
    fn phan_tram_lam_tron_xuong() {
        let p = VerifyProgress {
            total_bytes: 3,
            done_bytes: 2,
            ..Default::default()
        };
        // 66,6% → 66. Làm tròn LÊN sẽ cho ra 100% khi còn đang đọc dở, và
        // "100% mà vẫn quay" là thứ làm người dùng mất tin vào mọi thanh
        // tiến độ về sau.
        assert_eq!(p.percent(), Some(66));
    }

    #[test]
    fn khong_bao_gio_vuot_qua_100() {
        // Tệp nở ra giữa lượt đọc (đang được ghi tiếp) thì done > total.
        let p = VerifyProgress {
            total_bytes: 100,
            done_bytes: 250,
            ..Default::default()
        };
        assert_eq!(p.percent(), Some(100));
    }

    #[test]
    fn hai_luot_cung_luc_thi_luot_sau_bi_tu_choi() {
        let s = VerifyState::new();
        assert!(s.begin(4), "luot dau phai duoc chay");
        assert!(!s.begin(2), "luot thu hai phai bi tu choi");
        s.finish();
        assert!(s.begin(2), "xong roi thi luot moi phai chay duoc");
    }

    #[test]
    fn begin_xoa_sach_so_lieu_cua_luot_truoc() {
        // Không xoá thì lượt sau bắt đầu ở 100% của lượt trước — thanh tiến độ
        // đầy sẵn rồi tụt về, trông như phần mềm hỏng.
        let s = VerifyState::new();
        s.begin(2);
        s.set_total(1000);
        s.add_done(1000);
        s.finish();

        s.begin(3);
        let p = s.snapshot();
        assert_eq!(p.done_bytes, 0);
        assert_eq!(p.total_bytes, 0);
        assert_eq!(p.file_count, 3);
    }

    #[test]
    fn co_dung_giuong_len_va_doc_duoc() {
        let s = VerifyState::new();
        s.begin(1);
        assert!(!s.cancelled());
        s.cancel();
        assert!(s.cancelled());
        // Lượt mới phải xoá cờ dừng, nếu không nó chết ngay khi vừa sinh ra.
        s.finish();
        s.begin(1);
        assert!(!s.cancelled(), "luot moi van mang co dung cua luot truoc");
    }

    #[test]
    fn snapshot_phan_anh_dung_tien_do() {
        let s = VerifyState::new();
        s.begin(4);
        s.set_total(12_000_000_000);
        s.add_done(3_000_000_000);
        let p = s.snapshot();
        assert!(p.running);
        assert_eq!(p.file_count, 4);
        assert_eq!(p.percent(), Some(25));
    }
}
