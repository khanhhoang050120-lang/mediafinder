//! Shared application state.
//!
//! The one rule that matters here: **a search never holds a lock.**
//!
//! A query can touch hundreds of thousands of entries across every CPU core.
//! Holding a read guard for that long would stall the rebuild that wants to
//! swap a fresh index in, and — the classic Tauri deadlock — holding a
//! `std::sync::MutexGuard` across an `.await` in a command would wedge the
//! whole IPC runtime.
//!
//! `ArcSwap` sidesteps both. A search clones the `Arc` and is immediately done
//! with the shared cell; it then scans a snapshot that nothing can mutate,
//! because [`Index`](crate::index::model::Index) is immutable once built. A
//! rebuild constructs an entirely new index off-thread and swaps the pointer
//! atomically. Searches already in flight keep reading the old snapshot and
//! finish normally — the memory lives exactly as long as its last reader.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use parking_lot::RwLock;
use serde::Serialize;

use crate::index::model::Index;

/// What the UI shows about the loaded index.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexMeta {
    pub loaded: bool,
    pub file_count: usize,
    pub dir_count: usize,
    pub memory_bytes: usize,
    /// Unix seconds when the scan that produced this index ran.
    pub built_at_unix: i64,
    /// Set when there is no usable index, explaining why in the user's terms.
    pub problem: Option<String>,
}

pub struct AppState {
    index: ArcSwap<Index>,
    /// Monotonic id of the newest query. Any search whose own id no longer
    /// matches has been superseded and stops as soon as it notices.
    generation: AtomicU64,
    meta: RwLock<IndexMeta>,
    /// Bumped on every index replacement.
    ///
    /// Entry numbers are positions in the index, so after a rebuild the same
    /// number means a different file. Thumbnail URLs carry this epoch so a
    /// request issued before the swap is refused rather than answered with a
    /// picture of the wrong file.
    epoch: AtomicU64,
    /// True while an elevated indexer child is running.
    ///
    /// Guards against a second UAC prompt while the first scan is still going,
    /// and lets the UI disable the button rather than silently ignoring clicks.
    scanning: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            index: ArcSwap::from_pointee(Index::default()),
            generation: AtomicU64::new(0),
            meta: RwLock::new(IndexMeta::default()),
            epoch: AtomicU64::new(1),
            scanning: AtomicBool::new(false),
        }
    }

    /// Take a snapshot to search.
    ///
    /// Cheap — one atomic refcount bump — and, crucially, the shared cell is
    /// released the instant this returns.
    pub fn snapshot(&self) -> Arc<Index> {
        self.index.load_full()
    }

    /// Swap in a freshly built index.
    pub fn replace(&self, index: Index, built_at_unix: i64) {
        let meta = IndexMeta {
            loaded: true,
            file_count: index.len(),
            dir_count: index.dir_count(),
            memory_bytes: index.memory_bytes(),
            built_at_unix,
            problem: None,
        };
        self.index.store(Arc::new(index));
        *self.meta.write() = meta;
        // After this, every previously issued thumbnail URL is stale.
        self.epoch.fetch_add(1, Ordering::Release);
    }

    /// Record that no index could be loaded, and why.
    pub fn set_problem(&self, problem: impl Into<String>) {
        *self.meta.write() = IndexMeta {
            problem: Some(problem.into()),
            ..Default::default()
        };
    }

    pub fn meta(&self) -> IndexMeta {
        self.meta.read().clone()
    }

    /// Claim `id` as the newest query, superseding anything older.
    pub fn begin_query(&self, id: u64) {
        self.generation.store(id, Ordering::Relaxed);
    }

    pub fn generation(&self) -> &AtomicU64 {
        &self.generation
    }

    pub fn index_epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    pub fn is_scanning(&self) -> bool {
        self.scanning.load(Ordering::Relaxed)
    }

    pub fn set_scanning(&self, on: bool) {
        self.scanning.store(on, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::model::{IndexBuilder, MediaKind};

    fn index_with(n: usize) -> Index {
        let mut b = IndexBuilder::new();
        let d = b.add_dir(r"D:\M", 0);
        for i in 0..n {
            b.add_file(&format!("f{i}.mp4"), MediaKind::Video, d, i as u64 + 1);
        }
        b.finish()
    }

    #[test]
    fn starts_empty_and_unloaded() {
        let s = AppState::new();
        assert!(s.snapshot().is_empty());
        assert!(!s.meta().loaded);
    }

    #[test]
    fn replace_updates_both_index_and_meta() {
        let s = AppState::new();
        s.replace(index_with(7), 1_700_000_000);

        assert_eq!(s.snapshot().len(), 7);
        let m = s.meta();
        assert!(m.loaded);
        assert_eq!(m.file_count, 7);
        assert_eq!(m.built_at_unix, 1_700_000_000);
        assert!(m.problem.is_none());
    }

    #[test]
    fn a_snapshot_survives_the_index_being_replaced_underneath_it() {
        // The property the whole design rests on: a search in flight keeps
        // reading valid memory even though the shared pointer moved on.
        let s = AppState::new();
        s.replace(index_with(3), 0);

        let held = s.snapshot();
        s.replace(index_with(100), 0);

        assert_eq!(held.len(), 3, "old snapshot must stay intact and readable");
        assert_eq!(s.snapshot().len(), 100, "new readers get the new index");
    }

    #[test]
    fn problem_state_clears_the_counts() {
        let s = AppState::new();
        s.replace(index_with(5), 0);
        s.set_problem("chưa có cache");

        let m = s.meta();
        assert!(!m.loaded);
        assert_eq!(m.file_count, 0);
        assert_eq!(m.problem.as_deref(), Some("chưa có cache"));
    }

    #[test]
    fn epoch_changes_whenever_the_index_does() {
        // Thumbnail URLs are only safe because of this: a stale request must
        // not resolve against a different file.
        let s = AppState::new();
        let first = s.index_epoch();
        s.replace(index_with(3), 0);
        let second = s.index_epoch();
        assert_ne!(first, second);

        s.replace(index_with(4), 0);
        assert_ne!(second, s.index_epoch());
    }

    #[test]
    fn generation_tracks_the_newest_query() {
        let s = AppState::new();
        s.begin_query(1);
        assert_eq!(s.generation().load(Ordering::Relaxed), 1);
        s.begin_query(42);
        assert_eq!(s.generation().load(Ordering::Relaxed), 42);
    }
}
