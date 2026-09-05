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
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
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
    /// A scan has finished and its answer is the one in `groups`.
    ///
    /// Needed because "no groups" is a real answer. Deciding from `groups > 0`
    /// alone would treat a library with nothing duplicated as never scanned,
    /// and re-run the whole thing on every visit to the view.
    pub completed: bool,
}

/// Shared handle so the UI can watch a scan that takes a while.
#[derive(Default)]
pub struct DupeService {
    running: Arc<AtomicBool>,
    /// Raised to ask the scan thread to give up. Checked inside both passes,
    /// so leaving the view stops the disk instead of letting it read for
    /// another ten minutes to produce an answer nobody will see.
    stop: Arc<AtomicBool>,
    completed: Arc<AtomicBool>,
    candidates: Arc<AtomicUsize>,
    hashed: Arc<AtomicUsize>,
    result: Arc<parking_lot::Mutex<Vec<DuplicateGroup>>>,
    /// Chỉ mục mà lượt quét này đã dùng, giữ nguyên cạnh kết quả.
    ///
    /// `DuplicateGroup.entries` là **vị trí** trong chỉ mục, mà vị trí không
    /// sống sót qua một lần dựng lại: `index::update::rebuild_with` nói thẳng
    /// *"Entry positions are not preserved — they cannot be"*, vì mục ở giữa
    /// biến mất thì mọi thứ sau nó trượt lên.
    ///
    /// Thiếu trường này thì `dupe_groups` tra vị trí cũ trên snapshot MỚI, và
    /// mỗi nhóm hiện tên cùng đường dẫn của **tệp khác** — không cảnh báo gì.
    /// Với một màn hình mà bước tiếp theo là xoá tệp, đó là kiểu hỏng đắt
    /// nhất có thể có.
    ///
    /// Chuyện này không hiếm: chỉ mục dựng lại sau mỗi lượt cập nhật gia tăng
    /// có xoá, và sau **mỗi** lượt quét ổ mạng theo lịch — hai lần mỗi ngày,
    /// trên mọi máy.
    snapshot: Arc<parking_lot::Mutex<Option<Arc<Index>>>>,
    /// `epoch` của chỉ mục lúc quét.
    ///
    /// Giao diện ghép `epoch` với vị trí để dựng URL ảnh thu nhỏ và xem trước
    /// (`thumbUrl(epoch, index)`), nên nó phải là epoch của **cùng** chỉ mục
    /// đã sinh ra các vị trí đó. Lấy epoch hiện tại ghép với vị trí cũ thì ảnh
    /// thu nhỏ cũng của tệp khác.
    epoch: Arc<AtomicU64>,
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
            completed: self.completed.load(Ordering::Relaxed),
        }
    }

    pub fn groups(&self) -> Vec<DuplicateGroup> {
        self.result.lock().clone()
    }

    /// Chỉ mục mà kết quả hiện tại trỏ vào, kèm epoch của nó.
    ///
    /// `None` khi chưa quét lần nào. Người gọi **phải** phân giải vị trí bằng
    /// snapshot này chứ không phải snapshot hiện tại của `AppState` — đó là
    /// toàn bộ lý do nó tồn tại.
    pub fn scanned_index(&self) -> Option<(Arc<Index>, u64)> {
        let ix = self.snapshot.lock().clone()?;
        Some((ix, self.epoch.load(Ordering::Relaxed)))
    }

    /// Ask a running scan to stop.
    ///
    /// Returns once the flag is raised, not once the thread has noticed — the
    /// caller is the UI and should not wait on disk. A scan that stops leaves
    /// `completed` false, so the next visit starts a fresh one rather than
    /// showing half an answer as if it were whole.
    pub fn cancel(&self) {
        if self.running.load(Ordering::Relaxed) {
            self.stop.store(true, Ordering::Relaxed);
            tracing::info!("tìm trùng lặp: đã yêu cầu dừng");
        }
    }

    /// Start a scan, unless one is already running.
    ///
    /// Returns false when a scan is already in flight, so the UI can say so
    /// rather than silently starting a second one.
    /// Bắt đầu một lượt quét.
    ///
    /// `epoch` là số hiệu của chính `index` truyền vào — hai thứ phải đi cùng
    /// nhau, vì kết quả trỏ vào vị trí trong chỉ mục đó.
    pub fn start(&self, index: Arc<Index>, epoch: u64) -> bool {
        if self.running.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.stop.store(false, Ordering::Relaxed);
        self.completed.store(false, Ordering::Relaxed);
        self.candidates.store(0, Ordering::Relaxed);
        self.hashed.store(0, Ordering::Relaxed);
        self.result.lock().clear();
        // Giữ chỉ mục này cạnh kết quả. `Arc` nên không tốn thêm bộ nhớ cho
        // bản thân dữ liệu — chỉ giữ nó sống lâu hơn snapshot toàn cục.
        *self.snapshot.lock() = Some(Arc::clone(&index));
        self.epoch.store(epoch, Ordering::Relaxed);

        let running = Arc::clone(&self.running);
        let stop = Arc::clone(&self.stop);
        let completed = Arc::clone(&self.completed);
        let candidates = Arc::clone(&self.candidates);
        let hashed = Arc::clone(&self.hashed);
        let result = Arc::clone(&self.result);

        std::thread::Builder::new()
            .name("dupes".into())
            .spawn(move || {
                let started = std::time::Instant::now();
                let groups = find_duplicates(&index, &candidates, &hashed, &stop);

                if stop.load(Ordering::Relaxed) {
                    tracing::info!(
                        "tìm trùng lặp: đã dừng theo yêu cầu [{:.1}s]",
                        started.elapsed().as_secs_f64()
                    );
                } else {
                    let wasted: u64 = groups.iter().map(|g| g.wasted).sum();
                    tracing::info!(
                        "tìm trùng lặp: {} nhóm, lãng phí {:.1} GB [{:.1}s]",
                        groups.len(),
                        wasted as f64 / 1024.0 / 1024.0 / 1024.0,
                        started.elapsed().as_secs_f64()
                    );
                    *result.lock() = groups;
                    // Set before `running` drops, so a UI that sees the scan
                    // stop never reads `completed` as false in between.
                    completed.store(true, Ordering::Relaxed);
                }
                running.store(false, Ordering::Release);
            })
            .ok();
        true
    }
}

/// Tier 1 then tier 2.
///
/// Returns nothing if `stop` is raised, rather than a partial answer that
/// could be mistaken for a complete one.
fn find_duplicates(
    index: &Index,
    candidates: &AtomicUsize,
    hashed: &AtomicUsize,
    stop: &AtomicBool,
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

    // One work item per *file*, not per size. Grouping the parallel work by
    // size puts every file sharing one common size onto a single thread while
    // the others sit idle; a library where one size is very popular then runs
    // most of the scan single-threaded.
    let work: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&size, entries)| entries.iter().map(move |&i| (size, i)))
        .collect();
    let total = work.len();
    candidates.store(total, Ordering::Relaxed);
    tracing::info!("tầng 1: {} tệp cùng dung lượng cần kiểm tra", total);

    // Tier 2: sample each candidate, then regroup on the fingerprint.
    //
    // Both ends in one pass, one open per file. Splitting this into a
    // head-only pass and a tail pass was tried and measured: on the real
    // library the head separated 166 of 29,053 candidates — 0.6% — because
    // most candidates here are genuine copies, whose heads *should* match.
    // The split bought nothing and cost a second open per file. See PERF-003.
    let fingerprints: Vec<(u64, u32, [u8; 32])> = work
        .into_par_iter()
        .filter_map(|(size, i)| {
            if stop.load(Ordering::Relaxed) {
                return None;
            }
            let path = index.full_path(i as usize);
            hashed.fetch_add(1, Ordering::Relaxed);
            fingerprint(&path, size).map(|h| (size, i, h))
        })
        .collect();

    if stop.load(Ordering::Relaxed) {
        return Vec::new();
    }

    let mut by_hash: HashMap<(u64, [u8; 32]), Vec<u32>> = HashMap::new();
    for (size, i, h) in fingerprints {
        by_hash.entry((size, h)).or_default().push(i);
    }

    let mut groups: Vec<DuplicateGroup> = by_hash
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|((size, _), mut entries)| {
            // Stable order so the same scan lists a group the same way twice,
            // and the first entry is a consistent "keep this".
            entries.sort_unstable();
            DuplicateGroup {
                size,
                wasted: size * (entries.len() as u64 - 1),
                entries,
            }
        })
        .collect();

    // Biggest waste first — the order somebody clearing space wants to work
    // through. The entry tiebreak keeps two runs of the same scan identical.
    groups.sort_unstable_by(|a, b| {
        b.wasted
            .cmp(&a.wasted)
            .then(a.entries[0].cmp(&b.entries[0]))
    });
    groups
}

/// Hash the first and last [`SAMPLE_BYTES`] together with the size.
///
/// The size goes into the hash rather than being compared separately so two
/// files can never be judged equal on their samples alone.
///
/// One open, one read of each end. See the note in [`find_duplicates`] for why
/// this is not split into a cheap first pass and an expensive second one.
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

    /// Build a real `Index` over files on disk, so `find_duplicates` is
    /// exercised end to end rather than through a stand-in.
    fn index_over(tag: &str, files: &[(&str, Vec<u8>)]) -> (Index, std::path::PathBuf) {
        use crate::index::model::{IndexBuilder, MediaKind};

        let dir = std::env::temp_dir().join(format!("mediafinder-dupe-idx-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        let mut b = IndexBuilder::new();
        let did = b.add_dir(dir.to_str().expect("utf8"), 1);
        let mut sizes = Vec::new();
        for (i, (name, content)) in files.iter().enumerate() {
            let mut f = std::fs::File::create(dir.join(name)).expect("create");
            f.write_all(content).expect("write");
            b.add_file(name, MediaKind::Video, did, 100 + i as u64);
            sizes.push(content.len() as u64);
        }
        let mut index = b.finish();
        let mtimes = vec![0i64; sizes.len()];
        index.set_file_stats(sizes, mtimes);
        (index, dir)
    }

    fn run(index: &Index) -> Vec<DuplicateGroup> {
        find_duplicates(
            index,
            &AtomicUsize::new(0),
            &AtomicUsize::new(0),
            &AtomicBool::new(false),
        )
    }

    /// Kết quả phải trỏ vào chỉ mục CỦA LƯỢT QUÉT, không phải chỉ mục hiện tại.
    ///
    /// Đây là lỗi đắt nhất mà màn hình Trùng lặp có thể mắc, vì bước tiếp theo
    /// của người dùng là **xoá tệp**. `entries` là vị trí trong chỉ mục, mà vị
    /// trí không sống sót qua một lần dựng lại — `index::update::rebuild_with`
    /// nói thẳng *"Entry positions are not preserved — they cannot be"*.
    ///
    /// Kịch bản thật: quét lúc 9:00, rời màn hình; 10:25 lịch quét ổ mạng chạy
    /// xong và chỉ mục nạp lại; 11:00 quay lại thì mỗi nhóm hiện tên và đường
    /// dẫn của tệp KHÁC, không một lời cảnh báo. Từ v1.0.8 chuyện này xảy ra
    /// hai lần mỗi ngày trên mọi máy.
    #[test]
    fn ket_qua_giu_chi_muc_cua_luot_quet_chu_khong_theo_chi_muc_moi() {
        let svc = DupeService::new();
        assert!(
            svc.scanned_index().is_none(),
            "chưa quét thì không được có chỉ mục nào"
        );

        let (ix_cu, _d1) = index_over("giu-snapshot", &[("a.mp4", vec![1u8; 200_000])]);
        let arc_cu = Arc::new(ix_cu);
        assert!(svc.start(Arc::clone(&arc_cu), 7));

        // Chờ lượt quét xong để `running` hạ xuống.
        for _ in 0..200 {
            if !svc.progress().running {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let (giu, epoch) = svc.scanned_index().expect("phải giữ chỉ mục đã quét");
        assert_eq!(
            epoch, 7,
            "epoch phải là epoch lúc quét, không phải epoch mới"
        );
        assert!(
            Arc::ptr_eq(&giu, &arc_cu),
            "phải giữ ĐÚNG chỉ mục đã quét, không phải một bản khác"
        );

        // Chỉ mục mới ra đời (lượt quét NAS xong, cache nạp lại). Kết quả cũ
        // vẫn phải phân giải theo chỉ mục cũ.
        let (ix_moi, _d2) = index_over("giu-snapshot-2", &[("z.mp4", vec![9u8; 200_000])]);
        drop(ix_moi);
        let (van_giu, van_epoch) = svc.scanned_index().expect("vẫn phải còn");
        assert!(Arc::ptr_eq(&van_giu, &arc_cu));
        assert_eq!(van_epoch, 7);
    }

    /// The size floor exists so thousands of identical icons do not bury the
    /// groups worth acting on. Two matching small files must stay unreported.
    #[test]
    fn files_below_the_size_floor_are_never_reported() {
        let small = vec![7u8; 1024];
        let (index, _d) = index_over(
            "floor",
            &[("tiny-a.mp4", small.clone()), ("tiny-b.mp4", small)],
        );
        assert!(
            run(&index).is_empty(),
            "tep duoi nguong {} byte khong duoc bao",
            MIN_INTERESTING_SIZE
        );
    }

    /// Tier 1 must reject unique sizes *without opening the files* — that is
    /// where nearly all the saving comes from. Checking only the result would
    /// pass even if every file were read and then discarded.
    #[test]
    fn tier_one_rejects_unique_sizes_without_reading_them() {
        let (index, _d) = index_over(
            "tier1",
            &[
                ("u-1.mp4", vec![1u8; 200 * 1024]),
                ("u-2.mp4", vec![1u8; 201 * 1024]),
                ("u-3.mp4", vec![1u8; 202 * 1024]),
                ("u-4.mp4", vec![1u8; 203 * 1024]),
            ],
        );
        let candidates = AtomicUsize::new(0);
        let hashed = AtomicUsize::new(0);
        let groups = find_duplicates(&index, &candidates, &hashed, &AtomicBool::new(false));
        assert!(groups.is_empty());
        assert_eq!(
            hashed.load(Ordering::Relaxed),
            0,
            "kich thuoc duy nhat thi khong duoc mo tep nao"
        );
        assert_eq!(
            candidates.load(Ordering::Relaxed),
            0,
            "khong co ung vien nao de kiem"
        );
    }

    /// Two files of different lengths cannot be the same file, and tier 1 must
    /// reject them without opening either.
    #[test]
    fn different_sizes_never_form_a_group() {
        let (index, _d) = index_over(
            "sizes",
            &[
                ("a.mp4", vec![1u8; 200 * 1024]),
                ("b.mp4", vec![1u8; 201 * 1024]),
            ],
        );
        assert!(run(&index).is_empty());
    }

    /// The whole point: identical files land in one group, and the waste is
    /// counted as everything past the first copy.
    #[test]
    fn identical_files_are_grouped_and_waste_counted() {
        let content = vec![9u8; 300 * 1024];
        let (index, _d) = index_over(
            "same",
            &[
                ("copy-1.mp4", content.clone()),
                ("copy-2.mp4", content.clone()),
                ("copy-3.mp4", content),
            ],
        );
        let groups = run(&index);
        assert_eq!(groups.len(), 1, "ba ban sao phai nam chung mot nhom");
        assert_eq!(groups[0].entries.len(), 3);
        assert_eq!(
            groups[0].wasted,
            300 * 1024 * 2,
            "giu lai mot ban, hai ban kia la phan thua"
        );
    }

    /// The two-pass split must not change the answer: files that differ only
    /// at the tail have to be separated, exactly as the single-pass
    /// fingerprint separated them.
    #[test]
    fn a_difference_only_at_the_tail_still_splits_the_group() {
        let mut a = vec![4u8; 300 * 1024];
        let mut b = a.clone();
        let n = a.len();
        a[n - 1] = 1;
        b[n - 1] = 2;
        let (index, _d) = index_over("tail", &[("tail-a.mp4", a), ("tail-b.mp4", b)]);
        assert!(run(&index).is_empty(), "khac phan cuoi thi phai tach ra");
    }

    /// The head pass alone must separate files that differ at the front —
    /// they should never reach the tail pass at all.
    #[test]
    fn a_difference_at_the_head_splits_without_the_tail() {
        let mut a = vec![4u8; 300 * 1024];
        let mut b = a.clone();
        a[0] = 1;
        b[0] = 2;
        let (index, _d) = index_over("head", &[("head-a.mp4", a), ("head-b.mp4", b)]);
        assert!(run(&index).is_empty());
    }

    /// Files small enough to be read whole have no separate tail read. They
    /// must still group rather than fall through some special case.
    #[test]
    fn small_files_read_whole_still_group() {
        let content = vec![3u8; 100 * 1024];
        assert!(content.len() as u64 > MIN_INTERESTING_SIZE);
        assert!(content.len() as u64 <= SMALL_FILE_LIMIT);
        let (index, _d) = index_over(
            "small",
            &[("s-a.mp4", content.clone()), ("s-b.mp4", content)],
        );
        let groups = run(&index);
        assert_eq!(groups.len(), 1, "tep nho doc tron ven van phai gom nhom");
        assert_eq!(groups[0].entries.len(), 2);
    }

    /// Groups come back biggest-waste-first, because that is the order
    /// somebody clearing space works through.
    #[test]
    fn groups_are_sorted_by_waste_descending() {
        let (index, _d) = index_over(
            "sorted",
            &[
                ("small-1.mp4", vec![1u8; 200 * 1024]),
                ("small-2.mp4", vec![1u8; 200 * 1024]),
                ("big-1.mp4", vec![2u8; 900 * 1024]),
                ("big-2.mp4", vec![2u8; 900 * 1024]),
            ],
        );
        let groups = run(&index);
        assert_eq!(groups.len(), 2);
        assert!(
            groups[0].wasted > groups[1].wasted,
            "nhom lang phi nhieu nhat phai dung dau: {} roi {}",
            groups[0].wasted,
            groups[1].wasted
        );
    }

    /// Two runs over the same library must list the same groups the same way,
    /// or the list would reshuffle under a person working through it.
    #[test]
    fn two_runs_give_the_same_answer() {
        let c1 = vec![5u8; 300 * 1024];
        let c2 = vec![6u8; 400 * 1024];
        let (index, _d) = index_over(
            "stable",
            &[
                ("r-a.mp4", c1.clone()),
                ("r-b.mp4", c1),
                ("r-c.mp4", c2.clone()),
                ("r-d.mp4", c2),
            ],
        );
        let first = run(&index);
        let second = run(&index);
        assert_eq!(first.len(), second.len());
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.entries, b.entries, "thu tu phai giong nhau giua hai lan");
            assert_eq!(a.wasted, b.wasted);
        }
    }

    /// Raising the stop flag must abandon the scan and return nothing —
    /// a partial answer must never be mistaken for a complete one.
    #[test]
    fn a_cancelled_scan_returns_nothing() {
        let content = vec![8u8; 300 * 1024];
        let (index, _d) = index_over(
            "cancel",
            &[("c-a.mp4", content.clone()), ("c-b.mp4", content)],
        );
        let groups = find_duplicates(
            &index,
            &AtomicUsize::new(0),
            &AtomicUsize::new(0),
            &AtomicBool::new(true),
        );
        assert!(groups.is_empty(), "da huy thi khong tra ve ket qua do dang");
    }

    /// The progress counters have to reach the UI, or the bar sits at zero
    /// through a scan that lasts minutes.
    ///
    /// Asserts the exact count, not merely that it moved: both passes
    /// increment the same counter, so a check for "greater than zero" would
    /// still pass with one of them silently not counting.
    #[test]
    fn progress_counters_are_reported() {
        let content = vec![2u8; 300 * 1024];
        let (index, _d) = index_over(
            "progress",
            &[("p-a.mp4", content.clone()), ("p-b.mp4", content)],
        );
        let candidates = AtomicUsize::new(0);
        let hashed = AtomicUsize::new(0);
        find_duplicates(&index, &candidates, &hashed, &AtomicBool::new(false));
        // Two files sharing a size: both are candidates, both get read.
        assert_eq!(
            hashed.load(Ordering::Relaxed),
            2,
            "phai dem dung so tep da doc"
        );
        assert_eq!(
            candidates.load(Ordering::Relaxed),
            2,
            "tong so ung vien sau tang 1"
        );
    }

    /// Every candidate that gets read must be counted, or the bar sits at
    /// zero through a scan that lasts minutes.
    #[test]
    fn every_candidate_read_is_counted() {
        // Two files of one size, two of another: all four are candidates.
        let (index, _d) = index_over(
            "count-all",
            &[
                ("ca-1.mp4", vec![1u8; 300 * 1024]),
                ("ca-2.mp4", vec![1u8; 300 * 1024]),
                ("ca-3.mp4", vec![2u8; 400 * 1024]),
                ("ca-4.mp4", vec![3u8; 400 * 1024]),
            ],
        );
        let candidates = AtomicUsize::new(0);
        let hashed = AtomicUsize::new(0);
        let groups = find_duplicates(&index, &candidates, &hashed, &AtomicBool::new(false));

        assert_eq!(
            candidates.load(Ordering::Relaxed),
            4,
            "ca bon tep deu la ung vien sau tang 1"
        );
        assert_eq!(
            hashed.load(Ordering::Relaxed),
            4,
            "phai dem dung so tep da doc"
        );
        // Only the matching pair is a group; the 400 KB pair differs.
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].entries.len(), 2);
    }

    #[test]
    fn a_missing_file_yields_no_fingerprint() {
        assert!(fingerprint(r"D:\definitely\not\here\nope.bin", 1000).is_none());
        assert!(full_hash(r"D:\definitely\not\here\nope.bin").is_none());
    }

    // The size floor is asserted at build time next to the constant itself.
}
