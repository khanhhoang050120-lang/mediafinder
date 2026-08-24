//! Enumerate NTFS volumes and open volume handles.
//!
//! Opening `\\.\C:` for read requires Administrator (or SeBackupPrivilege), so
//! everything here runs only inside the elevated `--index` child process.

use std::ffi::c_void;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW, FILE_ATTRIBUTE_NORMAL,
    FILE_GENERIC_READ, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{FSCTL_QUERY_USN_JOURNAL, USN_JOURNAL_DATA_V0};
use windows::Win32::System::IO::DeviceIoControl;

use super::NtfsError;

/// A fixed drive, whether or not we can index it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Uppercase drive letter, e.g. `'C'`.
    pub letter: char,
    pub filesystem: String,
    pub label: String,
    pub serial: u32,
}

impl VolumeInfo {
    pub fn is_ntfs(&self) -> bool {
        self.filesystem.eq_ignore_ascii_case("NTFS")
    }

    /// The `\\.\C:` form used to open the raw volume. Note there is no trailing
    /// backslash — that form opens the root *directory* instead of the volume.
    fn device_path(&self) -> Vec<u16> {
        let s = format!(r"\\.\{}:", self.letter);
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

/// An owned volume handle that closes itself on drop.
pub struct VolumeHandle {
    handle: HANDLE,
    pub letter: char,
}

impl VolumeHandle {
    pub fn raw(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for VolumeHandle {
    fn drop(&mut self) {
        // Nothing useful to do if this fails, and it must not panic in a Drop.
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOVABLE: u32 = 2;

/// List every fixed/removable drive with a recognised filesystem.
///
/// Returns non-NTFS volumes too: the caller reports them as skipped rather
/// than silently ignoring them, so the user knows why a USB stick's files are
/// missing from search results.
pub fn list_volumes() -> Vec<VolumeInfo> {
    let mask = unsafe { GetLogicalDrives() };
    let mut out = Vec::new();

    for bit in 0..26u32 {
        if mask & (1 << bit) == 0 {
            continue;
        }
        let letter = (b'A' + bit as u8) as char;
        let root: Vec<u16> = format!(r"{letter}:\")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let drive_type = unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) };
        if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE {
            continue;
        }

        let mut label = [0u16; 256];
        let mut fs = [0u16; 64];
        let mut serial = 0u32;

        let ok = unsafe {
            GetVolumeInformationW(
                PCWSTR(root.as_ptr()),
                Some(&mut label),
                Some(&mut serial),
                None,
                None,
                Some(&mut fs),
            )
        };
        if ok.is_err() {
            // Empty card reader, unmounted drive, etc. Not an error worth surfacing.
            continue;
        }

        out.push(VolumeInfo {
            letter,
            filesystem: wide_to_string(&fs),
            label: wide_to_string(&label),
            serial,
        });
    }

    out
}

/// Open a volume for raw read. Requires elevation.
pub fn open_volume(vol: &VolumeInfo) -> Result<VolumeHandle, NtfsError> {
    if !vol.is_ntfs() {
        return Err(NtfsError::NotNtfs {
            letter: vol.letter,
            filesystem: vol.filesystem.clone(),
        });
    }

    let path = vol.device_path();
    let handle = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };

    match handle {
        Ok(h) => Ok(VolumeHandle {
            handle: h,
            letter: vol.letter,
        }),
        Err(e) if e.code().0 as u32 & 0xFFFF == ERROR_ACCESS_DENIED.0 => {
            Err(NtfsError::AccessDenied { letter: vol.letter })
        }
        Err(e) => Err(NtfsError::OpenVolume {
            letter: vol.letter,
            source: e,
        }),
    }
}

/// Journal state, needed for the incremental updates added in P8.
#[derive(Debug, Clone, Copy)]
pub struct JournalInfo {
    pub journal_id: u64,
    pub next_usn: i64,
}

/// Query the USN journal.
///
/// The full scan does **not** need this: `FSCTL_ENUM_USN_DATA` walks the MFT
/// directly and works with the journal disabled. It is queried only so the
/// starting USN can be recorded for later incremental updates, and a failure
/// here is informational rather than fatal.
pub fn query_journal(vol: &VolumeHandle) -> Result<JournalInfo, NtfsError> {
    let mut data = USN_JOURNAL_DATA_V0::default();
    let mut returned = 0u32;

    unsafe {
        DeviceIoControl(
            vol.handle,
            FSCTL_QUERY_USN_JOURNAL,
            None,
            0,
            Some(&mut data as *mut _ as *mut c_void),
            std::mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
            Some(&mut returned),
            None,
        )
    }
    .map_err(|e| NtfsError::JournalUnavailable {
        letter: vol.letter,
        source: e,
    })?;

    Ok(JournalInfo {
        journal_id: data.UsnJournalID,
        next_usn: data.NextUsn,
    })
}

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_path_has_no_trailing_backslash() {
        // `\\.\C:\` opens the root directory; `\\.\C:` opens the volume itself.
        // Getting this wrong makes every FSCTL fail with a confusing error.
        let vol = VolumeInfo {
            letter: 'C',
            filesystem: "NTFS".into(),
            label: String::new(),
            serial: 0,
        };
        let path = String::from_utf16_lossy(&vol.device_path());
        assert_eq!(path.trim_end_matches('\0'), r"\\.\C:");
    }

    #[test]
    fn recognises_ntfs_case_insensitively() {
        let mut vol = VolumeInfo {
            letter: 'D',
            filesystem: "ntfs".into(),
            label: String::new(),
            serial: 0,
        };
        assert!(vol.is_ntfs());
        vol.filesystem = "exFAT".into();
        assert!(!vol.is_ntfs());
    }

    #[test]
    fn wide_to_string_stops_at_nul() {
        let buf: Vec<u16> = "NTFS\0garbage".encode_utf16().collect();
        assert_eq!(wide_to_string(&buf), "NTFS");
    }
}
