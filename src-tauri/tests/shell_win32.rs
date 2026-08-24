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
