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
// Nhập sàn từ mã sản phẩm, KHÔNG chép giá trị của nó.
//
// Bản trước viết cứng `64 * 1024` ở bảy chỗ. Khi sàn nâng lên 1 MB thì cả bảy
// phép đo báo cáo con số của phiên bản đã chết — bảng "ứng viên nằm ở đâu" nói
// 169.711 tệp trong khi lượt quét thật chỉ kiểm 112 nghìn.
use mediafinder::media::dupes::MIN_INTERESTING_SIZE;

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

    // Chép lại tầng 1 để báo cáo nói được nó lọc đi bao nhiêu — nhưng dùng
    // ĐÚNG hằng số của mã sản phẩm.
    //
    // Bản trước viết cứng `64 * 1024`. Khi sàn nâng lên 1 MB thì dòng in ra
    // vẫn nói 169.711 ứng viên trong khi lượt quét thật chỉ kiểm 112 nghìn —
    // một phép đo báo cáo con số của phiên bản đã chết. Cùng loại lỗi với bài
    // đo truyền danh sách ổ mạng rỗng: chép lại thứ mình đang đo thì không đo
    // gì cả.
    let mut by_size: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for &s in sizes.iter() {
        if s >= MIN_INTERESTING_SIZE {
            *by_size.entry(s).or_default() += 1;
        }
    }
    let candidates_expected: usize = by_size.values().filter(|&&n| n > 1).sum();
    eprintln!(
        "tầng 1: {candidates_expected}/{total_files} tệp cùng dung lượng ({:.0}% bị loại miễn phí)",
        100.0 - candidates_expected as f64 / total_files as f64 * 100.0
    );

    // `start` tự hỏi Windows, nên bài đo này không truyền gì — và vì thế
    // không thể lặp lại lỗi cũ là truyền danh sách rỗng rồi đo nhầm một cấu
    // hình app không bao giờ chạy. Chỉ in ra để đối chiếu.
    let omang = mediafinder::media::omang::OMang::tu_he_thong();
    assert!(
        !omang.unc.is_empty(),
        "máy này phải đang gắn ổ mạng thì phép đo mới có nghĩa, thấy {:?}",
        omang.chu
    );
    eprintln!("ổ mạng: {:?}", omang.chu);
    for (c, unc) in &omang.unc {
        eprintln!("  {c}: {unc}");
    }

    let service = mediafinder::media::dupes::DupeService::new();
    let started = Instant::now();
    assert!(
        service.start(
            std::sync::Arc::new(index),
            0,
            mediafinder::media::dupescope::DupeScope::Everything,
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
        if s >= MIN_INTERESTING_SIZE {
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
        if s >= MIN_INTERESTING_SIZE {
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

/// Bước 1 và 2 của kế hoạch đo: **mỗi ổ mất bao lâu, và thứ tự đọc có đổi
/// được gì không**.
///
/// Đây là phép đo quyết định việc C (pool I/O riêng theo ổ) có đáng làm hay
/// không. Tài liệu đặt ngưỡng rõ: nhận nếu nhanh hơn ≥1,5 lần trên HDD hoặc
/// NAS tăng gần tuyến tính; bỏ nếu chênh dưới 20%.
///
/// Hai bẫy phải tránh, cả hai đã cắn một lần trong dự án này:
///
/// 1. **Cache Windows làm lượt sau nhanh giả.** Mọi so sánh A/B chạy trên hai
///    mẫu ứng viên RỜI NHAU, cùng lạnh — không phải cùng một mẫu chạy hai lần.
///    Phép đo luồng ở `netsched` từng sai đúng vì chuyện này: 64 luồng "nhanh
///    hơn 11 lần" hoá ra chỉ là nó chạy sau.
/// 2. **`hashed` đếm cả tệp không mở được.** Bài này in riêng số mở lỗi.
///
/// Chạy: `cargo test --test dupes_real -- --ignored --nocapture buoc_1_2`
#[test]
#[ignore = "đọc đĩa thật, mất vài chục phút; chạy với --ignored"]
fn buoc_1_2_nen_lanh_theo_o_va_thu_tu_doc() {
    use mediafinder::media::dupes::fingerprint_pub;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e}) — mở MediaFinder và quét một lần trước");
            return;
        }
    };
    let index = cache.index;

    let net: Vec<char> = mediafinder::ntfs::volume::list_volumes()
        .into_iter()
        .filter(|v| v.kind == mediafinder::ntfs::volume::VolumeKind::Network)
        .map(|v| v.letter.to_ascii_uppercase())
        .collect();

    // Tầng 1, tách theo ổ.
    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s >= MIN_INTERESTING_SIZE {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    let mut theo_o: std::collections::BTreeMap<char, Vec<(u64, u32)>> =
        std::collections::BTreeMap::new();
    for (&size, entries) in by_size.iter() {
        for &i in entries {
            let v = index.volume_of(i as usize);
            if v != 0 {
                theo_o.entry(v as char).or_default().push((size, i));
            }
        }
    }

    // Bao nhiêu tệp mỗi mẫu. Đủ lớn để trung bình hoá, đủ nhỏ để cả bài chạy
    // trong vài chục phút chứ không phải vài giờ.
    const MAU: usize = 1_500;

    eprintln!("\n=== BƯỚC 1: nền lạnh theo từng ổ ===");
    eprintln!("(mỗi ổ một mẫu {MAU} tệp chưa từng đọc trong phiên này)\n");

    for (o, viec) in theo_o.iter() {
        let loai = if net.contains(&o.to_ascii_uppercase()) {
            "mạng"
        } else {
            "máy "
        };
        if viec.len() < MAU * 2 {
            eprintln!("  {o}: {loai} chỉ có {} ứng viên — bỏ qua", viec.len());
            continue;
        }

        // Mẫu A: 1.500 tệp đầu theo thứ tự HashMap (tức ngẫu nhiên).
        let mau: Vec<(u64, u32)> = viec.iter().take(MAU).copied().collect();
        let loi = AtomicUsize::new(0);
        let t0 = Instant::now();
        for (size, i) in &mau {
            let p = index.full_path(*i as usize);
            if fingerprint_pub(&p, *size, index.kind(*i as usize)).is_none() {
                loi.fetch_add(1, Ordering::Relaxed);
            }
        }
        let dt = t0.elapsed().as_secs_f64();
        let n_loi = loi.load(Ordering::Relaxed);
        eprintln!(
            "  {o}: {loai} {MAU} tệp trong {:6.1}s = {:6.1} tệp/giây   ({} mở lỗi)",
            dt,
            MAU as f64 / dt,
            n_loi
        );
        // Ngoại suy cho cả ổ, nói rõ là ngoại suy.
        eprintln!(
            "       → cả ổ {} tệp sẽ mất khoảng {:.0} phút (ngoại suy)",
            viec.len(),
            viec.len() as f64 / (MAU as f64 / dt) / 60.0
        );
    }

    eprintln!("\n=== BƯỚC 2: thứ tự đọc — HashMap so với FRN ===");
    eprintln!("(hai mẫu RỜI NHAU cùng lạnh, không phải một mẫu chạy hai lần)\n");

    // Chỉ đo trên ổ trong máy: FRN là số bản ghi MFT, ổ mạng không có (FRN=0).
    for (o, viec) in theo_o.iter() {
        if net.contains(&o.to_ascii_uppercase()) || viec.len() < MAU * 2 {
            continue;
        }

        // Hai mẫu rời nhau: A lấy nửa đầu, B lấy nửa sau.
        let a: Vec<(u64, u32)> = viec.iter().take(MAU).copied().collect();
        let mut b: Vec<(u64, u32)> = viec.iter().skip(MAU).take(MAU).copied().collect();
        // B sắp theo FRN — xấp xỉ thứ tự tệp được tạo trên đĩa.
        b.sort_unstable_by_key(|&(_, i)| index.frn(i as usize));

        let do_mau = |m: &[(u64, u32)]| -> f64 {
            let t = Instant::now();
            for (size, i) in m {
                let _ = fingerprint_pub(
                    &index.full_path(*i as usize),
                    *size,
                    index.kind(*i as usize),
                );
            }
            t.elapsed().as_secs_f64()
        };

        let ta = do_mau(&a);
        let tb = do_mau(&b);
        eprintln!(
            "  {o}: thứ tự HashMap {:6.1}s · thứ tự FRN {:6.1}s → {:.2}×",
            ta,
            tb,
            ta / tb
        );
        eprintln!("       ngưỡng quyết định: nhận việc C nếu ≥1,50×, bỏ nếu <1,20×");
    }
}

/// Bước 2b: **tăng luồng trên ổ chậm có giúp không**.
///
/// Bước 1 đo được `Y:` chậm hơn `D:` hai mươi lăm lần (1,2 so với 30,3 tệp mỗi
/// giây, tuần tự). Câu hỏi quyết định thiết kế của việc C là: chậm vì **độ trễ
/// mỗi thao tác** hay vì **băng thông máy chủ**?
///
/// * Nếu là độ trễ: thêm luồng thì thông lượng tăng gần tuyến tính, vì phần
///   lớn thời gian là chờ máy chủ trả lời chứ không phải truyền dữ liệu.
/// * Nếu là băng thông: thêm luồng không đổi gì, và một pool riêng cho ổ đó
///   chỉ tốn công.
///
/// Bẫy phải tránh — và nó đã cắn hai lần trong dự án này: **cache Windows làm
/// lượt sau nhanh giả**. Mỗi mức luồng dùng một mẫu RỜI NHAU, chưa ai chạm.
/// Phép đo luồng ở `netsched` từng kết luận "64 luồng nhanh hơn 11 lần" chỉ vì
/// nó chạy sau; và bước 2 ở bài trên cho `0,01×` cũng vì đúng lỗi đó.
#[test]
#[ignore = "đọc đĩa thật trên NAS; chạy với --ignored"]
fn buoc_2b_tang_luong_tren_o_cham_co_giup_khong() {
    use mediafinder::media::dupes::fingerprint_pub;
    use rayon::prelude::*;

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e})");
            return;
        }
    };
    let index = cache.index;

    // Ổ chậm nhất theo bước 1.
    const O: char = 'Y';
    const MAU: usize = 250;

    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s >= MIN_INTERESTING_SIZE {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    let viec: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&size, e)| e.iter().map(move |&i| (size, i)))
        .filter(|&(_, i)| index.volume_of(i as usize) as char == O)
        .collect();

    let muc = [1usize, 4, 8, 16];
    if viec.len() < MAU * muc.len() {
        eprintln!(
            "ổ {O}: chỉ có {} ứng viên, cần {}",
            viec.len(),
            MAU * muc.len()
        );
        return;
    }

    eprintln!("\n=== BƯỚC 2b: ổ {O}:, mỗi mức luồng một mẫu {MAU} tệp RỜI NHAU ===\n");

    let mut nen = 0.0f64;
    for (n, &luong) in muc.iter().enumerate() {
        // Mẫu rời nhau: mức thứ n lấy đoạn thứ n.
        let mau: Vec<(u64, u32)> = viec.iter().skip(n * MAU).take(MAU).copied().collect();

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(luong)
            .build()
            .expect("dựng pool");

        let t = Instant::now();
        pool.install(|| {
            mau.par_iter().for_each(|(size, i)| {
                let _ = fingerprint_pub(
                    &index.full_path(*i as usize),
                    *size,
                    index.kind(*i as usize),
                );
            });
        });
        let dt = t.elapsed().as_secs_f64();
        let tps = MAU as f64 / dt;
        if n == 0 {
            nen = tps;
        }

        eprintln!(
            "  {luong:2} luồng: {:6.1}s = {:6.2} tệp/giây   ({:.2}× so với 1 luồng)",
            dt,
            tps,
            tps / nen
        );
    }

    eprintln!("\n  Đọc kết quả:");
    eprintln!("   • tăng gần tuyến tính  → chậm vì ĐỘ TRỄ, pool riêng theo ổ đáng làm");
    eprintln!("   • gần như không đổi    → chậm vì BĂNG THÔNG, pool riêng không giúp");
}

/// So có kiểm soát: một hàng đợi chung (bản cũ) và pool riêng theo thiết bị
/// (việc C), trên **cùng một hình dạng việc**.
///
/// Vì sao cần bài này. Lượt quét đầy đủ cho 49,5 tệp/giây với việc C, so với
/// 45,0 của mốc gốc — nhưng hai lượt đó chạy trên hai chỉ mục khác nhau
/// (169.711 và 197.301 ứng viên) vào hai thời điểm khác nhau. Đó không phải
/// phép so; đó là hai con số cạnh nhau.
///
/// Hai bẫy mà bài này tránh:
///
/// * **Bộ đệm tệp của Windows.** Bốn nhánh chạy trên bốn mẫu RỜI NHAU, nên
///   không nhánh nào được nhánh trước làm ấm hộ.
/// * **Trôi theo thời gian.** Tải NAS đổi theo giờ và theo số máy đang dùng.
///   Nên chạy hai vòng, vòng hai **đảo thứ tự**: nếu NAS chậm dần trong lúc đo
///   thì cái chậm đi rơi vào cả hai bên chứ không chỉ một bên.
#[test]
#[ignore = "đọc đĩa thật, chạy tay"]
fn buoc_3_mot_hang_doi_hay_pool_rieng() {
    use mediafinder::media::dupes::fingerprint_pub;
    use rayon::prelude::*;

    // Thiết kế "chia pool theo thiết bị" đã bị chính bài này bác, nên nó KHÔNG
    // còn trong mã sản phẩm. Định nghĩa lại ngay đây để phép đo vẫn chạy lại
    // được — kết luận nào cũng phải kiểm lại được, kể cả kết luận phủ định.
    const LUONG_NOI_BO: usize = 4;
    const LUONG_MANG: usize = 16;
    fn pool_key(volume: u8, unc: &std::collections::BTreeMap<char, String>) -> String {
        match unc.get(&(volume as char).to_ascii_uppercase()) {
            Some(u) => {
                let t = u.trim_start_matches('\\');
                match t.split('\\').next().filter(|h| !h.is_empty()) {
                    Some(h) => format!(r"\\{}", h.to_ascii_lowercase()),
                    None => "local".to_string(),
                }
            }
            None => "local".to_string(),
        }
    }
    fn so_luong(key: &str) -> usize {
        if key == "local" {
            LUONG_NOI_BO
        } else {
            LUONG_MANG
        }
    }

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e})");
            return;
        }
    };
    let index = cache.index;
    let remote = mediafinder::media::omang::OMang::tu_he_thong().unc;
    assert!(
        !remote.is_empty(),
        "phải đang gắn ổ mạng thì phép đo mới có nghĩa"
    );

    // Mẫu mỗi nhánh. Ở khoảng 45 tệp/giây thì 1.500 tệp là hơn nửa phút —
    // đủ dài để một lần mở chậm không lệch kết quả, đủ ngắn để chạy bốn nhánh.
    const MAU: usize = 1500;

    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s >= MIN_INTERESTING_SIZE {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    let tat_ca: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&size, e)| e.iter().map(move |&i| (size, i)))
        .collect();

    if tat_ca.len() < MAU * 4 {
        eprintln!("chỉ có {} ứng viên, cần {}", tat_ca.len(), MAU * 4);
        return;
    }

    // Bốn mẫu rời nhau. Thứ tự trong `tat_ca` đến từ duyệt HashMap nên đã trộn
    // các ổ sẵn — đúng thứ mà bản cũ gặp phải.
    let mau: Vec<&[(u64, u32)]> = (0..4).map(|n| &tat_ca[n * MAU..(n + 1) * MAU]).collect();

    for (n, m) in mau.iter().enumerate() {
        let mut dem: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for &(_, i) in m.iter() {
            *dem.entry(pool_key(index.volume_of(i as usize), &remote))
                .or_default() += 1;
        }
        eprintln!("mẫu {n}: {dem:?}");
    }

    // Bản CŨ: một `into_par_iter()` trên pool toàn cục, mọi ổ trộn chung.
    let cu = |viec: &[(u64, u32)]| -> (f64, usize) {
        let t = Instant::now();
        let ok: usize = viec
            .par_iter()
            .filter(|&&(size, i)| {
                fingerprint_pub(&index.full_path(i as usize), size, index.kind(i as usize))
                    .is_some()
            })
            .count();
        (t.elapsed().as_secs_f64(), ok)
    };

    // Bản MỚI: chia theo thiết bị, mỗi nhóm một pool riêng, các nhóm chạy
    // cùng lúc. Pool dựng MỘT LẦN, như mã sản phẩm.
    let moi = |viec: &[(u64, u32)]| -> (f64, usize) {
        let mut theo: std::collections::HashMap<String, Vec<(u64, u32)>> = Default::default();
        for &(size, i) in viec {
            theo.entry(pool_key(index.volume_of(i as usize), &remote))
                .or_default()
                .push((size, i));
        }
        let pools: std::collections::HashMap<String, rayon::ThreadPool> = theo
            .keys()
            .filter_map(|k| {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(so_luong(k))
                    .build()
                    .ok()
                    .map(|p| (k.clone(), p))
            })
            .collect();

        let t = Instant::now();
        let ok: usize = theo
            .into_iter()
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|(k, v)| {
                let f = |v: &Vec<(u64, u32)>| {
                    v.par_iter()
                        .filter(|&&(size, i)| {
                            fingerprint_pub(
                                &index.full_path(i as usize),
                                size,
                                index.kind(i as usize),
                            )
                            .is_some()
                        })
                        .count()
                };
                match pools.get(&k) {
                    Some(p) => p.install(|| f(&v)),
                    None => f(&v),
                }
            })
            .sum();
        (t.elapsed().as_secs_f64(), ok)
    };

    eprintln!("\n=== BƯỚC 3: một hàng đợi chung so với pool riêng theo thiết bị ===");
    eprintln!("mỗi nhánh {MAU} tệp, bốn mẫu rời nhau, vòng hai đảo thứ tự\n");

    // Vòng 1: cũ trước. Vòng 2: mới trước. Trôi theo thời gian rơi đều hai bên.
    let (t_cu_1, n_cu_1) = cu(mau[0]);
    eprintln!(
        "vòng 1 · cũ : {t_cu_1:7.1}s  {:5.1} tệp/giây  ({n_cu_1} đọc được)",
        MAU as f64 / t_cu_1
    );
    let (t_moi_1, n_moi_1) = moi(mau[1]);
    eprintln!(
        "vòng 1 · mới: {t_moi_1:7.1}s  {:5.1} tệp/giây  ({n_moi_1} đọc được)",
        MAU as f64 / t_moi_1
    );

    let (t_moi_2, n_moi_2) = moi(mau[2]);
    eprintln!(
        "vòng 2 · mới: {t_moi_2:7.1}s  {:5.1} tệp/giây  ({n_moi_2} đọc được)",
        MAU as f64 / t_moi_2
    );
    let (t_cu_2, n_cu_2) = cu(mau[3]);
    eprintln!(
        "vòng 2 · cũ : {t_cu_2:7.1}s  {:5.1} tệp/giây  ({n_cu_2} đọc được)",
        MAU as f64 / t_cu_2
    );

    let cu_tb = MAU as f64 * 2.0 / (t_cu_1 + t_cu_2);
    let moi_tb = MAU as f64 * 2.0 / (t_moi_1 + t_moi_2);
    eprintln!("\n--- trung bình hai vòng ---");
    eprintln!("cũ  (một hàng đợi chung) : {cu_tb:5.1} tệp/giây");
    eprintln!("mới (pool riêng theo ổ)  : {moi_tb:5.1} tệp/giây");
    eprintln!("việc C đổi được          : {:.2}×", moi_tb / cu_tb);
    eprintln!("\nNgưỡng của tài liệu để NHẬN việc C: 1,5×.");
}

/// Nếu ăn cắp việc mới là thứ thắng, thì lời giải là **thêm luồng cho hàng đợi
/// chung**, không phải chia hàng đợi.
///
/// Bước 3 cho thấy chia pool theo thiết bị chậm hơn 15%: luồng của ổ nào chỉ
/// đọc ổ đó, xong sớm là ngồi không, trong khi hàng đợi chung thì không luồng
/// nào rảnh khi còn tệp chưa đọc. Bài này hỏi câu còn lại: hàng đợi chung với
/// NHIỀU luồng hơn số CPU thì sao — vì phần lớn thời gian mỗi luồng là **chờ
/// NAS trả lời**, không phải tính toán.
///
/// Mỗi mức một mẫu RỜI NHAU, và mức mặc định chạy cả đầu lẫn cuối để đo trôi
/// theo thời gian: hai con số đó chênh nhau bao nhiêu chính là sai số của cả
/// bảng.
#[test]
#[ignore = "đọc đĩa thật, chạy tay"]
fn buoc_4_hang_doi_chung_bao_nhieu_luong() {
    use mediafinder::media::dupes::fingerprint_pub;
    use rayon::prelude::*;

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e})");
            return;
        }
    };
    let index = cache.index;

    const MAU: usize = 1500;
    let muc = [0usize, 24, 32, 48, 64, 0]; // 0 = mặc định (số CPU)

    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s >= MIN_INTERESTING_SIZE {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    // Bỏ qua đoạn đầu mà bước 3 vừa đọc — bộ đệm Windows còn ấm ở đó.
    let tat_ca: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&size, e)| e.iter().map(move |&i| (size, i)))
        .skip(MAU * 4)
        .collect();

    if tat_ca.len() < MAU * muc.len() {
        eprintln!(
            "chỉ còn {} ứng viên lạnh, cần {}",
            tat_ca.len(),
            MAU * muc.len()
        );
        return;
    }

    eprintln!("\n=== BƯỚC 4: hàng đợi CHUNG, bao nhiêu luồng ===");
    eprintln!("mỗi mức {MAU} tệp, mẫu rời nhau, mức mặc định chạy hai lần (đầu và cuối)\n");

    let mut ket: Vec<(usize, f64)> = Vec::new();
    for (n, &luong) in muc.iter().enumerate() {
        let viec = &tat_ca[n * MAU..(n + 1) * MAU];
        let chay = || -> f64 {
            let t = Instant::now();
            let _ok: usize = viec
                .par_iter()
                .filter(|&&(size, i)| {
                    fingerprint_pub(&index.full_path(i as usize), size, index.kind(i as usize))
                        .is_some()
                })
                .count();
            t.elapsed().as_secs_f64()
        };
        let giay = if luong == 0 {
            chay()
        } else {
            match rayon::ThreadPoolBuilder::new().num_threads(luong).build() {
                Ok(p) => p.install(chay),
                Err(e) => {
                    eprintln!("không dựng được pool {luong} luồng: {e}");
                    continue;
                }
            }
        };
        let tps = MAU as f64 / giay;
        let ten = if luong == 0 {
            "mặc định".to_string()
        } else {
            format!("{luong}")
        };
        eprintln!("{ten:>9} luồng: {giay:6.1}s  {tps:5.1} tệp/giây");
        ket.push((luong, tps));
    }

    let mac_dinh: Vec<f64> = ket
        .iter()
        .filter(|(l, _)| *l == 0)
        .map(|(_, t)| *t)
        .collect();
    if mac_dinh.len() == 2 {
        let lech = (mac_dinh[1] - mac_dinh[0]).abs() / mac_dinh[0] * 100.0;
        eprintln!(
            "\nmặc định đo hai lần: {:.1} và {:.1} tệp/giây — lệch {lech:.0}%",
            mac_dinh[0], mac_dinh[1]
        );
        eprintln!(
            "Mọi chênh lệch nhỏ hơn {lech:.0}% trong bảng trên là NHIỄU, không phải kết quả."
        );
        let nen = (mac_dinh[0] + mac_dinh[1]) / 2.0;
        eprintln!("\n--- so với mặc định ({nen:.1} tệp/giây) ---");
        for (l, t) in ket.iter().filter(|(l, _)| *l != 0) {
            eprintln!("{l:>3} luồng: {:.2}×", t / nen);
        }
    }
}

/// Pool riêng ưu tiên thấp có chậm hơn pool thường không?
///
/// Bước 4 đo hàng đợi chung ở nhiều mức luồng, nhưng bằng pool rayon trơn.
/// Mã sản phẩm dùng [`dupepool::dung`], khác ở hai chỗ: luồng đặt
/// `THREAD_PRIORITY_BELOW_NORMAL`, và pool tách khỏi pool toàn cục.
///
/// Hạ ưu tiên là để ô tìm kiếm không bị khựng. Nhưng nếu nó khiến lượt quét
/// chậm đi đáng kể thì cái giá đó phải nói ra, chứ không giấu trong một hằng
/// số. Bài này đo đúng phần chênh đó.
///
/// Bốn mẫu rời nhau, vòng hai đảo thứ tự — cùng cách với bước 3.
#[test]
#[ignore = "đọc đĩa thật, chạy tay"]
fn buoc_5_uu_tien_thap_co_lam_cham_khong() {
    use mediafinder::media::dupepool;
    use mediafinder::media::dupes::fingerprint_pub;
    use rayon::prelude::*;

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e})");
            return;
        }
    };
    let index = cache.index;

    const MAU: usize = 1500;

    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s >= MIN_INTERESTING_SIZE {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    // Bỏ qua phần mà bước 3 và bước 4 vừa đọc — bộ đệm Windows còn ấm ở đó.
    let tat_ca: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&size, e)| e.iter().map(move |&i| (size, i)))
        .skip(MAU * 10)
        .collect();

    if tat_ca.len() < MAU * 4 {
        eprintln!("chỉ còn {} ứng viên lạnh, cần {}", tat_ca.len(), MAU * 4);
        return;
    }

    let chay = |v: &[(u64, u32)]| -> usize {
        v.par_iter()
            .filter(|&&(size, i)| {
                fingerprint_pub(&index.full_path(i as usize), size, index.kind(i as usize))
                    .is_some()
            })
            .count()
    };

    // Pool sản phẩm: ưu tiên thấp, tách khỏi pool toàn cục.
    let san_pham = |v: &[(u64, u32)]| -> f64 {
        let p = dupepool::dung().expect("phải dựng được pool");
        let t = Instant::now();
        p.install(|| chay(v));
        t.elapsed().as_secs_f64()
    };
    // Cùng số luồng, ưu tiên bình thường — chỉ khác đúng một thứ.
    let thuong = |v: &[(u64, u32)]| -> f64 {
        let p = rayon::ThreadPoolBuilder::new()
            .num_threads(dupepool::LUONG)
            .build()
            .expect("phải dựng được pool");
        let t = Instant::now();
        p.install(|| chay(v));
        t.elapsed().as_secs_f64()
    };

    eprintln!("\n=== BƯỚC 5: ưu tiên thấp so với ưu tiên thường ===");
    eprintln!(
        "cùng {} luồng, mỗi nhánh {MAU} tệp, mẫu rời nhau\n",
        dupepool::LUONG
    );

    let a = thuong(&tat_ca[0..MAU]);
    eprintln!(
        "vòng 1 · thường  : {a:6.1}s  {:5.1} tệp/giây",
        MAU as f64 / a
    );
    let b = san_pham(&tat_ca[MAU..MAU * 2]);
    eprintln!(
        "vòng 1 · ưu tiên thấp: {b:6.1}s  {:5.1} tệp/giây",
        MAU as f64 / b
    );
    let c = san_pham(&tat_ca[MAU * 2..MAU * 3]);
    eprintln!(
        "vòng 2 · ưu tiên thấp: {c:6.1}s  {:5.1} tệp/giây",
        MAU as f64 / c
    );
    let d = thuong(&tat_ca[MAU * 3..MAU * 4]);
    eprintln!(
        "vòng 2 · thường  : {d:6.1}s  {:5.1} tệp/giây",
        MAU as f64 / d
    );

    let tb_thuong = MAU as f64 * 2.0 / (a + d);
    let tb_thap = MAU as f64 * 2.0 / (b + c);
    eprintln!("\n--- trung bình hai vòng ---");
    eprintln!("ưu tiên thường : {tb_thuong:5.1} tệp/giây");
    eprintln!("ưu tiên thấp   : {tb_thap:5.1} tệp/giây");
    eprintln!("cái giá của ưu tiên thấp: {:.2}×", tb_thap / tb_thuong);
}

/// Bỏ đọc phần đuôi có làm lượt quét nhanh lên không, và có đổi kết quả không?
///
/// # Vì sao hỏi câu này
///
/// Đo được NAS bão hoà quanh 65 tệp/giây bất kể thêm bao nhiêu luồng. Nhưng
/// 32 luồng chia cho 66 ms mỗi lần mở lẽ ra cho **485 tệp/giây**. Lệch bảy
/// lần, nghĩa là nút thắt không phải số luồng mà là **số lần đọc trên chính
/// NAS**.
///
/// Với tệp trên 1 MB, `fingerprint` làm một lần mở rồi **hai** lần đọc: 64 KB
/// đầu, nhảy tới cuối, 64 KB cuối. Lần nhảy đó là một lần seek thật trên đĩa
/// quay của NAS. Bỏ nó là bỏ một nửa số thao tác đĩa.
///
/// # Vì sao nó có thể an toàn
///
/// Phép đo cũ cho thấy phần đầu chỉ tách được 166 trong 29.053 ứng viên
/// (0,6%) — tức khi hai tệp trùng phần đầu thì gần như luôn trùng cả đuôi.
/// Nên bỏ đuôi hầu như không sinh nhóm sai. "Hầu như" không đủ để xoá tệp,
/// nhưng đã có nút Xác minh (việc H) đọc trọn nội dung trước khi xoá.
///
/// Bài này đo CẢ HAI: nhanh hơn bao nhiêu, và **có bao nhiêu nhóm khác đi**.
/// Con số thứ hai mới là con số quyết định.
#[test]
#[ignore = "đọc đĩa thật, chạy tay"]
fn buoc_6_bo_doc_duoi_duoc_gi_va_mat_gi() {
    use rayon::prelude::*;
    use std::io::{Read, Seek, SeekFrom};

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e})");
            return;
        }
    };
    let index = cache.index;
    let omang = mediafinder::media::omang::OMang::tu_he_thong();
    assert!(!omang.chu.is_empty(), "phải đang gắn ổ mạng");

    const MAU: usize = 1200;
    const DAU: u64 = 64 * 1024;
    const NGUONG_HAI_DAU: u64 = 1024 * 1024; // SMALL_FILE_LIMIT

    // Chỉ tệp NAS và chỉ tệp ĐỦ LỚN để hiện đang đọc hai đầu — tệp nhỏ vốn đã
    // đọc trọn một lần nên không có gì để bỏ.
    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s > NGUONG_HAI_DAU {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    let mut viec: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&s, v)| v.iter().map(move |&i| (s, i)))
        .filter(|&(_, i)| {
            let v = index.volume_of(i as usize);
            v != 0
                && omang
                    .chu
                    .iter()
                    .any(|n| (*n as u8).eq_ignore_ascii_case(&v))
        })
        .collect();
    // Sắp cố định: duyệt HashMap có thứ tự ngẫu nhiên MỖI TIẾN TRÌNH, nên hai
    // lượt chạy sẽ lấy hai tập mẫu khác nhau và không so được với nhau.
    viec.sort_unstable();

    if viec.len() < MAU * 4 {
        eprintln!("chỉ có {} ứng viên NAS lớn, cần {}", viec.len(), MAU * 4);
        return;
    }
    // Lấy từ cuối danh sách: các bước đo trước duyệt theo thứ tự HashMap nên
    // đã chạm ngẫu nhiên phần đầu.
    let n = viec.len();
    let mau: Vec<&[(u64, u32)]> = (0..4)
        .map(|k| &viec[n - MAU * (k + 1)..n - MAU * k])
        .collect();

    let van_tay = |path: &str, size: u64, ca_duoi: bool| -> Option<[u8; 32]> {
        let mut f = std::fs::File::open(path).ok()?;
        let mut h = blake3::Hasher::new();
        h.update(&size.to_le_bytes());
        let mut buf = vec![0u8; DAU as usize];
        f.read_exact(&mut buf).ok()?;
        h.update(&buf);
        if ca_duoi {
            f.seek(SeekFrom::End(-(DAU as i64))).ok()?;
            f.read_exact(&mut buf).ok()?;
            h.update(&buf);
        }
        Some(*h.finalize().as_bytes())
    };

    let chay = |v: &[(u64, u32)], ca_duoi: bool| -> (f64, Vec<(u64, u32, [u8; 32])>) {
        let t = Instant::now();
        let r: Vec<(u64, u32, [u8; 32])> = v
            .par_iter()
            .filter_map(|&(size, i)| {
                van_tay(&index.full_path(i as usize), size, ca_duoi).map(|h| (size, i, h))
            })
            .collect();
        (t.elapsed().as_secs_f64(), r)
    };

    eprintln!("\n=== BƯỚC 6: bỏ đọc phần đuôi ===");
    eprintln!("chỉ tệp NAS trên 1 MB, {MAU} tệp mỗi nhánh, mẫu rời nhau, vòng hai đảo thứ tự\n");

    let (t_hai_1, _) = chay(mau[0], true);
    eprintln!(
        "vòng 1 · hai đầu: {t_hai_1:6.1}s  {:5.1} tệp/giây",
        MAU as f64 / t_hai_1
    );
    let (t_dau_1, _) = chay(mau[1], false);
    eprintln!(
        "vòng 1 · chỉ đầu: {t_dau_1:6.1}s  {:5.1} tệp/giây",
        MAU as f64 / t_dau_1
    );
    let (t_dau_2, _) = chay(mau[2], false);
    eprintln!(
        "vòng 2 · chỉ đầu: {t_dau_2:6.1}s  {:5.1} tệp/giây",
        MAU as f64 / t_dau_2
    );
    let (t_hai_2, ket_hai) = chay(mau[3], true);
    eprintln!(
        "vòng 2 · hai đầu: {t_hai_2:6.1}s  {:5.1} tệp/giây",
        MAU as f64 / t_hai_2
    );

    let tb_hai = MAU as f64 * 2.0 / (t_hai_1 + t_hai_2);
    let tb_dau = MAU as f64 * 2.0 / (t_dau_1 + t_dau_2);
    eprintln!("\n--- trung bình hai vòng ---");
    eprintln!("hai đầu : {tb_hai:5.1} tệp/giây");
    eprintln!("chỉ đầu : {tb_dau:5.1} tệp/giây");
    eprintln!("bỏ đuôi được: {:.2}×", tb_dau / tb_hai);

    // --- Phần quan trọng hơn: có đổi KẾT QUẢ không? ---
    //
    // Chạy lại mẫu 3 ở chế độ chỉ-đầu. Cùng tập tệp, nên nhóm phải giống nhau;
    // mọi khác biệt là nhóm mà bỏ đuôi gộp nhầm hai tệp khác nội dung.
    let (_, ket_dau) = chay(mau[3], false);

    let gom = |r: &[(u64, u32, [u8; 32])]| -> std::collections::BTreeSet<Vec<u32>> {
        let mut m: std::collections::HashMap<(u64, [u8; 32]), Vec<u32>> = Default::default();
        for &(s, i, h) in r {
            m.entry((s, h)).or_default().push(i);
        }
        m.into_values()
            .filter(|v| v.len() > 1)
            .map(|mut v| {
                v.sort_unstable();
                v
            })
            .collect()
    };
    let g_hai = gom(&ket_hai);
    let g_dau = gom(&ket_dau);

    eprintln!("\n--- kết quả có đổi không (mẫu 3, cùng {MAU} tệp) ---");
    eprintln!("nhóm khi đọc hai đầu: {}", g_hai.len());
    eprintln!("nhóm khi chỉ đọc đầu: {}", g_dau.len());
    let chi_o_dau: Vec<_> = g_dau.difference(&g_hai).collect();
    let chi_o_hai: Vec<_> = g_hai.difference(&g_dau).collect();
    eprintln!(
        "nhóm CHỈ có khi bỏ đuôi (nghi gộp nhầm): {}",
        chi_o_dau.len()
    );
    eprintln!(
        "nhóm mất đi khi bỏ đuôi                : {}",
        chi_o_hai.len()
    );
    for g in chi_o_dau.iter().take(5) {
        eprintln!(
            "  gộp nhầm? {:?}",
            g.iter()
                .map(|&i| index.full_path(i as usize))
                .collect::<Vec<_>>()
        );
    }
    eprintln!("\nNgưỡng của tài liệu để NHẬN: tiết kiệm ≥30% VÀ số nhóm gộp nhầm = 0.");
}

/// Bốn cách lấy vân tay, cùng một tập tệp: cái nào nhanh, cái nào ĐÚNG.
///
/// Bước 6 cho thấy bỏ đuôi được 1,70× nhưng gộp nhầm 1 nhóm trên 451. Nó cũng
/// cho thấy điều quan trọng hơn: **thứ đắt là lần nhảy đầu đọc, không phải
/// byte**. Mở tệp qua SMB tốn 66 ms; đọc thêm 1 MB chỉ tốn 18 ms.
///
/// Nếu vậy thì "đọc 1 MB liền một mạch từ đầu" cũng chỉ tốn **một** lần nhảy —
/// bằng chỉ-đọc-đầu — nhưng phân biệt tốt hơn hẳn vì có gấp mười sáu lần dữ
/// liệu. Đây là thứ bước 6 chưa thử, và là ứng viên thật cho lượt quét nhanh.
///
/// Đo cả bốn trên **cùng một tập tệp** để so nhóm được, và đo thời gian trên
/// bốn mẫu rời nhau để không nhánh nào làm ấm bộ đệm cho nhánh khác.
#[test]
#[ignore = "đọc đĩa thật, chạy tay"]
fn buoc_7_bon_cach_lay_van_tay() {
    use rayon::prelude::*;
    use std::io::{Read, Seek, SeekFrom};

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e})");
            return;
        }
    };
    let index = cache.index;
    let omang = mediafinder::media::omang::OMang::tu_he_thong();
    assert!(!omang.chu.is_empty(), "phải đang gắn ổ mạng");

    const MAU: usize = 1200;
    const K: u64 = 1024;

    #[derive(Clone, Copy)]
    enum Cach {
        HaiDau(u64),
        ChiDau(u64),
        ChiDuoi(u64),
    }
    impl Cach {
        fn ten(&self) -> String {
            match self {
                Cach::HaiDau(n) => format!("hai đầu {}K+{}K", n / K, n / K),
                Cach::ChiDau(n) => format!("chỉ đầu {}K", n / K),
                Cach::ChiDuoi(n) => format!("chỉ đuôi {}K", n / K),
            }
        }
    }

    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s > 2 * 1024 * K {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    let mut viec: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&s, v)| v.iter().map(move |&i| (s, i)))
        .filter(|&(_, i)| {
            let v = index.volume_of(i as usize);
            v != 0
                && omang
                    .chu
                    .iter()
                    .any(|n| (*n as u8).eq_ignore_ascii_case(&v))
        })
        .collect();
    viec.sort_unstable();

    let cach = [
        Cach::HaiDau(64 * K),
        Cach::ChiDau(64 * K),
        Cach::ChiDau(1024 * K),
        Cach::ChiDuoi(64 * K),
    ];
    if viec.len() < MAU * (cach.len() + 1) {
        eprintln!(
            "chỉ có {} ứng viên NAS lớn, cần {}",
            viec.len(),
            MAU * (cach.len() + 1)
        );
        return;
    }

    let van_tay = |path: &str, size: u64, c: Cach| -> Option<[u8; 32]> {
        let mut f = std::fs::File::open(path).ok()?;
        let mut h = blake3::Hasher::new();
        h.update(&size.to_le_bytes());
        let doc = |f: &mut std::fs::File, h: &mut blake3::Hasher, n: u64| -> Option<()> {
            let mut buf = vec![0u8; n as usize];
            f.read_exact(&mut buf).ok()?;
            h.update(&buf);
            Some(())
        };
        match c {
            Cach::HaiDau(n) => {
                doc(&mut f, &mut h, n)?;
                f.seek(SeekFrom::End(-(n as i64))).ok()?;
                doc(&mut f, &mut h, n)?;
            }
            Cach::ChiDau(n) => doc(&mut f, &mut h, n)?,
            Cach::ChiDuoi(n) => {
                f.seek(SeekFrom::End(-(n as i64))).ok()?;
                doc(&mut f, &mut h, n)?;
            }
        }
        Some(*h.finalize().as_bytes())
    };

    let chay = |v: &[(u64, u32)], c: Cach| -> (f64, Vec<(u64, u32, [u8; 32])>) {
        let t = Instant::now();
        let r: Vec<(u64, u32, [u8; 32])> = v
            .par_iter()
            .filter_map(|&(size, i)| {
                van_tay(&index.full_path(i as usize), size, c).map(|h| (size, i, h))
            })
            .collect();
        (t.elapsed().as_secs_f64(), r)
    };
    let gom = |r: &[(u64, u32, [u8; 32])]| -> std::collections::BTreeSet<Vec<u32>> {
        let mut m: std::collections::HashMap<(u64, [u8; 32]), Vec<u32>> = Default::default();
        for &(s, i, h) in r {
            m.entry((s, h)).or_default().push(i);
        }
        m.into_values()
            .filter(|v| v.len() > 1)
            .map(|mut v| {
                v.sort_unstable();
                v
            })
            .collect()
    };

    eprintln!("\n=== BƯỚC 7: bốn cách lấy vân tay ===");
    eprintln!("chỉ tệp NAS trên 2 MB, {MAU} tệp mỗi nhánh\n");

    // --- Thời gian: mỗi cách một mẫu RỜI NHAU ---
    let n = viec.len();
    eprintln!("--- thời gian (mẫu rời nhau, lạnh) ---");
    let mut tps: Vec<(String, f64)> = Vec::new();
    for (k, &c) in cach.iter().enumerate() {
        let m = &viec[n - MAU * (k + 1)..n - MAU * k];
        let (t, _) = chay(m, c);
        eprintln!("{:>18}: {t:6.1}s  {:5.1} tệp/giây", c.ten(), MAU as f64 / t);
        tps.push((c.ten(), MAU as f64 / t));
    }
    let nen = tps[0].1;
    eprintln!("\n--- so với cách hiện tại ({nen:.1} tệp/giây) ---");
    for (ten, t) in tps.iter().skip(1) {
        eprintln!("{ten:>18}: {:.2}×", t / nen);
    }

    // --- Đúng/sai: CÙNG một mẫu cho cả bốn cách ---
    //
    // Mẫu riêng, chưa cách nào chạm — nhưng ở đây bộ đệm ấm là chuyện tốt: ta
    // đang so KẾT QUẢ, không so thời gian, và đọc lại từ cache thì nhanh.
    let m = &viec[n - MAU * (cach.len() + 1)..n - MAU * cach.len()];
    let chuan = gom(&chay(m, Cach::HaiDau(64 * K)).1);
    eprintln!(
        "\n--- kết quả (cùng {MAU} tệp; chuẩn = hai đầu 64K, {} nhóm) ---",
        chuan.len()
    );
    for &c in cach.iter().skip(1) {
        let g = gom(&chay(m, c).1);
        let nham: Vec<_> = g.difference(&chuan).collect();
        let mat = chuan.difference(&g).count();
        eprintln!(
            "{:>18}: {} nhóm · gộp nhầm {} · mất {mat}",
            c.ten(),
            g.len(),
            nham.len()
        );
        for x in nham.iter().take(3) {
            eprintln!(
                "      {:?}",
                x.iter()
                    .map(|&i| index.full_path(i as usize))
                    .collect::<Vec<_>>()
            );
        }
    }
}

/// Đọc bao nhiêu từ đầu tệp là đủ, và rẻ tới đâu.
///
/// Bước 7 cho hai số neo: đọc 64 K từ đầu là **1,85×** so với đọc hai đầu, còn
/// đọc 1 MB từ đầu là **0,99×** — tức mười sáu lần byte tốn đúng bằng một lần
/// nhảy đầu đọc. Giả thuyết "byte rẻ" sai; nhưng tỷ lệ quy đổi đó nói rằng
/// 128 K hay 256 K gần như miễn phí so với 64 K, mà mang gấp hai tới bốn lần
/// dữ liệu để phân biệt.
///
/// Bước 7 đo đúng/sai trên 1.200 tệp và thấy 0 nhóm gộp nhầm, nhưng bước 6
/// thấy 1 nhóm trên 451 ở một mẫu khác. Mẫu 1.200 tệp quá nhỏ để nói tỷ lệ
/// sai. Bài này dùng mẫu **lớn hơn ba lần rưỡi** cho phần đúng/sai — và ở đó
/// bộ đệm ấm là chuyện tốt, vì đang so kết quả chứ không so thời gian.
#[test]
#[ignore = "đọc đĩa thật, chạy tay"]
fn buoc_8_doc_bao_nhieu_tu_dau_la_du() {
    use rayon::prelude::*;
    use std::io::{Read, Seek, SeekFrom};

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e})");
            return;
        }
    };
    let index = cache.index;
    let omang = mediafinder::media::omang::OMang::tu_he_thong();
    assert!(!omang.chu.is_empty(), "phải đang gắn ổ mạng");

    const K: u64 = 1024;
    const MAU_GIO: usize = 1200; // mẫu đo thời gian, phải lạnh
    const MAU_DUNG: usize = 4200; // mẫu kiểm đúng/sai, càng lớn càng tin được

    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s > 2 * 1024 * K {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    let mut viec: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&s, v)| v.iter().map(move |&i| (s, i)))
        .filter(|&(_, i)| {
            let v = index.volume_of(i as usize);
            v != 0
                && omang
                    .chu
                    .iter()
                    .any(|n| (*n as u8).eq_ignore_ascii_case(&v))
        })
        .collect();
    viec.sort_unstable();

    let muc = [64 * K, 128 * K, 256 * K, 512 * K];
    let can = MAU_GIO * (muc.len() + 1) + MAU_DUNG;
    if viec.len() < can {
        eprintln!("chỉ có {} ứng viên NAS lớn, cần {can}", viec.len());
        return;
    }

    // `dau = None` nghĩa là cách hiện tại: 64 K đầu + 64 K cuối.
    let van_tay = |path: &str, size: u64, dau: Option<u64>| -> Option<[u8; 32]> {
        let mut f = std::fs::File::open(path).ok()?;
        let mut h = blake3::Hasher::new();
        h.update(&size.to_le_bytes());
        match dau {
            Some(n) => {
                let mut buf = vec![0u8; n as usize];
                f.read_exact(&mut buf).ok()?;
                h.update(&buf);
            }
            None => {
                let mut buf = vec![0u8; 64 * K as usize];
                f.read_exact(&mut buf).ok()?;
                h.update(&buf);
                f.seek(SeekFrom::End(-(64 * K as i64))).ok()?;
                f.read_exact(&mut buf).ok()?;
                h.update(&buf);
            }
        }
        Some(*h.finalize().as_bytes())
    };
    let chay = |v: &[(u64, u32)], dau: Option<u64>| -> (f64, Vec<(u64, u32, [u8; 32])>) {
        let t = Instant::now();
        let r: Vec<(u64, u32, [u8; 32])> = v
            .par_iter()
            .filter_map(|&(size, i)| {
                van_tay(&index.full_path(i as usize), size, dau).map(|h| (size, i, h))
            })
            .collect();
        (t.elapsed().as_secs_f64(), r)
    };
    let gom = |r: &[(u64, u32, [u8; 32])]| -> std::collections::BTreeSet<Vec<u32>> {
        let mut m: std::collections::HashMap<(u64, [u8; 32]), Vec<u32>> = Default::default();
        for &(s, i, h) in r {
            m.entry((s, h)).or_default().push(i);
        }
        m.into_values()
            .filter(|v| v.len() > 1)
            .map(|mut v| {
                v.sort_unstable();
                v
            })
            .collect()
    };

    let n = viec.len();
    eprintln!("\n=== BƯỚC 8: đọc bao nhiêu từ đầu ===");
    eprintln!("chỉ tệp NAS trên 2 MB\n");

    eprintln!("--- thời gian ({MAU_GIO} tệp mỗi mức, mẫu rời nhau, lạnh) ---");
    let (t0, _) = chay(&viec[n - MAU_GIO..n], None);
    eprintln!(
        "{:>14}: {t0:6.1}s  {:5.1} tệp/giây",
        "hai đầu 64K",
        MAU_GIO as f64 / t0
    );
    let nen = MAU_GIO as f64 / t0;
    for (k, &m) in muc.iter().enumerate() {
        let s = &viec[n - MAU_GIO * (k + 2)..n - MAU_GIO * (k + 1)];
        let (t, _) = chay(s, Some(m));
        let tps = MAU_GIO as f64 / t;
        eprintln!(
            "{:>14}: {t:6.1}s  {tps:5.1} tệp/giây   {:.2}×",
            format!("chỉ đầu {}K", m / K),
            tps / nen
        );
    }

    let md = &viec[n - MAU_GIO * (muc.len() + 1) - MAU_DUNG..n - MAU_GIO * (muc.len() + 1)];
    eprintln!("\n--- đúng/sai ({MAU_DUNG} tệp, cùng một tập cho mọi cách) ---");
    let chuan = gom(&chay(md, None).1);
    eprintln!("chuẩn (hai đầu 64K): {} nhóm", chuan.len());
    for &m in muc.iter() {
        let g = gom(&chay(md, Some(m)).1);
        let nham: Vec<_> = g.difference(&chuan).collect();
        let mat = chuan.difference(&g).count();
        eprintln!(
            "{:>14}: {} nhóm · gộp nhầm {} · mất {mat}",
            format!("chỉ đầu {}K", m / K),
            g.len(),
            nham.len()
        );
        for x in nham.iter().take(2) {
            eprintln!(
                "      {:?}",
                x.iter()
                    .map(|&i| index.full_path(i as usize))
                    .collect::<Vec<_>>()
            );
        }
    }
}

/// Đuôi tệp mới là chỗ phân biệt, không phải đầu.
///
/// Bước 8 cho một kết quả ngược trực giác: đọc **nhiều hơn** từ đầu không giảm
/// số nhóm gộp nhầm (6 nhóm ở cả 64 K, 128 K và 256 K) mà chỉ làm chậm đi. Lý
/// do nằm trong chính danh sách tệp bị gộp nhầm — **tất cả đều là `.MP3`**.
/// Audio cùng độ dài, cùng bộ mã hoá thì phần đầu giống nhau rất dài; thứ khác
/// nhau nằm ở cuối.
///
/// Nên câu hỏi đúng không phải "đọc bao nhiêu từ đầu" mà "đọc đầu hay đọc
/// đuôi". Bài này kiểm đuôi trên đúng mẫu lớn mà bước 8 đã dùng, để hai kết
/// quả so được với nhau.
#[test]
#[ignore = "đọc đĩa thật, chạy tay"]
fn buoc_9_dau_hay_duoi() {
    use rayon::prelude::*;
    use std::io::{Read, Seek, SeekFrom};

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e})");
            return;
        }
    };
    let index = cache.index;
    let omang = mediafinder::media::omang::OMang::tu_he_thong();
    assert!(!omang.chu.is_empty(), "phải đang gắn ổ mạng");

    const K: u64 = 1024;
    const MAU_GIO: usize = 1200;
    const MAU_DUNG: usize = 4200;

    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s > 2 * 1024 * K {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);
    let mut viec: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&s, v)| v.iter().map(move |&i| (s, i)))
        .filter(|&(_, i)| {
            let v = index.volume_of(i as usize);
            v != 0
                && omang
                    .chu
                    .iter()
                    .any(|n| (*n as u8).eq_ignore_ascii_case(&v))
        })
        .collect();
    viec.sort_unstable();

    #[derive(Clone, Copy, PartialEq)]
    enum C {
        HaiDau,
        Dau(u64),
        Duoi(u64),
    }
    let ten = |c: C| match c {
        C::HaiDau => "hai đầu 64K".to_string(),
        C::Dau(n) => format!("chỉ đầu {}K", n / K),
        C::Duoi(n) => format!("chỉ đuôi {}K", n / K),
    };
    let van_tay = |path: &str, size: u64, c: C| -> Option<[u8; 32]> {
        let mut f = std::fs::File::open(path).ok()?;
        let mut h = blake3::Hasher::new();
        h.update(&size.to_le_bytes());
        let doc = |f: &mut std::fs::File, h: &mut blake3::Hasher, n: u64| -> Option<()> {
            let mut b = vec![0u8; n as usize];
            f.read_exact(&mut b).ok()?;
            h.update(&b);
            Some(())
        };
        match c {
            C::HaiDau => {
                doc(&mut f, &mut h, 64 * K)?;
                f.seek(SeekFrom::End(-(64 * K as i64))).ok()?;
                doc(&mut f, &mut h, 64 * K)?;
            }
            C::Dau(n) => doc(&mut f, &mut h, n)?,
            C::Duoi(n) => {
                f.seek(SeekFrom::End(-(n as i64))).ok()?;
                doc(&mut f, &mut h, n)?;
            }
        }
        Some(*h.finalize().as_bytes())
    };
    let chay = |v: &[(u64, u32)], c: C| -> (f64, Vec<(u64, u32, [u8; 32])>) {
        let t = Instant::now();
        let r: Vec<(u64, u32, [u8; 32])> = v
            .par_iter()
            .filter_map(|&(s, i)| van_tay(&index.full_path(i as usize), s, c).map(|h| (s, i, h)))
            .collect();
        (t.elapsed().as_secs_f64(), r)
    };
    let gom = |r: &[(u64, u32, [u8; 32])]| -> std::collections::BTreeSet<Vec<u32>> {
        let mut m: std::collections::HashMap<(u64, [u8; 32]), Vec<u32>> = Default::default();
        for &(s, i, h) in r {
            m.entry((s, h)).or_default().push(i);
        }
        m.into_values()
            .filter(|v| v.len() > 1)
            .map(|mut v| {
                v.sort_unstable();
                v
            })
            .collect()
    };

    let cach = [C::Duoi(64 * K), C::Duoi(128 * K), C::Dau(64 * K)];
    let n = viec.len();
    eprintln!("\n=== BƯỚC 9: đầu hay đuôi ===\n");

    eprintln!("--- thời gian ({MAU_GIO} tệp mỗi mức, mẫu rời nhau, lạnh) ---");
    let (t0, _) = chay(&viec[n - MAU_GIO..n], C::HaiDau);
    let nen = MAU_GIO as f64 / t0;
    eprintln!("{:>14}: {t0:6.1}s  {nen:5.1} tệp/giây", ten(C::HaiDau));
    for (k, &c) in cach.iter().enumerate() {
        let s = &viec[n - MAU_GIO * (k + 2)..n - MAU_GIO * (k + 1)];
        let (t, _) = chay(s, c);
        let tps = MAU_GIO as f64 / t;
        eprintln!(
            "{:>14}: {t:6.1}s  {tps:5.1} tệp/giây   {:.2}×",
            ten(c),
            tps / nen
        );
    }

    let md = &viec[n - MAU_GIO * (cach.len() + 1) - MAU_DUNG..n - MAU_GIO * (cach.len() + 1)];
    eprintln!("\n--- đúng/sai ({MAU_DUNG} tệp, cùng tập với bước 8) ---");
    let chuan = gom(&chay(md, C::HaiDau).1);
    eprintln!("chuẩn (hai đầu 64K): {} nhóm", chuan.len());
    for &c in cach.iter() {
        let g = gom(&chay(md, c).1);
        let nham: Vec<_> = g.difference(&chuan).collect();
        let mat = chuan.difference(&g).count();
        eprintln!(
            "{:>14}: {} nhóm · gộp nhầm {} · mất {mat}",
            ten(c),
            g.len(),
            nham.len()
        );
        for x in nham.iter().take(3) {
            eprintln!(
                "      {:?}",
                x.iter()
                    .map(|&i| index.full_path(i as usize))
                    .collect::<Vec<_>>()
            );
        }
    }
}

/// Đọc đầu cho video, giữ hai đầu cho audio.
///
/// Bước 9 chốt: chỉ đọc đầu 64 K là **2,02×** và không làm mất nhóm nào. Cái
/// giá là 4 nhóm gộp nhầm trên 1.722 — và cả bốn đều là **audio** (`.MP3`,
/// `.wav`). Cơ chế rõ ràng: audio cùng độ dài, cùng bộ mã hoá thì phần đầu
/// giống nhau rất dài, còn video thì khung hình đầu đã khác nhau ngay.
///
/// Chỉ mục đã giữ sẵn [`MediaKind`], nên phân biệt hai loại không tốn một byte
/// đọc đĩa nào. Bài này đo xem cách trộn có lấy được gần trọn 2× mà bỏ hẳn lớp
/// sai đó không.
///
/// [`MediaKind`]: mediafinder::index::model::MediaKind
#[test]
#[ignore = "đọc đĩa thật, chạy tay"]
fn buoc_10_dau_cho_video_hai_dau_cho_audio() {
    use mediafinder::index::model::MediaKind;
    use rayon::prelude::*;
    use std::io::{Read, Seek, SeekFrom};

    let cache = match persist::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chưa có cache ({e})");
            return;
        }
    };
    let index = cache.index;
    let omang = mediafinder::media::omang::OMang::tu_he_thong();
    assert!(!omang.chu.is_empty(), "phải đang gắn ổ mạng");

    const K: u64 = 1024;
    const MAU_GIO: usize = 1200;
    const MAU_DUNG: usize = 4200;

    let mut by_size: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for (i, &s) in index.sizes().iter().enumerate() {
        if s > 2 * 1024 * K {
            by_size.entry(s).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);
    let mut viec: Vec<(u64, u32)> = by_size
        .iter()
        .flat_map(|(&s, v)| v.iter().map(move |&i| (s, i)))
        .filter(|&(_, i)| {
            let v = index.volume_of(i as usize);
            v != 0
                && omang
                    .chu
                    .iter()
                    .any(|n| (*n as u8).eq_ignore_ascii_case(&v))
        })
        .collect();
    viec.sort_unstable();

    // Audio chiếm bao nhiêu trong số ứng viên? Nếu nó là đa số thì cách trộn
    // gần như không nhanh hơn cách hiện tại, và phải nói ra.
    let so_audio = viec
        .iter()
        .filter(|&&(_, i)| index.kind(i as usize) == MediaKind::Audio)
        .count();
    eprintln!("\n=== BƯỚC 10: đầu cho video, hai đầu cho audio ===");
    eprintln!(
        "ứng viên NAS lớn: {} · audio {so_audio} ({:.1}%)\n",
        viec.len(),
        so_audio as f64 / viec.len() as f64 * 100.0
    );

    // 0 = hai đầu (như hiện tại), 1 = chỉ đầu, 2 = trộn theo loại tệp.
    let van_tay = |i: u32, size: u64, cach: u8| -> Option<[u8; 32]> {
        let hai_dau = match cach {
            0 => true,
            1 => false,
            _ => index.kind(i as usize) == MediaKind::Audio,
        };
        let mut f = std::fs::File::open(index.full_path(i as usize)).ok()?;
        let mut h = blake3::Hasher::new();
        h.update(&size.to_le_bytes());
        let mut b = vec![0u8; 64 * K as usize];
        f.read_exact(&mut b).ok()?;
        h.update(&b);
        if hai_dau {
            f.seek(SeekFrom::End(-(64 * K as i64))).ok()?;
            f.read_exact(&mut b).ok()?;
            h.update(&b);
        }
        Some(*h.finalize().as_bytes())
    };
    let chay = |v: &[(u64, u32)], cach: u8| -> (f64, Vec<(u64, u32, [u8; 32])>) {
        let t = Instant::now();
        let r: Vec<(u64, u32, [u8; 32])> = v
            .par_iter()
            .filter_map(|&(s, i)| van_tay(i, s, cach).map(|h| (s, i, h)))
            .collect();
        (t.elapsed().as_secs_f64(), r)
    };
    let gom = |r: &[(u64, u32, [u8; 32])]| -> std::collections::BTreeSet<Vec<u32>> {
        let mut m: std::collections::HashMap<(u64, [u8; 32]), Vec<u32>> = Default::default();
        for &(s, i, h) in r {
            m.entry((s, h)).or_default().push(i);
        }
        m.into_values()
            .filter(|v| v.len() > 1)
            .map(|mut v| {
                v.sort_unstable();
                v
            })
            .collect()
    };

    let n = viec.len();
    let ten = ["hai đầu", "chỉ đầu", "trộn"];
    eprintln!("--- thời gian ({MAU_GIO} tệp mỗi mức, mẫu rời nhau, lạnh) ---");
    let mut tps = [0f64; 3];
    for cach in 0u8..3 {
        let s = &viec[n - MAU_GIO * (cach as usize + 1)..n - MAU_GIO * cach as usize];
        let (t, _) = chay(s, cach);
        tps[cach as usize] = MAU_GIO as f64 / t;
        eprintln!(
            "{:>8}: {t:6.1}s  {:5.1} tệp/giây",
            ten[cach as usize], tps[cach as usize]
        );
    }
    eprintln!(
        "\nchỉ đầu: {:.2}×   trộn: {:.2}×",
        tps[1] / tps[0],
        tps[2] / tps[0]
    );

    let md = &viec[n - MAU_GIO * 3 - MAU_DUNG..n - MAU_GIO * 3];
    eprintln!("\n--- đúng/sai ({MAU_DUNG} tệp) ---");
    let chuan = gom(&chay(md, 0).1);
    eprintln!("chuẩn (hai đầu): {} nhóm", chuan.len());
    for cach in 1u8..3 {
        let g = gom(&chay(md, cach).1);
        let nham: Vec<_> = g.difference(&chuan).collect();
        eprintln!(
            "{:>8}: {} nhóm · gộp nhầm {} · mất {}",
            ten[cach as usize],
            g.len(),
            nham.len(),
            chuan.difference(&g).count()
        );
        for x in nham.iter().take(3) {
            eprintln!(
                "    {:?}",
                x.iter()
                    .map(|&i| index.full_path(i as usize))
                    .collect::<Vec<_>>()
            );
        }
    }
}
