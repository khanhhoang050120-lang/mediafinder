//! Phím tắt toàn cục, và phương án dự phòng khi tổ hợp đầu bị chiếm.
//!
//! # Vấn đề
//!
//! Trước module này app thử **đúng một** tổ hợp `Ctrl+Alt+Space`, thất bại thì
//! bỏ cuộc — và nói với người dùng:
//!
//! > *"đang bị ứng dụng khác chiếm — đóng ứng dụng đó rồi mở lại MediaFinder"*
//!
//! Lời khuyên đó gần như không dùng được. Ứng dụng chiếm phím thường là thứ
//! người ta cần chạy suốt ngày — bộ gõ tiếng Việt, phần mềm chụp màn hình,
//! công cụ của studio. "Đóng nó đi" không phải một lựa chọn, nên người dùng
//! mất hẳn phím tắt: đúng thứ chính để gọi cửa sổ, vì app khởi động ẩn.
//!
//! # Cách làm
//!
//! Thử lần lượt một danh sách, lấy tổ hợp đầu tiên đăng ký được. Phím tắt toàn
//! cục là tài nguyên của cả hệ điều hành — hoặc giành được, hoặc không — nên
//! không có cách nào biết trước ngoài việc thử.
//!
//! Tự chọn chứ không hỏi người dùng, vì phần mềm này chạy trên 20–40 máy của
//! studio: một tuỳ chọn thủ công nghĩa là ai đó phải đi đặt trên từng máy.

use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

/// Các tổ hợp sẽ thử, theo thứ tự.
///
/// `Ctrl+Alt+Space` đứng đầu vì nó là tổ hợp app đã dùng từ đầu — người quen
/// rồi thì không nên bị đổi.
///
/// Vì sao không có tổ hợp ngắn hơn: phím tắt toàn cục **lấy đi** tổ hợp đó
/// khỏi mọi ứng dụng khác trên máy, nên nó phải là thứ ít ai muốn. `Alt+Space`
/// là menu hệ thống của Windows, còn `Ctrl+Space` thuộc về việc chuyển bộ gõ ở
/// nhiều ngôn ngữ — **kể cả tiếng Việt**, thứ mọi người ở đây dùng hằng ngày.
///
/// Ba tổ hợp dự phòng chọn theo cùng nguyên tắc: đều ba phím, đều không đụng
/// tổ hợp hệ thống nào, và chữ cái gợi được cái tên — `F` cho *find*, `M` cho
/// *media*.
pub const CANDIDATES: &[&str] = &[
    "Ctrl+Alt+Space",
    "Ctrl+Alt+F",
    "Ctrl+Shift+Space",
    "Ctrl+Alt+M",
];

/// Tổ hợp đang thật sự dùng. Rỗng nghĩa là không giành được cái nào.
///
/// `Mutex<String>` chứ không phải hằng số, vì giá trị chỉ biết được lúc chạy —
/// và mọi chỗ hiện nó ra (khay hệ thống, tooltip, chân cửa sổ) phải đọc cùng
/// một nguồn. Một chỗ nào đó viết cứng `Ctrl+Alt+Space` là chỗ đó sẽ nói dối
/// ngay khi phải dùng phím dự phòng.
static IN_USE: Mutex<String> = Mutex::new(String::new());

/// Có giành được phím tắt nào không.
///
/// Giữ riêng bên cạnh [`IN_USE`] để chỗ chỉ cần biết có/không khỏi phải khoá.
pub static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Tổ hợp đang dùng, hoặc chuỗi rỗng nếu không có.
pub fn in_use() -> String {
    IN_USE.lock().clone()
}

/// Tổ hợp đầu danh sách — cái app *muốn* dùng.
///
/// Cần cho câu thông báo "X đang bị chiếm, đang dùng Y thay thế": nói được
/// cái mất thì người dùng mới hiểu vì sao phím quen của họ không còn.
pub fn preferred() -> &'static str {
    CANDIDATES[0]
}

/// Đang phải dùng phím dự phòng, tức tổ hợp ưu tiên đã bị chiếm.
pub fn is_fallback() -> bool {
    let cur = in_use();
    !cur.is_empty() && cur != preferred()
}

/// Ghi lại tổ hợp đã giành được. Chuỗi rỗng nghĩa là thất bại cả loạt.
pub fn set_in_use(combo: &str) {
    *IN_USE.lock() = combo.to_string();
    ACTIVE.store(!combo.is_empty(), Ordering::Relaxed);
}

/// Thử lần lượt và trả về tổ hợp đầu tiên đăng ký được.
///
/// Tách khỏi phần gọi Tauri để kiểm thử được: `try_one` trả `true` nếu giành
/// được. Bài kiểm thử đưa vào một hàm giả mô tả tình huống "hai phần mềm cùng
/// đòi một phím" mà không cần một hệ điều hành thật.
pub fn pick(mut try_one: impl FnMut(&str) -> bool) -> Option<&'static str> {
    CANDIDATES.iter().copied().find(|c| try_one(c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn lay_to_hop_dau_tien_gianh_duoc() {
        let da_thu = RefCell::new(Vec::new());
        let got = pick(|c| {
            da_thu.borrow_mut().push(c.to_string());
            true
        });
        assert_eq!(got, Some("Ctrl+Alt+Space"));
        // Giành được ngay thì DỪNG — không đăng ký thừa ba tổ hợp nữa, vì mỗi
        // tổ hợp giành được là một tổ hợp lấy khỏi mọi ứng dụng khác.
        assert_eq!(da_thu.borrow().len(), 1);
    }

    #[test]
    fn to_hop_dau_bi_chiem_thi_xuong_cai_ke_tiep() {
        let got = pick(|c| c != "Ctrl+Alt+Space");
        assert_eq!(got, Some("Ctrl+Alt+F"));
    }

    #[test]
    fn thu_het_ca_loat_theo_dung_thu_tu() {
        let da_thu = RefCell::new(Vec::new());
        let got = pick(|c| {
            da_thu.borrow_mut().push(c.to_string());
            false
        });
        assert_eq!(got, None, "không giành được cái nào thì phải trả None");
        assert_eq!(*da_thu.borrow(), CANDIDATES);
    }

    #[test]
    fn chi_con_to_hop_cuoi_van_dung() {
        let got = pick(|c| c == "Ctrl+Alt+M");
        assert_eq!(got, Some("Ctrl+Alt+M"));
    }

    #[test]
    fn danh_sach_khong_co_to_hop_trung_nhau() {
        // Trùng nhau thì lần thử thứ hai chắc chắn thất bại (chính app đang
        // giữ tổ hợp đó), và người dùng mất một phương án dự phòng mà không ai
        // biết.
        let mut v: Vec<&str> = CANDIDATES.to_vec();
        v.sort_unstable();
        let truoc = v.len();
        v.dedup();
        assert_eq!(v.len(), truoc, "danh sách có tổ hợp trùng nhau");
    }

    #[test]
    fn khong_to_hop_nao_dung_mot_minh_ctrl_space_hay_alt_space() {
        // `Alt+Space` là menu hệ thống của Windows; `Ctrl+Space` là chuyển bộ
        // gõ ở nhiều ngôn ngữ, kể cả tiếng Việt. Giành mất một trong hai là
        // lấy đi thứ người dùng cần hằng ngày để đổi lấy một phím tắt tiện.
        for c in CANDIDATES {
            assert_ne!(*c, "Alt+Space");
            assert_ne!(*c, "Ctrl+Space");
        }
    }

    #[test]
    fn nhan_ra_khi_dang_dung_phim_du_phong() {
        set_in_use("");
        assert!(!is_fallback(), "không có phím tắt thì không phải dự phòng");
        assert!(!ACTIVE.load(Ordering::Relaxed));

        set_in_use(preferred());
        assert!(
            !is_fallback(),
            "đúng tổ hợp ưu tiên thì không phải dự phòng"
        );
        assert!(ACTIVE.load(Ordering::Relaxed));

        set_in_use("Ctrl+Alt+F");
        assert!(is_fallback());
        assert_eq!(in_use(), "Ctrl+Alt+F");

        // Trả về trạng thái sạch cho các bài khác — `IN_USE` là biến toàn cục.
        set_in_use("");
    }
}
