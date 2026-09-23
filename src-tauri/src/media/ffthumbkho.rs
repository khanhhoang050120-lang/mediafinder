//! Ảnh thu nhỏ dựng bằng ffmpeg: nhớ trên đĩa, và không đọc ổ mạng để đoán.
//!
//! [`crate::media::ffthumb`] lo việc hỏi ffmpeg. Tệp này lo việc **khỏi phải
//! hỏi lại** — vì với tệp nằm trên NAS, mỗi lần hỏi là một lần kéo tệp qua
//! đường mạng dùng chung của cả studio.
//!
//! # Vì sao phải nhớ trên đĩa
//!
//! Ảnh mà shell dựng được thì Explorer đã nhớ hộ trong `thumbcache_*.db`:
//! lần sau hỏi là trả lời trong vài micro giây, kể cả sau khi khởi động lại
//! máy. Ảnh do ffmpeg dựng thì **không ai nhớ** ngoài LRU trong RAM của
//! [`crate::media::thumbnail`] — mất khi tắt app, và chỉ giữ 512 mục. Nên
//! mỗi phiên làm việc, mỗi tệp ProRes cuộn qua lại là một lượt ffmpeg nữa:
//! đọc moov + một khung ProRes (cỡ 1 MB với 1080p, 3,5 MB với 4K 710 Mbps)
//! rồi giải mã. Trên ổ trong máy đó là tới nửa giây CPU; trên NAS còn là
//! từng ấy MB qua đường mạng mà 20–40 máy đang dùng chung.
//!
//! Nhớ trên đĩa biến điều đó thành: **mỗi tệp, mỗi máy, đọc một lần**.
//!
//! # Vì sao mỗi ảnh một tệp, không gộp một kho
//!
//! Các kho khác của app ([`crate::media::dupestore`]…) là một tệp bincode đọc
//! trọn lúc khởi động. Ở đây không hợp: ảnh được hỏi lẻ từng cái theo nhịp
//! cuộn, từ bốn luồng cùng lúc, và cần ghi ngay khi có. Một tệp mỗi ảnh thì
//! đọc một ảnh là đọc một tệp nhỏ, ghi là ghi-tạm-rồi-đổi-tên — không khoá,
//! không bao giờ đọc phải nửa chừng, và một tệp hỏng chỉ làm mất đúng một ảnh.
//!
//! # Khoá
//!
//! `(đường dẫn, dung lượng, mtime, cỡ ảnh)` — cùng quy tắc mà
//! [`crate::media::dupestore`] đã trả giá để học: tệp bị sửa thì ảnh cũ nói
//! về một tệp khác. Đường dẫn viết thường vì Windows không phân biệt hoa
//! thường. Thêm [`PHIEN_BAN`] để đổi cách chọn khung (ví dụ mốc giây) là đủ
//! bỏ hết ảnh cũ mà không phải dọn tay.
//!
//! # Luật ổ mạng: không đọc để đoán
//!
//! Cùng nguyên tắc với `preview_prewarm` bên xem trước: việc **đoán trước**
//! (người dùng *có thể* sắp nhìn tới) được phép tốn CPU của máy này, nhưng
//! không được kéo tệp qua NAS. Giao diện tải trước nửa màn hình theo hướng
//! cuộn — với tệp ProRes trên NAS, mỗi lượt đoán đó là một lượt ffmpeg đọc
//! tệp qua mạng cho một ô có khi không ai cuộn tới. Nên khi yêu cầu là đoán
//! trước ([`dung`] với `du_doan`), tệp ở trên ổ mạng và chưa có ảnh trong kho,
//! câu trả lời là `Busy` — "hỏi lại sau", không bị nhớ là "không có ảnh" —
//! và ô đó được dựng thật khi nó hiện ra.
//!
//! Tệp trong máy vẫn được đoán trước như cũ: ở đó cái giá chỉ là CPU, và
//! cuộn tới nơi mà ảnh đã sẵn là đáng.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime};

use crate::media::thumbnail::ThumbError;

/// Đổi khi cách dựng ảnh đổi (mốc giây, bộ lọc co…), để ảnh cũ tự bị bỏ.
const PHIEN_BAN: &[u8] = b"ffthumb-v1";

/// Trần dung lượng của cả kho.
///
/// Một ảnh 192 px đo được 7–27 KB (ProRes 1080p và 4K). 256 MB là khoảng
/// 10.000 ảnh — cỡ toàn bộ số video không phải `.mp4`/`.webm` của thư viện
/// studio (~12.000 trên 310.395), mà chỉ một phần trong số đó cần ffmpeg.
/// Vượt trần thì phần bị đuổi là phần lâu nhất không ai nhìn.
const TRAN_BYTE: u64 = 256 * 1024 * 1024;

/// Dọn xuống còn chừng này khi vượt trần, để không phải dọn lại ở ngay lần
/// ghi kế tiếp.
const DON_CON: u64 = TRAN_BYTE / 4 * 3;

/// Cứ bấy nhiêu lần ghi thì kiểm trần một lần.
///
/// Kiểm trần phải liệt kê cả thư mục — vài nghìn tệp, vài chục mili giây.
/// Làm ở mỗi lần ghi là trả cái giá đó cho từng ảnh.
const KIEM_TRAN_MOI: u32 = 64;

/// Ảnh vừa được xem mà mtime đã cũ hơn chừng này thì làm mới mtime.
///
/// mtime là "lần cuối có người nhìn" cho việc đuổi ảnh cũ. Làm mới ở MỌI lần
/// đọc là một lần ghi metadata cho mỗi ô trên màn hình; một ngày là đủ mịn để
/// đuổi đúng thứ lâu không ai nhìn.
const LAM_MOI_SAU: Duration = Duration::from_secs(24 * 60 * 60);

/// Đầu tệp PNG. Tệp không mở đầu bằng nó là tệp hỏng — bỏ, không trả về.
const DAU_PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

static SO_LAN_GHI: AtomicU32 = AtomicU32::new(0);

/// Thư mục kho: `%LOCALAPPDATA%\MediaFinder\thumbs-ffmpeg`.
fn thu_muc() -> Option<PathBuf> {
    crate::index::persist::cache_dir()
        .ok()
        .map(|d| d.join("thumbs-ffmpeg"))
}

/// Ảnh đã dựng từ trước, nếu kho có.
///
/// [`crate::media::thumbnail`] gọi hàm này **trước** khi bắt Windows giải mã,
/// không phải sau: với một tệp ProRes, lượt giải mã của Windows là một lần mở
/// tệp và đọc phần đầu của nó chỉ để thất bại — đo trên NAS 62–159 ms mỗi
/// ảnh, so với vài mili giây khi kho trả lời trước.
pub fn tra(path: &str, co: u32) -> Option<Vec<u8>> {
    tra_trong(&thu_muc()?, path, co)
}

/// Dựng ảnh bằng ffmpeg rồi ghi vào kho — sau khi Windows chịu thua, hoặc
/// trước Windows với những tệp [`ffmpeg_truoc`] chọn.
///
/// Không tra kho — người gọi đã tra bằng [`tra`] trước lượt giải mã của shell.
/// Yêu cầu đoán trước cho tệp trên ổ mạng thì dừng ở đây với `Busy`.
pub fn dung(path: &str, co: u32, du_doan: bool) -> Result<Vec<u8>, ThumbError> {
    if khong_doc_de_doan(path, du_doan) {
        tracing::debug!("ảnh ffmpeg: không đoán trước trên ổ mạng {path}");
        return Err(ThumbError::Busy);
    }

    let png = crate::media::ffthumb::khung_hinh(path, co).ok_or(ThumbError::Unavailable)?;
    if let Some(d) = thu_muc() {
        luu_vao(&d, path, co, &png);
    }
    Ok(png)
}

/// Có hỏi ffmpeg TRƯỚC khi để Windows giải mã không.
///
/// Có, khi cả ba cùng đúng:
///
/// * **Tệp trên ổ mạng.** Ở đó lượt thử của Windows là đọc tệp qua NAS; ở ổ
///   trong máy nó rẻ, và thứ tự cũ (Windows trước) giữ nguyên.
/// * **Đuôi thuộc loại hay chứa codec dựng phim** — đúng danh sách mà xem
///   trước dùng ([`crate::media::ffstream::co_the_can_chuyen_ma`]). `.mp4` và
///   `.webm` (96% thư viện) không nằm trong đó, nên không đổi gì với chúng.
/// * **Máy có ffmpeg.** Không có thì chẳng có gì để hỏi trước.
///
/// Đo trên NAS của studio, ảnh chưa có ở đâu cả:
///
/// | Tệp `.mov` | Windows trước | ffmpeg trước |
/// |---|---|---|
/// | ProRes (2/3 số `.mov`) | 110–844 ms Windows thất bại, **rồi** ffmpeg | chỉ ffmpeg |
/// | H.264 | Windows dựng được, 232–395 ms | ffmpeg 213–255 ms |
///
/// Với loại Windows tự làm được, ffmpeg không chậm hơn; với ProRes thì bỏ
/// hẳn một lượt đọc qua mạng. ffmpeg chịu thua (codec mà bản tối giản không
/// có, như AV1) thì Windows vẫn được thử sau — không tệp nào mất ảnh vì
/// thứ tự này.
pub fn ffmpeg_truoc(path: &str) -> bool {
    crate::media::ffstream::co_the_can_chuyen_ma(path)
        && crate::ntfs::volume::la_o_mang(path)
        && crate::media::ffmpeg::co_san()
}

/// Luật ổ mạng — xem chú thích đầu tệp.
fn khong_doc_de_doan(path: &str, du_doan: bool) -> bool {
    du_doan && crate::ntfs::volume::la_o_mang(path)
}

/// Tên tệp trong kho cho một tệp nguồn ở trạng thái hiện tại của nó.
///
/// `None` khi không đọc được metadata — tệp đã biến mất, hoặc NAS không trả
/// lời; khi đó không có gì để tra lẫn để ghi.
fn ten_trong_kho(path: &str, co: u32) -> Option<String> {
    let md = std::fs::metadata(path).ok()?;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let mut h = blake3::Hasher::new();
    h.update(PHIEN_BAN);
    h.update(&[0]);
    h.update(path.to_lowercase().as_bytes());
    h.update(&[0]);
    h.update(&md.len().to_le_bytes());
    h.update(&mtime.to_le_bytes());
    h.update(&co.to_le_bytes());
    // 128 bit là quá thừa để hai tệp không đụng tên; tên ngắn thì liệt kê
    // thư mục nhanh hơn.
    Some(format!("{}.png", &h.finalize().to_hex()[..32]))
}

fn tra_trong(dir: &Path, path: &str, co: u32) -> Option<Vec<u8>> {
    let tep = dir.join(ten_trong_kho(path, co)?);
    let png = std::fs::read(&tep).ok()?;
    if !png.starts_with(DAU_PNG) {
        let _ = std::fs::remove_file(&tep);
        return None;
    }
    lam_moi_neu_cu(&tep);
    Some(png)
}

/// Đánh dấu "vừa có người nhìn" để lượt dọn không đuổi nhầm ảnh đang dùng.
fn lam_moi_neu_cu(tep: &Path) {
    let cu = std::fs::metadata(tep)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|tuoi| tuoi > LAM_MOI_SAU);
    if cu {
        if let Ok(f) = std::fs::File::options().write(true).open(tep) {
            let _ = f.set_modified(SystemTime::now());
        }
    }
}

fn luu_vao(dir: &Path, path: &str, co: u32, png: &[u8]) {
    let Some(ten) = ten_trong_kho(path, co) else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    // Tên tạm mang số tiến trình VÀ số lần ghi: bốn luồng cùng tiến trình có
    // thể ghi cùng một ảnh (hai cỡ, hay hai yêu cầu đua nhau), và dùng chung
    // một tên tạm thì `rename` sau có thể xuất bản một tệp trộn lẫn.
    let lan = SO_LAN_GHI.fetch_add(1, Ordering::Relaxed);
    let tam = dir.join(format!("{ten}.{}.{lan}.tmp", std::process::id()));
    let ok = std::fs::File::create(&tam)
        .and_then(|mut f| f.write_all(png))
        .is_ok();
    if !ok || std::fs::rename(&tam, dir.join(&ten)).is_err() {
        let _ = std::fs::remove_file(&tam);
        return;
    }
    if lan % KIEM_TRAN_MOI == 0 {
        thu_gon_trong(dir, TRAN_BYTE, DON_CON);
    }
}

/// Vượt `tran` thì đuổi ảnh lâu không ai nhìn nhất cho tới khi còn `con`.
///
/// Dọn luôn tệp tạm bỏ dở (tiến trình bị giết giữa lúc ghi) đã quá một giờ.
fn thu_gon_trong(dir: &Path, tran: u64, con: u64) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut anh: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    for e in rd.flatten() {
        let Ok(md) = e.metadata() else { continue };
        let luc = md.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let p = e.path();
        match p.extension().and_then(|x| x.to_str()) {
            Some("png") => anh.push((luc, md.len(), p)),
            Some("tmp") if luc.elapsed().is_ok_and(|t| t > Duration::from_secs(3600)) => {
                let _ = std::fs::remove_file(&p);
            }
            _ => {}
        }
    }
    let mut tong: u64 = anh.iter().map(|(_, n, _)| n).sum();
    if tong <= tran {
        return;
    }
    anh.sort_by_key(|(luc, _, _)| *luc);
    for (_, n, p) in anh {
        if tong <= con {
            break;
        }
        if std::fs::remove_file(&p).is_ok() {
            tong -= n;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thư mục riêng cho từng bài — không bao giờ đụng kho thật của người
    /// dùng (bài thử của `dupestore` từng ghi đè kho thật, xem ở đó).
    fn thu_muc_thu(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "mediafinder-ffthumbkho-{ten}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn png_gia(n: usize) -> Vec<u8> {
        let mut v = DAU_PNG.to_vec();
        v.resize(n.max(DAU_PNG.len()), 7);
        v
    }

    #[test]
    fn ghi_roi_doc_lai_duoc() {
        let d = thu_muc_thu("ghidoc");
        let nguon = d.join("nguon.mov");
        std::fs::write(&nguon, b"abc").unwrap();
        let nguon = nguon.to_str().unwrap();

        assert!(tra_trong(&d, nguon, 192).is_none());
        luu_vao(&d, nguon, 192, &png_gia(100));
        assert_eq!(tra_trong(&d, nguon, 192), Some(png_gia(100)));
        // Cỡ khác là ảnh khác.
        assert!(tra_trong(&d, nguon, 64).is_none());
        // Không để lại tệp tạm.
        let tam = std::fs::read_dir(&d)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "tmp"))
            .count();
        assert_eq!(tam, 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Tệp nguồn bị sửa thì ảnh cũ không còn đúng — phải trượt.
    #[test]
    fn tep_nguon_doi_thi_anh_cu_bi_bo_qua() {
        let d = thu_muc_thu("doi");
        let nguon = d.join("nguon.mov");
        std::fs::write(&nguon, b"abc").unwrap();
        let p = nguon.to_str().unwrap();
        luu_vao(&d, p, 192, &png_gia(100));

        std::fs::write(&nguon, b"abcdef").unwrap();
        assert!(tra_trong(&d, p, 192).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Hoa thường khác nhau vẫn là cùng một tệp trên Windows.
    #[test]
    fn duong_dan_khong_phan_biet_hoa_thuong() {
        let d = thu_muc_thu("hoathuong");
        let nguon = d.join("Nguon.MOV");
        std::fs::write(&nguon, b"abc").unwrap();
        let p = nguon.to_str().unwrap();
        luu_vao(&d, p, 192, &png_gia(100));
        assert!(tra_trong(&d, &p.to_uppercase(), 192).is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Tệp trong kho bị hỏng (không phải PNG) thì bỏ, không trả về.
    #[test]
    fn anh_hong_bi_bo() {
        let d = thu_muc_thu("hong");
        let nguon = d.join("nguon.mov");
        std::fs::write(&nguon, b"abc").unwrap();
        let p = nguon.to_str().unwrap();
        let ten = ten_trong_kho(p, 192).unwrap();
        std::fs::write(d.join(&ten), b"rac").unwrap();

        assert!(tra_trong(&d, p, 192).is_none());
        assert!(!d.join(&ten).exists(), "ảnh hỏng phải bị xoá");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Vượt trần thì ảnh lâu không ai nhìn đi trước; ảnh mới ở lại.
    #[test]
    fn vuot_tran_thi_duoi_anh_cu_nhat() {
        let d = thu_muc_thu("tran");
        let bay_gio = SystemTime::now();
        for i in 0..4u64 {
            let tep = d.join(format!("{i}.png"));
            std::fs::write(&tep, vec![0u8; 100]).unwrap();
            let f = std::fs::File::options().write(true).open(&tep).unwrap();
            // 0 là cũ nhất.
            f.set_modified(bay_gio - Duration::from_secs(1000 - i * 100))
                .unwrap();
        }
        thu_gon_trong(&d, 250, 200);
        let con: Vec<bool> = (0..4)
            .map(|i| d.join(format!("{i}.png")).exists())
            .collect();
        assert_eq!(con, vec![false, false, true, true]);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Chưa vượt trần thì không đụng gì.
    #[test]
    fn chua_vuot_tran_thi_giu_nguyen() {
        let d = thu_muc_thu("duoitran");
        for i in 0..3 {
            std::fs::write(d.join(format!("{i}.png")), vec![0u8; 100]).unwrap();
        }
        thu_gon_trong(&d, 1000, 500);
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 3);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Đoán trước trên ổ mạng thì không đọc; mọi tổ hợp khác thì đọc.
    #[test]
    fn chi_chan_doan_truoc_tren_o_mang() {
        let mang = r"\\khong-co-may-nay-7c2e\share\clip.mov";
        let trong_may = r"C:\clip.mov";
        assert!(khong_doc_de_doan(mang, true));
        assert!(
            !khong_doc_de_doan(mang, false),
            "ô đang hiện phải được dựng"
        );
        assert!(
            !khong_doc_de_doan(trong_may, true),
            "ổ trong máy vẫn đoán trước"
        );
        assert!(!khong_doc_de_doan(trong_may, false));
    }

    /// `dung` dừng ở luật ổ mạng TRƯỚC khi đụng tới tệp — nên đường UNC tới
    /// một máy không có thật trả `Busy` ngay, không có lượt dò máy chủ nào
    /// (`la_o_mang` nhận UNC là mạng mà không hỏi ai).
    #[test]
    fn doan_truoc_tren_o_mang_thi_khong_dong_toi_tep() {
        let t = std::time::Instant::now();
        let r = dung(r"\\khong-co-may-nay-7c2e\share\clip.mov", 192, true);
        assert!(matches!(r, Err(ThumbError::Busy)), "nhận được {r:?}");
        assert!(
            t.elapsed() < Duration::from_millis(200),
            "chậm bất thường — đã có lượt dò máy chủ"
        );
    }

    /// ffmpeg đi trước chỉ với `.mov`… trên ổ mạng, và chỉ khi máy có ffmpeg.
    /// `.mp4` và ổ trong máy giữ thứ tự cũ.
    #[test]
    fn ffmpeg_di_truoc_chi_voi_mov_tren_o_mang() {
        let co = crate::media::ffmpeg::co_san();
        assert_eq!(ffmpeg_truoc(r"\\nas-7c2e\share\clip.mov"), co);
        assert_eq!(ffmpeg_truoc(r"\\nas-7c2e\share\CLIP.MKV"), co);
        assert!(
            !ffmpeg_truoc(r"\\nas-7c2e\share\clip.mp4"),
            ".mp4 phải để Windows"
        );
        assert!(!ffmpeg_truoc(r"C:\clip.mov"), "ổ trong máy giữ thứ tự cũ");
    }

    /// Tệp không có thật: câu trả lời là "không có ảnh", không phải "bận".
    #[test]
    fn tep_khong_ton_tai_thi_khong_co_anh() {
        let p = std::env::temp_dir().join("mediafinder-khong-he-ton-tai-4b1d.mov");
        let r = dung(p.to_str().unwrap(), 192, true);
        assert!(matches!(r, Err(ThumbError::Unavailable)), "nhận được {r:?}");
        assert!(tra(p.to_str().unwrap(), 192).is_none());
    }
}
