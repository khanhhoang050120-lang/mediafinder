//! The cache must never be replaced by a worse one.
//!
//! Every volume that fails to open during a scan is skipped so the rest can
//! carry on. That is right, but it means a scan where *nothing* opened still
//! reaches the end holding an empty index — and saving that would wipe out a
//! perfectly good cache, forcing the user through another scan and another UAC
//! prompt to get back what they already had.
//!
//! Running `--index` without elevation reproduces it exactly: every volume
//! returns access-denied, and the run finishes with zero files.

#![cfg(windows)]

use std::process::Command;

use mediafinder::index::persist;

/// Run this crate's own binary with the given arguments.
fn run_indexer(args: &[&str]) -> std::process::Output {
    // The test binary lives in `target/<profile>/deps`, so the executable is
    // two directories up.
    let exe = std::env::current_exe()
        .expect("test exe path")
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("mediafinder.exe"))
        .expect("binary path");

    assert!(exe.exists(), "chưa build binary: {}", exe.display());
    Command::new(exe).args(args).output().expect("run indexer")
}

#[test]
#[ignore = "runs the indexer and touches the real cache; run with --ignored"]
fn a_scan_that_reaches_no_volume_leaves_the_cache_alone() {
    let path = match persist::cache_path() {
        Ok(p) if p.exists() => p,
        _ => {
            eprintln!("bỏ qua: chưa có cache để bảo vệ");
            return;
        }
    };

    let before = std::fs::metadata(&path).expect("stat cache").len();
    assert!(before > 0, "cache trống thì không chứng minh được gì");

    // Unelevated, so every volume fails to open — the exact failure this guard
    // exists for.
    let out = run_indexer(&["--index"]);
    // `tracing_subscriber::fmt()` writes to stdout by default; check both so
    // this does not break if that ever changes.
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        log.contains("Không quét được ổ đĩa nào"),
        "kỳ vọng lượt quét từ chối lưu, log:\n{log}"
    );

    let after = std::fs::metadata(&path).expect("stat cache").len();
    assert_eq!(
        before, after,
        "cache đã bị ghi đè bởi một lượt quét không đọc được ổ nào"
    );
}

#[test]
#[ignore = "runs the indexer; run with --ignored"]
fn a_failed_scan_reports_finished_so_the_ui_stops_polling() {
    // A progress file left without `finished` would spin the progress bar
    // forever, because the UI has no other signal that the child gave up.
    let _ = run_indexer(&["--index"]);

    let progress = mediafinder::ipc::elevate::read_progress()
        .expect("lượt quét thất bại vẫn phải ghi tiến độ");

    assert_eq!(progress.phase, "error");
    assert!(
        progress.finished,
        "phải đặt finished, nếu không UI poll mãi"
    );
    assert!(
        progress.error.is_some_and(|e| e.contains("giữ nguyên")),
        "thông báo phải trấn an rằng dữ liệu cũ còn nguyên"
    );
}

/// What is actually in the cache on this machine, broken down by drive.
///
/// A diagnostic, not an assertion: the right answer depends on what is on the
/// disks. Run it when a scan produces a count that looks wrong.
///
/// ```text
/// cargo test --test cache_safety -- --ignored inventory --nocapture
/// ```
#[test]
#[ignore = "đọc cache thật của máy này; chạy với --ignored"]
fn inventory() {
    use std::collections::BTreeMap;

    let cache = match mediafinder::index::persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("không nạp được cache: {e}");
            return;
        }
    };
    let ix = &cache.index;

    println!("schema  = {}", cache.schema_version);
    println!("tổng    = {} tệp / {} thư mục", ix.len(), ix.dir_count());

    for v in &cache.volumes {
        println!(
            "  stamp ổ {}: journal_id={:#x} next_usn={} file_count={}",
            v.letter, v.journal_id, v.next_usn, v.file_count
        );
    }

    let mut per_drive: BTreeMap<u8, usize> = BTreeMap::new();
    let mut with_frn = 0usize;
    for i in 0..ix.len() {
        *per_drive.entry(ix.volume_of(i)).or_default() += 1;
        if ix.frn(i) != 0 {
            with_frn += 1;
        }
    }
    for (drive, n) in &per_drive {
        println!("  ổ {}: {} tệp", *drive as char, n);
    }
    println!("có FRN  = {with_frn}/{}", ix.len());

    // Top-level directories, to see whether a whole branch went missing.
    let mut roots: BTreeMap<String, usize> = BTreeMap::new();
    for i in 0..ix.dir_count() {
        let p = ix.dir_path(i);
        let root: String = p.split('\\').take(2).collect::<Vec<_>>().join("\\");
        *roots.entry(root).or_default() += 1;
    }
    // Tệp thử của lượt kiểm chứng cập nhật nhanh, nếu còn.
    let mut hits = 0;
    for i in 0..ix.len() {
        // Soi cả đường dẫn, không chỉ tên tệp: một tệp trong thư mục thử mang
        // tên hoàn toàn bình thường, và lọc theo tên đã bỏ sót đúng nó.
        let path = ix.full_path(i);
        if path.contains("mf-") || path.contains("mediafinder-test") {
            println!(
                "  TỆP THỬ: {}  ({} byte, frn={})",
                path,
                ix.size(i),
                ix.frn(i)
            );
            hits += 1;
        }
    }
    println!("  → {hits} tệp thử trong index");

    println!("thư mục trên ổ C: đang có trong index:");
    for i in 0..ix.dir_count() {
        if ix.volume_of_dir(i) == b'C' {
            println!("    {}", ix.dir_path(i));
        }
    }

    println!("thư mục gốc cấp 1 ({} nhánh):", roots.len());
    for (r, n) in roots.iter().take(30) {
        println!("    {r}  ({n} thư mục con)");
    }
}
