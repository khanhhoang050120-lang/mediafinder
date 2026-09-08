//! Nhớ kết quả đối chiếu, để không bao giờ đọc lại thứ đã đọc.
//!
//! # Vì sao
//!
//! Đối chiếu toàn bộ một nhóm 3 × 16,65 GB trên ổ `D:` (HDD SATA, đo được
//! 61 MB/s đọc nguội) mất **~14 phút**. Không nhớ kết quả nghĩa là: xác minh
//! xong, đóng ứng dụng, mai mở lại, bấm lại — đọc lại đúng 50 GB ấy.
//!
//! Đây chính là bài học mà [`crate::media::dupestore`] đã trả giá để học ở
//! tầng 2: nhớ vân tay đưa lượt quét thứ hai từ **57,1 phút xuống 1,8 giây**.
//! Số luồng là chuyện 1,15×; nhớ kết quả là chuyện nghìn lần.
//!
//! # Khoá phải gồm cả `mtime`, và cả MỨC đã chạy
//!
//! `(đường dẫn, dung lượng, mtime)` của **mọi** tệp trong nhóm: tệp bị sửa thì
//! kết quả cũ nói về một tệp khác. Đây là cùng một quy tắc `dupestore` dùng.
//!
//! Nhưng còn một chiều nữa mà tầng 2 không có: **mức**. Một kết quả chạy ở mức
//! Nhanh không được đem trả lời cho câu hỏi "trùng từng byte chưa" — nó chưa
//! đọc từng byte. Nhập hai mức vào chung một khoá là biến "gần như chắc" thành
//! "chắc chắn" một cách im lặng, đúng thứ mà cả tầng 3 sinh ra để chống.
//!
//! Ngược lại thì được: đã đối chiếu **Toàn bộ** rồi thì câu hỏi mức Nhanh cũng
//! đã có lời đáp, và đáp mạnh hơn. [`Kho::tra`] tận dụng điều đó.
//!
//! # Vì sao nằm trong RAM, không ghi ra đĩa
//!
//! Khác `dupestore` — kho đó ghi ra đĩa vì lượt quét toàn thư viện là hàng giờ
//! và người dùng làm nó mỗi ngày. Ở đây phạm vi hẹp hơn nhiều: người dùng xác
//! minh vài nhóm trong một phiên rồi xoá. Ghi ra đĩa thêm một tệp trạng thái
//! nữa phải dọn, phải nâng cấp lược đồ, phải lo tệp hỏng — trả giá đó cho một
//! lợi ích chỉ xuất hiện khi ai đó đóng app rồi mở lại và bấm đúng nhóm cũ.
//!
//! Nếu về sau đo được rằng người dùng làm đúng thế thật, thì đây là chỗ để
//! thêm phần ghi đĩa, và `dupestore` là khuôn mẫu có sẵn.

use std::collections::HashMap;

use crate::media::verifyfast::{KetQua, Muc};

/// Dấu hiệu nhận biết một nhóm ở một thời điểm.
///
/// Sắp xếp trước khi băm: cùng một nhóm mà giao diện gửi theo thứ tự khác thì
/// vẫn phải trúng cùng một mục.
fn khoa(paths: &[String]) -> Option<u64> {
    let mut dau: Vec<(String, u64, i64)> = Vec::with_capacity(paths.len());
    for p in paths {
        let m = std::fs::metadata(p).ok()?;
        let mtime = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        dau.push((p.to_lowercase(), m.len(), mtime));
    }
    dau.sort();

    // FNV-1a, viết tay — cùng lý do như `dupestore::path_key`: kết quả ổn định
    // vĩnh viễn vì thuật toán nằm ngay đây, không phụ thuộc thư viện chuẩn.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut nap = |b: &[u8]| {
        for x in b {
            h ^= *x as u64;
            h = h.wrapping_mul(0x1000_0000_01b3);
        }
    };
    for (p, sz, mt) in &dau {
        nap(p.as_bytes());
        nap(&sz.to_le_bytes());
        nap(&mt.to_le_bytes());
        nap(b"|");
    }
    Some(h)
}

/// Kho kết quả đối chiếu của phiên hiện tại.
#[derive(Default)]
pub struct Kho {
    theo_nhom: HashMap<(u64, Muc), KetQua>,
}

impl Kho {
    pub fn new() -> Self {
        Self::default()
    }

    /// Kết quả đã có cho nhóm này ở mức này, nếu còn dùng được.
    ///
    /// Hỏi mức Nhanh mà đã có kết quả **Toàn bộ** thì trả kết quả Toàn bộ:
    /// nó trả lời được câu hỏi yếu hơn, và trả lời mạnh hơn. Chiều ngược lại
    /// thì không — xem ghi chú đầu module.
    pub fn tra(&self, paths: &[String], muc: Muc) -> Option<KetQua> {
        let k = khoa(paths)?;
        if let Some(kq) = self.theo_nhom.get(&(k, Muc::ToanBo)) {
            return Some(kq.clone());
        }
        if muc == Muc::Nhanh {
            return self.theo_nhom.get(&(k, Muc::Nhanh)).cloned();
        }
        None
    }

    /// Ghi nhận một kết quả.
    ///
    /// Lượt bị dừng giữa chừng **không được nhớ**: `groups` của nó mới là phần
    /// đọc kịp, và nhớ nó lại nghĩa là lần sau trả về một kết luận sai mà
    /// không đọc gì thêm.
    pub fn ghi(&mut self, paths: &[String], kq: &KetQua) {
        if kq.cancelled {
            return;
        }
        let Some(k) = khoa(paths) else { return };
        self.theo_nhom.insert((k, kq.muc), kq.clone());
    }

    pub fn len(&self) -> usize {
        self.theo_nhom.len()
    }

    pub fn is_empty(&self) -> bool {
        self.theo_nhom.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mf-vc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tep(dir: &std::path::Path, ten: &str, b: &[u8]) -> String {
        let p = dir.join(ten);
        std::fs::write(&p, b).unwrap();
        p.to_string_lossy().into_owned()
    }

    fn kq(muc: Muc) -> KetQua {
        KetQua {
            groups: vec![vec!["a".into(), "b".into()]],
            muc,
            ..Default::default()
        }
    }

    #[test]
    fn ghi_roi_tra_lai_duoc() {
        let dir = sandbox("hit");
        let a = tep(&dir, "a.bin", b"xin chao");
        let b = tep(&dir, "b.bin", b"xin chao");
        let ds = vec![a, b];

        let mut kho = Kho::new();
        assert!(kho.tra(&ds, Muc::Nhanh).is_none());
        kho.ghi(&ds, &kq(Muc::Nhanh));
        assert!(kho.tra(&ds, Muc::Nhanh).is_some());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// **Bất biến đắt nhất.** Kết quả mức Nhanh KHÔNG được trả lời cho câu hỏi
    /// mức Toàn bộ — nó chưa đọc từng byte.
    #[test]
    fn ket_qua_nhanh_khong_tra_loi_cho_cau_hoi_toan_bo() {
        let dir = sandbox("muc");
        let a = tep(&dir, "a.bin", b"xin chao");
        let b = tep(&dir, "b.bin", b"xin chao");
        let ds = vec![a, b];

        let mut kho = Kho::new();
        kho.ghi(&ds, &kq(Muc::Nhanh));
        assert!(
            kho.tra(&ds, Muc::ToanBo).is_none(),
            "ket qua muc Nhanh bi dem tra loi cho cau hoi Toan bo — bien 'gan \
             nhu chac' thanh 'chac chan' mot cach im lang"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Chiều ngược lại thì được: Toàn bộ trả lời được cả câu hỏi Nhanh.
    #[test]
    fn ket_qua_toan_bo_tra_loi_duoc_ca_cau_hoi_nhanh() {
        let dir = sandbox("manh");
        let a = tep(&dir, "a.bin", b"xin chao");
        let b = tep(&dir, "b.bin", b"xin chao");
        let ds = vec![a, b];

        let mut kho = Kho::new();
        kho.ghi(&ds, &kq(Muc::ToanBo));
        let ra = kho.tra(&ds, Muc::Nhanh).expect("phai dung duoc");
        assert_eq!(ra.muc, Muc::ToanBo, "phai tra ve ket qua manh hon");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Tệp bị sửa thì kết quả cũ nói về một tệp khác — phải đọc lại.
    #[test]
    fn tep_bi_sua_thi_khong_dung_ket_qua_cu() {
        let dir = sandbox("doi");
        let a = tep(&dir, "a.bin", b"xin chao");
        let b = tep(&dir, "b.bin", b"xin chao");
        let ds = vec![a.clone(), b];

        let mut kho = Kho::new();
        kho.ghi(&ds, &kq(Muc::Nhanh));
        assert!(kho.tra(&ds, Muc::Nhanh).is_some());

        // Sửa nội dung: dung lượng đổi, nên khoá đổi.
        std::fs::write(&a, b"xin chao ban").unwrap();
        assert!(
            kho.tra(&ds, Muc::Nhanh).is_none(),
            "tep da doi ma van dung ket qua cu"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Thứ tự đường dẫn không được đổi kết quả tra.
    #[test]
    fn thu_tu_duong_dan_khong_anh_huong() {
        let dir = sandbox("thutu");
        let a = tep(&dir, "a.bin", b"xin chao");
        let b = tep(&dir, "b.bin", b"xin chao");

        let mut kho = Kho::new();
        kho.ghi(&[a.clone(), b.clone()], &kq(Muc::Nhanh));
        assert!(
            kho.tra(&[b, a], Muc::Nhanh).is_some(),
            "dao thu tu lam truot cache"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Lượt bị dừng không được nhớ — nó chưa phải câu trả lời.
    #[test]
    fn luot_bi_dung_khong_duoc_nho() {
        let dir = sandbox("huy");
        let a = tep(&dir, "a.bin", b"xin chao");
        let b = tep(&dir, "b.bin", b"xin chao");
        let ds = vec![a, b];

        let mut kho = Kho::new();
        let mut bo_do = kq(Muc::ToanBo);
        bo_do.cancelled = true;
        kho.ghi(&ds, &bo_do);
        assert!(
            kho.tra(&ds, Muc::ToanBo).is_none(),
            "luot bo do bi nho lai — lan sau tra ve ket luan sai ma khong doc gi"
        );
        assert!(kho.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Tệp không tồn tại thì không dựng được khoá, và tra phải trượt chứ không
    /// được hoảng.
    #[test]
    fn tep_bien_mat_thi_truot_chu_khong_no() {
        let kho = Kho::new();
        let ma = vec!["D:\\khong-he-co\\a.bin".to_string()];
        assert!(kho.tra(&ma, Muc::Nhanh).is_none());
    }
}
