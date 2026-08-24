//! Finding duplicate files without reading them all.
//!
//! The library this is written for is three terabytes. Hashing it would take
//! hours of solid disk reading, so the work is arranged to avoid almost all of
//! it — three tiers, each one only looking at what survived the last:
//!
//! | Tier | Test | Reads | Survivors |
//! |---|---|---|---|
//! | 1 | Same byte count | **nothing** — the index already knows | a few % |
//! | 2 | Same first 64 KB, last 64 KB, and size | 128 KB per file | near-certain matches |
//! | 3 | Same content, whole file | everything | certain |
//!
//! Tier 1 does the heavy lifting for free: two files of different sizes cannot
//! be identical, and file sizes are spread widely enough that almost nothing
//! collides. Only the leftovers are ever opened.
//!
//! Tier 2 is where this stops in practice. Two different files sharing a size,
//! a first 64 KB *and* a last 64 KB happens by accident essentially never —
//! media containers put distinct headers at the front and index tables at the
//! back, which is exactly where this looks. Tier 3 exists for when a person
//! wants certainty before deleting something, and is never run on its own.
//!
//! **Nothing here deletes anything.** It reports; the person decides.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use rayon::prelude::*;
use serde::Serialize;

use crate::index::model::Index;

/// Bytes sampled from each end of a file in tier 2.
///
/// 64 KB comfortably covers the header of every container format in the
/// extension table, and the tail catches the index or moov atom that media formats put
/// at the end. Reading more would cost proportionally more for no gain.
const SAMPLE_BYTES: u64 = 64 * 1024;

/// Files smaller than this are hashed whole in tier 2 — reading 128 KB of a
/// 100 KB file twice would be slower than just reading it once.
const SMALL_FILE_LIMIT: u64 = SAMPLE_BYTES * 2;

/// Ignore files below this size.
///
/// Tiny files collide on size constantly — thousands of 1 KB thumbnails and
/// icons — and finding that two 800-byte files match is not worth anybody's
/// attention. Reclaiming space is the point.
const MIN_INTERESTING_SIZE: u64 = 64 * 1024;

// Checked when the crate is built. Thousands of icons and thumbnails share a
// size; reporting those would bury the groups actually worth acting on.
const _: () = assert!(MIN_INTERESTING_SIZE >= 64 * 1024);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    /// Bytes per copy.
    pub size: u64,
    /// Index positions of the files in this group, two or more.
    pub entries: Vec<u32>,
    /// What could be reclaimed by keeping one copy: `size * (count - 1)`.
    pub wasted: u64,
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupeProgress {
    pub running: bool,
    /// Files still to be sampled in tier 2.
    pub candidates: usize,
    pub hashed: usize,
    pub groups: usize,
    pub wasted: u64,
}

/// Shared handle so the UI can watch a scan that takes a while.
#[derive(Default)]
pub struct DupeService {
    running: Arc<AtomicBool>,
    candidates: Arc<AtomicUsize>,
    hashed: Arc<AtomicUsize>,
    result: Arc<parking_lot::Mutex<Vec<DuplicateGroup>>>,
}

impl DupeService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn progress(&self) -> DupeProgress {
        let groups = self.result.lock();
        DupeProgress {
            running: self.running.load(Ordering::Relaxed),
            candidates: self.candidates.load(Ordering::Relaxed),
            hashed: self.hashed.load(Ordering::Relaxed),
            groups: groups.len(),
            wasted: groups.iter().map(|g| g.wasted).sum(),
        }
    }

    pub fn groups(&self) -> Vec<DuplicateGroup> {
        self.result.lock().clone()
    }

    /// Start a scan, unless one is already running.
    ///
    /// Returns false when a scan is already in flight, so the UI can say so
    /// rather than silently starting a second one.
    pub fn start(&self, index: Arc<Index>) -> bool {
        if self.running.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.candidates.store(0, Ordering::Relaxed);
        self.hashed.store(0, Ordering::Relaxed);
        self.result.lock().clear();

        let running = Arc::clone(&self.running);
        let candidates = Arc::clone(&self.candidates);
        let hashed = Arc::clone(&self.hashed);
        let result = Arc::clone(&self.result);

        std::thread::Builder::new()
            .name("dupes".into())
            .spawn(move || {
                let started = std::time::Instant::now();
                let groups = find_duplicates(&index, &candidates, &hashed);
                let wasted: u64 = groups.iter().map(|g| g.wasted).sum();
                tracing::info!(
                    "tìm trùng lặp: {} nhóm, lãng phí {:.1} GB [{:.1}s]",
                    groups.len(),
                    wasted as f64 / 1024.0 / 1024.0 / 1024.0,
                    started.elapsed().as_secs_f64()
                );
                *result.lock() = groups;
                running.store(false, Ordering::Release);
            })
            .ok();
        true
    }
}

/// Tier 1 then tier 2.
fn find_duplicates(
    index: &Index,
    candidates: &AtomicUsize,
    hashed: &AtomicUsize,
) -> Vec<DuplicateGroup> {
    // Tier 1: group by size. Free — the index already holds every size, and
    // two files of different lengths cannot be the same file.
    let mut by_size: HashMap<u64, Vec<u32>> = HashMap::new();
    for (i, &size) in index.sizes().iter().enumerate() {
        if size >= MIN_INTERESTING_SIZE {
            by_size.entry(size).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    let work: Vec<(u64, Vec<u32>)> = by_size.into_iter().collect();
    let total: usize = work.iter().map(|(_, v)| v.len()).sum();
    candidates.store(total, Ordering::Relaxed);
    tracing::info!("tầng 1: {} tệp cùng dung lượng cần kiểm tra", total);

    // Tier 2: sample each candidate and regroup on the fingerprint. Parallel
    // because it is dominated by disk reads that overlap well.
    let mut groups: Vec<DuplicateGroup> = work
        .into_par_iter()
        .flat_map(|(size, entries)| {
            let mut by_hash: HashMap<[u8; 32], Vec<u32>> = HashMap::new();
            for i in entries {
                let path = index.full_path(i as usize);
                hashed.fetch_add(1, Ordering::Relaxed);
                if let Some(h) = fingerprint(&path, size) {
                    by_hash.entry(h).or_default().push(i);
                }
            }
            by_hash
                .into_values()
                .filter(|v| v.len() > 1)
                .map(|mut entries| {
                    // Stable order so the same scan lists a group the same way
                    // twice, and the first entry is a consistent "keep this".
                    entries.sort_unstable();
                    DuplicateGroup {
                        size,
                        wasted: size * (entries.len() as u64 - 1),
                        entries,
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();

    // Biggest waste first — the order somebody clearing space wants to work
    // through. The entry tiebreak keeps two runs of the same scan identical.
    groups.sort_unstable_by(|a, b| b.wasted.cmp(&a.wasted).then(a.entries[0].cmp(&b.entries[0])));
    groups
}

/// Hash the first and last [`SAMPLE_BYTES`] together with the size.
///
/// The size goes into the hash rather than being compared separately so two
/// files can never be judged equal on their samples alone.
fn fingerprint(path: &str, size: u64) -> Option<[u8; 32]> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(&size.to_le_bytes());

    if size <= SMALL_FILE_LIMIT {
        // Reading both ends of a small file would read most of it twice.
        let mut buf = Vec::with_capacity(size as usize);
        file.read_to_end(&mut buf).ok()?;
        hasher.update(&buf);
    } else {
        let mut buf = vec![0u8; SAMPLE_BYTES as usize];
        file.read_exact(&mut buf).ok()?;
        hasher.update(&buf);

        file.seek(SeekFrom::End(-(SAMPLE_BYTES as i64))).ok()?;
        file.read_exact(&mut buf).ok()?;
        hasher.update(&buf);
    }

    Some(*hasher.finalize().as_bytes())
}

/// Tier 3: hash a file end to end.
///
/// Only ever called for a group the person is about to act on, never during a
/// scan. On a 3 GB video this reads 3 GB.
pub fn full_hash(path: &str) -> Option<[u8; 32]> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(_) => return None,
        };
    }
    Some(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(name: &str, content: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("mediafinder-dupe-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).expect("create");
        f.write_all(content).expect("write");
        path
    }

    #[test]
    fn identical_files_share_a_fingerprint() {
        let content = vec![7u8; 200_000];
        let a = temp_file("dupe_a.bin", &content);
        let b = temp_file("dupe_b.bin", &content);

        let ha = fingerprint(&a.to_string_lossy(), content.len() as u64);
        let hb = fingerprint(&b.to_string_lossy(), content.len() as u64);
        assert!(ha.is_some());
        assert_eq!(ha, hb);
    }

    #[test]
    fn a_difference_in_the_middle_is_invisible_to_tier_two() {
        // Honest about the limit. Two files identical at both ends but
        // differing in the middle get the same fingerprint — which is why
        // tier 3 exists, and why nothing here deletes anything.
        let mut a = vec![7u8; 400_000];
        let mut b = a.clone();
        a[200_000] = 1;
        b[200_000] = 2;

        let pa = temp_file("mid_a.bin", &a);
        let pb = temp_file("mid_b.bin", &b);
        assert_eq!(
            fingerprint(&pa.to_string_lossy(), a.len() as u64),
            fingerprint(&pb.to_string_lossy(), b.len() as u64),
            "tầng 2 chỉ đọc hai đầu — đây là giới hạn đã biết"
        );

        // Tier 3 sees it.
        assert_ne!(
            full_hash(&pa.to_string_lossy()),
            full_hash(&pb.to_string_lossy()),
            "tầng 3 đọc toàn bộ nên phải phân biệt được"
        );
    }

    #[test]
    fn different_content_at_the_head_differs() {
        let a = vec![1u8; 200_000];
        let b = vec![2u8; 200_000];
        let pa = temp_file("head_a.bin", &a);
        let pb = temp_file("head_b.bin", &b);
        assert_ne!(
            fingerprint(&pa.to_string_lossy(), 200_000),
            fingerprint(&pb.to_string_lossy(), 200_000)
        );
    }

    #[test]
    fn a_small_file_is_read_whole() {
        // Below the two-sample threshold, reading both ends would read most of
        // the file twice; this path must still distinguish content.
        let a = temp_file("small_a.bin", b"hello world");
        let b = temp_file("small_b.bin", b"HELLO WORLD");
        assert_ne!(
            fingerprint(&a.to_string_lossy(), 11),
            fingerprint(&b.to_string_lossy(), 11)
        );
    }

    #[test]
    fn size_is_part_of_the_fingerprint() {
        // Two files whose samples match but whose lengths differ must not be
        // grouped — the size is mixed in so that cannot happen.
        let path = temp_file("size_x.bin", &vec![3u8; 200_000]);
        let p = path.to_string_lossy();
        assert_ne!(fingerprint(&p, 200_000), fingerprint(&p, 199_999));
    }

    #[test]
    fn a_missing_file_yields_no_fingerprint() {
        assert!(fingerprint(r"D:\definitely\not\here\nope.bin", 1000).is_none());
        assert!(full_hash(r"D:\definitely\not\here\nope.bin").is_none());
    }

    // The size floor is asserted at build time next to the constant itself.
}
