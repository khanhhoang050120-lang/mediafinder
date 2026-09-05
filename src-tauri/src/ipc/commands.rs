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
use crate::index::search::{search as run_search, SearchOptions};
use crate::ipc::elevate;
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

/// Everything about how to run one search, apart from its id.
///
/// Grouped rather than passed as separate arguments: the list had grown to
/// eight and every addition made the call site harder to read. The id stays
/// outside because it is not part of the question being asked — it is how a
/// superseded answer gets recognised.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub query: String,
    pub kinds: Vec<String>,
    pub limit: usize,
    pub filters: Filters,
    pub order: crate::index::search::Order,
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
    /// How recently the file was modified, in days back from now; `0`
    /// disables. Sent as a number of days rather than a timestamp so the
    /// window is always relative to *now* — a search left open overnight
    /// should still mean "the last seven days" in the morning.
    pub within_days: u32,
}

/// Turn "within N days" into a Unix timestamp, or 0 for no limit.
fn cutoff_for(days: u32) -> i64 {
    if days == 0 {
        return 0;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // Never negative: a clock set before 1970 would otherwise turn the filter
    // into "modified after some point in the past", which quietly matches
    // everything instead of failing.
    (now - days as i64 * 86_400).max(0)
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
    req: Query,
) -> Result<SearchResponse, String> {
    let Query {
        query,
        kinds,
        limit,
        filters,
        order,
    } = req;
    state.begin_query(id);

    // Snapshot, then release the shared cell immediately — see `state.rs`.
    let index = state.snapshot();
    let opts = SearchOptions {
        limit: limit.clamp(1, 20_000),
        order,
        modified_after: cutoff_for(filters.within_days),
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

/// Begin dragging files out of the window, into whatever accepts a file drop.
///
/// This cannot be done from the web side. A drag started by the WebView offers
/// the formats a web page can offer — text, a URL — and every application that
/// takes files wants `CF_HDROP`, the shell's own structure. CapCut, Explorer
/// and a browser's upload field all ignore what a WebView drag provides.
///
/// See `ipc::drag_source` for how the native drag is built, and for why it is
/// written here rather than taken from a crate.
///
/// **Blocks the window until the drop finishes.** `DoDragDrop` runs its own
/// modal loop and does not return until the user releases the button, which is
/// why this has to be on the UI thread and why the window sits still while a
/// drag is in flight. Explorer behaves the same way.
#[tauri::command]
pub fn start_file_drag(app: tauri::AppHandle, paths: Vec<String>) -> Result<(), String> {
    if paths.is_empty() {
        return Err("Không có tệp nào để kéo.".into());
    }

    // Filtered on the shell's own opinion rather than `Path::exists`: the drag
    // is built from shell items, so anything the shell will not carry has to go
    // before it reaches that code — and a file deleted since the last scan
    // should not fail a drag of four others.
    let files: Vec<std::path::PathBuf> = paths
        .iter()
        .map(std::path::PathBuf::from)
        .filter(|p| crate::ipc::drag_source::shell_accepts(p))
        .collect();
    if files.is_empty() {
        return Err("Tệp không còn ở đó nữa — thử quét lại.".into());
    }
    if files.len() < paths.len() {
        tracing::warn!(
            "bỏ {} tệp không còn tồn tại khỏi thao tác kéo",
            paths.len() - files.len()
        );
    }

    tracing::info!(
        "bắt đầu kéo {} tệp: {}",
        files.len(),
        files
            .iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy())
            .collect::<Vec<_>>()
            .join(" · ")
    );

    app.run_on_main_thread(move || {
        let refs: Vec<&std::path::Path> = files.iter().map(|p| p.as_path()).collect();
        if let Err(e) = crate::ipc::drag_source::drag_files(&refs) {
            tracing::warn!("không bắt đầu được thao tác kéo: {e}");
        }
    })
    .map_err(|e| e.to_string())?;

    Ok(())
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
    /// Đang phải dùng phím dự phòng vì tổ hợp ưu tiên bị chiếm.
    pub fallback: bool,
    /// Tổ hợp app *muốn* dùng — cần cho câu "X bị chiếm, đang dùng Y".
    pub preferred: String,
}

/// Report the global shortcut so the UI only advertises a key that works.
#[tauri::command]
pub fn hotkey_status() -> HotkeyStatus {
    use std::sync::atomic::Ordering;
    HotkeyStatus {
        // Tổ hợp ĐANG DÙNG THẬT, không phải tổ hợp mong muốn. Giao diện in
        // thẳng chuỗi này ra, nên viết cứng ở đây là để màn hình mời người
        // dùng bấm một phím không có tác dụng.
        combo: crate::hotkey::in_use(),
        active: crate::hotkey::ACTIVE.load(Ordering::Relaxed),
        // Có phải phím dự phòng không, và tổ hợp nào đã bị chiếm — để giao
        // diện giải thích được vì sao phím quen không còn tác dụng.
        fallback: crate::hotkey::is_fallback(),
        preferred: crate::hotkey::preferred().to_string(),
    }
}

/// Report whether a newer release exists, so the UI can offer the update.
///
/// Reads what the startup check already found; it never touches the network
/// itself, so the UI may poll it as often as it likes.
#[tauri::command]
pub fn update_status() -> crate::update::UpdateStatus {
    crate::update::status()
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

    // Prefer the scheduled task: it already has the privileges the journal
    // read needs, and starting it needs none — so the ordinary refresh costs
    // no UAC prompt. Falls back to an elevated child when the task is absent,
    // which is the case for anyone who declined to set it up.
    if elevate::try_run_scheduled_task() {
        state.set_scanning(true);
        std::thread::spawn(move || {
            wait_for_finish();
            app.state::<AppState>().set_scanning(false);
        });
        return Ok(());
    }

    let child = elevate::spawn_elevated_indexer(true).map_err(|e| e.to_string())?;
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

/// Wait for a scan started by the scheduled task to report itself finished.
///
/// There is no process handle to wait on — the task runs in its own session —
/// so completion is read from the progress file the indexer writes. The
/// deadline exists so a task that never reports cannot leave the button
/// disabled for the rest of the session.
fn wait_for_finish() {
    const DEADLINE: std::time::Duration = std::time::Duration::from_secs(20 * 60);
    let started = std::time::Instant::now();
    while started.elapsed() < DEADLINE {
        std::thread::sleep(std::time::Duration::from_millis(400));
        if elevate::read_progress().is_some_and(|p| p.finished) {
            return;
        }
    }
    tracing::warn!("tác vụ quét không báo kết thúc sau 20 phút — bỏ theo dõi");
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

    // The local phase must not announce completion: the network walk comes
    // after it, and the UI would otherwise stop watching before the slow half
    // had even started.
    let child = elevate::spawn_elevated_indexer(false).map_err(|e| e.to_string())?;
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

/// Chỉ mục cũ tới đâu — để màn hình "không có kết quả" nói đúng nguyên nhân
/// thay vì ngầm đổ lỗi cho người gõ.
///
/// Rẻ: chỉ đọc phần đầu tệp cache, không nạp lại chỉ mục. Giao diện chỉ gọi
/// khi đã không có kết quả nào, nên nó không nằm trên đường tìm kiếm.
/// Đánh dấu rằng lần khởi động kế tiếp đến từ một bản cập nhật.
///
/// Giao diện gọi ngay trước `downloadAndInstall`. Xem [`crate::relaunch`] để
/// biết vì sao cần: bộ cập nhật chuyển tiếp nguyên dòng lệnh cũ, nên trên máy
/// khởi động ẩn thì app mở lại ẩn và cửa sổ không bao giờ hiện.
#[tauri::command]
pub fn mark_updating() {
    crate::relaunch::danh_dau();
}

#[tauri::command]
pub fn index_freshness(state: State<'_, AppState>) -> crate::freshness::Freshness {
    crate::freshness::read(&state)
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
    /// `epoch` của chỉ mục mà lượt quét đã dùng.
    ///
    /// Giao diện dựng URL ảnh thu nhỏ bằng `thumbUrl(epoch, index)`, nên nó
    /// phải dùng epoch NÀY chứ không phải epoch hiện tại — ghép epoch mới với
    /// vị trí cũ thì ảnh thu nhỏ cũng là của tệp khác.
    pub epoch: u64,
}

/// Begin looking for duplicates.
#[tauri::command]
pub fn find_duplicates(
    state: State<'_, AppState>,
    dupes: State<'_, crate::media::dupes::DupeService>,
) -> Result<(), String> {
    if dupes.start(state.snapshot(), state.index_epoch()) {
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

/// Stop a scan the person has walked away from.
///
/// The scan reads the disk for minutes. Without this, closing the view left it
/// running to the end for an answer nobody would look at, competing for the
/// same drive as the searching the person went back to doing.
#[tauri::command]
pub fn cancel_duplicates(dupes: State<'_, crate::media::dupes::DupeService>) {
    dupes.cancel();
}

/// The finished groups, with every path resolved.
#[tauri::command]
pub fn dupe_groups(
    // KHÔNG nhận `AppState`: bản sửa này cắt hẳn phụ thuộc vào snapshot hiện
    // tại. Thêm nó lại là mở đường cho chính lỗi vừa sửa quay về.
    enrich: State<'_, crate::media::enrich::EnrichService>,
    dupes: State<'_, crate::media::dupes::DupeService>,
    limit: usize,
) -> Vec<DupeGroupView> {
    // Phân giải bằng chỉ mục mà LƯỢT QUÉT đã dùng, không phải snapshot hiện
    // tại. `entries` là vị trí, và vị trí không sống sót qua một lần dựng lại
    // chỉ mục — dùng snapshot mới thì mỗi nhóm hiện tên và đường dẫn của tệp
    // khác, im lặng, ngay trước khi người dùng bấm xoá.
    //
    // Chưa quét lần nào thì không có gì để hiện; trả rỗng thay vì lấy tạm
    // snapshot hiện tại, vì "lấy tạm" chính là lỗi cần sửa.
    let Some((index, epoch)) = dupes.scanned_index() else {
        return Vec::new();
    };
    let props = enrich.props();

    dupes
        .groups()
        .into_iter()
        .take(limit.clamp(1, 2_000))
        .map(|g| DupeGroupView {
            size: g.size,
            wasted: g.wasted,
            epoch,
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
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems};
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
        let err = reveal_in_explorer(r"D:\definitely\not\here\nope.mp4".into()).unwrap_err();
        assert!(err.contains("không còn"));
    }
}
