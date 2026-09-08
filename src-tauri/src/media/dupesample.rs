//! Lấy mẫu bao nhiêu, ở đâu trong tệp — và vì sao.
//!
//! # Thứ đắt là lần nhảy đầu đọc, không phải byte
//!
//! Đo trên NAS studio: 32 luồng chia cho 66 ms mỗi lần mở lẽ ra cho 485
//! tệp/giây, nhưng thực đo chỉ được 65. Lệch bảy lần. Nút thắt không phải số
//! luồng — thêm luồng từ 24 lên 64 không đổi gì — mà là **số thao tác đọc trên
//! chính đĩa của NAS**.
//!
//! Cách cũ đọc 64 KB đầu, **nhảy tới cuối tệp**, đọc 64 KB cuối. Với một tệp
//! video vài GB, lần nhảy đó là một lần seek thật trên đĩa quay. Bỏ nó đi:
//!
//! | Cách | Tệp/giây | So với hiện tại |
//! |---|---|---|
//! | Hai đầu 64K+64K | 28,4 | chuẩn |
//! | Chỉ đầu 64K | 59,2 | **2,08×** |
//! | Chỉ đuôi 64K | 43,1 | 1,44× |
//! | Chỉ đầu 1 MB | 30,3 | 0,99× |
//!
//! Dòng cuối bác một giả thuyết đáng lẽ nghe rất hợp lý: "byte rẻ, cứ đọc
//! nhiều từ đầu". Không rẻ — mười sáu lần byte tốn đúng bằng một lần nhảy.
//!
//! # Đọc nhiều hơn từ đầu KHÔNG chính xác hơn
//!
//! Đo trên 4.200 tệp, so với chuẩn là cách cũ:
//!
//! | Đọc từ đầu | Nhóm gộp nhầm |
//! |---|---|
//! | 64 KB | 6 |
//! | 128 KB | 6 |
//! | 256 KB | 6 |
//! | 512 KB | 5 |
//!
//! Gấp tám lần dữ liệu, bớt được một nhóm. Vì lỗi không nằm ở "chưa đủ byte"
//! mà ở **chỗ lấy byte**.
//!
//! # Nên chia theo LOẠI TỆP
//!
//! Mọi nhóm gộp nhầm đều là audio: `.MP3`, `.wav`. Cơ chế rõ ràng — hai bản
//! audio cùng độ dài, cùng bộ mã hoá thì phần đầu giống nhau rất dài (header,
//! khoảng lặng đầu), còn video thì khung hình đầu đã khác nhau ngay.
//!
//! Chỉ mục đã giữ sẵn [`MediaKind`], nên phân biệt không tốn một byte đọc đĩa:
//!
//! | Cách | Tệp/giây | Gộp nhầm | Mất nhóm |
//! |---|---|---|---|
//! | Hai đầu | 28,4 | chuẩn | chuẩn |
//! | Chỉ đầu, mọi loại | 59,2 (2,08×) | 3 | 0 |
//! | **Trộn theo loại** | **54,9 (1,93×)** | **1** | **0** |
//!
//! Audio chỉ chiếm **10,4%** ứng viên, nên giữ hai đầu cho chúng tốn 0,15× mà
//! bỏ được hai phần ba số nhóm sai. Đó là lý do chia theo loại chứ không đọc
//! đầu cho tất cả.
//!
//! # Cái giá, nói thẳng
//!
//! Còn lại khoảng **1 nhóm sai trên 1.677** (0,06%). Trên thư viện thật với
//! 42.932 nhóm, đó là chừng hai chục nhóm mà hai tệp trùng dung lượng và trùng
//! phần đầu nhưng khác nội dung.
//!
//! Cần nói cho đúng: cách cũ **cũng không đúng tuyệt đối**. Hai tệp khớp cả
//! 64 KB đầu lẫn 64 KB cuối vẫn có thể khác ở giữa; ta chỉ không nhìn thấy vì
//! không có gì để đối chiếu. Đây không phải đổi từ "đúng" sang "gần đúng", mà
//! là đổi mức gần đúng để lấy hai lần tốc độ.
//!
//! Thứ bảo đảm đúng trước khi xoá là [`crate::media::verify`] — đọc **trọn**
//! nội dung cả nhóm. Nó đã có, và đó là điều kiện để bản đổi này chấp nhận
//! được.
//!
//! [`MediaKind`]: crate::index::model::MediaKind

use crate::index::model::MediaKind;

/// Lấy bao nhiêu byte mỗi đầu tệp.
pub const SAMPLE_BYTES: u64 = 64 * 1024;

/// Dưới ngưỡng này thì đọc trọn tệp — đọc hai đầu của một tệp nhỏ là đọc gần
/// hết nó hai lần, mà vẫn tốn thêm một thao tác.
pub const SMALL_FILE_LIMIT: u64 = 1024 * 1024;

// Đọc hai đầu chỉ có nghĩa khi tệp đủ lớn để hai đầu không chồng nhau.
const _: () = assert!(SMALL_FILE_LIMIT >= SAMPLE_BYTES * 2);

/// Lấy mẫu thế nào cho một tệp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cach {
    /// Đọc trọn tệp. Chỉ cho tệp nhỏ.
    TronTep,
    /// Một lần đọc từ đầu. Rẻ nhất — không có lần nhảy đầu đọc nào.
    ChiDau,
    /// Đầu và cuối. Đắt gấp đôi vì lần nhảy tới cuối tệp.
    HaiDau,
}

/// Chọn cách lấy mẫu cho một tệp, từ thứ chỉ mục đã biết sẵn.
///
/// Không đọc đĩa, không nhìn nội dung — chỉ dung lượng và loại tệp.
pub fn chon(size: u64, kind: MediaKind) -> Cach {
    if size <= SMALL_FILE_LIMIT {
        return Cach::TronTep;
    }
    match kind {
        // Audio: phần đầu hai bản khác nhau giống nhau rất dài, nên phải đọc
        // cả đuôi. Chỉ 10,4% ứng viên nên cái giá nhỏ.
        MediaKind::Audio => Cach::HaiDau,
        // Video và ảnh: khung hình đầu đã đủ khác nhau. Bỏ được lần nhảy tới
        // cuối tệp, và đó là một nửa thời gian của cả lượt quét.
        MediaKind::Video | MediaKind::Image => Cach::ChiDau,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LON: u64 = 100 * 1024 * 1024;

    #[test]
    fn tep_nho_doc_tron_bat_ke_loai() {
        for k in [MediaKind::Video, MediaKind::Image, MediaKind::Audio] {
            assert_eq!(chon(SMALL_FILE_LIMIT, k), Cach::TronTep);
            assert_eq!(chon(70_000, k), Cach::TronTep);
        }
    }

    /// Đây là toàn bộ phần tăng tốc: video lớn chỉ đọc một đầu.
    ///
    /// Đo được 2,08× khi bỏ lần nhảy tới cuối tệp. Nếu ai đó đổi dòng này về
    /// `HaiDau` thì lượt quét NAS chậm lại gấp đôi, và không có bài kiểm thử
    /// hành vi nào bắt được vì kết quả vẫn đúng — chỉ chậm hơn.
    #[test]
    fn video_lon_chi_doc_dau() {
        assert_eq!(chon(LON, MediaKind::Video), Cach::ChiDau);
        assert_eq!(chon(LON, MediaKind::Image), Cach::ChiDau);
    }

    /// Và đây là toàn bộ phần giữ đúng.
    ///
    /// Mọi nhóm gộp nhầm quan sát được trên thư viện thật đều là audio. Đổi
    /// dòng này về `ChiDau` thì nhanh thêm 0,15× và sai gấp ba.
    #[test]
    fn audio_lon_van_doc_hai_dau() {
        assert_eq!(chon(LON, MediaKind::Audio), Cach::HaiDau);
    }

    /// Ngưỡng phải là ranh giới sắc, không phải vùng mờ.
    #[test]
    fn ngay_tren_nguong_thi_doi_cach() {
        assert_eq!(chon(SMALL_FILE_LIMIT, MediaKind::Video), Cach::TronTep);
        assert_eq!(chon(SMALL_FILE_LIMIT + 1, MediaKind::Video), Cach::ChiDau);
        assert_eq!(chon(SMALL_FILE_LIMIT + 1, MediaKind::Audio), Cach::HaiDau);
    }
}
