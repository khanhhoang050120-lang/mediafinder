//! Kiểm tra môi trường trước khi mở cửa sổ, và nói bằng tiếng người nếu thiếu.
//!
//! # Vì sao cần
//!
//! Người dùng của phần mềm này không đọc log và không biết WebView2 là gì. Nếu
//! thiếu nó, thứ họ thấy là **một cửa sổ trắng hoặc không có gì cả** — không có
//! câu nào nói tại sao, và không có gì để họ làm tiếp ngoài việc kết luận phần
//! mềm hỏng.
//!
//! Nên chỗ này chạy **trước khi Tauri khởi động**, và nếu thiếu thì hiện một
//! hộp thoại của Windows với đúng một câu hỏi được trả lời: *bây giờ tôi phải
//! làm gì.*
//!
//! # Đã đo: thật sự cần những gì
//!
//! Đọc bảng nhập của tệp exe cho ra 22 DLL, và **không có thư viện nào phải
//! cài thêm**:
//!
//! - `api-ms-win-crt-*` là UCRT, nằm sẵn trong Windows 10 và 11.
//! - `kernel32`, `user32`, `shell32`, `ole32`, `propsys`, `mpr`… đều là thành
//!   phần của Windows.
//! - `vcruntime140.dll` **không** xuất hiện: Rust đã liên kết tĩnh phần đó.
//!
//! Thứ duy nhất ở ngoài là **WebView2 Runtime**, và nó được nạp lúc chạy chứ
//! không nằm trong bảng nhập — nên chỉ kiểm tra được bằng registry, đúng như
//! bộ cài vẫn làm.

use std::fmt;

/// Bản Windows cũ nhất còn chạy được.
///
/// 17763 là Windows 10 phiên bản 1809. Dưới mức đó thì WebView2 không được hỗ
/// trợ chính thức, và có báo lỗi rõ ràng vẫn hơn là để nó hỏng theo cách khó
/// hiểu ở giữa chừng.
const MIN_BUILD: u32 = 17763;

/// Thứ còn thiếu, nếu có.
#[derive(Debug, PartialEq, Eq)]
pub enum Missing {
    /// WebView2 Runtime chưa được cài trên máy này.
    WebView2,
    /// Windows quá cũ.
    WindowsTooOld { build: u32 },
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Missing::WebView2 => write!(f, "thiếu WebView2 Runtime"),
            Missing::WindowsTooOld { build } => write!(f, "Windows quá cũ (build {build})"),
        }
    }
}

/// Câu chữ hiện cho người dùng: tiêu đề, rồi nội dung.
///
/// Viết cho người không biết code. Không có tên API, không có mã lỗi, và câu
/// cuối luôn là một việc cụ thể họ làm được ngay.
pub fn explain(missing: &Missing) -> (String, String) {
    match missing {
        Missing::WebView2 => (
            "MediaFinder chưa chạy được".to_string(),
            // Mỗi dòng dưới 46 ký tự, và tự xuống dòng thay vì để hộp thoại tự
            // ngắt.
            //
            // Đo được: hộp thoại của Windows rộng khoảng 50 ký tự, và dòng nào
            // dài hơn thì nó cắt **vào giữa chữ** chứ không lùi về khoảng
            // trắng — hai bản trước hiện ra "kết thúc bằ / ng", "nó sẽ tự c /
            // ài", "bản mới hơ / n". Với tiếng Việt thì một chữ bị cắt đôi
            // trông như phần mềm lỗi phông.
            concat!(
                "Máy này còn thiếu một thành phần của Windows
",
                "mà MediaFinder cần: WebView2 Runtime.

",
                "Cách xử lý: chạy lại bộ cài MediaFinder.
",
                "Đó là tệp có tên kết thúc bằng \"-setup.exe\".

",
                "Bộ cài đã mang sẵn thành phần này bên trong
",
                "nên nó sẽ tự cài giúp bạn. Không cần tải
",
                "gì thêm, cũng không cần mạng.

",
                "Nếu bạn mở MediaFinder bằng cách chép tệp
",
                "chương trình từ máy khác sang thì đây
",
                "chính là lý do. Hãy dùng bộ cài, đừng chép.",
            )
            .to_string(),
        ),
        Missing::WindowsTooOld { build } => (
            "Windows trên máy này quá cũ".to_string(),
            format!(
                concat!(
                    "MediaFinder cần Windows 10 phiên bản 1809
",
                    "trở lên. Máy này đang ở build {}.

",
                    "Cách xử lý: chạy Windows Update để cập
",
                    "nhật lên bản mới hơn, rồi mở lại
",
                    "MediaFinder.",
                ),
                build
            ),
        ),
    }
}

/// Khe thử: ép kết quả kiểm tra.
///
/// Hộp thoại này là thứ duy nhất người dùng thấy khi máy họ thiếu WebView2, nên
/// nó phải được **nhìn tận mắt** ít nhất một lần — mà tôi không có máy nào thiếu
/// WebView2 để thử. Biến môi trường này cho phép xem đúng hộp thoại đó.
///
/// Không có rủi ro với người dùng: không ai vô tình đặt biến môi trường, và tác
/// dụng duy nhất của nó là làm chương trình từ chối chạy — không phá gì cả.
fn forced() -> Option<Missing> {
    match std::env::var("MF_PREFLIGHT_FORCE").ok()?.as_str() {
        "webview2" => Some(Missing::WebView2),
        "windows-cu" => Some(Missing::WindowsTooOld { build: 9600 }),
        _ => None,
    }
}

/// Có thiếu gì không?
///
/// `None` nghĩa là chạy được.
pub fn check() -> Option<Missing> {
    if let Some(forced) = forced() {
        return Some(forced);
    }
    let build = windows_build();
    if build > 0 && build < MIN_BUILD {
        return Some(Missing::WindowsTooOld { build });
    }
    if !webview2_installed() {
        return Some(Missing::WebView2);
    }
    None
}

/// Ba nơi WebView2 có thể được ghi nhận.
///
/// Cài cho cả máy thì nằm ở `HKLM` (và ở nhánh `WOW6432Node` trên máy 64-bit);
/// cài cho một người dùng thì nằm ở `HKCU`. Thiếu bất kỳ nhánh nào trong ba
/// nhánh này cũng không sao — chỉ cần **một** nhánh có là đủ.
fn webview2_installed() -> bool {
    const CLIENT: &str =
        "SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    const CLIENT_WOW: &str =
        "SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    [
        (HKEY_LOCAL_MACHINE, CLIENT_WOW),
        (HKEY_LOCAL_MACHINE, CLIENT),
        (HKEY_CURRENT_USER, CLIENT),
    ]
    .iter()
    .any(|(root, path)| {
        read_string(*root, path, "pv").is_some_and(|v| !v.is_empty() && v != "0.0.0.0")
    })
}

/// Build của Windows, đọc từ registry.
///
/// Không dùng `GetVersionEx`: từ Windows 8.1 trở đi hàm đó nói dối với chương
/// trình không khai báo tương thích trong manifest. Registry thì không.
fn windows_build() -> u32 {
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
    read_string(
        HKEY_LOCAL_MACHINE,
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
        "CurrentBuildNumber",
    )
    .and_then(|s| s.trim().parse().ok())
    .unwrap_or(0)
}

fn read_string(
    root: windows::Win32::System::Registry::HKEY,
    path: &str,
    name: &str,
) -> Option<String> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ};

    let path = HSTRING::from(path);
    let name = HSTRING::from(name);
    let mut buf = [0u16; 256];
    let mut size = (buf.len() * 2) as u32;

    unsafe {
        let status = RegGetValueW(
            root,
            PCWSTR(path.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        );
        if status.is_err() {
            return None;
        }
    }
    let chars = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..chars.min(buf.len())]))
}

/// Hiện hộp thoại rồi dừng chương trình.
///
/// Dùng `MessageBoxW` của Windows chứ không phải cửa sổ của ứng dụng: lúc này
/// chưa có cửa sổ nào, và cái thiếu có thể chính là thứ dùng để vẽ cửa sổ.
pub fn show_and_exit(missing: &Missing) -> ! {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONWARNING, MB_OK};

    let (title, body) = explain(missing);
    tracing::error!("không khởi động được: {missing}");
    unsafe {
        let title = HSTRING::from(title);
        let body = HSTRING::from(body);
        MessageBoxW(
            None,
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONWARNING,
        );
    }
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_seam_stays_shut_unless_asked() {
        // Không đặt biến thì không có gì bị ép — nếu sai chỗ này thì phần mềm
        // từ chối chạy trên mọi máy.
        std::env::remove_var("MF_PREFLIGHT_FORCE");
        assert_eq!(forced(), None);
    }

    #[test]
    fn this_machine_has_everything_it_needs() {
        // Máy dựng ra bản phát hành mà thiếu thì mọi phép thử khác đều vô nghĩa.
        assert_eq!(check(), None, "máy này đang thiếu thứ gì đó");
    }

    #[test]
    fn the_build_number_is_readable_and_believable() {
        let build = windows_build();
        assert!(build >= MIN_BUILD, "đọc ra build {build}");
    }

    #[test]
    fn webview2_is_found_where_the_installer_looks() {
        assert!(webview2_installed());
    }

    #[test]
    fn no_line_is_wide_enough_for_windows_to_cut_a_word_in_half() {
        // Đo được: hộp thoại rộng khoảng 50 ký tự và cắt vào giữa chữ khi tràn.
        // 46 là ngưỡng an toàn, và test này giữ nó khỏi trôi khi ai đó sửa câu.
        for m in [Missing::WebView2, Missing::WindowsTooOld { build: 9600 }] {
            let (_, body) = explain(&m);
            for line in body.lines() {
                assert!(
                    line.chars().count() <= 46,
                    "dòng dài {} ký tự, sẽ bị cắt giữa chữ: {line:?}",
                    line.chars().count()
                );
            }
        }
    }

    #[test]
    fn every_message_names_something_the_user_can_actually_do() {
        for m in [Missing::WebView2, Missing::WindowsTooOld { build: 9600 }] {
            let (title, body) = explain(&m);
            assert!(!title.is_empty());
            assert!(
                body.contains("Cách xử lý:"),
                "thông báo phải nói người dùng làm gì tiếp:\n{body}"
            );
            // Không có tên API hay mã lỗi trong thứ người dùng đọc.
            for rac in ["HKEY", "registry", "RegGetValue", "0x", "Runtime error"] {
                assert!(!body.contains(rac), "lộ chi tiết kỹ thuật {rac:?}:\n{body}");
            }
        }
    }
}
