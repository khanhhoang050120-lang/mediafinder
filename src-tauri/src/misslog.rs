//! Ghi lại các truy vấn trả về 0 kết quả — tín hiệu rõ nhất cho biết bộ tìm
//! kiếm đang hỏng ở đâu (đề xuất nằm sẵn trong mục BT của PROGRESS.md).
//!
//! **Riêng tư là ràng buộc cứng, không phải tuỳ chọn.** Truy vấn tìm kiếm
//! tiết lộ người dùng có tệp gì trên máy. Vì thế: mặc định **tắt**; bật là
//! một hành động có chủ ý ngay trong giao diện; dữ liệu nằm trong một file
//! văn bản cạnh cache, có nút xem và nút xoá; **không gửi đi đâu, không bao
//! giờ.**
//!
//! Cái khó duy nhất: người dùng gõ từng phím, và `coalesce` phía frontend
//! bắn một lần tìm cho mỗi cụm phím — nghĩa là trên đường gõ "green screen"
//! sẽ có cả chuỗi tiền tố 0-kết-quả vô nghĩa ("gre", "green sc"…). Ghi hết
//! là biến log thành rác. Giải pháp: một ô **chờ lắng** — truy vấn 0-kết-quả
//! chỉ được ghi khi nó đứng yên đủ lâu (không có lần tìm nào đè lên trong
//! [`SETTLE`]); lần tìm kế tiếp tới sớm hơn thì nó bị thay thế, chưa từng
//! chạm đĩa.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Truy vấn 0-kết-quả phải đứng yên chừng này mới được coi là "người dùng
/// thật sự hỏi thế", không phải một chặng gõ dở.
const SETTLE: Duration = Duration::from_secs(2);

/// Quá cỡ này thì cắt bớt, giữ nửa mới nhất — log để đo, không phải để lưu trữ.
const MAX_BYTES: u64 = 256 * 1024;

/// Một dòng trong file, dạng JSON-lines.
#[derive(Debug, Serialize)]
struct Entry<'a> {
    ts_unix: u64,
    query: &'a str,
}

/// Tình hình cho giao diện.
#[derive(Debug, Clone, Serialize)]
pub struct MissLogStatus {
    pub enabled: bool,
    pub count: usize,
}

static ENABLED: AtomicBool = AtomicBool::new(false);
static LOADED: AtomicBool = AtomicBool::new(false);
static PENDING: Mutex<Option<(String, Instant)>> = Mutex::new(None);

/// Ghi đè thư mục dữ liệu trong kiểm thử — không đụng dữ liệu thật của máy.
static DIR_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

fn data_dir() -> Option<PathBuf> {
    if let Some(d) = DIR_OVERRIDE.lock().expect("misslog lock").clone() {
        return Some(d);
    }
    let cache = crate::index::persist::cache_path().ok()?;
    Some(cache.parent()?.to_path_buf())
}

fn log_path() -> Option<PathBuf> {
    Some(data_dir()?.join("misses.jsonl"))
}

/// Cờ bật/tắt là sự tồn tại của một file — đọc không cần parse gì, và một
/// người tò mò mở thư mục lên cũng hiểu ngay mình đang nhìn cái gì.
fn flag_path() -> Option<PathBuf> {
    Some(data_dir()?.join("misses.enabled"))
}

fn ensure_loaded() {
    if LOADED.swap(true, Ordering::SeqCst) {
        return;
    }
    let on = flag_path().map(|p| p.exists()).unwrap_or(false);
    ENABLED.store(on, Ordering::SeqCst);
}

pub fn is_enabled() -> bool {
    ensure_loaded();
    ENABLED.load(Ordering::SeqCst)
}

pub fn set_enabled(on: bool) {
    ensure_loaded();
    ENABLED.store(on, Ordering::SeqCst);
    let Some(flag) = flag_path() else { return };
    if on {
        if let Some(dir) = data_dir() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(flag, b"");
    } else {
        let _ = std::fs::remove_file(flag);
        // Tắt là ngừng ghi; dữ liệu đã ghi vẫn của người dùng — xoá là nút riêng.
        *PENDING.lock().expect("misslog lock") = None;
    }
}

/// Điểm móc duy nhất từ đường tìm kiếm. Rẻ khi tắt: một lần đọc atomic.
pub fn note_search(query: &str, zero_hits: bool) {
    if !is_enabled() {
        return;
    }
    note_search_at(query, zero_hits, Instant::now(), SETTLE);
}

/// Phần lõi tách riêng để kiểm thử được với đồng hồ và ngưỡng tuỳ ý.
fn note_search_at(query: &str, zero_hits: bool, now: Instant, settle: Duration) {
    let mut pending = PENDING.lock().expect("misslog lock");

    // Lần tìm mới tới: ô chờ cũ hoặc đã "chín" (đứng yên đủ lâu — ghi ra),
    // hoặc còn non (bị thay thế, coi như một chặng gõ dở).
    if let Some((old, since)) = pending.take() {
        if now.duration_since(since) >= settle {
            append(&old);
        }
    }

    let q = query.trim();
    if zero_hits && !q.is_empty() {
        *pending = Some((q.to_string(), now));
    }
}

/// Ép ô chờ đã chín xuống đĩa — gọi trước khi đọc/đếm, để "Xem" không thiếu
/// đúng cái truy vấn người dùng vừa bỏ cuộc.
fn flush_ripe(now: Instant, settle: Duration) {
    let mut pending = PENDING.lock().expect("misslog lock");
    if let Some((q, since)) = pending.take() {
        if now.duration_since(since) >= settle {
            append(&q);
        } else {
            *pending = Some((q, since));
        }
    }
}

fn append(query: &str) {
    let Some(path) = log_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let ts_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let Ok(line) = serde_json::to_string(&Entry { ts_unix, query }) else {
        return;
    };
    let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    let _ = writeln!(f, "{line}");
    trim_if_bloated(&path);
}

/// Giữ nửa mới nhất khi file vượt trần.
fn trim_if_bloated(path: &std::path::Path) {
    let too_big = std::fs::metadata(path)
        .map(|m| m.len() > MAX_BYTES)
        .unwrap_or(false);
    if !too_big {
        return;
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    let lines: Vec<&str> = content.lines().collect();
    let keep = &lines[lines.len() / 2..];
    let _ = std::fs::write(path, keep.join("\n") + "\n");
}

pub fn status() -> MissLogStatus {
    flush_ripe(Instant::now(), SETTLE);
    let count = log_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);
    MissLogStatus {
        enabled: is_enabled(),
        count,
    }
}

pub fn clear() {
    *PENDING.lock().expect("misslog lock") = None;
    if let Some(p) = log_path() {
        let _ = std::fs::remove_file(p);
    }
}

/// Đường dẫn file cho nút "Xem" — mở bằng ứng dụng văn bản mặc định.
pub fn file_path() -> Option<String> {
    let p = log_path()?;
    p.exists().then(|| p.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trạng thái nằm trong biến tĩnh dùng chung — các test nối đuôi nhau.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mf-misslog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        *DIR_OVERRIDE.lock().unwrap() = Some(dir.clone());
        *PENDING.lock().unwrap() = None;
        LOADED.store(true, Ordering::SeqCst);
        ENABLED.store(true, Ordering::SeqCst);
        dir
    }

    fn lines(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_to_string(dir.join("misses.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }

    /// Chuỗi gõ dở không được chạm đĩa; truy vấn đứng yên đủ lâu thì được ghi.
    #[test]
    fn chang_go_do_bi_thay_the_truy_van_dung_yen_duoc_ghi() {
        let _g = SERIAL.lock();
        let dir = sandbox("settle");
        let settle = Duration::from_millis(30);
        let t0 = Instant::now();

        // "gre" 0-hit, rồi "green sc" tới NGAY (chưa chín) → "gre" bị thay.
        note_search_at("gre", true, t0, settle);
        note_search_at("green sc", true, t0 + Duration::from_millis(5), settle);
        assert!(lines(&dir).is_empty(), "chang go do da cham dia");

        // "green screen" tới sau khi "green sc" đã chín → "green sc" được ghi.
        note_search_at("green screen", true, t0 + Duration::from_millis(50), settle);
        let l = lines(&dir);
        assert_eq!(l.len(), 1);
        assert!(l[0].contains("green sc"), "ghi sai truy van: {l:?}");

        let _ = std::fs::remove_dir_all(dir);
    }

    /// Lần tìm CÓ kết quả xoá ô chờ — người dùng đã tìm thấy, không có gì để đo.
    #[test]
    fn tim_thay_thi_o_cho_bi_xoa() {
        let _g = SERIAL.lock();
        let dir = sandbox("hit");
        let settle = Duration::from_millis(30);
        let t0 = Instant::now();

        note_search_at("khong ra", true, t0, settle);
        note_search_at("khong r", false, t0 + Duration::from_millis(5), settle);
        flush_ripe(t0 + Duration::from_secs(1), settle);
        assert!(lines(&dir).is_empty(), "truy van da-tim-thay van bi ghi");

        let _ = std::fs::remove_dir_all(dir);
    }

    /// status() ép ô chờ đã chín xuống đĩa — "Xem" không thiếu truy vấn cuối.
    #[test]
    fn status_ep_o_cho_da_chin() {
        let _g = SERIAL.lock();
        let dir = sandbox("flush");
        let settle = Duration::from_millis(1);

        note_search_at("bo cuoc o day", true, Instant::now(), settle);
        std::thread::sleep(Duration::from_millis(10));
        flush_ripe(Instant::now(), settle);
        assert_eq!(lines(&dir).len(), 1);
        assert_eq!(status().count, 1);

        clear();
        assert_eq!(status().count, 0);
        assert!(file_path().is_none(), "file da xoa ma van tra duong dan");

        let _ = std::fs::remove_dir_all(dir);
    }

    /// Tắt là ngừng ghi ngay — kể cả ô chờ đang treo.
    #[test]
    fn tat_la_ngung_ghi() {
        let _g = SERIAL.lock();
        let dir = sandbox("off");

        set_enabled(false);
        assert!(!is_enabled());
        note_search("bat ky", true);
        flush_ripe(
            Instant::now() + Duration::from_secs(10),
            Duration::from_millis(1),
        );
        assert!(lines(&dir).is_empty());

        set_enabled(true);
        assert!(
            dir.join("misses.enabled").exists(),
            "co bat phai nam tren dia"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    /// Vượt trần thì giữ nửa mới nhất.
    #[test]
    fn vuot_tran_giu_nua_moi() {
        let _g = SERIAL.lock();
        let dir = sandbox("trim");
        let path = dir.join("misses.jsonl");

        let long = format!("{{\"ts_unix\":1,\"query\":\"{}\"}}\n", "x".repeat(1024));
        let n = (MAX_BYTES / 1024) as usize + 10;
        std::fs::write(&path, long.repeat(n)).unwrap();
        append("dong moi nhat");
        let l = lines(&dir);
        assert!(l.len() < n, "khong cat gi ca: {} dong", l.len());
        assert!(
            l.last().unwrap().contains("dong moi nhat"),
            "mat dong moi nhat"
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
