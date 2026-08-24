//! Search benchmarks.
//!
//! The plan commits to p99 under 20 ms on a 500k-entry index, and to deciding
//! on the strength of a measurement — not a guess — whether the parallel scan
//! actually earns its keep. `single_vs_parallel` exists for exactly that.

use std::sync::atomic::AtomicU64;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mediafinder::index::model::{Index, IndexBuilder, MediaKind};
use mediafinder::index::search::{search, SearchOptions};

/// Build an index that behaves like a real library rather than a uniform list.
///
/// A synthetic set of identical names would let the branch predictor and the
/// substring finder do far better than they can on real data, so this varies
/// name length, word structure, and script — and salts in Vietnamese names,
/// because folded multi-byte text is the slowest thing the scan has to chew.
fn synthetic_index(entries: usize) -> Index {
    const STEMS: &[&str] = &[
        "holiday",
        "avatar",
        "family trip",
        "screen recording",
        "Tiếng Việt phỏng vấn",
        "Đà Nẵng biển",
        "concert live",
        "backup old archive footage",
        "IMG",
        "DSC",
    ];
    const EXTS: &[(&str, MediaKind)] = &[
        ("mp4", MediaKind::Video),
        ("mkv", MediaKind::Video),
        ("jpg", MediaKind::Image),
        ("png", MediaKind::Image),
        ("mp3", MediaKind::Audio),
        ("flac", MediaKind::Audio),
    ];

    let mut b = IndexBuilder::new();
    // Roughly the file-per-directory ratio seen on the real D: volume:
    // 122k files across 5.4k directories.
    let dir_count = (entries / 22).max(1);
    let dirs: Vec<u32> = (0..dir_count)
        .map(|d| b.add_dir(&format!(r"D:\Media\collection {}\subfolder {}", d / 40, d % 40)))
        .collect();

    b.reserve(dir_count, entries);
    for i in 0..entries {
        let stem = STEMS[i % STEMS.len()];
        let (ext, kind) = EXTS[i % EXTS.len()];
        let name = match i % 4 {
            0 => format!("{stem}.{ext}"),
            1 => format!("{stem} {i}.{ext}"),
            2 => format!("{stem}_{i}_1080p_x265.{ext}"),
            _ => format!("{i:06} - {stem} (edited).{ext}"),
        };
        b.add_file(&name, kind, dirs[i % dir_count]);
    }
    b.finish()
}

fn queries(c: &mut Criterion) {
    let index = synthetic_index(500_000);
    let cancel = AtomicU64::new(0);
    let opts = SearchOptions::default();

    let mut group = c.benchmark_group("search_500k");
    group.throughput(Throughput::Elements(index.len() as u64));

    // Each case stresses a different part of the scan.
    let cases: &[(&str, &str)] = &[
        ("common_term", "holiday"),          // matches a tenth of the index
        ("rare_term", "concert"),            // few hits, still a full scan
        ("two_tokens", "avatar 1080p"),      // AND across two finders
        ("three_tokens", "family trip 2024"),
        ("vietnamese_unfolded", "tieng viet"), // the fold's whole reason to exist
        ("no_match", "zzzzzzzznothing"),     // worst case: nothing short-circuits
        ("single_char", "a"),                // matches nearly everything
    ];

    for (name, query) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), query, |b, q| {
            b.iter(|| search(black_box(&index), black_box(q), &opts, &cancel, 0));
        });
    }
    group.finish();
}

/// Does parallelism actually pay here?
///
/// The plan flagged this as a real possibility: 500k folded names are only a
/// few megabytes, and `memmem` is fast enough that rayon's coordination could
/// cost more than the scan it splits up. Limit 1 versus the default is a cheap
/// proxy for how much of the time goes into selection rather than scanning.
fn single_vs_parallel(c: &mut Criterion) {
    let index = synthetic_index(500_000);
    let cancel = AtomicU64::new(0);

    let mut group = c.benchmark_group("selection_cost");
    for limit in [1usize, 100, 5_000] {
        let opts = SearchOptions {
            limit,
            ..Default::default()
        };
        group.bench_with_input(BenchmarkId::from_parameter(limit), &opts, |b, o| {
            b.iter(|| search(black_box(&index), "holiday", o, &cancel, 0));
        });
    }
    group.finish();
}

/// Index construction, which gates how long a rescan takes.
fn build(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
    group.sample_size(10);
    for n in [100_000usize, 500_000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| synthetic_index(black_box(n)));
        });
    }
    group.finish();
}

criterion_group!(benches, queries, single_vs_parallel, build);
criterion_main!(benches);
