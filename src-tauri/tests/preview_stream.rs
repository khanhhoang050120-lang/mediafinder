//! Phiên xem trước chạy thật: ffmpeg thật, tệp ProRes thật (tự tạo).
//!
//! Tự tạo tệp mẫu bằng chính ffmpeg thay vì dựa vào thư viện của một máy cụ
//! thể — nhờ vậy bài này chạy được ở bất kỳ đâu có ffmpeg. Máy không có
//! ffmpeg (CI) thì bỏ qua, vì ở đó tính năng này cũng không bật.
//!
//! Kiểm đúng những điều người dùng thấy:
//!   * mảnh đầu về đủ sớm để hình hiện ngay,
//!   * luồng ra là fMP4 hợp lệ và có **đủ** thời lượng, không cắt ở giây nào,
//!   * tua tới giữa video mở được một luồng mới từ đúng chỗ đó.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use mediafinder::media::{ffphien, ffstream};

/// Các bài ở đây dùng chung sổ phiên của cả tiến trình, và app không bao giờ
/// cho quá hai ffmpeg chạy cùng lúc — phiên thứ ba mở ra thì phiên cũ nhất bị
/// đóng. Đó là hành vi đúng của sản phẩm, nhưng khi `cargo test` chạy các bài
/// song song thì chúng đóng phiên của nhau và hỏng vì một lý do không liên
/// quan gì tới điều chúng kiểm. Khoá này nối tiếp chúng lại.
static TUAN_TU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lan_luot() -> std::sync::MutexGuard<'static, ()> {
    // Một bài hỏng không được kéo các bài sau hỏng theo.
    TUAN_TU.lock().unwrap_or_else(|e| e.into_inner())
}

/// Tạo một tệp ProRes 422 HQ `giay` giây, 1920×1080, có tiếng.
fn tao_prores(ten: &str, giay: u32) -> Option<PathBuf> {
    let ff = mediafinder::media::ffmpeg::duong_dan()?;
    let out = std::env::temp_dir().join(ten);
    if out.is_file() {
        return Some(out);
    }
    let st = std::process::Command::new(ff)
        .args(["-v", "error", "-y"])
        .args([
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc2=size=1920x1080:rate=25:duration={giay}"),
        ])
        .args([
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:duration={giay}"),
        ])
        .args([
            "-c:v",
            "prores_ks",
            "-profile:v",
            "3",
            "-pix_fmt",
            "yuv422p10le",
        ])
        .args(["-c:a", "pcm_s16le"])
        .arg(&out)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    st.success().then_some(out)
}

/// Đọc hết một phiên, trả về (thời điểm có dữ liệu đầu tiên, toàn bộ byte).
fn doc_het(id: u64) -> (Option<Duration>, Vec<u8>) {
    let t0 = Instant::now();
    let mut dau = None;
    let mut tat_ca = Vec::new();
    loop {
        match ffphien::doc(id, tat_ca.len(), Duration::from_secs(5)) {
            ffphien::KetQuaDoc::Tiep(d) => {
                if !d.is_empty() && dau.is_none() {
                    dau = Some(t0.elapsed());
                }
                tat_ca.extend(d);
            }
            ffphien::KetQuaDoc::Het(d) => {
                if !d.is_empty() && dau.is_none() {
                    dau = Some(t0.elapsed());
                }
                tat_ca.extend(d);
                break;
            }
            ffphien::KetQuaDoc::MatPhien => panic!("phiên mất giữa chừng"),
        }
    }
    (dau, tat_ca)
}

/// Đếm hộp `mdat` cấp cao nhất — mỗi hộp là một mảnh phát được.
fn dem_manh(b: &[u8]) -> usize {
    let (mut o, mut n) = (0usize, 0usize);
    while o + 8 <= b.len() {
        let co = u32::from_be_bytes(b[o..o + 4].try_into().unwrap()) as usize;
        assert!(co >= 8 && o + co <= b.len(), "luồng fMP4 hỏng ở byte {o}");
        if &b[o + 4..o + 8] == b"mdat" {
            n += 1;
        }
        o += co;
    }
    n
}

/// Thời lượng thật của một luồng MP4, hỏi ffprobe.
fn do_dai(b: &[u8], ten: &str) -> Option<f64> {
    let p = std::env::temp_dir().join(ten);
    std::fs::write(&p, b).ok()?;
    let mut cmd = mediafinder::media::ffmpeg::lenh_ffprobe()?;
    cmd.args([
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "default=nw=1:nk=1",
    ])
    .arg(&p);
    let out = cmd.output().ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

#[test]
fn phien_giao_du_ca_video_khong_cat_o_giay_nao() {
    let _lan_luot = lan_luot();
    let Some(tep) = tao_prores("mf-xem-truoc-8s.mov", 8) else {
        eprintln!("bỏ qua: máy này không có ffmpeg");
        return;
    };
    let tep = tep.to_string_lossy().to_string();

    let ffstream::KeHoach::ChuyenMa(tt) = ffstream::ke_hoach(&tep, false) else {
        panic!("ProRes phải đi đường chuyển mã");
    };
    assert_eq!(tt.codec, "prores");
    assert!(
        (tt.thoi_luong - 8.0).abs() < 0.1,
        "thời lượng {}",
        tt.thoi_luong
    );

    let p = ffphien::mo(&tep, 0.0).expect("phải mở được phiên");
    let mime = ffphien::cho_mime(&p, Duration::from_secs(20)).expect("phải có moov");
    assert!(mime.starts_with("video/mp4; codecs=\"avc1."), "{mime}");
    assert!(
        mime.contains("mp4a.40.2"),
        "có tiếng thì phải khai tiếng: {mime}"
    );

    let (dau, du_lieu) = doc_het(p.id);
    ffphien::dong(p.id);

    let dau = dau.expect("phải có dữ liệu");
    eprintln!("byte đầu sau {dau:?}, tổng {} KB", du_lieu.len() / 1024);
    // Giây đầu 10 mảnh (0,1 giây), sau đó 0,5 giây: 8 giây cho khoảng 24 mảnh.
    // Một mảnh duy nhất nghĩa là khung khoá dày đã mất — đúng cái làm hình hiện
    // chậm.
    let n = dem_manh(&du_lieu);
    assert!(n >= 20, "chỉ có {n} mảnh — khung khoá không còn dày");

    // Đủ độ dài: đây chính là lỗi "phát 5 giây rồi dừng".
    let d = do_dai(&du_lieu, "mf-xem-truoc-8s-ra.mp4").expect("ffprobe đọc được đầu ra");
    assert!(d > 7.5, "luồng ra chỉ dài {d}s trên 8s — bị cắt");
}

#[test]
fn tua_toi_giua_mo_duoc_luong_moi_tu_dung_cho_do() {
    let _lan_luot = lan_luot();
    let Some(tep) = tao_prores("mf-xem-truoc-tua-6s.mov", 6) else {
        eprintln!("bỏ qua: máy này không có ffmpeg");
        return;
    };
    let tep = tep.to_string_lossy().to_string();
    let ffstream::KeHoach::ChuyenMa(_) = ffstream::ke_hoach(&tep, false) else {
        panic!("ProRes phải đi đường chuyển mã");
    };

    let p = ffphien::mo(&tep, 4.0).expect("phải mở được phiên tua");
    assert!((p.tu_giay - 4.0).abs() < 1e-9);
    assert!(ffphien::cho_mime(&p, Duration::from_secs(20)).is_some());
    let (_, du_lieu) = doc_het(p.id);
    ffphien::dong(p.id);

    // Từ giây 4 của video 6 giây: còn khoảng 2 giây, không phải 6.
    let d = do_dai(&du_lieu, "mf-xem-truoc-tua-ra.mp4").expect("ffprobe đọc được");
    assert!((1.5..=2.5).contains(&d), "phiên tua dài {d}s, phải ~2s");
}

#[test]
fn mo_lai_cung_tep_cung_moc_thi_dung_lai_phien() {
    let _lan_luot = lan_luot();
    let Some(tep) = tao_prores("mf-xem-truoc-dunglai-3s.mov", 3) else {
        eprintln!("bỏ qua: máy này không có ffmpeg");
        return;
    };
    let tep = tep.to_string_lossy().to_string();
    let ffstream::KeHoach::ChuyenMa(_) = ffstream::ke_hoach(&tep, false) else {
        panic!("ProRes phải đi đường chuyển mã");
    };
    // Đây là cách phiên chuẩn bị sẵn lúc người dùng dừng ở dòng trở thành
    // phiên họ xem khi bấm vào — không chuyển mã lại từ đầu.
    let a = ffphien::mo(&tep, 0.0).expect("mở lần 1");
    let b = ffphien::mo(&tep, 0.0).expect("mở lần 2");
    assert_eq!(a.id, b.id, "cùng tệp cùng mốc phải dùng lại phiên");
    ffphien::dong(a.id);
}

/// Tệp không có tiếng — phần lớn stock footage — vẫn phải ra hình, và chuỗi codec
/// không được khai âm thanh (khai `mp4a` cho luồng không có tiếng thì Media
/// Source từ chối cả video).
#[test]
fn tep_khong_tieng_van_ra_hinh_va_khong_khai_tieng() {
    let _lan_luot = lan_luot();
    let Some(ff) = mediafinder::media::ffmpeg::duong_dan() else {
        eprintln!("bỏ qua: máy này không có ffmpeg");
        return;
    };
    let out = std::env::temp_dir().join("mf-xem-truoc-khongtieng-3s.mov");
    if !out.is_file() {
        let ok = std::process::Command::new(ff)
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=1280x720:rate=25:duration=3",
            ])
            .args(["-c:v", "prores_ks", "-profile:v", "3"])
            .arg(&out)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        assert!(ok, "không tạo được tệp mẫu");
    }
    let tep = out.to_string_lossy().to_string();
    let p = ffphien::mo(&tep, 0.0).expect("phải mở được phiên");
    let mime =
        ffphien::cho_mime(&p, Duration::from_secs(20)).expect("tệp không tiếng vẫn phải có moov");
    assert!(
        !mime.contains("mp4a"),
        "không tiếng thì không được khai tiếng: {mime}"
    );
    let (_, du_lieu) = doc_het(p.id);
    ffphien::dong(p.id);
    assert!(dem_manh(&du_lieu) >= 4, "phải ra hình");
}

/// Phiên chuẩn bị sẵn chưa ai xem chỉ được làm vài giây đầu rồi đứng lại.
///
/// Trước đây nó làm trọn tệp: rê chuột 250 ms lên một dòng là app đọc cả tệp
/// 1–4 GB, kể cả trên NAS cả studio dùng chung. Có người mở xem thì phải chạy
/// tiếp tới hết — không được kẹt ở trần của lúc chưa ai xem.
#[test]
fn phien_chua_ai_xem_chi_lam_vai_giay_dau_roi_dung() {
    let _lan_luot = lan_luot();
    let Some(ff) = mediafinder::media::ffmpeg::duong_dan() else {
        eprintln!("bỏ qua: máy này không có ffmpeg");
        return;
    };
    // Nhiễu hạt: nén kém, nên đầu ra 6 giây vượt xa trần 1 MB.
    let out = std::env::temp_dir().join("mf-xem-truoc-nhieu-6s.mov");
    if !out.is_file() {
        let ok = std::process::Command::new(ff)
            .args(["-v", "error", "-y", "-f", "lavfi"])
            .args([
                "-i",
                "testsrc2=size=1280x720:rate=25:duration=6,noise=alls=60:allf=t",
            ])
            .args(["-c:v", "prores_ks", "-profile:v", "3"])
            .arg(&out)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        assert!(ok, "không tạo được tệp mẫu");
    }
    let tep = out.to_string_lossy().to_string();
    let p = ffphien::mo(&tep, 0.0).expect("phải mở được phiên");

    // KHÔNG đọc gì — đúng như lúc con trỏ chỉ dừng trên dòng. Chờ tới khi số
    // byte đứng yên.
    let (mut truoc, mut yen) = (usize::MAX, 0);
    for _ in 0..80 {
        std::thread::sleep(Duration::from_millis(250));
        let n = p.so_byte();
        if n == truoc {
            yen += 1;
            if yen >= 4 {
                break;
            }
        } else {
            yen = 0;
            truoc = n;
        }
    }
    let dung_o = p.so_byte();
    assert!(dung_o > 0, "phiên chuẩn bị sẵn phải làm được phần đầu");
    // Trần 1 MB, cộng một lần đọc 256 KB đã ở trong tay luồng nền.
    assert!(
        dung_o <= 1024 * 1024 + 300 * 1024,
        "phiên chưa ai xem làm tới {} KB — không dừng ở trần",
        dung_o / 1024
    );
    assert!(!p.da_dong(), "chưa tới hạn tự dừng thì phiên phải còn");

    // Có người xem: trần nới ra, chạy tiếp tới hết.
    let (_, du_lieu) = doc_het(p.id);
    ffphien::dong(p.id);
    eprintln!(
        "dừng ở {} KB khi chưa ai xem, trọn {} KB khi xem",
        dung_o / 1024,
        du_lieu.len() / 1024
    );
    assert!(
        du_lieu.len() > dung_o * 2,
        "mở xem rồi mà không chạy tiếp: {} KB",
        du_lieu.len() / 1024
    );
}
