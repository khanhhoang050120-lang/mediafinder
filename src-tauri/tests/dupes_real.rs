//! Finding duplicates across the real library, on the machine that has one.
//!
//! The unit tests in `dupes.rs` prove the logic against a handful of temp
//! files. They say nothing about the thing that decides whether the feature is
//! usable: how long three terabytes on a mechanical drive takes, and whether
//! splitting tier 2 into two passes actually helps or merely moves the work
//! around.
//!
//! ```text
//! cargo test --test dupes_real -- --ignored --nocapture
//! ```

#![cfg(windows)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use mediafinder::index::persist;

/// The scan, end to end, over whatever the cache currently holds.
///
/// Reports the numbers that matter for judging the change: how many files
/// tier 1 left to check, how long the whole thing took, and what it found.
#[test]
#[ignore = "cần chỉ mục đã quét trên máy thật; chạy với --ignored"]
fn scan_the_real_library() {
    let index = match persist::load() {
        Ok(c) => c.index,
        Err(e) => {
            eprintln!("chưa có cache ({e}) — mở MediaFinder và quét một lần trước");
            return;
        }
    };

    let total_files = index.len();
    let sizes = index.sizes();
    eprintln!("chỉ mục: {total_files} tệp");

    // Reproduce tier 1 here so the report can say how much it saved, without
    // reaching into the module's private internals.
    let mut by_size: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for &s in sizes.iter() {
        if s >= 64 * 1024 {
            *by_size.entry(s).or_default() += 1;
        }
    }
    let candidates_expected: usize = by_size.values().filter(|&&n| n > 1).sum();
    eprintln!(
        "tầng 1: {candidates_expected}/{total_files} tệp cùng dung lượng ({:.0}% bị loại miễn phí)",
        100.0 - candidates_expected as f64 / total_files as f64 * 100.0
    );

    let service = mediafinder::media::dupes::DupeService::new();
    let started = Instant::now();
    assert!(
        service.start(
            std::sync::Arc::new(index),
            0,
            mediafinder::media::dupescope::DupeScope::Everything,
            Vec::new()
        ),
        "quét phải bắt đầu"
    );

    let mut last_report = Instant::now();
    loop {
        let p = service.progress();
        if !p.running {
            break;
        }
        if last_report.elapsed().as_secs() >= 15 {
            eprintln!(
                "  … {}/{} tệp, {:.0}s trôi qua",
                p.hashed,
                p.candidates,
                started.elapsed().as_secs_f64()
            );
            last_report = Instant::now();
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    let elapsed = started.elapsed();
    let p = service.progress();
    let groups = service.groups();
    let wasted: u64 = groups.iter().map(|g| g.wasted).sum();

    eprintln!();
    eprintln!("=== KẾT QUẢ ===");
    eprintln!("thời gian     : {:.1}s", elapsed.as_secs_f64());
    eprintln!("nhóm trùng lặp: {}", groups.len());
    eprintln!(
        "có thể thu hồi: {:.1} GB",
        wasted as f64 / 1024.0 / 1024.0 / 1024.0
    );
    eprintln!("completed     : {}", p.completed);

    assert!(p.completed, "quét xong thì completed phải là true");
    // Every group must be a real group: two or more files, waste consistent
    // with the size. A scan that reports a group of one is a bug that no
    // timing number would reveal.
    for g in &groups {
        assert!(g.entries.len() >= 2, "nhóm phải có từ hai tệp trở lên");
        assert_eq!(
            g.wasted,
            g.size * (g.entries.len() as u64 - 1),
            "phần lãng phí phải bằng kích thước nhân số bản thừa"
        );
    }
    // Sorted biggest-waste-first, so somebody clearing space starts where it
    // pays most.
    for w in groups.windows(2) {
        assert!(
            w[0].wasted >= w[1].wasted,
            "phải sắp theo lãng phí giảm dần"
        );
    }
}

/// How much the head-only first pass actually saves, measured rather than
/// assumed: read the head of every candidate, count how many still collide.
///
/// The whole optimisation rests on most candidates separating on their head
/// alone. If that fraction were small, the second pass would read nearly
/// everything anyway and the split would buy nothing.
#[test]
#[ignore = "đọc đĩa thật, mất vài phút; chạy với --ignored"]
fn how_much_does_the_head_pass_separate() {
    use std::collections::HashMap;
    use std::io::Read;

    let index = match persist::load() {
        Ok(c) => c.index,
        Err(e) => {
            eprintln!("chưa có cache ({e}) — bỏ qua");
            return;
        }
    };

    let mut by_size: HashMap<u64, Vec<u32>> = HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s >= 64 * 1024 {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);
    let candidates: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&s, v)| v.iter().map(move |&i| (s, i)))
        .collect();
    eprintln!("ứng viên sau tầng 1: {}", candidates.len());

    // Head only, exactly what pass A does.
    let started = Instant::now();
    let mut by_head: HashMap<(u64, [u8; 32]), usize> = HashMap::new();
    let mut read_ok = 0usize;
    for &(size, i) in &candidates {
        let path = index.full_path(i as usize);
        let Ok(mut f) = std::fs::File::open(&path) else {
            continue;
        };
        let mut hasher = blake3::Hasher::new();
        hasher.update(&size.to_le_bytes());
        let cap = std::cmp::min(size, 64 * 1024) as usize;
        let mut buf = vec![0u8; cap];
        if f.read_exact(&mut buf).is_err() {
            continue;
        }
        hasher.update(&buf);
        read_ok += 1;
        *by_head
            .entry((size, *hasher.finalize().as_bytes()))
            .or_default() += 1;
    }
    let head_secs = started.elapsed().as_secs_f64();

    let survivors: usize = by_head.values().filter(|&&n| n > 1).sum();
    eprintln!();
    eprintln!("=== PHA A (chỉ đọc phần đầu) ===");
    eprintln!("đọc được      : {read_ok} tệp trong {head_secs:.1}s");
    eprintln!(
        "còn trùng đầu : {survivors} tệp ({:.1}% ứng viên)",
        survivors as f64 / candidates.len() as f64 * 100.0
    );
    eprintln!(
        "=> pha B chỉ phải đọc {:.1}% số tệp, tiết kiệm {} lần seek",
        survivors as f64 / candidates.len() as f64 * 100.0,
        candidates.len() - survivors
    );

    let _ = AtomicBool::new(false);
    let _ = AtomicUsize::new(0);
    let _ = Ordering::Relaxed;
}

/// Bước 0 của kế hoạch đo: **ứng viên nằm ở đâu**.
///
/// Miễn phí — không đọc đĩa một byte nào, chỉ đọc chỉ mục. Đây là phép đo phải
/// chạy trước mọi quyết định về tốc độ, vì nó trả lời bốn câu mà hôm nay đang
/// phải đoán:
///
/// * Bao nhiêu việc nằm trên ổ mạng, bao nhiêu trên đĩa trong máy — quyết định
///   nên dồn công sức vào đâu.
/// * Ứng viên phân bố theo dải dung lượng thế nào — quyết định sàn dung lượng
///   (việc E) có đáng làm không, và nâng `SMALL_FILE_LIMIT` cắt được bao nhiêu.
/// * Bao nhiêu cặp cùng `mtime` — quyết định khoá (size, mtime) ở việc G có
///   dùng được không, và trả lời luôn câu "CapCut có giữ thời gian sửa không"
///   mà người dùng nói là không biết.
/// * Có cặp nào cùng (ổ, FRN) không — hardlink, cùng một tệp vật lý xuất hiện
///   hai lần, "trùng" mà không thu hồi được gì.
#[test]
#[ignore = "cần chỉ mục đã quét trên máy thật; chạy với --ignored"]
fn buoc_0_ung_vien_nam_o_dau() {
    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e}) — mở MediaFinder và quét một lần trước");
            return;
        }
    };
    let index = cache.index;
    let sizes = index.sizes();
    let mtimes = index.mtimes();

    // Ổ mạng: lấy từ chính danh sách ổ đang gắn, vì ổ ánh xạ (`Y:\…`) trông y
    // hệt đĩa trong máy trong chỉ mục.
    let net: Vec<char> = mediafinder::ntfs::volume::list_volumes()
        .into_iter()
        .filter(|v| v.kind == mediafinder::ntfs::volume::VolumeKind::Network)
        .map(|v| v.letter.to_ascii_uppercase())
        .collect();
    eprintln!("ổ mạng đang gắn: {net:?}");

    // Tầng 1, y hệt mã sản phẩm.
    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in sizes.iter().enumerate() {
        if s >= 64 * 1024 {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    let mut theo_o: std::collections::BTreeMap<char, (usize, u64)> =
        std::collections::BTreeMap::new();
    // Dải dung lượng: (nhãn, cận trên byte).
    let dai: [(&str, u64); 6] = [
        ("64K–1M", 1 << 20),
        ("1M–4M", 4 << 20),
        ("4M–16M", 16 << 20),
        ("16M–64M", 64 << 20),
        ("64M–256M", 256 << 20),
        ("≥256M", u64::MAX),
    ];
    let mut theo_dai = [(0usize, 0u64); 6];
    let mut cung_mtime = 0usize;
    let mut tong_cap = 0usize;
    let mut cung_frn = 0usize;

    for (&size, entries) in by_size.iter() {
        // Tiềm năng thu hồi của lớp: giữ một bản, bỏ phần còn lại.
        let tiem_nang = size * (entries.len() as u64 - 1);

        for &i in entries {
            let v = index.volume_of(i as usize);
            let e = theo_o.entry(v as char).or_insert((0, 0));
            e.0 += 1;
        }
        theo_o
            .entry(index.volume_of(entries[0] as usize) as char)
            .and_modify(|e| e.1 += tiem_nang);

        let d = dai.iter().position(|&(_, tran)| size < tran).unwrap_or(5);
        theo_dai[d].0 += entries.len();
        theo_dai[d].1 += tiem_nang;

        // Cặp trong cùng lớp: cùng mtime? cùng FRN?
        for a in 0..entries.len() {
            for b in (a + 1)..entries.len() {
                tong_cap += 1;
                let (ia, ib) = (entries[a] as usize, entries[b] as usize);
                if mtimes[ia] == mtimes[ib] && mtimes[ia] != 0 {
                    cung_mtime += 1;
                }
                let (fa, fb) = (index.frn(ia), index.frn(ib));
                if fa != 0 && fa == fb && index.volume_of(ia) == index.volume_of(ib) {
                    cung_frn += 1;
                }
            }
        }
    }

    let tong: usize = theo_o.values().map(|(n, _)| n).sum();
    let tren_mang: usize = theo_o
        .iter()
        .filter(|(o, _)| net.contains(&o.to_ascii_uppercase()))
        .map(|(_, (n, _))| n)
        .sum();

    eprintln!(
        "\n=== ỨNG VIÊN TẦNG 1: {tong} tệp / {} trong chỉ mục ===",
        index.len()
    );
    eprintln!(
        "  trên ổ mạng : {tren_mang} ({:.0}%)",
        100.0 * tren_mang as f64 / tong.max(1) as f64
    );
    eprintln!(
        "  trên đĩa máy: {} ({:.0}%)",
        tong - tren_mang,
        100.0 * (tong - tren_mang) as f64 / tong.max(1) as f64
    );

    eprintln!("\n--- theo ổ ---");
    for (o, (n, tiem)) in &theo_o {
        let loai = if net.contains(&o.to_ascii_uppercase()) {
            "mạng"
        } else {
            "máy "
        };
        eprintln!(
            "  {o}: {loai} {n:>8} tệp   tiềm năng {:>8.1} GB",
            *tiem as f64 / (1u64 << 30) as f64
        );
    }

    eprintln!("\n--- theo dải dung lượng ---");
    for (i, (nhan, _)) in dai.iter().enumerate() {
        let (n, tiem) = theo_dai[i];
        eprintln!(
            "  {nhan:>9}: {n:>8} tệp ({:>4.1}%)   tiềm năng {:>8.1} GB",
            100.0 * n as f64 / tong.max(1) as f64,
            tiem as f64 / (1u64 << 30) as f64
        );
    }

    eprintln!("\n--- cặp trong cùng lớp dung lượng: {tong_cap} ---");
    eprintln!(
        "  cùng mtime  : {cung_mtime} ({:.1}%) — quyết định khoá (size, mtime) có dùng được không",
        100.0 * cung_mtime as f64 / tong_cap.max(1) as f64
    );
    eprintln!("  cùng (ổ,FRN): {cung_frn} — hardlink, 'trùng' mà không thu hồi được gì");
}
