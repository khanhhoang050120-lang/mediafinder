//! Integration tests for the Win32 shell calls behind "Mở tệp" and
//! "Mở thư mục chứa tệp".
//!
//! These are the parts of the project a unit test cannot reach: COM
//! initialisation, PIDL construction, and `ShellExecuteW`'s peculiar return
//! convention. Everything else can be checked against synthetic data, but
//! whether `SHOpenFolderAndSelectItems` actually highlights the right file can
//! only be established by calling it.
//!
//! They are `#[ignore]`d because they have visible side effects — an Explorer
//! window opens. Run them deliberately:
//!
//! ```text
//! cargo test --test shell_win32 -- --ignored --test-threads=1
//! ```

#![cfg(windows)]

use std::fs;
use std::path::PathBuf;

use mediafinder::ipc::commands::shell;

/// Create a file in a temp folder, deliberately including the characters that
/// break the `explorer.exe /select,"..."` approach.
fn make_awkward_file() -> PathBuf {
    let dir = std::env::temp_dir().join("mediafinder-test");
    fs::create_dir_all(&dir).expect("create temp dir");

    // Commas are the reason this project uses the COM API: Explorer parses its
    // own command line and there is no way to escape a comma inside the path.
    // Vietnamese diacritics are here because every real filename on the target
    // machine has them.
    let path = dir.join("Bài 13, Tiếng Việt — Đà Nẵng (thử).mp4");
    fs::write(&path, b"not really a video").expect("write temp file");
    path
}

#[test]
#[ignore = "opens a File Explorer window; run with --ignored"]
fn reveal_selects_a_file_whose_name_contains_a_comma() {
    let path = make_awkward_file();
    let path_str = path.to_string_lossy().to_string();

    let result = shell::open_folder_and_select(&path_str);
    assert!(
        result.is_ok(),
        "SHOpenFolderAndSelectItems failed for {path_str}: {result:?}"
    );

    println!("Explorer nên đang mở và bôi đen: {path_str}");
}

#[test]
#[ignore = "opens a File Explorer window; run with --ignored"]
fn reveal_reports_an_error_for_a_path_that_cannot_exist() {
    // `ILCreateFromPathW` is documented to fail for a nonexistent path, which
    // is the branch that must not dereference a null PIDL.
    let result = shell::open_folder_and_select(r"D:\no\such\folder\nope.mp4");
    assert!(result.is_err(), "expected an error, got {result:?}");
}

#[test]
fn shell_execute_reports_failure_rather_than_pretending_to_succeed() {
    // No Explorer window and no COM: this only checks that a path Windows
    // cannot possibly open comes back as an `Err`, exercising the
    // "return value above 32 means success" convention that is easy to get
    // backwards.
    let result = shell::open_with_default_app(r"D:\definitely\not\here\nope.zzzz");
    assert!(result.is_err(), "expected an error, got {result:?}");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("không mở được"),
        "error should be phrased for the user, got: {msg}"
    );
}

/// The guard between a stale index entry and the application aborting.
///
/// The drag crate builds a shell item list and unwraps the result. A path the
/// shell will not resolve makes that `None`, and the panic lands inside the
/// window procedure where it cannot unwind — so it does not fail the drag, it
/// kills the process. `can_make_pidl` asks the same question first.
#[test]
fn the_shell_accepts_a_real_path_and_refuses_one_that_is_not_there() {
    use mediafinder::ipc::drag_source::shell_accepts as can_make_pidl;

    let real = std::env::current_exe().expect("đường dẫn tệp chạy");
    assert!(can_make_pidl(&real), "tệp có thật phải được chấp nhận");

    let gone = real.with_file_name("khong-bao-gio-ton-tai-9f3a1c.mp4");
    assert!(
        !can_make_pidl(&gone),
        "tệp không tồn tại phải bị từ chối — đây chính là trường hợp làm crate abort"
    );

    // A directory is a perfectly good drag item, so it must not be refused.
    let dir = real.parent().expect("thư mục cha");
    assert!(can_make_pidl(dir), "thư mục cũng phải được chấp nhận");
}

/// Why dragging a file from a network drive killed the application.
///
/// The drag crate canonicalises every path before building the shell item
/// list. On a mapped network drive `std::fs::canonicalize` resolves the
/// mapping and returns the `\?\UNC\...` form — which `ILCreateFromPathW`
/// refuses. The crate then unwraps the resulting `None` inside the window
/// procedure, where a panic cannot unwind, and the process aborts.
///
/// ```text
/// cargo test --test shell_win32 -- --ignored canonical --nocapture
/// ```
#[test]
#[ignore = "cần ổ mạng thật; chạy với --ignored"]
fn canonical_form_of_a_network_path_and_whether_the_shell_accepts_it() {
    use mediafinder::ipc::drag_source::shell_accepts as can_make_pidl;
    use mediafinder::ntfs::volume::{self, VolumeKind};

    let Some(drive) = volume::list_volumes()
        .into_iter()
        .find(|v| v.kind == VolumeKind::Network)
        .map(|v| v.letter)
    else {
        eprintln!("bỏ qua: máy này không có ổ mạng");
        return;
    };

    // Any file on that drive will do.
    let root = format!("{drive}:\\");
    let Some(file) = std::fs::read_dir(&root).ok().and_then(|rd| {
        rd.flatten()
            .find(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.path())
            .or_else(|| {
                // Nothing loose at the root; look one level down.
                std::fs::read_dir(&root).ok()?.flatten().find_map(|e| {
                    std::fs::read_dir(e.path())
                        .ok()?
                        .flatten()
                        .find_map(|f| f.file_type().ok()?.is_file().then(|| f.path()))
                })
            })
    }) else {
        eprintln!("bỏ qua: không tìm thấy tệp nào trên ổ {drive}:");
        return;
    };

    println!("  gốc:        {}", file.display());
    println!("  shell nhận: {}", can_make_pidl(&file));

    match dunce::canonicalize(&file) {
        Ok(canon) => {
            let ok = can_make_pidl(&canon);
            println!("  chuẩn hoá:  {}", canon.display());
            println!("  shell nhận: {ok}");
            assert!(
                ok,
                "dạng chuẩn hoá bị shell từ chối — đây chính là đường dẫn crate `drag` \
                 truyền vào, và là nguyên nhân ứng dụng tự tắt khi kéo tệp trên ổ mạng"
            );
        }
        Err(e) => println!("  chuẩn hoá thất bại: {e}"),
    }
}

/// Whether the shell will carry a file that lives on a mapped network drive.
///
/// Dragging local files works and dragging NAS files silently delivers nothing,
/// so the question is where the network path is lost. This asks the part that
/// can be asked without a mouse: `DoDragDrop` needs a real drag gesture, but
/// building the data object does not.
#[test]
fn the_shell_builds_a_data_object_for_a_file_on_a_network_drive() {
    use mediafinder::ipc::drag_source::{data_object_for, shell_accepts};
    use mediafinder::ntfs::volume::{list_volumes, VolumeKind};

    let Some(vol) = list_volumes()
        .into_iter()
        .find(|v| v.kind == VolumeKind::Network)
    else {
        eprintln!("máy này không có ổ mạng — bỏ qua");
        return;
    };
    let root = format!("{}:\\", vol.letter);

    // Any file will do; the point is the drive it sits on.
    let Some(file) = std::fs::read_dir(&root)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .find_map(|e| {
            let p = e.path();
            let sub = std::fs::read_dir(&p).ok()?;
            sub.flatten().find(|e| e.path().is_file()).map(|e| e.path())
        })
    else {
        eprintln!("không tìm được tệp nào trên {root} — bỏ qua");
        return;
    };

    eprintln!("tệp thử: {}", file.display());
    eprintln!("shell nhận đường dẫn: {}", shell_accepts(&file));

    match data_object_for(&[file.as_path()]) {
        Ok(_) => eprintln!("dựng được data object cho tệp trên ổ mạng"),
        Err(e) => panic!("shell từ chối dựng data object: {e}"),
    }
}

/// How long the shell takes to build the data object, local versus network.
///
/// Dragging a NAS file delivers nothing while the same gesture on a local file
/// works, and the data object itself builds fine — so the remaining suspect is
/// the clock. The drag only begins once this returns; if it takes longer than
/// the user's gesture, the button is already up by then and there is nothing
/// left to drag.
#[test]
fn how_long_the_shell_takes_to_build_a_data_object() {
    use mediafinder::ipc::drag_source::data_object_for;
    use mediafinder::ntfs::volume::{list_volumes, VolumeKind};
    use std::time::Instant;

    fn first_file(root: &str) -> Option<std::path::PathBuf> {
        std::fs::read_dir(root).ok()?.flatten().find_map(|e| {
            let sub = std::fs::read_dir(e.path()).ok()?;
            sub.flatten().find(|e| e.path().is_file()).map(|e| e.path())
        })
    }

    let mut cases: Vec<(&str, std::path::PathBuf)> = Vec::new();
    if let Some(p) = first_file("D:\\") {
        cases.push(("cục bộ", p));
    }
    if let Some(vol) = list_volumes()
        .into_iter()
        .find(|v| v.kind == VolumeKind::Network)
    {
        if let Some(p) = first_file(&format!("{}:\\", vol.letter)) {
            cases.push(("ổ mạng", p));
        }
    }

    for (nhãn, path) in &cases {
        // Twice: the first call pays for whatever the shell caches.
        for lần in 1..=2 {
            let t = Instant::now();
            let ok = data_object_for(&[path.as_path()]).is_ok();
            eprintln!(
                "{nhãn} lần {lần}: {:>7.1} ms  (dựng được: {ok})  {}",
                t.elapsed().as_secs_f64() * 1000.0,
                path.display()
            );
        }
    }
}

/// A data object for files that do not share a drive.
///
/// One NAS file drags fine and three local files drag fine, but a set mixing
/// the two delivers nothing. Explorer never has to do this — a folder view
/// shows one folder — so the shell may simply decline the mixed set.
#[test]
fn a_data_object_for_files_spread_across_drives() {
    use mediafinder::ipc::drag_source::data_object_for;
    use mediafinder::ntfs::volume::{list_volumes, VolumeKind};

    fn first_file(root: &str) -> Option<std::path::PathBuf> {
        std::fs::read_dir(root).ok()?.flatten().find_map(|e| {
            let sub = std::fs::read_dir(e.path()).ok()?;
            sub.flatten().find(|e| e.path().is_file()).map(|e| e.path())
        })
    }
    fn two_files(root: &str) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(root)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let sub = std::fs::read_dir(e.path()).ok()?;
                Some(
                    sub.flatten()
                        .filter(|e| e.path().is_file())
                        .map(|e| e.path())
                        .take(2)
                        .collect::<Vec<_>>(),
                )
            })
            .find(|v: &Vec<_>| v.len() == 2)
            .unwrap_or_default()
    }

    let local = two_files("D:\\");
    let Some(vol) = list_volumes()
        .into_iter()
        .find(|v| v.kind == VolumeKind::Network)
    else {
        eprintln!("máy này không có ổ mạng — bỏ qua");
        return;
    };
    let Some(nas) = first_file(&format!("{}:\\", vol.letter)) else {
        return;
    };
    if local.len() < 2 {
        return;
    }

    let cùng_ổ: Vec<&std::path::Path> = local.iter().map(|p| p.as_path()).collect();
    let khác_ổ: Vec<&std::path::Path> = vec![local[0].as_path(), nas.as_path()];

    eprintln!("cùng một ổ  : {:?}", data_object_for(&cùng_ổ).is_ok());
    for p in &khác_ổ {
        eprintln!("   {}", p.display());
    }
    match data_object_for(&khác_ổ) {
        Ok(_) => eprintln!("khác ổ      : dựng được"),
        Err(e) => eprintln!("khác ổ      : shell TỪ CHỐI — {e}"),
    }
}
