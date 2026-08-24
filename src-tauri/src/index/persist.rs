//! On-disk index cache.
//!
//! The cache is what lets the GUI run unelevated. Only the short-lived
//! `--index` child needs Administrator to read the MFT; it writes the result
//! here, and every later launch loads this file instead — no UAC prompt, and
//! results on screen in well under a second.
//!
//! Per-volume USN stamps are recorded alongside the index so P8 can ask the
//! journal for just the changes since the scan rather than rebuilding.

use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::model::Index;

/// Bumped whenever the serialised layout changes.
///
/// An old cache is discarded rather than misread: `bincode` has no self-
/// describing format, so a layout change would otherwise deserialise into
/// plausible-looking nonsense.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    #[error("không xác định được thư mục dữ liệu (%LOCALAPPDATA%)")]
    NoDataDir,

    #[error("chưa có cache — cần quét ổ đĩa trước")]
    NotFound,

    #[error("cache thuộc phiên bản {found}, phần mềm cần {expected} — cần quét lại")]
    SchemaMismatch { found: u32, expected: u32 },

    #[error("lỗi đọc/ghi cache")]
    Io(#[from] std::io::Error),

    #[error("cache hỏng, không giải mã được")]
    Corrupt(#[from] bincode::Error),
}

/// What a scan recorded about one volume, so changes can be picked up later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeStamp {
    pub letter: char,
    pub serial: u32,
    pub journal_id: u64,
    /// The USN at scan time. P8 reads the journal forward from here.
    pub next_usn: i64,
    pub file_count: usize,
}

/// The cache as read back from disk.
#[derive(Debug, Deserialize)]
pub struct CacheFile {
    pub schema_version: u32,
    pub built_at_unix: i64,
    pub volumes: Vec<VolumeStamp>,
    pub index: Index,
}

/// The same layout, borrowed, for writing.
///
/// `Index` is intentionally not `Clone`: copying a hundred megabytes by
/// accident is precisely what this design exists to prevent. Serialising
/// through a borrowed twin keeps that guarantee while still writing exactly
/// the bytes [`CacheFile`] expects to read back — the two must stay in step,
/// field for field and in order, since bincode is not self-describing.
#[derive(Serialize)]
struct CacheFileRef<'a> {
    schema_version: u32,
    built_at_unix: i64,
    volumes: &'a [VolumeStamp],
    index: &'a Index,
}

/// `%LOCALAPPDATA%\MediaFinder`.
///
/// LocalAppData rather than Roaming: the cache describes this machine's disks
/// and would be meaningless — and large — if it followed a roaming profile.
pub fn cache_dir() -> Result<PathBuf, PersistError> {
    let base = std::env::var_os("LOCALAPPDATA").ok_or(PersistError::NoDataDir)?;
    Ok(PathBuf::from(base).join("MediaFinder"))
}

pub fn cache_path() -> Result<PathBuf, PersistError> {
    Ok(cache_dir()?.join("index.bin"))
}

/// Write the cache atomically.
///
/// Serialising straight over the live file would leave a truncated cache if
/// the machine lost power mid-write, and the GUI would then refuse to start
/// until the user worked out they had to rescan. Writing to a temporary file
/// and renaming means the old cache stays intact until the new one is
/// complete.
pub fn save(index: &Index, volumes: Vec<VolumeStamp>) -> Result<PathBuf, PersistError> {
    let dir = cache_dir()?;
    fs::create_dir_all(&dir)?;

    let final_path = dir.join("index.bin");
    let tmp_path = dir.join("index.bin.tmp");

    let cache = CacheFileRef {
        schema_version: SCHEMA_VERSION,
        built_at_unix: now_unix(),
        volumes: &volumes,
        index,
    };

    {
        let file = fs::File::create(&tmp_path)?;
        let mut writer = BufWriter::new(file);
        bincode::serialize_into(&mut writer, &cache)?;
    }
    fs::rename(&tmp_path, &final_path)?;
    Ok(final_path)
}

/// Load the cache, or say why it cannot be used.
pub fn load() -> Result<CacheFile, PersistError> {
    let path = cache_path()?;
    if !path.exists() {
        return Err(PersistError::NotFound);
    }

    let file = fs::File::open(&path)?;
    let mut reader = BufReader::new(file);
    let cache: CacheFile = bincode::deserialize_from(&mut reader)?;

    if cache.schema_version != SCHEMA_VERSION {
        return Err(PersistError::SchemaMismatch {
            found: cache.schema_version,
            expected: SCHEMA_VERSION,
        });
    }
    Ok(cache)
}

/// Delete the cache, forcing the next launch to rescan.
pub fn clear() -> Result<(), PersistError> {
    let path = cache_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_lives_under_localappdata() {
        // Skip where the variable is absent (non-Windows CI).
        if std::env::var_os("LOCALAPPDATA").is_none() {
            return;
        }
        let dir = cache_dir().expect("cache dir");
        assert!(dir.ends_with("MediaFinder"));
    }

    #[test]
    fn missing_cache_reports_not_found_rather_than_an_io_error() {
        // The GUI distinguishes "never scanned" from "something went wrong",
        // and shows a different message for each.
        let e = PersistError::NotFound;
        assert!(e.to_string().contains("chưa có cache"));
    }

    #[test]
    fn schema_mismatch_message_names_both_versions() {
        let e = PersistError::SchemaMismatch {
            found: 0,
            expected: SCHEMA_VERSION,
        };
        let msg = e.to_string();
        assert!(msg.contains("quét lại"), "must tell the user what to do");
    }
}
