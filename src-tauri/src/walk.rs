//! Scanning a volume by walking its directories, for drives that have no MFT.
//!
//! Everything that makes the NTFS path fast — reading the master file table in
//! one pass — needs a disk attached to this machine. A mapped network drive is
//! an SMB session, not a volume: there is no MFT to read, no USN journal to
//! follow, and `\\.\Z:` cannot even be opened. Walking directories is the only
//! way in.
//!
//! It is slower, but far less than expected. Measured on the user's own NAS:
//! **1 300–1 600 entries/second** single-threaded, against 3 219/s doing the
//! same walk on a local disk. Two times slower, not a hundred. And the cost is
//! **latency**, not bandwidth — every `read_dir` is a round trip to the
//! server — which is exactly the shape of work that parallelises well.
//!
//! Two traps, both already paid for elsewhere in this project:
//!
//! * **Reparse points must be skipped.** A junction turns a tree into a graph:
//!   the walk counts the same files twice and wanders into directories the
//!   exclusion rules meant to keep it out of. This is not hypothetical — it
//!   happened to the verification script in `CHECK-005`, which reported three
//!   times too many files on `C:` until junctions were skipped.
//! * **Network files have no reference number.** Identity has to be the path.
//!   Entries are given FRN 0, which `index::update` already treats as "no
//!   identity" and never matches against a journal record — so an incremental
//!   update leaves network entries exactly as they are, which is correct: the
//!   journal knows nothing about them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::index::model::classify_name;
use crate::media::metadata::FileStats;
use crate::ntfs::tree::{ResolveOptions, ResolvedFile, ResolveStats, ResolvedSet};

/// How many directories to hand to rayon at once.
///
/// The walk goes level by level, so a level with only a handful of directories
/// cannot use every thread no matter what. Real trees fan out within two or
/// three levels, and until then the walk is fast anyway because there is
/// almost nothing in it.
const MIN_PARALLEL: usize = 4;

/// Guard against a chain that never reaches a leaf.
///
/// Reparse points are skipped, so a true cycle should be impossible — but a
/// server that answers oddly should stop the walk rather than hang it.
const MAX_DEPTH: usize = 64;

/// Progress, sampled while the walk runs.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WalkProgress {
    pub dirs_done: usize,
    pub files_seen: usize,
    pub media_kept: usize,
}

/// Walk `root` (e.g. `Z:\`) and collect every media file under it.
///
/// `on_progress` is called between levels rather than per entry: a network
/// walk turns up tens of thousands of files, and reporting each one would cost
/// more than the walk.
///
/// Returns early with whatever it has if `cancel` is set — a NAS walk runs for
/// minutes, and a user who changes their mind should not have to wait it out.
pub fn walk_volume(
    letter: char,
    opts: &ResolveOptions,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(WalkProgress),
) -> (ResolvedSet, Vec<FileStats>) {
    let root = format!("{letter}:");
    let mut dirs: Vec<String> = vec![root.clone()];
    // Directory path -> its id, so files can name their parent by index.
    let mut dir_id: HashMap<String, u32> = HashMap::new();
    dir_id.insert(root.to_ascii_lowercase(), 0);

    let mut files: Vec<ResolvedFile> = Vec::new();
    let mut file_stats: Vec<FileStats> = Vec::new();
    let mut stats = ResolveStats::default();

    let files_seen = AtomicUsize::new(0);
    let mut level: Vec<(PathBuf, u32)> = vec![(PathBuf::from(format!("{letter}:\\")), 0)];
    let mut depth = 0;

    while !level.is_empty() && depth < MAX_DEPTH {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        let read = |(path, id): &(PathBuf, u32)| read_one(path, *id, opts, &files_seen);
        let results: Vec<DirContents> = if level.len() >= MIN_PARALLEL {
            level.par_iter().map(read).collect()
        } else {
            level.iter().map(read).collect()
        };

        let mut next: Vec<(PathBuf, u32)> = Vec::new();
        for r in results {
            stats.directories_seen += 1;
            if r.unreadable {
                stats.orphaned += 1;
            }
            stats.excluded += r.excluded;

            for (path, name) in r.subdirs {
                let full = format!(
                    "{}\\{name}",
                    dirs[r.id as usize]
                );
                let key = full.to_ascii_lowercase();
                let id = match dir_id.get(&key) {
                    // A path reached twice means a link was followed despite
                    // the reparse check; take the first spelling and move on.
                    Some(&existing) => existing,
                    None => {
                        dirs.push(full);
                        let id = (dirs.len() - 1) as u32;
                        dir_id.insert(key, id);
                        id
                    }
                };
                next.push((path, id));
            }

            for (name, kind, fs) in r.media {
                stats.media_files_in += 1;
                stats.kept += 1;
                files.push(ResolvedFile {
                    name,
                    kind,
                    dir_id: r.id,
                    // No reference number exists over SMB. See the module note.
                    frn: 0,
                });
                file_stats.push(fs);
            }
        }

        on_progress(WalkProgress {
            dirs_done: stats.directories_seen,
            files_seen: files_seen.load(Ordering::Relaxed),
            media_kept: files.len(),
        });

        level = next;
        depth += 1;
    }

    if depth >= MAX_DEPTH {
        stats.too_deep += level.len();
    }

    (
        ResolvedSet {
            // Directories that ended up holding nothing are kept rather than
            // pruned: the table is small next to the file list, and pruning
            // would mean a second pass over everything to renumber the parents.
            dir_frns: vec![0; dirs.len()],
            dirs,
            files,
            stats,
        },
        file_stats,
    )
}

/// What one directory contained.
struct DirContents {
    id: u32,
    subdirs: Vec<(PathBuf, String)>,
    media: Vec<(String, crate::index::model::MediaKind, FileStats)>,
    excluded: usize,
    unreadable: bool,
}

/// Size and modification time, from the directory listing.
fn stats_of(entry: &std::fs::DirEntry) -> FileStats {
    let Ok(meta) = entry.metadata() else {
        return FileStats::default();
    };
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    FileStats {
        size: meta.len(),
        mtime,
    }
}

fn read_one(
    path: &Path,
    id: u32,
    opts: &ResolveOptions,
    files_seen: &AtomicUsize,
) -> DirContents {
    let mut out = DirContents {
        id,
        subdirs: Vec::new(),
        media: Vec::new(),
        excluded: 0,
        unreadable: false,
    };

    let Ok(entries) = std::fs::read_dir(path) else {
        // Permission denied, or the share went away mid-walk. Neither is worth
        // stopping for; a network drive that disappears takes its whole
        // subtree with it and the rest of the scan is still valid.
        out.unreadable = true;
        return out;
    };

    let mut seen = 0usize;
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };

        // Checked before anything else. A junction reports itself as a
        // directory, and following one is how a walk ends up counting the same
        // files twice — or wandering into a tree the rules meant to exclude.
        if kind.is_symlink() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        if kind.is_dir() {
            if opts.excludes_component(&name) {
                out.excluded += 1;
                continue;
            }
            out.subdirs.push((entry.path(), name));
        } else {
            seen += 1;
            if let Some(k) = classify_name(&name) {
                // Free on Windows: `DirEntry::metadata` reuses what the
                // directory enumeration already returned, with no extra system
                // call. Over SMB that difference is the whole ball game — a
                // separate stat per file would double the number of round
                // trips for a hundred thousand files.
                out.media.push((name, k, stats_of(&entry)));
            }
        }
    }
    files_seen.fetch_add(seen, Ordering::Relaxed);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build a small tree under a fresh temp directory and walk it.
    ///
    /// Uses a real directory rather than a fake filesystem, because what is
    /// being tested here *is* the interaction with the filesystem.
    struct Tree {
        root: PathBuf,
    }

    impl Tree {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!("mediafinder-walk-{name}"));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("tạo thư mục thử");
            Self { root }
        }

        fn dir(&self, rel: &str) -> PathBuf {
            let p = self.root.join(rel);
            fs::create_dir_all(&p).expect("tạo thư mục con");
            p
        }

        fn file(&self, rel: &str) {
            let p = self.root.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).expect("tạo thư mục cha");
            }
            fs::write(&p, b"x").expect("tạo tệp");
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// Walk an arbitrary directory, not a drive root.
    fn walk_dir(root: &Path, opts: &ResolveOptions) -> (ResolvedSet, Vec<FileStats>) {
        let cancel = AtomicBool::new(false);
        let files_seen = AtomicUsize::new(0);
        let mut dirs: Vec<String> = vec![root.to_string_lossy().into_owned()];
        let mut dir_id: HashMap<String, u32> = HashMap::new();
        dir_id.insert(dirs[0].to_ascii_lowercase(), 0);
        let mut files = Vec::new();
        let mut file_stats = Vec::new();
        let mut stats = ResolveStats::default();
        let mut level = vec![(root.to_path_buf(), 0u32)];

        while !level.is_empty() && !cancel.load(Ordering::Relaxed) {
            let results: Vec<DirContents> = level
                .iter()
                .map(|(p, id)| read_one(p, *id, opts, &files_seen))
                .collect();
            let mut next = Vec::new();
            for r in results {
                stats.directories_seen += 1;
                stats.excluded += r.excluded;
                for (path, name) in r.subdirs {
                    let full = format!("{}\\{name}", dirs[r.id as usize]);
                    dirs.push(full.clone());
                    let id = (dirs.len() - 1) as u32;
                    dir_id.insert(full.to_ascii_lowercase(), id);
                    next.push((path, id));
                }
                for (name, kind, fs) in r.media {
                    stats.kept += 1;
                    files.push(ResolvedFile {
                        name,
                        kind,
                        dir_id: r.id,
                        frn: 0,
                    });
                    file_stats.push(fs);
                }
            }
            level = next;
        }
        (
            ResolvedSet {
                dir_frns: vec![0; dirs.len()],
                dirs,
                files,
                stats,
            },
            file_stats,
        )
    }

    fn names(set: &ResolvedSet) -> Vec<String> {
        let mut v: Vec<String> = set
            .files
            .iter()
            .map(|f| format!("{}\\{}", set.dirs[f.dir_id as usize], f.name))
            .collect();
        v.sort();
        v
    }

    #[test]
    fn finds_media_and_ignores_everything_else() {
        let t = Tree::new("basic");
        t.file("phim.mp4");
        t.file("ghi chú.txt");
        t.file("ảnh.jpg");
        t.file("chương trình.exe");

        let (set, _) = walk_dir(&t.root, &ResolveOptions::default());
        let found: Vec<String> = set.files.iter().map(|f| f.name.clone()).collect();

        assert_eq!(set.files.len(), 2, "chỉ media mới được giữ: {found:?}");
        assert!(found.contains(&"phim.mp4".to_string()));
        assert!(found.contains(&"ảnh.jpg".to_string()));
    }

    #[test]
    fn descends_into_subdirectories_and_keeps_the_path_right() {
        let t = Tree::new("nested");
        t.file("a/b/c/sâu.mp4");
        t.file("a/nông.mkv");

        let (set, _) = walk_dir(&t.root, &ResolveOptions::default());
        let paths = names(&set);

        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.ends_with(r"a\b\c\sâu.mp4")), "{paths:?}");
        assert!(paths.iter().any(|p| p.ends_with(r"a\nông.mkv")), "{paths:?}");
    }

    #[test]
    fn skips_excluded_directories_and_everything_under_them() {
        let t = Tree::new("excluded");
        t.file("node_modules/gói/logo.png");
        t.file("giữ lại.png");
        t.dir(".recycle_bin");
        t.file(".recycle_bin/đã xoá.mp4");

        let (set, _) = walk_dir(&t.root, &ResolveOptions::default());
        let found: Vec<String> = set.files.iter().map(|f| f.name.clone()).collect();

        assert_eq!(found, vec!["giữ lại.png".to_string()], "{found:?}");
        assert_eq!(set.stats.excluded, 2);
    }

    #[test]
    fn a_directory_that_cannot_be_read_does_not_stop_the_walk() {
        // Not a permission test — that needs a second account. This checks the
        // shape of the failure: a path that is not a directory at all stands in
        // for one the walk is refused, and the walk must carry on regardless.
        let t = Tree::new("unreadable");
        t.file("thư mục giả/phim.mp4");
        let files_seen = AtomicUsize::new(0);
        let missing = t.root.join("không tồn tại");

        let r = read_one(&missing, 0, &ResolveOptions::default(), &files_seen);
        assert!(r.unreadable);
        assert!(r.media.is_empty());

        // And the real tree still walks.
        let (set, _) = walk_dir(&t.root, &ResolveOptions::default());
        assert_eq!(set.files.len(), 1);
    }

    #[test]
    fn an_empty_tree_produces_an_empty_result_rather_than_an_error() {
        let t = Tree::new("empty");
        let (set, _) = walk_dir(&t.root, &ResolveOptions::default());
        assert!(set.files.is_empty());
        assert_eq!(set.dirs.len(), 1, "chỉ có gốc");
    }

    #[test]
    fn size_and_time_come_back_with_the_walk_not_from_a_second_pass() {
        // The point of taking them here: over SMB a separate stat per file
        // would double the round trips, and there can be a hundred thousand
        // files. On Windows the directory listing already carries both.
        let t = Tree::new("stats");
        std::fs::write(t.root.join("phim.mp4"), vec![0u8; 4096]).expect("ghi tệp");

        let (set, stats) = walk_dir(&t.root, &ResolveOptions::default());

        assert_eq!(set.files.len(), 1);
        assert_eq!(stats.len(), set.files.len(), "phải song song với danh sách tệp");
        assert_eq!(stats[0].size, 4096);
        assert!(stats[0].mtime > 0, "phải có thời gian sửa đổi thật");
    }

    #[test]
    fn network_entries_carry_no_reference_number() {
        // Deliberate, and load-bearing: `index::update` treats 0 as "no
        // identity", so a USN journal record can never match a network entry.
        // That is correct — the journal of a local disk knows nothing about a
        // file on a NAS.
        let t = Tree::new("no-frn");
        t.file("phim.mp4");

        let (set, _) = walk_dir(&t.root, &ResolveOptions::default());
        assert!(set.files.iter().all(|f| f.frn == 0));
        assert!(set.dir_frns.iter().all(|&f| f == 0));
        assert_eq!(set.dir_frns.len(), set.dirs.len());
    }
}
