//! Trích một khung hình bằng ffmpeg, cho những tệp Windows không đọc nổi.
//!
//! Đây là **phương án dự phòng**, chạy sau khi `IShellItemImageFactory` đã
//! trả `WTS_E_FAILEDEXTRACTION`. Lý do đầy đủ nằm ở [`crate::media::ffmpeg`];
//! tệp này chỉ lo phần "hỏi ffmpeg thế nào cho đúng".
//!
//! # Lấy khung ở đâu trong tệp
//!
//! Không lấy khung 0. Rất nhiều video mở đầu bằng màn đen, một khung trắng,
//! hay logo — một thư viện mà mọi ảnh thu nhỏ đều đen thì chẳng khác gì không
//! có ảnh. Lấy ở [`MOC_GIAY`] giây, và nếu tệp ngắn hơn thì ffmpeg tự dừng ở
//! khung cuối đọc được, nên không cần hỏi độ dài trước.
//!
//! # Vì sao `-ss` đứng TRƯỚC `-i`
//!
//! Đặt sau `-i` thì ffmpeg giải mã tuần tự từ đầu tệp tới mốc cần lấy. Với
//! ProRes 4K 710 Mbps, ba giây đó là hơn 250 MB phải đọc và giải mã. Đặt
//! trước `-i` thì nó seek tới keyframe gần nhất rồi mới bắt đầu — đo được
//! **1,0 giây** thay vì hàng chục.
//!
//! Cái giá là mốc thời gian không chính xác tuyệt đối. Với một ảnh thu nhỏ
//! thì lệch vài khung hình không có nghĩa gì.
//!
//! # Vì sao xuất PNG qua đường ống, không qua tệp tạm
//!
//! Tệp tạm cần dọn, và một tiến trình bị giết giữa chừng sẽ để lại rác trong
//! `%TEMP%` mãi mãi. Đường ống không có vấn đề đó: tiến trình chết là ống
//! đóng. Ảnh thu nhỏ chỉ vài chục KB nên giữ trong bộ nhớ không đáng kể.

use std::io::Read;
use std::time::Duration;

/// Lấy khung ở giây thứ mấy.
///
/// Ba giây: qua được phần fade-in và logo mở đầu của hầu hết video studio,
/// nhưng vẫn nằm trong những tệp ngắn vài giây.
const MOC_GIAY: &str = "3";

/// Quá hạn này thì bỏ cuộc.
///
/// Đo được 1,0 giây cho ProRes 4K trên NAS. Mười lăm giây là rộng rãi cho một
/// tệp lớn hơn trên đường mạng chậm hơn, và vẫn đủ ngắn để một tệp hỏng không
/// giữ chỗ trong hàng đợi thumbnail quá lâu — hàng đợi chỉ có 4 worker, nên
/// một tệp treo vô hạn sẽ làm đói cả danh sách.
const HAN_GIAY: u64 = 15;

/// Trích một khung hình, trả về PNG đã co về `co` pixel cạnh dài.
///
/// `None` khi không có ffmpeg, khi ffmpeg bỏ cuộc, hoặc khi quá hạn — mọi
/// trường hợp đều dẫn tới cùng một kết quả trên màn hình: huy hiệu màu theo
/// loại tệp, đúng như trước khi có tệp này.
pub fn khung_hinh(path: &str, co: u32) -> Option<Vec<u8>> {
    let mut cmd = crate::media::ffmpeg::lenh()?;

    cmd.args(["-v", "error"])
        // TRƯỚC `-i`: seek nhanh. Xem chú thích đầu tệp.
        .args(["-ss", MOC_GIAY])
        .args(["-i", path])
        .args(["-frames:v", "1"])
        // `-an`/`-sn`: không đụng tới audio và phụ đề. Không cần, và bỏ đi
        // thì ffmpeg khỏi phải khởi tạo bộ giải mã cho chúng.
        .arg("-an")
        .arg("-sn")
        // Co về đúng cỡ ngay trong ffmpeg: co ở đây rẻ hơn nhiều so với đẩy
        // một khung 4K (3840×2160×4 = 33 MB) qua đường ống rồi mới co.
        //
        // `-2` cho cạnh còn lại: giữ tỉ lệ, và làm tròn về số chẵn — vài bộ
        // lọc từ chối cạnh lẻ.
        //
        // `format=rgb24|rgba`: ép về 8 bit mỗi kênh. Thiếu nó, nguồn 10 bit
        // (ProRes nào cũng vậy) cho ra PNG 16 bit mỗi kênh — đo được 119 KB
        // cho một ảnh 192×108, so với 27 KB khi ép 8 bit, mà mắt không thấy
        // khác gì ở cỡ này. Nhân lên 512 mục của LRU và cả kho trên đĩa thì
        // đó là bốn lần bộ nhớ cho không. `rgba` giữ lại kênh alpha của
        // ProRes 4444 (hình trên nền trong suốt).
        .args([
            "-vf",
            &format!("scale={co}:-2:force_original_aspect_ratio=decrease,format=rgb24|rgba"),
        ])
        .args(["-f", "image2"])
        .args(["-c:v", "png"])
        // `pipe:1` — ra stdout, không qua tệp tạm.
        .arg("pipe:1");

    let mut con = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!("ffmpeg: không chạy được cho {path}: {e}");
            return None;
        }
    };

    // Đọc stdout trên luồng này trong khi chờ: nếu chỉ `wait()` rồi mới đọc,
    // một ảnh lớn hơn bộ đệm ống sẽ làm ffmpeg chặn ở lệnh ghi và cả hai bên
    // cùng chờ nhau mãi mãi.
    let mut png = Vec::new();
    if let Some(mut out) = con.stdout.take() {
        let _ = out.read_to_end(&mut png);
    }

    match cho_trong_han(&mut con, Duration::from_secs(HAN_GIAY)) {
        Some(true) if !png.is_empty() => Some(png),
        Some(true) => {
            tracing::debug!("ffmpeg: không có khung hình nào cho {path}");
            None
        }
        Some(false) => {
            tracing::debug!("ffmpeg: bỏ cuộc với {path}");
            None
        }
        None => {
            // Quá hạn. Giết tiến trình, nếu không nó còn đọc đĩa sau khi
            // không ai chờ kết quả nữa.
            let _ = con.kill();
            let _ = con.wait();
            tracing::warn!("ffmpeg: quá {HAN_GIAY}s với {path} — bỏ qua");
            None
        }
    }
}

/// Chờ tiến trình, trả `None` nếu quá hạn.
///
/// `std` không có `wait_timeout`, và kéo cả một crate về chỉ để chờ có hạn là
/// không đáng. Hỏi theo nhịp: tệp nào cũng xong trong khoảng một giây, nên
/// nhịp 20 ms không tốn gì mà vẫn không làm chậm ca thường.
fn cho_trong_han(con: &mut std::process::Child, han: Duration) -> Option<bool> {
    let bat_dau = std::time::Instant::now();
    loop {
        match con.try_wait() {
            Ok(Some(st)) => return Some(st.success()),
            Ok(None) => {}
            Err(_) => return Some(false),
        }
        if bat_dau.elapsed() >= han {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Không có ffmpeg thì trả `None` chứ không hoảng — máy CI là như vậy.
    #[test]
    fn tra_none_khi_khong_co_gi_de_doc() {
        // Đường dẫn không tồn tại: có ffmpeg hay không, kết quả vẫn phải là
        // `None` chứ không phải một panic hay một PNG rỗng.
        assert_eq!(khung_hinh(r"Z:\khong-he-ton-tai-9f3a.mov", 192), None);
    }
}
