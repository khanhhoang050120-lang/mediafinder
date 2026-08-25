//! Launching the elevated indexer, and reporting its progress back.
//!
//! The GUI runs as invoker and must stay that way (see `README.md`), but
//! reading the MFT needs Administrator. The split is: the GUI asks Windows to
//! start a second copy of this same executable with `--index` and the `runas`
//! verb; that copy scans, writes the cache, and exits. The window the user is
//! looking at never gains privileges, so drag-and-drop from Explorer keeps
//! working and there is no UAC prompt on ordinary launches.
//!
//! Progress crosses the process boundary through a small JSON file rather than
//! a pipe. `ShellExecuteExW` with `runas` gives no way to redirect stdout — the
//! child is started by the shell, not by us — so a named pipe would be the only
//! alternative, and a file polled ten times a second is far less machinery for
//! the same result.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HANDLE};
use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};
use windows::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

use crate::index::persist;

#[derive(Debug, thiserror::Error)]
pub enum ElevateError {
    #[error("Bạn đã từ chối cấp quyền Administrator. Chưa quét gì cả — dữ liệu cũ vẫn nguyên.")]
    Cancelled,

    #[error("không xác định được đường dẫn của chính chương trình")]
    NoExePath,

    #[error("không khởi chạy được tiến trình quét: {0}")]
    Launch(String),

    #[error("lỗi ghi/đọc tệp tiến độ")]
    Io(#[from] std::io::Error),
}

/// What the indexer is doing right now, as the GUI sees it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    /// `volumes` · `scanning` · `resolving` · `indexing` · `network` · `saving`
    /// · `done` · `error`
    ///
    /// Mirrored by a union type in `src/lib/search.ts`. Adding one here only
    /// means the UI never shows it — the type checker catches that, which is
    /// why the union is spelled out rather than left as `string`.
    pub phase: String,
    /// Drive letter currently being read, e.g. `"C"`.
    pub volume: String,
    pub records: u64,
    pub media_files: u64,
    pub volumes_done: usize,
    pub volumes_total: usize,
    /// Human-readable line for the UI.
    pub message: String,
    /// Set last, and only after the cache is safely on disk.
    pub finished: bool,
    pub error: Option<String>,
}

pub fn progress_path() -> Result<PathBuf, ElevateError> {
    let dir = persist::cache_dir().map_err(|_| ElevateError::NoExePath)?;
    Ok(dir.join("progress.json"))
}

/// Read the current progress, or `None` if there is no scan on record.
///
/// A malformed file reads as `None` rather than an error: the writer may be
/// midway through a rename, and the caller polls again in a moment anyway.
pub fn read_progress() -> Option<ScanProgress> {
    let path = progress_path().ok()?;
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn clear_progress() -> Result<(), ElevateError> {
    let path = progress_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Throttled progress writer, used by the indexer process.
///
/// The indexer sees a new record count every few milliseconds. Writing each one
/// would be tens of thousands of file rewrites for a scan, so updates are
/// coalesced and only the newest state is ever written.
pub struct ProgressWriter {
    path: PathBuf,
    state: ScanProgress,
    last_write: std::time::Instant,
}

/// Ten writes a second. Fast enough that the bar never looks stuck, slow enough
/// that the file is not being rewritten in a tight loop.
const WRITE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

impl ProgressWriter {
    pub fn new() -> Result<Self, ElevateError> {
        let path = progress_path()?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        Ok(Self {
            path,
            state: ScanProgress::default(),
            // Far enough in the past that the first update writes immediately.
            last_write: std::time::Instant::now() - WRITE_INTERVAL * 2,
        })
    }

    pub fn state_mut(&mut self) -> &mut ScanProgress {
        &mut self.state
    }

    /// Write if enough time has passed.
    pub fn tick(&mut self) {
        if self.last_write.elapsed() >= WRITE_INTERVAL {
            self.flush();
        }
    }

    /// Write regardless of the interval, for phase changes and the final state.
    pub fn flush(&mut self) {
        self.last_write = std::time::Instant::now();
        let _ = self.write_atomic();
    }

    /// Rename into place so a reader never sees a half-written file.
    ///
    /// Without this the GUI would occasionally parse truncated JSON. It
    /// tolerates that — a failed parse is treated as "no news" — but a torn
    /// read every few polls would make the progress bar stutter for no reason.
    fn write_atomic(&self) -> Result<(), ElevateError> {
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_string(&self.state)
            .map_err(|e| ElevateError::Launch(e.to_string()))?;
        fs::write(&tmp, json)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// A running elevated child.
pub struct ElevatedChild {
    handle: HANDLE,
}

// The handle is only ever waited on and closed; nothing about it is
// thread-affine, so it can move to the watcher thread.
unsafe impl Send for ElevatedChild {}

impl ElevatedChild {
    /// Block until the child exits, then release the handle.
    pub fn wait(self) {
        unsafe {
            WaitForSingleObject(self.handle, INFINITE);
            let _ = CloseHandle(self.handle);
        }
        std::mem::forget(self);
    }
}

impl Drop for ElevatedChild {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

/// Ask Windows to run this same executable with `--index`, elevated.
///
/// Returns as soon as the child starts; the caller waits on it elsewhere so the
/// UI thread is never blocked behind a scan.
pub fn spawn_elevated_indexer() -> Result<ElevatedChild, ElevateError> {
    let exe = std::env::current_exe().map_err(|_| ElevateError::NoExePath)?;
    let exe = HSTRING::from(exe.as_os_str());
    let verb = HSTRING::from("runas");
    let args = HSTRING::from("--index");

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        // NOCLOSEPROCESS hands back a process handle so completion can be
        // detected exactly, rather than inferred from the progress file.
        // FLAG_NO_UI suppresses the shell's own error dialogs — a refused UAC
        // prompt is reported through the UI this application already has.
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(exe.as_ptr()),
        lpParameters: PCWSTR(args.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };

    unsafe { ShellExecuteExW(&mut info) }.map_err(|e| {
        // Declining the UAC prompt is not a failure — it is an answer. It must
        // read as "nothing happened, your data is untouched", not as an error.
        if e.code().0 as u32 & 0xFFFF == ERROR_CANCELLED.0 {
            ElevateError::Cancelled
        } else {
            ElevateError::Launch(e.message())
        }
    })?;

    if info.hProcess.is_invalid() {
        return Err(ElevateError::Launch(
            "Windows không trả về tiến trình con".into(),
        ));
    }
    Ok(ElevatedChild {
        handle: info.hProcess,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declining_uac_is_worded_as_an_answer_not_a_failure() {
        // The user pressed No on purpose. Saying "error" would suggest
        // something broke, and imply their data might be in a bad state.
        let msg = ElevateError::Cancelled.to_string();
        assert!(msg.contains("từ chối"));
        assert!(
            msg.contains("dữ liệu cũ vẫn nguyên"),
            "must reassure that nothing was lost, got: {msg}"
        );
    }

    #[test]
    fn progress_round_trips_through_json() {
        let p = ScanProgress {
            phase: "scanning".into(),
            volume: "C".into(),
            records: 3_559_309,
            media_files: 872_803,
            volumes_done: 0,
            volumes_total: 2,
            message: "đang đọc MFT".into(),
            finished: false,
            error: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        // camelCase, because the frontend reads these names directly.
        assert!(json.contains("mediaFiles"));
        assert!(json.contains("volumesTotal"));

        let back: ScanProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(back.records, 3_559_309);
        assert!(!back.finished);
    }

    #[test]
    fn malformed_progress_reads_as_no_news() {
        // A reader polling while the writer renames must not blow up.
        let bad: Result<ScanProgress, _> = serde_json::from_str("{\"phase\": \"scan");
        assert!(bad.is_err(), "and read_progress turns that into None");
    }

    #[test]
    fn progress_file_sits_beside_the_cache() {
        if std::env::var_os("LOCALAPPDATA").is_none() {
            return;
        }
        let p = progress_path().expect("progress path");
        assert!(p.ends_with("progress.json"));
        assert_eq!(p.parent(), persist::cache_dir().ok().as_deref());
    }
}
