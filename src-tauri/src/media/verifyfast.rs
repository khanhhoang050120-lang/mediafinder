//! Đối chiếu nội dung theo khối — dừng ngay khi khác, và có mức lấy mẫu.
//!
//! # Vì sao module này tồn tại
//!
//! Cách cũ ([`crate::media::verify`]) băm trọn từng tệp rồi mới so vân tay. Hai
//! hệ quả, cả hai đều đo được trên máy studio:
//!
//! * **Không dừng sớm được.** Tệp thứ hai khác ngay byte đầu vẫn phải đọc hết.
//!   Với nhóm 3 × 16,65 GB trên ổ `D:` (HDD SATA, đo được **61–95 MB/s** đọc
//!   nguội) đó là ~14 phút để kết luận một điều lẽ ra biết sau vài giây. Mà
//!   "tầng 2 gom nhầm" chính là ca mà cả tầng 3 sinh ra để bắt.
//! * **Phải đọc trọn dù chỉ muốn biết đại khái.** Không có mức nào ở giữa
//!   "đoán từ hai đầu tệp" và "đọc 50 GB".
//!
//! Đọc song song **không** cứu được: cả ba bản sao nằm trên cùng một đĩa cơ,
//! đo được chỉ **1,17×** (95 → 111 MB/s). Đúng bài học `dupepool.rs` đã ghi.
//! Nút thắt là đầu đọc, không phải luồng.
//!
//! # Hai mức, và vì sao mặc định là Nhanh
//!
//! | Mức | Đọc | Nhóm 50 GB trên HDD 61 MB/s |
//! |---|---|---|
//! | [`Muc::Nhanh`] | ~1% rải đều + trọn hai đầu | **~15 giây** |
//! | [`Muc::ToanBo`] | trọn từng byte | ~14 phút |
//!
//! Một nút mất 14 phút thì thực tế không ai bấm, và một tính năng không ai
//! dùng thì bằng không. Mức Nhanh đọc **200 khối 1 MiB rải đều khắp tệp** cộng
//! trọn 8 MiB mỗi đầu — hai tệp khác nhau mà qua lọt cả 200 điểm ấy là chuyện
//! không xảy ra với dữ liệu thật. Nhưng nó **là** xác suất chứ không phải chứng
//! minh, nên nhãn giao diện phải nói đúng như vậy, và nút Toàn bộ vẫn còn đó.
//!
//! # Đối chiếu trực tiếp, không băm
//!
//! So từng khối giữa các tệp thay vì băm rồi so vân tay: dừng được ngay tại
//! khối đầu tiên khác nhau, và bỏ luôn chi phí băm. Đổi lại phải mở đồng thời
//! N tệp — với một nhóm trùng lặp thì N là vài, không phải vài nghìn.

use std::io::{Read, Seek, SeekFrom};

use serde::{Deserialize, Serialize};

/// Đọc bao nhiêu cho mức Nhanh.
///
/// 200 khối × 1 MiB = 200 MiB mỗi tệp, bất kể tệp lớn cỡ nào — nên chi phí
/// **không tăng theo dung lượng**, đó là điểm mấu chốt. Cộng trọn hai đầu vì
/// đó là nơi định dạng video đặt header và bảng chỉ mục, tức nơi khác biệt hay
/// nằm nhất.
const SO_KHOI_MAU: u64 = 200;
const CO_KHOI: usize = 1024 * 1024;
/// Đọc trọn chừng này ở mỗi đầu tệp, kể cả ở mức Nhanh.
const HAI_DAU: u64 = 8 * 1024 * 1024;

/// Đệm đọc. 8 MiB chứ không phải 1: trên SMB mỗi lần đọc là một vòng mạng, và
/// đệm lớn giảm số vòng theo đúng tỉ lệ.
const DEM: usize = 8 * 1024 * 1024;

/// Mức đối chiếu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Muc {
    /// ~1% rải đều + trọn hai đầu. Mặc định.
    #[default]
    Nhanh,
    /// Trọn từng byte.
    ToanBo,
}

/// Kết quả đối chiếu một nhóm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KetQua {
    /// Các cụm trùng nhau **ở những phần đã đọc**.
    pub groups: Vec<Vec<String>>,
    pub unreadable: Vec<String>,
    pub cancelled: bool,
    /// Mức đã chạy. Giao diện phải nói ra: "trùng ở 200 điểm kiểm" khác hẳn
    /// "trùng từng byte", và gộp hai câu đó làm một là nói quá điều đã chứng
    /// minh — đúng thứ mà cả tầng 3 sinh ra để chống.
    pub muc: Muc,
    /// Đã đọc bao nhiêu byte thật sự, để giao diện nói được cái giá.
    pub bytes_read: u64,
}

/// Các đoạn cần đọc của một tệp, theo mức.
///
/// Tách khỏi phần đọc đĩa để **kiểm thử được bằng số học thuần** — không cần
/// dựng tệp 16 GB mới biết kế hoạch đọc có đúng không.
pub fn ke_hoach_doc(size: u64, muc: Muc) -> Vec<(u64, u64)> {
    if muc == Muc::ToanBo || size <= HAI_DAU * 2 + SO_KHOI_MAU * CO_KHOI as u64 {
        // Tệp nhỏ hơn lượng mẫu thì đọc trọn rẻ hơn nhảy lung tung — và trên
        // đĩa cơ, nhảy lung tung ĐẮT hơn đọc thẳng.
        return vec![(0, size)];
    }

    let mut ra = Vec::with_capacity(SO_KHOI_MAU as usize + 2);
    ra.push((0, HAI_DAU));

    // Rải đều giữa hai đầu. Đều đặn chứ không ngẫu nhiên: cùng một tệp phải
    // cho cùng một kế hoạch ở mọi lần chạy, nếu không thì kết quả không lặp
    // lại được và không ai gỡ lỗi nổi.
    let dau = HAI_DAU;
    let cuoi = size - HAI_DAU;
    let khoang = cuoi - dau;
    for i in 0..SO_KHOI_MAU {
        let off = dau + khoang * i / SO_KHOI_MAU;
        ra.push((off, CO_KHOI as u64));
    }
    ra.push((cuoi, HAI_DAU));
    ra
}

/// Tổng số byte mà một lượt đối chiếu **sẽ đọc**, theo mức.
///
/// Đây là mẫu số đúng cho thanh tiến độ — không phải tổng dung lượng nhóm. Ở
/// mức Nhanh hai con số chênh nhau gần trăm lần (nhóm 50 GB chỉ đọc 0,63 GB),
/// nên lấy nhầm thì thanh bò tới 1,3% rồi nhảy phắt sang xong.
///
/// Là **trần trên**: lượt dừng sớm vì tìm thấy khác biệt sẽ đọc ít hơn, và
/// thanh tiến độ nhảy vọt tới kết quả. Đó là hướng sai đúng — hứa nhiều hơn
/// rồi xong sớm thì không ai phàn nàn, hứa ít hơn rồi chạy quá 100% mới là
/// thứ làm người dùng mất tin.
pub fn tong_se_doc(paths: &[String], muc: Muc) -> u64 {
    paths
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| {
            ke_hoach_doc(m.len(), muc)
                .iter()
                .map(|(_, n)| n)
                .sum::<u64>()
        })
        .sum()
}

/// Một tệp đang được đối chiếu.
struct Mo {
    path: String,
    f: std::fs::File,
}

/// Đối chiếu nội dung một nhóm tệp.
///
/// `dung_lai` được hỏi giữa các khối; trả `true` là dừng.
/// `da_doc` được gọi sau mỗi khối với số byte vừa đọc, cho thanh tiến độ.
pub fn doi_chieu(
    paths: &[String],
    muc: Muc,
    dung_lai: &dyn Fn() -> bool,
    da_doc: &dyn Fn(u64),
) -> KetQua {
    let mut mo: Vec<Mo> = Vec::new();
    let mut unreadable = Vec::new();
    let mut size: Option<u64> = None;

    for p in paths {
        match std::fs::File::open(p) {
            Ok(f) => {
                let sz = f.metadata().map(|m| m.len()).unwrap_or(0);
                // Dung lượng khác nhau là đã khác nhau — không cần đọc byte
                // nào. Tầng 2 gom theo dung lượng nên chuyện này chỉ xảy ra
                // khi tệp bị sửa giữa lúc quét và lúc đối chiếu, nhưng đúng
                // lúc ấy mà tin số cũ thì sai.
                size = Some(size.map_or(sz, |s: u64| s.min(sz)));
                mo.push(Mo { path: p.clone(), f });
            }
            Err(e) => {
                tracing::info!("đối chiếu: không đọc được {p}: {e}");
                unreadable.push(p.clone());
            }
        }
    }

    if mo.len() < 2 {
        return KetQua {
            groups: mo.into_iter().map(|m| vec![m.path]).collect(),
            unreadable,
            muc,
            ..Default::default()
        };
    }

    let size = size.unwrap_or(0);
    let ke_hoach = ke_hoach_doc(size, muc);

    // Bắt đầu: mọi tệp cùng một cụm. Đọc tới đâu tách tới đó.
    let mut cum: Vec<Vec<usize>> = vec![(0..mo.len()).collect()];
    let mut da_doc_tong = 0u64;
    let mut cancelled = false;

    'ngoai: for (off, dai) in ke_hoach {
        let mut da = 0u64;
        while da < dai {
            if dung_lai() {
                cancelled = true;
                break 'ngoai;
            }
            let lay = (dai - da).min(DEM as u64) as usize;
            let vi_tri = off + da;

            // Đọc cùng một đoạn ở mọi tệp còn trong cuộc, rồi tách cụm theo
            // nội dung đoạn đó.
            let mut cum_moi: Vec<Vec<usize>> = Vec::new();
            for nhom in &cum {
                if nhom.len() < 2 {
                    // Đã tách riêng rồi thì thôi đọc — không còn ai để so.
                    cum_moi.push(nhom.clone());
                    continue;
                }
                let mut theo_noi_dung: Vec<(Vec<u8>, Vec<usize>)> = Vec::new();
                for &i in nhom {
                    let mut buf = vec![0u8; lay];
                    let doc_duoc = mo[i]
                        .f
                        .seek(SeekFrom::Start(vi_tri))
                        .and_then(|_| doc_du(&mut mo[i].f, &mut buf));
                    match doc_duoc {
                        Ok(n) => {
                            buf.truncate(n);
                            da_doc_tong += n as u64;
                            da_doc(n as u64);
                        }
                        Err(_) => {
                            buf.clear();
                        }
                    }
                    match theo_noi_dung.iter_mut().find(|(b, _)| *b == buf) {
                        Some((_, ds)) => ds.push(i),
                        None => theo_noi_dung.push((buf, vec![i])),
                    }
                }
                for (_, ds) in theo_noi_dung {
                    cum_moi.push(ds);
                }
            }
            cum = cum_moi;

            // Mọi tệp đã tách riêng: không còn gì để so, dừng đọc.
            if cum.iter().all(|n| n.len() < 2) {
                break 'ngoai;
            }
            da += lay as u64;
        }
    }

    let mut groups: Vec<Vec<String>> = cum
        .into_iter()
        .map(|n| n.into_iter().map(|i| mo[i].path.clone()).collect())
        .collect();
    groups.sort_by(|a: &Vec<String>, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));

    KetQua {
        groups,
        unreadable,
        cancelled,
        muc,
        bytes_read: da_doc_tong,
    }
}

/// Đọc cho đầy đệm, hoặc tới hết tệp.
///
/// `Read::read` được phép trả về ít hơn số byte yêu cầu mà không phải lỗi —
/// trên SMB điều đó xảy ra thường xuyên. Đọc một lần rồi so là so hai đoạn dài
/// khác nhau của hai tệp giống hệt nhau, và kết luận "khác nội dung".
fn doc_du(f: &mut std::fs::File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut da = 0usize;
    while da < buf.len() {
        match f.read(&mut buf[da..]) {
            Ok(0) => break,
            Ok(n) => da += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(da)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mf-vf-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tep(dir: &std::path::Path, ten: &str, b: &[u8]) -> String {
        let p = dir.join(ten);
        std::fs::write(&p, b).unwrap();
        p.to_string_lossy().into_owned()
    }

    const KHONG_DUNG: fn() -> bool = || false;

    #[test]
    fn toan_bo_luon_doc_tron_tep() {
        let kh = ke_hoach_doc(100_000_000_000, Muc::ToanBo);
        assert_eq!(kh, vec![(0, 100_000_000_000)]);
    }

    /// Chi phí mức Nhanh **không tăng theo dung lượng** — đó là toàn bộ lý do
    /// nó tồn tại. Tệp 16 GB và tệp 160 GB phải đọc xấp xỉ bằng nhau.
    #[test]
    fn muc_nhanh_khong_dat_hon_khi_tep_lon_hon() {
        let a: u64 = ke_hoach_doc(16 * 1024 * 1024 * 1024, Muc::Nhanh)
            .iter()
            .map(|(_, n)| n)
            .sum();
        let b: u64 = ke_hoach_doc(160 * 1024 * 1024 * 1024, Muc::Nhanh)
            .iter()
            .map(|(_, n)| n)
            .sum();
        assert_eq!(a, b, "chi phi muc Nhanh phai la hang so");
        // 200 MiB mẫu + 16 MiB hai đầu.
        assert_eq!(a, SO_KHOI_MAU * CO_KHOI as u64 + HAI_DAU * 2);
    }

    /// Tệp nhỏ hơn lượng mẫu thì đọc trọn — nhảy lung tung trên đĩa cơ đắt hơn
    /// đọc thẳng, nên "lấy mẫu" ở đó là tự làm chậm mình.
    #[test]
    fn tep_nho_thi_doc_tron_du_o_muc_nhanh() {
        let nho = 4 * 1024 * 1024;
        assert_eq!(ke_hoach_doc(nho, Muc::Nhanh), vec![(0, nho)]);
    }

    #[test]
    fn ke_hoach_khong_bao_gio_doc_qua_cuoi_tep() {
        let size = 64 * 1024 * 1024 * 1024;
        for (off, n) in ke_hoach_doc(size, Muc::Nhanh) {
            assert!(off + n <= size, "doc qua cuoi tep: {off}+{n} > {size}");
        }
    }

    /// Mẫu số của thanh tiến độ phải khớp số byte THẬT SỰ đọc.
    ///
    /// Lỗi đã lọt ra tận app thật: `set_total` chỉ nằm trong module cũ, còn
    /// lệnh IPC đã chuyển sang gọi `doi_chieu` — nên `totalBytes` mãi là 0 và
    /// giao diện mãi nói "đang đọc…", không bao giờ hiện phần trăm. Không bài
    /// nào bắt được vì không bài nào nối hai đầu lại.
    #[test]
    fn tong_se_doc_khop_so_byte_that_su_doc() {
        let dir = sandbox("tongdoc");
        let noi_dung = vec![4u8; 3_000_000];
        let a = tep(&dir, "a.bin", &noi_dung);
        let b = tep(&dir, "b.bin", &noi_dung);
        let ds = vec![a, b];

        for muc in [Muc::Nhanh, Muc::ToanBo] {
            let du_bao = tong_se_doc(&ds, muc);
            let kq = doi_chieu(&ds, muc, &KHONG_DUNG, &|_| {});
            assert_eq!(
                du_bao, kq.bytes_read,
                "mau so thanh tien do lech so byte that su doc, muc {muc:?}"
            );
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Ở mức Nhanh, mẫu số phải là lượng SẼ đọc chứ không phải dung lượng
    /// nhóm — hai con số chênh nhau gần trăm lần.
    #[test]
    fn tong_se_doc_o_muc_nhanh_nho_hon_han_dung_luong_nhom() {
        let size = 16 * 1024 * 1024 * 1024u64;
        let mot_tep: u64 = ke_hoach_doc(size, Muc::Nhanh).iter().map(|(_, n)| n).sum();
        assert!(
            mot_tep * 50 < size,
            "muc Nhanh doc {mot_tep} tren tep {size} — le ra phai duoi 2%"
        );
    }

    #[test]
    fn ban_sao_that_ve_mot_cum() {
        let dir = sandbox("same");
        let noi_dung = vec![9u8; 3_000_000];
        let a = tep(&dir, "a.bin", &noi_dung);
        let b = tep(&dir, "b.bin", &noi_dung);
        let kq = doi_chieu(&[a, b], Muc::ToanBo, &KHONG_DUNG, &|_| {});
        assert_eq!(kq.groups.len(), 1);
        assert_eq!(kq.groups[0].len(), 2);
        assert!(!kq.cancelled);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// **Bất biến trung tâm.** Kẻ giả dạng cùng dung lượng, cùng hai đầu, khác
    /// ở giữa — đúng ca mà tầng 2 gom nhầm và tầng 3 sinh ra để bắt.
    #[test]
    fn tach_duoc_ke_gia_dang_khac_o_giua() {
        let dir = sandbox("fake");
        let mut that = vec![0xAAu8; 3_000_000];
        that[1_500_000] = 1;
        let mut gia = that.clone();
        gia[1_500_000] = 2;

        let a = tep(&dir, "a.bin", &that);
        let b = tep(&dir, "b.bin", &that);
        let g = tep(&dir, "gia.bin", &gia);

        let kq = doi_chieu(&[a, b, g.clone()], Muc::ToanBo, &KHONG_DUNG, &|_| {});
        assert_eq!(kq.groups.len(), 2, "phai tach lam hai: {:?}", kq.groups);
        assert_eq!(kq.groups[0].len(), 2);
        assert_eq!(kq.groups[1], vec![g]);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// **Dừng sớm.** Khác nhau ngay đầu tệp thì phải thôi đọc, không cày hết.
    ///
    /// Đây là thứ tiết kiệm 14 phút xuống vài giây ở ca tầng 2 gom nhầm.
    #[test]
    fn khac_ngay_dau_thi_thoi_doc_phan_con_lai() {
        let dir = sandbox("early");
        let mut a_b = vec![5u8; 40 * 1024 * 1024];
        a_b[0] = 1;
        let mut b_b = a_b.clone();
        b_b[0] = 2;
        let a = tep(&dir, "a.bin", &a_b);
        let b = tep(&dir, "b.bin", &b_b);

        let kq = doi_chieu(&[a, b], Muc::ToanBo, &KHONG_DUNG, &|_| {});
        assert_eq!(kq.groups.len(), 2);
        assert!(
            kq.bytes_read < 40 * 1024 * 1024,
            "khac ngay dau ma van doc het: {} byte",
            kq.bytes_read
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tep_bien_mat_vao_unreadable_khong_phai_khac_noi_dung() {
        let dir = sandbox("gone");
        let a = tep(&dir, "a.bin", b"con day");
        let b = tep(&dir, "b.bin", b"con day");
        let ma = dir.join("da-xoa.bin").to_string_lossy().into_owned();
        let kq = doi_chieu(&[a, b, ma.clone()], Muc::ToanBo, &KHONG_DUNG, &|_| {});
        assert_eq!(kq.unreadable, vec![ma]);
        assert_eq!(kq.groups.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Dừng giữa chừng thì phải nói ra, không được hiện như một kết luận.
    #[test]
    fn dung_giua_chung_thi_bat_co() {
        let dir = sandbox("cancel");
        let noi_dung = vec![7u8; 5_000_000];
        let a = tep(&dir, "a.bin", &noi_dung);
        let b = tep(&dir, "b.bin", &noi_dung);
        let kq = doi_chieu(&[a, b], Muc::ToanBo, &|| true, &|_| {});
        assert!(kq.cancelled);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tien_do_bao_dung_so_byte_da_doc() {
        let dir = sandbox("prog");
        let noi_dung = vec![3u8; 2_000_000];
        let a = tep(&dir, "a.bin", &noi_dung);
        let b = tep(&dir, "b.bin", &noi_dung);

        let dem = std::sync::atomic::AtomicU64::new(0);
        let kq = doi_chieu(&[a, b], Muc::ToanBo, &KHONG_DUNG, &|n| {
            dem.fetch_add(n, std::sync::atomic::Ordering::Relaxed);
        });
        assert_eq!(
            dem.load(std::sync::atomic::Ordering::Relaxed),
            kq.bytes_read,
            "so byte bao cho thanh tien do phai khop so byte that su doc"
        );
        assert_eq!(kq.bytes_read, 4_000_000, "hai tep 2 MB thi doc 4 MB");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn muc_duoc_ghi_vao_ket_qua_de_giao_dien_noi_dung_su_that() {
        let dir = sandbox("muc");
        let a = tep(&dir, "a.bin", b"xin chao");
        let b = tep(&dir, "b.bin", b"xin chao");
        assert_eq!(
            doi_chieu(&[a.clone(), b.clone()], Muc::Nhanh, &KHONG_DUNG, &|_| {}).muc,
            Muc::Nhanh
        );
        assert_eq!(
            doi_chieu(&[a, b], Muc::ToanBo, &KHONG_DUNG, &|_| {}).muc,
            Muc::ToanBo
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
