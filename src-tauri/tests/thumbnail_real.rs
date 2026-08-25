//! Does the shell thumbnail path actually produce a picture?
//!
//! The unit tests only prove that a missing file fails cleanly. Whether
//! `IShellItemImageFactory` returns a real frame from a real video — and
//! whether the BGRA→RGBA swap and the top-down row order are right — can only
//! be established by rendering one and looking at it.
//!
//! Ignored because it depends on the machine's own media library:
//!
//! ```text
//! cargo test --test thumbnail_real -- --ignored --nocapture
//! ```

#![cfg(windows)]

use std::sync::atomic::AtomicU64;

use mediafinder::index::persist;
use mediafinder::index::search::{search, SearchOptions};
use mediafinder::media::thumbnail::ThumbnailService;

/// Pull a few real media paths out of the on-disk index.
fn sample_paths(limit: usize) -> Vec<(String, &'static str)> {
    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bỏ qua: không nạp được cache ({e})");
            return Vec::new();
        }
    };

    let cancel = AtomicU64::new(0);
    let mut out = Vec::new();

    // One of each kind: a video frame, a photo, and an audio file's cover art
    // are three quite different code paths inside the shell.
    for (query, label) in [("mp4", "video"), ("jpg", "image"), ("mp3", "audio")] {
        let opts = SearchOptions {
            limit,
            ..Default::default()
        };
        for hit in search(&cache.index, query, &opts, &[], &cancel, 0).hits {
            let path = cache.index.full_path(hit.index as usize);
            if std::path::Path::new(&path).exists() {
                out.push((path, label));
                break;
            }
        }
    }
    out
}

#[test]
#[ignore = "needs a built index and real media files; run with --ignored"]
fn renders_real_thumbnails_from_the_users_own_library() {
    let paths = sample_paths(40);
    if paths.is_empty() {
        eprintln!("bỏ qua: không tìm thấy tệp media nào để thử");
        return;
    }

    let service = ThumbnailService::new();
    let out_dir = std::env::temp_dir().join("mediafinder-thumbs");
    std::fs::create_dir_all(&out_dir).expect("create output dir");

    let mut rendered = 0;
    for (id, (path, label)) in paths.iter().enumerate() {
        let started = std::time::Instant::now();
        match service.get(id as u64, path, 192) {
            Ok(png) => {
                // A PNG signature is the cheapest proof that what came back is
                // an image and not, say, an empty buffer.
                assert_eq!(
                    &png[..8],
                    &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
                    "không phải PNG hợp lệ"
                );
                assert!(png.len() > 200, "PNG quá nhỏ, nhiều khả năng là ảnh rỗng");

                let file = out_dir.join(format!("{label}.png"));
                std::fs::write(&file, png.as_slice()).expect("write png");
                println!(
                    "  {label:<6} {:>6} byte  {:>5.0}ms  {}",
                    png.len(),
                    started.elapsed().as_secs_f64() * 1000.0,
                    file.display()
                );
                println!("          nguồn: {path}");
                rendered += 1;
            }
            Err(e) => println!("  {label:<6} KHÔNG DỰNG ĐƯỢC: {e}\n          nguồn: {path}"),
        }
    }

    assert!(rendered > 0, "không dựng được thumbnail nào");
}

#[test]
#[ignore = "needs a built index; run with --ignored"]
fn the_cache_makes_a_second_request_far_cheaper() {
    let paths = sample_paths(40);
    let Some((path, _)) = paths.first() else {
        eprintln!("bỏ qua: không có tệp để thử");
        return;
    };

    let service = ThumbnailService::new();

    let cold = std::time::Instant::now();
    let first = service.get(0, path, 192).expect("first render");
    let cold = cold.elapsed();

    let warm = std::time::Instant::now();
    let second = service.get(0, path, 192).expect("cached render");
    let warm = warm.elapsed();

    println!(
        "  lần đầu {:.2}ms · lần sau {:.3}ms · {} byte",
        cold.as_secs_f64() * 1000.0,
        warm.as_secs_f64() * 1000.0,
        first.len()
    );

    assert!(
        std::sync::Arc::ptr_eq(&first, &second),
        "phải trả về cùng một bộ nhớ đệm"
    );
    assert!(
        warm < cold || warm.as_micros() < 200,
        "lần thứ hai phải gần như tức thì, đo được {warm:?}"
    );
}

/// How well does the shell make thumbnails for files on a network drive?
///
/// 87% of this machine's library is on a NAS. If thumbnails do not work there,
/// the grid view — the whole point of which is finding footage by eye — is
/// blank for almost everything.
///
/// ```text
/// cargo test --test thumbnail_real -- --ignored network --nocapture
/// ```
#[test]
#[ignore = "cần ổ mạng thật và cache đã có; chạy với --ignored"]
fn network_thumbnails() {
    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bỏ qua: {e}");
            return;
        }
    };
    let ix = &cache.index;

    // A spread across the index rather than the first N: the first thousand
    // entries are all one folder, and one folder is not a sample.
    let mut paths: Vec<(String, char)> = Vec::new();
    let step = (ix.len() / 400).max(1);
    let mut i = 0;
    while i < ix.len() && paths.len() < 40 {
        let drive = ix.volume_of(i) as char;
        if matches!(drive, 'F' | 'Y' | 'Z') {
            paths.push((ix.full_path(i), drive));
        }
        i += step;
    }
    if paths.is_empty() {
        eprintln!("bỏ qua: không có mục nào trên ổ mạng");
        return;
    }

    let service = ThumbnailService::new();
    let mut ok = 0;
    let mut failed = 0;
    let mut total_ms = 0.0;
    let mut slowest = 0.0f64;

    for (n, (path, drive)) in paths.iter().enumerate() {
        let started = std::time::Instant::now();
        let result = service.get(n as u64, path, 192);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        total_ms += ms;
        slowest = slowest.max(ms);
        match result {
            Ok(png) => {
                ok += 1;
                if n < 5 {
                    println!("  {drive}: {:>6} byte {:>7.0}ms  {path}", png.len(), ms);
                }
            }
            Err(e) => {
                failed += 1;
                if failed <= 5 {
                    println!("  {drive}: HỎNG {:>7.0}ms  {e}  {path}", ms);
                }
            }
        }
    }

    println!(
        "\nổ mạng: {ok}/{} dựng được thumbnail · trung bình {:.0}ms · chậm nhất {:.0}ms",
        paths.len(),
        total_ms / paths.len() as f64,
        slowest
    );
}
