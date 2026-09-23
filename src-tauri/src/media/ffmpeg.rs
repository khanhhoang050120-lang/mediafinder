//! Tìm ffmpeg, và chạy nó mà không bật cửa sổ console.
//!
//! # Vì sao cần tới ffmpeg
//!
//! Ảnh thu nhỏ và xem trước đều dựa vào Windows: `IShellItemImageFactory` cho
//! ảnh, và bộ giải mã của WebView2 cho video. Cả hai đều dừng ở cùng một chỗ
//! — **codec mà Windows không có**.
//!
//! Đo trên thư viện thật của studio, tệp
//! `6164429_Summer Ocean Waves…_4K.mov`:
//!
//! | Cách hỏi | Kết quả |
//! |---|---|
//! | `SIIGBF_THUMBNAILONLY` (app vẫn dùng) | `0x8004B200` — không trích được |
//! | cho phép icon, 192/256/512 | ảnh **vuông** ở mọi cỡ, tức icon chung |
//! | ffmpeg | khung hình thật, **1,0 giây** |
//!
//! Tệp đó là Apple ProRes 422 HQ (`hcpa`), 710 Mbps, 4K. Windows không kèm bộ
//! giải mã ProRes, nên nó **không thể** dựng ảnh dù hỏi cách nào. Cùng lý do
//! khiến khung xem trước phát ra một hình chữ nhật đen trong khi thanh thời
//! gian vẫn chạy và vẫn có tiếng: WebView2 mở được container `.mov`, đọc được
//! luồng audio, và bó tay ở luồng video.
//!
//! Đây là loại tệp trung tâm của công việc studio, không phải ca hiếm.
//!
//! # Vì sao KHÔNG thay Windows bằng ffmpeg ở mọi nơi
//!
//! Đường shell vẫn là đường chính, và nó nhanh hơn nhiều bậc:
//!
//! * Explorer đã cache sẵn ảnh trong `thumbcache_*.db`, nên phần lớn yêu cầu
//!   trả lời trong **vài micro giây** mà không giải mã gì.
//! * ffmpeg phải khởi tạo tiến trình, mở tệp, seek, giải mã một khung — đo
//!   được **~1 giây** cho ProRes 4K trên NAS.
//!
//! Nghìn lần chênh lệch đó là lý do ffmpeg chỉ được gọi **sau khi** shell đã
//! từ chối. Với `.mp4` và `.webm` — chiếm 298.425 trên 310.395 video của thư
//! viện này — không có gì thay đổi.
//!
//! # Vì sao dò tìm chứ không cứng hoá một đường dẫn
//!
//! Nhị phân đi kèm bộ cài, nên đường dẫn cạnh tệp exe là chỗ tìm trước tiên.
//! Nhưng bản dựng từ mã nguồn (`cargo run`) không có nó, và máy lập trình
//! thường đã có ffmpeg trong PATH — nên PATH là phương án dự phòng, không
//! phải để thay thế. Thiếu cả hai thì mọi thứ lặng lẽ quay về hành vi cũ:
//! huy hiệu màu thay cho ảnh, và khung xem trước báo không xem trước được.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Đừng để một cửa sổ console nhấp nháy trên màn hình người dùng.
///
/// `CREATE_NO_WINDOW`. Thiếu cờ này thì mỗi lần cuộn qua một tệp ProRes là
/// một ô console đen bật lên rồi tắt — trong chế độ lưới, đó là hàng chục ô
/// mỗi giây.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `BELOW_NORMAL_PRIORITY_CLASS` — xem chú thích trong [`lenh`].
#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

/// Đường dẫn tới ffmpeg, dò một lần cho cả vòng đời tiến trình.
///
/// `OnceLock` chứ không dò lại mỗi lần: việc dò đụng tới hệ thống tệp, và câu
/// trả lời không đổi trong một phiên chạy.
static FFMPEG: OnceLock<Option<PathBuf>> = OnceLock::new();

/// ffmpeg nằm ở đâu, hoặc `None` nếu máy này không có.
pub fn duong_dan() -> Option<&'static std::path::Path> {
    FFMPEG.get_or_init(tim).as_deref()
}

/// Máy này có ffmpeg không — để giao diện nói đúng vì sao thiếu ảnh.
pub fn co_san() -> bool {
    duong_dan().is_some()
}

/// Dò theo thứ tự: cạnh tệp exe (bản cài), rồi PATH (máy lập trình).
fn tim() -> Option<PathBuf> {
    let ten = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };

    // Cạnh tệp exe. `tauri.ffmpeg.conf.json` ánh xạ `binaries/ffmpeg.exe`
    // thành `ffmpeg.exe`, nên bộ cài đặt nó vào `$INSTDIR` ngay cạnh
    // `mediafinder.exe` — và `tauri build`/`tauri dev` với cấu hình đó cũng
    // chép nó vào cạnh exe trong `target/`.
    if let Some(ung_vien) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join(ten)))
    {
        if ung_vien.is_file() {
            tracing::info!("ffmpeg: dùng bản đi kèm {}", ung_vien.display());
            return Some(ung_vien);
        }
    }

    // PATH. `where`/`which` trả về nhiều dòng khi có nhiều bản; lấy dòng đầu.
    let tra = if cfg!(windows) { "where" } else { "which" };
    let mut cmd = Command::new(tra);
    cmd.arg(ten).stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    if let Ok(out) = cmd.output() {
        if out.status.success() {
            if let Some(dong) = String::from_utf8_lossy(&out.stdout).lines().next() {
                let p = PathBuf::from(dong.trim());
                if p.is_file() {
                    tracing::info!("ffmpeg: tìm thấy trong PATH {}", p.display());
                    return Some(p);
                }
            }
        }
    }

    tracing::info!(
        "ffmpeg: không tìm thấy — tệp dùng codec Windows không đọc được \
         (ProRes, DNxHD…) sẽ không có ảnh thu nhỏ và không xem trước được"
    );
    None
}

/// Dựng một `Command` đã tắt console và ngắt mọi đường vào/ra thừa.
///
/// Dùng chung cho mọi lời gọi ffmpeg, nên không có chỗ nào quên `CREATE_NO_WINDOW`.
pub fn lenh() -> Option<Command> {
    let path = duong_dan()?;
    let mut cmd = Command::new(path);
    // `-nostdin`: không có ai gõ phím vào đây, và thiếu nó thì ffmpeg có thể
    // ngồi chờ một đầu vào không bao giờ tới.
    cmd.arg("-nostdin")
        .arg("-hide_banner")
        .stdin(Stdio::null())
        // `null`, KHÔNG phải `piped`. Không ai đọc stderr, mà một ống không ai
        // đọc sẽ đầy: gặp tệp hỏng, ffmpeg in lỗi cho từng khối hình, bộ đệm
        // ống (vài KB) đầy, và ffmpeg đứng im mãi ở lệnh ghi tiếp theo — kéo
        // theo cả phiên xem trước đang chờ nó.
        .stderr(Stdio::null())
        .stdout(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Ưu tiên thấp: giải mã ProRes 4K ăn trọn mọi lõi CPU, và ở mức bình
        // thường nó tranh với chính giao diện đang vẽ video đó — ô tìm kiếm
        // và cuộn danh sách sẽ khựng. Máy rảnh thì ưu tiên thấp vẫn được
        // trọn CPU, nên tốc độ chuyển mã không đổi.
        cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }
    Some(cmd)
}

/// Như [`lenh`] nhưng cho ffprobe, nằm cạnh ffmpeg trong mọi bản phân phối.
///
/// Ưu tiên bình thường: ffprobe chạy vài chục mili giây và nằm trên đường
/// người dùng đang chờ, nên không có lý do gì để nó nhường ai.
pub fn lenh_ffprobe() -> Option<Command> {
    let ffprobe = duong_dan()?.with_file_name(if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    });
    if !ffprobe.is_file() {
        return None;
    }
    let mut cmd = Command::new(ffprobe);
    cmd.stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    Some(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Không kiểm rằng ffmpeg CÓ — máy CI không có, và đó là trạng thái hợp
    /// lệ. Kiểm cái bất biến đúng trên mọi máy: dò hai lần cho cùng một câu
    /// trả lời, vì cả chương trình dựa vào việc nó được quyết định một lần.
    #[test]
    fn viec_do_tim_cho_cung_mot_ket_qua() {
        assert_eq!(duong_dan().is_some(), co_san());
        assert_eq!(duong_dan(), duong_dan());
    }

    /// Có ffmpeg thì phải dựng được lệnh; không có thì phải trả `None` chứ
    /// không hoảng.
    #[test]
    fn lenh_chi_dung_duoc_khi_co_ffmpeg() {
        assert_eq!(lenh().is_some(), co_san());
    }
}
