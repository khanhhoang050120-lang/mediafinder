//! NTFS volume access: enumerate records, then resolve them into paths.
//!
//! Split into two phases because MFT/USN records carry only a parent
//! *reference number*, never a path — a record's full path is unknowable until
//! every directory record has been read. Filtering by directory therefore
//! cannot happen during the read; only extension filtering can.

pub mod tree;
pub mod usn_enum;
pub mod volume;

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
}
