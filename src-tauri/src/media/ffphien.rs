//! Phiên xem trước chuyển mã: ffmpeg chạy nền, trang lấy dần từng mảnh.
//!
//! # Vì sao là phiên, không phải một lời gọi
//!
//! Một lời gọi thì phải chuyển mã **xong** rồi mới trả về được — đó là lý do
//! bản trước bắt người dùng chờ, rồi để giảm chờ lại cắt video còn 5 giây,
//! và video dừng hẳn ở giây thứ 5. Sai cả hai đầu.
//!
//! Phiên thì khác: ffmpeg chạy, từng byte nó xuất ra được nối vào bộ nhớ ngay
//! khi về, và trang hỏi "từ byte N trở đi có gì chưa?" bao nhiêu lần tuỳ ý.
//! Mảnh đầu về sau ~0,4 giây là trình phát có cái để chiếu; phần còn lại
//! tiếp tục về trong lúc người dùng xem.
//!
//! # Tự kìm khi chạy trước quá xa
//!
//! Luồng nền ngừng đọc ffmpeg khi dữ liệu chưa ai lấy vượt trần. Ống đầy thì
//! ffmpeg đứng ở lệnh ghi — tức **CPU và đọc đĩa tự dừng**, không cần tín hiệu
//! gì. Hai trần, vì hai loại phiên khác nhau:
//!
//! * **Chưa ai xem** — phiên chuẩn bị sẵn khi con trỏ dừng trên dòng:
//!   [`DI_TRUOC_CHUA_XEM`], chỉ đủ cho vài giây đầu. Đó là thứ duy nhất cần có
//!   sẵn để hình hiện ngay lúc bấm; phần còn lại tải tiếp trong lúc xem.
//! * **Đang có người xem** — [`DI_TRUOC_TOI_DA`], đủ rộng để trình phát không
//!   bao giờ đói.
//!
//! Bản trước chỉ có trần thứ hai, và vì bản chuyển mã trọn của stock footage nằm
//! dưới trần đó, **chỉ cần rê chuột 250 ms lên một dòng là app đọc trọn tệp
//! 1–4 GB** — kể cả trên NAS mà cả studio dùng chung, kể cả khi không ai mở xem.
//!
//! # Dọn dẹp
//!
//! Phiên đang bị kìm mà không ai quay lại thì **tự dừng** ([`BO_KHI_CHUA_XEM`],
//! [`BO_KHI_NGOI_KHONG`]): mỗi tiến trình giải mã ProRes 4K giữ vài trăm MB bộ đệm
//! khung hình kể cả khi đang đứng im. Trước đây việc dọn chỉ xảy ra lúc có phiên
//! mới được mở, nên một phiên bị bỏ rơi có thể nằm đó tới khi tắt app. Không
//! bao giờ có quá [`SO_PHIEN_CHAY`] tiến trình ffmpeg cùng lúc.

use std::io::Read;
use std::process::{Child, ChildStdout};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};

/// Byte chưa ai lấy mà ffmpeg được phép chạy trước, khi đang có người xem.
///
/// 48 MB: rộng hơn bản chuyển mã trọn của mọi tệp đo được trong thư viện
/// (3,6 MB cho 4K 12 giây, 22 MB cho 6K 16,5 giây). Trang vốn lấy liên tục
/// nên hiếm khi chạm trần này; nó chỉ chặn một video dài chất đầy bộ nhớ.
const DI_TRUOC_TOI_DA: usize = 48 * 1024 * 1024;

/// Byte một phiên **chưa ai xem** được làm sẵn.
///
/// 1 MB đầu ra là khoảng 1–3 giây video ở 720p (bản chuyển mã nặng 0,3–1,3
/// MB/giây tuỳ độ hạt của nguồn) — dư để hình hiện ngay lúc bấm, và chặn ở
/// vài giây nguồn thay vì cả tệp 1–4 GB.
const DI_TRUOC_CHUA_XEM: usize = 1024 * 1024;

/// Phiên đang xem mà không ai đọc quá lâu thì đóng.
///
/// Trang lấy dữ liệu liên tục tới hết tệp, nên im lặng chừng này nghĩa là lớp
/// xem trước đã đóng mà lệnh đóng không tới được.
const BO_KHI_NGOI_KHONG: Duration = Duration::from_secs(45);

/// Phiên chuẩn bị sẵn mà không ai mở xem trong chừng này thì đóng.
///
/// Rộng hơn khoảng dừng trên một dòng trước khi bấm xem; ngắn hơn nhiều so với
/// để một ffmpeg đứng im giữ vài trăm MB bộ nhớ. Rê chuột lại vào dòng thì
/// đồng hồ tính lại từ đầu.
const BO_KHI_CHUA_XEM: Duration = Duration::from_secs(15);

/// Tối đa bấy nhiêu tiến trình ffmpeg cùng chạy.
///
/// Hai: phiên đang xem, cộng một phiên chuẩn bị sẵn hoặc một phiên tua. Thêm
/// nữa chỉ chia CPU ra mỏng hơn — mọi phiên cùng chậm.
const SO_PHIEN_CHAY: usize = 2;

/// Một lần đọc trả nhiều nhất bấy nhiêu byte.
const MOI_LAN_DOC: usize = 4 * 1024 * 1024;

/// Một lượt chuyển mã, từ một mốc của một tệp.
pub struct Phien {
    pub id: u64,
    pub path: String,
    /// Giây trong tệp gốc mà đầu ra bắt đầu từ đó.
    pub tu_giay: f64,
    tt: Mutex<TrangThai>,
    cv: Condvar,
    con: Mutex<Option<Child>>,
}

struct TrangThai {
    du_lieu: Vec<u8>,
    /// ffmpeg đã xuất hết.
    xong: bool,
    /// Người dùng đã đóng, hoặc phiên bị dọn.
    dong: bool,
    /// Byte xa nhất đã được lấy — mốc để tính "chạy trước bao xa".
    da_doc_toi: usize,
    /// Đã có ai lấy dữ liệu chưa. Chưa thì đây là phiên chuẩn bị sẵn, và
    /// chỉ được làm tới [`DI_TRUOC_CHUA_XEM`].
    co_nguoi_xem: bool,
    lan_cuoi: Instant,
    /// Chuỗi codec khai cho Media Source, đọc từ hộp `moov`.
    mime: Option<String>,
}

/// Kết quả một lần đọc.
#[derive(Debug, PartialEq, Eq)]
pub enum KetQuaDoc {
    /// Có (hoặc tạm chưa có) dữ liệu, và còn nữa.
    Tiep(Vec<u8>),
    /// Đoạn cuối cùng — sau đoạn này không còn gì.
    Het(Vec<u8>),
    /// Phiên không còn: đã đóng, bị dọn, hoặc ffmpeg hỏng không ra gì.
    MatPhien,
}

static CAC_PHIEN: Mutex<Vec<Arc<Phien>>> = Mutex::new(Vec::new());
static SO_HIEU: AtomicU64 = AtomicU64::new(1);

/// Mở (hoặc dùng lại) một phiên cho `path` từ giây `tu_giay`.
///
/// Dùng lại khi đã có phiên cùng tệp cùng mốc — đó là cách một phiên chuẩn bị
/// sẵn (con trỏ dừng trên dòng, cú nhấn chuột đầu của double-click) trở thành
/// phiên người dùng xem khi bấm vào, không phải chuyển mã lại từ đầu.
///
/// # Kiểm tra và khởi động trong CÙNG một lần khoá
///
/// Di chuột vào dòng rồi nhấn xuống là hai lời xin chuẩn bị sẵn cách nhau vài
/// chục mili giây. "Kiểm tra chưa có, thả khoá, rồi khởi động" thì cả hai cùng
/// thấy chưa có và cùng khởi động — hai ffmpeg giải mã cùng một tệp 4K, cả hai
/// cùng chậm. Giữ khoá qua lúc khởi động tốn chừng vài chục mili giây, và
/// không ai khác cần khoá này lâu hơn thế.
///
/// `None` khi không chạy được ffmpeg.
pub fn mo(path: &str, tu_giay: f64) -> Option<Arc<Phien>> {
    don_dep();

    // Đã chuyển mã trọn tệp này rồi: không cần ffmpeg nữa.
    if tu_giay < 0.05 {
        if let Some(b) = crate::media::ffcache::tra(path) {
            return Some(mo_tu_byte(path, b.as_ref().clone()));
        }
    }

    let (p, bo) = {
        let mut ds = CAC_PHIEN.lock();
        if let Some(p) = ds
            .iter()
            .find(|p| p.path == path && (p.tu_giay - tu_giay).abs() < 0.05 && !p.tt.lock().dong)
        {
            p.tt.lock().lan_cuoi = Instant::now();
            return Some(Arc::clone(p));
        }

        let bo = chon_nhuong_cho(&mut ds);
        let Some(mut con) = crate::media::ffstream::bat_dau(path, tu_giay) else {
            // Không khởi động được: trả lại những phiên vừa bị chọn nhường chỗ.
            ds.extend(bo);
            return None;
        };
        let Some(out) = con.stdout.take() else {
            let _ = con.kill();
            ds.extend(bo);
            return None;
        };
        let p = Arc::new(Phien {
            id: SO_HIEU.fetch_add(1, Ordering::Relaxed),
            path: path.to_string(),
            tu_giay,
            tt: Mutex::new(TrangThai {
                du_lieu: Vec::new(),
                xong: false,
                dong: false,
                da_doc_toi: 0,
                co_nguoi_xem: false,
                lan_cuoi: Instant::now(),
                mime: None,
            }),
            cv: Condvar::new(),
            con: Mutex::new(Some(con)),
        });
        let nen = Arc::clone(&p);
        if std::thread::Builder::new()
            .name(format!("xem-truoc-{}", p.id))
            .spawn(move || doc_nen(nen, out))
            .is_err()
        {
            // Không có ai đọc thì ffmpeg đứng mãi ở ống đầy: dừng nó ngay.
            ket_thuc(&p);
            ds.extend(bo);
            return None;
        }
        ds.push(Arc::clone(&p));
        (p, bo)
    };
    // Dừng tiến trình của những phiên bị nhường chỗ — ngoài khoá, vì đợi một
    // tiến trình thoát không phải việc của người đang giữ khoá chung.
    for b in &bo {
        tracing::debug!("xem trước: nhường chỗ, đóng phiên {} ({})", b.id, b.path);
        ket_thuc(b);
    }
    Some(p)
}

/// Phiên đã xong sẵn, dựng từ bản chuyển mã trọn trong kho.
fn mo_tu_byte(path: &str, du_lieu: Vec<u8>) -> Arc<Phien> {
    let mime = doc_mime(&du_lieu);
    let p = Arc::new(Phien {
        id: SO_HIEU.fetch_add(1, Ordering::Relaxed),
        path: path.to_string(),
        tu_giay: 0.0,
        tt: Mutex::new(TrangThai {
            du_lieu,
            xong: true,
            dong: false,
            da_doc_toi: 0,
            co_nguoi_xem: false,
            lan_cuoi: Instant::now(),
            mime,
        }),
        cv: Condvar::new(),
        con: Mutex::new(None),
    });
    CAC_PHIEN.lock().push(Arc::clone(&p));
    p
}

/// Luồng nền: chép đầu ra của ffmpeg vào bộ nhớ của phiên.
fn doc_nen(p: Arc<Phien>, mut out: ChildStdout) {
    let mut tam = vec![0u8; 256 * 1024];
    loop {
        // Kìm lại khi đã chạy trước người đọc quá xa. Không đọc thì ống đầy,
        // và ffmpeg tự đứng ở lệnh ghi — CPU dừng theo.
        {
            let mut tt = p.tt.lock();
            loop {
                if tt.dong {
                    return;
                }
                let (tran, han) = if tt.co_nguoi_xem {
                    (DI_TRUOC_TOI_DA, BO_KHI_NGOI_KHONG)
                } else {
                    (DI_TRUOC_CHUA_XEM, BO_KHI_CHUA_XEM)
                };
                if tt.du_lieu.len().saturating_sub(tt.da_doc_toi) <= tran {
                    break;
                }
                if tt.lan_cuoi.elapsed() > han {
                    // Không ai quay lại: tự dừng, trả CPU và bộ nhớ. Đánh dấu đóng
                    // để `mo` không dùng lại nó và `don_dep` gỡ nó ra lần sau.
                    tt.dong = true;
                    drop(tt);
                    if let Some(mut c) = p.con.lock().take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                    p.cv.notify_all();
                    return;
                }
                p.cv.wait_for(&mut tt, Duration::from_millis(500));
            }
        }
        let n = match out.read(&mut tam) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let mut tt = p.tt.lock();
        tt.du_lieu.extend_from_slice(&tam[..n]);
        if tt.mime.is_none() {
            tt.mime = doc_mime(&tt.du_lieu);
        }
        drop(tt);
        p.cv.notify_all();
    }

    let thanh_cong = p
        .con
        .lock()
        .take()
        .and_then(|mut c| c.wait().ok())
        .is_some_and(|s| s.success());

    let mut tt = p.tt.lock();
    tt.xong = true;
    // Chuyển mã trọn từ đầu tệp: nhớ lại, để mở lại tệp này là tức thì.
    if thanh_cong && !tt.dong && p.tu_giay < 0.05 && !tt.du_lieu.is_empty() {
        crate::media::ffcache::luu(&p.path, Arc::new(tt.du_lieu.clone()));
    }
    drop(tt);
    p.cv.notify_all();
}

impl Phien {
    /// Số byte đã chuyển mã xong — không đánh dấu phiên là "có người xem",
    /// nên kiểm thử nhìn được một phiên chuẩn bị sẵn mà không làm đổi nó.
    pub fn so_byte(&self) -> usize {
        self.tt.lock().du_lieu.len()
    }

    /// Phiên đã đóng (bởi người dùng, hoặc tự dừng vì không ai quay lại).
    pub fn da_dong(&self) -> bool {
        self.tt.lock().dong
    }
}

/// Chờ tới khi biết chuỗi codec (hộp `moov` đã về), hoặc hết hạn.
///
/// `moov` ra ngay sau khi ffmpeg giải mã được khung hình đầu tiên — thường
/// dưới nửa giây. Hết hạn nghĩa là tệp không chuyển mã được.
pub fn cho_mime(p: &Phien, cho: Duration) -> Option<String> {
    let han = Instant::now() + cho;
    let mut tt = p.tt.lock();
    while tt.mime.is_none() && !tt.xong && !tt.dong {
        if p.cv.wait_until(&mut tt, han).timed_out() {
            break;
        }
    }
    tt.mime.clone()
}

/// Lấy dữ liệu từ byte `tu_byte` trở đi.
///
/// Chờ tối đa `cho` nếu chưa có gì mới. Hết hạn mà vẫn chưa có thì trả đoạn
/// rỗng kèm "còn nữa" — trang hỏi lại, không coi đó là lỗi.
pub fn doc(id: u64, tu_byte: usize, cho: Duration) -> KetQuaDoc {
    let Some(p) = tim(id) else {
        return KetQuaDoc::MatPhien;
    };
    let han = Instant::now() + cho;
    let mut tt = p.tt.lock();
    tt.lan_cuoi = Instant::now();
    // Lần đọc đầu tiên biến phiên chuẩn bị sẵn thành phiên đang xem: trần nới ra,
    // và luồng nền đang bị kìm được đánh thức ở `notify_all` bên dưới.
    tt.co_nguoi_xem = true;
    while tt.du_lieu.len() <= tu_byte && !tt.xong && !tt.dong {
        if p.cv.wait_until(&mut tt, han).timed_out() {
            break;
        }
    }
    if tt.dong || (tt.xong && tt.du_lieu.is_empty()) {
        return KetQuaDoc::MatPhien;
    }
    let het = tt.du_lieu.len().min(tu_byte.saturating_add(MOI_LAN_DOC));
    let doan = if tu_byte < het {
        tt.du_lieu[tu_byte..het].to_vec()
    } else {
        Vec::new()
    };
    tt.da_doc_toi = tt.da_doc_toi.max(het);
    let con_nua = !(tt.xong && het >= tt.du_lieu.len());
    drop(tt);
    // Người đọc vừa lấy đi một đoạn: luồng nền có thể đang bị kìm, đánh thức.
    p.cv.notify_all();
    if con_nua {
        KetQuaDoc::Tiep(doan)
    } else {
        KetQuaDoc::Het(doan)
    }
}

/// Đóng phiên và dừng ffmpeg của nó.
pub fn dong(id: u64) {
    let p = {
        let mut ds = CAC_PHIEN.lock();
        let Some(i) = ds.iter().position(|p| p.id == id) else {
            return;
        };
        ds.remove(i)
    };
    ket_thuc(&p);
}

fn ket_thuc(p: &Phien) {
    p.tt.lock().dong = true;
    if let Some(mut c) = p.con.lock().take() {
        let _ = c.kill();
        let _ = c.wait();
    }
    p.cv.notify_all();
}

fn tim(id: u64) -> Option<Arc<Phien>> {
    CAC_PHIEN.lock().iter().find(|p| p.id == id).cloned()
}

/// Đóng những phiên đã bị bỏ rơi.
fn don_dep() {
    let bo: Vec<Arc<Phien>> = {
        let mut ds = CAC_PHIEN.lock();
        let (bo, giu): (Vec<_>, Vec<_>) = ds.drain(..).partition(|p| {
            let tt = p.tt.lock();
            tt.dong || tt.lan_cuoi.elapsed() > BO_KHI_NGOI_KHONG
        });
        *ds = giu;
        bo
    };
    for p in &bo {
        ket_thuc(p);
    }
}

/// Chọn những phiên phải nhường chỗ cho một tiến trình mới: những phiên còn
/// chạy lâu nhất không ai đụng tới, cho tới khi còn dưới [`SO_PHIEN_CHAY`].
///
/// Làm trên danh sách người gọi đang khoá, và chỉ **gỡ** chúng ra — việc dừng
/// tiến trình để người gọi làm sau khi thả khoá.
fn chon_nhuong_cho(ds: &mut Vec<Arc<Phien>>) -> Vec<Arc<Phien>> {
    let mut chay: Vec<(Instant, u64)> = ds
        .iter()
        .filter_map(|p| {
            let tt = p.tt.lock();
            (!tt.xong && !tt.dong).then_some((tt.lan_cuoi, p.id))
        })
        .collect();
    chay.sort();
    let thua = (chay.len() + 1).saturating_sub(SO_PHIEN_CHAY);
    let ids: Vec<u64> = chay.iter().take(thua).map(|&(_, id)| id).collect();
    let (bo, giu): (Vec<_>, Vec<_>) = ds.drain(..).partition(|p| ids.contains(&p.id));
    *ds = giu;
    bo
}

/// Chuỗi MIME kèm codec cho Media Source, đọc từ hộp `moov` của fMP4.
///
/// Đọc thật chứ không khai cố định: x264 tự chọn profile theo tham số và
/// theo nguồn, và trình phát cần đúng chuỗi đó. `None` khi `moov` chưa về
/// trọn, hoặc không có luồng H.264.
pub fn doc_mime(b: &[u8]) -> Option<String> {
    let mut o = 0usize;
    while o + 8 <= b.len() {
        let co = u32::from_be_bytes(b[o..o + 4].try_into().ok()?) as usize;
        if co < 8 {
            return None;
        }
        if o + co > b.len() {
            return None; // hộp chưa về trọn
        }
        if &b[o + 4..o + 8] == b"moov" {
            let moov = &b[o..o + co];
            let avc = tim_con(moov, b"avcC")?;
            // avcC: [cỡ 4][`avcC` 4][phiên bản 1][profile][tương thích][level]
            if avc + 8 > moov.len() {
                return None;
            }
            let mut codecs = vec![format!(
                "avc1.{:02X}{:02X}{:02X}",
                moov[avc + 5],
                moov[avc + 6],
                moov[avc + 7]
            )];
            if tim_con(moov, b"mp4a").is_some() {
                // AAC-LC — thứ bộ mã hoá `aac` của ffmpeg luôn xuất ra.
                codecs.push("mp4a.40.2".into());
            }
            return Some(format!("video/mp4; codecs=\"{}\"", codecs.join(", ")));
        }
        o += co;
    }
    None
}

/// Vị trí của mã hộp `loai` (4 byte) trong `b`.
fn tim_con(b: &[u8], loai: &[u8; 4]) -> Option<usize> {
    b.windows(4).position(|w| w == loai)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dựng một hộp MP4: [cỡ][loại][nội dung].
    fn hop(loai: &[u8; 4], noi_dung: &[u8]) -> Vec<u8> {
        let mut v = ((8 + noi_dung.len()) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(loai);
        v.extend_from_slice(noi_dung);
        v
    }

    fn fmp4_gia(co_tieng: bool) -> Vec<u8> {
        // avcC: phiên bản 1, profile High (0x64), tương thích 0x00, level 3.1 (0x1F)
        let mut trong = hop(b"avcC", &[1, 0x64, 0x00, 0x1F, 0xFF, 0xE1]);
        if co_tieng {
            trong.extend(hop(b"mp4a", &[0; 8]));
        }
        let mut v = hop(b"ftyp", b"iso5\0\0\x02\0");
        v.extend(hop(b"moov", &trong));
        v.extend(hop(b"moof", &[0; 16]));
        v.extend(hop(b"mdat", &[7; 64]));
        v
    }

    #[test]
    fn doc_duoc_chuoi_codec_tu_moov() {
        assert_eq!(
            doc_mime(&fmp4_gia(false)).as_deref(),
            Some("video/mp4; codecs=\"avc1.64001F\"")
        );
        assert_eq!(
            doc_mime(&fmp4_gia(true)).as_deref(),
            Some("video/mp4; codecs=\"avc1.64001F, mp4a.40.2\"")
        );
    }

    #[test]
    fn moov_chua_ve_tron_thi_chua_tra_loi() {
        let b = fmp4_gia(false);
        // Cắt giữa hộp moov.
        let moov = b.windows(4).position(|w| w == b"moov").unwrap();
        assert_eq!(doc_mime(&b[..moov + 6]), None);
    }

    /// Đọc lần lượt từng đoạn phải ghép lại đúng dữ liệu gốc, và chỉ đoạn
    /// cuối mới mang dấu "hết".
    #[test]
    fn doc_tung_doan_ghep_lai_dung_ban_goc() {
        let goc: Vec<u8> = (0..(MOI_LAN_DOC * 2 + 123))
            .map(|i| (i % 251) as u8)
            .collect();
        let p = mo_tu_byte("tep-thu-doc.mov", goc.clone());

        let mut ghep = Vec::new();
        let mut so_het = 0;
        loop {
            match doc(p.id, ghep.len(), Duration::from_millis(10)) {
                KetQuaDoc::Tiep(d) => ghep.extend(d),
                KetQuaDoc::Het(d) => {
                    ghep.extend(d);
                    so_het += 1;
                    break;
                }
                KetQuaDoc::MatPhien => panic!("phiên không được mất giữa chừng"),
            }
        }
        assert_eq!(ghep, goc);
        assert_eq!(so_het, 1);
        dong(p.id);
    }

    #[test]
    fn dong_roi_thi_doc_bao_mat_phien() {
        let p = mo_tu_byte("tep-thu-dong.mov", vec![1, 2, 3]);
        dong(p.id);
        assert_eq!(doc(p.id, 0, Duration::from_millis(10)), KetQuaDoc::MatPhien);
    }

    #[test]
    fn chua_co_du_lieu_thi_cho_roi_tra_rong_con_nua() {
        // Phiên đang chạy nhưng ffmpeg chưa xuất gì: trang phải nhận "chưa có,
        // hỏi lại", không phải "hết" hay "mất phiên".
        let p = Arc::new(Phien {
            id: SO_HIEU.fetch_add(1, Ordering::Relaxed),
            path: "tep-cho.mov".into(),
            tu_giay: 0.0,
            tt: Mutex::new(TrangThai {
                du_lieu: Vec::new(),
                xong: false,
                dong: false,
                da_doc_toi: 0,
                co_nguoi_xem: false,
                lan_cuoi: Instant::now(),
                mime: None,
            }),
            cv: Condvar::new(),
            con: Mutex::new(None),
        });
        CAC_PHIEN.lock().push(Arc::clone(&p));
        let t = Instant::now();
        assert_eq!(
            doc(p.id, 0, Duration::from_millis(50)),
            KetQuaDoc::Tiep(Vec::new())
        );
        assert!(
            t.elapsed() >= Duration::from_millis(40),
            "phải chờ chứ không trả ngay"
        );
        dong(p.id);
    }
}
