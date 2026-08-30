//! Chỉ mục cũ tới đâu — dữ liệu để màn hình "không có kết quả" nói đúng sự thật.
//!
//! # Vấn đề
//!
//! Trước module này, app chỉ có **một** câu cho mọi trường hợp không tìm thấy:
//! *"Không tìm thấy kết quả nào"*. Câu đó gộp bốn tình huống khác hẳn nhau —
//! gõ sai tên, bộ lọc đang che, tệp mới lưu vào ổ trong máy, tệp mới đưa lên
//! ổ mạng — và ngầm đổ lỗi cho người dùng ở cả bốn.
//!
//! Nó đã gây thiệt hại thật: một người tìm tệp `.avif` trong lúc chip lọc
//! *Video* đang bật, không thấy gì, và kết luận **công cụ tìm kiếm kém đi**.
//! Công cụ không sai; màn hình nói sai về nguyên nhân.
//!
//! # Vì sao đọc mốc quét chứ không đọc USN journal
//!
//! Ý đầu tiên là hỏi journal: *"ổ D: có bao nhiêu thay đổi kể từ lần quét?"* —
//! câu trả lời chính xác, trong một mili giây. Nhưng
//! [CHECK-004](../../docs/check.md) đã đo dứt khoát: `FSCTL_READ_USN_JOURNAL`
//! đòi handle mở với `FILE_READ_DATA` trên volume, mà mức đó **cần quyền
//! Administrator**. Bất biến kiến trúc của dự án là GUI không bao giờ chạy
//! elevated, nên đường đó đóng.
//!
//! Đổi lại: cache đã lưu sẵn, **cho từng ổ**, chữ cái và số tệp, cùng một mốc
//! `built_at_unix` chung. Không cần quyền gì. Mất phần "bao nhiêu thay đổi",
//! nhưng giữ được phần quyết định — **chỉ mục cũ tới đâu** — và đó là thứ
//! phân biệt "tệp mới chưa được ghi nhận" với "tên gõ sai".
//!
//! # Vì sao tách ổ trong máy khỏi ổ mạng
//!
//! Hai loại ổ có độ tin cậy khác nhau, nên hai câu thông báo phải khác nhau:
//!
//! * **Ổ trong máy** được tác vụ nền quét lại mỗi ngày, tốn ~1 mili giây. Mốc
//!   quét gần như luôn mới, nên nếu nó đã cũ thì đó là tín hiệu đáng tin.
//! * **Ổ mạng** không có journal (đó chính là gốc của BUG-025) và mỗi lượt
//!   quét mất khoảng hai phút. Mốc của nó cũ là chuyện thường, không phải dấu
//!   hiệu hỏng hóc.
//!
//! Gộp chung thì hoặc là app kêu oan về ổ mạng suốt ngày, hoặc là nó im lặng
//! về ổ trong máy đúng lúc cần lên tiếng.

use serde::Serialize;

/// Một ổ trong chỉ mục, kèm việc nó được quét lúc nào.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VolumeFreshness {
    /// `"D"` hoặc `"Y"`.
    pub letter: String,
    /// Ổ mạng ánh xạ hay đĩa trong máy.
    pub network: bool,
    /// Số tệp media của riêng ổ này trong chỉ mục.
    pub file_count: usize,
}

/// Chỉ mục cũ tới đâu, chia theo loại ổ.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Freshness {
    /// Mốc quét gần nhất, giây Unix. `0` nghĩa là chưa có chỉ mục.
    pub built_at_unix: i64,
    /// Các ổ trong máy.
    pub local: Vec<VolumeFreshness>,
    /// Các ổ mạng đang gắn và **có trong chỉ mục**.
    pub network: Vec<VolumeFreshness>,
    /// Có ổ mạng đang gắn mà chỉ mục chưa hề biết tới không.
    ///
    /// Khác hẳn "ổ mạng cũ": ổ này **chưa từng** được quét, nên mọi tệp trên
    /// nó đều vô hình chứ không chỉ vài tệp mới. Đáng một câu riêng.
    pub unscanned_network: Vec<String>,
}

/// Dựng câu trả lời từ dữ liệu thô.
///
/// Tách khỏi phần đọc cache và đọc danh sách ổ, để kiểm thử được mà không cần
/// một tệp cache thật hay một ổ mạng thật.
///
/// * `stamps` — `(chữ cái, số tệp)` **đếm từ chính chỉ mục**, không phải từ
///   trường `volumes` của cache.
/// * `mounted_network` — chữ cái các ổ mạng đang gắn, viết hoa.
///
/// # Vì sao không dùng `volumes` của cache
///
/// Đó là lỗi đã mắc và người dùng bắt được ngay lượt thử đầu: sau khi quét ổ
/// mạng xong, app vẫn báo *"ổ mạng chưa được quét lần nào"* — và sẽ báo như
/// vậy vĩnh viễn.
///
/// Nguyên nhân: `volumes` lưu con trỏ USN journal, mà **ổ mạng cố ý không có
/// dòng nào trong đó** — không có journal nào để ghi vị trí, và bịa ra một
/// dòng sẽ khiến bản cập nhật nhanh tưởng nó theo dõi được ổ mạng. Chú thích
/// ngay tại chỗ hợp nhất đã nói rõ điều này; tôi đọc sót.
///
/// Đếm từ chỉ mục thì không có chỗ cho hiểu lầm đó: chỉ mục suy ra ổ từ đường
/// dẫn của từng tệp, nên "có tệp nào trên ổ Y: không" là câu trả lời trực
/// tiếp, đúng cho mọi loại ổ.
pub fn build(built_at_unix: i64, stamps: &[(char, usize)], mounted_network: &[char]) -> Freshness {
    let is_net = |c: char| mounted_network.iter().any(|n| n.eq_ignore_ascii_case(&c));

    let mut out = Freshness {
        built_at_unix,
        ..Default::default()
    };

    for (letter, file_count) in stamps {
        let v = VolumeFreshness {
            letter: letter.to_ascii_uppercase().to_string(),
            network: is_net(*letter),
            file_count: *file_count,
        };
        if v.network {
            out.network.push(v);
        } else {
            out.local.push(v);
        }
    }

    // Ổ mạng đang gắn mà chỉ mục không có dòng nào — chưa từng quét.
    for n in mounted_network {
        let co = stamps.iter().any(|(l, _)| l.eq_ignore_ascii_case(n));
        if !co {
            out.unscanned_network
                .push(n.to_ascii_uppercase().to_string());
        }
    }

    out
}

/// Đọc từ cache trên đĩa và danh sách ổ đang gắn.
///
/// Rẻ: chỉ đọc phần đầu tệp cache, không nạp cả chỉ mục. Lỗi thì trả về giá
/// trị rỗng thay vì `Result` — người gọi là màn hình "không có kết quả", và ở
/// đó "không biết gì thêm" là một câu trả lời hợp lệ, còn một hộp lỗi thì
/// không giúp được ai.
pub fn read(state: &crate::state::AppState) -> Freshness {
    use crate::ntfs::volume::{self, VolumeKind};

    let mounted: Vec<char> = volume::list_volumes()
        .into_iter()
        .filter(|v| v.kind == VolumeKind::Network)
        .map(|v| v.letter)
        .collect();

    let meta = state.meta();
    if !meta.loaded {
        return build(0, &[], &mounted);
    }

    // Đếm tệp theo ổ từ chính chỉ mục đang chạy. `volume_of` suy ra chữ cái từ
    // đường dẫn, nên nó đúng cho cả ổ mạng — thứ mà `volumes` của cache cố ý
    // bỏ trống.
    //
    // Một lượt duyệt qua toàn bộ chỉ mục, khoảng 400 nghìn mục. Chấp nhận
    // được vì nó chỉ chạy khi màn hình đã không có kết quả nào, tức là ngoài
    // đường tìm kiếm.
    let ix = state.snapshot();
    let mut dem: std::collections::BTreeMap<char, usize> = std::collections::BTreeMap::new();
    for i in 0..ix.len() {
        let v = ix.volume_of(i);
        if v != 0 {
            *dem.entry(v as char).or_insert(0) += 1;
        }
    }

    let stamps: Vec<(char, usize)> = dem.into_iter().collect();
    build(meta.built_at_unix, &stamps, &mounted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tach_o_trong_may_khoi_o_mang() {
        let f = build(1_700_000_000, &[('C', 10), ('D', 20), ('Y', 30)], &['Y']);
        assert_eq!(
            f.local
                .iter()
                .map(|v| v.letter.as_str())
                .collect::<Vec<_>>(),
            ["C", "D"]
        );
        assert_eq!(
            f.network
                .iter()
                .map(|v| v.letter.as_str())
                .collect::<Vec<_>>(),
            ["Y"]
        );
        assert_eq!(f.built_at_unix, 1_700_000_000);
    }

    #[test]
    fn nhan_ra_o_mang_anh_xa_du_duong_dan_trong_khong_khac_gi_dia_trong_may() {
        // `Y:\…` trong chỉ mục trông y hệt một đĩa cắm trong máy. Chỉ danh
        // sách ổ đang gắn mới phân biệt được — thiếu nó thì mọi ổ NAS bị xếp
        // nhầm vào nhóm "quét mỗi ngày", và câu thông báo sai hoàn toàn.
        let f = build(0, &[('Y', 5)], &[]);
        assert_eq!(
            f.local.len(),
            1,
            "thiếu danh sách ổ mạng thì Y: là ổ nội bộ"
        );

        let f = build(0, &[('Y', 5)], &['Y']);
        assert_eq!(f.network.len(), 1);
        assert!(f.local.is_empty());
    }

    #[test]
    fn chu_thuong_va_chu_hoa_la_cung_mot_o() {
        let f = build(0, &[('y', 5)], &['Y']);
        assert_eq!(f.network.len(), 1, "y: và Y: phải là cùng một ổ");
        assert_eq!(f.network[0].letter, "Y", "nhãn phải viết hoa");
        assert!(f.unscanned_network.is_empty());
    }

    #[test]
    fn o_mang_gan_nhung_chua_tung_quet_duoc_bao_rieng() {
        // Ổ này khác "ổ mạng cũ": MỌI tệp trên nó đều vô hình, không phải chỉ
        // vài tệp mới. Gộp chung thì người dùng bấm "quét lại" và tưởng mình
        // đã cập nhật, trong khi ổ đó chưa bao giờ được đụng tới.
        let f = build(1_700_000_000, &[('C', 10)], &['Y', 'Z']);
        assert_eq!(f.unscanned_network, ["Y", "Z"]);
        assert!(f.network.is_empty());
    }

    #[test]
    fn o_mang_da_quet_roi_thi_khong_con_bao_la_chua_quet() {
        // Lỗi người dùng bắt được ngay lượt thử đầu, và nó SỐNG SÓT qua cả năm
        // ca kiểm thử ở trên: sau khi quét ổ mạng xong, app vẫn báo "ổ mạng
        // chưa được quét lần nào" — vĩnh viễn.
        //
        // Gốc: bản đầu đọc số tệp từ trường `volumes` của cache, mà ổ mạng CỐ Ý
        // không có dòng nào trong đó (không có journal để ghi vị trí con trỏ).
        // Nên "không có dòng" bị hiểu thành "chưa quét", trong khi nó nghĩa là
        // "loại ổ này không dùng journal".
        //
        // Nay đếm từ chính chỉ mục, nên hễ có tệp trên ổ đó là biết đã quét.
        let f = build(1_700_000_000, &[('C', 10), ('Y', 159_095)], &['Y']);
        assert!(
            f.unscanned_network.is_empty(),
            "ổ Y: có 159.095 tệp trong chỉ mục mà vẫn bị coi là chưa quét"
        );
        assert_eq!(f.network.len(), 1);
        assert_eq!(f.network[0].file_count, 159_095);
    }

    #[test]
    fn chi_o_mang_khong_co_tep_nao_moi_bi_goi_la_chua_quet() {
        // Ranh giới đúng: quét rồi (dù chỉ vài tệp) khác hẳn chưa quét lần nào.
        let f = build(1_700_000_000, &[('C', 10), ('Y', 1)], &['Y', 'Z']);
        assert_eq!(f.unscanned_network, ["Z"], "chỉ Z: mới là ổ chưa quét");
    }

    #[test]
    fn chua_co_chi_muc_thi_khong_no_va_khong_bia_ra_gi() {
        let f = build(0, &[], &[]);
        assert_eq!(f.built_at_unix, 0);
        assert!(f.local.is_empty() && f.network.is_empty());
        assert!(f.unscanned_network.is_empty());
    }
}
