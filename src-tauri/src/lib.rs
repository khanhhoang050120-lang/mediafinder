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

/// Indexer mode (`--index`). Runs elevated, scans NTFS volumes, writes the
/// cache, then exits. Implemented in P1.
pub fn run_indexer() {
    tracing::info!("indexer mode");
    unimplemented!("NTFS enumeration lands in P1");
}
