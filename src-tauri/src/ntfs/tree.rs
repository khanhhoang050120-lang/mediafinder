//! Phase 2: build the directory tree, resolve paths, apply exclusions.
//!
//! Phase 1 hands over a flat pile of records in MFT order. Nothing in that
//! pile knows its own path — each record knows only its parent's reference
//! number, and parents arrive in no particular order. Reconstructing paths is
//! therefore a separate pass, and directory-based exclusion can only happen
//! here.
//!
//! Two things make this cheap despite running over hundreds of thousands of
//! files:
//!
//!   * **Every directory is resolved at most once.** Results are memoised by
//!     FRN, so a path eight levels deep costs eight lookups the first time and
//!     one lookup for every later file underneath it.
//!   * **Directory paths are stored once, not once per file.** Resolution
//!     produces exactly the lookup table the index wants: a `Vec<String>` of
//!     directory paths plus a `dir_id` per file.
//!
//! This module contains no Win32 types on purpose — it takes plain
//! `RawRecord`s, which is what lets the awkward logic be unit-tested on CI
//! with no NTFS volume and no Administrator rights.

use std::collections::{HashMap, HashSet};

use super::{record_number, RawRecord, ROOT_RECORD_NUMBER};
use crate::index::model::MediaKind;

/// Directory names skipped along with everything beneath them.
///
/// Matched case-insensitively against a **single path component, at any
/// depth**. That means a user folder that happens to be called `Windows`
/// would also be skipped. The simpler rule is worth it: matching only at the
/// volume root would let `C:\Users\me\AppData\...` through, and AppData is
/// exactly the noise this is meant to remove.
pub const DEFAULT_EXCLUDED: &[&str] = &[
    "windows",
    "windows.old",
    "winsxs",
    "$recycle.bin",
    "$winreagent",
    "system volume information",
    "recovery",
    "programdata",
    "appdata",
    "program files",
    "program files (x86)",
    "perflogs",
    "msocache",
    "$sysreset",
    "node_modules",
];

#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// Directory component names to exclude, compared case-insensitively.
    ///
    /// A `Vec` rather than a `HashSet`: hashing would force a lowercase
    /// `String` allocation for every path component examined, and with only a
    /// handful of entries a linear `eq_ignore_ascii_case` scan is both faster
    /// and allocation-free.
    pub excluded: Vec<String>,
    /// Guard against a malformed parent chain; real NTFS trees are far shallower.
    pub max_depth: usize,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            excluded: DEFAULT_EXCLUDED.iter().map(|s| s.to_string()).collect(),
            max_depth: 128,
        }
    }
}

/// A media file with its directory reduced to an index into [`ResolvedSet::dirs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFile {
    pub name: String,
    pub kind: MediaKind,
    pub dir_id: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResolveStats {
    pub directories_seen: usize,
    pub media_files_in: usize,
    pub kept: usize,
    /// Dropped because some component of the path was excluded.
    pub excluded: usize,
    /// Dropped because the parent chain broke before reaching the root.
    pub orphaned: usize,
    /// Dropped because the parent chain looped back on itself.
    pub cycles: usize,
    pub too_deep: usize,
}

#[derive(Debug)]
pub struct ResolvedSet {
    /// Absolute directory paths. Index 0 is always the volume root (`C:`).
    pub dirs: Vec<String>,
    pub files: Vec<ResolvedFile>,
    pub stats: ResolveStats,
}

/// Outcome of resolving one directory, memoised per FRN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirState {
    Ok(u32),
    Excluded,
    Orphan,
    Cycle,
    TooDeep,
}

struct DirNode {
    name: String,
    parent_frn: u64,
}

const ROOT_DIR_ID: u32 = 0;

/// Turn phase-1 records into absolute paths.
pub fn resolve(records: Vec<RawRecord>, letter: char, opts: &ResolveOptions) -> ResolvedSet {
    let mut nodes: HashMap<u64, DirNode> = HashMap::new();
    let mut files: Vec<RawRecord> = Vec::new();

    for r in records {
        if r.is_dir {
            nodes.insert(
                r.frn,
                DirNode {
                    name: r.name,
                    parent_frn: r.parent_frn,
                },
            );
        } else if r.kind.is_some() {
            files.push(r);
        }
    }

    let mut resolver = Resolver {
        nodes,
        opts,
        // Index 0 is the volume root. Note the missing trailing backslash:
        // joining appends one, so `C:` + `\` + `Users` gives `C:\Users`.
        dirs: vec![format!("{letter}:")],
        memo: HashMap::new(),
        stats: ResolveStats::default(),
    };
    resolver.stats.directories_seen = resolver.nodes.len();
    resolver.stats.media_files_in = files.len();

    let mut out = Vec::with_capacity(files.len());
    for f in files {
        match resolver.resolve_dir(f.parent_frn) {
            DirState::Ok(dir_id) => {
                resolver.stats.kept += 1;
                out.push(ResolvedFile {
                    name: f.name,
                    // Safe: files are only collected when `kind` is `Some`.
                    kind: f.kind.expect("file record without a media kind"),
                    dir_id,
                });
            }
            DirState::Excluded => resolver.stats.excluded += 1,
            DirState::Orphan => resolver.stats.orphaned += 1,
            DirState::Cycle => resolver.stats.cycles += 1,
            DirState::TooDeep => resolver.stats.too_deep += 1,
        }
    }

    ResolvedSet {
        dirs: resolver.dirs,
        files: out,
        stats: resolver.stats,
    }
}

struct Resolver<'a> {
    nodes: HashMap<u64, DirNode>,
    opts: &'a ResolveOptions,
    dirs: Vec<String>,
    memo: HashMap<u64, DirState>,
    stats: ResolveStats,
}

impl Resolver<'_> {
    /// Resolve one directory FRN to its state, memoising the whole chain.
    ///
    /// Iterative rather than recursive: a corrupted parent chain could
    /// otherwise blow the stack before the depth guard ever fired.
    fn resolve_dir(&mut self, start: u64) -> DirState {
        let mut chain: Vec<u64> = Vec::new();
        let mut seen: HashSet<u64> = HashSet::new();
        let mut cur = start;

        // Walk up to the first node whose answer is already known.
        let base = loop {
            if let Some(&state) = self.memo.get(&cur) {
                break state;
            }
            // The root is recognised by record number, not by name: its own
            // record calls it "." and points at itself.
            if record_number(cur) == ROOT_RECORD_NUMBER {
                break DirState::Ok(ROOT_DIR_ID);
            }
            let Some(node) = self.nodes.get(&cur) else {
                break DirState::Orphan;
            };
            if node.parent_frn == cur {
                break DirState::Ok(ROOT_DIR_ID);
            }
            if !seen.insert(cur) {
                break DirState::Cycle;
            }
            if chain.len() >= self.opts.max_depth {
                break DirState::TooDeep;
            }
            chain.push(cur);
            cur = node.parent_frn;
        };

        let DirState::Ok(base_id) = base else {
            // Orphan / cycle / too-deep poisons everything below it, and every
            // one of those directories is now answered in O(1).
            for &frn in &chain {
                self.memo.insert(frn, base);
            }
            self.memo.insert(start, base);
            return base;
        };

        // Walk back down, building each path from its parent's.
        let mut path = self.dirs[base_id as usize].clone();
        let mut state = base;
        let mut excluded = false;

        for &frn in chain.iter().rev() {
            let node = &self.nodes[&frn];

            if !excluded
                && (self.opts.excluded.iter()).any(|e| e.eq_ignore_ascii_case(&node.name))
            {
                excluded = true;
            }
            if excluded {
                // Once a component is excluded so is everything under it, so
                // there is no point building the rest of the path.
                self.memo.insert(frn, DirState::Excluded);
                state = DirState::Excluded;
                continue;
            }

            path.push('\\');
            path.push_str(&node.name);
            let id = self.dirs.len() as u32;
            self.dirs.push(path.clone());
            self.memo.insert(frn, DirState::Ok(id));
            state = DirState::Ok(id);
        }

        self.memo.insert(start, state);
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Root FRN as NTFS actually reports it: record 5 with a sequence number
    /// in the high bits, so tests exercise the masking too.
    const ROOT: u64 = 0x0005_0000_0000_0005;

    fn dir(frn: u64, parent: u64, name: &str) -> RawRecord {
        RawRecord {
            frn,
            parent_frn: parent,
            name: name.into(),
            is_dir: true,
            kind: None,
        }
    }

    fn file(frn: u64, parent: u64, name: &str) -> RawRecord {
        RawRecord {
            frn,
            parent_frn: parent,
            name: name.into(),
            is_dir: false,
            kind: Some(MediaKind::Video),
        }
    }

    fn path_of(set: &ResolvedSet, f: &ResolvedFile) -> String {
        format!("{}\\{}", set.dirs[f.dir_id as usize], f.name)
    }

    #[test]
    fn resolves_a_deep_chain() {
        // C:\Users\padoma\Videos\2024\Trips\clip.mp4  — six levels below root.
        let records = vec![
            dir(10, ROOT, "Users"),
            dir(11, 10, "padoma"),
            dir(12, 11, "Videos"),
            dir(13, 12, "2024"),
            dir(14, 13, "Trips"),
            file(100, 14, "clip.mp4"),
        ];
        let set = resolve(records, 'C', &ResolveOptions::default());

        assert_eq!(set.files.len(), 1);
        assert_eq!(
            path_of(&set, &set.files[0]),
            r"C:\Users\padoma\Videos\2024\Trips\clip.mp4"
        );
        assert_eq!(set.stats.kept, 1);
    }

    #[test]
    fn file_directly_in_the_volume_root() {
        let set = resolve(vec![file(100, ROOT, "movie.mp4")], 'D', &ResolveOptions::default());
        assert_eq!(path_of(&set, &set.files[0]), r"D:\movie.mp4");
    }

    #[test]
    fn root_that_points_at_itself_is_still_recognised() {
        // Some volumes report the root with an FRN that does not mask to 5,
        // but whose parent is itself. Both forms must terminate the walk.
        let odd_root = 0x1234_0000_0000_9999u64;
        let records = vec![
            dir(odd_root, odd_root, "."),
            dir(10, odd_root, "Media"),
            file(100, 10, "a.mp4"),
        ];
        let set = resolve(records, 'E', &ResolveOptions::default());
        assert_eq!(path_of(&set, &set.files[0]), r"E:\Media\a.mp4");
    }

    #[test]
    fn preserves_vietnamese_names_exactly() {
        let records = vec![
            dir(10, ROOT, "Phim"),
            dir(11, 10, "Tiếng Việt"),
            file(100, 11, "Đà Nẵng 2024.mp4"),
        ];
        let set = resolve(records, 'C', &ResolveOptions::default());
        assert_eq!(
            path_of(&set, &set.files[0]),
            r"C:\Phim\Tiếng Việt\Đà Nẵng 2024.mp4"
        );
    }

    #[test]
    fn files_sharing_a_directory_share_one_dir_id() {
        let records = vec![
            dir(10, ROOT, "Videos"),
            file(100, 10, "a.mp4"),
            file(101, 10, "b.mp4"),
            file(102, 10, "c.mp4"),
        ];
        let set = resolve(records, 'C', &ResolveOptions::default());

        assert_eq!(set.files.len(), 3);
        let ids: HashSet<u32> = set.files.iter().map(|f| f.dir_id).collect();
        assert_eq!(ids.len(), 1, "one directory must yield one entry");
        // Root plus Videos, and nothing more.
        assert_eq!(set.dirs.len(), 2);
    }

    #[test]
    fn excludes_a_directory_and_everything_under_it() {
        let records = vec![
            dir(10, ROOT, "Windows"),
            dir(11, 10, "Media"),
            file(100, 11, "chimes.mp4"),
            dir(20, ROOT, "Videos"),
            file(101, 20, "keep.mp4"),
        ];
        let set = resolve(records, 'C', &ResolveOptions::default());

        assert_eq!(set.files.len(), 1);
        assert_eq!(set.files[0].name, "keep.mp4");
        assert_eq!(set.stats.excluded, 1);
    }

    #[test]
    fn exclusion_is_case_insensitive() {
        let records = vec![
            dir(10, ROOT, "Users"),
            dir(11, 10, "me"),
            dir(12, 11, "AppData"),
            dir(13, 12, "Local"),
            file(100, 13, "cached.mp4"),
        ];
        let set = resolve(records, 'C', &ResolveOptions::default());
        assert!(set.files.is_empty());
        assert_eq!(set.stats.excluded, 1);
    }

    #[test]
    fn orphan_is_counted_not_panicked_on() {
        // Parent 99 was never enumerated.
        let set = resolve(vec![file(100, 99, "lost.mp4")], 'C', &ResolveOptions::default());
        assert!(set.files.is_empty());
        assert_eq!(set.stats.orphaned, 1);
    }

    #[test]
    fn cycle_in_the_parent_chain_terminates() {
        // A -> B -> A, reachable from neither root nor anywhere else.
        let records = vec![
            dir(10, 11, "A"),
            dir(11, 10, "B"),
            file(100, 10, "trapped.mp4"),
        ];
        let set = resolve(records, 'C', &ResolveOptions::default());
        assert!(set.files.is_empty());
        assert_eq!(set.stats.cycles, 1, "must detect the loop, not hang");
    }

    #[test]
    fn depth_limit_is_enforced() {
        let mut records = vec![dir(10, ROOT, "d0")];
        for i in 1..40u64 {
            records.push(dir(10 + i, 9 + i, &format!("d{i}")));
        }
        records.push(file(1000, 49, "deep.mp4"));

        let opts = ResolveOptions {
            max_depth: 8,
            ..Default::default()
        };
        let set = resolve(records, 'C', &opts);
        assert!(set.files.is_empty());
        assert_eq!(set.stats.too_deep, 1);
    }

    #[test]
    fn memoisation_does_not_duplicate_directory_entries() {
        // Two branches sharing a long prefix: the prefix must be stored once.
        let records = vec![
            dir(10, ROOT, "Users"),
            dir(11, 10, "me"),
            dir(12, 11, "Videos"),
            dir(13, 11, "Music"),
            file(100, 12, "v.mp4"),
            file(101, 13, "m.mp4"),
        ];
        let set = resolve(records, 'C', &ResolveOptions::default());

        // C:, Users, me, Videos, Music — five, with no repeats.
        assert_eq!(set.dirs.len(), 5);
        let unique: HashSet<&String> = set.dirs.iter().collect();
        assert_eq!(unique.len(), 5, "no directory path may appear twice");
    }

    #[test]
    fn non_media_records_are_ignored() {
        let mut txt = file(100, ROOT, "notes.txt");
        txt.kind = None;
        let set = resolve(vec![txt], 'C', &ResolveOptions::default());
        assert!(set.files.is_empty());
        assert_eq!(set.stats.media_files_in, 0);
    }
}
