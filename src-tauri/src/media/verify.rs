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
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerifyOutcome {
    pub groups: Vec<Vec<String>>,
    pub unreadable: Vec<String>,
}

/// Hash trọn một tệp theo dòng chảy — đệm 1 MiB, không kéo cả tệp vào RAM.
fn hash_full(path: &str) -> std::io::Result<blake3::Hash> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize())
}

/// Xác minh danh sách đường dẫn: cụm theo nội dung thật.
///
/// Chạy tuần tự có chủ ý — một nhóm hiếm khi quá vài tệp, và các bản sao
/// thường nằm cùng một ổ: hai luồng cùng đọc một đĩa cơ chỉ đổi tuần tự lấy
/// tiếng lạch cạch. Cụm lớn xếp trước để giao diện đọc từ trên xuống.
pub fn verify_paths(paths: &[String]) -> VerifyOutcome {
    let mut by_hash: HashMap<blake3::Hash, Vec<String>> = HashMap::new();
    let mut unreadable = Vec::new();

    for p in paths {
        match hash_full(p) {
            Ok(h) => by_hash.entry(h).or_default().push(p.clone()),
            Err(e) => {
                tracing::info!("xác minh trùng lặp: không đọc được {p}: {e}");
                unreadable.push(p.clone());
            }
        }
    }

    let mut groups: Vec<Vec<String>> = by_hash.into_values().collect();
    groups.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    VerifyOutcome { groups, unreadable }
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
        let _ = std::fs::remove_dir_all(dir);
    }
}
