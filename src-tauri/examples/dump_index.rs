//! Xem nhanh nội dung "database" của MediaFinder
//! (`%LOCALAPPDATA%\MediaFinder\index.bin`).
//!
//! Chạy: `cargo run --example dump_index [-- <số dòng>]`

use mediafinder::index::model::MediaKind;
use mediafinder::index::persist;

fn main() {
    let rows: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Không đọc được cache: {e}");
            std::process::exit(1);
        }
    };

    println!("=== index.bin ===");
    println!("Phiên bản schema : {}", cache.schema_version);
    println!("Quét lúc (unix)  : {}", cache.built_at_unix);
    println!();
    println!("--- Volume đã quét ---");
    for v in &cache.volumes {
        println!(
            "  {}:  serial={:#010x}  journal_id={:#x}  next_usn={}  files={}",
            v.letter, v.serial, v.journal_id, v.next_usn, v.file_count
        );
    }

    let idx = &cache.index;
    let (mut nv, mut ni, mut na) = (0usize, 0usize, 0usize);
    let mut total_bytes = 0u64;
    for i in 0..idx.len() {
        match idx.kind(i) {
            MediaKind::Video => nv += 1,
            MediaKind::Image => ni += 1,
            MediaKind::Audio => na += 1,
        }
        total_bytes += idx.size(i);
    }

    println!();
    println!("--- Thống kê ---");
    println!("Tổng số tệp      : {}", idx.len());
    println!("  Video          : {nv}");
    println!("  Ảnh            : {ni}");
    println!("  Âm thanh       : {na}");
    println!("Số thư mục       : {}", idx.dir_count());
    println!("Tổng dung lượng  : {:.2} GB", total_bytes as f64 / 1e9);

    println!();
    println!("--- {rows} dòng đầu ---");
    for i in 0..idx.len().min(rows) {
        println!(
            "[{i}] {:5}  {:>12} B  {}\\{}",
            idx.kind(i).as_str(),
            idx.size(i),
            idx.dir(i),
            idx.name(i)
        );
    }
}
