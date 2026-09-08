//! Tầng 3 của tìm-trùng: xác minh **toàn bộ nội dung** một nhóm, theo yêu cầu.
//!
//! Tầng 2 (dupes.rs) đối chiếu dung lượng và hai đầu tệp — đúng cho việc
//! *tìm ứng viên*, và thanh trạng thái đã tự thú là sai nếu lấy làm căn cứ
//! *xoá*. Tầng này trả món nợ đó: hash trọn vẹn từng byte, nhưng chỉ cho
//! đúng nhóm người dùng sắp hành động — vài giây cho một nhóm, thay vì hàng
//! giờ cho cả thư viện mà tuyệt đại đa số không ai đụng tới.
//!
//! Đây là điều kiện tiên quyết kỹ thuật của tính năng Thùng-rác (mục 7 lộ
//! trình): không bao giờ xoá thứ mới chỉ "giống hai đầu".

use std::collections::HashMap;
use std::io::Read;

use serde::Serialize;

/// Kết quả xác minh một nhóm.
///
/// `groups` là các cụm **trùng thật sự từng byte** — một cụm duy nhất chứa
/// tất cả các tệp đọc được nghĩa là nhóm ứng viên đúng là bản sao của nhau;
/// nhiều cụm nghĩa là tầng 2 đã gom nhầm ít nhất một tệp. `unreadable` liệt
/// kê tệp không đọc nổi (đã xoá, NAS rớt, khoá) — về chúng, ta **không nói
/// gì cả**: không đọc được không phải là "khác nội dung".
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct VerifyOutcome {
    pub groups: Vec<Vec<String>>,
    pub unreadable: Vec<String>,
    /// Lượt này bị người dùng dừng giữa chừng, nên `groups` **chưa phải câu
    /// trả lời** — nó chỉ là phần đã đọc kịp.
    ///
    /// Không có trường này thì một lượt dừng ở tệp thứ hai trên bốn sẽ hiện ra
    /// đúng như một kết luận "trùng thật": hai tệp đầu cùng hash, một cụm duy
    /// nhất. Người dùng xoá hai tệp còn lại mà chưa ai đọc chúng lần nào.
    #[serde(default)]
    pub cancelled: bool,
}

/// Hash trọn một tệp theo dòng chảy — đệm 1 MiB, không kéo cả tệp vào RAM.
///
/// `tien_do` được cộng dồn sau **mỗi khối 1 MiB**, không phải sau mỗi tệp: một
/// tệp 11,2 GB mà chỉ báo khi xong thì thanh tiến độ đứng im hàng phút rồi
/// nhảy một bậc lớn — đúng thứ mà việc thêm thanh tiến độ sinh ra để tránh.
///
/// Trả `Ok(None)` khi người dùng xin dừng giữa chừng: đó không phải lỗi đọc,
/// và gộp nó vào `Err` sẽ khiến tệp bị liệt vào "không đọc được" — một lời
/// khẳng định sai về chính tệp của họ.
fn hash_full(
    path: &str,
    tien_do: Option<&crate::media::verifyprogress::VerifyState>,
) -> std::io::Result<Option<blake3::Hash>> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        if let Some(t) = tien_do {
            if t.cancelled() {
                return Ok(None);
            }
        }
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        if let Some(t) = tien_do {
            t.add_done(n as u64);
        }
    }
    Ok(Some(hasher.finalize()))
}

/// Tổng dung lượng nhóm, để biết mẫu số của thanh tiến độ.
///
/// Đo bằng `metadata()` chứ không lấy con số tầng 2 đã có: tầng 2 gom nhóm
/// theo dung lượng nên mọi tệp trong nhóm *đáng lẽ* bằng nhau, nhưng "đáng lẽ"
/// là thứ tầng 3 tồn tại để kiểm chứ không phải để tin. Tệp không đo được thì
/// bỏ qua — nó sẽ rơi vào `unreadable` ở vòng chính.
fn tong_dung_luong(paths: &[String]) -> u64 {
    paths
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum()
}

/// Xác minh danh sách đường dẫn: cụm theo nội dung thật.
///
/// Chạy tuần tự có chủ ý — một nhóm hiếm khi quá vài tệp, và các bản sao
/// thường nằm cùng một ổ: hai luồng cùng đọc một đĩa cơ chỉ đổi tuần tự lấy
/// tiếng lạch cạch. Cụm lớn xếp trước để giao diện đọc từ trên xuống.
pub fn verify_paths(paths: &[String]) -> VerifyOutcome {
    verify_paths_with(paths, None)
}

/// Như [`verify_paths`], nhưng báo tiến độ và nghe cờ dừng.
///
/// Tách làm hai hàm để mọi lời gọi cũ và toàn bộ bài kiểm thử cũ không phải
/// bận tâm tới tiến độ — thứ chỉ giao diện cần.
///
/// `cancelled` trong kết quả cho giao diện phân biệt hai chuyện hoàn toàn khác
/// nhau: *"đã đọc hết và đây là câu trả lời"* với *"anh bảo dừng nên tôi chưa
/// có câu trả lời"*. Gộp chúng lại là để một lượt dở dang hiện ra như một kết
/// luận — đúng kiểu sai mà cả tầng 3 sinh ra để chống.
pub fn verify_paths_with(
    paths: &[String],
    tien_do: Option<&crate::media::verifyprogress::VerifyState>,
) -> VerifyOutcome {
    if let Some(t) = tien_do {
        t.set_total(tong_dung_luong(paths));
    }

    let mut by_hash: HashMap<blake3::Hash, Vec<String>> = HashMap::new();
    let mut unreadable = Vec::new();
    let mut cancelled = false;

    for p in paths.iter() {
        match hash_full(p, tien_do) {
            Ok(Some(h)) => by_hash.entry(h).or_default().push(p.clone()),
            Ok(None) => {
                // Người dùng xin dừng. Bỏ dở tại đây, và KHÔNG xếp những tệp
                // chưa đọc vào `unreadable` — chúng đọc được, chỉ là chưa đọc.
                cancelled = true;
                break;
            }
            Err(e) => {
                tracing::info!("xác minh trùng lặp: không đọc được {p}: {e}");
                unreadable.push(p.clone());
            }
        }
    }

    let mut groups: Vec<Vec<String>> = by_hash.into_values().collect();
    groups.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    VerifyOutcome {
        groups,
        unreadable,
        cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mf-verify-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn file(dir: &std::path::Path, name: &str, bytes: &[u8]) -> String {
        let p = dir.join(name);
        std::fs::write(&p, bytes).unwrap();
        p.to_string_lossy().into_owned()
    }

    /// Ba bản sao thật + một kẻ giả dạng cùng-dung-lượng-cùng-hai-đầu: tầng 2
    /// gom chung, tầng 3 phải tách được kẻ giả ra.
    #[test]
    fn tach_duoc_ke_gia_dang_cung_hai_dau() {
        let dir = sandbox("fake");
        // 4 KiB: hai đầu giống hệt, khác đúng một byte ở giữa bụng.
        let mut that = vec![0xAAu8; 4096];
        that[2048] = 1;
        let mut gia = that.clone();
        gia[2048] = 2;

        let a = file(&dir, "a.bin", &that);
        let b = file(&dir, "b.bin", &that);
        let c = file(&dir, "c.bin", &that);
        let d = file(&dir, "gia.bin", &gia);

        let out = verify_paths(&[a.clone(), b.clone(), c.clone(), d.clone()]);
        assert!(out.unreadable.is_empty());
        assert_eq!(
            out.groups.len(),
            2,
            "phai tach lam hai cum: {:?}",
            out.groups
        );
        assert_eq!(out.groups[0].len(), 3, "cum lon xep truoc");
        assert_eq!(out.groups[1], vec![d]);

        let _ = std::fs::remove_dir_all(dir);
    }

    /// Nhóm toàn bản sao thật: đúng một cụm, không tệp nào bị nghi oan.
    #[test]
    fn ban_sao_that_ve_mot_cum() {
        let dir = sandbox("real");
        let a = file(&dir, "a.bin", b"noi dung y het nhau");
        let b = file(&dir, "b.bin", b"noi dung y het nhau");
        let out = verify_paths(&[a, b]);
        assert_eq!(out.groups.len(), 1);
        assert_eq!(out.groups[0].len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Tệp biến mất giữa chừng: vào `unreadable`, không phải "khác nội dung",
    /// và không kéo đổ các tệp còn lại.
    #[test]
    fn tep_bien_mat_khong_keo_do_ca_nhom() {
        let dir = sandbox("gone");
        let a = file(&dir, "a.bin", b"con day");
        let b = file(&dir, "b.bin", b"con day");
        let ma = dir.join("da-xoa.bin").to_string_lossy().into_owned();
        let out = verify_paths(&[a, b, ma.clone()]);
        assert_eq!(out.unreadable, vec![ma]);
        assert_eq!(out.groups.len(), 1);
        assert_eq!(out.groups[0].len(), 2);
        assert!(!out.cancelled, "khong ai dung ma lai bao la da dung");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Tiến độ phải nhích theo **byte đã đọc**, và tổng phải là tổng thật.
    ///
    /// Đếm theo tệp thì thanh tiến độ nhảy bậc và đứng im giữa các bậc — với
    /// nhóm 4 tệp 11,2 GB nó tệ hơn không có gì, vì nó hứa một độ mịn không có.
    #[test]
    fn tien_do_dem_theo_byte_va_ve_toi_100() {
        use crate::media::verifyprogress::VerifyState;

        let dir = sandbox("progress");
        // Đủ lớn để vượt vài khối 1 MiB, nên `add_done` được gọi nhiều lần.
        let noi_dung = vec![7u8; 3 * 1024 * 1024 + 12_345];
        let a = file(&dir, "a.bin", &noi_dung);
        let b = file(&dir, "b.bin", &noi_dung);

        let st = VerifyState::new();
        assert!(st.begin(2));
        let out = verify_paths_with(&[a, b], Some(&st));
        st.finish();

        let p = st.snapshot();
        assert_eq!(
            p.total_bytes,
            noi_dung.len() as u64 * 2,
            "tong phai la tong dung luong that cua ca nhom"
        );
        assert_eq!(p.done_bytes, p.total_bytes, "doc xong ma khong dem du");
        assert_eq!(p.percent(), Some(100));
        assert_eq!(p.file_count, 2, "phai bao nhom co bao nhieu tep");
        assert_eq!(out.groups.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// **Bất biến đắt nhất của cả module.** Lượt bị dừng giữa chừng KHÔNG được
    /// hiện ra như một kết luận.
    ///
    /// Dừng ở tệp thứ nhất trên ba: hai tệp chưa đọc không được xếp vào
    /// `unreadable` (chúng đọc được, chỉ là chưa đọc), và `cancelled` phải bật
    /// để giao diện biết đây chưa phải câu trả lời. Thiếu cờ này thì màn hình
    /// hiện "✓ trùng thật" cho một nhóm mới đọc được một phần ba — rồi người
    /// dùng xoá hai tệp chưa ai đọc lần nào.
    #[test]
    fn luot_bi_dung_khong_duoc_hien_ra_nhu_mot_ket_luan() {
        use crate::media::verifyprogress::VerifyState;

        let dir = sandbox("cancel");
        let noi_dung = vec![3u8; 2 * 1024 * 1024];
        let a = file(&dir, "a.bin", &noi_dung);
        let b = file(&dir, "b.bin", &noi_dung);
        let c = file(&dir, "c.bin", &noi_dung);

        let st = VerifyState::new();
        assert!(st.begin(3));
        // Giương cờ TRƯỚC khi chạy: vòng đọc kiểm nó ở đầu mỗi khối, nên lượt
        // này dừng ngay từ khối đầu tiên — tất định, không phụ thuộc thời gian.
        st.cancel();
        let out = verify_paths_with(&[a, b, c], Some(&st));
        st.finish();

        assert!(out.cancelled, "dung giua chung ma khong bat co");
        assert!(
            out.unreadable.is_empty(),
            "tep chua doc bi vu oan la khong doc duoc: {:?}",
            out.unreadable
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
