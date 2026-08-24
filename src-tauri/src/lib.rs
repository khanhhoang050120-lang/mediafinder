//! MediaFinder — instant media file search for Windows/NTFS.
//!
//! Layering:
//!   `ntfs`   — Win32 volume access and MFT/USN enumeration (phase 1 + 2)
//!   `index`  — the in-memory database, folding, and the search algorithm
//!   `media`  — thumbnails, metadata enrichment, duplicate detection
//!   `ipc`    — Tauri commands, the `thumb://` protocol, elevation plumbing
//!   `state`  — shared application state (ArcSwap index + search generation)

use std::sync::atomic::{AtomicBool, Ordering};

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
    use state::AppState;

    tracing::info!("starting GUI");
    let app_state = AppState::new();

    // Load whatever the last scan left behind. Failing to find a cache is an
    // ordinary first-run condition, not an error — the UI says so and offers
    // to scan, rather than the window refusing to open.
    match index::persist::load() {
        Ok(cache) => {
            tracing::info!(
                "nạp cache: {} tệp, {} thư mục",
                cache.index.len(),
                cache.index.dir_count()
            );
            app_state.replace(cache.index, cache.built_at_unix);
        }
        Err(e) => {
            tracing::warn!("không nạp được cache: {e}");
            app_state.set_problem(e.to_string());
        }
    }

    // Start reading media properties for whatever the cache holds. This runs
    // for tens of minutes on a large library, so it begins now rather than
    // when the user first presses a filter.
    let enrich = media::enrich::EnrichService::new();
    enrich.start(app_state.snapshot());

    tauri::Builder::default()
        // Single instance first: it must intercept a second launch before the
        // rest of the setup runs, or two copies would race to read the same
        // cache and register the same hotkey.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Someone launched the app again — or pressed the hotkey while a
            // copy was already open. Show what they already have.
            summon(app);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            register_hotkey(app.handle());
            Ok(())
        })
        .manage(app_state)
        .manage(media::thumbnail::ThumbnailService::new())
        .manage(enrich)
        .manage(media::dupes::DupeService::new())
        .register_asynchronous_uri_scheme_protocol(
            ipc::protocol::SCHEME,
            |ctx, request, responder| ipc::protocol::handle(ctx.app_handle(), request, responder),
        )
        .invoke_handler(tauri::generate_handler![
            ipc::commands::search,
            ipc::commands::index_status,
            ipc::commands::open_file,
            ipc::commands::reveal_in_explorer,
            ipc::commands::request_scan,
            ipc::commands::scan_progress,
            ipc::commands::reload_index,
            ipc::commands::enrich_status,
            ipc::commands::find_duplicates,
            ipc::commands::dupe_progress,
            ipc::commands::dupe_groups,
            ipc::commands::hotkey_status,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Tauri application");
}

/// The hotkey that brings the window forward from anywhere.
///
/// `Ctrl+Alt+Space` rather than something shorter: a global shortcut is taken
/// away from every other application on the machine, so it has to be one
/// nothing else is likely to want. Plain `Alt+Space` is the Windows system
/// menu, and `Ctrl+Space` belongs to input-method switching in several
/// languages — including Vietnamese.
pub const HOTKEY: &str = "Ctrl+Alt+Space";

/// Whether [`HOTKEY`] is actually ours.
///
/// A global shortcut is a process-wide OS resource, claimed once at startup and
/// never given back, so a process-wide flag is an honest way to hold the answer.
/// The UI reads it before offering the key: a hint that names a shortcut which
/// does nothing is worse than no hint at all.
pub static HOTKEY_ACTIVE: AtomicBool = AtomicBool::new(false);

fn register_hotkey(app: &tauri::AppHandle) {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let handle = app.clone();
    let result = app.global_shortcut().on_shortcut(HOTKEY, move |_app, _sc, event| {
        // Fire on press only. Without this the window would toggle twice per
        // keypress — once down, once up — and end up back where it started.
        if event.state == ShortcutState::Pressed {
            toggle(&handle);
        }
    });

    match result {
        Ok(()) => {
            HOTKEY_ACTIVE.store(true, Ordering::Relaxed);
            tracing::info!("phím tắt toàn cục: {HOTKEY}");
        }
        // Another application already owns this combination. Not fatal — the
        // window still works, it just cannot be summoned — so say so and carry
        // on rather than refusing to start. The flag stays false and the UI
        // stops advertising the key.
        Err(e) => tracing::warn!("không đăng ký được phím tắt {HOTKEY}: {e}"),
    }
}

/// Bring the window to the front, focused and ready to type into.
fn summon(app: &tauri::AppHandle) {
    use tauri::Manager;

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    // Unminimise before showing: a minimised window that is merely shown stays
    // in the taskbar.
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();

    // Focusing the window is not the same as focusing the search box: the caret
    // could be anywhere in the page, or nowhere at all if the window was
    // hidden. A launcher whose hotkey does not leave you able to type is not
    // doing its job, so tell the UI to put the caret back where it belongs.
    //
    // A dedicated event rather than a plain window-focus listener: this fires
    // only when the user asked for the window, not every time they alt-tab back
    // to it mid-edit, which would select and then wipe whatever they had typed.
    use tauri::Emitter;
    let _ = window.emit("summon", ());
}

/// Show the window, or hide it if it is already the one in front.
///
/// Toggling matters for a launcher: the same key that summons it should
/// dismiss it, so it can be opened and closed without the hand leaving the
/// keyboard.
fn toggle(app: &tauri::AppHandle) {
    use tauri::Manager;

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);

    if visible && focused {
        let _ = window.hide();
    } else {
        summon(app);
    }
}

/// Indexer mode (`--index`). Runs elevated, scans every NTFS volume, and — from
/// P4 onwards — writes the result to the on-disk cache.
///
/// `--dry-run` stops after reporting statistics and sample paths, which is how
/// the scan is validated without needing the rest of the pipeline.
/// Update the cache from the change journals instead of rescanning.
///
/// A full scan walks every MFT record on every volume — 38 seconds on this
/// machine, most of it spent reading four million records to find out that
/// almost nothing moved. The journal already knows what moved.
///
/// Returns `false` when the incremental path cannot be used, and the caller
/// falls back to a full scan. That happens for ordinary reasons — no cache
/// yet, a schema change, a journal that wrapped while the machine was off —
/// so it is a normal outcome, not a failure.
///
/// Needs Administrator, like every direct volume read (CHECK-004).
pub fn run_incremental() -> bool {
    use index::update::{rebuild_with, Change};
    use ntfs::usn_journal::{self, Batch, Cursor};
    use ntfs::volume;
    use rayon::prelude::*;

    let started = std::time::Instant::now();

    let cache = match index::persist::load() {
        Ok(c) => c,
        Err(e) => {
            tracing::info!("không dùng được cập nhật nhanh: {e}");
            return false;
        }
    };
    if cache.index.is_empty() || cache.volumes.is_empty() {
        tracing::info!("không dùng được cập nhật nhanh: cache rỗng");
        return false;
    }

    let by_letter: std::collections::HashMap<char, _> = volume::list_volumes()
        .into_iter()
        .map(|v| (v.letter, v))
        .collect();

    let mut changes: Vec<Change> = Vec::new();
    let mut stamps: Vec<index::persist::VolumeStamp> = Vec::new();

    for st in &cache.volumes {
        let Some(info) = by_letter.get(&st.letter) else {
            // The drive is not attached right now. Leaving its entries in the
            // index is the right call — they will still be there when it comes
            // back, and dropping them would look like the files were deleted.
            tracing::warn!("ổ {} không còn gắn, giữ nguyên phần đã có", st.letter);
            stamps.push(st.clone());
            continue;
        };
        let handle = match volume::open_volume(info) {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("{e}");
                return false;
            }
        };

        let from = Cursor {
            journal_id: st.journal_id,
            next_usn: st.next_usn,
        };
        match usn_journal::read_batch(&handle, st.letter, from) {
            Ok(Batch::Changes { changes: c, next }) => {
                tracing::info!("ổ {}: {} thay đổi từ journal", st.letter, c.len());
                changes.extend(c);
                let mut updated = st.clone();
                updated.next_usn = next.next_usn;
                stamps.push(updated);
            }
            Ok(Batch::Restart(r)) => {
                // Not an error. The journal simply cannot answer, so the only
                // honest thing left is to look at the volume itself.
                tracing::warn!("{}", r.message(st.letter));
                return false;
            }
            Err(e) => {
                tracing::warn!("ổ {}: đọc journal thất bại: {e}", st.letter);
                return false;
            }
        }
    }

    if changes.is_empty() {
        tracing::info!(
            "không có thay đổi nào — chỉ mục vẫn đúng [{:.2}s]",
            started.elapsed().as_secs_f64()
        );
        // Still worth saving: the cursors moved forward even with nothing to
        // apply, and not saving them would re-read the same stretch of journal
        // every time.
        let _ = index::persist::save(&cache.index, stamps);
        return true;
    }

    let (mut ix, stats) = rebuild_with(&cache.index, &changes);
    tracing::info!(
        "áp thay đổi: +{} tệp, -{} tệp, {} tệp chuyển chỗ, +{} thư mục, -{} thư mục, \
         {} thư mục đổi tên, {} bỏ qua (ngoài phạm vi index)",
        stats.files_added,
        stats.files_removed,
        stats.files_moved,
        stats.dirs_added,
        stats.dirs_removed,
        stats.dirs_renamed,
        stats.unresolved
    );

    // Only the entries the journal could not describe need a disk lookup: a
    // journal record carries no size, so a newly created file arrives at zero
    // bytes. Everything else kept the figures already measured for it.
    let missing: Vec<u32> = (0..ix.len() as u32)
        .filter(|&i| ix.mtime(i as usize) == 0)
        .collect();
    if !missing.is_empty() {
        let stats_started = std::time::Instant::now();
        let found: Vec<(u32, media::metadata::FileStats)> = missing
            .par_iter()
            .map(|&i| {
                (
                    i,
                    media::metadata::file_stats(&ix.full_path(i as usize)).unwrap_or_default(),
                )
            })
            .collect();
        let mut sizes: Vec<u64> = (0..ix.len()).map(|i| ix.size(i)).collect();
        let mut mtimes: Vec<i64> = (0..ix.len()).map(|i| ix.mtime(i)).collect();
        for (i, st) in found {
            sizes[i as usize] = st.size;
            mtimes[i as usize] = st.mtime;
        }
        ix.set_file_stats(sizes, mtimes);
        tracing::info!(
            "đọc dung lượng cho {} mục mới [{:.2}s]",
            missing.len(),
            stats_started.elapsed().as_secs_f64()
        );
    }

    match index::persist::save(&ix, stamps) {
        Ok(path) => tracing::info!(
            "cập nhật nhanh xong: {} mục, ghi {} [{:.2}s]",
            ix.len(),
            path.display(),
            started.elapsed().as_secs_f64()
        ),
        Err(e) => {
            tracing::error!("không ghi được cache: {e}");
            return false;
        }
    }
    true
}

/// Read back the journal from each cursor the scan just recorded.
///
/// The cursor is captured *before* a volume is enumerated, and enumerating
/// every volume on a machine takes the better part of a minute — so by the
/// time this runs there has almost certainly been real file activity to
/// report. That makes it a genuine end-to-end check of the journal reader
/// against a live volume, for free, inside a process that is already elevated.
///
/// Nothing here changes the index. It only says, in the log, whether the
/// incremental path would have worked.
fn check_journal_cursors(stamps: &[index::persist::VolumeStamp]) {
    use ntfs::usn_journal::{self, Batch, Cursor};
    use ntfs::volume;

    let by_letter: std::collections::HashMap<char, _> = volume::list_volumes()
        .into_iter()
        .map(|v| (v.letter, v))
        .collect();

    for st in stamps {
        if st.journal_id == 0 {
            tracing::info!("ổ {}: không có USN journal, bỏ qua tự kiểm tra", st.letter);
            continue;
        }
        let Some(info) = by_letter.get(&st.letter) else {
            continue;
        };
        let Ok(handle) = volume::open_volume(info) else {
            continue;
        };

        let from = Cursor {
            journal_id: st.journal_id,
            next_usn: st.next_usn,
        };
        let started = std::time::Instant::now();
        match usn_journal::read_batch(&handle, st.letter, from) {
            Ok(Batch::Changes { changes, next }) => {
                tracing::info!(
                    "ổ {}: tự kiểm tra journal — {} thay đổi kể từ usn={} (nay {}) [{:.0}ms]",
                    st.letter,
                    changes.len(),
                    st.next_usn,
                    next.next_usn,
                    started.elapsed().as_secs_f64() * 1000.0
                );
                // A few examples, because a count alone cannot show that the
                // names and reference numbers came out the right way round.
                for c in changes.iter().take(3) {
                    tracing::info!("    {}", describe(c));
                }
            }
            Ok(Batch::Restart(r)) => tracing::warn!("{}", r.message(st.letter)),
            Err(e) => tracing::warn!("ổ {}: tự kiểm tra journal thất bại: {e}", st.letter),
        }
    }
}

/// Follow the change journal and print what it says.
///
/// A developer path, not a feature: the journal reader is unit-tested against
/// hand-built records, which proves it parses the layout but says nothing
/// about whether a real volume behaves the way the documentation claims.
/// Needs Administrator, like every other direct volume read.
///
/// ```text
/// mediafinder.exe --watch          # every NTFS volume
/// mediafinder.exe --watch D        # just one
/// ```
pub fn run_watch(args: &[String]) {
    use ntfs::usn_journal::{self, Batch, Cursor};
    use ntfs::volume;

    // A bare letter after --watch, if given.
    let only: Option<char> = args
        .iter()
        .skip_while(|a| *a != "--watch")
        .nth(1)
        .and_then(|a| a.chars().next())
        .map(|c| c.to_ascii_uppercase());

    let volumes: Vec<_> = volume::list_volumes()
        .into_iter()
        .filter(|v| v.is_scannable())
        .filter(|v| only.map_or(true, |c| v.letter.to_ascii_uppercase() == c))
        .collect();

    if volumes.is_empty() {
        tracing::error!("không có ổ NTFS nào để theo dõi");
        return;
    }

    // Start from where each volume is *now*, not from the stored cursor: this
    // is for watching changes happen, so anything already in the journal is
    // history and would only bury what the user is about to do.
    let mut watched = Vec::new();
    for v in volumes {
        let handle = match volume::open_volume(&v) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("{e}");
                continue;
            }
        };
        match volume::query_journal(&handle) {
            Ok(j) => {
                tracing::info!(
                    "theo dõi ổ {}: journal_id={:#x} bắt đầu từ usn={}",
                    v.letter,
                    j.journal_id,
                    j.next_usn
                );
                watched.push((
                    v.letter,
                    handle,
                    Cursor {
                        journal_id: j.journal_id,
                        next_usn: j.next_usn,
                    },
                ));
            }
            Err(e) => tracing::error!("{e}"),
        }
    }
    if watched.is_empty() {
        return;
    }

    tracing::info!("Ctrl+C để dừng. Hãy thử tạo, đổi tên, xoá một tệp media.");

    loop {
        for (letter, handle, cursor) in watched.iter_mut() {
            match usn_journal::read_batch(handle, *letter, *cursor) {
                Ok(Batch::Changes { changes, next }) => {
                    *cursor = next;
                    for c in &changes {
                        tracing::info!("{}", describe(c));
                    }
                }
                Ok(Batch::Restart(r)) => {
                    tracing::warn!("{}", r.message(*letter));
                    return;
                }
                Err(e) => {
                    tracing::error!("{e}");
                    return;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn describe(c: &index::update::Change) -> String {
    use index::update::Change;
    match c {
        Change::Gone { volume, frn } => {
            format!("{}: XOÁ      frn={frn}", *volume as char)
        }
        Change::Present {
            volume,
            frn,
            parent_frn,
            name,
            is_dir,
        } => format!(
            "{}: {} frn={frn} cha={parent_frn} {name}",
            *volume as char,
            if *is_dir { "THƯ MỤC " } else { "CÓ MẶT  " }
        ),
    }
}

pub fn run_indexer() {
    use index::model::IndexBuilder;
    use ntfs::{tree, usn_enum, volume};
    use rayon::prelude::*;

    let dry_run = std::env::args().any(|a| a == "--dry-run");
    let full = std::env::args().any(|a| a == "--full");
    tracing::info!(dry_run, full, "indexer starting");

    // Try the journal first. It answers in well under a second when it can,
    // and a full scan is 38 — but it cannot always answer, so this is an
    // attempt rather than a decision.
    if !dry_run && !full && run_incremental() {
        return;
    }

    let mut builder = IndexBuilder::new();

    // Progress goes to a file the GUI polls. `--dry-run` is a developer path
    // with no GUI watching, so it writes nothing.
    let mut progress = if dry_run {
        None
    } else {
        match ipc::elevate::ProgressWriter::new() {
            Ok(w) => Some(w),
            Err(e) => {
                tracing::warn!("không mở được tệp tiến độ: {e}");
                None
            }
        }
    };

    let volumes = volume::list_volumes();
    if volumes.is_empty() {
        tracing::error!("không tìm thấy ổ đĩa nào");
        return;
    }

    // Non-NTFS volumes are reported rather than silently skipped, so a user
    // whose USB stick is missing from the results knows why.
    for v in &volumes {
        if let Some(why) = v.skip_reason() {
            tracing::warn!("bỏ qua ổ {}: {}", v.letter, why);
        }
    }

    let ntfs_volumes: Vec<_> = volumes.iter().filter(|v| v.is_scannable()).collect();
    if ntfs_volumes.is_empty() {
        tracing::error!("không có ổ NTFS nào để quét");
        if let Some(p) = progress.as_mut() {
            let st = p.state_mut();
            st.phase = "error".into();
            st.error = Some("Không tìm thấy ổ NTFS nào để quét.".into());
            st.finished = true;
            p.flush();
        }
        return;
    }

    if let Some(p) = progress.as_mut() {
        let st = p.state_mut();
        st.phase = "volumes".into();
        st.volumes_total = ntfs_volumes.len();
        st.message = format!("Chuẩn bị quét {} ổ đĩa…", ntfs_volumes.len());
        p.flush();
    }

    let opts = tree::ResolveOptions::default();
    let mut grand_total = 0usize;
    let mut stamps: Vec<index::persist::VolumeStamp> = Vec::new();
    // Counted so a scan that reached no volume at all can refuse to save.
    let mut volumes_ok = 0usize;
    let volumes_total = ntfs_volumes.len();

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
        let journal = match volume::query_journal(&handle) {
            Ok(j) => {
                tracing::info!("USN journal id={:#x} next_usn={}", j.journal_id, j.next_usn);
                Some(j)
            }
            Err(e) => {
                tracing::warn!("{e}");
                None
            }
        };

        if let Some(p) = progress.as_mut() {
            let st = p.state_mut();
            st.phase = "scanning".into();
            st.volume = v.letter.to_string();
            st.records = 0;
            st.message = format!("Đang đọc bảng tệp của ổ {}:…", v.letter);
            p.flush();
        }

        let scan_started = std::time::Instant::now();
        let mut reporter = progress.take();
        let (records, scan_stats) = match usn_enum::enumerate(&handle, |n| {
            tracing::info!("  … đã đọc {n} bản ghi");
            if let Some(p) = reporter.as_mut() {
                let st = p.state_mut();
                st.records = n;
                st.message = format!("Ổ {}: đã đọc {} bản ghi", v.letter, n);
                p.tick();
            }
        }) {
            Ok(r) => {
                progress = reporter;
                r
            }
            Err(e) => {
                tracing::error!("{e}");
                progress = reporter;
                continue;
            }
        };
        let scan_time = scan_started.elapsed();

        if let Some(p) = progress.as_mut() {
            let st = p.state_mut();
            st.phase = "resolving".into();
            st.message = format!("Ổ {}: đang dựng lại đường dẫn…", v.letter);
            p.flush();
        }

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

        // Fold into the shared index. Directory ids are per-volume, so each
        // one is re-registered and remapped as it is added.
        let build_started = std::time::Instant::now();
        builder.reserve(set.dirs.len(), set.files.len());
        let remap: Vec<u32> = set
            .dirs
            .iter()
            .zip(&set.dir_frns)
            .map(|(d, &frn)| builder.add_dir(d, frn))
            .collect();
        for f in &set.files {
            builder.add_file(&f.name, f.kind, remap[f.dir_id as usize], f.frn);
        }
        tracing::info!(
            "nạp vào index: {:.2}s",
            build_started.elapsed().as_secs_f64()
        );

        if let Some(p) = progress.as_mut() {
            let st = p.state_mut();
            st.volumes_done += 1;
            st.media_files += set.files.len() as u64;
            st.phase = "indexing".into();
            st.message = format!(
                "Xong ổ {}: {} tệp media",
                v.letter,
                set.files.len()
            );
            p.flush();
        }

        volumes_ok += 1;
        stamps.push(index::persist::VolumeStamp {
            letter: v.letter,
            serial: v.serial,
            journal_id: journal.map(|j| j.journal_id).unwrap_or(0),
            next_usn: journal.map(|j| j.next_usn).unwrap_or(0),
            file_count: set.files.len(),
        });
    }

    let mut ix = builder.finish();

    // Fast metadata pass: size and modification time for every entry.
    //
    // `GetFileAttributesEx` reads the MFT record without opening the file, and
    // the scan has just walked that same table, so the records are still warm
    // in the cache. Parallel because the calls are independent and each is
    // short enough that thread overhead would otherwise dominate.
    if !ix.is_empty() {
        if let Some(p) = progress.as_mut() {
            let st = p.state_mut();
            st.phase = "stats".into();
            st.message = format!("Đang đọc dung lượng {} tệp…", ix.len());
            p.flush();
        }

        let stats_started = std::time::Instant::now();
        let paths: Vec<String> = (0..ix.len()).map(|i| ix.full_path(i)).collect();
        let stats: Vec<media::metadata::FileStats> = paths
            .par_iter()
            .map(|p| media::metadata::file_stats(p).unwrap_or_default())
            .collect();

        let total_bytes: u64 = stats.iter().map(|s| s.size).sum();
        ix.set_file_stats(
            stats.iter().map(|s| s.size).collect(),
            stats.iter().map(|s| s.mtime).collect(),
        );
        tracing::info!(
            "đọc dung lượng: {} tệp, tổng {:.1} GB [{:.2}s]",
            stats.len(),
            total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
            stats_started.elapsed().as_secs_f64()
        );
    }

    tracing::info!(
        "TỔNG: {grand_total} tệp media · index {} mục / {} thư mục · RAM ~{:.1} MB",
        ix.len(),
        ix.dir_count(),
        ix.memory_bytes() as f64 / (1024.0 * 1024.0)
    );

    if dry_run {
        demo_searches(&ix);
        tracing::info!("--dry-run: không ghi cache");
        return;
    }

    // Never replace a working cache with nothing.
    //
    // Every volume that fails to open is skipped and the scan carries on, so
    // if *all* of them fail — no elevation, a locked disk, USN disabled —
    // execution still arrives here holding an empty index. Saving it would
    // silently destroy a perfectly good cache and force the user through
    // another scan, with another UAC prompt, to get back what they had.
    //
    // A scan that reached nothing has nothing to say; leave the old cache
    // exactly where it is.
    if volumes_ok == 0 || ix.is_empty() {
        let why = format!(
            "Không quét được ổ đĩa nào ({volumes_total} ổ NTFS đều thất bại). Chỉ mục cũ được giữ nguyên."
        );
        tracing::error!("{why}");
        if let Some(p) = progress.as_mut() {
            let st = p.state_mut();
            st.phase = "error".into();
            st.error = Some(why);
            st.message = "Không quét được ổ đĩa nào".into();
            st.finished = true;
            p.flush();
        }
        return;
    }

    if volumes_ok < volumes_total {
        tracing::warn!("chỉ quét được {volumes_ok}/{volumes_total} ổ đĩa");
    }

    if let Some(p) = progress.as_mut() {
        let st = p.state_mut();
        st.phase = "saving".into();
        st.message = "Đang lưu chỉ mục…".into();
        p.flush();
    }

    // Prove the stored cursors are usable before anything comes to depend on
    // them. Free to do here and impossible to do anywhere else: reading the
    // journal needs Administrator (measured — see `docs/check.md` CHECK-004),
    // and this is the one process that has it.
    check_journal_cursors(&stamps);

    let outcome = index::persist::save(&ix, stamps);
    match &outcome {
        Ok(path) => tracing::info!("đã ghi cache: {}", path.display()),
        Err(e) => tracing::error!("không ghi được cache: {e}"),
    }

    // `finished` is set only now, after the cache is safely on disk. The GUI
    // reloads the moment it sees that flag, so flipping it any earlier would
    // race the file it is about to read.
    if let Some(p) = progress.as_mut() {
        let st = p.state_mut();
        match outcome {
            Ok(_) => {
                st.phase = "done".into();
                st.message = if volumes_ok < volumes_total {
                    format!(
                        "Đã lập chỉ mục {} tệp media (chỉ quét được {}/{} ổ)",
                        ix.len(),
                        volumes_ok,
                        volumes_total
                    )
                } else {
                    format!("Đã lập chỉ mục {} tệp media", ix.len())
                };
            }
            Err(e) => {
                st.phase = "error".into();
                st.error = Some(e.to_string());
                st.message = "Không lưu được chỉ mục".into();
            }
        }
        st.finished = true;
        p.flush();
    }
}

/// Run a few searches against the freshly built index.
///
/// The point is the Vietnamese cases: proving on real filenames, not synthetic
/// test data, that a query typed without diacritics finds names that have them.
fn demo_searches(ix: &index::model::Index) {
    use index::search::{search, SearchOptions};
    use std::sync::atomic::AtomicU64;

    if ix.is_empty() {
        return;
    }
    let cancel = AtomicU64::new(0);
    let opts = SearchOptions {
        limit: 5,
        ..Default::default()
    };

    // Queries chosen from directory names that actually exist on the scanned
    // volumes, typed *without* diacritics. Guessing at plausible-sounding
    // Vietnamese words proves nothing: an empty result set is impossible to
    // tell apart from a broken fold.
    tracing::info!("--- thử tìm kiếm (gõ KHÔNG dấu, dữ liệu CÓ dấu) ---");
    for query in ["nhac nen", "nang dong", "bai", "hung", "screenshot"] {
        let started = std::time::Instant::now();
        let outcome = search(ix, query, &opts, &[], &cancel, 0);
        let elapsed = started.elapsed();
        let note = match outcome.relaxed {
            Some(r) => format!(" (khớp một phần {}/{} từ)", r.best_matched, r.total_tokens),
            None => String::new(),
        };
        tracing::info!(
            "  \"{}\" → {} kết quả trong {:.2}ms{}",
            query,
            outcome.hits.len(),
            elapsed.as_secs_f64() * 1000.0,
            note
        );
        for h in outcome.hits.iter().take(3) {
            tracing::info!(
                "      [{}] {}",
                h.score,
                ix.full_path(h.index as usize)
            );
        }
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

#[cfg(test)]
mod tests {
    use super::HOTKEY;

    /// Locks the reasoning behind the combination rather than the combination
    /// itself — changing the key is fine, dropping one of these modifiers is
    /// not.
    ///
    /// `Alt+Space` alone opens the Windows system menu on every window, and
    /// `Ctrl+Space` is the input-method switch in several languages, Vietnamese
    /// among them. Either would work on the test machine and then quietly fight
    /// the operating system on a user's.
    #[test]
    fn the_hotkey_avoids_combinations_windows_and_the_ime_already_use() {
        assert!(
            HOTKEY.contains("Ctrl") && HOTKEY.contains("Alt"),
            "phím tắt {HOTKEY} phải giữ cả Ctrl lẫn Alt: thiếu Ctrl thì đụng \
             menu hệ thống của Windows, thiếu Alt thì đụng phím chuyển bộ gõ"
        );
    }
}
