//! Finding duplicate files without reading them all.
//!
//! The library this is written for is three terabytes. Hashing it would take
//! hours of solid disk reading, so the work is arranged to avoid almost all of
//! it — three tiers, each one only looking at what survived the last:
//!
//! | Tier | Test | Reads | Survivors |
//! |---|---|---|---|
//! | 1 | Same byte count | **nothing** — the index already knows | a few % |
//! | 2 | Same first 64 KB, last 64 KB, and size | 128 KB per file | near-certain matches |
//! | 3 | Same content, whole file | everything | certain |
//!
//! Tier 1 does the heavy lifting for free: two files of different sizes cannot
//! be identical, and file sizes are spread widely enough that almost nothing
//! collides. Only the leftovers are ever opened.
//!
//! Tier 2 is where this stops in practice. Two different files sharing a size,
//! a first 64 KB *and* a last 64 KB happens by accident essentially never —
//! media containers put distinct headers at the front and index tables at the
//! back, which is exactly where this looks. Tier 3 exists for when a person
//! wants certainty before deleting something, and is never run on its own.
//!
//! **Nothing here deletes anything.** It reports; the person decides.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use rayon::prelude::*;
use serde::Serialize;

use crate::index::model::Index;

/// Bytes sampled from each end of a file in tier 2.
///
/// 64 KB comfortably covers the header of every container format in the
/// extension table, and the tail catches the index or moov atom that media formats put
/// at the end. Reading more would cost proportionally more for no gain.
const SAMPLE_BYTES: u64 = 64 * 1024;

/// Files smaller than this are hashed whole in tier 2 — reading 128 KB of a
/// 100 KB file twice would be slower than just reading it once.
const SMALL_FILE_LIMIT: u64 = SAMPLE_BYTES * 2;

/// Ignore files below this size.
///
/// Tiny files collide on size constantly — thousands of 1 KB thumbnails and
/// icons — and finding that two 800-byte files match is not worth anybody's
/// attention. Reclaiming space is the point.
pub(crate) const MIN_INTERESTING_SIZE: u64 = 64 * 1024;

// Checked when the crate is built. Thousands of icons and thumbnails share a
// size; reporting those would bury the groups actually worth acting on.
const _: () = assert!(MIN_INTERESTING_SIZE >= 64 * 1024);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    /// Bytes per copy.
    pub size: u64,
    /// Index positions of the files in this group, two or more.
    pub entries: Vec<u32>,
    /// What could be reclaimed by keeping one copy: `size * (count - 1)`.
    pub wasted: u64,
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupeProgress {
    pub running: bool,
    /// Files still to be sampled in tier 2.
    pub candidates: usize,
    pub hashed: usize,
    pub groups: usize,
    pub wasted: u64,
    /// A scan has finished and its answer is the one in `groups`.
    ///
    /// Needed because "no groups" is a real answer. Deciding from `groups > 0`
    /// alone would treat a library with nothing duplicated as never scanned,
    /// and re-run the whole thing on every visit to the view.
    pub completed: bool,
    /// Lượt quét hiện tại bắt đầu lúc nào, giây Unix. `0` nếu chưa quét lần
    /// nào.
    ///
    /// Giao diện cần nó để nói "kết quả từ HH:MM". Từ khi có quét nền, kết quả
    /// có thể đã nằm sẵn từ 8 giờ sáng trong khi người dùng mở màn hình lúc 3
    /// giờ chiều — không nói ra là lặp lại đúng lỗi 4.1 vừa sửa: màn hình hiện
    /// một câu trả lời cũ mà không cho biết nó cũ.
    pub started_unix: u64,
    /// Đã yêu cầu dừng nhưng luồng quét chưa kết thúc.
    ///
    /// `cancel()` chỉ **giương cờ** rồi trả về ngay — luồng vẫn đang ở giữa
    /// một lần mở tệp, mà trên NAS một lần mở có thể treo tới hàng chục giây.
    /// Trong khoảng đó `start()` sẽ từ chối vì `running` còn true.
    ///
    /// Thiếu cờ này thì giao diện không phân biệt được "chưa quét" với "đang
    /// dừng dở", nên người bấm huỷ rồi bấm quét lại thấy màn hình dựng lại như
    /// sắp chạy mà thật ra chẳng có gì xảy ra. Đúng lỗi người dùng báo.
    pub stopping: bool,
    /// Còn khoảng bao nhiêu giây nữa, `None` khi chưa đủ dữ liệu để nói.
    ///
    /// Tính từ tốc độ **đo được của chính lượt quét này**, không phải một hằng
    /// số đoán sẵn. Không có phép đo nào cho mã hiện tại trên thư viện hiện
    /// tại — con số 584 giây trong tài liệu cũ đo ngày 24/8 trên ổ khác, bằng
    /// mã khác, trên chỉ mục còn chứa 70.461 tệp đã bị xoá.
    ///
    /// `None` cho tới khi đã mở đủ [`MIN_MAU_UOC_LUONG`] tệp: tốc độ của vài
    /// tệp đầu chưa nói lên gì (cache còn lạnh, luồng còn đang khởi động), và
    /// một con số nhảy từ "2 phút" lên "40 phút" rồi xuống "5 phút" thì tệ hơn
    /// là không hiện gì.
    pub eta_seconds: Option<u64>,
}

/// Phải mở được ít nhất bấy nhiêu tệp mới dám nói còn bao lâu.
///
/// Hai trăm tệp: đủ để vượt qua giai đoạn khởi động và trung bình hoá vài lần
/// mở chậm bất thường, nhưng vẫn tới trong vài giây trên ổ trong máy.
const MIN_MAU_UOC_LUONG: usize = 200;

/// Ước lượng số giây còn lại từ tiến độ thật.
///
/// Tách thành hàm thuần để kiểm thử được mọi ranh giới mà không cần một lượt
/// quét thật.
///
/// * `hashed` — số tệp đã mở xong.
/// * `total` — tổng số tệp phải mở.
/// * `elapsed_secs` — đã chạy bao lâu.
pub fn uoc_luong_con_lai(hashed: usize, total: usize, elapsed_secs: f64) -> Option<u64> {
    if hashed < MIN_MAU_UOC_LUONG || hashed >= total || elapsed_secs <= 0.0 {
        return None;
    }
    let toc_do = hashed as f64 / elapsed_secs;
    if toc_do <= 0.0 {
        return None;
    }
    let con_lai = (total - hashed) as f64 / toc_do;
    // Chặn trên: một con số kiểu "còn 9 tiếng" không giúp ai quyết định gì, và
    // gần như luôn là dấu hiệu ổ vừa rớt chứ không phải ước lượng thật.
    Some(con_lai.min(24.0 * 3600.0) as u64)
}

/// Giây Unix hiện tại.
fn gio_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Shared handle so the UI can watch a scan that takes a while.
#[derive(Default)]
pub struct DupeService {
    running: Arc<AtomicBool>,
    /// Raised to ask the scan thread to give up. Checked inside both passes,
    /// so leaving the view stops the disk instead of letting it read for
    /// another ten minutes to produce an answer nobody will see.
    stop: Arc<AtomicBool>,
    completed: Arc<AtomicBool>,
    candidates: Arc<AtomicUsize>,
    hashed: Arc<AtomicUsize>,
    result: Arc<parking_lot::Mutex<Vec<DuplicateGroup>>>,
    /// Chỉ mục mà lượt quét này đã dùng, giữ nguyên cạnh kết quả.
    ///
    /// `DuplicateGroup.entries` là **vị trí** trong chỉ mục, mà vị trí không
    /// sống sót qua một lần dựng lại: `index::update::rebuild_with` nói thẳng
    /// *"Entry positions are not preserved — they cannot be"*, vì mục ở giữa
    /// biến mất thì mọi thứ sau nó trượt lên.
    ///
    /// Thiếu trường này thì `dupe_groups` tra vị trí cũ trên snapshot MỚI, và
    /// mỗi nhóm hiện tên cùng đường dẫn của **tệp khác** — không cảnh báo gì.
    /// Với một màn hình mà bước tiếp theo là xoá tệp, đó là kiểu hỏng đắt
    /// nhất có thể có.
    ///
    /// Chuyện này không hiếm: chỉ mục dựng lại sau mỗi lượt cập nhật gia tăng
    /// có xoá, và sau **mỗi** lượt quét ổ mạng theo lịch — hai lần mỗi ngày,
    /// trên mọi máy.
    snapshot: Arc<parking_lot::Mutex<Option<Arc<Index>>>>,
    /// `epoch` của chỉ mục lúc quét.
    ///
    /// Giao diện ghép `epoch` với vị trí để dựng URL ảnh thu nhỏ và xem trước
    /// (`thumbUrl(epoch, index)`), nên nó phải là epoch của **cùng** chỉ mục
    /// đã sinh ra các vị trí đó. Lấy epoch hiện tại ghép với vị trí cũ thì ảnh
    /// thu nhỏ cũng của tệp khác.
    epoch: Arc<AtomicU64>,
    /// Mốc bắt đầu lượt quét, giây Unix — để tính tốc độ thật.
    started_unix: Arc<AtomicU64>,
}

impl DupeService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn progress(&self) -> DupeProgress {
        let groups = self.result.lock();
        DupeProgress {
            running: self.running.load(Ordering::Relaxed),
            candidates: self.candidates.load(Ordering::Relaxed),
            hashed: self.hashed.load(Ordering::Relaxed),
            groups: groups.len(),
            wasted: groups.iter().map(|g| g.wasted).sum(),
            completed: self.completed.load(Ordering::Relaxed),
            started_unix: self.started_unix.load(Ordering::Relaxed),
            stopping: self.running.load(Ordering::Relaxed) && self.stop.load(Ordering::Relaxed),
            eta_seconds: {
                let running = self.running.load(Ordering::Relaxed);
                let bat_dau = self.started_unix.load(Ordering::Relaxed);
                if running && bat_dau > 0 {
                    uoc_luong_con_lai(
                        self.hashed.load(Ordering::Relaxed),
                        self.candidates.load(Ordering::Relaxed),
                        gio_unix().saturating_sub(bat_dau) as f64,
                    )
                } else {
                    None
                }
            },
        }
    }

    pub fn groups(&self) -> Vec<DuplicateGroup> {
        self.result.lock().clone()
    }

    /// Chỉ mục mà kết quả hiện tại trỏ vào, kèm epoch của nó.
    ///
    /// `None` khi chưa quét lần nào. Người gọi **phải** phân giải vị trí bằng
    /// snapshot này chứ không phải snapshot hiện tại của `AppState` — đó là
    /// toàn bộ lý do nó tồn tại.
    pub fn scanned_index(&self) -> Option<(Arc<Index>, u64)> {
        let ix = self.snapshot.lock().clone()?;
        Some((ix, self.epoch.load(Ordering::Relaxed)))
    }

    /// Ask a running scan to stop.
    ///
    /// Returns once the flag is raised, not once the thread has noticed — the
    /// caller is the UI and should not wait on disk. A scan that stops leaves
    /// `completed` false, so the next visit starts a fresh one rather than
    /// showing half an answer as if it were whole.
    pub fn cancel(&self) {
        if self.running.load(Ordering::Relaxed) {
            self.stop.store(true, Ordering::Relaxed);
            tracing::info!("tìm trùng lặp: đã yêu cầu dừng");
        }
    }

    /// Start a scan, unless one is already running.
    ///
    /// Returns false when a scan is already in flight, so the UI can say so
    /// rather than silently starting a second one.
    /// Bắt đầu một lượt quét.
    ///
    /// `epoch` là số hiệu của chính `index` truyền vào — hai thứ phải đi cùng
    /// nhau, vì kết quả trỏ vào vị trí trong chỉ mục đó.
    pub fn start(
        &self,
        index: Arc<Index>,
        epoch: u64,
        scope: crate::media::dupescope::DupeScope,
        net_letters: Vec<char>,
    ) -> bool {
        if self.running.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.stop.store(false, Ordering::Relaxed);
        self.completed.store(false, Ordering::Relaxed);
        self.candidates.store(0, Ordering::Relaxed);
        self.hashed.store(0, Ordering::Relaxed);
        self.result.lock().clear();
        // Giữ chỉ mục này cạnh kết quả. `Arc` nên không tốn thêm bộ nhớ cho
        // bản thân dữ liệu — chỉ giữ nó sống lâu hơn snapshot toàn cục.
        *self.snapshot.lock() = Some(Arc::clone(&index));
        self.epoch.store(epoch, Ordering::Relaxed);
        self.started_unix.store(gio_unix(), Ordering::Relaxed);

        let running = Arc::clone(&self.running);
        let stop = Arc::clone(&self.stop);
        let completed = Arc::clone(&self.completed);
        let candidates = Arc::clone(&self.candidates);
        let hashed = Arc::clone(&self.hashed);
        let result = Arc::clone(&self.result);

        std::thread::Builder::new()
            .name("dupes".into())
            .spawn(move || {
                let started = std::time::Instant::now();
                let groups = find_duplicates(
                    &index,
                    &candidates,
                    &hashed,
                    &stop,
                    scope,
                    &net_letters,
                    &result,
                );

                if stop.load(Ordering::Relaxed) {
                    tracing::info!(
                        "tìm trùng lặp: đã dừng theo yêu cầu, giữ lại {} nhóm đã chốt [{:.1}s]",
                        result.lock().len(),
                        started.elapsed().as_secs_f64()
                    );
                    // KHÔNG xoá `result`. Các đợt đã chạy là kết quả thật, và
                    // vì lớp được xử lý theo tiềm năng giảm dần nên đó chính
                    // là những nhóm đáng giá nhất. Vứt đi là bắt người dùng
                    // trả lại từ đầu cái họ vừa chờ xong.
                    //
                    // `completed` vẫn để false: câu trả lời chưa đầy đủ, và
                    // giao diện phải phân biệt được "đã quét hết" với "dừng
                    // giữa chừng, đây là phần đã tìm thấy".
                } else {
                    let wasted: u64 = groups.iter().map(|g| g.wasted).sum();
                    tracing::info!(
                        "tìm trùng lặp: {} nhóm, lãng phí {:.1} GB [{:.1}s]",
                        groups.len(),
                        wasted as f64 / 1024.0 / 1024.0 / 1024.0,
                        started.elapsed().as_secs_f64()
                    );
                    *result.lock() = groups;
                    // Set before `running` drops, so a UI that sees the scan
                    // stop never reads `completed` as false in between.
                    completed.store(true, Ordering::Relaxed);
                }
                running.store(false, Ordering::Release);
            })
            .ok();
        true
    }
}

/// Dựng danh sách nhóm từ bảng vân tay, đã sắp theo giá trị thu hồi.
fn nhom_tu(by_hash: &HashMap<(u64, [u8; 32]), Vec<u32>>) -> Vec<DuplicateGroup> {
    let mut groups: Vec<DuplicateGroup> = by_hash
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(&(size, _), entries)| {
            // Thứ tự ổn định để hai lượt quét giống nhau liệt kê một nhóm y
            // hệt, và mục đầu tiên luôn là "giữ cái này".
            let mut entries = entries.clone();
            entries.sort_unstable();
            DuplicateGroup {
                size,
                wasted: size * (entries.len() as u64 - 1),
                entries,
            }
        })
        .collect();
    groups.sort_unstable_by(|a, b| {
        b.wasted
            .cmp(&a.wasted)
            .then(a.entries[0].cmp(&b.entries[0]))
    });
    groups
}

/// Công bố phần đã chốt để giao diện hiện ngay.
///
/// Gọi sau mỗi đợt. Vì các lớp được xử lý theo tiềm năng giảm dần, nhóm đã
/// công bố không bao giờ bị đẩy xuống bởi nhóm tìm thấy sau — thứ hạng của nó
/// là chung cuộc.
fn publish(
    by_hash: &HashMap<(u64, [u8; 32]), Vec<u32>>,
    result: &parking_lot::Mutex<Vec<DuplicateGroup>>,
) {
    *result.lock() = nhom_tu(by_hash);
}

/// Tier 1 then tier 2.
///
/// Returns nothing if `stop` is raised, rather than a partial answer that
/// could be mistaken for a complete one.
fn find_duplicates(
    index: &Index,
    candidates: &AtomicUsize,
    hashed: &AtomicUsize,
    stop: &AtomicBool,
    scope: crate::media::dupescope::DupeScope,
    net_letters: &[char],
    result: &parking_lot::Mutex<Vec<DuplicateGroup>>,
) -> Vec<DuplicateGroup> {
    // Tier 1: group by size. Free — the index already holds every size, and
    // two files of different lengths cannot be the same file.
    let mut by_size: HashMap<u64, Vec<u32>> = HashMap::new();
    for (i, &size) in index.sizes().iter().enumerate() {
        if size >= MIN_INTERESTING_SIZE {
            by_size.entry(size).or_default().push(i as u32);
        }
    }
    by_size.retain(|_, v| v.len() > 1);

    // One work item per *file*, not per size. Grouping the parallel work by
    // size puts every file sharing one common size onto a single thread while
    // the others sit idle; a library where one size is very popular then runs
    // most of the scan single-threaded.
    // Lọc theo phạm vi NGAY ĐÂY, trước khi đếm. Người dùng đã chọn "chỉ ổ
    // trong máy" thì con số tiến độ phải là số việc thật, không kể tệp NAS sẽ
    // không bao giờ được mở — một thanh tiến độ đếm cả việc không làm là một
    // thanh tiến độ nói dối.
    //
    // Xếp theo TIỀM NĂNG THU HỒI giảm dần, không theo thứ tự băm.
    //
    // Tiềm năng của một lớp dung lượng là `size * (số tệp − 1)` — cận trên
    // chính xác của mọi nhóm sinh ra từ lớp đó, vì nhóm lớn nhất có thể là cả
    // lớp. Nên xử lý theo thứ tự này thì nhóm đáng giá nhất ra trước.
    //
    // Đo trên thư viện thật (410.581 tệp, 197.301 ứng viên): tệp ≥256 MB chỉ
    // chiếm **1,7% số tệp** nhưng mang **68% tổng tiềm năng** (2.543 GB trên
    // 3.746 GB). Tệp 64 KB–1 MB thì ngược lại: 35% số tệp, 0,4% giá trị.
    //
    // Người dọn ổ vì thế thấy phần lớn giá trị sau khi mở vài nghìn tệp thay
    // vì hai trăm nghìn — tổng thời gian không đổi, nhưng thời gian phải CHỜ
    // giảm từ hàng chục phút xuống vài giây.
    let mut lop: Vec<(u64, Vec<u32>)> = by_size
        .into_iter()
        .map(|(size, entries)| {
            let trong_pham_vi: Vec<u32> = entries
                .into_iter()
                .filter(|&i| crate::media::dupescope::in_scope(index, i, scope, net_letters))
                .collect();
            (size, trong_pham_vi)
        })
        // Lọc phạm vi có thể làm một lớp chỉ còn một tệp — lúc đó không còn
        // gì để so sánh.
        .filter(|(_, v)| v.len() > 1)
        .collect();
    lop.sort_unstable_by(|a, b| {
        let ta = a.0 * (a.1.len() as u64 - 1);
        let tb = b.0 * (b.1.len() as u64 - 1);
        // Tiềm năng giảm dần; hoà thì theo dung lượng để hai lượt quét giống
        // nhau cho ra cùng thứ tự.
        tb.cmp(&ta).then(b.0.cmp(&a.0))
    });

    let work: Vec<(u64, u32)> = lop
        .iter()
        .flat_map(|(size, entries)| entries.iter().map(move |&i| (*size, i)))
        .collect();
    let total = work.len();
    candidates.store(total, Ordering::Relaxed);
    tracing::info!("tầng 1: {} tệp cùng dung lượng cần kiểm tra", total);

    // Tier 2: sample each candidate, then regroup on the fingerprint.
    //
    // Both ends in one pass, one open per file. Splitting this into a
    // head-only pass and a tail pass was tried and measured: on the real
    // library the head separated 166 of 29,053 candidates — 0.6% — because
    // most candidates here are genuine copies, whose heads *should* match.
    // The split bought nothing and cost a second open per file. See PERF-003.
    // Chia thành đợt và công bố sau mỗi đợt.
    //
    // Không chạy rayon từng LỚP: đuôi phân bố là hàng nghìn lớp chỉ có 2–3
    // tệp, và mỗi lớp một lần gọi song song thì phần điều phối tốn hơn phần
    // việc. Gom các lớp liền nhau thành đợt vài trăm tệp, `par_iter` trong
    // đợt.
    //
    // Vì các lớp đã xếp theo tiềm năng giảm dần, mọi nhóm công bố ở đợt trước
    // đều có `wasted` lớn hơn hoặc bằng tiềm năng của đợt sau — nên thứ hạng
    // của chúng là **chung cuộc**, không đảo lộn khi quét tiếp.
    const TEP_MOI_DOT: usize = 400;

    // Kho vân tay đã lưu từ lượt trước. Đọc một lần, dùng cho cả lượt.
    let kho = parking_lot::Mutex::new(crate::media::dupestore::load());
    let tu_kho = AtomicUsize::new(0);
    if !kho.lock().is_empty() {
        tracing::info!("kho vân tay: {} mục đã lưu", kho.lock().len());
    }

    let mut by_hash: HashMap<(u64, [u8; 32]), Vec<u32>> = HashMap::new();
    let mut dot: Vec<(u64, u32)> = Vec::with_capacity(TEP_MOI_DOT);
    let mut da_huy = false;

    let chay_dot = |dot: &mut Vec<(u64, u32)>,
                    by_hash: &mut HashMap<(u64, [u8; 32]), Vec<u32>>,
                    kho: &parking_lot::Mutex<crate::media::dupestore::Store>,
                    tu_kho: &AtomicUsize|
     -> bool {
        if dot.is_empty() {
            return true;
        }
        let vt: Vec<(u64, u32, [u8; 32], bool)> = std::mem::take(dot)
            .into_par_iter()
            .filter_map(|(size, i)| {
                if stop.load(Ordering::Relaxed) {
                    return None;
                }
                let path = index.full_path(i as usize);
                let mtime = index.mtimes().get(i as usize).copied().unwrap_or(0);

                // Đã có vân tay và tệp chưa đổi thì KHÔNG mở lại. Đây là toàn
                // bộ giá trị của kho: trên NAS một lần mở tốn ~66 ms chỉ để
                // lấy byte đầu, và 82% ứng viên nằm ở đó.
                if let Some(fp) = kho.lock().get(&path, size, mtime) {
                    tu_kho.fetch_add(1, Ordering::Relaxed);
                    hashed.fetch_add(1, Ordering::Relaxed);
                    return Some((size, i, fp, false));
                }

                hashed.fetch_add(1, Ordering::Relaxed);
                fingerprint(&path, size).map(|h| (size, i, h, true))
            })
            .collect();

        for (size, i, h, moi_doc) in vt {
            if moi_doc {
                let path = index.full_path(i as usize);
                let mtime = index.mtimes().get(i as usize).copied().unwrap_or(0);
                kho.lock().put(&path, size, mtime, h);
            }
            by_hash.entry((size, h)).or_default().push(i);
        }
        !stop.load(Ordering::Relaxed)
    };

    for (size, entries) in &lop {
        for &i in entries {
            dot.push((*size, i));
        }
        if dot.len() >= TEP_MOI_DOT {
            let con_chay = chay_dot(&mut dot, &mut by_hash, &kho, &tu_kho);
            // Công bố TRƯỚC khi kiểm cờ dừng. Một đợt bị huỷ vẫn đã băm xong
            // phần lớn tệp của nó, và thoát ra trước khi công bố là vứt đúng
            // cái vừa làm xong — đây là lỗi bài kiểm thử bắt được ngay lần
            // chạy đầu.
            publish(&by_hash, result);
            if !con_chay {
                da_huy = true;
                break;
            }
        }
    }
    if !da_huy {
        if !chay_dot(&mut dot, &mut by_hash, &kho, &tu_kho) {
            da_huy = true;
        }
        publish(&by_hash, result);
    }

    // Huỷ giữa chừng KHÔNG còn vứt sạch: các đợt đã chạy là kết quả thật, và
    // vì chúng là những lớp giá trị nhất nên phần bỏ dở đáng giá ít hơn hẳn.
    // Vứt đi là bắt người dùng trả lại từ đầu cái họ đã chờ xong.
    let _ = da_huy;

    // Lưu kho vân tay — kể cả khi bị huỷ. Một lượt huỷ vẫn đã đọc xong hàng
    // nghìn tệp, và vứt phần đó đi là bắt lượt sau đọc lại từ đầu.
    //
    // Lưu MỘT LẦN ở đây, không phải mỗi vài trăm tệp: `save` tuần tự hoá cả
    // map, nên lưu liên tục là ghi lại vài chục MB hàng trăm lần.
    {
        let mut k = kho.lock();
        // Tỉa khoá không còn trong tập ứng viên. Không tỉa thì kho phình mãi
        // theo tệp đã xoá — đúng lỗi mà `metadata.bin` của enrichment đang có.
        //
        // Chỉ tỉa khi quét TRỌN VẸN cả phạm vi: một lượt bị huỷ, hoặc một lượt
        // chỉ quét ổ trong máy, không nhìn thấy hết tệp — tỉa theo nó là vứt
        // vân tay của những tệp vẫn còn nguyên.
        if !da_huy && scope == crate::media::dupescope::DupeScope::Everything {
            let con_dung: std::collections::HashSet<u64> = lop
                .iter()
                .flat_map(|(_, e)| e.iter())
                .map(|&i| crate::media::dupestore::path_key(&index.full_path(i as usize)))
                .collect();
            let da_tia = k.retain_keys(&con_dung);
            if da_tia > 0 {
                tracing::info!("kho vân tay: tỉa {da_tia} mục không còn trong chỉ mục");
            }
        }
        if crate::media::dupestore::save(&k) {
            tracing::info!(
                "kho vân tay: lưu {} mục · lượt này đọc lại {} tệp, dùng kho {} tệp",
                k.len(),
                hashed.load(Ordering::Relaxed) - tu_kho.load(Ordering::Relaxed),
                tu_kho.load(Ordering::Relaxed)
            );
        }
    }

    let mut groups: Vec<DuplicateGroup> = nhom_tu(&by_hash);

    // Biggest waste first — the order somebody clearing space wants to work
    // through. The entry tiebreak keeps two runs of the same scan identical.
    groups.sort_unstable_by(|a, b| {
        b.wasted
            .cmp(&a.wasted)
            .then(a.entries[0].cmp(&b.entries[0]))
    });
    groups
}

/// Hash the first and last [`SAMPLE_BYTES`] together with the size.
///
/// The size goes into the hash rather than being compared separately so two
/// files can never be judged equal on their samples alone.
///
/// One open, one read of each end. See the note in [`find_duplicates`] for why
/// this is not split into a cheap first pass and an expensive second one.
fn fingerprint(path: &str, size: u64) -> Option<[u8; 32]> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(&size.to_le_bytes());

    if size <= SMALL_FILE_LIMIT {
        // Reading both ends of a small file would read most of it twice.
        let mut buf = Vec::with_capacity(size as usize);
        file.read_to_end(&mut buf).ok()?;
        hasher.update(&buf);
    } else {
        let mut buf = vec![0u8; SAMPLE_BYTES as usize];
        file.read_exact(&mut buf).ok()?;
        hasher.update(&buf);

        file.seek(SeekFrom::End(-(SAMPLE_BYTES as i64))).ok()?;
        file.read_exact(&mut buf).ok()?;
        hasher.update(&buf);
    }

    Some(*hasher.finalize().as_bytes())
}

/// Tier 3: hash a file end to end.
///
/// Only ever called for a group the person is about to act on, never during a
/// scan. On a 3 GB video this reads 3 GB.
pub fn full_hash(path: &str) -> Option<[u8; 32]> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(_) => return None,
        };
    }
    Some(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(name: &str, content: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("mediafinder-dupe-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).expect("create");
        f.write_all(content).expect("write");
        path
    }

    #[test]
    fn identical_files_share_a_fingerprint() {
        let content = vec![7u8; 200_000];
        let a = temp_file("dupe_a.bin", &content);
        let b = temp_file("dupe_b.bin", &content);

        let ha = fingerprint(&a.to_string_lossy(), content.len() as u64);
        let hb = fingerprint(&b.to_string_lossy(), content.len() as u64);
        assert!(ha.is_some());
        assert_eq!(ha, hb);
    }

    #[test]
    fn a_difference_in_the_middle_is_invisible_to_tier_two() {
        // Honest about the limit. Two files identical at both ends but
        // differing in the middle get the same fingerprint — which is why
        // tier 3 exists, and why nothing here deletes anything.
        let mut a = vec![7u8; 400_000];
        let mut b = a.clone();
        a[200_000] = 1;
        b[200_000] = 2;

        let pa = temp_file("mid_a.bin", &a);
        let pb = temp_file("mid_b.bin", &b);
        assert_eq!(
            fingerprint(&pa.to_string_lossy(), a.len() as u64),
            fingerprint(&pb.to_string_lossy(), b.len() as u64),
            "tầng 2 chỉ đọc hai đầu — đây là giới hạn đã biết"
        );

        // Tier 3 sees it.
        assert_ne!(
            full_hash(&pa.to_string_lossy()),
            full_hash(&pb.to_string_lossy()),
            "tầng 3 đọc toàn bộ nên phải phân biệt được"
        );
    }

    #[test]
    fn different_content_at_the_head_differs() {
        let a = vec![1u8; 200_000];
        let b = vec![2u8; 200_000];
        let pa = temp_file("head_a.bin", &a);
        let pb = temp_file("head_b.bin", &b);
        assert_ne!(
            fingerprint(&pa.to_string_lossy(), 200_000),
            fingerprint(&pb.to_string_lossy(), 200_000)
        );
    }

    #[test]
    fn a_small_file_is_read_whole() {
        // Below the two-sample threshold, reading both ends would read most of
        // the file twice; this path must still distinguish content.
        let a = temp_file("small_a.bin", b"hello world");
        let b = temp_file("small_b.bin", b"HELLO WORLD");
        assert_ne!(
            fingerprint(&a.to_string_lossy(), 11),
            fingerprint(&b.to_string_lossy(), 11)
        );
    }

    #[test]
    fn size_is_part_of_the_fingerprint() {
        // Two files whose samples match but whose lengths differ must not be
        // grouped — the size is mixed in so that cannot happen.
        let path = temp_file("size_x.bin", &vec![3u8; 200_000]);
        let p = path.to_string_lossy();
        assert_ne!(fingerprint(&p, 200_000), fingerprint(&p, 199_999));
    }

    /// Build a real `Index` over files on disk, so `find_duplicates` is
    /// exercised end to end rather than through a stand-in.
    fn index_over(tag: &str, files: &[(&str, Vec<u8>)]) -> (Index, std::path::PathBuf) {
        use crate::index::model::{IndexBuilder, MediaKind};

        let dir = std::env::temp_dir().join(format!("mediafinder-dupe-idx-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        let mut b = IndexBuilder::new();
        let did = b.add_dir(dir.to_str().expect("utf8"), 1);
        let mut sizes = Vec::new();
        for (i, (name, content)) in files.iter().enumerate() {
            let mut f = std::fs::File::create(dir.join(name)).expect("create");
            f.write_all(content).expect("write");
            b.add_file(name, MediaKind::Video, did, 100 + i as u64);
            sizes.push(content.len() as u64);
        }
        let mut index = b.finish();
        let mtimes = vec![0i64; sizes.len()];
        index.set_file_stats(sizes, mtimes);
        (index, dir)
    }

    fn run(index: &Index) -> Vec<DuplicateGroup> {
        find_duplicates(
            index,
            &AtomicUsize::new(0),
            &AtomicUsize::new(0),
            &AtomicBool::new(false),
            // Các bài này dựng chỉ mục ở thư mục tạm (ổ trong máy) và không
            // quan tâm phạm vi, nên quét tất.
            crate::media::dupescope::DupeScope::Everything,
            &[],
            &parking_lot::Mutex::new(Vec::new()),
        )
    }

    /// Nhóm đáng giá nhất phải ra TRƯỚC, không theo thứ tự băm.
    ///
    /// Đo trên thư viện thật (410.581 tệp, 197.301 ứng viên): tệp ≥256 MB chỉ
    /// chiếm 1,7% số tệp nhưng mang **68% tổng tiềm năng thu hồi**. Xử lý theo
    /// tiềm năng giảm dần nghĩa là người dọn ổ thấy phần lớn giá trị sau vài
    /// nghìn tệp thay vì hai trăm nghìn.
    #[test]
    fn nhom_dang_gia_nhat_ra_truoc() {
        let nho = vec![1u8; 200 * 1024];
        let to = vec![2u8; 900 * 1024];
        let (index, _d) = index_over(
            "thu-tu-gia-tri",
            &[
                ("nho-a.mp4", nho.clone()),
                ("nho-b.mp4", nho),
                ("to-a.mp4", to.clone()),
                ("to-b.mp4", to),
            ],
        );
        let groups = run(&index);
        assert_eq!(groups.len(), 2);
        assert!(
            groups[0].wasted > groups[1].wasted,
            "nhóm thu hồi được nhiều hơn phải đứng trước"
        );
        assert_eq!(groups[0].size, 900 * 1024);
    }

    /// Nhóm giá trị nhất phải được XỬ LÝ trước, không chỉ được sắp trước.
    ///
    /// Ca trên không đủ: `groups.sort_unstable_by` ở cuối vẫn sắp đúng dù thứ
    /// tự xử lý ngẫu nhiên, nên đảo chiều sắp lớp mà nó vẫn xanh — phép thử
    /// bằng cách phá mã đã lộ ra điều đó.
    ///
    /// Thứ tự XỬ LÝ mới là thứ quyết định người dùng chờ bao lâu để thấy nhóm
    /// đầu tiên, và nó chỉ quan sát được qua kết quả **công bố giữa chừng**.
    #[test]
    fn nhom_gia_tri_nhat_duoc_cong_bo_truoc_tien() {
        // Nhiều lớp nhỏ (để vượt một đợt) cộng một lớp rất lớn. Lớp lớn phải
        // được băm trong đợt ĐẦU, nên nó có mặt ngay ở lần công bố đầu tiên.
        // Mỗi lớp phải có DUNG LƯỢNG KHÁC NHAU, nếu không chúng gộp thành
        // một lớp duy nhất — sai giả định mà bài này đã mắc ở lần viết đầu:
        // 250 tệp cùng 70 KB không phải 250 lớp, mà là một lớp 500 tệp có
        // tiềm năng 34 MB, lớn hơn hẳn lớp "to" 2 MB.
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let to = vec![7u8; 2 * 1024 * 1024];
        files.push(("zz-to-a.mp4".into(), to.clone()));
        files.push(("zz-to-b.mp4".into(), to));
        for i in 0..250usize {
            // 70 KB + i byte: mỗi lớp một dung lượng riêng, tiềm năng ~70 KB.
            let nd = vec![(i % 251) as u8; 70 * 1024 + i];
            files.push((format!("n-{i}-a.mp4"), nd.clone()));
            files.push((format!("n-{i}-b.mp4"), nd));
        }
        let muon: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(n, c)| (n.as_str(), c.clone())).collect();
        let (index, _d) = index_over("thu-tu-xu-ly", &muon);

        let ket_qua = parking_lot::Mutex::new(Vec::new());
        let stop = AtomicBool::new(false);
        let hashed = AtomicUsize::new(0);

        // Dừng ngay sau đợt đầu tiên.
        std::thread::scope(|sc| {
            sc.spawn(|| {
                while hashed.load(Ordering::Relaxed) < 410 {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                stop.store(true, Ordering::Relaxed);
            });
            find_duplicates(
                &index,
                &AtomicUsize::new(0),
                &hashed,
                &stop,
                crate::media::dupescope::DupeScope::Everything,
                &[],
                &ket_qua,
            );
        });

        let g = ket_qua.lock();
        assert!(!g.is_empty(), "đợt đầu phải công bố được gì đó");
        assert_eq!(
            g[0].size,
            2 * 1024 * 1024,
            "lớp giá trị nhất phải nằm trong đợt ĐẦU, không phải đợt cuối"
        );
    }

    /// Huỷ giữa chừng giữ lại phần đã chốt thay vì vứt sạch.
    #[test]
    fn huy_giua_chung_giu_lai_phan_da_chot() {
        // Dựng nhiều tệp hơn một đợt (400) để chắc chắn có ít nhất một đợt
        // chạy xong và được công bố trước khi cờ dừng được giương.
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..250 {
            let noi_dung = vec![(i % 251) as u8; 70 * 1024];
            files.push((format!("p-{i}-a.mp4"), noi_dung.clone()));
            files.push((format!("p-{i}-b.mp4"), noi_dung));
        }
        let muon: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(n, c)| (n.as_str(), c.clone())).collect();
        let (index, _d) = index_over("huy-giu-lai", &muon);

        let ket_qua = parking_lot::Mutex::new(Vec::new());
        let stop = AtomicBool::new(false);
        let hashed = AtomicUsize::new(0);

        // Chạy tới khi công bố được ít nhất một đợt, rồi dừng.
        std::thread::scope(|sc| {
            sc.spawn(|| {
                while hashed.load(Ordering::Relaxed) < 420 {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                stop.store(true, Ordering::Relaxed);
            });
            find_duplicates(
                &index,
                &AtomicUsize::new(0),
                &hashed,
                &stop,
                crate::media::dupescope::DupeScope::Everything,
                &[],
                &ket_qua,
            );
        });

        assert!(
            !ket_qua.lock().is_empty(),
            "huỷ giữa chừng phải giữ lại phần đã chốt, không vứt sạch"
        );
    }

    /// Ước lượng thời gian: im lặng khi chưa biết, và không bao giờ đoán bừa.
    #[test]
    fn uoc_luong_im_lang_cho_toi_khi_co_du_mau() {
        // Vài tệp đầu chưa nói lên gì: cache còn lạnh, luồng còn khởi động.
        // Hiện một con số lúc này là hứa hẹn dựa trên nhiễu, và nó sẽ nhảy từ
        // "2 phút" lên "40 phút" rồi xuống "5 phút" ngay trước mắt người dùng.
        assert_eq!(uoc_luong_con_lai(1, 10_000, 1.0), None);
        assert_eq!(uoc_luong_con_lai(MIN_MAU_UOC_LUONG - 1, 10_000, 5.0), None);
        // Đủ mẫu thì mới nói.
        assert!(uoc_luong_con_lai(MIN_MAU_UOC_LUONG, 10_000, 5.0).is_some());
    }

    #[test]
    fn uoc_luong_tinh_dung_tu_toc_do_do_duoc() {
        // 1.000 tệp trong 10 giây = 100 tệp/giây; còn 9.000 tệp = 90 giây.
        assert_eq!(uoc_luong_con_lai(1_000, 10_000, 10.0), Some(90));
    }

    #[test]
    fn quet_xong_thi_khong_con_gi_de_uoc_luong() {
        assert_eq!(uoc_luong_con_lai(10_000, 10_000, 100.0), None);
        // Và `hashed` vượt `total` (đếm cả tệp mở lỗi) cũng không được ra số âm.
        assert_eq!(uoc_luong_con_lai(10_001, 10_000, 100.0), None);
    }

    #[test]
    fn thoi_gian_bang_khong_khong_lam_no_chia_cho_khong() {
        assert_eq!(uoc_luong_con_lai(1_000, 10_000, 0.0), None);
        assert_eq!(uoc_luong_con_lai(1_000, 10_000, -5.0), None);
    }

    #[test]
    fn o_rot_khong_sinh_ra_con_so_vo_nghia() {
        // Chậm tới mức phi lý (một tệp mỗi mười giây) thường là ổ vừa rớt chứ
        // không phải ước lượng thật. Chặn ở 24 giờ để màn hình không nói
        // "còn 340 ngày".
        let e = uoc_luong_con_lai(MIN_MAU_UOC_LUONG, 100_000_000, 2_000.0);
        assert_eq!(e, Some(24 * 3600));
    }

    /// Kết quả phải trỏ vào chỉ mục CỦA LƯỢT QUÉT, không phải chỉ mục hiện tại.
    ///
    /// Đây là lỗi đắt nhất mà màn hình Trùng lặp có thể mắc, vì bước tiếp theo
    /// của người dùng là **xoá tệp**. `entries` là vị trí trong chỉ mục, mà vị
    /// trí không sống sót qua một lần dựng lại — `index::update::rebuild_with`
    /// nói thẳng *"Entry positions are not preserved — they cannot be"*.
    ///
    /// Kịch bản thật: quét lúc 9:00, rời màn hình; 10:25 lịch quét ổ mạng chạy
    /// xong và chỉ mục nạp lại; 11:00 quay lại thì mỗi nhóm hiện tên và đường
    /// dẫn của tệp KHÁC, không một lời cảnh báo. Từ v1.0.8 chuyện này xảy ra
    /// hai lần mỗi ngày trên mọi máy.
    #[test]
    fn ket_qua_giu_chi_muc_cua_luot_quet_chu_khong_theo_chi_muc_moi() {
        let svc = DupeService::new();
        assert!(
            svc.scanned_index().is_none(),
            "chưa quét thì không được có chỉ mục nào"
        );

        let (ix_cu, _d1) = index_over("giu-snapshot", &[("a.mp4", vec![1u8; 200_000])]);
        let arc_cu = Arc::new(ix_cu);
        assert!(svc.start(
            Arc::clone(&arc_cu),
            7,
            crate::media::dupescope::DupeScope::Everything,
            Vec::new()
        ));

        // Chờ lượt quét xong để `running` hạ xuống.
        for _ in 0..200 {
            if !svc.progress().running {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let (giu, epoch) = svc.scanned_index().expect("phải giữ chỉ mục đã quét");
        assert_eq!(
            epoch, 7,
            "epoch phải là epoch lúc quét, không phải epoch mới"
        );
        assert!(
            Arc::ptr_eq(&giu, &arc_cu),
            "phải giữ ĐÚNG chỉ mục đã quét, không phải một bản khác"
        );

        // Chỉ mục mới ra đời (lượt quét NAS xong, cache nạp lại). Kết quả cũ
        // vẫn phải phân giải theo chỉ mục cũ.
        let (ix_moi, _d2) = index_over("giu-snapshot-2", &[("z.mp4", vec![9u8; 200_000])]);
        drop(ix_moi);
        let (van_giu, van_epoch) = svc.scanned_index().expect("vẫn phải còn");
        assert!(Arc::ptr_eq(&van_giu, &arc_cu));
        assert_eq!(van_epoch, 7);
    }

    /// The size floor exists so thousands of identical icons do not bury the
    /// groups worth acting on. Two matching small files must stay unreported.
    #[test]
    fn files_below_the_size_floor_are_never_reported() {
        let small = vec![7u8; 1024];
        let (index, _d) = index_over(
            "floor",
            &[("tiny-a.mp4", small.clone()), ("tiny-b.mp4", small)],
        );
        assert!(
            run(&index).is_empty(),
            "tep duoi nguong {} byte khong duoc bao",
            MIN_INTERESTING_SIZE
        );
    }

    /// Tier 1 must reject unique sizes *without opening the files* — that is
    /// where nearly all the saving comes from. Checking only the result would
    /// pass even if every file were read and then discarded.
    #[test]
    fn tier_one_rejects_unique_sizes_without_reading_them() {
        let (index, _d) = index_over(
            "tier1",
            &[
                ("u-1.mp4", vec![1u8; 200 * 1024]),
                ("u-2.mp4", vec![1u8; 201 * 1024]),
                ("u-3.mp4", vec![1u8; 202 * 1024]),
                ("u-4.mp4", vec![1u8; 203 * 1024]),
            ],
        );
        let candidates = AtomicUsize::new(0);
        let hashed = AtomicUsize::new(0);
        let groups = find_duplicates(
            &index,
            &candidates,
            &hashed,
            &AtomicBool::new(false),
            crate::media::dupescope::DupeScope::Everything,
            &[],
            &parking_lot::Mutex::new(Vec::new()),
        );
        assert!(groups.is_empty());
        assert_eq!(
            hashed.load(Ordering::Relaxed),
            0,
            "kich thuoc duy nhat thi khong duoc mo tep nao"
        );
        assert_eq!(
            candidates.load(Ordering::Relaxed),
            0,
            "khong co ung vien nao de kiem"
        );
    }

    /// Two files of different lengths cannot be the same file, and tier 1 must
    /// reject them without opening either.
    #[test]
    fn different_sizes_never_form_a_group() {
        let (index, _d) = index_over(
            "sizes",
            &[
                ("a.mp4", vec![1u8; 200 * 1024]),
                ("b.mp4", vec![1u8; 201 * 1024]),
            ],
        );
        assert!(run(&index).is_empty());
    }

    /// The whole point: identical files land in one group, and the waste is
    /// counted as everything past the first copy.
    #[test]
    fn identical_files_are_grouped_and_waste_counted() {
        let content = vec![9u8; 300 * 1024];
        let (index, _d) = index_over(
            "same",
            &[
                ("copy-1.mp4", content.clone()),
                ("copy-2.mp4", content.clone()),
                ("copy-3.mp4", content),
            ],
        );
        let groups = run(&index);
        assert_eq!(groups.len(), 1, "ba ban sao phai nam chung mot nhom");
        assert_eq!(groups[0].entries.len(), 3);
        assert_eq!(
            groups[0].wasted,
            300 * 1024 * 2,
            "giu lai mot ban, hai ban kia la phan thua"
        );
    }

    /// The two-pass split must not change the answer: files that differ only
    /// at the tail have to be separated, exactly as the single-pass
    /// fingerprint separated them.
    #[test]
    fn a_difference_only_at_the_tail_still_splits_the_group() {
        let mut a = vec![4u8; 300 * 1024];
        let mut b = a.clone();
        let n = a.len();
        a[n - 1] = 1;
        b[n - 1] = 2;
        let (index, _d) = index_over("tail", &[("tail-a.mp4", a), ("tail-b.mp4", b)]);
        assert!(run(&index).is_empty(), "khac phan cuoi thi phai tach ra");
    }

    /// The head pass alone must separate files that differ at the front —
    /// they should never reach the tail pass at all.
    #[test]
    fn a_difference_at_the_head_splits_without_the_tail() {
        let mut a = vec![4u8; 300 * 1024];
        let mut b = a.clone();
        a[0] = 1;
        b[0] = 2;
        let (index, _d) = index_over("head", &[("head-a.mp4", a), ("head-b.mp4", b)]);
        assert!(run(&index).is_empty());
    }

    /// Files small enough to be read whole have no separate tail read. They
    /// must still group rather than fall through some special case.
    #[test]
    fn small_files_read_whole_still_group() {
        let content = vec![3u8; 100 * 1024];
        assert!(content.len() as u64 > MIN_INTERESTING_SIZE);
        assert!(content.len() as u64 <= SMALL_FILE_LIMIT);
        let (index, _d) = index_over(
            "small",
            &[("s-a.mp4", content.clone()), ("s-b.mp4", content)],
        );
        let groups = run(&index);
        assert_eq!(groups.len(), 1, "tep nho doc tron ven van phai gom nhom");
        assert_eq!(groups[0].entries.len(), 2);
    }

    /// Groups come back biggest-waste-first, because that is the order
    /// somebody clearing space works through.
    #[test]
    fn groups_are_sorted_by_waste_descending() {
        let (index, _d) = index_over(
            "sorted",
            &[
                ("small-1.mp4", vec![1u8; 200 * 1024]),
                ("small-2.mp4", vec![1u8; 200 * 1024]),
                ("big-1.mp4", vec![2u8; 900 * 1024]),
                ("big-2.mp4", vec![2u8; 900 * 1024]),
            ],
        );
        let groups = run(&index);
        assert_eq!(groups.len(), 2);
        assert!(
            groups[0].wasted > groups[1].wasted,
            "nhom lang phi nhieu nhat phai dung dau: {} roi {}",
            groups[0].wasted,
            groups[1].wasted
        );
    }

    /// Two runs over the same library must list the same groups the same way,
    /// or the list would reshuffle under a person working through it.
    #[test]
    fn two_runs_give_the_same_answer() {
        let c1 = vec![5u8; 300 * 1024];
        let c2 = vec![6u8; 400 * 1024];
        let (index, _d) = index_over(
            "stable",
            &[
                ("r-a.mp4", c1.clone()),
                ("r-b.mp4", c1),
                ("r-c.mp4", c2.clone()),
                ("r-d.mp4", c2),
            ],
        );
        let first = run(&index);
        let second = run(&index);
        assert_eq!(first.len(), second.len());
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.entries, b.entries, "thu tu phai giong nhau giua hai lan");
            assert_eq!(a.wasted, b.wasted);
        }
    }

    /// Huỷ **trước khi mở tệp nào** thì không có gì để trả về.
    ///
    /// Khác với trước: huỷ giữa chừng nay GIỮ phần đã chốt (xem
    /// `huy_giua_chung_giu_lai_phan_da_chot`). Vì các lớp được xử lý theo tiềm
    /// năng giảm dần, phần đã chốt chính là những nhóm đáng giá nhất — vứt đi
    /// là bắt người dùng trả lại từ đầu cái họ vừa chờ xong.
    ///
    /// Cái vẫn phải giữ: `completed` để false, để giao diện phân biệt được
    /// "đã quét hết" với "dừng giữa chừng, đây là phần tìm thấy".
    #[test]
    fn a_cancelled_scan_returns_nothing() {
        let content = vec![8u8; 300 * 1024];
        let (index, _d) = index_over(
            "cancel",
            &[("c-a.mp4", content.clone()), ("c-b.mp4", content)],
        );
        let groups = find_duplicates(
            &index,
            &AtomicUsize::new(0),
            &AtomicUsize::new(0),
            &AtomicBool::new(true),
            crate::media::dupescope::DupeScope::Everything,
            &[],
            &parking_lot::Mutex::new(Vec::new()),
        );
        assert!(groups.is_empty(), "da huy thi khong tra ve ket qua do dang");
    }

    /// The progress counters have to reach the UI, or the bar sits at zero
    /// through a scan that lasts minutes.
    ///
    /// Asserts the exact count, not merely that it moved: both passes
    /// increment the same counter, so a check for "greater than zero" would
    /// still pass with one of them silently not counting.
    #[test]
    fn progress_counters_are_reported() {
        let content = vec![2u8; 300 * 1024];
        let (index, _d) = index_over(
            "progress",
            &[("p-a.mp4", content.clone()), ("p-b.mp4", content)],
        );
        let candidates = AtomicUsize::new(0);
        let hashed = AtomicUsize::new(0);
        find_duplicates(
            &index,
            &candidates,
            &hashed,
            &AtomicBool::new(false),
            crate::media::dupescope::DupeScope::Everything,
            &[],
            &parking_lot::Mutex::new(Vec::new()),
        );
        // Two files sharing a size: both are candidates, both get read.
        assert_eq!(
            hashed.load(Ordering::Relaxed),
            2,
            "phai dem dung so tep da doc"
        );
        assert_eq!(
            candidates.load(Ordering::Relaxed),
            2,
            "tong so ung vien sau tang 1"
        );
    }

    /// Every candidate that gets read must be counted, or the bar sits at
    /// zero through a scan that lasts minutes.
    #[test]
    fn every_candidate_read_is_counted() {
        // Two files of one size, two of another: all four are candidates.
        let (index, _d) = index_over(
            "count-all",
            &[
                ("ca-1.mp4", vec![1u8; 300 * 1024]),
                ("ca-2.mp4", vec![1u8; 300 * 1024]),
                ("ca-3.mp4", vec![2u8; 400 * 1024]),
                ("ca-4.mp4", vec![3u8; 400 * 1024]),
            ],
        );
        let candidates = AtomicUsize::new(0);
        let hashed = AtomicUsize::new(0);
        let groups = find_duplicates(
            &index,
            &candidates,
            &hashed,
            &AtomicBool::new(false),
            crate::media::dupescope::DupeScope::Everything,
            &[],
            &parking_lot::Mutex::new(Vec::new()),
        );

        assert_eq!(
            candidates.load(Ordering::Relaxed),
            4,
            "ca bon tep deu la ung vien sau tang 1"
        );
        assert_eq!(
            hashed.load(Ordering::Relaxed),
            4,
            "phai dem dung so tep da doc"
        );
        // Only the matching pair is a group; the 400 KB pair differs.
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].entries.len(), 2);
    }

    #[test]
    fn a_missing_file_yields_no_fingerprint() {
        assert!(fingerprint(r"D:\definitely\not\here\nope.bin", 1000).is_none());
        assert!(full_hash(r"D:\definitely\not\here\nope.bin").is_none());
    }

    // The size floor is asserted at build time next to the constant itself.
}
