//! Making the app work by itself on a machine it has never run on.
//!
//! Two things have to exist for MediaFinder to behave the way it is described:
//! a shortcut in the user's Startup folder, and a scheduled task that refreshes
//! the index. On the machine this was built on, both were created by hand. That
//! is fine for one machine and useless for forty: a new user would install the
//! app, get no hotkey after the next login, and a UAC prompt on every single
//! "Quét lại".
//!
//! # One prompt, not two
//!
//! Neither piece needs its own permission dialog.
//!
//! - The **scheduled task** needs Administrator to register. It is created from
//!   inside the elevated indexer — the process the user already approved for
//!   the first scan — so it costs no prompt of its own.
//! - The **Startup shortcut** needs no privilege at all: it is a file in the
//!   user's own profile. The GUI writes it.
//!
//! So the whole setup rides along with the first scan the user asks for, and
//! after that the task carries the privilege and nothing ever prompts again.
//!
//! # Nothing here overwrites a user's choice
//!
//! Both functions check before they create. Someone who deletes the shortcut
//! because they do not want the app at login, or who edits the task to run at a
//! different hour, gets to keep that decision — the next scan will not quietly
//! undo it.

use std::path::PathBuf;

use crate::ipc::elevate::TASK_NAME;

/// Name of the shortcut this writes into the Startup folder.
const SHORTCUT_NAME: &str = "MediaFinder.lnk";

/// What the scheduled task does, and when.
///
/// Written as XML rather than assembled from `schtasks` flags because the flags
/// cannot express two triggers at once, and "at logon **and** daily" is exactly
/// what this needs. `{EXE}` and `{USER}` are filled in before use.
///
/// `HighestAvailable` is the whole point: it is what lets the task read the USN
/// journal, which an ordinary process cannot ([CHECK-004](../../docs/check.md)).
/// A task started this way runs elevated **without** a prompt, which is why
/// "Quét lại" is free after the first time.
const TASK_XML: &str = r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Cập nhật chỉ mục MediaFinder từ USN journal. Chạy nhanh, vài giây. [schedule-v2: 15 min]</Description>
    <URI>\{TASK}</URI>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{USER}</UserId>
      <Delay>PT1M</Delay>
    </LogonTrigger>
    <CalendarTrigger>
      <StartBoundary>2026-01-01T13:00:00</StartBoundary>
      <Enabled>true</Enabled>
      <ScheduleByDay>
        <DaysInterval>1</DaysInterval>
      </ScheduleByDay>
      <!-- Lịch v2 (P9 giai đoạn 2, phiên bản thực dụng): bản vá gia tăng chỉ
           tốn ~0,45 giây, nên lặp mỗi 15 phút gần như miễn phí — và tệp mới
           tải về tự hiện trong danh sách sau tối đa một khắc, thay vì "ngày
           mai". Realtime thật sự cần đọc USN trong GUI, mà GUI cố ý chạy
           asInvoker — bức tường đó không đáng phá vì 15 phút đã đủ tươi. -->
      <Repetition>
        <Interval>PT15M</Interval>
        <Duration>P1D</Duration>
        <StopAtDurationEnd>false</StopAtDurationEnd>
      </Repetition>
    </CalendarTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{USER}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <ExecutionTimeLimit>PT1H</ExecutionTimeLimit>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{EXE}</Command>
      <Arguments>--index</Arguments>
    </Exec>
  </Actions>
</Task>
"#;

/// Run a `schtasks` command without flashing a console window.
///
/// This happens while the user is looking at the app, and a black rectangle
/// appearing for a fraction of a second reads as something going wrong.
fn schtasks(args: &[&str]) -> std::io::Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("schtasks.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

/// Is the refresh task already registered?
pub fn scheduled_task_exists() -> bool {
    schtasks(&["/Query", "/TN", TASK_NAME])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Dấu hiệu phiên bản lịch, nằm trong Description của task.
///
/// 20–40 máy ngoài kia đã đăng ký lịch v1 (đăng nhập + mỗi ngày). Chúng
/// không tự biết lịch mới tồn tại — nhưng chính task đó chạy indexer với
/// quyền cao, nên indexer ở lần chạy kế tiếp có thể tự thay lịch cho mình.
///
/// **Thuần ASCII có chủ đích.** Bản đầu dùng "[lịch v2" — marker nằm đúng
/// trong task, nhưng `schtasks /XML` in ra UTF-8 và chữ `ị` là ký tự nhiều
/// byte, nên mọi phép so trên mẩu-ASCII-lọc-ra đều trượt. Hậu quả không hề
/// vô hại: lịch đã nâng vẫn bị coi là cũ, và indexer xoá-tạo-lại task ở
/// **mỗi** lần chạy, mãi mãi. Đo trên máy thật mới lộ ra (P22).
const SCHEDULE_MARK: &str = "[schedule-v2:";

fn schedule_is_current() -> bool {
    let Ok(out) = schtasks(&["/Query", "/TN", TASK_NAME, "/XML"]) else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    // schtasks /XML in ra UTF-16 hoặc mã trang OEM tuỳ máy; marker toàn ASCII
    // trừ chữ có dấu — so bằng đoạn ASCII "ch v2" là đủ chắc và miễn nhiễm
    // với mã hoá đầu ra.
    // `schtasks /XML` in UTF-8 trên máy đo được, nhưng UTF-16 cũng từng thấy
    // tuỳ phiên bản Windows. Marker thuần ASCII sống sót qua cả hai (và qua
    // cả mã trang OEM), nên chỉ cần thử hai cách đọc là đủ chắc.
    let text_utf8 = String::from_utf8_lossy(&out.stdout).into_owned();
    if text_utf8.contains(SCHEDULE_MARK) {
        return true;
    }
    let utf16: Vec<u16> = out
        .stdout
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&utf16).contains(SCHEDULE_MARK)
}

/// Nâng lịch lên phiên bản hiện hành nếu máy còn mang lịch cũ.
///
/// **Gọi từ tiến trình indexer (đã elevated).** Xoá-rồi-tạo-lại thay vì
/// /Change vì schtasks không sửa được Repetition qua cờ lệnh.
pub fn upgrade_schedule_if_stale() {
    if !scheduled_task_exists() || schedule_is_current() {
        return;
    }
    tracing::info!("lịch định kỳ là bản cũ — nâng lên lịch v2 (mỗi 15 phút)");
    match schtasks(&["/Delete", "/TN", TASK_NAME, "/F"]) {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            tracing::warn!(
                "không xoá được lịch cũ (mã {:?}): {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
            return;
        }
        Err(e) => {
            tracing::warn!("không gọi được schtasks: {e}");
            return;
        }
    }
    ensure_scheduled_task();
}

/// Register the refresh task, if it is not there already.
///
/// **Must be called from an elevated process.** Returns whether the task exists
/// when this returns — so `true` also covers "it was already there".
///
/// Failure is not fatal anywhere: without the task the app still works, it just
/// asks for permission on every scan. So this logs and moves on rather than
/// taking the scan down with it.
pub fn ensure_scheduled_task() -> bool {
    if scheduled_task_exists() {
        return true;
    }

    let Ok(exe) = std::env::current_exe() else {
        tracing::warn!("không xác định được đường dẫn chương trình, bỏ qua tác vụ định kỳ");
        return false;
    };
    let Some(user) = current_user() else {
        tracing::warn!("không xác định được tài khoản đang dùng, bỏ qua tác vụ định kỳ");
        return false;
    };

    let xml = TASK_XML
        .replace("{EXE}", &exe.to_string_lossy())
        .replace("{USER}", &user)
        .replace("{TASK}", TASK_NAME);

    // UTF-16 with a BOM: the XML header says so, and `schtasks` reads the
    // header rather than sniffing. A UTF-8 file here is rejected with a
    // singularly unhelpful "The task XML is malformed".
    let path = std::env::temp_dir().join("mediafinder-task.xml");
    let mut bytes = vec![0xFF, 0xFE];
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    if let Err(e) = std::fs::write(&path, &bytes) {
        tracing::warn!("không ghi được mô tả tác vụ: {e}");
        return false;
    }

    let created = match schtasks(&[
        "/Create",
        "/TN",
        TASK_NAME,
        "/XML",
        &path.to_string_lossy(),
        "/F",
    ]) {
        Ok(out) if out.status.success() => {
            tracing::info!("đã tạo tác vụ cập nhật định kỳ — từ giờ quét lại không hỏi quyền nữa");
            true
        }
        Ok(out) => {
            tracing::warn!(
                "không tạo được tác vụ định kỳ (mã {:?}): {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
            false
        }
        Err(e) => {
            tracing::warn!("không gọi được schtasks: {e}");
            false
        }
    };
    let _ = std::fs::remove_file(&path);
    created
}

/// `DOMAIN\user` for the account this process belongs to.
fn current_user() -> Option<String> {
    let domain = std::env::var("USERDOMAIN").ok()?;
    let name = std::env::var("USERNAME").ok()?;
    Some(format!("{domain}\\{name}"))
}

/// Where Windows keeps this user's startup items.
pub fn startup_dir() -> Option<PathBuf> {
    use windows::Win32::UI::Shell::{FOLDERID_Startup, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
    unsafe {
        let raw = SHGetKnownFolderPath(&FOLDERID_Startup, KF_FLAG_DEFAULT, None).ok()?;
        let path = raw.to_string().ok()?;
        windows::Win32::System::Com::CoTaskMemFree(Some(raw.0 as *const _));
        Some(PathBuf::from(path))
    }
}

/// Write the Startup shortcut, if it is not there already.
///
/// Needs no privilege — it is a file in the user's own profile.
///
/// `--minimized` is deliberate: starting a window at every login would be an
/// imposition. The app registers its hotkey and waits, which is the only reason
/// it needs to be running at all.
pub fn ensure_startup_shortcut() -> bool {
    let Some(dir) = startup_dir() else {
        tracing::warn!("không tìm được thư mục Startup");
        return false;
    };
    let link = dir.join(SHORTCUT_NAME);
    if link.exists() {
        return true;
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };

    match write_shortcut(&exe, &link) {
        Ok(()) => {
            tracing::info!("đã tạo lối tắt tự khởi động: {}", link.display());
            true
        }
        Err(e) => {
            tracing::warn!("không tạo được lối tắt tự khởi động: {e}");
            false
        }
    }
}

fn write_shortcut(exe: &std::path::Path, link: &std::path::Path) -> windows::core::Result<()> {
    use windows::core::{Interface, HSTRING, PCWSTR};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    unsafe {
        // Idempotent, and the GUI thread may or may not have done it already.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        let exe_w = HSTRING::from(exe.as_os_str());
        shell.SetPath(PCWSTR(exe_w.as_ptr()))?;
        shell.SetArguments(PCWSTR(HSTRING::from("--minimized").as_ptr()))?;
        shell.SetDescription(PCWSTR(
            HSTRING::from("MediaFinder — tìm media tức thì (Ctrl+Alt+Space)").as_ptr(),
        ))?;
        if let Some(dir) = exe.parent() {
            let dir_w = HSTRING::from(dir.as_os_str());
            shell.SetWorkingDirectory(PCWSTR(dir_w.as_ptr()))?;
        }

        let file: IPersistFile = shell.cast()?;
        let link_w = HSTRING::from(link.as_os_str());
        file.Save(PCWSTR(link_w.as_ptr()), true)?;
        Ok(())
    }
}

/// Delete the refresh task. Needs Administrator; returns whether it is gone.
fn delete_scheduled_task() -> bool {
    match schtasks(&["/Delete", "/TN", TASK_NAME, "/F"]) {
        Ok(out) if out.status.success() => {
            tracing::info!("đã xoá tác vụ cập nhật định kỳ");
            true
        }
        Ok(_) => false,
        Err(e) => {
            tracing::warn!("không gọi được schtasks: {e}");
            false
        }
    }
}

/// Ask for Administrator once, purely to delete the task, and wait.
///
/// The uninstaller runs unelevated, so without this the task survives the
/// application it launches: every login and every 13:00 Windows would start a
/// program that is no longer on disk. One machine could live with that; forty
/// leaves forty pieces of litter nobody knows to clean up.
///
/// A declined prompt is not an error. The uninstall carries on, and the
/// uninstall instructions name the one command that finishes the job.
fn delete_scheduled_task_elevated() -> bool {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let exe = HSTRING::from(exe.as_os_str());
    let verb = HSTRING::from("runas");
    let args = HSTRING::from("--remove-task");

    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(exe.as_ptr()),
            lpParameters: PCWSTR(args.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };
        if ShellExecuteExW(&mut info).is_err() {
            tracing::info!("người dùng từ chối quyền, để lại tác vụ định kỳ");
            return false;
        }
        if !info.hProcess.is_invalid() {
            use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};
            WaitForSingleObject(info.hProcess, INFINITE);
            let _ = windows::Win32::Foundation::CloseHandle(info.hProcess);
        }
    }
    !scheduled_task_exists()
}

/// Delete only the scheduled task. The elevated half of [`remove_setup`].
pub fn remove_task_only() {
    delete_scheduled_task();
}

/// Undo everything [`ensure_scheduled_task`] and [`ensure_startup_shortcut`]
/// created.
///
/// Run by the uninstaller. Both steps are attempted independently: removing the
/// task may fail without Administrator, and that must not stop the shortcut
/// from going away.
///
/// `may_prompt` is false for a silent uninstall, where a UAC dialog nobody is
/// watching would hang the whole thing.
pub fn remove_setup(may_prompt: bool) {
    if let Some(dir) = startup_dir() {
        let link = dir.join(SHORTCUT_NAME);
        match std::fs::remove_file(&link) {
            Ok(()) => tracing::info!("đã xoá lối tắt tự khởi động"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::warn!("không xoá được lối tắt tự khởi động: {e}"),
        }
    }

    if delete_scheduled_task() {
        return;
    }
    if !may_prompt {
        tracing::info!("gỡ cài đặt im lặng — để lại tác vụ định kỳ");
        return;
    }
    // Unelevated delete was refused, which is the ordinary case: the task was
    // registered with Administrator and takes Administrator to remove.
    if !delete_scheduled_task_elevated() {
        tracing::info!("tác vụ định kỳ vẫn còn — xem hướng dẫn gỡ cài đặt");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_task_description_is_filled_in_completely() {
        let xml = TASK_XML
            .replace("{EXE}", "C:\\App\\mediafinder.exe")
            .replace("{USER}", "MAY\\nguoidung")
            .replace("{TASK}", TASK_NAME);
        assert!(
            !xml.contains('{'),
            "còn chỗ trống chưa điền trong mô tả tác vụ:\n{xml}"
        );
        assert!(xml.contains("<Arguments>--index</Arguments>"));
        assert!(xml.contains("<RunLevel>HighestAvailable</RunLevel>"));
    }

    #[test]
    fn both_triggers_are_present() {
        // One without the other is a different product: logon-only misses the
        // whole working day, daily-only misses a machine that was off at 13:00.
        assert!(TASK_XML.contains("<LogonTrigger>"));
        assert!(TASK_XML.contains("<CalendarTrigger>"));
    }

    #[test]
    fn the_startup_folder_is_findable_on_this_machine() {
        let dir = startup_dir().expect("Windows luôn có thư mục Startup");
        assert!(dir.is_dir(), "{} phải là thư mục", dir.display());
    }
}

#[cfg(test)]
mod schedule_v2_tests {
    use super::*;

    /// XML và hằng số marker phải kể cùng một câu chuyện — ai đó sửa lịch
    /// trong XML mà quên marker (hoặc ngược lại) thì upgrade_schedule sẽ
    /// hoặc nâng cấp mãi mãi, hoặc không bao giờ nâng.
    #[test]
    fn lich_v2_marker_va_xml_khop_nhau() {
        assert!(
            TASK_XML.contains(SCHEDULE_MARK),
            "Description trong XML phai mang marker {SCHEDULE_MARK:?}"
        );
        assert!(
            TASK_XML.contains("<Interval>PT15M</Interval>"),
            "lich v2 la moi-15-phut; thieu Repetition thi marker dang noi doi"
        );
        assert!(
            TASK_XML.contains("<Repetition>") && TASK_XML.contains("StopAtDurationEnd"),
            "khoi Repetition phai du hinh hai"
        );
        // Bat bien dat gia nhat cua khoi nay, do bang may that moi ra: marker
        // phai THUAN ASCII. Mot chu co dau trong marker la ky tu nhieu byte,
        // va moi phep so tren dau ra schtasks deu truot — lich da nang van bi
        // coi la cu, indexer xoa-tao-lai task mai mai.
        assert!(
            SCHEDULE_MARK.is_ascii(),
            "marker phai thuan ASCII de song sot qua moi cach ma hoa dau ra"
        );
    }
}
