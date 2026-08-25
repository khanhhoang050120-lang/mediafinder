//! Tauri commands.
//!
//! Every command returns `Result<_, String>` and never unwraps anything that
//! came from the frontend or the filesystem. That is the other half of the
//! `panic = "abort"` decision recorded in `docs/risk.md` (RISK-001): rather
//! than argue about whether a panic unwinds, don't panic. A path that vanished
//! between the scan and the click becomes a message on screen, not a dead
//! application.

use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::index::model::MediaKind;
use crate::ipc::elevate;
use crate::index::search::{search as run_search, SearchOptions};
use crate::state::{AppState, IndexMeta};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub name: String,
    /// Absolute path, built only for the results actually returned.
    pub path: String,
    pub dir: String,
    pub kind: &'static str,
    pub score: i32,
    /// How many of the query's words this file actually contains.
    pub matched: u16,
    /// Position in the index, used to build the thumbnail URL.
    pub index: u32,
    pub size: u64,

    // Zero means "not read yet" — the background enrichment has not reached
    // this file. The UI shows nothing rather than inventing a value.
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
}

/// Present when nothing matched the whole query, so the UI can say plainly
/// that these are partial matches rather than letting the user assume the
/// results are exact.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelaxedInfo {
    pub total_tokens: usize,
    pub best_matched: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    /// Echoed back so the frontend can drop answers to superseded keystrokes.
    pub id: u64,
    pub hits: Vec<SearchHit>,
    pub elapsed_ms: f64,
    pub searched: usize,
    pub relaxed: Option<RelaxedInfo>,
    /// Which index these results came from; thumbnail URLs carry it so a
    /// rescan cannot make them point at the wrong file.
    pub epoch: u64,
}

/// The property filters, grouped rather than passed as loose arguments.
///
/// Three more parameters on a command that already had six is the point at
/// which the call site stops being readable — and where swapping two `u64`s
/// by accident becomes silent.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Filters {
    /// Shortest side in pixels; `0` disables.
    pub min_height: u32,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
}

fn parse_kinds(kinds: &[String]) -> Vec<MediaKind> {
    kinds
        .iter()
        .filter_map(|k| match k.as_str() {
            "video" => Some(MediaKind::Video),
            "image" => Some(MediaKind::Image),
            "audio" => Some(MediaKind::Audio),
            _ => None,
        })
        .collect()
}

/// Search the loaded index.
///
/// `id` must increase with every keystroke. Claiming it here supersedes any
/// older query, which then abandons its work at the next chunk boundary — this
/// is what lets the UI search on every keystroke with no debounce at all.
#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    state_enrich: State<'_, crate::media::enrich::EnrichService>,
    id: u64,
    query: String,
    kinds: Vec<String>,
    limit: usize,
    filters: Filters,
) -> Result<SearchResponse, String> {
    state.begin_query(id);

    // Snapshot, then release the shared cell immediately — see `state.rs`.
    let index = state.snapshot();
    let opts = SearchOptions {
        limit: limit.clamp(1, 20_000),
        kinds: parse_kinds(&kinds),
        min_height: filters.min_height,
        min_duration_ms: filters.min_duration_ms,
        max_duration_ms: filters.max_duration_ms,
    };

    // Properties are kept parallel to the index and swapped in as enrichment
    // progresses; taking a snapshot here means a search never blocks behind
    // the background reader.
    let enrich = state_enrich.props();

    // Run inline rather than on a blocking pool. The search itself already
    // fans out across every core through rayon, and at a few milliseconds for
    // half a million entries it is far too short to be worth another hop.
    let started = std::time::Instant::now();
    let outcome = run_search(&index, &query, &opts, &enrich, state.generation(), id);
    let elapsed = started.elapsed();

    // A superseded search returns nothing; the frontend drops it by id anyway.
    if state.generation().load(Ordering::Relaxed) != id {
        return Ok(SearchResponse {
            id,
            hits: Vec::new(),
            elapsed_ms: elapsed.as_secs_f64() * 1000.0,
            searched: index.len(),
            relaxed: None,
            epoch: state.index_epoch(),
        });
    }

    let hits = outcome
        .hits
        .into_iter()
        .map(|h| {
            let i = h.index as usize;
            let p = enrich.get(i).copied().unwrap_or_default();
            SearchHit {
                name: index.name(i).to_string(),
                path: index.full_path(i),
                dir: index.dir(i).to_string(),
                kind: index.kind(i).as_str(),
                score: h.score,
                matched: h.matched,
                index: h.index,
                size: index.size(i),
                width: p.width,
                height: p.height,
                duration_ms: p.duration_ms,
            }
        })
        .collect();

    Ok(SearchResponse {
        id,
        hits,
        elapsed_ms: elapsed.as_secs_f64() * 1000.0,
        searched: index.len(),
        relaxed: outcome.relaxed.map(|r| RelaxedInfo {
            total_tokens: r.total_tokens,
            best_matched: r.best_matched,
        }),
        epoch: state.index_epoch(),
    })
}

#[tauri::command]
pub fn index_status(state: State<'_, AppState>) -> IndexMeta {
    state.meta()
}

/// How far the background property reader has got.
#[tauri::command]
pub fn enrich_status(
    enrich: State<'_, crate::media::enrich::EnrichService>,
) -> crate::media::enrich::EnrichStatus {
    enrich.status()
}

/// The summon shortcut, and whether the app actually managed to claim it.
///
/// The combination is sent to the frontend rather than written there too, so
/// there is exactly one place in the codebase that decides what the key is.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotkeyStatus {
    /// The combination, `+`-separated — the UI splits it into keycaps.
    pub combo: String,
    /// False when another application got there first.
    pub active: bool,
}

/// Report the global shortcut so the UI only advertises a key that works.
#[tauri::command]
pub fn hotkey_status() -> HotkeyStatus {
    use std::sync::atomic::Ordering;
    HotkeyStatus {
        combo: crate::HOTKEY.to_string(),
        active: crate::HOTKEY_ACTIVE.load(Ordering::Relaxed),
    }
}

/// Start a rescan in an elevated child process.
///
/// Returns as soon as the child is running. The scan itself takes tens of
/// seconds, so blocking here would freeze the window for the whole of it;
/// instead a watcher thread waits for the child and the UI polls
/// [`scan_progress`].
#[tauri::command]
pub fn request_scan(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state.is_scanning() {
        return Err("Đang có một lượt quét chạy rồi.".into());
    }

    // Clear stale progress first, so a poll arriving before the child writes
    // anything cannot show the previous scan's numbers.
    let _ = elevate::clear_progress();

    let child = elevate::spawn_elevated_indexer().map_err(|e| e.to_string())?;
    state.set_scanning(true);

    std::thread::spawn(move || {
        child.wait();
        // Whatever happened — success, crash, or the child refusing to run —
        // the scan is over. Without clearing this the button would stay
        // disabled forever after a crash.
        app.state::<AppState>().set_scanning(false);
    });

    Ok(())
}

/// Scan the local disks, then walk every mapped network drive.
///
/// Two phases, in that order and deliberately so: the local scan is seconds
/// and covers where most files are, the network walk is minutes. Doing the
/// local part first means the fast, common result is already in place before
/// the slow part begins — and if the user cancels, they still have it.
///
/// The network walk runs **here, in the GUI process**. It has to: mapped
/// drives belong to a logon session and the elevated indexer cannot see them
/// (CHECK-007). It needs no privilege, so nothing is given up.
#[tauri::command]
pub fn request_scan_with_network(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if state.is_scanning() {
        return Err("Đang có một lượt quét chạy rồi.".into());
    }
    let _ = elevate::clear_progress();

    let child = elevate::spawn_elevated_indexer().map_err(|e| e.to_string())?;
    state.set_scanning(true);
    state.request_cancel(false);

    std::thread::spawn(move || {
        // Phase one: the elevated child does the local disks and writes the
        // cache. Waited for rather than run alongside, because both phases
        // write the same file.
        child.wait();

        let st = app.state::<AppState>();
        if !st.cancel_requested() {
            // Phase two, in this process.
            crate::scan_network_volumes(st.cancel_flag());
        }
        st.set_scanning(false);
    });

    Ok(())
}

/// Ask the running scan to stop.
///
/// Only the network phase can honour this — the local scan happens in another
/// process entirely. Said plainly in the UI rather than pretending otherwise.
#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) {
    state.request_cancel(true);
}

/// Which mapped network drives exist, for the UI to name in the button.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDrive {
    pub letter: String,
    pub remote: String,
}

#[tauri::command]
pub fn network_drives() -> Vec<NetworkDrive> {
    use crate::ntfs::volume::{self, VolumeKind};
    volume::list_volumes()
        .into_iter()
        .filter(|v| v.kind == VolumeKind::Network)
        .map(|v| NetworkDrive {
            letter: v.letter.to_string(),
            remote: v.remote.unwrap_or_default(),
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub scanning: bool,
    pub progress: Option<elevate::ScanProgress>,
}

#[tauri::command]
pub fn scan_progress(state: State<'_, AppState>) -> ScanStatus {
    ScanStatus {
        scanning: state.is_scanning(),
        progress: elevate::read_progress(),
    }
}

/// Load the cache the indexer just wrote and swap it in.
///
/// Called by the UI once progress reports `finished`. Safe by construction:
/// the indexer writes the cache *before* setting that flag, so by the time the
/// UI asks, the file is complete.
#[tauri::command]
pub fn reload_index(
    state: State<'_, AppState>,
    enrich: State<'_, crate::media::enrich::EnrichService>,
) -> Result<IndexMeta, String> {
    match crate::index::persist::load() {
        Ok(cache) => {
            state.replace(cache.index, cache.built_at_unix);
            // Point enrichment at the new index. Anything already known for a
            // path whose size and time still match is reused, so a rescan does
            // not throw away an hour of reading.
            enrich.start(state.snapshot());
            Ok(state.meta())
        }
        Err(e) => {
            state.set_problem(e.to_string());
            Err(e.to_string())
        }
    }
}

/// A duplicate group, with the paths resolved for display.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupeGroupView {
    pub size: u64,
    pub wasted: u64,
    pub files: Vec<SearchHit>,
}

/// Begin looking for duplicates.
#[tauri::command]
pub fn find_duplicates(
    state: State<'_, AppState>,
    dupes: State<'_, crate::media::dupes::DupeService>,
) -> Result<(), String> {
    if dupes.start(state.snapshot()) {
        Ok(())
    } else {
        Err("Đang tìm trùng lặp rồi.".into())
    }
}

#[tauri::command]
pub fn dupe_progress(
    dupes: State<'_, crate::media::dupes::DupeService>,
) -> crate::media::dupes::DupeProgress {
    dupes.progress()
}

/// The finished groups, with every path resolved.
#[tauri::command]
pub fn dupe_groups(
    state: State<'_, AppState>,
    enrich: State<'_, crate::media::enrich::EnrichService>,
    dupes: State<'_, crate::media::dupes::DupeService>,
    limit: usize,
) -> Vec<DupeGroupView> {
    let index = state.snapshot();
    let props = enrich.props();

    dupes
        .groups()
        .into_iter()
        .take(limit.clamp(1, 2_000))
        .map(|g| DupeGroupView {
            size: g.size,
            wasted: g.wasted,
            files: g
                .entries
                .iter()
                .filter_map(|&e| {
                    let i = e as usize;
                    if i >= index.len() {
                        return None;
                    }
                    let p = props.get(i).copied().unwrap_or_default();
                    Some(SearchHit {
                        name: index.name(i).to_string(),
                        path: index.full_path(i),
                        dir: index.dir(i).to_string(),
                        kind: index.kind(i).as_str(),
                        score: 0,
                        matched: 0,
                        index: e,
                        size: index.size(i),
                        width: p.width,
                        height: p.height,
                        duration_ms: p.duration_ms,
                    })
                })
                .collect(),
        })
        .collect()
}

/// Open a file with whatever Windows uses for that type.
#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("Tệp không còn tồn tại:\n{path}"));
    }
    shell::open_with_default_app(&path)
}

/// Open File Explorer with this file selected — the "Open file location"
/// action.
#[tauri::command]
pub fn reveal_in_explorer(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.exists() {
        return shell::open_folder_and_select(&path);
    }

    // The file is gone but its folder may not be. Opening the folder is more
    // useful than an error — it is usually exactly where the user was headed.
    match p.parent() {
        Some(dir) if dir.exists() => shell::open_with_default_app(&dir.to_string_lossy()),
        _ => Err(format!("Tệp và thư mục chứa nó đều không còn:\n{path}")),
    }
}

/// Win32 shell integration.
///
/// Public so the integration tests in `tests/` can exercise the real COM and
/// ShellExecute calls. Those are the parts most likely to be wrong, and no
/// unit test can reach them.
pub mod shell {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
    };
    use windows::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    /// `ShellExecuteW` returns a fake HINSTANCE; anything above 32 means success.
    const SHELL_EXECUTE_SUCCESS_THRESHOLD: isize = 32;

    pub fn open_with_default_app(path: &str) -> Result<(), String> {
        let file = HSTRING::from(path);
        let verb = HSTRING::from("open");

        let result = unsafe {
            ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(file.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };

        if result.0 as isize > SHELL_EXECUTE_SUCCESS_THRESHOLD {
            Ok(())
        } else {
            Err(format!(
                "Windows không mở được tệp này (mã {}).\nCó thể chưa có ứng dụng mặc định cho định dạng đó.\n{path}",
                result.0 as isize
            ))
        }
    }

    /// Open the containing folder with the item selected.
    ///
    /// Uses `SHOpenFolderAndSelectItems` rather than the far shorter
    /// `explorer.exe /select,"path"`. The command-line form breaks on any path
    /// containing a comma — and media filenames are full of them — because
    /// Explorer parses its own argument string and there is no way to escape
    /// the separator. The COM call takes a PIDL and has no parsing to get
    /// wrong.
    ///
    /// Passing the *file's* own PIDL with an empty item list is the documented
    /// idiom for "open the parent and highlight this".
    pub fn open_folder_and_select(path: &str) -> Result<(), String> {
        let wide = HSTRING::from(path);

        unsafe {
            // Explorer's shell APIs require COM on the calling thread.
            // S_FALSE means it was already initialised here, which is fine.
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
            let needs_uninit = hr.is_ok();

            let pidl = ILCreateFromPathW(PCWSTR(wide.as_ptr()));
            if pidl.is_null() {
                if needs_uninit {
                    CoUninitialize();
                }
                return Err(format!("Không dựng được PIDL cho đường dẫn:\n{path}"));
            }

            let result = SHOpenFolderAndSelectItems(pidl, None, 0);

            ILFree(Some(pidl));
            if needs_uninit {
                CoUninitialize();
            }

            result.map_err(|e| format!("Không mở được File Explorer: {e}\n{path}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_kinds_and_ignores_junk() {
        let got = parse_kinds(&[
            "video".into(),
            "nonsense".into(),
            "audio".into(),
            String::new(),
        ]);
        assert_eq!(got, vec![MediaKind::Video, MediaKind::Audio]);
    }

    #[test]
    fn no_kinds_means_no_filter() {
        assert!(parse_kinds(&[]).is_empty());
    }

    #[test]
    fn opening_a_missing_file_is_an_error_not_a_panic() {
        let err = open_file(r"D:\definitely\not\here\nope.mp4".into()).unwrap_err();
        assert!(err.contains("không còn tồn tại"));
    }

    #[test]
    fn revealing_a_missing_file_under_a_missing_folder_is_an_error() {
        let err =
            reveal_in_explorer(r"D:\definitely\not\here\nope.mp4".into()).unwrap_err();
        assert!(err.contains("không còn"));
    }
}
