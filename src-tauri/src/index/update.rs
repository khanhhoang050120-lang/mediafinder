//! Applying file-system changes to an index that cannot be modified.
//!
//! [`Index`] is immutable by design: filenames are `Span`s into one shared
//! arena, so there is nowhere to put a longer name and no way to reclaim the
//! bytes of a deleted one. That looks like it rules out incremental updates.
//!
//! It does not, because building a whole new index is cheap. Measured with
//! `cargo bench --bench search -- build`: 37,5 ms for 100 000 entries, 183 ms
//! for 500 000 — and that includes folding every name again. The 38 seconds a
//! rescan takes is *disk enumeration*, not this.
//!
//! So changes are collected, coalesced, and applied by reading the old index
//! and writing a new one. No overlay, no tombstones, no compaction pass, and
//! nothing in the rest of the program has to learn that entries can come from
//! two places.

use std::collections::HashMap;

use super::model::{classify_name, Index, IndexBuilder};
use crate::ntfs::{record_number, ROOT_RECORD_NUMBER};

/// A file reference number is unique per volume, never per machine, so
/// identity is always the pair.
type Key = (u8, u64);

/// Directories nested deeper than this are treated as a broken chain.
///
/// The same limit `tree.rs` uses. A cycle here would come from a corrupt
/// journal batch rather than a reparse point, but the failure mode — walking
/// forever — is identical.
const MAX_DEPTH: usize = 128;

/// What one entry looks like after a batch of journal records.
///
/// Deliberately a *final state* rather than an operation. A file created,
/// renamed twice and moved produces four journal records and one `Present`;
/// collapsing them before touching the index is what makes a batch cheap, and
/// it removes any question of applying operations in the right order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Nothing exists under this reference number any more.
    ///
    /// Whether it was a file or a directory is not stated: the index already
    /// knows which one it had, and a journal record for something never
    /// indexed is nothing to act on either way.
    Gone { volume: u8, frn: u64 },

    /// The entry exists, with this parent and this name.
    Present {
        volume: u8,
        frn: u64,
        parent_frn: u64,
        name: String,
        is_dir: bool,
    },
}

/// What the file system says about a directory the index has never heard of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirAnswer {
    /// It exists, it belongs in the index, and this is its absolute path.
    Path(String),
    /// It exists but is deliberately not indexed — under `Windows`, `AppData`,
    /// a dot-directory, and so on. Nothing is wrong; the file simply is not
    /// one this index tracks.
    Excluded,
    /// No such directory, or it could not be opened.
    Unknown,
}

/// Names a directory that the index does not already know.
///
/// The index only ever holds directories that contain indexed media, because
/// that is the only way one gets added. So a journal record saying "a new
/// `.mp4` appeared under directory 12345" can name a directory that exists on
/// disk and yet is nowhere in the table — the folder was made last week and is
/// only now getting its first video.
///
/// Without a way to ask the file system, that file is dropped and stays
/// invisible until the next full scan. This trait is that way to ask, kept
/// behind an interface so the rebuild itself stays free of Win32 and testable
/// on any machine.
pub trait DirLookup {
    fn path_of(&self, volume: u8, frn: u64) -> DirAnswer;
}

/// A lookup that never knows anything, for callers that have no volume open.
pub struct NoLookup;

impl DirLookup for NoLookup {
    fn path_of(&self, _volume: u8, _frn: u64) -> DirAnswer {
        DirAnswer::Unknown
    }
}

/// Not a reference number NTFS ever hands out, so it is safe as "no identity".
///
/// Record 0 is `$MFT` itself and always carries a sequence number in the high
/// bits; the root is record 5. A zero reaches here only from an index built
/// before FRNs existed, or from `Index::frn` answering for an entry that has
/// none — and matching on it would make one change collide with *every* such
/// entry at once.
const NO_FRN: u64 = 0;

impl Change {
    fn key(&self) -> Key {
        match *self {
            Change::Gone { volume, frn } => (volume, frn),
            Change::Present { volume, frn, .. } => (volume, frn),
        }
    }
}

/// What a batch actually did, for logging and for tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UpdateStats {
    pub files_added: usize,
    pub files_removed: usize,
    pub files_moved: usize,
    pub dirs_added: usize,
    pub dirs_removed: usize,
    pub dirs_renamed: usize,
    /// Changes whose parent directory is deliberately not indexed.
    ///
    /// Entirely expected: a file created under `C:\Windows` or inside
    /// `AppData` is reported by the journal like any other, and declining it
    /// is the correct outcome rather than a miss.
    pub excluded: usize,

    /// Changes whose parent directory could not be named at all.
    ///
    /// This is the one worth watching. Every such change is a file that exists
    /// on disk and is missing from the index until the next full scan.
    pub unresolved: usize,

    /// Directories named by asking the file system rather than by the index.
    pub dirs_looked_up: usize,
}

/// Build a new index from `old` with `changes` applied.
///
/// Entry positions are not preserved — they cannot be, since entries in the
/// middle disappear. Everything keyed by position must be rebuilt, which is
/// already how it works after a rescan: `epoch` invalidates thumbnail URLs and
/// enrichment re-seeds from its own path-keyed store.
pub fn rebuild_with(old: &Index, changes: &[Change]) -> (Index, UpdateStats) {
    rebuild_with_lookup(old, changes, &NoLookup)
}

/// The same, able to ask the file system about directories it does not know.
///
/// See [`DirLookup`]. Without one, the first media file dropped into a folder
/// that never held any is invisible until the next full scan.
pub fn rebuild_with_lookup(
    old: &Index,
    changes: &[Change],
    lookup: &dyn DirLookup,
) -> (Index, UpdateStats) {
    let mut stats = UpdateStats::default();

    // Last record for a given entry wins: it describes where the entry ended
    // up, and everything before it is history.
    let mut latest: HashMap<Key, &Change> = HashMap::with_capacity(changes.len());
    for c in changes {
        if c.key().1 == NO_FRN {
            continue;
        }
        latest.insert(c.key(), c);
    }

    let mut dirs = DirTable::from_index(old);
    dirs.apply(&latest, lookup, &mut stats);

    // Before the table is frozen into the builder: find homes for files whose
    // parent is not in it. Done here rather than while adding the files,
    // because `install` hands out the final directory ids and nothing can be
    // added after that.
    dirs.discover_parents(&latest, lookup, &mut stats);

    // Where each old entry lives now, so a moved file keeps the size and
    // timestamp already known about it.
    let mut old_pos: HashMap<Key, u32> = HashMap::with_capacity(old.len());
    for i in 0..old.len() {
        if old.frn(i) != NO_FRN {
            old_pos.insert((old.volume_of(i), old.frn(i)), i as u32);
        }
    }

    let mut b = IndexBuilder::new();
    b.reserve(dirs.live_count(), old.len() + latest.len());

    let remap = dirs.install(&mut b);

    let mut sizes: Vec<u64> = Vec::with_capacity(old.len());
    let mut mtimes: Vec<i64> = Vec::with_capacity(old.len());

    // Carry over everything the batch did not touch.
    for i in 0..old.len() {
        let key = (old.volume_of(i), old.frn(i));

        // An entry with no reference number cannot be the subject of any
        // change, and must never be mistaken for one that is.
        match latest.get(&key).filter(|_| key.1 != NO_FRN) {
            Some(Change::Gone { .. }) => {
                stats.files_removed += 1;
                continue;
            }
            // Re-added below, from its new name and parent.
            Some(Change::Present { .. }) => continue,
            None => {}
        }

        // The file itself was not mentioned, but the directory holding it may
        // have been deleted — in which case the file is gone too, and the
        // journal never says so about each file individually.
        let Some(dir_id) = remap[old.dir_ids()[i] as usize] else {
            stats.files_removed += 1;
            continue;
        };

        b.add_file(old.name(i), old.kind(i), dir_id, old.frn(i));
        sizes.push(old.size(i));
        mtimes.push(old.mtime(i));
    }

    // Then everything the batch created or moved.
    for c in latest.values() {
        let Change::Present {
            volume,
            frn,
            parent_frn,
            name,
            is_dir,
        } = c
        else {
            continue;
        };
        if *is_dir {
            continue;
        }
        let Some(kind) = classify_name(name) else {
            // Not media, or media the index does not track. Nothing to add,
            // and nothing wrong either.
            continue;
        };
        let Some(dir_id) = dirs.resolve(*volume, *parent_frn).and_then(|d| remap[d as usize]) else {
            if dirs.was_excluded(*volume, *parent_frn) {
                stats.excluded += 1;
            } else {
                stats.unresolved += 1;
            }
            continue;
        };

        let was_known = old_pos.get(&(*volume, *frn));
        if was_known.is_some() {
            stats.files_moved += 1;
        } else {
            stats.files_added += 1;
        }

        b.add_file(name, kind, dir_id, *frn);
        // A moved file is the same bytes at a new path, so what was already
        // measured about it still holds. A new one is left at zero for the
        // metadata pass to fill in — the journal does not carry a size.
        sizes.push(was_known.map_or(0, |&p| old.size(p as usize)));
        mtimes.push(was_known.map_or(0, |&p| old.mtime(p as usize)));
    }

    let mut ix = b.finish();
    ix.set_file_stats(sizes, mtimes);
    (ix, stats)
}

/// The directory table, mid-rewrite.
///
/// Paths are absolute strings rather than parent links, which turns out to be
/// the useful representation here: renaming a directory is a prefix rewrite of
/// everything under it, and deleting one is a prefix match. Neither needs the
/// tree that produced them.
struct DirTable {
    /// `None` once the directory has been deleted.
    paths: Vec<Option<String>>,
    frns: Vec<u64>,
    /// Volume of each entry, so lookups never confuse two drives.
    vols: Vec<u8>,
    by_frn: HashMap<Key, u32>,
    /// Directories the file system was asked about and would not name.
    ///
    /// The value says *why*: `true` for a directory that is deliberately not
    /// indexed, `false` for one that could not be named at all. Keeping the
    /// two apart is the whole point — the first is a correct outcome, the
    /// second is a file quietly missing from the index.
    declined: HashMap<Key, bool>,
}

impl DirTable {
    fn from_index(old: &Index) -> Self {
        let n = old.dir_count();
        let mut t = Self {
            paths: Vec::with_capacity(n),
            frns: Vec::with_capacity(n),
            vols: Vec::with_capacity(n),
            by_frn: HashMap::with_capacity(n),
            declined: HashMap::new(),
        };
        for i in 0..n {
            let vol = old.volume_of_dir(i);
            t.push(vol, old.dir_frn(i), old.dir_path(i).to_string());
        }
        t
    }

    fn push(&mut self, vol: u8, frn: u64, path: String) -> u32 {
        let id = self.paths.len() as u32;
        self.paths.push(Some(path));
        self.frns.push(frn);
        self.vols.push(vol);
        self.by_frn.insert((vol, Self::lookup_frn(frn)), id);
        id
    }

    /// The key a directory is filed under.
    ///
    /// The volume root is the one directory with no record of its own: NTFS
    /// reports it as record 5 with a sequence number in the high bits, and
    /// `tree.rs` files it under the bare record number. A journal record
    /// naming the root as a parent carries the full reference, so both have to
    /// arrive at the same key or every file in the root of a drive would look
    /// like an orphan.
    fn lookup_frn(frn: u64) -> u64 {
        if record_number(frn) == ROOT_RECORD_NUMBER {
            ROOT_RECORD_NUMBER
        } else {
            frn
        }
    }

    fn resolve(&self, vol: u8, frn: u64) -> Option<u32> {
        self.by_frn.get(&(vol, Self::lookup_frn(frn))).copied()
    }

    /// Was this directory left out on purpose, rather than simply not found?
    fn was_excluded(&self, vol: u8, frn: u64) -> bool {
        self.declined
            .get(&(vol, Self::lookup_frn(frn)))
            .copied()
            .unwrap_or(false)
    }

    /// Resolve, and if that fails ask the file system once.
    fn resolve_or_ask(&mut self, vol: u8, frn: u64, lookup: &dyn DirLookup) -> Option<u32> {
        if let Some(id) = self.resolve(vol, frn) {
            return Some(id);
        }
        let key = (vol, Self::lookup_frn(frn));
        if self.declined.contains_key(&key) {
            return None;
        }
        match lookup.path_of(vol, frn) {
            DirAnswer::Path(path) => Some(self.push(vol, frn, path)),
            // Remembered either way: the answer will not change within one
            // batch, and asking again means another file-system round trip.
            DirAnswer::Excluded => {
                self.declined.insert(key, true);
                None
            }
            DirAnswer::Unknown => {
                self.declined.insert(key, false);
                None
            }
        }
    }

    /// Give every file change a directory to land in, asking the file system
    /// for the ones the index has never seen.
    fn discover_parents(
        &mut self,
        latest: &HashMap<Key, &Change>,
        lookup: &dyn DirLookup,
        stats: &mut UpdateStats,
    ) {
        // Collected first so the borrow of `latest` ends before the table is
        // mutated, and deduplicated because one new folder usually arrives
        // holding many files at once.
        let mut wanted: Vec<Key> = latest
            .values()
            .filter_map(|c| match c {
                Change::Present {
                    volume,
                    parent_frn,
                    is_dir: false,
                    ..
                } => Some((*volume, Self::lookup_frn(*parent_frn))),
                _ => None,
            })
            .filter(|k| self.resolve(k.0, k.1).is_none())
            .collect();
        wanted.sort_unstable();
        wanted.dedup();

        for (vol, frn) in wanted {
            if self.resolve_or_ask(vol, frn, lookup).is_some() {
                stats.dirs_looked_up += 1;
            }
        }
    }

    fn live_count(&self) -> usize {
        self.paths.iter().filter(|p| p.is_some()).count()
    }

    fn apply(
        &mut self,
        latest: &HashMap<Key, &Change>,
        lookup: &dyn DirLookup,
        stats: &mut UpdateStats,
    ) {
        // Creations first, and in dependency order: a new directory's parent
        // may itself be new and appear later in the batch.
        let mut pending: HashMap<Key, (u64, &str)> = HashMap::new();
        for (&key, c) in latest {
            if let Change::Present {
                parent_frn,
                name,
                is_dir: true,
                ..
            } = c
            {
                if self.resolve(key.0, key.1).is_none() {
                    pending.insert(key, (*parent_frn, name.as_str()));
                }
            }
        }
        let keys: Vec<Key> = pending.keys().copied().collect();
        for key in keys {
            // A new folder's parent may be one the index never knew either —
            // a whole tree created under a directory that held nothing before.
            if let Some((parent_frn, _)) = pending.get(&key).copied() {
                let _ = self.resolve_or_ask(key.0, parent_frn, lookup);
            }
            if self.materialise(key, &pending, 0).is_none() {
                stats.unresolved += 1;
            } else {
                stats.dirs_added += 1;
            }
        }

        // Then renames and moves of directories that already existed.
        for (&key, c) in latest {
            let Change::Present {
                parent_frn,
                name,
                is_dir: true,
                ..
            } = c
            else {
                continue;
            };
            let Some(id) = self.resolve(key.0, key.1) else {
                continue;
            };
            let Some(parent) = self
                .resolve(key.0, *parent_frn)
                .and_then(|p| self.paths[p as usize].clone())
            else {
                continue;
            };
            let fresh = format!("{parent}\\{name}");
            let Some(current) = self.paths[id as usize].clone() else {
                continue;
            };
            if fresh == current {
                continue;
            }
            self.rewrite_prefix(&current, &fresh);
            stats.dirs_renamed += 1;
        }

        // Deletions last, so a directory deleted in the same batch that
        // renamed it is still found under whichever reference the records use.
        for (&key, c) in latest {
            if !matches!(c, Change::Gone { .. }) {
                continue;
            }
            let Some(id) = self.resolve(key.0, key.1) else {
                continue;
            };
            let Some(path) = self.paths[id as usize].clone() else {
                continue;
            };
            stats.dirs_removed += 1;
            self.paths[id as usize] = None;
            for i in 0..self.paths.len() {
                if self.vols[i] == key.0 && matches!(&self.paths[i], Some(p) if is_under(p, &path)) {
                    self.paths[i] = None;
                    stats.dirs_removed += 1;
                }
            }
        }
    }

    /// Create a pending directory, and its pending parents, on demand.
    fn materialise(
        &mut self,
        key: Key,
        pending: &HashMap<Key, (u64, &str)>,
        depth: usize,
    ) -> Option<u32> {
        if let Some(id) = self.resolve(key.0, key.1) {
            return Some(id);
        }
        if depth >= MAX_DEPTH {
            return None;
        }
        let &(parent_frn, name) = pending.get(&key)?;
        let parent_id = match self.resolve(key.0, parent_frn) {
            Some(id) => id,
            None => self.materialise((key.0, parent_frn), pending, depth + 1)?,
        };
        let parent = self.paths[parent_id as usize].clone()?;
        Some(self.push(key.0, key.1, format!("{parent}\\{name}")))
    }

    /// Move a directory and everything beneath it to a new path.
    fn rewrite_prefix(&mut self, from: &str, to: &str) {
        for path in self.paths.iter_mut().flatten() {
            if path.eq_ignore_ascii_case(from) {
                *path = to.to_string();
            } else if is_under(path, from) {
                *path = format!("{to}{}", &path[from.len()..]);
            }
        }
    }

    /// Add the surviving directories to `b`, returning old id → new id.
    fn install(&self, b: &mut IndexBuilder) -> Vec<Option<u32>> {
        let mut remap = vec![None; self.paths.len()];
        for (i, path) in self.paths.iter().enumerate() {
            if let Some(path) = path {
                remap[i] = Some(b.add_dir(path, self.frns[i]));
            }
        }
        remap
    }
}

/// Is `path` inside directory `prefix`?
///
/// The separator check is what stops `D:\Phim2` from being swept up by a
/// rename of `D:\Phim`.
fn is_under(path: &str, prefix: &str) -> bool {
    path.len() > prefix.len()
        && path.as_bytes()[prefix.len()] == b'\\'
        && path[..prefix.len()].eq_ignore_ascii_case(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::model::MediaKind;

    const ROOT: u64 = 0x0005_0000_0000_0005;

    /// A small library: `D:\Phim` with two films, `D:\Nhạc` with one song.
    fn library() -> Index {
        let mut b = IndexBuilder::new();
        let root = b.add_dir("D:", ROOT_RECORD_NUMBER);
        let phim = b.add_dir(r"D:\Phim", 10);
        let nhac = b.add_dir(r"D:\Nhạc", 11);
        b.add_file("root.mp4", MediaKind::Video, root, 99);
        b.add_file("avatar.mkv", MediaKind::Video, phim, 100);
        b.add_file("titanic.mkv", MediaKind::Video, phim, 101);
        b.add_file("bài hát.mp3", MediaKind::Audio, nhac, 102);
        let mut ix = b.finish();
        ix.set_file_stats(vec![10, 20, 30, 40], vec![1, 2, 3, 4]);
        ix
    }

    fn paths(ix: &Index) -> Vec<String> {
        (0..ix.len()).map(|i| ix.full_path(i)).collect()
    }

    fn present(frn: u64, parent: u64, name: &str, is_dir: bool) -> Change {
        Change::Present {
            volume: b'D',
            frn,
            parent_frn: parent,
            name: name.to_string(),
            is_dir,
        }
    }

    fn gone(frn: u64) -> Change {
        Change::Gone {
            volume: b'D',
            frn,
        }
    }

    #[test]
    fn no_changes_reproduces_the_same_library() {
        let old = library();
        let (new, stats) = rebuild_with(&old, &[]);

        assert_eq!(paths(&new), paths(&old));
        assert_eq!(new.dir_count(), old.dir_count());
        assert_eq!(stats, UpdateStats::default());
        // Sizes and timestamps must survive, or every rebuild would throw away
        // the fast metadata pass.
        assert_eq!(new.sizes(), old.sizes());
    }

    #[test]
    fn a_deleted_file_disappears_and_the_rest_stay() {
        let (new, stats) = rebuild_with(&library(), &[gone(100)]);

        assert!(!paths(&new).contains(&r"D:\Phim\avatar.mkv".to_string()));
        assert!(paths(&new).contains(&r"D:\Phim\titanic.mkv".to_string()));
        assert_eq!(new.len(), 3);
        assert_eq!(stats.files_removed, 1);
    }

    #[test]
    fn a_renamed_file_is_findable_under_its_new_name_only() {
        let (new, stats) = rebuild_with(&library(), &[present(100, 10, "AVATAR 2009.mkv", false)]);

        let p = paths(&new);
        assert!(p.contains(&r"D:\Phim\AVATAR 2009.mkv".to_string()));
        assert!(!p.contains(&r"D:\Phim\avatar.mkv".to_string()));
        assert_eq!(new.len(), 4, "đổi tên không được làm số mục thay đổi");
        assert_eq!(stats.files_moved, 1);
        assert_eq!(stats.files_added, 0);
    }

    #[test]
    fn a_moved_file_keeps_the_size_already_measured_for_it() {
        // Nothing about the bytes changed, so re-measuring would be wasted I/O
        // — and until it happened the file would show as 0 bytes.
        let (new, _) = rebuild_with(&library(), &[present(100, 11, "avatar.mkv", false)]);

        let i = (0..new.len())
            .find(|&i| new.full_path(i) == r"D:\Nhạc\avatar.mkv")
            .expect("tệp đã chuyển thư mục");
        assert_eq!(new.size(i), 20);
        assert_eq!(new.mtime(i), 2);
    }

    #[test]
    fn a_new_file_lands_in_an_existing_directory() {
        let (new, stats) = rebuild_with(&library(), &[present(200, 10, "dune.mp4", false)]);

        assert!(paths(&new).contains(&r"D:\Phim\dune.mp4".to_string()));
        assert_eq!(stats.files_added, 1);
        // No size yet: a journal record does not carry one.
        let i = (0..new.len())
            .find(|&i| new.full_path(i) == r"D:\Phim\dune.mp4")
            .unwrap();
        assert_eq!(new.size(i), 0);
    }

    #[test]
    fn a_file_in_the_root_of_a_drive_resolves_through_the_full_root_reference() {
        // The index files the root under bare record number 5; the journal
        // names it with its sequence number attached. Both have to land on the
        // same directory or every file dropped on `D:\` looks like an orphan.
        let (new, stats) = rebuild_with(&library(), &[present(201, ROOT, "new.mp4", false)]);

        assert!(paths(&new).contains(&r"D:\new.mp4".to_string()));
        assert_eq!(stats.unresolved, 0);
    }

    #[test]
    fn a_file_created_in_a_brand_new_directory_chain() {
        let changes = vec![
            present(20, 10, "2024", true),
            present(21, 20, "Q4", true),
            present(300, 21, "phim mới.mp4", false),
        ];
        let (new, stats) = rebuild_with(&library(), &changes);

        assert!(paths(&new).contains(&r"D:\Phim\2024\Q4\phim mới.mp4".to_string()));
        assert_eq!(stats.dirs_added, 2);
        assert_eq!(stats.files_added, 1);
    }

    #[test]
    fn new_directories_resolve_whatever_order_they_arrive_in() {
        // A `HashMap` hands them back in no particular order, so the child may
        // well be seen before its parent exists.
        let changes = vec![
            present(300, 21, "phim mới.mp4", false),
            present(21, 20, "Q4", true),
            present(20, 10, "2024", true),
        ];
        let (new, _) = rebuild_with(&library(), &changes);
        assert!(paths(&new).contains(&r"D:\Phim\2024\Q4\phim mới.mp4".to_string()));
    }

    #[test]
    fn renaming_a_directory_moves_everything_underneath_it() {
        let mut b = IndexBuilder::new();
        let _root = b.add_dir("D:", ROOT_RECORD_NUMBER);
        let phim = b.add_dir(r"D:\Phim", 10);
        let sub = b.add_dir(r"D:\Phim\2024", 12);
        b.add_file("a.mkv", MediaKind::Video, phim, 100);
        b.add_file("b.mkv", MediaKind::Video, sub, 101);
        let old = b.finish();

        let (new, stats) = rebuild_with(&old, &[present(10, ROOT, "Movies", true)]);

        let p = paths(&new);
        assert!(p.contains(&r"D:\Movies\a.mkv".to_string()));
        assert!(
            p.contains(&r"D:\Movies\2024\b.mkv".to_string()),
            "thư mục con phải đi theo: {p:?}"
        );
        assert_eq!(stats.dirs_renamed, 1);
        assert_eq!(stats.files_moved, 0, "không tệp nào được nhắc tên");
    }

    #[test]
    fn renaming_a_directory_leaves_its_similarly_named_sibling_alone() {
        // `D:\Phim2` starts with `D:\Phim` as a string but is not inside it.
        // Without the separator check every rename would drag in its siblings.
        let mut b = IndexBuilder::new();
        let _root = b.add_dir("D:", ROOT_RECORD_NUMBER);
        let a = b.add_dir(r"D:\Phim", 10);
        let c = b.add_dir(r"D:\Phim2", 13);
        b.add_file("a.mkv", MediaKind::Video, a, 100);
        b.add_file("c.mkv", MediaKind::Video, c, 102);
        let old = b.finish();

        let (new, _) = rebuild_with(&old, &[present(10, ROOT, "Movies", true)]);

        let p = paths(&new);
        assert!(p.contains(&r"D:\Movies\a.mkv".to_string()));
        assert!(p.contains(&r"D:\Phim2\c.mkv".to_string()), "{p:?}");
    }

    #[test]
    fn deleting_a_directory_takes_its_files_with_it() {
        // The journal reports the directory going away; it does not repeat
        // itself for each file that was inside.
        let (new, stats) = rebuild_with(&library(), &[gone(10)]);

        let p = paths(&new);
        assert!(!p.iter().any(|s| s.starts_with(r"D:\Phim")), "{p:?}");
        assert!(p.contains(&r"D:\Nhạc\bài hát.mp3".to_string()));
        assert_eq!(stats.files_removed, 2);
        assert_eq!(stats.dirs_removed, 1);
    }

    #[test]
    fn deleting_a_directory_takes_its_subdirectories_too() {
        let mut b = IndexBuilder::new();
        let _root = b.add_dir("D:", ROOT_RECORD_NUMBER);
        let phim = b.add_dir(r"D:\Phim", 10);
        let sub = b.add_dir(r"D:\Phim\2024", 12);
        b.add_file("a.mkv", MediaKind::Video, phim, 100);
        b.add_file("b.mkv", MediaKind::Video, sub, 101);
        let old = b.finish();

        let (new, _) = rebuild_with(&old, &[gone(10)]);
        assert_eq!(new.len(), 0, "{:?}", paths(&new));
    }

    #[test]
    fn an_frn_shared_across_two_volumes_only_affects_its_own() {
        let mut b = IndexBuilder::new();
        let c = b.add_dir(r"C:\Media", 10);
        let d = b.add_dir(r"D:\Media", 10);
        b.add_file("same.mp4", MediaKind::Video, c, 100);
        b.add_file("same.mp4", MediaKind::Video, d, 100);
        let old = b.finish();

        // Reference numbers restart on every volume, so this pair is a normal
        // state of affairs, not a corrupt index.
        let (new, _) = rebuild_with(&old, &[gone(100)]);

        let p = paths(&new);
        assert_eq!(p, vec![r"C:\Media\same.mp4".to_string()], "{p:?}");
    }

    #[test]
    fn a_change_under_a_directory_the_index_never_knew_is_ignored() {
        // Files appear under `C:\Windows` constantly. They were excluded at
        // scan time, so their parent is not in the table and there is nothing
        // to do — but it must not drop the batch or panic.
        let (new, stats) = rebuild_with(&library(), &[present(400, 9999, "x.mp4", false)]);

        assert_eq!(new.len(), 4);
        assert_eq!(stats.unresolved, 1);
        assert_eq!(stats.files_added, 0);
    }

    /// A stand-in for the file system: knows a few directories by reference
    /// number, refuses one on purpose, and knows nothing of the rest.
    struct FakeFs {
        known: Vec<(u8, u64, &'static str)>,
        excluded: Vec<(u8, u64)>,
    }

    impl DirLookup for FakeFs {
        fn path_of(&self, volume: u8, frn: u64) -> DirAnswer {
            if self.excluded.iter().any(|&(v, f)| v == volume && f == frn) {
                return DirAnswer::Excluded;
            }
            match self
                .known
                .iter()
                .find(|&&(v, f, _)| v == volume && f == frn)
            {
                Some(&(_, _, path)) => DirAnswer::Path(path.to_string()),
                None => DirAnswer::Unknown,
            }
        }
    }

    #[test]
    fn the_first_media_file_in_an_old_empty_folder_is_found_by_asking() {
        // RISK-003. The folder was made last week and held nothing the index
        // wanted, so it is not in the table. Today it gets its first video.
        // Before this, that file stayed invisible until the next full scan.
        let fs = FakeFs {
            known: vec![(b'D', 77, r"D:\Phim\Chưa dùng")],
            excluded: vec![],
        };
        let changes = vec![present(700, 77, "phim đầu tiên.mp4", false)];

        // Without a lookup: dropped, and counted as a miss rather than as a
        // deliberate exclusion.
        let (before, s1) = rebuild_with(&library(), &changes);
        assert_eq!(before.len(), 4);
        assert_eq!(s1.unresolved, 1);
        assert_eq!(s1.excluded, 0);

        // With one: found.
        let (after, s2) = rebuild_with_lookup(&library(), &changes, &fs);
        assert!(paths(&after).contains(&r"D:\Phim\Chưa dùng\phim đầu tiên.mp4".to_string()));
        assert_eq!(s2.files_added, 1);
        assert_eq!(s2.dirs_looked_up, 1);
        assert_eq!(s2.unresolved, 0);
    }

    #[test]
    fn a_folder_the_file_system_declines_counts_as_excluded_not_as_a_miss() {
        // `C:\Windows\Temp` produces journal records constantly. Declining
        // them is the correct answer, and it must not look like the index is
        // losing files — otherwise the one number worth watching is buried.
        let fs = FakeFs {
            known: vec![],
            excluded: vec![(b'D', 88)],
        };
        let (new, stats) = rebuild_with_lookup(
            &library(),
            &[present(701, 88, "rác.mp4", false)],
            &fs,
        );

        assert_eq!(new.len(), 4);
        assert_eq!(stats.excluded, 1);
        assert_eq!(stats.unresolved, 0, "bị loại có chủ ý không phải là bỏ sót");
    }

    #[test]
    fn a_folder_that_cannot_be_named_at_all_still_counts_as_a_miss() {
        let fs = FakeFs {
            known: vec![],
            excluded: vec![],
        };
        let (_, stats) =
            rebuild_with_lookup(&library(), &[present(702, 99, "x.mp4", false)], &fs);
        assert_eq!(stats.unresolved, 1);
        assert_eq!(stats.excluded, 0);
    }

    #[test]
    fn one_new_folder_holding_many_files_is_asked_about_once() {
        // A folder usually turns up holding a whole batch of files. Asking the
        // file system once per file would be one handle open per file.
        struct Counting {
            asked: std::cell::Cell<usize>,
        }
        impl DirLookup for Counting {
            fn path_of(&self, _volume: u8, _frn: u64) -> DirAnswer {
                self.asked.set(self.asked.get() + 1);
                DirAnswer::Path(r"D:\Phim\Mới".to_string())
            }
        }

        let fs = Counting {
            asked: std::cell::Cell::new(0),
        };
        let changes: Vec<Change> = (0..50)
            .map(|i| present(800 + i, 77, &format!("tệp {i}.mp4"), false))
            .collect();

        let (new, stats) = rebuild_with_lookup(&library(), &changes, &fs);

        assert_eq!(stats.files_added, 50);
        assert_eq!(fs.asked.get(), 1, "50 tệp cùng một thư mục = 1 lần hỏi");
        assert_eq!(new.dir_count(), 4, "chỉ thêm đúng một thư mục");
    }

    #[test]
    fn a_lookup_is_not_consulted_for_folders_the_index_already_knows() {
        // Every avoidable question is a file-system round trip, and the index
        // is right there.
        struct Forbidden;
        impl DirLookup for Forbidden {
            fn path_of(&self, _volume: u8, _frn: u64) -> DirAnswer {
                panic!("không được hỏi hệ thống tệp về thư mục đã có trong index");
            }
        }
        let (new, _) = rebuild_with_lookup(
            &library(),
            &[present(900, 10, "thêm.mp4", false)],
            &Forbidden,
        );
        assert!(paths(&new).contains(&r"D:\Phim\thêm.mp4".to_string()));
    }

    #[test]
    fn a_new_folder_under_an_unknown_folder_resolves_through_both() {
        // The journal describes the new subfolder, and the file system
        // supplies the parent the index never had.
        let fs = FakeFs {
            known: vec![(b'D', 77, r"D:\Kho")],
            excluded: vec![],
        };
        let changes = vec![
            present(78, 77, "2026", true),
            present(901, 78, "clip.mp4", false),
        ];
        let (new, stats) = rebuild_with_lookup(&library(), &changes, &fs);

        assert!(
            paths(&new).contains(&r"D:\Kho\2026\clip.mp4".to_string()),
            "{:?}",
            paths(&new)
        );
        assert_eq!(stats.files_added, 1);
    }

    #[test]
    fn a_non_media_file_is_not_added() {
        let (new, stats) = rebuild_with(&library(), &[present(401, 10, "notes.txt", false)]);
        assert_eq!(new.len(), 4);
        assert_eq!(stats.files_added, 0);
    }

    #[test]
    fn the_last_record_for_an_entry_is_the_one_that_counts() {
        // A file created then renamed twice within one batch is one entry at
        // its final name, not three.
        let changes = vec![
            present(500, 10, "tmp.mp4", false),
            present(500, 10, "draft.mp4", false),
            present(500, 10, "final.mp4", false),
        ];
        let (new, stats) = rebuild_with(&library(), &changes);

        let p = paths(&new);
        assert!(p.contains(&r"D:\Phim\final.mp4".to_string()));
        assert!(!p.contains(&r"D:\Phim\tmp.mp4".to_string()));
        assert_eq!(stats.files_added, 1);
    }

    #[test]
    fn a_file_created_and_deleted_within_one_batch_never_appears() {
        let changes = vec![present(501, 10, "temp.mp4", false), gone(501)];
        let (new, stats) = rebuild_with(&library(), &changes);

        assert_eq!(new.len(), 4);
        assert_eq!(stats.files_added, 0);
    }

    #[test]
    fn zero_is_not_an_identity_and_matches_nothing() {
        // Found by a benchmark, not by a test: every entry in the synthetic
        // index had been given FRN 0, so one `Gone { frn: 0 }` deleted all
        // 500 000 of them — and the benchmark reported applying a hundred
        // changes as seven times *faster* than applying none, because there
        // was almost nothing left to rebuild.
        //
        // In production every entry has a real reference number, so this can
        // only arise from an index built before FRNs existed. The damage if it
        // did would be total and silent.
        let mut b = IndexBuilder::new();
        let d = b.add_dir(r"D:\Phim", 10);
        b.add_file("a.mkv", MediaKind::Video, d, 0);
        b.add_file("b.mkv", MediaKind::Video, d, 0);
        b.add_file("c.mkv", MediaKind::Video, d, 500);
        let old = b.finish();

        let (new, stats) = rebuild_with(&old, &[gone(0)]);
        assert_eq!(new.len(), 3, "không mục nào được phép biến mất");
        assert_eq!(stats.files_removed, 0);

        // And a real reference number still works on the same index.
        let (new, _) = rebuild_with(&old, &[gone(500)]);
        assert_eq!(new.len(), 2);
    }

    #[test]
    fn a_change_carrying_no_reference_number_is_dropped() {
        let (new, stats) = rebuild_with(&library(), &[present(0, 10, "ghost.mp4", false)]);
        assert_eq!(new.len(), 4);
        assert_eq!(stats.files_added, 0);
    }

    #[test]
    fn the_new_index_is_searchable_by_the_new_name() {
        // The point of all of this. Rebuilding has to produce a real index,
        // folded and all — not merely a correct list of paths.
        use crate::index::search::{search, SearchOptions};
        use std::sync::atomic::AtomicU64;

        let (new, _) = rebuild_with(&library(), &[present(600, 10, "Tiếng Việt.mp4", false)]);

        let cancel = AtomicU64::new(0);
        let hits = search(&new, "tieng viet", &SearchOptions::default(), &[], &cancel, 0).hits;
        assert_eq!(hits.len(), 1);
        assert_eq!(new.full_path(hits[0].index as usize), r"D:\Phim\Tiếng Việt.mp4");
    }
}
