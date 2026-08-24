//! Reading the change journal, so the index can be updated without rescanning.
//!
//! `FSCTL_ENUM_USN_DATA` (see `usn_enum.rs`) walks the whole MFT and answers
//! "what is on this volume". `FSCTL_READ_USN_JOURNAL` reads the same records
//! from the change log instead, and answers "what happened since USN *n*".
//! Both hand back `USN_RECORD_V2`, so the record layout is shared.
//!
//! The two things that make this harder than it looks are not the reading:
//!
//! * The journal can be **deleted and recreated** — after which the stored
//!   position means nothing, and there is no error that says so plainly.
//!   `journal_id` is the only way to tell.
//! * The journal is a **fixed-size ring**. Fall far enough behind and the
//!   records covering the gap are simply gone. Windows reports that as
//!   `ERROR_JOURNAL_ENTRY_DELETED`, which is easy to treat as a failure when
//!   it actually means "rescan, you have missed too much".
//!
//! Both answers are the same: throw away the incremental path and do a full
//! scan. Getting them *wrong* is worse than a slow rescan — it would leave an
//! index quietly describing a volume that no longer looks like that.

use std::ffi::c_void;

use windows::Win32::Foundation::{
    ERROR_INVALID_PARAMETER, ERROR_JOURNAL_DELETE_IN_PROGRESS, ERROR_JOURNAL_ENTRY_DELETED,
    ERROR_JOURNAL_NOT_ACTIVE,
};
use windows::Win32::System::Ioctl::{FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0};
use windows::Win32::System::IO::DeviceIoControl;

use super::volume::VolumeHandle;
use super::NtfsError;
use crate::index::update::Change;

/// One megabyte, matching the full scan. Journal batches are usually far
/// smaller; this only caps how much a single call can return.
const BUFFER_BYTES: usize = 1024 * 1024;

const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;

/// Reasons, from `winioctl.h`. Only the four that change what an entry *is*.
mod reason {
    /// Not read by the logic — a created file is `Present` by the same rule as
    /// a written one — but kept so the set of reasons that matter is written
    /// down in one place, and used by the tests.
    #[allow(dead_code)]
    pub const FILE_CREATE: u32 = 0x0000_0100;
    pub const FILE_DELETE: u32 = 0x0000_0200;
    pub const RENAME_OLD_NAME: u32 = 0x0000_1000;
    pub const RENAME_NEW_NAME: u32 = 0x0000_2000;
}

/// Byte offsets inside `USN_RECORD_V2`.
///
/// The same record the full scan parses, plus the two fields only the journal
/// fills in. Read by offset rather than through a struct so a record split
/// across the end of the buffer can never be misread.
mod rec {
    pub const RECORD_LENGTH: usize = 0;
    pub const MAJOR_VERSION: usize = 4;
    pub const FILE_REFERENCE_NUMBER: usize = 8;
    pub const PARENT_FILE_REFERENCE_NUMBER: usize = 16;
    pub const TIMESTAMP: usize = 32;
    pub const REASON: usize = 40;
    pub const FILE_ATTRIBUTES: usize = 52;
    pub const FILE_NAME_LENGTH: usize = 56;
    pub const FILE_NAME_OFFSET: usize = 58;
    pub const MIN_SIZE: usize = 60;
}

/// Where reading left off, and which journal it left off in.
///
/// The pair is stored per volume in the cache (`VolumeStamp`) and is
/// meaningless split up: a USN from a journal that has since been recreated
/// points at an unrelated place in the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub journal_id: u64,
    pub next_usn: i64,
}

/// Why the incremental path cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restart {
    /// The journal was deleted and recreated; the stored position is not a
    /// position in *this* journal.
    JournalReplaced,
    /// The ring wrapped past the stored position while we were not looking.
    RecordsLost,
    /// The journal is switched off, or being deleted right now.
    JournalOff,
}

impl Restart {
    /// What to tell the user, who did not ask about journals.
    pub fn message(self, letter: char) -> String {
        match self {
            Restart::JournalReplaced => {
                format!("ổ {letter}: nhật ký thay đổi đã được tạo lại — cần quét lại toàn bộ")
            }
            Restart::RecordsLost => format!(
                "ổ {letter}: có quá nhiều thay đổi kể từ lần quét trước, nhật ký đã ghi đè mất \
                 phần cũ — cần quét lại toàn bộ"
            ),
            Restart::JournalOff => format!(
                "ổ {letter}: nhật ký thay đổi đang tắt (bật bằng: \
                 fsutil usn createjournal m=32000000 a=4000000 {letter}:)"
            ),
        }
    }
}

#[derive(Debug)]
pub enum Batch {
    /// Records read. `next` is where to resume; equal to the old position when
    /// nothing has happened.
    Changes { changes: Vec<Change>, next: Cursor },
    /// The incremental path is not usable and a full scan is required.
    Restart(Restart),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JournalStats {
    pub records_seen: usize,
    /// `RENAME_OLD_NAME` records, which describe a name that is about to stop
    /// existing and so are deliberately ignored.
    pub rename_halves: usize,
    pub malformed: usize,
    pub wrong_version: usize,
}

/// Read one batch of changes from `vol`, starting at `from`.
///
/// Returns as soon as the journal has no more records — it does not wait. A
/// caller wanting to follow the journal live calls this on a timer; blocking
/// inside `DeviceIoControl` would make the thread impossible to stop.
pub fn read_batch(vol: &VolumeHandle, letter: char, from: Cursor) -> Result<Batch, NtfsError> {
    // The journal has to be the one the cursor came from. Checked before
    // reading a single record, because `FSCTL_READ_USN_JOURNAL` with a wrong
    // id answers `ERROR_INVALID_PARAMETER` — a message that says nothing about
    // journals at all.
    let live = super::volume::query_journal(vol).map_err(|e| match e {
        NtfsError::JournalUnavailable { .. } => e,
        other => other,
    })?;
    if live.journal_id != from.journal_id {
        return Ok(Batch::Restart(Restart::JournalReplaced));
    }

    let mut buffer: Vec<u64> = vec![0; BUFFER_BYTES / 8];
    let mut changes = Vec::new();
    let mut stats = JournalStats::default();
    let mut usn = from.next_usn;

    loop {
        let input = READ_USN_JOURNAL_DATA_V0 {
            StartUsn: usn,
            // Everything. Filtering here would drop the `CLOSE`-only records
            // that are often the first sign a file finished being written.
            ReasonMask: u32::MAX,
            ReturnOnlyOnClose: 0,
            // Never block: this call must return whether or not anything has
            // happened, so the caller stays in control of its own timing.
            Timeout: 0,
            BytesToWaitFor: 0,
            UsnJournalID: from.journal_id,
        };

        let mut returned = 0u32;
        let result = unsafe {
            DeviceIoControl(
                vol.raw(),
                FSCTL_READ_USN_JOURNAL,
                Some(&input as *const _ as *const c_void),
                std::mem::size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                Some(buffer.as_mut_ptr() as *mut c_void),
                BUFFER_BYTES as u32,
                Some(&mut returned),
                None,
            )
        };

        if let Err(e) = result {
            return Ok(match restart_reason(&e) {
                Some(r) => Batch::Restart(r),
                None => {
                    return Err(NtfsError::Enumerate {
                        letter,
                        source: e,
                    })
                }
            });
        }

        // Every reply begins with the USN to resume from. A reply carrying
        // only that and no records means the journal is caught up.
        if (returned as usize) < 8 {
            break;
        }
        let bytes = unsafe {
            std::slice::from_raw_parts(buffer.as_ptr() as *const u8, returned as usize)
        };
        let next = i64::from_le_bytes(bytes[..8].try_into().unwrap());
        parse_buffer(&bytes[8..], letter, &mut changes, &mut stats);

        if next == usn {
            // No progress: nothing left to read. Guarding on this rather than
            // on the record count, because a buffer holding only records that
            // were all filtered out still advances the position.
            usn = next;
            break;
        }
        usn = next;
    }

    if stats.malformed > 0 || stats.wrong_version > 0 {
        tracing::warn!(
            "ổ {letter}: {} bản ghi journal hỏng, {} sai phiên bản",
            stats.malformed,
            stats.wrong_version
        );
    }

    Ok(Batch::Changes {
        changes,
        next: Cursor {
            journal_id: from.journal_id,
            next_usn: usn,
        },
    })
}

/// One deletion, as the journal recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deletion {
    pub name: String,
    /// The entry's own reference number.
    ///
    /// Needed to rebuild the tree afterwards: when a whole folder is deleted,
    /// its children name it as their parent, and it in turn names *its*
    /// parent — so the path can be walked back up out of the deleted records
    /// themselves, until it reaches a folder that still exists.
    pub frn: u64,
    pub parent_frn: u64,
    pub is_dir: bool,
    /// Windows FILETIME: 100-nanosecond ticks since 1601-01-01 UTC.
    pub filetime: i64,
}

/// Everything the journal still remembers about files that were deleted.
///
/// Separate from [`read_batch`] because it needs something that one throws
/// away: the **name**. A `Change::Gone` deliberately carries only a reference
/// number, since the index already knows what that entry was — but a person
/// asking "what happened to my files" needs the names back.
///
/// Reads from the oldest record the journal still holds, so how far back it
/// can see depends on the journal's size and how busy the volume has been.
pub fn audit_deletions(
    vol: &VolumeHandle,
    letter: char,
    media_only: bool,
) -> Result<(Vec<Deletion>, i64, i64), NtfsError> {
    use crate::index::model::classify_name;

    let live = super::volume::query_journal(vol)?;
    let mut buffer: Vec<u64> = vec![0; BUFFER_BYTES / 8];
    let mut out = Vec::new();
    // `FirstUsn` is the oldest record still in the ring.
    let mut usn = live.first_usn;
    let oldest = usn;

    loop {
        let input = READ_USN_JOURNAL_DATA_V0 {
            StartUsn: usn,
            ReasonMask: u32::MAX,
            ReturnOnlyOnClose: 0,
            Timeout: 0,
            BytesToWaitFor: 0,
            UsnJournalID: live.journal_id,
        };
        let mut returned = 0u32;
        let result = unsafe {
            DeviceIoControl(
                vol.raw(),
                FSCTL_READ_USN_JOURNAL,
                Some(&input as *const _ as *const c_void),
                std::mem::size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                Some(buffer.as_mut_ptr() as *mut c_void),
                BUFFER_BYTES as u32,
                Some(&mut returned),
                None,
            )
        };
        if let Err(e) = result {
            if restart_reason(&e).is_some() {
                break;
            }
            return Err(NtfsError::Enumerate { letter, source: e });
        }
        if (returned as usize) < 8 {
            break;
        }
        let bytes =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const u8, returned as usize) };
        let next = i64::from_le_bytes(bytes[..8].try_into().unwrap());
        collect_deletions(&bytes[8..], media_only, &mut out, classify_name);
        if next == usn {
            break;
        }
        usn = next;
    }

    Ok((out, oldest, usn))
}

fn collect_deletions(
    mut bytes: &[u8],
    media_only: bool,
    out: &mut Vec<Deletion>,
    is_media: impl Fn(&str) -> Option<crate::index::model::MediaKind>,
) {
    while bytes.len() >= rec::MIN_SIZE {
        let len = u32::from_le_bytes(bytes[rec::RECORD_LENGTH..][..4].try_into().unwrap()) as usize;
        if len < rec::MIN_SIZE || len > bytes.len() {
            break;
        }
        let record = &bytes[..len];
        bytes = &bytes[len..];

        if u16::from_le_bytes(record[rec::MAJOR_VERSION..][..2].try_into().unwrap()) != 2 {
            continue;
        }
        let why = u32::from_le_bytes(record[rec::REASON..][..4].try_into().unwrap());
        if why & reason::FILE_DELETE == 0 {
            continue;
        }
        let name_len =
            u16::from_le_bytes(record[rec::FILE_NAME_LENGTH..][..2].try_into().unwrap()) as usize;
        let name_off =
            u16::from_le_bytes(record[rec::FILE_NAME_OFFSET..][..2].try_into().unwrap()) as usize;
        if name_len == 0 || name_off + name_len > len {
            continue;
        }
        let name = String::from_utf16_lossy(
            &record[name_off..name_off + name_len]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<u16>>(),
        );
        let attrs = u32::from_le_bytes(record[rec::FILE_ATTRIBUTES..][..4].try_into().unwrap());
        let is_dir = attrs & FILE_ATTRIBUTE_DIRECTORY != 0;
        if media_only && !is_dir && is_media(&name).is_none() {
            continue;
        }
        out.push(Deletion {
            name,
            frn: u64::from_le_bytes(
                record[rec::FILE_REFERENCE_NUMBER..][..8].try_into().unwrap(),
            ),
            parent_frn: u64::from_le_bytes(
                record[rec::PARENT_FILE_REFERENCE_NUMBER..][..8]
                    .try_into()
                    .unwrap(),
            ),
            is_dir,
            filetime: i64::from_le_bytes(record[rec::TIMESTAMP..][..8].try_into().unwrap()),
        });
    }
}

/// Translate a Win32 error into "the incremental path is over", or `None` if
/// it is a real failure.
fn restart_reason(e: &windows::core::Error) -> Option<Restart> {
    let code = e.code().0 as u32 & 0xFFFF;
    if code == ERROR_JOURNAL_ENTRY_DELETED.0 {
        Some(Restart::RecordsLost)
    } else if code == ERROR_JOURNAL_NOT_ACTIVE.0 || code == ERROR_JOURNAL_DELETE_IN_PROGRESS.0 {
        Some(Restart::JournalOff)
    } else if code == ERROR_INVALID_PARAMETER.0 {
        // Reached despite the id check above if the journal is replaced
        // between the query and the read. Rare, and the answer is the same.
        Some(Restart::JournalReplaced)
    } else {
        None
    }
}

/// Turn one buffer of `USN_RECORD_V2` structures into changes.
///
/// Split from the FFI loop for the same reason `usn_enum::parse_buffer` is:
/// this is where the decisions live, and it can be exercised with a hand-built
/// buffer on any machine, with no volume and no elevation.
fn parse_buffer(mut bytes: &[u8], letter: char, out: &mut Vec<Change>, stats: &mut JournalStats) {
    let volume = (letter as u8).to_ascii_uppercase();

    while bytes.len() >= rec::MIN_SIZE {
        let len = u32::from_le_bytes(bytes[rec::RECORD_LENGTH..][..4].try_into().unwrap()) as usize;
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
        if name_len == 0 || name_off + name_len > len {
            stats.malformed += 1;
            continue;
        }

        let frn =
            u64::from_le_bytes(record[rec::FILE_REFERENCE_NUMBER..][..8].try_into().unwrap());
        let parent_frn = u64::from_le_bytes(
            record[rec::PARENT_FILE_REFERENCE_NUMBER..][..8]
                .try_into()
                .unwrap(),
        );
        let why = u32::from_le_bytes(record[rec::REASON..][..4].try_into().unwrap());
        let attrs = u32::from_le_bytes(record[rec::FILE_ATTRIBUTES..][..4].try_into().unwrap());

        if why & reason::FILE_DELETE != 0 {
            out.push(Change::Gone { volume, frn });
            continue;
        }

        // A rename produces two records: one naming what the entry used to be
        // called, one naming what it is called now. Only the second describes
        // the present, and if a batch ends between them the next batch starts
        // at the record after the first — so the second is never lost.
        if why & reason::RENAME_OLD_NAME != 0 && why & reason::RENAME_NEW_NAME == 0 {
            stats.rename_halves += 1;
            continue;
        }

        let name_bytes = &record[name_off..name_off + name_len];
        let name: String = String::from_utf16_lossy(
            &name_bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<u16>>(),
        );

        // Every other reason — created, written, closed, renamed into place —
        // says the same thing: this entry exists, here, under this name.
        //
        // Non-media names are kept rather than filtered out, and that is
        // deliberate: renaming `phim.mp4` to `phim.txt` produces a record for
        // a name the index does not want, and dropping it here would leave the
        // old entry in the index for ever. `rebuild_with` removes the old
        // entry and declines to add the new one, which is the right answer.
        out.push(Change::Present {
            volume,
            frn,
            parent_frn,
            name,
            is_dir: attrs & FILE_ATTRIBUTE_DIRECTORY != 0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one `USN_RECORD_V2` in the exact layout the kernel produces.
    fn record(frn: u64, parent: u64, name: &str, why: u32, is_dir: bool) -> Vec<u8> {
        let utf16: Vec<u16> = name.encode_utf16().collect();
        let name_bytes: Vec<u8> = utf16.iter().flat_map(|c| c.to_le_bytes()).collect();
        let len = (rec::MIN_SIZE + name_bytes.len()).div_ceil(8) * 8;

        let mut r = vec![0u8; len];
        r[rec::RECORD_LENGTH..][..4].copy_from_slice(&(len as u32).to_le_bytes());
        r[rec::MAJOR_VERSION..][..2].copy_from_slice(&2u16.to_le_bytes());
        r[rec::FILE_REFERENCE_NUMBER..][..8].copy_from_slice(&frn.to_le_bytes());
        r[rec::PARENT_FILE_REFERENCE_NUMBER..][..8].copy_from_slice(&parent.to_le_bytes());
        r[rec::REASON..][..4].copy_from_slice(&why.to_le_bytes());
        r[rec::FILE_ATTRIBUTES..][..4]
            .copy_from_slice(&if is_dir { FILE_ATTRIBUTE_DIRECTORY } else { 0 }.to_le_bytes());
        r[rec::FILE_NAME_LENGTH..][..2].copy_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        r[rec::FILE_NAME_OFFSET..][..2].copy_from_slice(&(rec::MIN_SIZE as u16).to_le_bytes());
        r[rec::MIN_SIZE..rec::MIN_SIZE + name_bytes.len()].copy_from_slice(&name_bytes);
        r
    }

    fn parse(records: &[Vec<u8>]) -> (Vec<Change>, JournalStats) {
        let buf: Vec<u8> = records.iter().flatten().copied().collect();
        let mut out = Vec::new();
        let mut stats = JournalStats::default();
        parse_buffer(&buf, 'd', &mut out, &mut stats);
        (out, stats)
    }

    #[test]
    fn a_created_file_becomes_present() {
        let (out, stats) = parse(&[record(100, 10, "phim.mp4", reason::FILE_CREATE, false)]);
        assert_eq!(stats.records_seen, 1);
        assert_eq!(
            out,
            vec![Change::Present {
                // Lowercase in, uppercase out: the index stores drive letters
                // uppercased, and a mismatch would make every lookup miss.
                volume: b'D',
                frn: 100,
                parent_frn: 10,
                name: "phim.mp4".to_string(),
                is_dir: false,
            }]
        );
    }

    #[test]
    fn a_deleted_file_becomes_gone() {
        let (out, _) = parse(&[record(100, 10, "phim.mp4", reason::FILE_DELETE, false)]);
        assert_eq!(
            out,
            vec![Change::Gone {
                volume: b'D',
                frn: 100
            }]
        );
    }

    #[test]
    fn only_the_new_half_of_a_rename_is_acted_on() {
        // Both halves carry the same reference number. Acting on the first
        // would file the entry under a name that no longer exists.
        let (out, stats) = parse(&[
            record(100, 10, "cũ.mp4", reason::RENAME_OLD_NAME, false),
            record(100, 10, "mới.mp4", reason::RENAME_NEW_NAME, false),
        ]);

        assert_eq!(stats.records_seen, 2);
        assert_eq!(stats.rename_halves, 1);
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0], Change::Present { name, .. } if name == "mới.mp4"));
    }

    #[test]
    fn a_rename_half_alone_at_the_end_of_a_batch_does_nothing() {
        // The batch can end between the two halves. Emitting the old name here
        // would rename the file backwards until the next batch corrected it.
        let (out, stats) = parse(&[record(100, 10, "cũ.mp4", reason::RENAME_OLD_NAME, false)]);
        assert!(out.is_empty());
        assert_eq!(stats.rename_halves, 1);
    }

    #[test]
    fn a_delete_wins_over_every_other_reason_in_the_same_record() {
        // The closing record of a deletion carries `FILE_DELETE | CLOSE`.
        const CLOSE: u32 = 0x8000_0000;
        let (out, _) = parse(&[record(
            100,
            10,
            "phim.mp4",
            reason::FILE_DELETE | CLOSE,
            false,
        )]);
        assert!(matches!(out[0], Change::Gone { .. }));
    }

    #[test]
    fn directories_are_marked_as_such() {
        let (out, _) = parse(&[record(20, 10, "Phim", reason::FILE_CREATE, true)]);
        assert!(matches!(&out[0], Change::Present { is_dir: true, .. }));
    }

    #[test]
    fn a_name_the_index_does_not_want_is_still_reported() {
        // Renaming `phim.mp4` to `phim.txt` must reach `rebuild_with`, which
        // removes the old entry. Filtering it out here would leave the index
        // claiming a file that no longer goes by that name.
        let (out, _) = parse(&[record(
            100,
            10,
            "phim.txt",
            reason::RENAME_NEW_NAME,
            false,
        )]);
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0], Change::Present { name, .. } if name == "phim.txt"));
    }

    #[test]
    fn several_records_in_one_buffer_are_all_read() {
        let (out, stats) = parse(&[
            record(100, 10, "a.mp4", reason::FILE_CREATE, false),
            record(101, 10, "b.mkv", reason::FILE_CREATE, false),
            record(102, 10, "c.mp3", reason::FILE_DELETE, false),
        ]);
        assert_eq!(stats.records_seen, 3);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn a_record_claiming_an_impossible_length_stops_the_walk() {
        // A zero length would loop forever; an oversized one would read past
        // the end of the buffer.
        let mut bad = record(100, 10, "a.mp4", reason::FILE_CREATE, false);
        bad[rec::RECORD_LENGTH..][..4].copy_from_slice(&0u32.to_le_bytes());
        let (out, stats) = parse(&[bad]);
        assert!(out.is_empty());
        assert_eq!(stats.malformed, 1);
    }

    #[test]
    fn a_name_running_past_the_end_of_its_record_is_rejected() {
        let mut bad = record(100, 10, "a.mp4", reason::FILE_CREATE, false);
        bad[rec::FILE_NAME_LENGTH..][..2].copy_from_slice(&9999u16.to_le_bytes());
        let (out, stats) = parse(&[bad]);
        assert!(out.is_empty());
        assert_eq!(stats.malformed, 1);
    }

    #[test]
    fn a_record_of_a_version_we_cannot_read_is_skipped_not_guessed_at() {
        let mut v3 = record(100, 10, "a.mp4", reason::FILE_CREATE, false);
        v3[rec::MAJOR_VERSION..][..2].copy_from_slice(&3u16.to_le_bytes());
        let (out, stats) = parse(&[v3]);
        assert!(out.is_empty());
        assert_eq!(stats.wrong_version, 1);
    }

    /// Build the error Windows actually returns for a Win32 code.
    fn win32(code: u32) -> windows::core::Error {
        windows::core::Error::from_hresult(windows::core::HRESULT::from_win32(code))
    }

    #[test]
    fn falling_behind_the_ring_is_a_reason_to_rescan_not_a_failure() {
        // `ERROR_JOURNAL_ENTRY_DELETED` reads like something broke. It means
        // the records covering the gap have been overwritten — the ordinary
        // outcome of leaving a machine off for a week. Treating it as a hard
        // error would turn "rescan and carry on" into "updates stop working".
        assert_eq!(
            restart_reason(&win32(ERROR_JOURNAL_ENTRY_DELETED.0)),
            Some(Restart::RecordsLost)
        );
    }

    #[test]
    fn a_journal_that_is_off_or_being_deleted_is_recognised() {
        assert_eq!(
            restart_reason(&win32(ERROR_JOURNAL_NOT_ACTIVE.0)),
            Some(Restart::JournalOff)
        );
        assert_eq!(
            restart_reason(&win32(ERROR_JOURNAL_DELETE_IN_PROGRESS.0)),
            Some(Restart::JournalOff)
        );
    }

    #[test]
    fn a_wrong_journal_id_is_reported_as_a_replaced_journal() {
        // Windows answers a mismatched `UsnJournalID` with a generic
        // "invalid parameter", which says nothing about journals at all.
        assert_eq!(
            restart_reason(&win32(ERROR_INVALID_PARAMETER.0)),
            Some(Restart::JournalReplaced)
        );
    }

    #[test]
    fn a_genuine_failure_is_not_swallowed_as_a_rescan() {
        // Access denied means the process is not elevated. Reporting that as
        // "please rescan" would send the user round a loop that cannot end.
        const ERROR_ACCESS_DENIED: u32 = 5;
        assert_eq!(restart_reason(&win32(ERROR_ACCESS_DENIED)), None);
    }

    #[test]
    fn every_restart_reason_says_which_drive_and_what_to_do() {
        for r in [
            Restart::JournalReplaced,
            Restart::RecordsLost,
            Restart::JournalOff,
        ] {
            let msg = r.message('D');
            assert!(msg.contains("ổ D"), "{msg}");
            assert!(
                msg.contains("quét lại") || msg.contains("fsutil"),
                "thông báo phải nói người dùng cần làm gì: {msg}"
            );
        }
    }

    #[test]
    fn a_batch_translates_into_changes_the_index_can_apply() {
        // End to end across the seam: journal bytes in, new index out.
        use crate::index::model::{IndexBuilder, MediaKind};
        use crate::index::update::rebuild_with;

        let mut b = IndexBuilder::new();
        let d = b.add_dir(r"D:\Phim", 10);
        b.add_file("cũ.mp4", MediaKind::Video, d, 100);
        let old = b.finish();

        let (changes, _) = parse(&[
            record(100, 10, "cũ.mp4", reason::RENAME_OLD_NAME, false),
            record(100, 10, "mới.mp4", reason::RENAME_NEW_NAME, false),
            record(101, 10, "thêm.mkv", reason::FILE_CREATE, false),
        ]);
        let (new, stats) = rebuild_with(&old, &changes);

        let paths: Vec<String> = (0..new.len()).map(|i| new.full_path(i)).collect();
        assert!(paths.contains(&r"D:\Phim\mới.mp4".to_string()), "{paths:?}");
        assert!(paths.contains(&r"D:\Phim\thêm.mkv".to_string()), "{paths:?}");
        assert!(!paths.contains(&r"D:\Phim\cũ.mp4".to_string()), "{paths:?}");
        assert_eq!(stats.files_moved, 1);
        assert_eq!(stats.files_added, 1);
    }
}
