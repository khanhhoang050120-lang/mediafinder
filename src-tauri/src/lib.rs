//! MediaFinder — instant media file search for Windows/NTFS.
//!
//! Layering:
//!   `ntfs`   — Win32 volume access and MFT/USN enumeration (phase 1 + 2)
//!   `index`  — the in-memory database, folding, and the search algorithm
//!   `media`  — thumbnails, metadata enrichment, duplicate detection
//!   `ipc`    — Tauri commands, the `thumb://` protocol, elevation plumbing
//!   `state`  — shared application state (ArcSwap index + search generation)

pub mod index;
pub mod ipc;
pub mod media;
pub mod ntfs;
pub mod state;

/// Initialise tracing. Verbosity is controlled by `RUST_LOG`.
pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("mediafinder=info,warn"));
    let _ = fmt().with_env_filter(filter).try_init();
}

/// GUI mode. Runs unelevated; loads the index from the on-disk cache.
pub fn run_gui() {
    tracing::info!("starting GUI");
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to start Tauri application");
}

/// Indexer mode (`--index`). Runs elevated, scans every NTFS volume, and — from
/// P4 onwards — writes the result to the on-disk cache.
///
/// `--dry-run` stops after reporting statistics and sample paths, which is how
/// the scan is validated without needing the rest of the pipeline.
pub fn run_indexer() {
    use ntfs::{tree, usn_enum, volume};

    let dry_run = std::env::args().any(|a| a == "--dry-run");
    tracing::info!(dry_run, "indexer starting");

    let volumes = volume::list_volumes();
    if volumes.is_empty() {
        tracing::error!("không tìm thấy ổ đĩa nào");
        return;
    }

    // Non-NTFS volumes are reported rather than silently skipped, so a user
    // whose USB stick is missing from the results knows why.
    for v in volumes.iter().filter(|v| !v.is_ntfs()) {
        tracing::warn!(
            "bỏ qua ổ {}: ({}) — không phải NTFS nên không có MFT/USN",
            v.letter,
            v.filesystem
        );
    }

    let ntfs_volumes: Vec<_> = volumes.iter().filter(|v| v.is_ntfs()).collect();
    if ntfs_volumes.is_empty() {
        tracing::error!("không có ổ NTFS nào để quét");
        return;
    }

    let opts = tree::ResolveOptions::default();
    let mut grand_total = 0usize;

    for v in ntfs_volumes {
        let started = std::time::Instant::now();
        tracing::info!("=== ổ {}: ({}) ===", v.letter, v.label);

        let handle = match volume::open_volume(v) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("{e}");
                continue;
            }
        };

        // Informational only: the full scan uses FSCTL_ENUM_USN_DATA, which
        // walks the MFT directly and works with the journal disabled. The USN
        // is recorded so P8 can pick up incremental changes from here.
        match volume::query_journal(&handle) {
            Ok(j) => tracing::info!("USN journal id={:#x} next_usn={}", j.journal_id, j.next_usn),
            Err(e) => tracing::warn!("{e}"),
        }

        let scan_started = std::time::Instant::now();
        let (records, scan_stats) = match usn_enum::enumerate(&handle, |n| {
            tracing::info!("  … đã đọc {n} bản ghi");
        }) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("{e}");
                continue;
            }
        };
        let scan_time = scan_started.elapsed();

        let resolve_started = std::time::Instant::now();
        let set = tree::resolve(records, v.letter, &opts);
        let resolve_time = resolve_started.elapsed();

        tracing::info!(
            "pha 1: {} bản ghi ({} thư mục, {} tệp) → giữ {} tệp media  [{:.2}s]",
            scan_stats.records_seen,
            scan_stats.directories,
            scan_stats.files_seen,
            scan_stats.media_kept,
            scan_time.as_secs_f64()
        );
        if scan_stats.malformed > 0 || scan_stats.wrong_version > 0 {
            tracing::warn!(
                "pha 1: {} bản ghi hỏng, {} bản ghi sai phiên bản",
                scan_stats.malformed,
                scan_stats.wrong_version
            );
        }

        let s = set.stats;
        tracing::info!(
            "pha 2: {} thư mục → {} thư mục duy nhất; giữ {} / loại {} (thư mục cấm) [{:.2}s]",
            s.directories_seen,
            set.dirs.len(),
            s.kept,
            s.excluded,
            resolve_time.as_secs_f64()
        );
        if s.orphaned > 0 || s.cycles > 0 || s.too_deep > 0 {
            tracing::warn!(
                "pha 2: {} mồ côi, {} vòng lặp, {} quá sâu",
                s.orphaned,
                s.cycles,
                s.too_deep
            );
        }
        tracing::info!(
            "ổ {}: xong sau {:.2}s",
            v.letter,
            started.elapsed().as_secs_f64()
        );

        if dry_run {
            print_samples(&set);
        }
        grand_total += set.files.len();
    }

    tracing::info!("TỔNG: {grand_total} tệp media");
    if dry_run {
        tracing::info!("--dry-run: không ghi cache (ghi cache thuộc giai đoạn P4)");
    }
}

/// Print a spread of resolved paths for eyeballing against Explorer.
///
/// Sampled across the whole set rather than taking the first N, because the
/// first N all live in the same directory and would prove nothing about deep
/// or unusual paths.
fn print_samples(set: &ntfs::tree::ResolvedSet) {
    const WANTED: usize = 20;
    if set.files.is_empty() {
        tracing::info!("(không có tệp media nào để lấy mẫu)");
        return;
    }

    let step = (set.files.len() / WANTED).max(1);
    tracing::info!("--- {} đường dẫn mẫu ---", WANTED.min(set.files.len()));
    for f in set.files.iter().step_by(step).take(WANTED) {
        tracing::info!(
            "  [{}] {}\\{}",
            f.kind.as_str(),
            set.dirs[f.dir_id as usize],
            f.name
        );
    }

    // The deepest path is where resolution is most likely to be wrong, so it
    // is always worth showing regardless of sampling.
    if let Some(deepest) = set
        .files
        .iter()
        .max_by_key(|f| set.dirs[f.dir_id as usize].matches('\\').count())
    {
        tracing::info!(
            "--- đường dẫn sâu nhất ---\n  {}\\{}",
            set.dirs[deepest.dir_id as usize],
            deepest.name
        );
    }
}
