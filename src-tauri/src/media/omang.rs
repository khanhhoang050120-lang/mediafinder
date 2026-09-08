//! Ổ mạng đang gắn: chữ ổ và máy chủ, hỏi thẳng Windows.
//!
//! # Vì sao đây là một module riêng
//!
//! Trong cùng một ngày, một danh sách ổ mạng **rỗng** gây hai lỗi ở hai chỗ
//! không liên quan gì nhau:
//!
//! * Bài đo trên thư viện thật truyền `Vec::new()`, nên mọi ổ bị coi là đĩa
//!   trong máy và phép đo đo một cấu hình không tồn tại.
//! * Quét nền lúc máy rảnh truyền `Vec::new()` với phạm vi `LocalOnly`, và
//!   `in_scope` viết là `!(là_ổ_mạng && danh_sách.chứa(ổ))` — với danh sách
//!   rỗng thì vế sau luôn sai, nên hàm trả `true` cho **mọi** tệp. Quét nền
//!   đọc trọn NAS, đúng thứ nó tuyên bố không bao giờ làm, trên 20–40 máy mỗi
//!   sáng.
//!
//! Không sửa được bằng cách kiểm "danh sách có rỗng không": **rỗng là hợp lệ**
//! trên máy không gắn ổ mạng nào. Hai trường hợp đó không phân biệt được từ
//! bên trong.
//!
//! Nên cách sửa là bỏ hẳn cơ hội truyền sai: [`DupeService::start`] tự gọi
//! [`OMang::tu_he_thong`] chứ không nhận danh sách từ ai. Không còn tham số
//! thì không còn chỗ gọi nào truyền rỗng được, và trình biên dịch canh giúp
//! thay vì một bài kiểm thử phải nhớ ra mà viết.
//!
//! [`DupeService::start`]: crate::media::dupes::DupeService::start

use std::collections::BTreeMap;

/// Ổ mạng đang gắn trên máy này.
#[derive(Debug, Clone, Default)]
pub struct OMang {
    /// Chữ ổ mạng, **viết hoa**. Rỗng nghĩa là máy này không gắn ổ mạng nào.
    pub chu: Vec<char>,
    /// Chữ ổ (viết hoa) → đường dẫn UNC, ví dụ `\\192.168.1.213\padoma 8`.
    pub unc: BTreeMap<char, String>,
}

impl OMang {
    /// Hỏi Windows. Đây là đường **duy nhất** mã sản phẩm được dùng.
    ///
    /// Cả hai trường lấy từ cùng một lượt `list_volumes()`, nên chúng không
    /// bao giờ lệch nhau — một ổ có trong `chu` mà thiếu trong `unc` sẽ làm
    /// nó vừa bị coi là ổ mạng (loại khỏi phạm vi `LocalOnly`) vừa được xếp
    /// vào nhóm đĩa trong máy.
    pub fn tu_he_thong() -> Self {
        use crate::ntfs::volume::{self, VolumeKind};
        let mut chu = Vec::new();
        let mut unc = BTreeMap::new();
        for v in volume::list_volumes() {
            if v.kind != VolumeKind::Network {
                continue;
            }
            let c = v.letter.to_ascii_uppercase();
            chu.push(c);
            if let Some(r) = v.remote {
                unc.insert(c, r);
            }
        }
        Self { chu, unc }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hai trường phải nói về cùng một tập ổ.
    ///
    /// Không kiểm nội dung — máy chạy bài này có thể không gắn ổ mạng nào, và
    /// đó là kết quả hợp lệ. Chỉ kiểm tính nhất quán, thứ đúng trên mọi máy.
    #[test]
    fn moi_o_trong_unc_deu_co_trong_chu() {
        let m = OMang::tu_he_thong();
        for c in m.unc.keys() {
            assert!(
                m.chu.contains(c),
                "ổ {c} có đường dẫn UNC nhưng không nằm trong danh sách chữ ổ"
            );
        }
    }

    #[test]
    fn chu_o_luon_viet_hoa() {
        // `in_scope` so chữ ổ bằng `eq_ignore_ascii_case` nên chữ thường không
        // gây lỗi ở đó, nhưng `pool_key` và các bản đồ khác thì tra theo khoá
        // chính xác.
        let m = OMang::tu_he_thong();
        for c in &m.chu {
            assert_eq!(*c, c.to_ascii_uppercase(), "chữ ổ phải viết hoa");
        }
        for c in m.unc.keys() {
            assert_eq!(*c, c.to_ascii_uppercase(), "khoá bản đồ UNC phải viết hoa");
        }
    }

    /// Canh ràng buộc bằng cách đọc mã nguồn: `DupeService::start` không được
    /// nhận danh sách ổ mạng từ bên ngoài.
    ///
    /// Đây chính là lỗi đã xảy ra. Nếu ai đó thêm lại tham số đó thì lớp lỗi
    /// "truyền rỗng" mở lại, và không có bài kiểm thử hành vi nào bắt được —
    /// vì trên máy CI không gắn ổ mạng, rỗng là câu trả lời đúng.
    #[test]
    fn start_khong_nhan_danh_sach_o_mang_tu_ben_ngoai() {
        let nguon = include_str!("dupes.rs");
        let than = nguon.split("mod tests").next().unwrap_or(nguon);
        let i = than
            .find("pub fn start(")
            .expect("phải có DupeService::start");
        let than_ham = &than[i..];
        let het = than_ham
            .find(") -> bool")
            .expect("phải tìm được hết danh sách tham số");
        let tham_so = &than_ham[..het];

        assert!(
            !tham_so.contains("net_letters"),
            "start() không được nhận `net_letters`: một chỗ gọi truyền rỗng là \
             quét nền đọc trọn NAS. Gọi `OMang::tu_he_thong()` bên trong."
        );
        assert!(
            !tham_so.contains("remote"),
            "start() không được nhận bản đồ ổ mạng: xem `net_letters` ở trên."
        );
    }
}
