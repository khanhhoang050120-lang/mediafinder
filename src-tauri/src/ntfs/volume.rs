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

/// How a drive is attached, which decides whether the MFT is reachable at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeKind {
    /// An internal disk.
    Local,
    /// A USB stick, card reader, external disk.
    Removable,
    /// A mapped network drive — a NAS or a Windows share.
    ///
    /// Reported as NTFS by `GetVolumeInformationW`, because SMB passes on
    /// whatever filesystem the *server* is using. That is misleading here: the
    /// client sees an SMB session, not a volume, and `\\.\Z:` cannot be
    /// opened. There is no MFT and no USN journal to read on this side of the
    /// wire, however NTFS the far end may be.
    Network,
}

/// A drive, whether or not we can index it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Uppercase drive letter, e.g. `'C'`.
    pub letter: char,
    pub filesystem: String,
    pub label: String,
    pub serial: u32,
    pub kind: VolumeKind,
    /// Where a network drive points, e.g. `\\192.168.1.213\media`.
    pub remote: Option<String>,
}

impl VolumeInfo {
    pub fn is_ntfs(&self) -> bool {
        self.filesystem.eq_ignore_ascii_case("NTFS")
    }

    /// Can the MFT enumerator read this drive?
    ///
    /// Two separate reasons it might not be, and they need different words for
    /// the user: the wrong filesystem, or the right filesystem on the far side
    /// of a network.
    pub fn is_scannable(&self) -> bool {
        self.is_ntfs() && self.kind != VolumeKind::Network
    }

    /// Why this drive is being skipped, or `None` if it is not.
    pub fn skip_reason(&self) -> Option<String> {
        match (self.kind, self.is_ntfs()) {
            (VolumeKind::Network, _) => Some(format!(
                "ổ mạng ({}) — đọc MFT/USN chỉ làm được với đĩa gắn trực tiếp, \
                 qua SMB thì máy này không thấy MFT của máy chủ",
                self.remote.as_deref().unwrap_or("không rõ địa chỉ")
            )),
            (_, false) => Some(format!(
                "dùng {}, không phải NTFS nên không có MFT/USN",
                self.filesystem
            )),
            (_, true) => None,
        }
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

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;

/// List every drive that could plausibly hold a media library.
///
/// Includes drives that cannot be indexed — the wrong filesystem, or a network
/// share — because the caller reports those rather than silently ignoring
/// them. A drive that is missing from the results *and* missing from the
/// warnings gives the user nothing to go on.
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
        let kind = match drive_type {
            DRIVE_FIXED => VolumeKind::Local,
            DRIVE_REMOVABLE => VolumeKind::Removable,
            DRIVE_REMOTE => VolumeKind::Network,
            // CD-ROM, RAM disk, unknown. Nothing anyone keeps a library on.
            _ => continue,
        };

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
            kind,
            remote: if kind == VolumeKind::Network {
                remote_path(letter)
            } else {
                None
            },
        });
    }

    out
}

/// Where a mapped drive actually points, for a message the user can act on.
///
/// "Skipping Z:" is useless when three drives are mapped; "skipping Z:
/// (\\192.168.1.213\padoma 1)" names the machine to go and look at.
fn remote_path(letter: char) -> Option<String> {
    use windows::Win32::Foundation::NO_ERROR;
    use windows::Win32::NetworkManagement::WNet::WNetGetConnectionW;

    let local: Vec<u16> = format!("{letter}:")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut buf = [0u16; 260];
    let mut len = buf.len() as u32;
    let rc = unsafe {
        WNetGetConnectionW(
            PCWSTR(local.as_ptr()),
            Some(windows::core::PWSTR(buf.as_mut_ptr())),
            &mut len,
        )
    };
    if rc == NO_ERROR {
        Some(wide_to_string(&buf))
    } else {
        None
    }
}

/// Open a volume for raw read. Requires elevation.
pub fn open_volume(vol: &VolumeInfo) -> Result<VolumeHandle, NtfsError> {
    if vol.kind == VolumeKind::Network {
        return Err(NtfsError::NotNtfs {
            letter: vol.letter,
            filesystem: format!("ổ mạng {}", vol.remote.as_deref().unwrap_or("")),
        });
    }
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

    fn info(letter: char, fs: &str, kind: VolumeKind) -> VolumeInfo {
        VolumeInfo {
            letter,
            filesystem: fs.into(),
            label: String::new(),
            serial: 0,
            kind,
            remote: None,
        }
    }

    #[test]
    fn device_path_has_no_trailing_backslash() {
        // `\\.\C:\` opens the root directory; `\\.\C:` opens the volume itself.
        // Getting this wrong makes every FSCTL fail with a confusing error.
        let vol = info('C', "NTFS", VolumeKind::Local);
        let path = String::from_utf16_lossy(&vol.device_path());
        assert_eq!(path.trim_end_matches('\0'), r"\\.\C:");
    }

    #[test]
    fn a_network_drive_is_never_scannable_however_ntfs_it_claims_to_be() {
        // SMB reports whatever filesystem the server runs, so a NAS answers
        // "NTFS" and passes every filesystem check — while `\.\Z:` cannot be
        // opened at all. Trusting the filesystem name alone is what let three
        // network drives vanish without a word.
        let nas = info('Z', "NTFS", VolumeKind::Network);
        assert!(nas.is_ntfs());
        assert!(!nas.is_scannable());
        assert!(info('D', "NTFS", VolumeKind::Local).is_scannable());
    }

    #[test]
    fn every_skipped_drive_can_say_why_and_every_indexed_one_stays_quiet() {
        // The point of the type: a drive missing from the results *and* from
        // the warnings leaves the user nothing to go on.
        assert!(info('D', "NTFS", VolumeKind::Local).skip_reason().is_none());

        let fat = info('G', "FAT32", VolumeKind::Removable)
            .skip_reason()
            .expect("FAT32 phải nêu lý do");
        assert!(fat.contains("FAT32"), "{fat}");

        let mut nas = info('Z', "NTFS", VolumeKind::Network);
        nas.remote = Some(r"\192.168.1.213\padoma 1".into());
        let why = nas.skip_reason().expect("ổ mạng phải nêu lý do");
        assert!(why.contains("ổ mạng"), "{why}");
        // Naming the server matters when several drives are mapped.
        assert!(why.contains("192.168.1.213"), "{why}");
    }

    #[test]
    #[ignore = "in ra ổ đĩa thật của máy này; chạy với --ignored"]
    fn list_the_drives_on_this_machine() {
        for v in list_volumes() {
            println!(
                "  {}: {:<6} {:?}{}  -> {}",
                v.letter,
                v.filesystem,
                v.kind,
                v.remote
                    .as_deref()
                    .map(|r| format!(" [{r}]"))
                    .unwrap_or_default(),
                match v.skip_reason() {
                    Some(why) => format!("BỎ QUA: {why}"),
                    None => "quét".to_string(),
                }
            );
        }
    }

    #[test]
    fn recognises_ntfs_case_insensitively() {
        let mut vol = info('D', "ntfs", VolumeKind::Local);
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
