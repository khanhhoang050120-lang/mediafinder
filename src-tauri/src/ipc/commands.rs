//! Tauri commands.
//!
//! Every command returns `Result<_, String>` and never unwraps anything that
//! came from the frontend or the filesystem. That is the other half of the
//! `panic = "abort"` decision recorded in `docs/risk.md` (RISK-001): rather
//! than argue about whether a panic unwinds, don't panic. A path that vanished
//! between the scan and the click becomes a message on screen, not a dead
//! application.

use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::State;

use crate::index::model::MediaKind;
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
    id: u64,
    query: String,
    kinds: Vec<String>,
    limit: usize,
) -> Result<SearchResponse, String> {
    state.begin_query(id);

    // Snapshot, then release the shared cell immediately — see `state.rs`.
    let index = state.snapshot();
    let opts = SearchOptions {
        limit: limit.clamp(1, 20_000),
        kinds: parse_kinds(&kinds),
    };

    // Run inline rather than on a blocking pool. The search itself already
    // fans out across every core through rayon, and at a few milliseconds for
    // half a million entries it is far too short to be worth another hop.
    let started = std::time::Instant::now();
    let outcome = run_search(&index, &query, &opts, state.generation(), id);
    let elapsed = started.elapsed();

    // A superseded search returns nothing; the frontend drops it by id anyway.
    if state.generation().load(Ordering::Relaxed) != id {
        return Ok(SearchResponse {
            id,
            hits: Vec::new(),
            elapsed_ms: elapsed.as_secs_f64() * 1000.0,
            searched: index.len(),
            relaxed: None,
        });
    }

    let hits = outcome
        .hits
        .into_iter()
        .map(|h| {
            let i = h.index as usize;
            SearchHit {
                name: index.name(i).to_string(),
                path: index.full_path(i),
                dir: index.dir(i).to_string(),
                kind: index.kind(i).as_str(),
                score: h.score,
                matched: h.matched,
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
    })
}

#[tauri::command]
pub fn index_status(state: State<'_, AppState>) -> IndexMeta {
    state.meta()
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
