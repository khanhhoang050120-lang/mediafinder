//! MediaFinder — instant media file search for Windows/NTFS.
//!
//! Layering:
//!   `ntfs`   — Win32 volume access and MFT/USN enumeration (phase 1 + 2)
//!   `index`  — the in-memory database, folding, and the search algorithm
//!   `media`  — thumbnails, metadata enrichment, duplicate detection
//!   `ipc`    — Tauri commands, the `thumb://` protocol, elevation plumbing
//!   `state`  — shared application state (ArcSwap index + search generation)
//!   `update` — checking whether a newer release exists

use std::sync::atomic::{AtomicBool, Ordering};

pub mod diag;
pub mod index;
pub mod ipc;
pub mod lastcheck;
pub mod media;
pub mod misslog;
pub mod netscan_mark;
pub mod ntfs;
pub mod preflight;
pub mod setup;
pub mod state;
pub mod taskhealth;
pub mod update;
pub mod walk;

/// Initialise tracing. Verbosity is controlled by `RUST_LOG`.
pub fn init_tracing() {
    // Chi tiết nằm ở module `diag`: stderr cho phiên dev, file cho bản đã
    // cài — nơi trước đây mọi chẩn đoán đều là suy luận chay vì log không
    // đi đâu cả.
    diag::init();
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            use tauri::Manager;

            // The window is declared invisible in `tauri.conf.json` and shown
            // here instead. Declaring it visible and hiding it afterwards
            // would flash a window on the screen at every login, which is
            // exactly what starting minimised is meant to avoid.
            let quiet = std::env::args().any(|a| a == "--minimized");
            if let Some(window) = app.get_webview_window("main") {
                if quiet {
                    tracing::info!("khởi động ẩn: bấm {HOTKEY} để gọi cửa sổ");
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // Needs no privilege: a file in the user's own profile. Checked
            // rather than rewritten, so someone who deletes it keeps that
            // decision.
            setup::ensure_startup_shortcut();

            register_hotkey(app.handle());
            watch_cache(app.handle());
            build_tray(app.handle());

            // Sau `build_tray`: nếu có bản mới thì tin báo đi vào tooltip của
            // khay, và cái khay đó phải tồn tại trước đã.
            //
            // Chạy cả khi khởi động ẩn — lúc đăng nhập là dịp tốt nhất để hỏi,
            // và vì phần này chỉ báo tin chứ không tự tải, nó không làm phiền
            // ai: không có hộp thoại nào bật lên từ hư không.
            update::check_in_background(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window must not end the process.
            //
            // The hotkey lives in this process; killing it takes the hotkey
            // with it, so tidying the window away would quietly disable the
            // main way back in. Hiding keeps the program reachable and costs
            // 45 MB of an idle process.
            //
            // Only reached for the window's own close button. A shutdown ends
            // the session rather than asking each window to close, so this
            // cannot hold up a restart.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .manage(app_state)
        .manage(media::thumbnail::ThumbnailService::new())
        .manage(enrich)
        .manage(media::dupes::DupeService::new())
        .register_asynchronous_uri_scheme_protocol(
            ipc::protocol::SCHEME,
            |ctx, request, responder| ipc::protocol::handle(ctx.app_handle(), request, responder),
        )
        .register_asynchronous_uri_scheme_protocol(
            ipc::media_stream::SCHEME,
            |ctx, request, responder| {
                ipc::media_stream::handle(ctx.app_handle(), request, responder)
            },
        )
        .invoke_handler(tauri::generate_handler![
            ipc::commands::search,
            ipc::commands::index_status,
            ipc::commands::open_file,
            ipc::commands::net_scan_mark,
            taskhealth::task_health,
            lastcheck::last_check,
            ipc::commands::verify_dupe_group,
            ipc::commands::miss_log_status,
            ipc::commands::miss_log_set_enabled,
            ipc::commands::miss_log_clear,
            ipc::commands::miss_log_open,
            ipc::commands::open_releases_page,
            ipc::commands::start_file_drag,
            ipc::commands::reveal_in_explorer,
            ipc::commands::request_scan,
            ipc::commands::request_scan_with_network,
            ipc::commands::cancel_scan,
            ipc::commands::network_drives,
            ipc::commands::scan_progress,
            ipc::commands::reload_index,
            ipc::commands::enrich_status,
            ipc::commands::find_duplicates,
            ipc::commands::dupe_progress,
            ipc::commands::cancel_duplicates,
            ipc::commands::dupe_groups,
            ipc::commands::hotkey_status,
            ipc::commands::update_status,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Tauri application");
}

/// The tray icon, and the only way to actually quit.
///
/// Closing the window hides it, so without something in the tray there would
/// be no sign the program is still running and no way to stop it short of Task
/// Manager. The icon is that sign; "Thoát" is that way.
fn build_tray(app: &tauri::AppHandle) {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let open = MenuItemBuilder::with_id("open", format!("Mở MediaFinder  ({HOTKEY})"))
        .build(app)
        .ok();
    let logs = MenuItemBuilder::with_id("logs", "Xem nhật ký")
        .build(app)
        .ok();
    let quit = MenuItemBuilder::with_id("quit", "Thoát").build(app).ok();
    let (Some(open), Some(logs), Some(quit)) = (open, logs, quit) else {
        tracing::warn!("không dựng được mục menu khay hệ thống");
        return;
    };

    let menu = match MenuBuilder::new(app)
        .item(&open)
        .item(&logs)
        .separator()
        .item(&quit)
        .build()
    {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("không dựng được menu khay hệ thống: {e}");
            return;
        }
    };

    let result = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
            // Never expected: the icon is compiled in. Falling back to a blank
            // one still leaves something clickable in the tray, which beats
            // having no way to quit.
            tauri::image::Image::new_owned(vec![0; 4], 1, 1)
        }))
        .tooltip(format!("MediaFinder — {HOTKEY} để tìm kiếm"))
        .menu(&menu)
        // The menu belongs on right-click only. Left-click is the quick
        // gesture and should do the quick thing.
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => summon(app),
            // Mở thư mục nhật ký bằng Explorer — khi một trong 20–40 máy kia
            // gặp chuyện, câu trả lời nằm trong file thay vì một phiên đoán.
            "logs" => {
                if let Some(dir) = diag::logs_dir() {
                    let _ = std::fs::create_dir_all(&dir);
                    if let Err(e) =
                        crate::ipc::commands::shell::open_with_default_app(&dir.to_string_lossy())
                    {
                        tracing::warn!("không mở được thư mục nhật ký: {e}");
                    }
                }
            }
            "quit" => {
                tracing::info!("thoát theo yêu cầu từ khay hệ thống");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle(tray.app_handle());
            }
        })
        .build(app);

    match result {
        Ok(_) => tracing::info!("khay hệ thống: sẵn sàng"),
        // Not fatal, but it does mean closing the window would leave no way
        // back except the hotkey — so say so rather than fail silently.
        Err(e) => tracing::warn!(
            "không dựng được biểu tượng khay hệ thống: {e} — \
             đóng cửa sổ vẫn ẩn chứ không thoát, dùng {HOTKEY} để gọi lại"
        ),
    }
}

/// Reload the index when something else rewrites the cache.
///
/// Without this the automatic update is nearly useless. A scheduled task
/// refreshes the cache at login, and the window — which may have loaded it
/// seconds earlier, or may have been open for days — would go on searching
/// yesterday's index until it was restarted. The update would happen and
/// nobody would see it.
///
/// Watches the file's modification time rather than its contents: the cache is
/// written to a temporary file and renamed into place, so the timestamp only
/// ever moves when a complete new one has landed.
fn watch_cache(app: &tauri::AppHandle) {
    use tauri::Emitter;

    let Ok(path) = index::persist::cache_path() else {
        return;
    };
    let handle = app.clone();

    // Five seconds. The cache changes a handful of times a day at most, so
    // anything faster would be a timer that does nothing several million times
    // for every time it does something.
    const EVERY: std::time::Duration = std::time::Duration::from_secs(5);

    std::thread::spawn(move || {
        let mut seen = modified_at(&path);
        loop {
            std::thread::sleep(EVERY);
            let now = modified_at(&path);
            if now == seen || now.is_none() {
                continue;
            }
            seen = now;

            match index::persist::load() {
                Ok(cache) => {
                    tracing::info!(
                        "cache đã thay đổi bên ngoài — nạp lại: {} tệp",
                        cache.index.len()
                    );
                    use tauri::Manager;
                    handle
                        .state::<state::AppState>()
                        .replace(cache.index, cache.built_at_unix);
                    // Tell the window, so it refreshes what it is showing
                    // instead of waiting for the user to type again.
                    let _ = handle.emit("index-reloaded", ());
                }
                // Half-written, or being replaced right now. The next tick
                // will find it settled.
                Err(e) => tracing::debug!("chưa nạp lại được cache: {e}"),
            }
        }
    });
}

fn modified_at(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
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
    let result = app
        .global_shortcut()
        .on_shortcut(HOTKEY, move |_app, _sc, event| {
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
/// What a network scan did.
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkScanOutcome {
    pub drives: usize,
    pub files: usize,
    pub seconds: f64,
    pub cancelled: bool,
}

/// Walk every mapped network drive and merge the result into the cache.
///
/// Runs **in the GUI process, unelevated**, and that is not a shortcut — it is
/// the only place it can run. Mapped drives belong to a logon session, so the
/// elevated indexer cannot see them at all (CHECK-007). Walking directories
/// needs no privilege, so nothing is lost by it.
///
/// Slow by nature: minutes, against seconds for a local scan. That is why it
/// is a button the user presses rather than something that happens on every
/// scan — most searches are for files on the local disk, and paying a
/// ten-minute network walk for them would be absurd.
pub fn scan_network_volumes(cancel: &std::sync::atomic::AtomicBool) -> NetworkScanOutcome {
    use ntfs::volume::{self, VolumeKind};

    let started = std::time::Instant::now();
    let mut outcome = NetworkScanOutcome::default();

    let drives: Vec<char> = volume::list_volumes()
        .into_iter()
        .filter(|v| v.kind == VolumeKind::Network)
        .map(|v| v.letter)
        .collect();

    if drives.is_empty() {
        tracing::info!("không có ổ mạng nào được gắn");
        return outcome;
    }
    tracing::info!(
        "quét ổ mạng: {}",
        drives
            .iter()
            .map(|c| format!("{c}:"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut progress = ipc::elevate::ProgressWriter::new().ok();
    let opts = ntfs::tree::ResolveOptions::default();
    let mut walked: Vec<(
        char,
        ntfs::tree::ResolvedSet,
        Vec<media::metadata::FileStats>,
    )> = Vec::new();

    for (n, letter) in drives.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            outcome.cancelled = true;
            break;
        }
        let drive_started = std::time::Instant::now();
        let (set, stats) = walk::walk_volume(*letter, &opts, cancel, |p| {
            if let Some(w) = progress.as_mut() {
                let st = w.state_mut();
                st.phase = "network".into();
                st.volume = letter.to_string();
                st.records = p.files_seen as u64;
                st.media_files = p.media_kept as u64;
                st.volumes_done = n;
                st.volumes_total = drives.len();
                st.message = format!(
                    "Đang quét ổ mạng {letter}: — {} thư mục, {} tệp media",
                    p.dirs_done, p.media_kept
                );
                w.tick();
            }
        });
        tracing::info!(
            "ổ {letter}: {} thư mục · {} tệp media  [{:.1}s]",
            set.dirs.len(),
            set.files.len(),
            drive_started.elapsed().as_secs_f64()
        );
        outcome.files += set.files.len();
        walked.push((*letter, set, stats));
    }

    if walked.is_empty() {
        return finish_network_scan(outcome, started, progress, "Không quét được ổ mạng nào");
    }

    // Merge. The rule is the same one the local scan uses in reverse: an entry
    // belonging to a drive this run did not touch is left exactly as it was.
    let Ok(previous) = index::persist::load() else {
        tracing::error!("không nạp được cache để hợp nhất — huỷ, không ghi đè");
        return finish_network_scan(outcome, started, progress, "Không nạp được chỉ mục");
    };

    let touched: std::collections::HashSet<u8> = walked
        .iter()
        .map(|(letter, _, _)| (*letter as u8).to_ascii_uppercase())
        .collect();

    let old = &previous.index;
    let mut builder = index::model::IndexBuilder::new();
    let mut sizes: Vec<u64> = Vec::with_capacity(old.len());
    let mut mtimes: Vec<i64> = Vec::with_capacity(old.len());
    let mut dir_remap: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();

    for i in 0..old.len() {
        if touched.contains(&old.volume_of(i)) {
            continue;
        }
        let old_dir = old.dir_ids()[i];
        let dir_id = *dir_remap.entry(old_dir).or_insert_with(|| {
            builder.add_dir(
                old.dir_path(old_dir as usize),
                old.dir_frn(old_dir as usize),
            )
        });
        builder.add_file(old.name(i), old.kind(i), dir_id, old.frn(i));
        sizes.push(old.size(i));
        mtimes.push(old.mtime(i));
    }
    let kept = sizes.len();

    for (_, set, stats) in &walked {
        let remap: Vec<u32> = set.dirs.iter().map(|d| builder.add_dir(d, 0)).collect();
        for (f, st) in set.files.iter().zip(stats) {
            builder.add_file(&f.name, f.kind, remap[f.dir_id as usize], 0);
            sizes.push(st.size);
            mtimes.push(st.mtime);
        }
    }

    let mut ix = builder.finish();
    ix.set_file_stats(sizes, mtimes);

    // Volume stamps: the local ones carry journal cursors and must survive
    // untouched. Network drives get no stamp — there is no journal to record a
    // position in, and inventing one would make an incremental update think it
    // could follow them.
    let stamps: Vec<index::persist::VolumeStamp> = previous
        .volumes
        .iter()
        .filter(|st| !touched.contains(&(st.letter as u8).to_ascii_uppercase()))
        .cloned()
        .collect();

    match index::persist::save(&ix, stamps) {
        Ok(path) => tracing::info!(
            "hợp nhất xong: {} mục ổ cục bộ + {} mục ổ mạng = {} · ghi {}",
            kept,
            outcome.files,
            ix.len(),
            path.display()
        ),
        Err(e) => {
            tracing::error!("không ghi được cache: {e}");
            return finish_network_scan(outcome, started, progress, "Không ghi được chỉ mục");
        }
    }

    let message = if outcome.cancelled {
        format!("Đã dừng — giữ lại {} tệp trên ổ mạng", outcome.files)
    } else {
        format!(
            "Đã quét {} ổ mạng, tìm thấy {} tệp media",
            walked.len(),
            outcome.files
        )
    };
    outcome.drives = walked.len();
    finish_network_scan(outcome, started, progress, &message)
}

fn finish_network_scan(
    mut outcome: NetworkScanOutcome,
    started: std::time::Instant,
    mut progress: Option<ipc::elevate::ProgressWriter>,
    message: &str,
) -> NetworkScanOutcome {
    outcome.seconds = started.elapsed().as_secs_f64();
    if let Some(w) = progress.as_mut() {
        let st = w.state_mut();
        st.phase = "done".into();
        st.message = message.to_string();
        // Set last, and only now: the UI reloads the moment it sees this, so
        // flipping it before the cache was written would race the file it is
        // about to read.
        st.finished = true;
        w.flush();
    }
    tracing::info!("{message} [{:.1}s]", outcome.seconds);
    // Quy tắc "chỉ lượt đi trọn mới để lại dấu vết" sống trong
    // `netscan_mark::record_outcome` — một chỗ, có kiểm thử gọi tới.
    crate::netscan_mark::record_outcome(&outcome);
    outcome
}

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
    use index::update::Change;
    use ntfs::usn_journal::{self, Batch, Cursor};
    use ntfs::volume;
    use rayon::prelude::*;

    let started = std::time::Instant::now();

    // The UI watches `progress.json` and nothing else. A run that finishes
    // without ever writing `finished` looks exactly like a crash to it — which
    // is what this path did until it was tried through the button rather than
    // from a command line.
    let mut progress = ipc::elevate::ProgressWriter::new().ok();

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
    // Kept open past the journal read: applying the changes needs them again,
    // to name directories the index has never seen (RISK-003).
    let mut open: Vec<(u8, volume::VolumeHandle)> = Vec::new();

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
        if let Some(w) = progress.as_mut() {
            let p = w.state_mut();
            p.phase = "scanning".into();
            p.volume = st.letter.to_string();
            p.volumes_total = cache.volumes.len();
            p.message = format!("Đang đọc nhật ký thay đổi ổ {}:…", st.letter);
            w.flush();
        }

        match usn_journal::read_batch(&handle, st.letter, from) {
            Ok(Batch::Changes { changes: c, next }) => {
                tracing::info!("ổ {}: {} thay đổi từ journal", st.letter, c.len());
                changes.extend(c);
                let mut updated = st.clone();
                updated.next_usn = next.next_usn;
                stamps.push(updated);
                open.push(((st.letter as u8).to_ascii_uppercase(), handle));
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
        finish_incremental(
            progress,
            &format!("Không có thay đổi nào — {} tệp", cache.index.len()),
        );
        return true;
    }

    let lookup = ntfs::dir_lookup::VolumeDirLookup::new(
        open.iter().map(|(letter, h)| (*letter, h)).collect(),
        ntfs::tree::ResolveOptions::default(),
    );
    let (mut ix, stats) = index::update::rebuild_with_lookup(&cache.index, &changes, &lookup);
    tracing::info!(
        "áp thay đổi: +{} tệp, -{} tệp, {} tệp chuyển chỗ, +{} thư mục ({} hỏi hệ thống tệp), \
         -{} thư mục, {} thư mục đổi tên",
        stats.files_added,
        stats.files_removed,
        stats.files_moved,
        stats.dirs_added,
        stats.dirs_looked_up,
        stats.dirs_removed,
        stats.dirs_renamed
    );
    tracing::info!(
        "  bỏ qua {} thay đổi ngoài phạm vi index — đúng như thiết kế",
        stats.excluded
    );
    // Split from the line above on purpose. One number means "working as
    // intended", the other means files are missing — reporting them together
    // as a single "skipped" count is what would hide the second behind the
    // first.
    if stats.unresolved > 0 {
        tracing::warn!(
            "  {} thay đổi không tra được thư mục cha — những tệp này sẽ thiếu \
             trong chỉ mục cho tới lần quét đầy đủ kế tiếp",
            stats.unresolved
        );
    }

    // Nothing in the index actually moved. Write nothing.
    //
    // This is what makes running the update every few minutes reasonable. The
    // cache is 47 MB; rewriting it every time just to advance a cursor would
    // be roughly 13 GB of pointless SSD writes a day, for a machine where most
    // journal traffic is temporary files nobody is searching for.
    //
    // The cursor is left where it was as a consequence, so the next run reads
    // the same stretch of journal again. Measured: the entire ring reads in
    // 0.2 s, so re-reading is far cheaper than the write it avoids. If enough
    // happens that the ring wraps past the stored position, `read_batch`
    // reports it and a full scan follows — which is the correct answer anyway.
    if stats.files_added == 0
        && stats.files_removed == 0
        && stats.files_moved == 0
        && stats.dirs_added == 0
        && stats.dirs_removed == 0
        && stats.dirs_renamed == 0
    {
        tracing::info!(
            "{} bản ghi journal, không mục nào trong chỉ mục thay đổi — không ghi lại cache [{:.2}s]",
            changes.len(),
            started.elapsed().as_secs_f64()
        );
        // Đóng dấu "đã kiểm" dù không ghi lại cache. Đây chính là ca mà
        // `built_at_unix` không nói được: máy khoẻ, tác vụ vừa chạy, nhưng vì
        // không có gì đổi nên mốc trong `index.bin` đứng yên — và giao diện
        // tưởng chỉ mục đã cũ hàng giờ.
        lastcheck::record(false);
        finish_incremental(
            progress,
            &format!("Không có thay đổi nào — {} tệp", ix.len()),
        );
        return true;
    }

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

    if let Some(w) = progress.as_mut() {
        let p = w.state_mut();
        p.phase = "saving".into();
        p.message = "Đang lưu chỉ mục…".into();
        w.flush();
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
            // Deliberately not reported as finished: returning false sends the
            // caller to a full scan, which writes its own progress. Claiming
            // success here would stop the UI on a run that achieved nothing.
            return false;
        }
    }

    // Đóng dấu SAU khi cache đã ghi thành công. Đặt trước đó thì một lượt ghi
    // hỏng vẫn để lại dấu "vừa kiểm xong", tức nói dối đúng lúc cần nói thật.
    lastcheck::record(true);

    finish_incremental(
        progress,
        &format!(
            "Cập nhật xong: {} tệp (+{} −{})",
            ix.len(),
            stats.files_added,
            stats.files_removed
        ),
    );
    true
}

/// Should this run raise the `finished` flag the UI stops on?
///
/// False when the caller has more to do afterwards — a network walk that this
/// process cannot perform, so it runs in the GUI once this one exits.
fn announces_finish() -> bool {
    !std::env::args().any(|a| a == "--no-finish")
}

/// Mark the run finished, after the cache is safely on disk.
///
/// Set last and only here: the UI reloads the moment it sees the flag, so
/// raising it any earlier would race the file it is about to read.
fn finish_incremental(mut progress: Option<ipc::elevate::ProgressWriter>, message: &str) {
    if !announces_finish() {
        return;
    }
    if let Some(w) = progress.as_mut() {
        let p = w.state_mut();
        p.phase = "done".into();
        p.message = message.to_string();
        p.finished = true;
        w.flush();
    }
}

/// Report what the change journal still remembers being deleted.
///
/// Written to answer one question the index cannot: an index says what is
/// there now, never what used to be. When a scan returns far fewer files than
/// the last one, the only honest source is the journal — it recorded every
/// deletion as it happened, with the name and the time.
///
/// Reads nothing but the journal and writes nothing at all.
///
/// ```text
/// mediafinder.exe --audit D
/// ```
pub fn run_audit(args: &[String]) {
    use ntfs::usn_journal;
    use ntfs::volume;
    use std::collections::BTreeMap;

    let only: Option<char> = args
        .iter()
        .skip_while(|a| *a != "--audit")
        .nth(1)
        .and_then(|a| a.chars().next())
        .map(|c| c.to_ascii_uppercase());

    for v in volume::list_volumes() {
        if !v.is_scannable() {
            continue;
        }
        if only.is_some_and(|c| v.letter.to_ascii_uppercase() != c) {
            continue;
        }
        let Ok(handle) = volume::open_volume(&v) else {
            tracing::error!("ổ {}: không mở được", v.letter);
            continue;
        };

        let started = std::time::Instant::now();
        match usn_journal::audit_deletions(&handle, v.letter, true) {
            Ok((deletions, oldest, newest)) => {
                let files = deletions.iter().filter(|d| !d.is_dir).count();
                let dirs = deletions.len() - files;
                tracing::info!(
                    "ổ {}: nhật ký còn nhớ từ usn={} tới {} — {} tệp media và {} thư mục đã bị xoá [{:.1}s]",
                    v.letter,
                    oldest,
                    newest,
                    files,
                    dirs,
                    started.elapsed().as_secs_f64()
                );

                // Group by day, so a mass deletion shows up as a spike rather
                // than as a wall of individual lines.
                let mut per_day: BTreeMap<String, usize> = BTreeMap::new();
                for d in deletions.iter().filter(|d| !d.is_dir) {
                    *per_day.entry(filetime_day(d.filetime)).or_default() += 1;
                }
                for (day, n) in &per_day {
                    tracing::info!("    {day}: {n} tệp media");
                }

                // And by extension, because "70 000 files" says much less than
                // "70 000 thumbnails" or "70 000 videos".
                let mut per_ext: BTreeMap<String, usize> = BTreeMap::new();
                for d in deletions.iter().filter(|d| !d.is_dir) {
                    let ext = d
                        .name
                        .rsplit_once('.')
                        .map(|(_, e)| e.to_ascii_lowercase())
                        .unwrap_or_default();
                    *per_ext.entry(ext).or_default() += 1;
                }
                let mut by_count: Vec<_> = per_ext.into_iter().collect();
                by_count.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                for (ext, n) in by_count.iter().take(12) {
                    tracing::info!("    .{ext}: {n}");
                }

                // Which folders lost files. The journal names only the parent
                // reference number, but the index still holds a table of
                // directories with theirs — so any folder that survived the
                // deletion can be named.
                if let Ok(cache) = index::persist::load() {
                    let ix = &cache.index;
                    let mut by_frn: std::collections::HashMap<u64, usize> =
                        std::collections::HashMap::new();
                    for i in 0..ix.dir_count() {
                        if ix.volume_of_dir(i) == (v.letter as u8).to_ascii_uppercase() {
                            by_frn.insert(ix.dir_frn(i), i);
                        }
                    }
                    // Folders that were deleted too, so a path can be walked
                    // back up out of the deleted records themselves.
                    let gone_dirs: std::collections::HashMap<u64, (&str, u64)> = deletions
                        .iter()
                        .filter(|d| d.is_dir)
                        .map(|d| (d.frn, (d.name.as_str(), d.parent_frn)))
                        .collect();

                    let resolve = |mut frn: u64| -> Option<String> {
                        let mut parts: Vec<&str> = Vec::new();
                        for _ in 0..64 {
                            if let Some(&i) = by_frn.get(&frn) {
                                let mut path = ix.dir_path(i).to_string();
                                for part in parts.iter().rev() {
                                    path.push('\\');
                                    path.push_str(part);
                                }
                                return Some(path);
                            }
                            let (name, parent) = *gone_dirs.get(&frn)?;
                            parts.push(name);
                            frn = parent;
                        }
                        None
                    };

                    let mut per_dir: BTreeMap<String, usize> = BTreeMap::new();
                    let mut unknown = 0usize;
                    for d in deletions.iter().filter(|d| !d.is_dir) {
                        match resolve(d.parent_frn) {
                            Some(path) => *per_dir.entry(path).or_default() += 1,
                            None => unknown += 1,
                        }
                    }
                    let mut ranked: Vec<_> = per_dir.into_iter().collect();
                    ranked.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                    tracing::info!("  thư mục đã mất tệp ({} thư mục):", ranked.len());
                    for (dir, n) in ranked.iter().take(25) {
                        tracing::info!("    {n:>6}  {dir}");
                    }
                    if unknown > 0 {
                        tracing::info!("    {unknown} tệp không dựng lại được đường dẫn");
                    }

                    // The top of each deleted tree: the folder whose own parent
                    // still exists. That is the thing that was actually deleted;
                    // everything below it merely went with it.
                    let mut roots: BTreeMap<String, usize> = BTreeMap::new();
                    for d in deletions.iter().filter(|d| d.is_dir) {
                        if by_frn.contains_key(&d.parent_frn) {
                            if let Some(parent) = resolve(d.parent_frn) {
                                *roots.entry(format!("{parent}\\{}", d.name)).or_default() += 1;
                            }
                        }
                    }
                    if !roots.is_empty() {
                        tracing::info!("  thư mục GỐC bị xoá (thư mục cha vẫn còn):");
                        for dir in roots.keys().take(25) {
                            tracing::info!("    {dir}");
                        }
                    }
                }

                for d in deletions.iter().filter(|d| !d.is_dir).take(8) {
                    tracing::info!("    ví dụ: {} ({})", d.name, filetime_day(d.filetime));
                }
            }
            Err(e) => tracing::error!("ổ {}: {e}", v.letter),
        }
    }
}

/// Windows FILETIME to a `YYYY-MM-DD HH` string, without pulling in a date crate.
fn filetime_day(ft: i64) -> String {
    // FILETIME counts 100-nanosecond ticks from 1601-01-01; Unix time counts
    // seconds from 1970-01-01. 11644473600 seconds separate the two epochs.
    const TICKS_PER_SEC: i64 = 10_000_000;
    const EPOCH_DIFF: i64 = 11_644_473_600;
    let unix = ft / TICKS_PER_SEC - EPOCH_DIFF;
    if unix <= 0 {
        return "?".into();
    }
    let days = unix / 86_400;
    let hour = (unix % 86_400) / 3600;

    // Civil-from-days, the standard algorithm — no external crate, no drift.
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {hour:02}h")
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
            if *is_dir {
                "THƯ MỤC "
            } else {
                "CÓ MẶT  "
            }
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

    // Tiến trình này chạy elevated — là chỗ duy nhất tự thay được lịch của
    // chính mình. Máy còn mang lịch v1 (mỗi ngày) sẽ tự lên v2 (mỗi 15 phút)
    // ở lần chạy định kỳ kế tiếp, không ai phải bấm gì.
    setup::upgrade_schedule_if_stale();

    // This process is elevated — the user approved that for the scan they asked
    // for. Registering the refresh task here means it costs no second prompt,
    // and after this the task carries the privilege so nothing prompts again.
    //
    // `--dry-run` is a developer path and must not touch the machine.
    if !dry_run {
        setup::ensure_scheduled_task();
    }

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
            st.message = format!("Xong ổ {}: {} tệp media", v.letter, set.files.len());
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

    // Carry over every entry belonging to a drive this run did not scan.
    //
    // Not an optimisation — a correctness rule, and the one place where this
    // feature could destroy weeks of scanning. The elevated indexer cannot see
    // mapped network drives at all (CHECK-007), so a plain "Quét lại" rebuilds
    // the index with no `Z:` in it whatsoever. Without this, a user who spent
    // twenty minutes indexing a NAS loses all of it by pressing the button
    // that is supposed to refresh their local disk.
    //
    // Stated without mentioning networks on purpose: a drive that was not
    // scanned is a drive this run has nothing to say about, whether it is a
    // NAS, an unplugged USB stick, or a disk that failed to open.
    let scanned: std::collections::HashSet<u8> = stamps
        .iter()
        .map(|s| (s.letter as u8).to_ascii_uppercase())
        .collect();
    let mut carried: Vec<(u64, i64)> = Vec::new();
    let mut carried_stamps: Vec<index::persist::VolumeStamp> = Vec::new();

    if let Ok(previous) = index::persist::load() {
        let old = &previous.index;
        let mut dir_remap: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();

        for i in 0..old.len() {
            if scanned.contains(&old.volume_of(i)) {
                continue;
            }
            let old_dir = old.dir_ids()[i];
            let dir_id = *dir_remap.entry(old_dir).or_insert_with(|| {
                builder.add_dir(
                    old.dir_path(old_dir as usize),
                    old.dir_frn(old_dir as usize),
                )
            });
            builder.add_file(old.name(i), old.kind(i), dir_id, old.frn(i));
            // Kept rather than re-measured: these files are on a drive this
            // process cannot reach, and asking about them over the network
            // would cost more than the whole local scan.
            carried.push((old.size(i), old.mtime(i)));
        }

        for st in &previous.volumes {
            if !scanned.contains(&(st.letter as u8).to_ascii_uppercase()) {
                carried_stamps.push(st.clone());
            }
        }

        if !carried.is_empty() {
            let drives: std::collections::BTreeSet<char> =
                carried_stamps.iter().map(|s| s.letter).collect();
            tracing::info!(
                "giữ lại {} mục từ ổ không quét lần này ({}) — lần quét này không có \
                 thẩm quyền nói gì về chúng",
                carried.len(),
                drives
                    .iter()
                    .map(|c| format!("{c}:"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    stamps.extend(carried_stamps);

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
        // Only what was scanned. The carried entries already have their
        // figures, and they were appended last so they are exactly the tail.
        let fresh = ix.len() - carried.len();
        let paths: Vec<String> = (0..fresh).map(|i| ix.full_path(i)).collect();
        let mut stats: Vec<media::metadata::FileStats> = paths
            .par_iter()
            .map(|p| media::metadata::file_stats(p).unwrap_or_default())
            .collect();
        stats.extend(
            carried
                .iter()
                .map(|&(size, mtime)| media::metadata::FileStats { size, mtime }),
        );

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
    if let Some(p) = progress.as_mut().filter(|_| announces_finish()) {
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
            tracing::info!("      [{}] {}", h.score, ix.full_path(h.index as usize));
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
    use super::{filetime_day, HOTKEY};

    /// A wrong date here would be worse than no date: it is read while working
    /// out when files disappeared, and a plausible-looking wrong day would send
    /// the search in the wrong direction.
    #[test]
    fn filetime_converts_to_the_right_calendar_day() {
        // Anchors computed independently: FILETIME = (unix + 11644473600) × 10^7.
        assert_eq!(filetime_day(125_911_584_000_000_000), "2000-01-01 00h");
        // 2026-08-24 12:00 UTC
        assert_eq!(filetime_day(134_320_464_000_000_000), "2026-08-24 12h");
        // A leap day, which is where naive date maths usually breaks.
        // 2024-02-29 00:00 UTC = unix 1709164800
        assert_eq!(filetime_day(133_536_384_000_000_000), "2024-02-29 00h");
        // Nonsense in, honest answer out — never a date from 1601.
        assert_eq!(filetime_day(0), "?");
    }

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
