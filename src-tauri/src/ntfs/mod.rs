//! NTFS volume access: enumerate records, then resolve them into paths.
//!
//! Split into two phases because MFT/USN records carry only a parent
//! *reference number*, never a path — a record's full path is unknowable until
//! every directory record has been read. Filtering by directory therefore
//! cannot happen during the read; only extension filtering can.

pub mod tree;
pub mod usn_enum;
pub mod volume;

use crate::index::model::MediaKind;

/// One record as produced by phase 1.
///
/// Deliberately free of any Win32 type. This is the seam that makes phase 2
/// (tree building and path resolution — where the tricky logic lives)
/// unit-testable on CI, with no NTFS volume and no Administrator rights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRecord {
    /// File Reference Number — this record's identity on the volume.
    pub frn: u64,
    /// FRN of the containing directory. The only link to the tree we get.
    pub parent_frn: u64,
    pub name: String,
    pub is_dir: bool,
    /// `Some` only for files whose extension matched the media table.
    /// Always `None` for directories, which are kept solely to build the tree.
    pub kind: Option<MediaKind>,
}

/// The record number of the NTFS root directory, always 5.
///
/// An FRN packs a 48-bit record number with a 16-bit sequence number, so the
/// raw FRN of the root is not simply `5` and must be masked before comparing.
pub const ROOT_RECORD_NUMBER: u64 = 5;

/// Strip the sequence number from a File Reference Number.
#[inline]
pub fn record_number(frn: u64) -> u64 {
    frn & 0x0000_FFFF_FFFF_FFFF
}

/// Everything that can go wrong reaching the filesystem.
///
/// Each variant carries the drive letter because a scan covers several volumes
/// and "access denied" is useless without knowing which one.
#[derive(Debug, thiserror::Error)]
pub enum NtfsError {
    #[error("ổ {letter}: dùng {filesystem}, không phải NTFS — không có MFT/USN để đọc")]
    NotNtfs { letter: char, filesystem: String },

    #[error("ổ {letter}: bị từ chối truy cập — cần chạy với quyền Administrator")]
    AccessDenied { letter: char },

    #[error("ổ {letter}: không mở được volume")]
    OpenVolume {
        letter: char,
        #[source]
        source: windows::core::Error,
    },

    #[error("ổ {letter}: lỗi khi đọc MFT qua FSCTL_ENUM_USN_DATA")]
    Enumerate {
        letter: char,
        #[source]
        source: windows::core::Error,
    },

    #[error("ổ {letter}: USN Journal không khả dụng (bật bằng: fsutil usn createjournal m=32000000 a=4000000 {letter}:)")]
    JournalUnavailable {
        letter: char,
        #[source]
        source: windows::core::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_sequence_number_off_the_frn() {
        // Root directory as it actually appears on a volume: record 5,
        // sequence 5 in the high 16 bits.
        assert_eq!(record_number(0x0005_0000_0000_0005), ROOT_RECORD_NUMBER);
        assert_eq!(record_number(5), ROOT_RECORD_NUMBER);
        assert_eq!(record_number(0xFFFF_0000_0000_002A), 42);
    }
}
