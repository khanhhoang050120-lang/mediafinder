//! Naming a directory the index has never heard of.
//!
//! The index only ever contains directories that hold indexed media, because
//! that is the only way one gets added — `tree.rs` creates a directory entry
//! while resolving some file's parent chain, and never otherwise. So a folder
//! made last week that held nothing but documents is simply not in the table.
//!
//! Drop the first `.mp4` into it and the journal reports the new file with a
//! parent reference number that resolves to nothing. Without this module the
//! file is dropped and stays invisible until the next full scan (RISK-003).
//!
//! NTFS can answer the question directly: `OpenFileById` opens a file by its
//! reference number, and `GetFinalPathNameByHandleW` says where that handle
//! points. Both need the volume open for reading, so this only works inside
//! the elevated indexer — which is where updates are applied anyway.

use std::collections::HashMap;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::{
    FileIdType, GetFinalPathNameByHandleW, OpenFileById, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_ID_DESCRIPTOR, FILE_ID_DESCRIPTOR_0, FILE_NAME_NORMALIZED, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use super::tree::ResolveOptions;
use super::volume::VolumeHandle;
use crate::index::update::{DirAnswer, DirLookup};

/// Enough to read a name, and nothing more.
///
/// `FILE_READ_ATTRIBUTES` alone is what `GetFinalPathNameByHandleW` needs. Not
/// asking for read access matters: the directory may be in use, and a request
/// for more than is needed is a request that can be refused.
const FILE_READ_ATTRIBUTES: u32 = 0x0080;

/// Longer than any path Windows will hand back, `\\?\` prefix included.
const PATH_BUF: usize = 32_768;

/// Answers "where is directory *n* on volume *V*" by asking NTFS.
pub struct VolumeDirLookup<'a> {
    volumes: HashMap<u8, &'a VolumeHandle>,
    opts: ResolveOptions,
}

impl<'a> VolumeDirLookup<'a> {
    /// `volumes` are open volume handles, keyed by uppercase drive letter.
    pub fn new(volumes: HashMap<u8, &'a VolumeHandle>, opts: ResolveOptions) -> Self {
        Self { volumes, opts }
    }
}

impl DirLookup for VolumeDirLookup<'_> {
    fn path_of(&self, volume: u8, frn: u64) -> DirAnswer {
        let Some(handle) = self.volumes.get(&volume) else {
            return DirAnswer::Unknown;
        };

        let descriptor = FILE_ID_DESCRIPTOR {
            dwSize: std::mem::size_of::<FILE_ID_DESCRIPTOR>() as u32,
            Type: FileIdType,
            Anonymous: FILE_ID_DESCRIPTOR_0 { FileId: frn as i64 },
        };

        let opened = unsafe {
            OpenFileById(
                handle.raw(),
                &descriptor,
                FILE_READ_ATTRIBUTES,
                // Share everything. This is a read of a name on a live system;
                // refusing to share would make the answer depend on whether
                // someone happens to have the folder open.
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                // Without this a directory cannot be opened at all.
                FILE_FLAG_BACKUP_SEMANTICS,
            )
        };
        let Ok(file) = opened else {
            // Deleted between the journal record and now, or not openable.
            // Ordinary on a live volume, and nothing can be done about it.
            return DirAnswer::Unknown;
        };

        let mut buf = vec![0u16; PATH_BUF];
        let len = unsafe { GetFinalPathNameByHandleW(file, &mut buf, FILE_NAME_NORMALIZED) };
        unsafe { CloseHandle(file) }.ok();

        if len == 0 || len as usize >= buf.len() {
            return DirAnswer::Unknown;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let path = strip_extended_prefix(&path);

        // The whole point of the index is that it leaves certain trees alone.
        // A path that arrives by this route has skipped `tree.rs` entirely, so
        // it has to be filtered here or the exclusions would have a hole in
        // them exactly the width of this feature.
        if self.opts.excludes_path(path) {
            return DirAnswer::Excluded;
        }
        DirAnswer::Path(path.to_string())
    }
}

/// `GetFinalPathNameByHandleW` answers in `\\?\C:\...` form.
///
/// The index stores plain `C:\...`, and a mismatch would file the same folder
/// twice under two spellings.
fn strip_extended_prefix(path: &str) -> &str {
    path.strip_prefix(r"\\?\").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extended_prefix_is_removed_but_a_plain_path_is_left_alone() {
        assert_eq!(strip_extended_prefix(r"\\?\D:\Phim"), r"D:\Phim");
        assert_eq!(strip_extended_prefix(r"D:\Phim"), r"D:\Phim");
        // A UNC path keeps its own shape; it simply has no `\\?\` to remove.
        assert_eq!(
            strip_extended_prefix(r"\\?\UNC\server\share"),
            r"UNC\server\share"
        );
    }
}
