//! Background enrichment: filling in width, height and duration over time.
//!
//! Reading these means opening the file. Measured on a real library the shell
//! takes 4–80 ms per file, averaging around 25 ms — so a 117k-file library is
//! roughly **50 minutes** of work. That cannot happen during a scan the user is
//! watching, and it cannot happen when they press a filter button.
//!
//! So it happens quietly, in the background, and the results are kept:
//!
//! * **Persisted between runs.** The cost is paid once, not once per launch.
//! * **Keyed by path, not by position.** Entry numbers change on every rescan;
//!   a path does not. Size and modification time are stored alongside so a
//!   file that changed is re-read rather than trusted.
//! * **Video first.** Resolution and duration are what people filter video by;
//!   an audio file's dimensions are always zero. Doing the useful ones first
//!   means the filter becomes usable in minutes rather than at the end.
//! * **Below-normal priority, few threads.** This runs while the user is doing
//!   something else on their own machine. Being fast is worth less than being
//!   unnoticeable.

use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
};

use crate::index::model::{Index, MediaKind};
use crate::media::metadata::{media_props, MediaProps};

/// See `persist.rs` — the version lives outside the encoded blob so it stays
/// readable when the layout it guards has changed.
const MAGIC: &[u8; 8] = b"MFMETA01";
const SCHEMA_VERSION: u32 = 1;
const HEADER_LEN: usize = MAGIC.len() + std::mem::size_of::<u32>();

/// Threads doing the reading.
///
/// Two, not more. Each request opens a file, so this is disk-bound; extra
/// threads would only make the user's own work slower without finishing sooner.
const WORKERS: usize = 2;

/// Save after this many newly read files.
///
/// Small enough that closing the app loses at most a few seconds of work,
/// large enough that saving is not itself a cost.
const SAVE_EVERY: usize = 500;

// Budget invariants, checked when the crate is built rather than when tests
// are run. This work happens while the user is doing something else on their
// own machine: more threads would take the disk away from them without
// finishing meaningfully sooner, and saving more often than every few hundred
// files would make saving itself the bottleneck.
const _: () = assert!(WORKERS < 4, "quá nhiều luồng sẽ chiếm đĩa của người dùng");
const _: () = assert!(WORKERS * SAVE_EVERY >= 200, "một lượt lưu phải bao đủ công việc");

/// One file's stored answer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Entry {
    /// Size and mtime at the time the properties were read. If either differs
    /// now, the file changed and the stored answer is about a different file.
    size: u64,
    mtime: i64,
    props: MediaProps,
}

#[derive(Default, Serialize, Deserialize)]
struct Store {
    /// Keyed by a hash of the lowercase full path.
    ///
    /// A hash rather than the path itself: 117k paths averaging 90 bytes would
    /// be 10 MB of keys, and the map is consulted once per entry per rescan.
    by_path: HashMap<u64, Entry>,
}

/// What the UI shows about enrichment.
#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichStatus {
    pub running: bool,
    /// Entries with usable properties, whether read now or loaded from disk.
    pub done: usize,
    pub total: usize,
}

pub struct EnrichService {
    store: Arc<Mutex<Store>>,
    /// Properties laid out parallel to the index, so search can filter by
    /// position without hashing a path per entry.
    props: Arc<ArcSwap<Vec<MediaProps>>>,
    done: Arc<AtomicUsize>,
    total: Arc<AtomicUsize>,
    running: Arc<AtomicBool>,
    /// Bumped when the index is replaced; workers from the previous index see
    /// the change and stop rather than writing answers about the wrong file.
    generation: Arc<AtomicU64>,
}

impl Default for EnrichService {
    fn default() -> Self {
        Self::new()
    }
}

impl EnrichService {
    pub fn new() -> Self {
        let store = load_store().unwrap_or_default();
        tracing::info!("nạp metadata: {} mục", store.by_path.len());
        Self {
            store: Arc::new(Mutex::new(store)),
            props: Arc::new(ArcSwap::from_pointee(Vec::new())),
            done: Arc::new(AtomicUsize::new(0)),
            total: Arc::new(AtomicUsize::new(0)),
            running: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Properties aligned with the current index, for filtering.
    ///
    /// Empty until [`EnrichService::start`] has been called for this index.
    pub fn props(&self) -> Arc<Vec<MediaProps>> {
        self.props.load_full()
    }

    pub fn status(&self) -> EnrichStatus {
        EnrichStatus {
            running: self.running.load(Ordering::Relaxed),
            done: self.done.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
        }
    }

    /// Point enrichment at a (new) index and begin filling in what is missing.
    ///
    /// Any workers still running for a previous index stop on their own.
    pub fn start(&self, index: Arc<Index>) {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let total = index.len();
        self.total.store(total, Ordering::Relaxed);

        if total == 0 {
            self.props.store(Arc::new(Vec::new()));
            self.done.store(0, Ordering::Relaxed);
            return;
        }

        // Seed from what is already known, so a relaunch starts with every
        // previously read file already filtered rather than blank.
        let (seeded, known) = self.seed_from_store(&index);
        self.props.store(Arc::new(seeded));
        self.done.store(known, Ordering::Relaxed);
        tracing::info!("enrichment: {known}/{total} mục đã có sẵn");

        if known >= total {
            self.running.store(false, Ordering::Relaxed);
            return;
        }

        let props = self.props.load();
        let mut queue: Vec<u32> = (0..total as u32)
            .filter(|&i| props[i as usize].is_empty())
            .collect();
        order_queue(&mut queue, |i| index.kind(i as usize));

        self.running.store(true, Ordering::Relaxed);
        let queue = Arc::new(Mutex::new(queue));
        let active = Arc::new(AtomicUsize::new(WORKERS));

        for n in 0..WORKERS {
            let ctx = WorkerCtx {
                index: Arc::clone(&index),
                queue: Arc::clone(&queue),
                store: Arc::clone(&self.store),
                props: Arc::clone(&self.props),
                done: Arc::clone(&self.done),
                running: Arc::clone(&self.running),
                generation: Arc::clone(&self.generation),
                active: Arc::clone(&active),
                my_generation: generation,
            };
            std::thread::Builder::new()
                .name(format!("enrich-{n}"))
                .spawn(move || worker(ctx))
                .ok();
        }
    }

    /// Build the props array from what the store already holds.
    fn seed_from_store(&self, index: &Index) -> (Vec<MediaProps>, usize) {
        let store = self.store.lock();
        let mut out = vec![MediaProps::default(); index.len()];
        let mut known = 0;

        for (i, slot) in out.iter_mut().enumerate() {
            let key = path_key(&index.full_path(i));
            if let Some(entry) = store.by_path.get(&key) {
                // A file whose size or timestamp moved is not the file this
                // answer was about.
                if entry.size == index.size(i) && entry.mtime == index.mtime(i) {
                    *slot = entry.props;
                    known += 1;
                }
            }
        }
        (out, known)
    }
}

struct WorkerCtx {
    index: Arc<Index>,
    queue: Arc<Mutex<Vec<u32>>>,
    store: Arc<Mutex<Store>>,
    props: Arc<ArcSwap<Vec<MediaProps>>>,
    done: Arc<AtomicUsize>,
    running: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    active: Arc<AtomicUsize>,
    my_generation: u64,
}

fn worker(ctx: WorkerCtx) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        // The user is doing something else on this machine. Finishing sooner
        // is worth less than staying out of the way.
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }

    // Accumulate locally and publish in batches: swapping the shared array is
    // cheap but not free, and a filter that updates twice a second is
    // indistinguishable from one that updates on every file.
    let mut pending: Vec<(u32, MediaProps)> = Vec::with_capacity(SAVE_EVERY);

    loop {
        if ctx.generation.load(Ordering::Acquire) != ctx.my_generation {
            break; // a new index arrived; these answers are about the old one
        }
        let Some(i) = ctx.queue.lock().pop() else {
            break;
        };

        let path = ctx.index.full_path(i as usize);
        let props = media_props(&path).unwrap_or_default();

        // Record even an empty answer, so a file the shell cannot read is not
        // retried on every launch forever.
        ctx.store.lock().by_path.insert(
            path_key(&path),
            Entry {
                size: ctx.index.size(i as usize),
                mtime: ctx.index.mtime(i as usize),
                props,
            },
        );
        pending.push((i, props));

        if pending.len() >= SAVE_EVERY {
            publish(&ctx, &mut pending);
            let _ = save_store(&ctx.store.lock());
        }
    }

    publish(&ctx, &mut pending);

    // The last worker out saves and clears the flag.
    if ctx.active.fetch_sub(1, Ordering::AcqRel) == 1 {
        let _ = save_store(&ctx.store.lock());
        ctx.running.store(false, Ordering::Relaxed);
        tracing::info!(
            "enrichment xong: {} mục",
            ctx.done.load(Ordering::Relaxed)
        );
    }
}

/// Copy a batch of results into the shared array.
fn publish(ctx: &WorkerCtx, pending: &mut Vec<(u32, MediaProps)>) {
    if pending.is_empty() {
        return;
    }
    let current = ctx.props.load_full();
    let mut next = (*current).clone();
    let mut gained = 0;
    for (i, props) in pending.drain(..) {
        if let Some(slot) = next.get_mut(i as usize) {
            if slot.is_empty() && !props.is_empty() {
                gained += 1;
            }
            *slot = props;
        }
    }
    ctx.props.store(Arc::new(next));
    ctx.done.fetch_add(gained, Ordering::Relaxed);
}

/// Order the work queue so the most valuable files are read first.
///
/// Workers take jobs with [`Vec::pop`], which removes from the **end**, so the
/// highest priority must sort *last*. Getting this backwards is silent and
/// expensive: the first attempt sorted video first and therefore read every
/// audio file first, and since audio has no dimensions the resolution filter
/// found nothing at all after tens of thousands of files had been read.
///
/// Video leads because resolution and duration are what video gets filtered
/// by. Images have dimensions but are rarely filtered on them, and an audio
/// file's dimensions are always zero.
fn order_queue(queue: &mut [u32], kind_of: impl Fn(u32) -> MediaKind) {
    queue.sort_by_key(|&i| match kind_of(i) {
        MediaKind::Audio => 0u8,
        MediaKind::Image => 1,
        MediaKind::Video => 2, // popped first
    });
}

/// Hash a path for use as a stable key.
///
/// Lowercased first: Windows paths are case-insensitive, and the same file
/// reached through a differently-cased path must not be read twice.
fn path_key(path: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.to_lowercase().hash(&mut h);
    h.finish()
}

fn store_path() -> Option<PathBuf> {
    crate::index::persist::cache_dir().ok().map(|d| d.join("metadata.bin"))
}

fn load_store() -> Option<Store> {
    let path = store_path()?;
    let file = fs::File::open(path).ok()?;
    let mut reader = BufReader::new(file);

    let mut header = [0u8; HEADER_LEN];
    reader.read_exact(&mut header).ok()?;
    if &header[..MAGIC.len()] != MAGIC {
        return None;
    }
    let version = u32::from_le_bytes(header[MAGIC.len()..].try_into().ok()?);
    if version != SCHEMA_VERSION {
        return None;
    }
    bincode::deserialize_from(&mut reader).ok()
}

fn save_store(store: &Store) -> Option<()> {
    let path = store_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).ok()?;
    }
    let tmp = path.with_extension("bin.tmp");
    {
        let file = fs::File::create(&tmp).ok()?;
        let mut writer = BufWriter::new(file);
        writer.write_all(MAGIC).ok()?;
        writer.write_all(&SCHEMA_VERSION.to_le_bytes()).ok()?;
        bincode::serialize_into(&mut writer, store).ok()?;
        writer.flush().ok()?;
    }
    fs::rename(&tmp, &path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_is_read_before_anything_else() {
        // Workers `pop` from the end, so priority order is *reverse* index
        // order. This got written backwards first time and the resolution
        // filter silently matched nothing.
        let kinds = [
            MediaKind::Audio,
            MediaKind::Video,
            MediaKind::Image,
            MediaKind::Video,
            MediaKind::Audio,
        ];
        let mut queue: Vec<u32> = (0..kinds.len() as u32).collect();
        order_queue(&mut queue, |i| kinds[i as usize]);

        // Drain the way a worker does and record what it actually gets.
        let taken: Vec<MediaKind> = std::iter::from_fn(|| queue.pop())
            .map(|i| kinds[i as usize])
            .collect();

        assert_eq!(
            &taken[..2],
            &[MediaKind::Video, MediaKind::Video],
            "video phải được đọc trước, thứ tự lấy việc là ngược lại"
        );
        assert_eq!(taken[2], MediaKind::Image);
        assert_eq!(&taken[3..], &[MediaKind::Audio, MediaKind::Audio]);
    }

    #[test]
    fn path_key_ignores_case_like_windows_does() {
        assert_eq!(path_key(r"D:\Phim\A.mp4"), path_key(r"d:\phim\a.MP4"));
        assert_ne!(path_key(r"D:\Phim\A.mp4"), path_key(r"D:\Phim\B.mp4"));
    }

    #[test]
    fn store_header_is_readable_without_decoding_the_body() {
        // Same lesson as the index cache: a version inside the encoded blob
        // cannot be read at the one moment it is needed.
        assert_eq!(HEADER_LEN, 12);
        assert_eq!(&MAGIC[..6], b"MFMETA");
    }

    #[test]
    fn a_fresh_service_reports_nothing_running() {
        let s = EnrichService::new();
        let st = s.status();
        assert!(!st.running);
        assert_eq!(st.total, 0);
        assert!(s.props().is_empty());
    }

    // The worker/save budget is asserted at build time next to the constants
    // themselves; see the `const _: () = assert!(…)` pair above.
}
