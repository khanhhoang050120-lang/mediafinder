//! Pool đọc đĩa riêng cho quét trùng lặp: một hàng đợi chung, ưu tiên thấp.
//!
//! # Vì sao pool RIÊNG
//!
//! Trước đây tầng 2 chạy `into_par_iter()` trên pool rayon **toàn cục** — cùng
//! pool mà ô tìm kiếm dùng ([`crate::index::search`]) và duyệt NAS nền dùng.
//! Theo cách rayon chia việc, một truy vấn mới không chen được vào đoạn đang
//! chạy: nó chờ tới khi một luồng xong đoạn hiện tại, mà mỗi đoạn là hàng trăm
//! tệp, mỗi tệp cả trăm mili giây trên NAS. Đó là lỗi 4.2 và 4.4 của
//! `DE-XUAT-TRUNG-LAP.md`: gõ tìm kiếm trong lúc quét trùng thì ô tìm kiếm đơ.
//!
//! Pool riêng cắt đứt chuyện đó, và [`THREAD_PRIORITY_BELOW_NORMAL`] bảo đảm
//! việc của người đang ngồi trước máy luôn thắng.
//!
//! # Vì sao MỘT hàng đợi chung, không chia theo ổ
//!
//! Bản đầu chia việc theo thiết bị vật lý — đĩa trong máy một pool, mỗi máy
//! chủ NAS một pool — vì đo được các ổ chênh nhau tới 25 lần (`D:` 30,3
//! tệp/giây, `Y:` 1,2). Ý tưởng nghe hợp lý: một luồng bốc phải tệp trên `Y:`
//! đứng chờ 0,8 giây, trong khoảng đó nó lẽ ra đã đọc xong 24 tệp trên `D:`.
//!
//! **Đo có kiểm soát thì nó chậm hơn 15%.** Bốn mẫu 1.500 tệp rời nhau, vòng
//! hai đảo thứ tự để trôi theo thời gian rơi đều hai bên:
//!
//! | Cách chia | Vòng 1 | Vòng 2 | Trung bình |
//! |---|---|---|---|
//! | Một hàng đợi chung | 52,8 | 57,1 | **54,8 tệp/giây** |
//! | Pool riêng theo ổ | 42,0 | 51,9 | 46,5 tệp/giây |
//!
//! Nhánh chia pool tốt nhất (51,9) vẫn thua nhánh chung tệ nhất (52,8).
//!
//! Lý do là **ăn cắp việc**: với một hàng đợi chung, không luồng nào rảnh khi
//! còn tệp chưa đọc. Chia theo thiết bị thì luồng của ổ nào chỉ đọc ổ đó, xong
//! sớm là ngồi không trong khi nhóm khác còn việc.
//!
//! Bài học đắt hơn nằm ở phép đo cũ. Nó đo **16 luồng so với 1 luồng trên
//! riêng ổ `Y:`** và ra 24,7×, rồi con số đó được dùng để biện minh cho việc
//! chia pool. Nhưng bản gốc chưa bao giờ chạy 1 luồng — nó chạy 12 luồng ăn
//! cắp việc. So với một mốc không tồn tại thì ra 24,7×; so với mốc thật thì ra
//! 0,85×.
//!
//! # Vì sao 32 luồng
//!
//! Cùng phép đo, hàng đợi chung, mỗi mức một mẫu 1.500 tệp rời nhau. Mức mặc
//! định (12 = số CPU của máy đo) chạy hai lần, đầu và cuối, để đo sai số:
//!
//! | Luồng | Tệp/giây |
//! |---|---|
//! | mặc định (12), đo đầu | 52,8 |
//! | 24 | 63,4 |
//! | **32** | **63,7** |
//! | 48 | 66,9 |
//! | 64 | 62,7 |
//! | mặc định (12), đo cuối | 58,7 |
//!
//! Mặc định đo hai lần lệch nhau **11%** — đó là sàn nhiễu, và cả dải 24→64
//! nằm trong khoảng nhiễu của nhau. Nghĩa là thêm luồng được khoảng 1,15× rồi
//! **chạm trần ngay ở 24**: NAS bão hoà quanh 63–67 tệp/giây, không phải thiếu
//! luồng.
//!
//! Chọn 32 chứ không phải 48 dù 48 đo được cao nhất: chênh lệch giữa chúng nằm
//! dưới sàn nhiễu, nên đó không phải kết quả. Và cái không đo được trên một
//! máy là 20–40 máy studio cùng quét — mỗi luồng thêm là tải thêm đổ lên chính
//! NAS mà cả studio đang dùng để làm việc.
//!
//! Lời hứa thật của tính năng này không nằm ở đây. Kho vân tay bền
//! ([`crate::media::dupestore`]) đưa lượt quét thứ hai từ **57,1 phút xuống
//! 1,8 giây** trên cùng thư viện, cùng kết quả. Số luồng là chuyện 1,15×; kho
//! vân tay là chuyện 1.904×.
//!
//! [`THREAD_PRIORITY_BELOW_NORMAL`]: https://learn.microsoft.com/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadpriority

/// Số luồng đọc đĩa cho một lượt quét trùng lặp.
///
/// Ba mươi hai. Xem bảng đo ở đầu module: dải 24–64 luồng đều cho khoảng
/// 1,15× so với mặc định và chênh nhau dưới sàn nhiễu, nên đây là điểm giữa
/// vùng bằng phẳng chứ không phải điểm cực đại của một đường cong.
pub const LUONG: usize = 32;

// Kiểm lúc biên dịch. Máy studio có 12 CPU logic; đặt số này bằng hoặc thấp
// hơn số CPU là quay về đúng hành vi mặc định mà phép đo cho thấy chậm hơn,
// và mất luôn lý do tồn tại của hằng số này.
const _: () = assert!(LUONG > 12);

/// Dựng pool đọc đĩa cho một lượt quét.
///
/// Trả `None` nếu không dựng được — chỗ gọi phải chạy tuần tự chứ đừng bỏ tệp.
/// Chậm hơn nhiều, nhưng một lượt quét chậm vẫn hơn một lượt quét thiếu tệp mà
/// không nói gì.
pub fn dung() -> Option<rayon::ThreadPool> {
    match rayon::ThreadPoolBuilder::new()
        .num_threads(LUONG)
        .thread_name(|n| format!("dupe-{n}"))
        .start_handler(|_| uu_tien_thap())
        .build()
    {
        Ok(p) => {
            tracing::info!("pool đọc đĩa cho quét trùng lặp: {LUONG} luồng");
            Some(p)
        }
        Err(e) => {
            tracing::warn!("không dựng được pool quét trùng lặp: {e}");
            None
        }
    }
}

/// Hạ ưu tiên luồng hiện tại, như enrichment đã làm.
///
/// Người dùng đang ngồi trước máy chờ ô tìm kiếm trả lời; lượt quét này thì
/// không ai đang chờ. Quan trọng hơn hẳn khi số luồng là 32 chứ không phải 12.
fn uu_tien_thap() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Threading::{
            GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
        };
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dung_duoc_pool_dung_so_luong() {
        let p = dung().expect("phải dựng được pool");
        assert_eq!(p.current_num_threads(), LUONG);
    }

    /// Pool phải TÁCH khỏi pool toàn cục.
    ///
    /// Đây là toàn bộ lý do module này tồn tại: quét trùng lặp chạy chung pool
    /// với ô tìm kiếm thì gõ tìm kiếm trong lúc quét sẽ đơ (lỗi 4.2). Nếu
    /// `install` chạy trên pool toàn cục thì số luồng bên trong sẽ bằng số CPU
    /// chứ không phải `LUONG`.
    #[test]
    fn tach_khoi_pool_toan_cuc() {
        let p = dung().expect("phải dựng được pool");
        let trong = p.install(rayon::current_num_threads);
        assert_eq!(trong, LUONG, "việc phải chạy trong pool riêng");

        let ngoai = rayon::current_num_threads();
        assert_ne!(
            ngoai, LUONG,
            "pool toàn cục không được có đúng {LUONG} luồng, \
             nếu không bài này không phân biệt được hai pool"
        );
    }

    /// Luồng phải mang tên nhận ra được.
    ///
    /// Không phải chuyện thẩm mỹ: khi một máy studio báo "app chiếm đĩa", thứ
    /// đầu tiên nhìn là danh sách luồng trong Process Explorer.
    #[test]
    fn luong_co_ten_rieng() {
        let p = dung().expect("phải dựng được pool");
        let ten = p.install(|| std::thread::current().name().map(str::to_string));
        assert!(
            ten.as_deref().is_some_and(|t| t.starts_with("dupe-")),
            "luồng phải tên dupe-N, thấy {ten:?}"
        );
    }
}
