//! Phạm vi quét trùng lặp, và ước lượng trước khi bắt đầu.
//!
//! # Vì sao cần hỏi trước
//!
//! Quét trùng lặp trên ổ mạng đắt hơn ổ trong máy rất nhiều lần, và cái giá đó
//! không đổ lên máy người bấm nút — nó đổ lên **NAS mà cả studio đang dùng để
//! làm việc**. Hai mươi tới bốn mươi máy cùng quét là hai mươi tới bốn mươi
//! luồng đọc ngẫu nhiên trên cùng vài ổ đĩa.
//!
//! Nên người bấm phải được biết mình đang chọn gì, và chọn được. Không đặt một
//! mặc định im lặng theo hướng nào cả: mặc định "có" thì người chỉ muốn dọn ổ
//! C: phải chờ NAS; mặc định "không" thì người muốn dọn NAS tưởng app bỏ sót.
//!
//! # Con số hiện ra phải là con số thật
//!
//! Tầng 1 của quét trùng — gom theo dung lượng — **không đọc đĩa một byte
//! nào**: chỉ mục đã giữ sẵn mọi dung lượng. Nên đếm chính xác "có bao nhiêu
//! tệp phải mở trên ổ trong máy, bao nhiêu trên ổ mạng" là việc vài mili giây,
//! làm được **trước** khi hỏi.
//!
//! Đó là điều kiện để câu hỏi trung thực. Một hộp thoại nói "khoảng 30 phút"
//! dựa trên phỏng đoán thì tệ hơn không nói gì: người dùng tin nó, rồi mất
//! niềm tin khi thực tế khác hẳn.
//!
//! # Điều module này CỐ Ý không làm
//!
//! Nó **không** đoán số phút. Không có phép đo nào cho mã hiện tại trên thư
//! viện hiện tại: con số 584 giây trong tài liệu cũ đo ngày 24/8 trên ổ cục bộ
//! bằng mã khác, trên một chỉ mục còn chứa 70.461 tệp đã bị xoá. Nó không dùng
//! làm mốc được.
//!
//! Nên chỗ này trả về **số tệp phải mở**, tách theo loại ổ, và để phần ước
//! lượng thời gian cho lúc quét đang chạy — khi đã có tốc độ thật của chính
//! máy đó. Xem [`crate::media::dupes::DupeProgress`].

use serde::{Deserialize, Serialize};

use crate::index::model::Index;

/// Dưới ngưỡng này thì không đáng coi là trùng lặp — giữ khớp với
/// `dupes::MIN_INTERESTING_SIZE`.
use crate::media::dupes::MIN_INTERESTING_SIZE;

/// Quét những ổ nào.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DupeScope {
    /// Chỉ đĩa trong máy. Không đụng tới NAS.
    #[default]
    LocalOnly,
    /// Cả ổ mạng.
    Everything,
}

/// Có bao nhiêu tệp phải mở, tách theo loại ổ.
///
/// Đây là con số **đếm được**, không phải ước lượng: tầng 1 chỉ đọc chỉ mục.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeEstimate {
    /// Số tệp ứng viên trên đĩa trong máy.
    pub local_files: usize,
    /// Số tệp ứng viên trên ổ mạng.
    pub network_files: usize,
    /// Các chữ ổ mạng có ứng viên, đã sắp xếp — để câu hỏi gọi đúng tên ổ.
    pub network_drives: Vec<String>,
}

impl ScopeEstimate {
    /// Có ổ mạng nào đáng hỏi không.
    ///
    /// Không có thì đừng hỏi: một hộp thoại chỉ có một câu trả lời đúng là một
    /// hộp thoại thừa.
    pub fn has_network(&self) -> bool {
        self.network_files > 0
    }

    /// Số tệp phải mở với một phạm vi cho trước.
    pub fn files_for(&self, scope: DupeScope) -> usize {
        match scope {
            DupeScope::LocalOnly => self.local_files,
            DupeScope::Everything => self.local_files + self.network_files,
        }
    }
}

/// Đếm ứng viên tầng 1, tách theo loại ổ.
///
/// Lặp lại đúng phép lọc của tầng 1 (`dupes::find_duplicates`): bỏ tệp dưới
/// ngưỡng, gom theo dung lượng, giữ lớp có từ hai tệp trở lên. Không đọc đĩa.
///
/// `net_letters` là chữ cái các ổ mạng đang gắn, viết hoa — ổ mạng ánh xạ
/// trông y hệt đĩa trong máy trong chỉ mục (`Y:\…`), nên không có danh sách
/// này thì mọi ổ NAS bị đếm nhầm vào nhóm "trong máy" và câu hỏi mất hết ý
/// nghĩa.
pub fn estimate(index: &Index, net_letters: &[char]) -> ScopeEstimate {
    use std::collections::{BTreeSet, HashMap};

    let is_net = |v: u8| {
        v != 0
            && net_letters
                .iter()
                .any(|n| (*n as u8).eq_ignore_ascii_case(&v))
    };

    let mut by_size: HashMap<u64, Vec<u32>> = HashMap::new();
    for (i, &size) in index.sizes().iter().enumerate() {
        if size >= MIN_INTERESTING_SIZE {
            by_size.entry(size).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    let mut out = ScopeEstimate::default();
    let mut drives: BTreeSet<String> = BTreeSet::new();
    for entries in by_size.values() {
        for &i in entries {
            let v = index.volume_of(i as usize);
            if is_net(v) {
                out.network_files += 1;
                drives.insert(format!("{}:", (v as char).to_ascii_uppercase()));
            } else {
                out.local_files += 1;
            }
        }
    }
    out.network_drives = drives.into_iter().collect();
    out
}

/// Lọc danh sách việc theo phạm vi.
///
/// Trả về `true` nếu tệp ở vị trí này nằm trong phạm vi được chọn.
pub fn in_scope(index: &Index, i: u32, scope: DupeScope, net_letters: &[char]) -> bool {
    match scope {
        DupeScope::Everything => true,
        DupeScope::LocalOnly => {
            let v = index.volume_of(i as usize);
            !(v != 0
                && net_letters
                    .iter()
                    .any(|n| (*n as u8).eq_ignore_ascii_case(&v)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::model::{IndexBuilder, MediaKind};

    /// Dựng một chỉ mục nhỏ: `(thư mục, tên, dung lượng)`.
    fn idx(files: &[(&str, &str, u64)]) -> Index {
        let mut b = IndexBuilder::new();
        let mut sizes = Vec::new();
        let mut dirs = std::collections::HashMap::new();
        for (dir, name, size) in files {
            let did = *dirs.entry(*dir).or_insert_with(|| b.add_dir(dir, 1));
            b.add_file(name, MediaKind::Video, did, 0);
            sizes.push(*size);
        }
        let mut index = b.finish();
        let n = sizes.len();
        index.set_file_stats(sizes, vec![0i64; n]);
        index
    }

    const LON: u64 = 5_000_000;

    #[test]
    fn dem_dung_va_tach_dung_o_mang_khoi_o_trong_may() {
        let index = idx(&[
            ("D:\\m", "a.mp4", LON),
            ("D:\\m", "b.mp4", LON),
            ("Y:\\p", "c.mp4", LON),
            ("Y:\\p", "d.mp4", LON),
        ]);
        let e = estimate(&index, &['Y']);
        assert_eq!(e.local_files, 2);
        assert_eq!(e.network_files, 2);
        assert_eq!(e.network_drives, ["Y:"]);
        assert!(e.has_network());
    }

    #[test]
    fn o_mang_anh_xa_khong_co_danh_sach_thi_bi_dem_nham_la_o_trong_may() {
        // `Y:\…` trong chỉ mục trông y hệt một đĩa cắm trong máy. Thiếu danh
        // sách ổ mạng thì câu hỏi "có quét NAS không?" không bao giờ hiện ra,
        // và người dùng lặng lẽ quét cả NAS mà không biết.
        let index = idx(&[("Y:\\p", "c.mp4", LON), ("Y:\\p", "d.mp4", LON)]);
        let e = estimate(&index, &[]);
        assert_eq!(e.network_files, 0, "thiếu danh sách thì Y: thành ổ nội bộ");
        assert_eq!(e.local_files, 2);
        assert!(
            !e.has_network(),
            "và câu hỏi sẽ không hiện — đúng lỗi cần tránh"
        );
    }

    #[test]
    fn chi_dem_tep_thuc_su_phai_mo() {
        // Dung lượng chỉ có một tệp thì tầng 1 loại ngay, không đọc đĩa. Đếm
        // nó vào là hứa một con số lớn hơn việc thật.
        let index = idx(&[
            ("D:\\m", "a.mp4", LON),
            ("D:\\m", "b.mp4", LON),
            ("D:\\m", "rieng.mp4", LON + 1),
        ]);
        assert_eq!(estimate(&index, &[]).local_files, 2);
    }

    #[test]
    fn tep_duoi_nguong_khong_duoc_dem() {
        let index = idx(&[("D:\\m", "a.mp4", 1000), ("D:\\m", "b.mp4", 1000)]);
        assert_eq!(estimate(&index, &[]).local_files, 0);
    }

    #[test]
    fn files_for_cong_dung_theo_pham_vi() {
        let e = ScopeEstimate {
            local_files: 10,
            network_files: 200,
            network_drives: vec!["Y:".into()],
        };
        assert_eq!(e.files_for(DupeScope::LocalOnly), 10);
        assert_eq!(e.files_for(DupeScope::Everything), 210);
    }

    #[test]
    fn khong_co_o_mang_thi_khong_hoi() {
        let index = idx(&[("D:\\m", "a.mp4", LON), ("D:\\m", "b.mp4", LON)]);
        assert!(!estimate(&index, &['Y']).has_network());
    }

    #[test]
    fn loc_theo_pham_vi_bo_dung_tep_o_mang() {
        let index = idx(&[("D:\\m", "a.mp4", LON), ("Y:\\p", "c.mp4", LON)]);
        assert!(in_scope(&index, 0, DupeScope::LocalOnly, &['Y']));
        assert!(!in_scope(&index, 1, DupeScope::LocalOnly, &['Y']));
        // Phạm vi đầy đủ thì không loại gì.
        assert!(in_scope(&index, 1, DupeScope::Everything, &['Y']));
    }

    #[test]
    fn mac_dinh_la_chi_o_trong_may() {
        // Mặc định an toàn: không tự đọc NAS khi chưa ai đồng ý.
        assert_eq!(DupeScope::default(), DupeScope::LocalOnly);
    }
}
