//! Phase 1: drive `FSCTL_ENUM_USN_DATA` and yield `RawRecord`s.
//!
//! `FSCTL_ENUM_USN_DATA` walks the MFT and hands back records Windows has
//! already parsed. Choosing it over reading `$MFT` raw avoids writing an NTFS
//! parser (fixup arrays, run lists, `$ATTRIBUTE_LIST`) and — the trap that is
//! easiest to miss — avoids DOS 8.3 short names, which a raw parser surfaces
//! as a second `$FILE_NAME` attribute and which would otherwise make every
//! file appear twice.
//!
//! What this phase can and cannot filter:
//!   * **Can**  filter by extension — the name is right there in the record.
//!   * **Cannot** filter by directory — a record carries only its parent's
//!     reference number, and that parent may not have been read yet. Directory
//!     exclusion belongs to phase 2 (`tree.rs`), after the whole tree exists.
//!
//! Every directory is therefore kept regardless of the media filter: the tree
//! cannot be reconstructed without them.

use std::ffi::c_void;

use windows::Win32::Foundation::ERROR_HANDLE_EOF;
use windows::Win32::System::Ioctl::{FSCTL_ENUM_USN_DATA, MFT_ENUM_DATA_V0};
use windows::Win32::System::IO::DeviceIoControl;

use super::volume::VolumeHandle;
use super::{NtfsError, RawRecord};
use crate::index::model::{kind_from_ext, MediaKind, MAX_EXT_LEN};

/// 1 MiB per call. Larger buffers mean fewer syscalls; beyond this the
/// returns flatten out while the allocation starts to matter.
const BUFFER_BYTES: usize = 1024 * 1024;

const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;

/// Report progress at most once per this many records. Emitting per record
/// would put millions of events through the IPC channel and freeze the WebView.
const PROGRESS_EVERY: u64 = 20_000;

/// Byte offsets inside `USN_RECORD_V2`. Read manually rather than through the
/// struct so a record straddling the tail of the buffer can never be
/// misinterpreted, and so alignment is never assumed.
mod rec {
    pub const RECORD_LENGTH: usize = 0;
    pub const MAJOR_VERSION: usize = 4;
    pub const FILE_REFERENCE_NUMBER: usize = 8;
    pub const PARENT_FILE_REFERENCE_NUMBER: usize = 16;
    pub const FILE_ATTRIBUTES: usize = 52;
    pub const FILE_NAME_LENGTH: usize = 56;
    pub const FILE_NAME_OFFSET: usize = 58;
    pub const MIN_SIZE: usize = 60;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ScanStats {
    /// Every record the MFT handed back.
    pub records_seen: u64,
    pub directories: u64,
    pub files_seen: u64,
    /// Files whose extension matched the media table.
    pub media_kept: u64,
    /// Records rejected because `MajorVersion != 2`.
    pub wrong_version: u64,
    /// Records too short to contain a `USN_RECORD_V2` header.
    pub malformed: u64,
}

/// Enumerate one volume.
///
/// `on_progress` is called periodically with the running record count. It is
/// deliberately throttled here rather than at the call site: emitting an event
/// per record would flood the IPC channel and freeze the WebView.
pub fn enumerate(
    vol: &VolumeHandle,
    mut on_progress: impl FnMut(u64),
) -> Result<(Vec<RawRecord>, ScanStats), NtfsError> {
    // Backed by u64 so the buffer base is 8-byte aligned, matching the
    // alignment NTFS gives each record (RecordLength is always a multiple of 8).
    let mut buffer: Vec<u64> = vec![0; BUFFER_BYTES / 8];
    let mut input = MFT_ENUM_DATA_V0 {
        StartFileReferenceNumber: 0,
        LowUsn: 0,
        HighUsn: i64::MAX,
    };

    let mut records = Vec::new();
    let mut stats = ScanStats::default();
    let mut last_reported = 0u64;

    loop {
        let mut returned = 0u32;
        let result = unsafe {
            DeviceIoControl(
                vol.raw(),
                FSCTL_ENUM_USN_DATA,
                Some(&input as *const _ as *const c_void),
                std::mem::size_of::<MFT_ENUM_DATA_V0>() as u32,
                Some(buffer.as_mut_ptr() as *mut c_void),
                BUFFER_BYTES as u32,
                Some(&mut returned),
                None,
            )
        };

        if let Err(e) = result {
            // EOF is how this FSCTL says "the MFT is exhausted" — the normal
            // way out of this loop, not a failure.
            if e.code().0 as u32 & 0xFFFF == ERROR_HANDLE_EOF.0 {
                break;
            }
            return Err(NtfsError::Enumerate {
                letter: vol.letter,
                source: e,
            });
        }

        // Fewer than 8 bytes means not even the continuation cursor came back.
        if (returned as usize) <= std::mem::size_of::<u64>() {
            break;
        }

        let bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const u8, returned as usize) };

        // The first 8 bytes are the cursor for the next call, not a record.
        input.StartFileReferenceNumber = u64::from_le_bytes(bytes[..8].try_into().unwrap());

        parse_buffer(&bytes[8..], &mut records, &mut stats);

        if stats.records_seen - last_reported >= PROGRESS_EVERY {
            on_progress(stats.records_seen);
            last_reported = stats.records_seen;
        }
    }

    // Final tally, unless the last in-loop report already covered it.
    if stats.records_seen != last_reported {
        on_progress(stats.records_seen);
    }
    Ok((records, stats))
}

/// Walk one buffer of back-to-back `USN_RECORD_V2` structures.
///
/// Split out from the FFI loop so it can be exercised by unit tests with a
/// hand-built buffer, no volume and no elevation required.
fn parse_buffer(mut bytes: &[u8], out: &mut Vec<RawRecord>, stats: &mut ScanStats) {
    while bytes.len() >= rec::MIN_SIZE {
        let len = u32::from_le_bytes(bytes[rec::RECORD_LENGTH..][..4].try_into().unwrap()) as usize;

        // A zero or oversized length would loop forever or read out of bounds.
        if len < rec::MIN_SIZE || len > bytes.len() {
            stats.malformed += 1;
            break;
        }
        let record = &bytes[..len];
        bytes = &bytes[len..];
        stats.records_seen += 1;

        let major = u16::from_le_bytes(record[rec::MAJOR_VERSION..][..2].try_into().unwrap());
        if major != 2 {
            stats.wrong_version += 1;
            continue;
        }

        let name_len =
            u16::from_le_bytes(record[rec::FILE_NAME_LENGTH..][..2].try_into().unwrap()) as usize;
        let name_off =
            u16::from_le_bytes(record[rec::FILE_NAME_OFFSET..][..2].try_into().unwrap()) as usize;
        if name_off + name_len > len || name_len == 0 {
            stats.malformed += 1;
            continue;
        }

        let name_bytes = &record[name_off..name_off + name_len];
        let name_u16: Vec<u16> = name_bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();

        let attrs = u32::from_le_bytes(record[rec::FILE_ATTRIBUTES..][..4].try_into().unwrap());
        let is_dir = attrs & FILE_ATTRIBUTE_DIRECTORY != 0;

        // Directories are always kept: without the full set of them the tree
        // cannot be rebuilt in phase 2, however few of them hold media.
        let kind = if is_dir {
            stats.directories += 1;
            None
        } else {
            stats.files_seen += 1;
            match classify_utf16(&name_u16) {
                Some(k) => {
                    stats.media_kept += 1;
                    Some(k)
                }
                // Not a media file: drop it now rather than carry millions of
                // names through phase 2.
                None => continue,
            }
        };

        let frn = u64::from_le_bytes(
            record[rec::FILE_REFERENCE_NUMBER..][..8]
                .try_into()
                .unwrap(),
        );
        let parent_frn = u64::from_le_bytes(
            record[rec::PARENT_FILE_REFERENCE_NUMBER..][..8]
                .try_into()
                .unwrap(),
        );

        out.push(RawRecord {
            frn,
            parent_frn,
            name: String::from_utf16_lossy(&name_u16),
            is_dir,
            kind,
        });
    }
}

/// Classify a UTF-16 filename by extension without allocating.
///
/// Runs once per file record — several million times per scan — so it works
/// straight off the UTF-16 slice and only the survivors are ever decoded to
/// `String`.
fn classify_utf16(name: &[u16]) -> Option<MediaKind> {
    let dot = name.iter().rposition(|&c| c == u16::from(b'.'))?;
    let ext = &name[dot + 1..];
    if ext.is_empty() || ext.len() > MAX_EXT_LEN {
        return None;
    }

    let mut buf = [0u8; MAX_EXT_LEN];
    for (slot, &c) in buf.iter_mut().zip(ext) {
        // Any non-ASCII byte rules out every extension in the table.
        if c > 0x7F {
            return None;
        }
        *slot = (c as u8).to_ascii_lowercase();
    }
    kind_from_ext(&buf[..ext.len()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn classifies_regardless_of_case() {
        assert_eq!(classify_utf16(&utf16("Movie.MKV")), Some(MediaKind::Video));
        assert_eq!(classify_utf16(&utf16("photo.JpEg")), Some(MediaKind::Image));
    }

    #[test]
    fn ignores_non_media_and_extensionless() {
        assert_eq!(classify_utf16(&utf16("setup.exe")), None);
        assert_eq!(classify_utf16(&utf16("Makefile")), None);
        assert_eq!(classify_utf16(&utf16("archive.tar.gz")), None);
    }

    #[test]
    fn uses_the_last_dot_only() {
        // A dot in the stem must not be mistaken for the extension separator.
        assert_eq!(
            classify_utf16(&utf16("S01.E02.1080p.mkv")),
            Some(MediaKind::Video)
        );
    }

    #[test]
    fn handles_vietnamese_names() {
        assert_eq!(
            classify_utf16(&utf16("Tiếng Việt - Đà Nẵng.mp4")),
            Some(MediaKind::Video)
        );
    }

    #[test]
    fn rejects_non_ascii_extension_without_panicking() {
        // A multi-byte char in the extension slot must bail cleanly, not index
        // past the stack buffer.
        assert_eq!(classify_utf16(&utf16("file.mữ")), None);
        assert_eq!(classify_utf16(&utf16("file.")), None);
    }

    /// Build one `USN_RECORD_V2` in the exact layout the kernel produces.
    fn make_record(frn: u64, parent: u64, name: &str, is_dir: bool) -> Vec<u8> {
        let name_u16: Vec<u16> = name.encode_utf16().collect();
        let name_bytes: Vec<u8> = name_u16.iter().flat_map(|c| c.to_le_bytes()).collect();
        let name_off = 60usize;
        // NTFS pads every record out to an 8-byte boundary.
        let len = (name_off + name_bytes.len()).div_ceil(8) * 8;

        let mut r = vec![0u8; len];
        r[rec::RECORD_LENGTH..][..4].copy_from_slice(&(len as u32).to_le_bytes());
        r[rec::MAJOR_VERSION..][..2].copy_from_slice(&2u16.to_le_bytes());
        r[rec::FILE_REFERENCE_NUMBER..][..8].copy_from_slice(&frn.to_le_bytes());
        r[rec::PARENT_FILE_REFERENCE_NUMBER..][..8].copy_from_slice(&parent.to_le_bytes());
        r[rec::FILE_ATTRIBUTES..][..4].copy_from_slice(
            &(if is_dir {
                FILE_ATTRIBUTE_DIRECTORY
            } else {
                0x80
            })
            .to_le_bytes(),
        );
        r[rec::FILE_NAME_LENGTH..][..2].copy_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        r[rec::FILE_NAME_OFFSET..][..2].copy_from_slice(&(name_off as u16).to_le_bytes());
        r[name_off..name_off + name_bytes.len()].copy_from_slice(&name_bytes);
        r
    }

    #[test]
    fn parses_a_stream_of_records() {
        let mut buf = Vec::new();
        buf.extend(make_record(10, 5, "Videos", true));
        buf.extend(make_record(11, 10, "holiday.mp4", false));
        buf.extend(make_record(12, 10, "notes.txt", false));
        buf.extend(make_record(13, 10, "song.flac", false));

        let mut out = Vec::new();
        let mut stats = ScanStats::default();
        parse_buffer(&buf, &mut out, &mut stats);

        assert_eq!(stats.records_seen, 4);
        assert_eq!(stats.directories, 1);
        assert_eq!(stats.files_seen, 3);
        assert_eq!(stats.media_kept, 2, "only the mp4 and flac are media");

        // The directory plus the two media files; notes.txt is dropped here.
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].name, "Videos");
        assert!(out[0].is_dir);
        assert_eq!(out[1].name, "holiday.mp4");
        assert_eq!(out[1].kind, Some(MediaKind::Video));
        assert_eq!(out[1].parent_frn, 10);
        assert_eq!(out[2].kind, Some(MediaKind::Audio));
    }

    #[test]
    fn stops_on_a_zero_length_record_instead_of_looping_forever() {
        let mut buf = make_record(10, 5, "Videos", true);
        buf.extend(vec![0u8; 64]); // RecordLength == 0

        let mut out = Vec::new();
        let mut stats = ScanStats::default();
        parse_buffer(&buf, &mut out, &mut stats);

        assert_eq!(stats.records_seen, 1);
        assert_eq!(stats.malformed, 1);
    }

    #[test]
    fn rejects_a_record_claiming_more_bytes_than_the_buffer_holds() {
        let mut r = make_record(10, 5, "Videos", true);
        r[rec::RECORD_LENGTH..][..4].copy_from_slice(&9999u32.to_le_bytes());

        let mut out = Vec::new();
        let mut stats = ScanStats::default();
        parse_buffer(&r, &mut out, &mut stats);

        assert!(out.is_empty(), "must not read past the buffer");
        assert_eq!(stats.malformed, 1);
    }

    #[test]
    fn skips_records_of_an_unexpected_version() {
        let mut r = make_record(10, 5, "Videos", true);
        r[rec::MAJOR_VERSION..][..2].copy_from_slice(&3u16.to_le_bytes());

        let mut out = Vec::new();
        let mut stats = ScanStats::default();
        parse_buffer(&r, &mut out, &mut stats);

        assert_eq!(stats.wrong_version, 1);
        assert!(out.is_empty());
    }
}
