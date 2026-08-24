//! Does `IPropertyStore` actually return a video's dimensions and duration?
//!
//! Property handlers are supplied by codecs, not by the OS, so what works on
//! one machine may return nothing on another. The only way to know is to ask
//! it about real files and print what comes back.
//!
//! ```text
//! cargo test --test metadata_real -- --ignored --nocapture
//! ```

#![cfg(windows)]

use std::sync::atomic::AtomicU64;

use mediafinder::index::persist;
use mediafinder::index::search::{search, SearchOptions};
use mediafinder::media::metadata::{file_stats, media_props};

#[test]
#[ignore = "needs a built index and real media; run with --ignored"]
fn reads_dimensions_and_duration_from_real_files() {
    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bỏ qua: không nạp được cache ({e})");
            return;
        }
    };

    // COM is per-thread and the property handlers expect an apartment.
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
    }

    let cancel = AtomicU64::new(0);
    let mut checked = 0;
    let mut with_props = 0;

    for (query, label) in [("mp4", "video"), ("jpg", "image"), ("mp3", "audio")] {
        let opts = SearchOptions { limit: 60, ..Default::default() };
        let mut shown = 0;
        for hit in search(&cache.index, query, &opts, &[], &cancel, 0).hits {
            if shown >= 3 {
                break;
            }
            let path = cache.index.full_path(hit.index as usize);
            if !std::path::Path::new(&path).exists() {
                continue;
            }
            shown += 1;
            checked += 1;

            let started = std::time::Instant::now();
            let props = media_props(&path).unwrap_or_default();
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            let stats = file_stats(&path).unwrap_or_default();

            if !props.is_empty() {
                with_props += 1;
            }
            println!(
                "  {label:<6} {:>5}x{:<5} {:>8} ms  {:>10} byte  {:>6.1}ms đọc",
                props.width,
                props.height,
                props.duration_ms,
                stats.size,
                elapsed
            );
            println!("         {}", std::path::Path::new(&path).file_name().unwrap().to_string_lossy());
        }
    }

    println!("\n  đọc được thuộc tính cho {with_props}/{checked} tệp");
    assert!(checked > 0, "không có tệp nào để thử");
    assert!(
        with_props > 0,
        "không đọc được thuộc tính của bất kỳ tệp nào — property handler có vấn đề"
    );
}
