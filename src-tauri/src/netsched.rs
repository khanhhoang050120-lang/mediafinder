//! Lịch tự quét lại ổ mạng.
//!
//! # Vì sao phải có tệp này
//!
//! Ổ trong máy được tác vụ Windows quét lại mỗi ngày, và việc đó gần như miễn
//! phí — nó đọc USN journal, hỏi "có gì đổi từ lần trước?" và nhận câu trả lời
//! trong **1 mili giây**.
//!
//! Ổ mạng thì không có journal nào để hỏi. Tệ hơn: tác vụ Windows chạy
//! **elevated**, mà ổ mạng ánh xạ thuộc về phiên đăng nhập, nên tiến trình
//! elevated **không nhìn thấy chúng** (CHECK-007). Nói cách khác, đường quét
//! tự động đang có về mặt kỹ thuật *không thể* chạm tới ổ mạng — không phải
//! ai đó quên gọi.
//!
//! Hệ quả đo được: `scan_network_volumes()` trước tệp này có đúng **một** chỗ
//! gọi, là nút "+ Ổ mạng" người dùng tự bấm. Ai không bấm thì phần NAS của chỉ
//! mục đứng im vĩnh viễn — một tệp đồng nghiệp vừa đẩy lên sáng nay là một tệp
//! không tìm ra.
//!
//! # Vì sao lịch nằm trong tiến trình GUI
//!
//! Vì đó là nơi *duy nhất* thấy được ổ mạng. Đây không phải đường tắt: cùng lý
//! do khiến `scan_network_volumes` phải chạy unelevated.
//!
//! # Cái giá, đo trên NAS thật
//!
//! Log của một lần quét đủ bốn ổ studio:
//!
//! | Ổ | Máy chủ | Thời gian | Thư mục |
//! |---|---|---|---|
//! | `F:` | .214 | 13,1s | 4.108 |
//! | `H:` | .214 | 18,7s | 8.394 |
//! | `Y:` | .213 | **477,0s** | 7.686 |
//! | `Z:` | .213 | 15,1s | 958 |
//!
//! Tổng **8 phút 44**, trong đó riêng `Y:` chiếm 91%. Con số đó là lý do lịch
//! ở đây thưa chứ không dày, và là lý do có [`SAU_KHOI_DONG`]: tám phút rưỡi
//! đọc mạng ngay lúc người ta vừa đăng nhập là tám phút rưỡi tranh băng thông
//! với chính công việc họ đang mở ra làm.
//!
//! Một điều đã thử và **bác bỏ**: nâng số luồng của `rayon`. Phép đo đầu tiên
//! cho thấy 64 luồng nhanh hơn 12 luồng mười một lần, nhưng phép đo đó tự lừa
//! mình — nó chạy 12 luồng trên thư mục lạnh rồi chạy 64 luồng trên chính thư
//! mục vừa được Windows cache. Đo lại bằng hai mẫu riêng biệt cùng lạnh thì
//! **12 luồng còn nhanh hơn 64** (987 so với 586 thư mục/giây). Số luồng không
//! phải nguyên nhân.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Chờ bao lâu sau khi đăng nhập rồi mới quét lần đầu.
///
/// Không quét ngay: lúc mới đăng nhập là lúc máy bận nhất — Windows còn đang
/// dựng desktop, các phần mềm khác cùng khởi động, và người dùng đang mở thứ
/// họ định làm. Chen một lượt đọc mạng tám phút rưỡi vào đó là tranh băng
/// thông với chính công việc đó.
///
/// Năm phút cũng đủ để một người "đăng nhập rồi đổi ý tắt máy" không phải trả
/// giá cho một lượt quét chẳng ai dùng.
pub const SAU_KHOI_DONG: Duration = Duration::from_secs(5 * 60);

/// Khoảng cách giữa hai lượt quét.
///
/// Mười hai tiếng, tức khoảng hai lượt một ngày: một lượt sau khi đăng nhập
/// buổi sáng, một lượt quanh giữa trưa.
///
/// Không dày hơn, và con số 8 phút 44 ở đầu tệp là lý do. Quét bốn lần một
/// ngày là **35 phút** đọc mạng mỗi máy mỗi ngày; nhân với 20–40 máy thì cái
/// giá đó đổ hết lên chính NAS mà mọi người đang cần dùng để làm việc.
pub const GIUA_HAI_LUOT: Duration = Duration::from_secs(12 * 60 * 60);

/// Kiểm tra lại mỗi phút xem đã tới giờ chưa.
///
/// Ngủ từng phút thay vì ngủ thẳng mười hai tiếng, vì máy có thể ngủ đông giữa
/// chừng: một `sleep` dài đo theo đồng hồ *chạy*, nên máy ngủ tám tiếng thì
/// lượt quét bị đẩy lùi tám tiếng. Đối chiếu với đồng hồ tường mỗi phút thì
/// máy tỉnh dậy là biết mình đã trễ.
const NHIP: Duration = Duration::from_secs(60);

/// Đợi thêm bao lâu khi tới giờ mà máy đang bận.
///
/// Nếu đang có một lượt quét khác chạy (người dùng tự bấm, hoặc lượt quét ổ
/// trong máy), lịch này lùi lại chứ không chen vào — hai lượt cùng ghi một tệp
/// cache là hỏng cả hai.
const THU_LAI: Duration = Duration::from_secs(2 * 60);

/// Giãn giờ tối đa bao nhiêu giây giữa các máy.
///
/// Hai mươi phút. Không có nó thì 20–40 máy của studio cùng đăng nhập lúc 8
/// giờ sáng sẽ cùng bắt đầu quét sau đúng [`SAU_KHOI_DONG`], và cả bốn mươi
/// máy cùng nện vào NAS một lúc — biến một lượt quét tám phút rưỡi thành một
/// cơn nghẽn mà chính những người đang cần NAS để làm việc phải chịu.
const GIAN_TOI_DA: u64 = 20 * 60;

/// Độ giãn của riêng máy này, cố định theo tên máy.
///
/// Dùng tên máy thay vì số ngẫu nhiên, vì hai lý do. Thứ nhất: không phải thêm
/// một thư viện chỉ để lấy đúng một con số. Thứ hai, và quan trọng hơn — nó
/// **ổn định**. Máy A luôn quét sớm hơn máy B, mỗi ngày, thay vì hai máy bốc
/// thăm lại mỗi lần khởi động và thỉnh thoảng lại trùng nhau.
///
/// Không cần chất lượng mật mã: chỉ cần các tên máy khác nhau rơi vào các
/// khoảnh khắc khác nhau.
fn do_gian() -> u64 {
    let ten = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default();
    if ten.is_empty() {
        return 0;
    }
    // FNV-1a, 64 bit. Chọn nó vì viết gọn trong sáu dòng và trải đều — không
    // vì tính chất mật mã nào.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in ten.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h % GIAN_TOI_DA
}

/// Lịch đã chạy chưa — chặn việc dựng hai luồng nếu `setup` bị gọi hai lần.
static DA_CHAY: AtomicBool = AtomicBool::new(false);

/// Mốc lượt quét gần nhất, giây Unix. `0` nghĩa là chưa quét lần nào trong
/// phiên này.
///
/// Cố ý **không** lưu xuống đĩa. Một mốc đọc từ phiên trước sẽ nói "vừa quét
/// hai tiếng trước" trong khi máy vừa khởi động lại và chỉ mục trong bộ nhớ
/// hoàn toàn mới — mà lượt quét sau khi đăng nhập chính là lượt đáng giá nhất
/// trong ngày.
static LAN_CUOI: AtomicU64 = AtomicU64::new(0);

/// Giây Unix hiện tại.
fn bay_gio() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Đã tới lúc quét lượt kế tiếp chưa.
///
/// Tách khỏi phần chạy thật để kiểm thử được mà không cần NAS, không cần chờ
/// mười hai tiếng, và không cần một `AppHandle`.
///
/// * `lan_cuoi` — mốc lượt gần nhất (giây Unix), `0` nếu chưa có.
/// * `bat_dau` — mốc luồng bắt đầu chạy, để tính [`SAU_KHOI_DONG`].
/// * `gian` — độ giãn của riêng máy này, xem [`do_gian`]. Chỉ áp cho lượt đầu:
///   các lượt sau đã tự lệch nhau rồi, vì chúng đếm từ lượt trước của chính
///   máy đó.
pub fn den_luot(bay_gio: u64, lan_cuoi: u64, bat_dau: u64, gian: u64) -> bool {
    if lan_cuoi == 0 {
        // Lượt đầu tiên: đếm từ lúc khởi động, không phải từ mốc 0 — nếu không
        // thì `bay_gio - 0` luôn lớn hơn mọi ngưỡng và lượt quét chạy ngay
        // giữa cơn bão đăng nhập.
        return bay_gio.saturating_sub(bat_dau) >= SAU_KHOI_DONG.as_secs() + gian;
    }
    bay_gio.saturating_sub(lan_cuoi) >= GIUA_HAI_LUOT.as_secs()
}

/// Ghi lại rằng vừa quét xong.
///
/// Gọi cả khi lượt quét bị huỷ giữa chừng: một lượt bị huỷ vẫn đã tốn công của
/// NAS, và thử lại ngay lập tức là cách chắc chắn nhất để biến một lần huỷ
/// thành một vòng lặp bận.
pub fn danh_dau_da_quet() {
    LAN_CUOI.store(bay_gio(), Ordering::Relaxed);
}

/// Số giây kể từ lượt quét gần nhất; `None` nếu chưa quét lần nào.
pub fn giay_ke_tu_lan_cuoi() -> Option<u64> {
    match LAN_CUOI.load(Ordering::Relaxed) {
        0 => None,
        t => Some(bay_gio().saturating_sub(t)),
    }
}

/// Dựng luồng lịch. Gọi một lần lúc `setup`.
///
/// Không làm gì nếu đã dựng rồi — `setup` chạy hai lần là chuyện không nên
/// xảy ra, nhưng hai luồng cùng quét NAS thì tệ hơn nhiều so với một dòng log.
pub fn dung_lich(app: tauri::AppHandle) {
    if DA_CHAY.swap(true, Ordering::SeqCst) {
        tracing::warn!("lịch quét ổ mạng đã chạy rồi, bỏ qua lần dựng thứ hai");
        return;
    }

    if let Err(e) = std::thread::Builder::new()
        .name("netsched".into())
        .spawn(move || vong_lap(app))
    {
        // Không dựng được luồng thì mở lại chốt: một lần thử sau còn có cơ hội.
        DA_CHAY.store(false, Ordering::SeqCst);
        tracing::warn!("không dựng được luồng lịch quét ổ mạng: {e}");
    }
}

/// Vòng lặp: cứ mỗi [`NHIP`] thì hỏi đã tới lượt chưa.
fn vong_lap(app: tauri::AppHandle) {
    use tauri::Manager;

    let bat_dau = bay_gio();
    let gian = do_gian();
    tracing::info!(
        "lịch quét ổ mạng: lượt đầu sau {} phút (giãn thêm {} phút cho máy này), rồi mỗi {} tiếng",
        SAU_KHOI_DONG.as_secs() / 60,
        gian / 60,
        GIUA_HAI_LUOT.as_secs() / 3600
    );

    loop {
        std::thread::sleep(NHIP);

        if !den_luot(bay_gio(), LAN_CUOI.load(Ordering::Relaxed), bat_dau, gian) {
            continue;
        }

        let Some(st) = app.try_state::<crate::state::AppState>() else {
            // Ứng dụng đang tắt.
            return;
        };

        // Máy đang bận thì lùi lại chứ không chen vào: hai lượt quét cùng ghi
        // một tệp cache là hỏng cả hai. Lùi bằng cách ngủ thêm rồi hỏi lại —
        // KHÔNG đánh dấu đã quét, vì chưa quét gì cả.
        if st.is_scanning() {
            tracing::info!("tới lượt quét ổ mạng nhưng đang có lượt quét khác — lùi lại");
            std::thread::sleep(THU_LAI);
            continue;
        }

        // Không có ổ mạng nào thì không có việc gì. Vẫn đánh dấu, để khỏi hỏi
        // lại mỗi phút trên máy chẳng bao giờ gắn ổ mạng.
        if crate::ipc::commands::network_drives().is_empty() {
            danh_dau_da_quet();
            continue;
        }

        st.set_scanning(true);
        st.request_cancel(false);
        let ket_qua = crate::scan_network_volumes(st.cancel_flag());
        st.set_scanning(false);

        // Đánh dấu kể cả khi bị huỷ: một lượt huỷ vẫn tốn công của NAS, và
        // thử lại ngay là cách chắc chắn nhất để biến nó thành vòng lặp bận.
        danh_dau_da_quet();

        tracing::info!(
            "lịch quét ổ mạng xong: {} ổ · {} tệp · {:.1}s{}",
            ket_qua.drives,
            ket_qua.files,
            ket_qua.seconds,
            if ket_qua.cancelled {
                " (bị huỷ)"
            } else {
                ""
            }
        );

        // Không cần bắn sự kiện gì cho giao diện: `watch_cache` đang theo dõi
        // chính tệp mà `scan_network_volumes` vừa ghi, và nó bắn
        // `index-reloaded` khi thấy tệp đổi. Bắn thêm ở đây là hai đường cùng
        // nói một tin, và đường thứ hai sẽ lặng lẽ sai đi khi cách lưu đổi.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luot_dau_cho_het_khoang_lang_sau_khoi_dong() {
        let bat_dau = 1_000_000;
        // Vừa khởi động: chưa tới lượt.
        assert!(!den_luot(bat_dau, 0, bat_dau, 0));
        // Còn một giây nữa mới đủ.
        assert!(!den_luot(
            bat_dau + SAU_KHOI_DONG.as_secs() - 1,
            0,
            bat_dau,
            0
        ));
        // Đủ rồi.
        assert!(den_luot(bat_dau + SAU_KHOI_DONG.as_secs(), 0, bat_dau, 0));
    }

    #[test]
    fn luot_dau_khong_duoc_tinh_tu_moc_khong() {
        // Nếu `lan_cuoi == 0` bị đem trừ thẳng vào `bay_gio` thì hiệu là cả
        // nửa thế kỷ giây, luôn vượt mọi ngưỡng, và lượt quét chạy ngay lúc
        // đăng nhập — đúng lúc đắt nhất.
        let bat_dau = 1_700_000_000;
        assert!(!den_luot(bat_dau + 1, 0, bat_dau, 0));
    }

    #[test]
    fn cac_luot_sau_cach_nhau_dung_muoi_hai_tieng() {
        let cuoi = 1_700_000_000;
        assert!(!den_luot(cuoi + 1, cuoi, 0, 0));
        assert!(!den_luot(cuoi + GIUA_HAI_LUOT.as_secs() - 1, cuoi, 0, 0));
        assert!(den_luot(cuoi + GIUA_HAI_LUOT.as_secs(), cuoi, 0, 0));
    }

    #[test]
    fn dong_ho_chay_lui_khong_lam_no_quet_lien_tuc() {
        // Đổi giờ hệ thống, hoặc đồng bộ NTP kéo đồng hồ lùi lại. `saturating_sub`
        // cho ra 0 chứ không tràn số — nên câu trả lời là "chưa tới lượt", và
        // lịch chờ tiếp thay vì quét mỗi phút một lần.
        let cuoi = 1_700_000_000;
        assert!(!den_luot(cuoi - 10_000, cuoi, 0, 0));
        assert!(!den_luot(0, cuoi, 0, 0));
    }

    #[test]
    fn gian_gio_lam_luot_dau_toi_muon_hon_dung_bang_do_gian() {
        let bat_dau = 1_000_000;
        let gian = 7 * 60;
        // Tới đúng mốc thường lệ nhưng chưa tính phần giãn: chưa đến lượt.
        assert!(!den_luot(
            bat_dau + SAU_KHOI_DONG.as_secs(),
            0,
            bat_dau,
            gian
        ));
        // Đủ cả phần giãn thì mới tới.
        assert!(den_luot(
            bat_dau + SAU_KHOI_DONG.as_secs() + gian,
            0,
            bat_dau,
            gian
        ));
    }

    #[test]
    fn gian_gio_khong_cham_toi_cac_luot_sau() {
        // Các lượt sau đếm từ lượt trước của chính máy đó, nên chúng đã lệch
        // nhau sẵn. Cộng thêm phần giãn vào đây là dồn độ trễ mỗi ngày một ít,
        // và sau vài tuần thì máy có tên "xui" quét muộn hơn hẳn máy khác.
        let cuoi = 1_700_000_000;
        let sau = cuoi + GIUA_HAI_LUOT.as_secs();
        assert!(den_luot(sau, cuoi, 0, 0));
        assert!(den_luot(sau, cuoi, 0, GIAN_TOI_DA));
    }

    #[test]
    fn do_gian_luon_nam_trong_khoang_va_on_dinh() {
        let g = do_gian();
        assert!(g < GIAN_TOI_DA, "độ giãn {g} vượt trần {GIAN_TOI_DA}");
        // Ổn định: gọi hai lần phải ra cùng một số, nếu không thì mỗi phút
        // vòng lặp lại hỏi một mốc khác và lượt đầu không bao giờ tới.
        assert_eq!(g, do_gian());
    }

    #[test]
    fn cac_ten_may_khac_nhau_roi_vao_cac_khoanh_khac_khac_nhau() {
        // Băm cùng cách mà `do_gian` dùng. Nếu hàm băm dồn cục thì phần giãn
        // vô dụng: bốn mươi máy vẫn quét cùng lúc, chỉ là muộn hơn.
        fn bam(ten: &str) -> u64 {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in ten.as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x1000_0000_01b3);
            }
            h % GIAN_TOI_DA
        }
        let tens: Vec<String> = (1..=40).map(|n| format!("STUDIO-{n:02}")).collect();
        let mut moc: Vec<u64> = tens.iter().map(|t| bam(t)).collect();
        moc.sort_unstable();
        moc.dedup();
        // Bốn mươi tên máy phải cho ít nhất ba mươi khoảnh khắc khác nhau.
        assert!(
            moc.len() >= 30,
            "chỉ có {} mốc khác nhau cho 40 máy — phần giãn giờ không trải đều",
            moc.len()
        );
    }

    #[test]
    fn hai_muc_thoi_gian_phai_hop_ly_voi_nhau() {
        // Nhịp kiểm tra phải ngắn hơn hẳn khoảng cách hai lượt, nếu không thì
        // lượt quét trễ tới cả một nhịp.
        assert!(NHIP < GIUA_HAI_LUOT);
        assert!(NHIP < SAU_KHOI_DONG);
        // Và khoảng lặng sau khởi động phải ngắn hơn một lượt, nếu không thì
        // lượt đầu tiên chẳng bao giờ là lượt đầu tiên.
        assert!(SAU_KHOI_DONG < GIUA_HAI_LUOT);
        // Lùi lại khi máy bận phải ngắn hơn nhịp chờ chung, nếu không một lần
        // bận đẩy lượt quét đi xa hơn cả một chu kỳ kiểm tra.
        assert!(THU_LAI < GIUA_HAI_LUOT);
        // Phần giãn phải nhỏ hơn hẳn khoảng lặng đầu, nếu không thì máy "xui"
        // nhất chờ lâu gấp đôi máy "may" nhất trước lượt quét đầu tiên.
        assert!(GIAN_TOI_DA <= SAU_KHOI_DONG.as_secs() * 4);
    }
}
