//! Quét trùng lặp ổ trong máy lúc máy rảnh, để người dùng không phải chờ.
//!
//! # Ý tưởng, và số liệu đứng sau nó
//!
//! Người ta tới công ty lúc 8 giờ, mở máy, rồi pha cà phê và đọc email. Ứng
//! dụng đã chạy nền từ lúc đăng nhập nhưng chưa ai dùng tới. Đó là khoảng thời
//! gian rẻ nhất trong ngày để đọc đĩa.
//!
//! Đo trên thư viện thật của studio, bằng chính mã hiện tại:
//!
//! * **45 tệp/giây** — số đo thật, không phải ngoại suy.
//! * Quét trọn 197.301 ứng viên: **1,2 giờ**. Tài liệu cũ ước tính ~30 phút;
//!   thực tế đắt hơn hai lần.
//! * Riêng ổ trong máy (36.319 tệp): **13 phút**.
//!
//! Mười ba phút chạy nền lúc 8 giờ sáng đổi lấy việc bấm nút lúc 10 giờ là có
//! kết quả ngay. Đó là toàn bộ lý do của tệp này.
//!
//! # Vì sao KHÔNG BAO GIỜ tự quét ổ mạng
//!
//! 82% ứng viên nằm trên NAS (160.982 trên 197.301). Nếu chạy nền quét cả NAS
//! thì **20–40 máy cùng đọc NAS mỗi sáng**, đúng lúc mọi người vừa tới và bắt
//! đầu mở dự án.
//!
//! Đây không phải lo xa: lịch quét ổ mạng ở [`crate::netsched`] đã phải giãn
//! giờ ngẫu nhiên 0–20 phút giữa các máy để tránh, mà lượt đó chỉ mất ~2 phút.
//! Quét trùng NAS là hàng chục phút.
//!
//! Ổ mạng chỉ được quét khi người dùng **tự bấm và tự chọn** — xem
//! [`crate::media::dupescope`].
//!
//! # "Máy rảnh" nghĩa là gì
//!
//! Ứng dụng không biết người dùng đang dựng phim hay đang đọc email. Ba tín
//! hiệu dùng được, và cả ba phải cùng đúng:
//!
//! 1. **Enrichment đã xong** — dấu hiệu đợt đọc đĩa nặng nhất lúc khởi động đã
//!    qua. Chen vào giữa nó là hai việc cùng tranh đĩa.
//! 2. **Không có truy vấn tìm kiếm nào trong [`YEN_LANG`]** — người dùng chưa
//!    cần tới ứng dụng.
//! 3. **Chưa có lượt quét nào đang chạy** — kể cả lượt người dùng tự bấm.
//!
//! Và nó **dừng ngay** khi người dùng gõ tìm kiếm: việc của họ luôn thắng.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// Chờ bao lâu không có truy vấn nào thì coi là máy rảnh.
///
/// Mười phút. Ngắn hơn thì một người vừa tìm xong rồi quay sang việc khác vẫn
/// bị coi là rảnh, và đĩa bắt đầu chạy ngay dưới tay họ. Dài hơn thì bỏ lỡ
/// đúng khoảng buổi sáng mà tính năng này sinh ra để tận dụng.
pub const YEN_LANG: Duration = Duration::from_secs(10 * 60);

/// Nhịp kiểm tra điều kiện.
///
/// Ba mươi giây: đủ nhanh để bắt đầu ngay khi máy vừa rảnh, đủ chậm để một
/// vòng lặp chạy cả ngày không đáng kể.
const NHIP: Duration = Duration::from_secs(30);

/// Đã dựng luồng chưa — chặn việc dựng hai luồng nếu `setup` chạy hai lần.
static DA_CHAY: AtomicBool = AtomicBool::new(false);

/// Đã quét nền xong trong phiên này chưa.
///
/// Quét đúng **một lần** mỗi phiên. Chỉ mục ổ trong máy được tác vụ nền cập
/// nhật mỗi ngày, nên quét lại nhiều lần trong một phiên là đọc lại cùng những
/// tệp đó để ra cùng câu trả lời.
static DA_XONG: AtomicBool = AtomicBool::new(false);

/// Người dùng có tắt tính năng này không.
///
/// Người chưa từng dùng tới màn Trùng lặp không nên phải trả giá đọc đĩa cho
/// nó. Mặc định **bật**, vì cái giá là 13 phút ở mức ưu tiên thấp và cái được
/// là không phải chờ.
static DA_TAT: AtomicBool = AtomicBool::new(false);

/// Số hiệu truy vấn lần cuối thấy được, và lúc nào thấy.
static QUERY_CUOI: AtomicU64 = AtomicU64::new(0);
static MOC_YEN: AtomicU64 = AtomicU64::new(0);

/// Bật hoặc tắt quét nền.
pub fn dat_bat(bat: bool) {
    DA_TAT.store(!bat, Ordering::Relaxed);
    tracing::info!("quét trùng lặp nền: {}", if bat { "bật" } else { "tắt" });
}

/// Đang bật không.
pub fn dang_bat() -> bool {
    !DA_TAT.load(Ordering::Relaxed)
}

/// Đã quét nền xong trong phiên này chưa.
pub fn da_xong() -> bool {
    DA_XONG.load(Ordering::Relaxed)
}

/// Giây Unix hiện tại.
fn bay_gio() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Đủ điều kiện để bắt đầu quét nền chưa.
///
/// Tách thành hàm thuần để kiểm thử được mọi tổ hợp mà không cần một ứng dụng
/// thật, một chỉ mục thật, hay phải chờ mười phút.
///
/// * `bat` — tính năng đang bật.
/// * `da_xong` — đã quét trong phiên này rồi.
/// * `enrich_xong` — enrichment đã chạy xong.
/// * `dang_quet` — có lượt quét nào (trùng lặp hoặc chỉ mục) đang chạy.
/// * `giay_yen` — bao lâu rồi không có truy vấn nào.
pub fn du_dieu_kien(
    bat: bool,
    da_xong: bool,
    enrich_xong: bool,
    dang_quet: bool,
    giay_yen: u64,
) -> bool {
    bat && !da_xong && enrich_xong && !dang_quet && giay_yen >= YEN_LANG.as_secs()
}

/// Dựng luồng theo dõi. Gọi một lần lúc `setup`.
pub fn dung_lich(app: tauri::AppHandle) {
    if DA_CHAY.swap(true, Ordering::SeqCst) {
        tracing::warn!("quét trùng lặp nền đã chạy rồi, bỏ qua lần dựng thứ hai");
        return;
    }
    MOC_YEN.store(bay_gio(), Ordering::Relaxed);

    if let Err(e) = std::thread::Builder::new()
        .name("dupe-idle".into())
        .spawn(move || vong_lap(app))
    {
        DA_CHAY.store(false, Ordering::SeqCst);
        tracing::warn!("không dựng được luồng quét trùng lặp nền: {e}");
    }
}

fn vong_lap(app: tauri::AppHandle) {
    use tauri::Manager;

    tracing::info!(
        "quét trùng lặp nền: chờ máy rảnh {} phút, chỉ ổ trong máy",
        YEN_LANG.as_secs() / 60
    );

    loop {
        std::thread::sleep(NHIP);

        let Some(st) = app.try_state::<crate::state::AppState>() else {
            return; // ứng dụng đang tắt
        };

        // Người dùng vừa tìm kiếm thì đồng hồ yên lặng đếm lại từ đầu.
        let q = st.generation().load(Ordering::Relaxed);
        if q != QUERY_CUOI.load(Ordering::Relaxed) {
            QUERY_CUOI.store(q, Ordering::Relaxed);
            MOC_YEN.store(bay_gio(), Ordering::Relaxed);
        }

        if DA_XONG.load(Ordering::Relaxed) {
            return; // xong việc của phiên này, không cần vòng lặp nữa
        }

        let Some(dupes) = app.try_state::<crate::media::dupes::DupeService>() else {
            return;
        };
        let enrich_xong = app
            .try_state::<crate::media::enrich::EnrichService>()
            .map(|e| !e.status().running)
            .unwrap_or(false);

        let giay_yen = bay_gio().saturating_sub(MOC_YEN.load(Ordering::Relaxed));
        let dang_quet = st.is_scanning() || dupes.progress().running;

        if !du_dieu_kien(
            dang_bat(),
            DA_XONG.load(Ordering::Relaxed),
            enrich_xong,
            dang_quet,
            giay_yen,
        ) {
            continue;
        }

        // CHỈ ổ trong máy. Xem chú thích đầu tệp: 82% ứng viên nằm trên NAS,
        // và 40 máy cùng đọc NAS mỗi sáng là cái giá đổ lên chính NAS mà cả
        // studio đang dùng để làm việc.
        tracing::info!("quét trùng lặp nền: máy rảnh {giay_yen}s, bắt đầu quét ổ trong máy");
        let bat_dau = std::time::Instant::now();

        if !dupes.start(
            st.snapshot(),
            st.index_epoch(),
            crate::media::dupescope::DupeScope::LocalOnly,
            Vec::new(),
        ) {
            continue; // ai đó vừa bắt đầu một lượt trước ta
        }

        // Theo dõi tới khi xong, và DỪNG NGAY nếu người dùng gõ tìm kiếm.
        // Việc của họ luôn thắng — họ đang ngồi trước máy chờ câu trả lời, còn
        // lượt quét này không ai đang chờ.
        loop {
            std::thread::sleep(Duration::from_secs(2));
            if !dupes.progress().running {
                break;
            }
            let q = st.generation().load(Ordering::Relaxed);
            if q != QUERY_CUOI.load(Ordering::Relaxed) {
                QUERY_CUOI.store(q, Ordering::Relaxed);
                MOC_YEN.store(bay_gio(), Ordering::Relaxed);
                tracing::info!("quét trùng lặp nền: người dùng đang tìm kiếm — nhường");
                dupes.cancel();
                break;
            }
        }

        // Đánh dấu xong kể cả khi bị nhường: phần đã chốt vẫn được giữ (xem
        // `dupes::find_duplicates`), và kho vân tay đã lưu — nên lần người
        // dùng tự bấm sẽ nhanh hơn hẳn dù lượt nền này chưa chạy hết.
        DA_XONG.store(true, Ordering::Relaxed);
        tracing::info!(
            "quét trùng lặp nền: kết thúc sau {:.0}s, {} nhóm",
            bat_dau.elapsed().as_secs_f64(),
            dupes.progress().groups
        );
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YEN: u64 = 600;

    #[test]
    fn du_dieu_kien_khi_moi_thu_deu_san_sang() {
        assert!(du_dieu_kien(true, false, true, false, YEN));
    }

    #[test]
    fn tat_thi_khong_bao_gio_quet() {
        // Người chưa từng dùng màn Trùng lặp không phải trả giá đọc đĩa cho nó.
        assert!(!du_dieu_kien(false, false, true, false, YEN));
    }

    #[test]
    fn quet_dung_mot_lan_moi_phien() {
        // Chỉ mục ổ trong máy cập nhật mỗi ngày một lần, nên quét lại trong
        // cùng phiên là đọc lại cùng những tệp đó để ra cùng câu trả lời.
        assert!(!du_dieu_kien(true, true, true, false, YEN));
    }

    #[test]
    fn cho_enrichment_xong_da() {
        // Enrichment là đợt đọc đĩa nặng nhất lúc khởi động; chen vào giữa là
        // hai việc cùng tranh đĩa và cả hai cùng chậm.
        assert!(!du_dieu_kien(true, false, false, false, YEN));
    }

    #[test]
    fn khong_chen_vao_luot_quet_dang_chay() {
        assert!(!du_dieu_kien(true, false, true, true, YEN));
    }

    #[test]
    fn may_chua_yen_du_lau_thi_cho() {
        // Người vừa tìm xong rồi quay sang việc khác vẫn đang dùng máy; bắt
        // đầu đọc đĩa ngay dưới tay họ là đúng thứ tính năng này phải tránh.
        assert!(!du_dieu_kien(true, false, true, false, 0));
        assert!(!du_dieu_kien(true, false, true, false, YEN - 1));
        assert!(du_dieu_kien(true, false, true, false, YEN));
    }

    /// Quét nền phải dùng phạm vi CHỈ Ổ TRONG MÁY, không bao giờ là cả NAS.
    ///
    /// Đây là ràng buộc quan trọng nhất của tệp này và cũng là thứ dễ mất nhất
    /// khi ai đó "dọn dẹp" mã: đổi một hằng số là 20–40 máy cùng đọc NAS mỗi
    /// sáng, đúng lúc mọi người vừa tới và bắt đầu mở dự án.
    ///
    /// Đo được: 82% ứng viên (160.982 trên 197.301) nằm trên ổ mạng. Lịch quét
    /// NAS ở `netsched` đã phải giãn giờ ngẫu nhiên giữa các máy để tránh, mà
    /// lượt đó chỉ mất ~2 phút; quét trùng NAS là hàng chục phút.
    ///
    /// Đọc thẳng mã nguồn vì phạm vi được truyền vào một lời gọi bên trong
    /// vòng lặp — không có cách nào quan sát nó mà không dựng cả ứng dụng.
    #[test]
    fn quet_nen_khong_bao_gio_dung_pham_vi_ca_o_mang() {
        let nguon = include_str!("dupeidle.rs");
        assert!(
            nguon.contains("DupeScope::LocalOnly"),
            "quét nền phải dùng phạm vi chỉ ổ trong máy"
        );
        // Chỉ soi phần TRƯỚC khối `mod tests` — chính bài này nhắc tới tên
        // biến thể kia, nên soi cả tệp thì nó tự làm mình đỏ. Và bỏ qua dòng
        // chú thích: một cái tên trong câu giải thích là vô hại, trong một lời
        // gọi thì không.
        let than = nguon.split("mod tests").next().unwrap_or(nguon);
        let goi_ca_o_mang = than
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .any(|l| l.contains("DupeScope::Everything"));
        assert!(
            !goi_ca_o_mang,
            "quét nền KHÔNG được dùng phạm vi cả ổ mạng — 40 máy cùng đọc NAS mỗi sáng"
        );
    }

    #[test]
    fn nguong_yen_lang_hop_ly_so_voi_nhip_kiem_tra() {
        // Nhịp phải ngắn hơn hẳn ngưỡng, nếu không thì thời điểm bắt đầu trễ
        // tới cả một nhịp so với lúc máy thật sự rảnh.
        assert!(NHIP < YEN_LANG);
        assert!(
            YEN_LANG.as_secs() >= 5 * 60,
            "quá ngắn thì chen vào giữa việc của người dùng"
        );
    }

    #[test]
    fn bat_tat_doi_duoc_va_doc_lai_dung() {
        let truoc = dang_bat();
        dat_bat(false);
        assert!(!dang_bat());
        assert!(!du_dieu_kien(dang_bat(), false, true, false, YEN));
        dat_bat(true);
        assert!(dang_bat());
        dat_bat(truoc);
    }
}
