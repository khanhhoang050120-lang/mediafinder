//! Sau khi cập nhật xong thì cửa sổ phải hiện ra.
//!
//! # Vấn đề
//!
//! Hộp thoại cập nhật hứa *"ứng dụng sẽ tự khởi động lại"*. Trên máy studio thì
//! lời hứa đó **không giữ được**, và người dùng không có cách nào biết tại sao.
//!
//! Chuỗi sự việc:
//!
//! 1. Lối tắt Startup chạy app với `--minimized` ([`crate::setup`]) — cố ý, vì
//!    bật một cửa sổ lên mỗi lần đăng nhập là làm phiền.
//! 2. Người dùng gọi cửa sổ bằng phím tắt, thấy lời mời cập nhật, bấm đồng ý.
//! 3. `tauri-plugin-updater` khởi động lại app kèm **đúng tham số dòng lệnh
//!    hiện tại** — `updater.rs:797` lấy `current_exe_args()[1..]` rồi chuyển
//!    tiếp qua `/ARGS`. Tức là `--minimized` đi theo.
//! 4. App mở lại **ẩn ở khay hệ thống**. Màn hình không có gì thay đổi.
//!
//! Người dùng vừa bấm "Cập nhật", chờ, rồi thấy… không có gì. Kết luận tự nhiên
//! nhất là bản cập nhật hỏng — trong khi nó đã cài xong hoàn toàn bình thường.
//!
//! # Cách sửa
//!
//! Không sửa được ở phía thư viện: `current_exe_args` là `pub(crate)`, không có
//! đường ghi đè. Nên đánh dấu ý định trước khi cài, và đọc lại lúc khởi động.
//!
//! Một tệp mốc rỗng trong thư mục dữ liệu của app: ghi ngay trước khi bộ cài
//! chạy, đọc-rồi-xoá ở lần khởi động kế tiếp. Có mốc nghĩa là "lần chạy này đến
//! từ một bản cập nhật, nên hiện cửa sổ dù dòng lệnh nói gì".
//!
//! Vì sao là tệp chứ không phải biến môi trường hay tham số: tiến trình cũ
//! **chết hẳn** trước khi tiến trình mới sinh ra, và bộ cài đứng giữa hai bên.
//! Không có gì sống sót qua khoảng đó ngoài đĩa.
//!
//! Mốc tự dọn: đọc là xoá ngay. Sót lại một mốc chỉ khiến một lần khởi động
//! hiện cửa sổ thừa — phiền, nhưng không mất gì; còn xoá nhầm thì đúng bằng
//! hành vi cũ.

use std::path::PathBuf;

/// Tên tệp mốc. Dấu chấm đầu để nó nằm cạnh nhau khi liệt kê thư mục.
const TEN_MOC: &str = ".vua-cap-nhat";

/// Đường dẫn tệp mốc, cạnh chỉ mục trong thư mục dữ liệu của app.
fn duong_dan() -> Option<PathBuf> {
    crate::index::persist::cache_path()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(TEN_MOC)))
}

/// Đánh dấu "lần khởi động kế tiếp là do vừa cập nhật".
///
/// Gọi ngay trước khi giao quyền cho bộ cài. Thất bại thì bỏ qua trong im lặng:
/// không ghi được mốc chỉ có nghĩa là cửa sổ mở lại ẩn — đúng bằng hành vi
/// hiện tại, không tệ hơn. Chặn cả một bản cập nhật vì không ghi nổi một tệp
/// rỗng thì mới là đánh đổi sai.
pub fn danh_dau() {
    let Some(p) = duong_dan() else {
        tracing::debug!("không xác định được chỗ ghi mốc cập nhật");
        return;
    };
    match std::fs::write(&p, b"") {
        Ok(()) => tracing::info!("đã đánh dấu: mở cửa sổ sau khi cập nhật xong"),
        Err(e) => tracing::debug!("không ghi được mốc cập nhật: {e}"),
    }
}

/// Lần chạy này có phải đến từ một bản cập nhật không. **Đọc là xoá.**
///
/// Xoá ngay trong cùng lần gọi, kể cả khi phần sau đó hỏng: một mốc còn lại sẽ
/// khiến MỌI lần khởi động sau đều bật cửa sổ lên, biến một phiền toái một lần
/// thành một phiền toái vĩnh viễn.
pub fn vua_cap_nhat() -> bool {
    let Some(p) = duong_dan() else {
        return false;
    };
    doc_va_xoa(&p)
}

/// Phần đọc-rồi-xoá, tách ra để kiểm thử được với một tệp thật.
///
/// Trả `true` đúng một lần cho mỗi mốc: lần gọi thứ hai trên cùng đường dẫn
/// phải trả `false`, nếu không thì mọi lần khởi động sau đều bật cửa sổ lên.
fn doc_va_xoa(p: &std::path::Path) -> bool {
    if !p.exists() {
        return false;
    }
    if let Err(e) = std::fs::remove_file(p) {
        // Xoá không được thì coi như không có mốc. Thà bỏ lỡ một lần hiện cửa
        // sổ còn hơn hiện nó mãi mãi.
        tracing::warn!("không xoá được mốc cập nhật, bỏ qua: {e}");
        return false;
    }
    tracing::info!("lần chạy này đến từ bản cập nhật — hiện cửa sổ");
    true
}

/// Có nên hiện cửa sổ lúc khởi động không.
///
/// Tách riêng khỏi phần đọc đĩa để kiểm thử được cả bảng quyết định mà không
/// cần một tệp thật.
pub fn nen_hien_cua_so(co_co_minimized: bool, vua_cap_nhat: bool) -> bool {
    // Vừa cập nhật thì luôn hiện, kể cả khi dòng lệnh bảo ẩn — chính đó là
    // trường hợp cần sửa. Người dùng vừa bấm một nút và đang chờ thấy kết quả.
    if vua_cap_nhat {
        return true;
    }
    !co_co_minimized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn khoi_dong_binh_thuong_thi_hien_cua_so() {
        assert!(nen_hien_cua_so(false, false));
    }

    #[test]
    fn loi_tat_startup_thi_khong_hien() {
        // Bật cửa sổ mỗi lần đăng nhập là làm phiền — đây là lý do
        // `--minimized` tồn tại, và bản sửa không được phá điều đó.
        assert!(!nen_hien_cua_so(true, false));
    }

    #[test]
    fn vua_cap_nhat_thi_hien_du_dong_lenh_bao_an() {
        // Đây là ca của chính bản sửa này. Bộ cập nhật chuyển tiếp nguyên
        // `--minimized` sang tiến trình mới, nên nếu chỉ nhìn dòng lệnh thì
        // cửa sổ không bao giờ hiện, và người vừa bấm "Cập nhật" tưởng bản
        // cập nhật hỏng.
        assert!(nen_hien_cua_so(true, true));
    }

    #[test]
    fn vua_cap_nhat_tu_dong_lenh_khong_co_minimized_cung_hien() {
        assert!(nen_hien_cua_so(false, true));
    }

    #[test]
    fn moc_chi_dung_duoc_dung_mot_lan() {
        // Ca này canh chỗ nguy hiểm nhất của module: nếu mốc KHÔNG bị xoá sau
        // khi đọc thì mọi lần khởi động sau đều bật cửa sổ lên — kể cả lúc
        // đăng nhập, đúng thứ mà `--minimized` sinh ra để tránh. Một phiền
        // toái một lần biến thành phiền toái vĩnh viễn.
        //
        // Phép thử bằng cách phá mã đã lộ ra rằng phần này trước đó không có
        // bài nào canh: bỏ hẳn lệnh xoá mà cả bộ kiểm thử vẫn xanh.
        let p = std::env::temp_dir().join(format!("mf-test-moc-{}", std::process::id()));
        let _ = std::fs::remove_file(&p);

        assert!(!doc_va_xoa(&p), "chưa có mốc thì phải trả false");

        std::fs::write(&p, b"").expect("ghi mốc thử");
        assert!(doc_va_xoa(&p), "có mốc thì phải trả true");
        assert!(!p.exists(), "đọc xong phải xoá mốc");
        assert!(!doc_va_xoa(&p), "lần thứ hai phải trả false");
    }

    /// Tên tệp tạm phải mang số hiệu tiến trình.
    ///
    /// Không thuộc về module này, nhưng đặt ở đây vì nó cùng một gốc: từ khi
    /// có lịch tự quét ổ mạng thì có ĐÚNG hai tiến trình cùng ghi cache —
    /// tiến trình giao diện và tác vụ nền `--index`. Cùng một tên tạm nghĩa
    /// là hai `File::create` trên một đường dẫn, ghi đè lên nhau, rồi cái
    /// `rename` sau xuất bản một tệp trộn lẫn.
    ///
    /// Không có bài này thì ai đó "dọn dẹp" tên tệp cho gọn sẽ mang lỗi trở
    /// lại, và nó chỉ lộ ra khi hai lượt quét tình cờ trùng giờ.
    #[test]
    fn ten_tep_tam_phai_rieng_theo_tien_trinh() {
        let pid = std::process::id().to_string();
        for (nguon, mau) in [
            (include_str!("index/persist.rs"), "index.bin."),
            (include_str!("media/enrich.rs"), "bin."),
            (include_str!("ipc/elevate.rs"), "json."),
        ] {
            assert!(
                nguon.contains("std::process::id()"),
                "một chỗ ghi tệp tạm không còn mang số hiệu tiến trình (mẫu {mau})"
            );
        }
        assert!(!pid.is_empty());
    }
}
