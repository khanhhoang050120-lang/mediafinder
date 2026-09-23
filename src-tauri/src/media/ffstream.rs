//! Chuyển mã để xem trước những video WebView2 không giải mã được.
//!
//! # Vấn đề
//!
//! Khung xem trước phát một hình chữ nhật đen trong khi thanh thời gian vẫn
//! chạy và vẫn có tiếng. Đó không phải lỗi của app: WebView2 mở được container
//! `.mov`, đọc được luồng audio, và bó tay ở luồng video — vì luồng đó là
//! Apple ProRes, thứ Windows không có bộ giải mã. Đo trên mẫu 30 tệp `.mov`
//! của thư viện: **hai phần ba là ProRes**.
//!
//! # Cách làm: giao từng mảnh, phát ngay mảnh đầu
//!
//! ffmpeg giải mã và mã hoá lại thành H.264 dạng MP4 phân mảnh (fMP4), và
//! trang nhận **từng mảnh một** qua [`crate::media::ffphien`] rồi nạp vào
//! trình phát bằng Media Source Extensions. Mảnh đầu về là hình hiện, phần
//! còn lại chuyển mã tiếp trong lúc người dùng đang xem — video 30 giây phát
//! đủ 30 giây, thanh thời gian hiện đủ 30 giây ngay từ đầu.
//!
//! Không đi qua `media://`: bộ phục vụ URI của Tauri chỉ nhận một thân đáp ứng
//! đã hoàn chỉnh, nên qua đó thì phải chuyển mã xong mới gửi được gì.
//!
//! # Vì sao khung khoá dày là thứ quyết định tốc độ
//!
//! fMP4 chỉ cắt mảnh ở khung khoá. x264 mặc định đặt khung khoá mỗi 250 khung
//! hình — **10 giây** video ở 25 khung/giây — nên ffmpeg không giao được gì
//! cho trình phát tới khi làm xong cả 10 giây ấy. Đo thời điểm mảnh phát
//! được đầu tiên về tới nơi:
//!
//! | Tệp | GOP mặc định | Khung khoá mỗi 1s | **Mỗi 0,5s** |
//! |---|---|---|---|
//! | `6164429_…_4K.mov` (1,5 GB) | 3,14s | 0,44s | **0,37s** |
//! | `380638_…_6K.mov` (3,7 GB) | 12,46s | 1,02s | **0,74s** |
//!
//! Tổng thời gian chuyển mã gần như không đổi, nên cái giá chỉ là tệp đầu ra
//! lớn hơn chút vì nhiều khung khoá hơn — và nó nằm trong bộ nhớ, không đi
//! qua mạng. Bản cuối còn dày hơn nữa trong giây đầu — xem [`KHUNG_KHOA`].
//!
//! Ép `yuv420p` giảm tổng thời gian 12% (5,46 → 4,79s trên tệp 4K): ProRes
//! giải ra 4:2:2 10-bit, và để nguyên thì x264 mã hoá ở 4:2:2 10-bit — chậm
//! hơn, mà WebView2 còn chưa chắc giải được.
//!
//! # Những thứ ĐÃ THỬ và **không** giúp
//!
//! Ghi lại để người sau không thử lại. Nghẽn nằm ở **giải mã ProRes bằng
//! CPU**, không ở mã hoá, không ở số luồng:
//!
//! | Đòn bẩy | Kết quả trên tệp 6K |
//! |---|---|
//! | Thêm luồng (`-threads 16`) | 10,3s → 10,0s — đã bão hoà 12 CPU |
//! | Hạ 720p → 360p | 3,58s → 3,63s — không đổi |
//! | Giảm 25 → 12 khung/giây | 10,0s → 9,1s — vẫn phải giải mã hết |
//! | `ultrafast` thay `veryfast` | 10,2s → 9,7s — chỉ 4–18% |
//! | `-lowres` | không đổi — bộ giải mã ProRes không hỗ trợ |
//! | Bỏ `-tune zerolatency` | mảnh đầu chậm hơn: 0,44s → 0,62s |
//!
//! # Vì sao không bao giờ nhanh bằng `.mp4`
//!
//! Một `.mp4` H.264 không chuyển mã gì cả — app chỉ đọc byte từ đĩa và
//! WebView2 giải mã bằng phần cứng. ProRes thì phải giải mã bằng CPU rồi mã
//! hoá lại. Máy đo không có NVENC/QSV dùng được ("No capable devices found").
//! Đó là hai khối lượng công việc khác hẳn nhau, không phải một chỗ chưa tối
//! ưu.
//!
//! Và **ổ `D:` là đĩa cơ** (đo được 61–95 MB/s đọc nguội), trong khi ProRes
//! 422 HQ 4K là ~89 MB/s. Tệp chưa từng đọc thì chỉ riêng việc lấy byte lên
//! đã chậm ngang thời gian thực, bất kể giải mã nhanh cỡ nào.

use std::collections::HashMap;
use std::process::{Child, Stdio};
use std::sync::Arc;

use parking_lot::Mutex;

/// Khung hình sau chuyển mã nằm gọn trong ô vuông cạnh này.
///
/// 1280: thừa cho một khung xem trước trong cửa sổ 900×620. Theo ô vuông chứ
/// không theo chiều ngang, để video dọc (2160×3840) ra 720×1280 chứ không phải
/// 1280×2276 — cao hơn cả màn hình và đắt gấp ba để mã hoá.
///
/// Không bao giờ phóng to: nguồn nhỏ hơn thì giữ nguyên cỡ.
const CANH_TOI_DA: u32 = 1280;

/// Khi nào đặt khung khoá — cũng là khi nào cắt mảnh.
///
/// **Dày 0,1 giây trong giây đầu, rồi 0,5 giây.** Mảnh đầu càng ngắn thì
/// ffmpeg càng ít phải đọc trước khi giao được khung hình đầu tiên — và trên
/// đĩa cơ nguội, đọc mới là thứ đắt: ProRes 4K là 60–160 MB mỗi giây, còn ổ `D:`
/// đọc nguội được 60–95 MB/s.
///
/// A/B trên 12 tệp nguội (mỗi tệp chỉ đo được một lần — lần sau đã ấm):
///
/// | Cách | Mảnh đầu (có hình), trung bình |
/// |---|---|
/// | đều 0,5 giây | 666 ms |
/// | **dày 0,1 giây trong giây đầu** | **388 ms** |
///
/// Sau giây đầu thì thưa ra: khung khoá tốn bit, và lúc đó trình phát đã có
/// hình — không còn ai đợi từng trăm mili giây nữa. Thử 0,25 giây đều suốt
/// video thì không nhanh hơn 0,5 giây: cái lợi chỉ nằm ở mảnh đầu.
///
/// Công thức: khung khoá thứ `n` đặt ở `n × 0,1` giây cho 10 cái đầu, rồi
/// `1 + (n − 10) × 0,5`.
const KHUNG_KHOA: &str = "expr:gte(t,if(lt(n_forced,10),n_forced*0.1,1+(n_forced-10)*0.5))";

/// Đuôi tệp nào **có thể** cần chuyển mã.
///
/// # Vì sao lại là một danh sách đuôi tệp, khi đuôi không quyết định codec
///
/// Vì câu hỏi ở đây khác. Chỗ khác cần biết "tệp này chứa codec gì" — đuôi
/// không trả lời được. Ở đây câu hỏi là "có đáng bỏ ra một lần hỏi ffprobe
/// không", và với `.mp4`/`.webm` thì câu trả lời luôn là không: chúng chạy
/// tốt qua đường thẳng, chiếm 298.425 trên 310.395 video của thư viện này.
///
/// Tệp nào lọt lưới mà WebView2 vẫn báo lỗi thì giao diện xin chuyển mã bắt
/// buộc (xem `bat_buoc` của [`ke_hoach`]), nên danh sách này là đường tắt chứ
/// không phải cổng chặn.
pub fn co_the_can_chuyen_ma(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "mov" | "mkv" | "avi" | "mts" | "m2ts" | "ts" | "wmv" | "asf" | "flv" | "mpg" | "mpeg"
    )
}

/// Codec video mà WebView2 phát được thẳng, không cần chuyển mã.
///
/// Danh sách **cho phép**, không phải danh sách cấm: một codec lạ chưa ai
/// kiểm chứng thì đi đường chuyển mã — chậm hơn một chút nhưng chắc chắn hiện
/// hình, còn đoán sai theo chiều kia là màn đen im lặng.
const PHAT_THANG_DUOC: &[&str] = &["h264", "vp8", "vp9", "av1"];

/// Điều ffprobe cho biết về một tệp.
#[derive(Debug, Clone, PartialEq)]
pub struct ThongTin {
    /// Codec của luồng video đầu tiên, viết thường.
    pub codec: String,
    /// Giây. Trang đặt thời lượng này cho trình phát ngay từ đầu, nên thanh
    /// thời gian hiện đủ độ dài thật dù mới có vài giây đầu được chuyển mã.
    pub thoi_luong: f64,
}

/// Nên phát tệp thế nào.
#[derive(Debug, Clone, PartialEq)]
pub enum KeHoach {
    /// Đưa thẳng tệp gốc cho trình phát qua `media://`.
    PhatThang,
    /// Chuyển mã rồi mới phát được.
    ChuyenMa(Arc<ThongTin>),
}

/// Quyết định cách phát `path`.
///
/// `bat_buoc`: giao diện đã thử phát thẳng và WebView2 báo lỗi — chuyển mã
/// bất kể đuôi tệp và codec nói gì. Đó là lưới an toàn cho những tệp như
/// HEVC trong `.mp4`, mà danh sách ở trên không bắt được.
///
/// Không hỏi được ffprobe (không có ffmpeg, tệp hỏng) thì phát thẳng: đó là
/// hành vi cũ, và tệp nào ffprobe không đọc nổi thì ffmpeg cũng không chuyển
/// mã nổi.
pub fn ke_hoach(path: &str, bat_buoc: bool) -> KeHoach {
    if !bat_buoc && !co_the_can_chuyen_ma(path) {
        return KeHoach::PhatThang;
    }
    let Some(tt) = tham_do(path) else {
        return KeHoach::PhatThang;
    };
    if !bat_buoc && PHAT_THANG_DUOC.contains(&tt.codec.as_str()) {
        return KeHoach::PhatThang;
    }
    tracing::info!("xem trước: {path} dùng codec {} — chuyển mã", tt.codec);
    KeHoach::ChuyenMa(tt)
}

/// Kết quả ffprobe đã hỏi, để mở lại một tệp không phải hỏi lại.
///
/// Khoá gồm dung lượng và thời điểm sửa: tệp bị ghi đè thì câu trả lời cũ nói
/// về một tệp khác.
type KhoaThamDo = (String, u64, i64);
static DA_THAM_DO: Mutex<Option<HashMap<KhoaThamDo, Arc<ThongTin>>>> = Mutex::new(None);

/// Hỏi ffprobe một lần cho cả ba điều cần biết: codec, âm thanh, thời lượng.
fn tham_do(path: &str) -> Option<Arc<ThongTin>> {
    let md = std::fs::metadata(path).ok()?;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let khoa = (path.to_string(), md.len(), mtime);

    if let Some(tt) = DA_THAM_DO
        .lock()
        .as_ref()
        .and_then(|m| m.get(&khoa).cloned())
    {
        return Some(tt);
    }

    let mut cmd = crate::media::ffmpeg::lenh_ffprobe()?;
    cmd.args(["-v", "error"])
        .args([
            "-show_entries",
            "stream=codec_type,codec_name:format=duration",
        ])
        .args(["-of", "json"])
        .arg(path);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let tt = Arc::new(doc_tham_do(&out.stdout)?);

    let mut kho = DA_THAM_DO.lock();
    let m = kho.get_or_insert_with(HashMap::new);
    // Chặn phình vô hạn trong một phiên dài. Xoá sạch rẻ và đơn giản hơn đuổi
    // từng mục; cái giá chỉ là vài lần hỏi lại ffprobe.
    if m.len() > 512 {
        m.clear();
    }
    m.insert(khoa, Arc::clone(&tt));
    Some(tt)
}

/// Đọc JSON của ffprobe. Tách ra để kiểm thử được mà không cần tệp media.
fn doc_tham_do(json: &[u8]) -> Option<ThongTin> {
    let v: serde_json::Value = serde_json::from_slice(json).ok()?;
    let streams = v.get("streams")?.as_array()?;
    fn loai(s: &serde_json::Value) -> Option<&str> {
        s.get("codec_type").and_then(|t| t.as_str())
    }
    let codec = streams
        .iter()
        .find(|s| loai(s) == Some("video"))?
        .get("codec_name")?
        .as_str()?
        .to_ascii_lowercase();
    let thoi_luong = v
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok())
        .filter(|d| d.is_finite() && *d > 0.0)?;
    Some(ThongTin { codec, thoi_luong })
}

/// Tham số ffmpeg cho một phiên bắt đầu từ `tu_giay`.
///
/// Tách khỏi [`bat_dau`] để kiểm thử được những lựa chọn quyết định tốc độ —
/// một lần sửa vô tình bỏ `-force_key_frames` là mảnh đầu chậm lại gấp bảy.
///
/// **Không phụ thuộc vào ffprobe**, và đó là cố ý: nhờ vậy ffmpeg khởi động
/// ngay, song song với ffprobe, thay vì đợi nó — đo được ffprobe mất 70–570 ms
/// trên đĩa cơ nguội. Âm thanh lấy bằng `-map 0:a:0?` (dấu `?`: có thì lấy,
/// không có thì thôi), và luồng ra có tiếng hay không thì đọc thẳng từ hộp
/// `moov` của nó ([`crate::media::ffphien::doc_mime`]).
pub fn tham_so(path: &str, tu_giay: f64) -> Vec<String> {
    let mut a: Vec<String> = vec!["-v".into(), "error".into()];
    if tu_giay > 0.0 {
        // TRƯỚC `-i`: ffmpeg nhảy thẳng tới gần mốc rồi giải mã chính xác tới
        // đó, thay vì giải mã từ đầu tệp. Đầu ra bắt đầu từ mốc 0 — trang dời
        // nó về đúng chỗ bằng `timestampOffset`.
        a.extend(["-ss".into(), format!("{tu_giay:.3}")]);
    }
    a.extend(["-i".into(), path.into()]);
    a.extend([
        "-map".into(),
        "0:v:0".into(),
        "-map".into(),
        "0:a:0?".into(),
    ]);
    a.extend([
        "-vf".into(),
        // `min(…, iw)`: không phóng to nguồn nhỏ. Dấu phẩy trong biểu thức
        // nằm trong nháy đơn để bộ phân tích bộ lọc không tách nó ra.
        format!(
            "scale=w='min({c},iw)':h='min({c},ih)':force_original_aspect_ratio=decrease:force_divisible_by=2:flags=neighbor",
            c = CANH_TOI_DA
        ),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "ultrafast".into(),
        "-tune".into(),
        "zerolatency".into(),
        "-crf".into(),
        "26".into(),
        "-force_key_frames".into(),
        KHUNG_KHOA.into(),
    ]);
    a.extend([
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "128k".into(),
        "-ac".into(),
        "2".into(),
    ]);
    a.extend([
        // `empty_moov`: phần đầu (`moov`) ra ngay, không đợi tới cuối tệp.
        // `frag_keyframe`: mỗi khung khoá mở một mảnh mới.
        // `default_base_moof`: Media Source của Chromium đòi cờ này.
        "-movflags".into(),
        "frag_keyframe+empty_moov+default_base_moof".into(),
        "-f".into(),
        "mp4".into(),
        "pipe:1".into(),
    ]);
    a
}

/// Khởi động ffmpeg, đầu ra fMP4 trên stdout.
///
/// `None` khi máy không có ffmpeg hoặc không chạy được nó.
pub fn bat_dau(path: &str, tu_giay: f64) -> Option<Child> {
    let mut cmd = crate::media::ffmpeg::lenh()?;
    cmd.args(tham_so(path, tu_giay)).stdout(Stdio::piped());
    match cmd.spawn() {
        Ok(con) => {
            tracing::info!("xem trước: chuyển mã {path} từ giây {tu_giay:.1}");
            Some(con)
        }
        Err(e) => {
            tracing::warn!("xem trước: không chạy được ffmpeg cho {path}: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chi_hoi_ve_nhung_duoi_co_the_chua_codec_la() {
        // .mp4 và .webm chiếm gần như toàn bộ thư viện và luôn phát thẳng
        // được — hỏi ffprobe cho chúng là cái giá vô ích mỗi lần mở.
        assert!(!co_the_can_chuyen_ma("a.mp4"));
        assert!(!co_the_can_chuyen_ma("a.webm"));
        assert!(co_the_can_chuyen_ma("a.mov"));
        assert!(co_the_can_chuyen_ma("a.MOV"));
        assert!(co_the_can_chuyen_ma("a.mkv"));
        assert!(co_the_can_chuyen_ma("a.avi"));
    }

    #[test]
    fn anh_va_nhac_khong_bao_gio_di_duong_chuyen_ma() {
        for name in ["a.jpg", "a.png", "a.mp3", "a.flac", "khong-duoi"] {
            assert!(!co_the_can_chuyen_ma(name), "{name}");
        }
    }

    #[test]
    fn khong_hoi_duoc_thi_phat_thang() {
        // Tệp không tồn tại: ffprobe thất bại, và câu trả lời phải là phát
        // thẳng — tức hành vi cũ — chứ không dựng một tiến trình chuyển mã cho
        // một tệp không có thật. Kể cả khi giao diện xin bắt buộc.
        assert_eq!(
            ke_hoach(r"Z:\khong-he-ton-tai-9f3a.mov", false),
            KeHoach::PhatThang
        );
        assert_eq!(
            ke_hoach(r"Z:\khong-he-ton-tai-9f3a.mov", true),
            KeHoach::PhatThang
        );
    }

    #[test]
    fn doc_duoc_ket_qua_ffprobe_that() {
        // Đúng hình dạng ffprobe in ra cho một tệp ProRes có tiếng.
        let json = br#"{"programs":[],"streams":[
            {"codec_name":"prores","codec_type":"video"},
            {"codec_name":"pcm_s24le","codec_type":"audio"}],
            "format":{"duration":"18.000000"}}"#;
        let tt = doc_tham_do(json).expect("phải đọc được");
        assert_eq!(tt.codec, "prores");
        assert_eq!(tt.thoi_luong, 18.0);
    }

    #[test]
    fn tieng_la_tuy_chon_de_ffmpeg_khong_phai_cho_ffprobe() {
        // Phần lớn stock footage không có tiếng. `-map 0:a:0` cứng thì ffmpeg thoát
        // ngay trên những tệp đó; `?` làm âm thanh thành "có thì lấy", nên không cần
        // hỏi ffprobe trước khi khởi động.
        let a = tham_so("x.mov", 0.0);
        assert!(a.iter().any(|x| x == "0:a:0?"), "âm thanh phải là tuỳ chọn");
        assert!(
            !a.iter().any(|x| x == "0:a:0"),
            "âm thanh bắt buộc làm hỏng tệp không tiếng"
        );
    }

    #[test]
    fn tep_khong_co_video_hoac_thoi_luong_thi_khong_dung_duoc() {
        let chi_tieng = br#"{"streams":[{"codec_name":"aac","codec_type":"audio"}],
            "format":{"duration":"3.0"}}"#;
        assert!(doc_tham_do(chi_tieng).is_none());
        let khong_do_dai = br#"{"streams":[{"codec_name":"prores","codec_type":"video"}],
            "format":{}}"#;
        assert!(doc_tham_do(khong_do_dai).is_none());
    }

    /// Khoá lại thứ quyết định mảnh đầu về sau 0,4 giây thay vì 3 giây.
    #[test]
    fn tham_so_giu_nhung_lua_chon_quyet_dinh_toc_do() {
        let a = tham_so("x.mov", 0.0);
        let co = |x: &str| a.iter().any(|v| v == x);
        assert!(
            co("-force_key_frames"),
            "thiếu khung khoá dày: mảnh đầu chậm gấp 7"
        );
        let kk = &a[a.iter().position(|v| v == "-force_key_frames").unwrap() + 1];
        assert!(
            kk.contains("0.1"),
            "mảnh đầu phải ngắn — có hình nhanh hơn ~40% trên đĩa nguội"
        );
        assert!(
            co("yuv420p"),
            "thiếu yuv420p: mã hoá 4:2:2 10-bit, chậm hơn 12%"
        );
        assert!(co("frag_keyframe+empty_moov+default_base_moof"));
        assert!(co("ultrafast") && co("zerolatency"));
        // Phiên từ đầu tệp không tua.
        assert!(!co("-ss"));
    }

    #[test]
    fn phien_tua_nhay_truoc_khi_mo_tep() {
        let a = tham_so("x.mov", 12.5);
        let ss = a.iter().position(|v| v == "-ss").expect("phải có -ss");
        let i = a.iter().position(|v| v == "-i").expect("phải có -i");
        assert!(
            ss < i,
            "-ss phải đứng TRƯỚC -i để nhảy thẳng, không giải mã từ đầu"
        );
        assert_eq!(a[ss + 1], "12.500");
    }
}
