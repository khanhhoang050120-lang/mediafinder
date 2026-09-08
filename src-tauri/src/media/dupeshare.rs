//! Chia sẻ vân tay ổ mạng giữa các máy studio.
//!
//! # Vấn đề, đo được
//!
//! Sau khi bỏ đọc đuôi và nâng sàn dung lượng, lượt quét gần chạm trần phần
//! cứng: NAS phục vụ được khoảng **65 lần mở tệp mỗi giây** và không đổi dù
//! thêm luồng (đo ở P50 — 24 tới 64 luồng cho cùng một con số). Còn 112.432
//! tệp phải mở, trong đó **79% nằm trên ổ mạng**.
//!
//! Nhưng nội dung ổ mạng **giống hệt nhau trên cả 20–40 máy studio**, và hôm
//! nay mỗi máy tự đọc lại từ đầu. Bốn mươi máy đọc cùng một nội dung bốn mươi
//! lần là bốn mươi lần tải lên chính NAS mà cả studio đang dùng để làm việc.
//!
//! Máy đầu tiên đọc rồi để lại vân tay ngay trên share; máy thứ hai trở đi đọc
//! tệp đó thay vì đọc 89 nghìn tệp media.
//!
//! # Vì sao khoá theo UNC, không theo chữ ổ
//!
//! `\\192.168.1.213\padoma 8` có thể là `Y:` trên máy này và `W:` trên máy
//! khác. Khoá theo `Y:\a\b.mp4` thì máy kia không tra được gì. Khoá phải là
//! đường dẫn UNC đầy đủ, chữ thường — thứ giống nhau trên mọi máy.
//!
//! # Vì sao mỗi máy một tệp riêng
//!
//! Bốn mươi máy cùng ghi một tệp là bài toán khoá phân tán trên SMB, và
//! `rename` nguyên tử qua SMB không phải thứ nên đánh cược. Mỗi máy ghi
//! `<tên-máy>.bin` của riêng nó, người đọc gộp mọi tệp tìm thấy. Không tranh
//! khoá, không cần đồng bộ, và một máy ghi hỏng chỉ làm hỏng phần của nó.
//!
//! # Vì sao tin được vân tay của máy khác
//!
//! Không tin mù. Mỗi mục mang `(dung lượng, thời gian sửa)`, và người đọc chỉ
//! dùng vân tay khi **cả hai còn khớp với chỉ mục của chính mình**. Tệp đã đổi
//! thì mục cũ tự bị loại — đúng cơ chế mà kho cục bộ
//! ([`crate::media::dupestore`]) đã chạy.
//!
//! Rủi ro còn lại là một máy ghi vân tay **sai** (đĩa lỗi, bản app cũ). Số
//! phiên bản trong header chặn bản cũ; phần còn lại do
//! [`crate::media::verify`] chặn — đọc trọn nội dung trước khi xoá.
//!
//! # Máy chủ nào được ghi
//!
//! Chỉ máy chủ nằm trong [`DUOC_GHI`]. Đây là **danh sách cho phép**, không
//! phải danh sách cấm: một thư mục chia sẻ mới xuất hiện trong studio sẽ
//! **không** được ghi vào cho tới khi có người quyết định, thay vì được ghi vào
//! rồi mới phát hiện ra.
//!
//! `\\192.168.1.214` (ổ `F:` và `H:`) là **máy trạm của người khác**, không
//! phải NAS. Nó nằm trong [`CAM_GHI`] để nếu ai đó thêm nhầm vào danh sách cho
//! phép thì hỏng lúc biên dịch chứ không hỏng trên bốn mươi máy.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::media::dupestore::Store;

/// Máy chủ được phép GHI vân tay lên.
///
/// Danh sách cho phép, không phải danh sách cấm. Xem chú thích đầu module.
pub const DUOC_GHI: &[&str] = &[r"\\192.168.1.213"];

/// Máy chủ CẤM ghi, kèm lý do.
///
/// `\\192.168.1.214` là máy trạm studio đang chia sẻ thư mục (ổ `F:` và `H:`),
/// không phải NAS. Ghi vào đó là ghi vào máy người khác đang làm việc.
pub const CAM_GHI: &[&str] = &[r"\\192.168.1.214"];

// Kiểm LÚC BIÊN DỊCH rằng hai danh sách không giao nhau.
//
// Chú thích đầu module hứa điều này, và lúc đầu nó là lời hứa suông: một bài
// kiểm thử lúc chạy thì bỏ sạch `CAM_GHI` vẫn xanh, vì danh sách cho phép đã
// tự loại `.214` rồi. Phép đo bằng cách phá mã lôi ra chỗ đó.
//
// Nay nếu ai thêm `\192.168.1.214` vào `DUOC_GHI` thì crate **không dịch
// được** — không phải một bài đỏ mà ai đó có thể bỏ qua, mà là không ra được
// bản cài để đẩy lên bốn mươi máy.
const fn bang(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        // Chữ hoa/thường: hằng số trong tệp này đều viết thường, và so sánh
        // lúc chạy trong `duoc_ghi` mới là chỗ xử lý chữ hoa.
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn hai_danh_sach_giao_nhau() -> bool {
    let mut i = 0;
    while i < DUOC_GHI.len() {
        let mut j = 0;
        while j < CAM_GHI.len() {
            if bang(DUOC_GHI[i].as_bytes(), CAM_GHI[j].as_bytes()) {
                return true;
            }
            j += 1;
        }
        i += 1;
    }
    false
}

const _: () = assert!(
    !hai_danh_sach_giao_nhau(),
    "một máy chủ nằm trong cả danh sách cho phép lẫn danh sách cấm"
);

/// Thư mục chứa vân tay dùng chung, ngay trên share.
pub const THU_MUC: &str = ".mediafinder";

/// Đọc nhiều nhất bao nhiêu tệp vân tay mỗi share.
///
/// Bốn mươi máy là bốn mươi tệp, mỗi tệp cỡ 7 MB — đọc hết là gần 300 MB qua
/// SMB *trước khi* lượt quét bắt đầu, tức đúng thứ tính năng này sinh ra để
/// tránh. Nội dung các tệp gần như trùng nhau vì chúng nói về cùng một NAS,
/// nên vài tệp mới nhất đã phủ gần hết.
pub const TOI_DA_TEP: usize = 8;

/// Đọc từ máy chủ nào cũng được — đọc không ghi gì cả, nên không có gì để
/// hỏng. Nhưng chỉ có ích ở máy chủ mà một máy nào đó đã ghi vào.
pub fn duoc_doc(_may_chu: &str) -> bool {
    true
}

/// Có được ghi lên máy chủ này không.
///
/// Phải nằm trong [`DUOC_GHI`] **và** không nằm trong [`CAM_GHI`]. Hai điều
/// kiện là thừa về mặt logic, cố ý: danh sách cấm là chỗ ghi lại *lý do*, và
/// bài kiểm thử canh cho chúng không bao giờ giao nhau.
pub fn duoc_ghi(may_chu: &str) -> bool {
    let m = may_chu.to_ascii_lowercase();
    DUOC_GHI.iter().any(|x| x.eq_ignore_ascii_case(&m))
        && !CAM_GHI.iter().any(|x| x.eq_ignore_ascii_case(&m))
}

/// Tên máy chủ từ một đường dẫn UNC: `\\192.168.1.213\padoma 8` → `\\192.168.1.213`.
pub fn may_chu(unc: &str) -> Option<String> {
    let t = unc.trim_start_matches('\\');
    let h = t.split('\\').next()?;
    if h.is_empty() {
        None
    } else {
        Some(format!(r"\\{}", h.to_ascii_lowercase()))
    }
}

/// Đường dẫn UNC đầy đủ của một tệp trên ổ mạng, chữ thường.
///
/// `Y:\a\b.mp4` với `Y:` → `\\192.168.1.213\padoma 8` cho
/// `\\192.168.1.213\padoma 8\a\b.mp4`.
///
/// Trả `None` cho tệp không nằm trên ổ mạng đã biết — chúng thuộc kho cục bộ,
/// không chia sẻ được (đường dẫn `D:\...` của máy này vô nghĩa với máy khác).
pub fn duong_unc(duong_may: &str, unc_theo_o: &BTreeMap<char, String>) -> Option<String> {
    let mut c = duong_may.chars();
    let o = c.next()?.to_ascii_uppercase();
    if c.next()? != ':' {
        return None;
    }
    let goc = unc_theo_o.get(&o)?;
    // Phần sau `Y:` đã mang sẵn dấu gạch chéo đầu.
    let duoi: String = duong_may.chars().skip(2).collect();
    Some(format!("{}{}", goc.trim_end_matches('\\'), duoi).to_ascii_lowercase())
}

/// Tên tệp vân tay của máy này.
///
/// Theo tên máy để bốn mươi máy không ghi đè lên nhau. Tên máy Windows chỉ
/// gồm chữ, số và gạch nối, nhưng vẫn lọc lại: một tên lạ lọt vào đây thành
/// đường dẫn ghi ra ngoài thư mục đã định.
pub fn ten_tep_may_nay() -> String {
    let ten = std::env::var("COMPUTERNAME").unwrap_or_default();
    let sach: String = ten
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if sach.is_empty() {
        "may-khong-ten.bin".to_string()
    } else {
        format!("{}.bin", sach.to_ascii_lowercase())
    }
}

/// Thư mục vân tay dùng chung của một share.
pub fn thu_muc_cua(share_unc: &str) -> PathBuf {
    PathBuf::from(share_unc.trim_end_matches('\\')).join(THU_MUC)
}

/// Gộp vân tay của mọi máy đã ghi lên các share này.
///
/// Đọc mọi `*.bin` trong `<share>\.mediafinder\`. Lỗi ở một tệp chỉ bỏ tệp đó,
/// không bỏ cả lượt: một máy ghi hỏng không được làm ba mươi chín máy kia mất
/// tác dụng.
///
/// Trả về kho đã gộp và số tệp đã đọc được.
pub fn gop_tu_share(shares: &[String]) -> (Store, usize) {
    let mut gop = Store::default();
    let mut so_tep = 0usize;

    for share in shares {
        let d = thu_muc_cua(share);
        let Ok(doc) = std::fs::read_dir(&d) else {
            continue; // chưa máy nào ghi, hoặc không đọc được — không phải lỗi
        };

        // Sắp theo lần ghi gần nhất, và chỉ lấy `TOI_DA_TEP` tệp đầu.
        //
        // Bốn mươi máy là bốn mươi tệp, mỗi tệp cỡ 7 MB — đọc hết là gần 300 MB
        // qua SMB trước khi quét bắt đầu. Nhưng nội dung các tệp gần như trùng
        // nhau (cùng một NAS, cùng nội dung), nên vài tệp mới nhất đã phủ gần
        // hết. Không cắt im lặng: phần bị bỏ được ghi vào log.
        let mut tep: Vec<(std::time::SystemTime, PathBuf)> = doc
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("bin"))
            .map(|p| {
                let t = p
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                (t, p)
            })
            .collect();
        tep.sort_unstable_by_key(|(t, _)| std::cmp::Reverse(*t));

        if tep.len() > TOI_DA_TEP {
            tracing::info!(
                "vân tay dùng chung ở {}: {} tệp, chỉ đọc {TOI_DA_TEP} tệp mới nhất",
                d.display(),
                tep.len()
            );
            tep.truncate(TOI_DA_TEP);
        }

        for (_, p) in tep {
            let kho = crate::media::dupestore::load_from(&p);
            if kho.is_empty() {
                continue;
            }
            tracing::info!("vân tay dùng chung: {} mục từ {}", kho.len(), p.display());
            gop.gop_them(&kho);
            so_tep += 1;
        }
    }
    (gop, so_tep)
}

/// Ghi vân tay của máy này lên các share được phép.
///
/// Trả về số share đã ghi được. Không ghi được thì thôi — share chỉ-đọc là
/// chuyện bình thường, và tính năng này là phần thêm chứ không phải điều kiện
/// để quét trùng lặp chạy đúng.
pub fn ghi_len_share(shares: &[String], kho: &Store) -> usize {
    if kho.is_empty() {
        return 0;
    }

    // LỌC TRƯỚC, làm sau.
    //
    // Bản đầu lọc bên trong vòng lặp, ngay trước `create_dir_all`. Nó đúng,
    // nhưng đúng **nhờ thứ tự hai câu lệnh** — ai đó sắp xếp lại vòng lặp là
    // bốn mươi máy bắt đầu tạo thư mục trên máy trạm của một người, và không
    // bài kiểm thử nào bắt được vì hàm vẫn trả về 0.
    //
    // Lọc trước thì phần thân hàm **không cầm** đường dẫn nào bị cấm, nên nó
    // không thể chạm vào dù có sắp xếp lại thế nào.
    let cho_phep = loc_share_duoc_ghi(shares);
    if cho_phep.is_empty() {
        return 0;
    }

    let ten = ten_tep_may_nay();
    let mut n = 0usize;
    for d in cho_phep.iter().map(|s| thu_muc_cua(s)) {
        if std::fs::create_dir_all(&d).is_err() {
            tracing::info!("không tạo được {}: có thể share chỉ đọc", d.display());
            continue;
        }
        if crate::media::dupestore::save_to(&d.join(&ten), kho) {
            tracing::info!("đã ghi {} mục vân tay lên {}", kho.len(), d.display());
            n += 1;
        }
    }
    n
}

/// Giữ lại những share nằm trên máy chủ được phép ghi.
///
/// Tách ra thành hàm riêng để kiểm được **mà không chạm vào hệ thống tệp**:
/// bài thử gọi thẳng hàm này và thấy `\192.168.1.214` bị loại, thay vì phải
/// suy ra điều đó từ việc `ghi_len_share` trả về 0 — một con số 0 cũng có thể
/// đến từ share chỉ đọc, tức bài thử sẽ xanh cả khi thư mục đã bị tạo ra.
pub fn loc_share_duoc_ghi(shares: &[String]) -> Vec<String> {
    shares
        .iter()
        .filter(|s| may_chu(s).is_some_and(|mc| duoc_ghi(&mc)))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Máy trạm của người khác KHÔNG được ghi vào.
    ///
    /// `\\192.168.1.214` chia sẻ ổ `F:` và `H:` nhưng là máy trạm studio, không
    /// phải NAS. Đây là ràng buộc người dùng nêu ra, và nó không suy ra được
    /// từ mã — nên phải có bài canh.
    #[test]
    fn khong_ghi_len_may_tram_214() {
        assert!(
            !duoc_ghi(r"\\192.168.1.214"),
            "F: và H: là máy trạm, không phải NAS"
        );
        assert!(!duoc_ghi(r"\\192.168.1.214".to_ascii_uppercase().as_str()));
    }

    #[test]
    fn ghi_duoc_len_nas_213() {
        assert!(duoc_ghi(r"\\192.168.1.213"));
    }

    /// Danh sách cho phép, không phải danh sách cấm.
    ///
    /// Một thư mục chia sẻ mới xuất hiện trong studio phải KHÔNG được ghi vào
    /// cho tới khi có người quyết định. Nếu chỗ này đảo thành danh sách cấm
    /// thì mọi máy chủ lạ đều được ghi, và cái sai chỉ lộ ra sau khi đã ghi.
    #[test]
    fn may_chu_la_thi_khong_duoc_ghi() {
        assert!(!duoc_ghi(r"\\192.168.1.99"));
        assert!(!duoc_ghi(r"\\nas-moi"));
        assert!(!duoc_ghi(""));
    }

    /// Danh sách cấm phải còn nội dung, và phép kiểm lúc biên dịch phải còn đó.
    ///
    /// Bài kiểm thử lúc chạy KHÔNG canh được việc `.214` bị thêm vào danh sách
    /// cho phép — đo bằng cách phá mã cho thấy bỏ sạch `CAM_GHI` mà mọi bài
    /// vẫn xanh, vì danh sách cho phép đã tự loại `.214` rồi. Thứ canh thật là
    /// `const _: () = assert!(...)` ở đầu tệp: thêm `.214` vào `DUOC_GHI` thì
    /// crate không dịch được.
    ///
    /// Bài này canh cho phép kiểm đó không bị xoá, và canh cho `.214` không
    /// biến mất khỏi danh sách cấm — mất nó là mất chỗ ghi lại *lý do*.
    #[test]
    fn may_tram_214_van_nam_trong_danh_sach_cam() {
        assert!(
            CAM_GHI.iter().any(|c| c.contains("192.168.1.214")),
            "máy trạm .214 phải nằm trong danh sách cấm — đó là ràng buộc người dùng nêu"
        );
        let nguon = include_str!("dupeshare.rs");
        let than = nguon.split("mod tests").next().unwrap_or(nguon);
        assert!(
            than.contains("!hai_danh_sach_giao_nhau()"),
            "phép kiểm lúc biên dịch phải còn đó — bài lúc chạy không thay được nó"
        );
    }

    fn ban_do() -> BTreeMap<char, String> {
        [
            ('F', r"\\192.168.1.214\f".to_string()),
            ('Y', r"\\192.168.1.213\padoma 8".to_string()),
        ]
        .into_iter()
        .collect()
    }

    /// Khoá phải là UNC, để máy khác tra được.
    #[test]
    fn duong_may_thanh_duong_unc() {
        let m = ban_do();
        assert_eq!(
            duong_unc(r"Y:\a\b.mp4", &m).as_deref(),
            Some(r"\\192.168.1.213\padoma 8\a\b.mp4")
        );
    }

    /// Chữ ổ khác nhau giữa hai máy vẫn phải cho cùng một khoá.
    ///
    /// Đây là toàn bộ lý do khoá theo UNC: `\\192.168.1.213\padoma 8` là `Y:`
    /// trên máy này và có thể là `W:` trên máy khác. Khoá theo chữ ổ thì máy
    /// kia không tra được gì, và tính năng này thành vô dụng một cách im lặng.
    #[test]
    fn chu_o_khac_nhau_van_ra_cung_mot_khoa() {
        let may_a: BTreeMap<char, String> = [('Y', r"\\192.168.1.213\padoma 8".to_string())]
            .into_iter()
            .collect();
        let may_b: BTreeMap<char, String> = [('W', r"\\192.168.1.213\padoma 8".to_string())]
            .into_iter()
            .collect();
        assert_eq!(
            duong_unc(r"Y:\phim\a.mp4", &may_a),
            duong_unc(r"W:\phim\a.mp4", &may_b)
        );
    }

    #[test]
    fn chu_hoa_thuong_khong_lam_lech_khoa() {
        let m = ban_do();
        assert_eq!(duong_unc(r"y:\A\B.MP4", &m), duong_unc(r"Y:\a\b.mp4", &m));
    }

    /// Đĩa trong máy KHÔNG chia sẻ được.
    ///
    /// `D:\du-an\a.mp4` của máy này là một tệp khác trên máy khác. Chia sẻ nó
    /// là báo trùng lặp giữa hai tệp không liên quan gì nhau.
    #[test]
    fn dia_trong_may_khong_co_khoa_chia_se() {
        let m = ban_do();
        assert_eq!(duong_unc(r"D:\du-an\a.mp4", &m), None);
        assert_eq!(duong_unc(r"C:\x.mp4", &m), None);
    }

    #[test]
    fn duong_hong_khong_lam_no() {
        let m = ban_do();
        assert_eq!(duong_unc("", &m), None);
        assert_eq!(duong_unc("Y", &m), None);
        assert_eq!(duong_unc(r"\\may\share\a.mp4", &m), None);
    }

    /// Tên tệp phải là một tên tệp, không phải một đường dẫn.
    #[test]
    fn ten_tep_khong_thoat_ra_khoi_thu_muc() {
        let t = ten_tep_may_nay();
        assert!(
            !t.contains('\\') && !t.contains('/') && !t.contains(".."),
            "thấy {t}"
        );
        assert!(t.ends_with(".bin"));
    }

    #[test]
    fn may_chu_tach_dung_tu_unc() {
        assert_eq!(
            may_chu(r"\\192.168.1.213\padoma 8").as_deref(),
            Some(r"\\192.168.1.213")
        );
        assert_eq!(may_chu(""), None);
        assert_eq!(may_chu(r"\\"), None);
    }

    #[test]
    fn thu_muc_nam_tren_chinh_share() {
        let d = thu_muc_cua(r"\\192.168.1.213\padoma 8");
        assert_eq!(d, PathBuf::from(r"\\192.168.1.213\padoma 8\.mediafinder"));
    }

    /// Không có gì để ghi thì đừng chạm vào share.
    #[test]
    fn kho_rong_thi_khong_ghi_gi() {
        let n = ghi_len_share(
            &[r"\\192.168.1.213\padoma 8".to_string()],
            &Store::default(),
        );
        assert_eq!(n, 0, "kho rỗng thì không được tạo tệp nào trên share");
    }

    /// Bộ lọc phải loại share của máy `.214`, KHÔNG chạm hệ thống tệp.
    ///
    /// Đây là ràng buộc người dùng nêu thành lời: không tạo thư mục ẩn và
    /// không đặt tệp tên máy trên `\192.168.1.214` (ổ `F:` và `H:`), chỉ làm
    /// trên NAS `\192.168.1.213`.
    ///
    /// Kiểm ở đây chứ không kiểm qua `ghi_len_share` trả về 0: con số 0 cũng
    /// đến từ share chỉ đọc, nên bài thử kiểu đó sẽ xanh cả khi thư mục đã bị
    /// tạo ra trên máy trạm rồi.
    #[test]
    fn bo_loc_giu_nas_213_va_loai_may_tram_214() {
        let vao = vec![
            r"\192.168.1.214".to_string(),
            r"\192.168.1.214\h".to_string(),
            r"\192.168.1.213\padoma 8".to_string(),
            r"\192.168.1.213\padoma 1".to_string(),
        ];
        let ra = loc_share_duoc_ghi(&vao);
        assert_eq!(
            ra,
            vec![
                r"\192.168.1.213\padoma 8".to_string(),
                r"\192.168.1.213\padoma 1".to_string()
            ],
            "chỉ NAS .213 được giữ lại; F: và H: trên .214 phải bị loại"
        );
        assert!(
            !ra.iter().any(|s| s.contains("192.168.1.214")),
            "không đường dẫn nào của máy trạm được lọt qua"
        );
    }

    /// Thân hàm ghi không được cầm đường dẫn chưa lọc.
    ///
    /// Bản đầu lọc bên trong vòng lặp, ngay trước `create_dir_all` — đúng,
    /// nhưng đúng nhờ thứ tự hai câu lệnh. Bài này canh cho phép lọc nằm ở đầu
    /// hàm, để sắp xếp lại vòng lặp không mở lại đường ghi vào máy trạm.
    #[test]
    fn ghi_len_share_loc_truoc_khi_cham_he_thong_tep() {
        let nguon = include_str!("dupeshare.rs");
        let than = nguon.split("mod tests").next().unwrap_or(nguon);
        let i = than.find("pub fn ghi_len_share").expect("phải có hàm");
        let sau = &than[i..];
        let het = sau.find("pub fn loc_share_duoc_ghi").unwrap_or(sau.len());

        // Bỏ dòng chú thích trước khi soi.
        //
        // Lần viết đầu bài này đỏ oan: chính lời giải thích ngay trong hàm có
        // nhắc `create_dir_all`, và nó đứng trước lời gọi bộ lọc. Bài đang canh
        // THỨ TỰ MÃ CHẠY, nên phải nhìn mã chạy.
        let ma: String = sau[..het]
            .lines()
            .filter(|d| !d.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join(
                "
",
            );

        let loc = ma.find("loc_share_duoc_ghi(").expect("phải gọi bộ lọc");
        let tao = ma
            .find("std::fs::create_dir_all")
            .expect("phải có tạo thư mục");
        assert!(
            loc < tao,
            "phải lọc share TRƯỚC khi chạm hệ thống tệp, nếu không .214 bị tạo thư mục"
        );
        assert!(
            !ma.contains("for share in shares"),
            "không được lặp trên danh sách CHƯA lọc trong thân hàm ghi"
        );
    }

    /// Share trên máy cấm thì không được ghi, dù kho có dữ liệu.
    ///
    /// Bài này chạy được trên mọi máy vì nó dừng ở phép kiểm danh sách, trước
    /// khi chạm tới hệ thống tệp.
    #[test]
    fn share_tren_may_cam_thi_khong_ghi() {
        let mut kho = Store::default();
        kho.put(r"\\192.168.1.214\f\a.mp4", 5_000_000, 123, [1u8; 32]);
        let n = ghi_len_share(&[r"\\192.168.1.214\f".to_string()], &kho);
        assert_eq!(n, 0, "không được ghi lên máy trạm");
    }
}
