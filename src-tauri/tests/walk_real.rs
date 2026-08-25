//! Walking a real network drive, on the machine that has one.
//!
//! The unit tests in `walk.rs` prove the logic against a temp directory a few
//! entries deep. They say nothing about the thing that actually decides
//! whether this feature is usable: how long a share with terabytes on it takes
//! to walk over SMB, and whether parallelism helps or the server simply
//! serialises everything anyway.
//!
//! ```text
//! cargo test --test walk_real -- --ignored --nocapture
//! ```

#![cfg(windows)]

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use mediafinder::ntfs::tree::ResolveOptions;
use mediafinder::ntfs::volume::{self, VolumeKind};
use mediafinder::walk::walk_volume;

fn network_drives() -> Vec<char> {
    volume::list_volumes()
        .into_iter()
        .filter(|v| v.kind == VolumeKind::Network)
        .map(|v| v.letter)
        .collect()
}

#[test]
#[ignore = "cần ổ mạng thật; chạy với --ignored"]
fn walk_every_network_drive_and_report_what_it_costs() {
    let drives = network_drives();
    if drives.is_empty() {
        eprintln!("bỏ qua: máy này không có ổ mạng nào");
        return;
    }

    let opts = ResolveOptions::default();
    let cancel = AtomicBool::new(false);
    let mut grand_files = 0usize;
    let mut grand_secs = 0.0;

    for letter in drives {
        let started = Instant::now();
        let mut last = Instant::now();
        let (set, stats) = walk_volume(letter, &opts, &cancel, |p| {
            // Print at most once a second: a long walk should show it is alive
            // without burying the result.
            if last.elapsed().as_secs_f64() > 1.0 {
                last = Instant::now();
                println!(
                    "    … {} thư mục · {} tệp · {} media",
                    p.dirs_done, p.files_seen, p.media_kept
                );
            }
        });
        let secs = started.elapsed().as_secs_f64();
        grand_files += set.files.len();
        grand_secs += secs;

        println!(
            "ổ {letter}: {} thư mục · {} tệp media · {} thư mục bị loại · \
             {} thư mục không đọc được  [{:.1}s]",
            set.dirs.len(),
            set.files.len(),
            set.stats.excluded,
            set.stats.orphaned,
            secs
        );
        if secs > 0.0 {
            println!(
                "         {:.0} thư mục/giây",
                set.dirs.len() as f64 / secs
            );
        }
        assert_eq!(
            stats.len(),
            set.files.len(),
            "dung lượng phải đi kèm từng tệp, không phải một lượt đo riêng"
        );
        let bytes: u64 = stats.iter().map(|s| s.size).sum();
        println!(
            "         {:.1} GB, đọc kèm ngay trong lúc duyệt",
            bytes as f64 / 1024.0 / 1024.0 / 1024.0
        );
        for (f, st) in set.files.iter().zip(&stats).take(3) {
            println!(
                "         ví dụ: {}\\{}  ({} byte)",
                set.dirs[f.dir_id as usize], f.name, st.size
            );
        }
    }

    println!("TỔNG: {grand_files} tệp media trong {grand_secs:.1}s");
    assert!(grand_files > 0, "không tìm thấy tệp media nào trên ổ mạng");
}

#[test]
#[ignore = "cần ổ mạng thật; chạy với --ignored"]
fn a_cancelled_walk_stops_instead_of_running_to_the_end() {
    // A NAS walk runs for minutes. A user who changes their mind must not have
    // to wait it out, and the cancel has to be checked often enough to matter.
    let Some(&letter) = network_drives().first() else {
        eprintln!("bỏ qua: máy này không có ổ mạng nào");
        return;
    };

    let cancel = AtomicBool::new(false);
    let opts = ResolveOptions::default();
    let started = Instant::now();

    let (set, _) = walk_volume(letter, &opts, &cancel, |p| {
        // Ask it to stop as soon as it has done any real work at all.
        if p.dirs_done > 200 {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    });

    let secs = started.elapsed().as_secs_f64();
    println!(
        "dừng sau {} thư mục · {} tệp media  [{:.1}s]",
        set.dirs.len(),
        set.files.len(),
        secs
    );
    assert!(
        secs < 60.0,
        "huỷ phải có tác dụng trong vòng vài giây, đo được {secs:.1}s"
    );
}

/// The whole thing: walk the network drives and merge into the real cache.
///
/// Deliberately touches the machine's own cache rather than a copy, because
/// the thing being tested is precisely whether merging preserves what was
/// already there. A test on a copy would prove nothing about that.
///
/// ```text
/// cargo test --test walk_real -- --ignored merge --nocapture
/// ```
#[test]
#[ignore = "quét ổ mạng thật và ghi vào cache thật; chạy với --ignored"]
fn merge_network_drives_into_the_real_cache() {
    use std::collections::BTreeMap;

    let before = match mediafinder::index::persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bỏ qua: không nạp được cache ({e})");
            return;
        }
    };
    let mut local_before: BTreeMap<u8, usize> = BTreeMap::new();
    for i in 0..before.index.len() {
        *local_before.entry(before.index.volume_of(i)).or_default() += 1;
    }
    let stamps_before = before.volumes.len();
    println!("trước: {} mục · {local_before:?}", before.index.len());

    let cancel = AtomicBool::new(false);
    let outcome = mediafinder::scan_network_volumes(&cancel);
    println!("{outcome:?}");

    let after = mediafinder::index::persist::load().expect("nạp lại cache");
    let mut per_drive: BTreeMap<u8, usize> = BTreeMap::new();
    for i in 0..after.index.len() {
        *per_drive.entry(after.index.volume_of(i)).or_default() += 1;
    }
    println!("sau:   {} mục · {per_drive:?}", after.index.len());

    // The point of the merge: local entries are not collateral damage.
    for (drive, n) in &local_before {
        if network_drives().contains(&(*drive as char)) {
            continue;
        }
        assert_eq!(
            per_drive.get(drive),
            Some(n),
            "ổ {} không được đụng tới",
            *drive as char
        );
    }
    assert!(after.index.len() > before.index.len(), "phải thêm mục mới");

    // Journal cursors belong to local drives and must survive; network drives
    // must not acquire one, or an incremental update would think it can follow
    // a journal that does not exist.
    assert_eq!(after.volumes.len(), stamps_before, "số mốc volume không đổi");
    for st in &after.volumes {
        assert!(
            !network_drives().contains(&st.letter),
            "ổ mạng {} không được có mốc journal",
            st.letter
        );
    }

    // And the result has to be searchable, not merely stored.
    let cancel2 = std::sync::atomic::AtomicU64::new(0);
    let hits = mediafinder::index::search::search(
        &after.index,
        "mp4",
        &mediafinder::index::search::SearchOptions::default(),
        &[],
        &cancel2,
        0,
    )
    .hits;
    println!("tìm 'mp4' → {} kết quả", hits.len());
    assert!(!hits.is_empty());
}
