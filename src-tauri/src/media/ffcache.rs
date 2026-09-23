//! Nhớ bản chuyển mã TRỌN VẸN, để mở lại một tệp vừa xem là tức thì.
//!
//! # Vì sao
//!
//! Người ta đi qua các kết quả bằng phím mũi tên, và rất hay quay lại tệp vừa
//! xem. Không nhớ thì mỗi lần quay lại là một lượt chuyển mã nữa — vài giây
//! CPU trọn các lõi cho một thứ đã làm xong rồi.
//!
//! Chỉ bản chuyển mã **từ đầu tới cuối tệp** được nhớ. Một phiên tua (bắt đầu
//! giữa chừng) chỉ có một khúc, và khúc thì không trả lời được câu hỏi "cho
//! tôi cả video".
//!
//! # Vì sao nằm trong RAM chứ không ghi đĩa
//!
//! Cùng lý lẽ mà [`crate::media::verifycache`] đã ghi: phạm vi hẹp. Người
//! dùng xem trước vài tệp trong một phiên rồi thôi; ghi ra đĩa là thêm một
//! tệp trạng thái phải dọn, phải nâng cấp lược đồ, phải lo hỏng — để đổi lấy
//! một lợi ích chỉ xuất hiện khi ai đó đóng app rồi mở lại đúng tệp cũ.
//!
//! # Trần, và vì sao có hạn dùng
//!
//! Bốn mục và tối đa [`TRAN_BYTE`]. Một bản chuyển mã nặng 3–25 MB tuỳ độ dài
//! và độ hạt của nguồn. App này chạy ngầm cả ngày ở khay hệ thống, nên giữ
//! ~100 MB video đã xem từ sáng tới tối là không đáng — mục không ai đụng quá
//! [`HAN_DUNG`] thì bỏ.
//!
//! # Khoá phải gồm `mtime`
//!
//! Cùng quy tắc mà [`crate::media::dupestore`] đã trả giá để học: tệp bị sửa
//! thì bản cũ nói về một tệp khác. Khoá là `(đường dẫn, dung lượng, mtime)`.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Giữ nhiều nhất bấy nhiêu bản.
const SO_MUC: usize = 4;

/// Và tổng cộng không quá chừng này.
const TRAN_BYTE: usize = 96 * 1024 * 1024;

/// Mục không ai đụng quá lâu thì bỏ.
const HAN_DUNG: Duration = Duration::from_secs(10 * 60);

/// Nhận dạng một tệp tại một thời điểm.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Khoa {
    path: String,
    size: u64,
    mtime: i64,
}

struct Muc {
    khoa: Khoa,
    /// `Arc` để trả về mà không sao chép vài chục MB.
    du_lieu: Arc<Vec<u8>>,
    lan_cuoi: Instant,
}

/// Kho dùng chung cho cả tiến trình.
///
/// `static` chứ không phải `State` của Tauri: người dùng nó ([`crate::media::ffphien`])
/// là mã tự do không giữ trạng thái nào, và luồn một tham số qua đó chỉ để
/// tới đây thì tốn hơn là đáng.
static KHO: Mutex<VecDeque<Muc>> = Mutex::new(VecDeque::new());

/// Đọc metadata để dựng khoá. `None` khi tệp không còn.
fn khoa_cua(path: &str) -> Option<Khoa> {
    let md = std::fs::metadata(path).ok()?;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Some(Khoa {
        path: path.to_string(),
        size: md.len(),
        mtime,
    })
}

/// Bỏ mục quá hạn, rồi mục cũ nhất cho tới khi vừa trần.
fn thu_gon(kho: &mut VecDeque<Muc>) {
    kho.retain(|m| m.lan_cuoi.elapsed() < HAN_DUNG);
    while kho.len() > SO_MUC || kho.iter().map(|m| m.du_lieu.len()).sum::<usize>() > TRAN_BYTE {
        if kho.pop_back().is_none() {
            break;
        }
    }
}

/// Bản chuyển mã trọn của `path`, nếu đã có.
pub fn tra(path: &str) -> Option<Arc<Vec<u8>>> {
    let khoa = khoa_cua(path)?;
    let mut kho = KHO.lock();
    thu_gon(&mut kho);
    let i = kho.iter().position(|m| m.khoa == khoa)?;
    // Đưa lên đầu: mục vừa dùng là mục ít đáng bị đuổi nhất.
    let mut muc = kho.remove(i)?;
    muc.lan_cuoi = Instant::now();
    let du_lieu = Arc::clone(&muc.du_lieu);
    kho.push_front(muc);
    Some(du_lieu)
}

/// Ghi nhớ bản chuyển mã trọn vừa xong.
pub fn luu(path: &str, du_lieu: Arc<Vec<u8>>) {
    let Some(khoa) = khoa_cua(path) else {
        return;
    };
    let mut kho = KHO.lock();
    kho.retain(|m| m.khoa != khoa);
    kho.push_front(Muc {
        khoa,
        du_lieu,
        lan_cuoi: Instant::now(),
    });
    thu_gon(&mut kho);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Các bài dưới đây dùng chung `KHO` — trạng thái toàn cục của cả tiến
    /// trình. Chạy song song thì bài này dọn mất thứ bài kia vừa ghi. Khoá
    /// này nối tiếp chúng lại; cả nhóm chạy trong vài mili giây.
    static TUAN_TU: Mutex<()> = Mutex::new(());

    #[test]
    fn tep_khong_ton_tai_thi_khong_tra_cung_khong_luu() {
        let _t = TUAN_TU.lock();
        let p = r"Z:\khong-he-ton-tai-8c1f.mov";
        assert!(tra(p).is_none());
        luu(p, Arc::new(vec![1, 2, 3]));
        assert!(tra(p).is_none());
    }

    /// Lưu rồi tra lại phải ra đúng dữ liệu đó — dùng chính tệp mã nguồn này
    /// làm tệp có thật.
    #[test]
    fn luu_roi_tra_lai_duoc() {
        let _t = TUAN_TU.lock();
        let p = file!();
        if !std::path::Path::new(p).is_file() {
            return; // chạy từ thư mục khác; không phải lỗi
        }
        luu(p, Arc::new(vec![9, 9, 9]));
        assert_eq!(tra(p).map(|d| d.to_vec()), Some(vec![9u8, 9, 9]));
        // Lưu lại thì thay, không giữ hai bản.
        luu(p, Arc::new(vec![7]));
        assert_eq!(tra(p).map(|d| d.to_vec()), Some(vec![7u8]));
        assert_eq!(KHO.lock().iter().filter(|m| m.khoa.path == p).count(), 1);
    }

    /// Quá trần thì mục cũ nhất bị đuổi, không phải mục vừa dùng.
    #[test]
    fn muc_cu_nhat_bi_duoi_truoc() {
        let _t = TUAN_TU.lock();
        let mut kho = KHO.lock();
        kho.clear();
        for i in 0..SO_MUC + 2 {
            kho.push_front(Muc {
                khoa: Khoa {
                    path: format!("t{i}"),
                    size: 1,
                    mtime: 0,
                },
                du_lieu: Arc::new(vec![i as u8]),
                lan_cuoi: Instant::now(),
            });
            thu_gon(&mut kho);
        }
        assert_eq!(kho.len(), SO_MUC);
        assert!(
            !kho.iter().any(|m| m.khoa.path == "t0"),
            "mục cũ nhất phải đi trước"
        );
        kho.clear();
    }

    /// Tổng dung lượng cũng là một trần, không chỉ số mục.
    #[test]
    fn qua_tran_dung_luong_thi_duoi() {
        let _t = TUAN_TU.lock();
        let mut kho = KHO.lock();
        kho.clear();
        for i in 0..2 {
            kho.push_front(Muc {
                khoa: Khoa {
                    path: format!("lon{i}"),
                    size: 1,
                    mtime: 0,
                },
                du_lieu: Arc::new(vec![0; TRAN_BYTE / 2 + 1]),
                lan_cuoi: Instant::now(),
            });
            thu_gon(&mut kho);
        }
        assert_eq!(
            kho.len(),
            1,
            "hai mục cộng lại vượt trần thì chỉ giữ mục mới"
        );
        assert_eq!(kho[0].khoa.path, "lon1");
        kho.clear();
    }
}
