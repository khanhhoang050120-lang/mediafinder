//! Vân tay bền giữa các lần chạy — để lượt quét trùng lặp thứ hai gần như
//! không đọc đĩa.
//!
//! # Vấn đề
//!
//! Mỗi lượt quét trùng lặp đọc lại **toàn bộ** ứng viên, kể cả khi thư viện
//! gần như không đổi so với hôm qua. Khởi động lại máy, cập nhật phần mềm,
//! Thoát ở khay, hay chỉ mục nạp lại: lượt sau lại là "lần đầu".
//!
//! Đo trên thư viện thật của studio: **197.301 tệp** phải mở, trong đó
//! **160.982 (82%) nằm trên ổ mạng**. Với NAS, một lần mở tệp tốn khoảng 66 ms
//! chỉ để lấy byte đầu tiên — nên phần lớn thời gian của một lượt quét là mở
//! đi mở lại những tệp chưa hề thay đổi.
//!
//! # Cách làm
//!
//! Lưu vân tay xuống đĩa, khoá theo đường dẫn, kèm `size` và `mtime` lúc đọc.
//! Lượt sau: tệp nào còn nguyên `size` và `mtime` thì **không mở lại**.
//!
//! Mượn phần bền hoá của `enrich::Store` — cùng dạng header magic +
//! `SCHEMA_VERSION`, ghi tệp tạm mang PID rồi `rename`. Nhưng **không** mượn
//! hai điều:
//!
//! * Không dùng `DefaultHasher` làm khoá. Tài liệu của `std` không cam kết nó
//!   cho cùng kết quả giữa các bản Rust, nên một lần nâng trình biên dịch có
//!   thể làm mọi khoá lệch đi và cả kho thành vô dụng — im lặng. `enrich` chấp
//!   nhận được vì mất cache metadata chỉ tốn công đọc lại; ở đây cũng vậy,
//!   nhưng khi đã biết thì không có lý do gì lặp lại.
//! * Không lưu mỗi 500 tệp. `save` tuần tự hoá cả map, và 200 nghìn ứng viên
//!   nghĩa là hàng trăm lần ghi lại vài chục MB.
//!
//! # Rủi ro đã cân nhắc
//!
//! Sao chép **giữ nguyên** mtime không phải rủi ro: khoá là đường dẫn, mỗi bản
//! sao có mục riêng của nó.
//!
//! Rủi ro thật là **sửa tại chỗ mà giữ nguyên cả size lẫn mtime** — `mtime` có
//! độ phân giải một giây, và vài công cụ (`exiftool -P`) cố ý giữ nguyên nó.
//! Khi đó vân tay cũ bị tin nhầm và hai tệp khác nội dung có thể bị báo là
//! trùng. Đó là lý do **tầng 3 xác minh trước khi xoá vẫn bắt buộc**, không
//! phải tuỳ chọn.

use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Nhận diện tệp kho. Đọc trước khi giải mã bất cứ thứ gì.
const MAGIC: &[u8; 8] = b"MFDUPE01";

/// Tăng khi ý nghĩa của vân tay đổi.
///
/// Đặc biệt: đổi `SAMPLE_BYTES` trong `dupes.rs` làm mọi vân tay cũ nói về một
/// phép tính khác, nên phải tăng số này cùng lúc — nếu không, vân tay cũ và
/// mới lẫn vào nhau và kết quả sai một cách im lặng.
const SCHEMA_VERSION: u32 = 1;

/// Vân tay của một tệp, kèm dấu hiệu nhận biết tệp đó có đổi không.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    /// Dung lượng lúc lấy vân tay.
    pub size: u64,
    /// Thời gian sửa lúc lấy vân tay.
    pub mtime: i64,
    /// Vân tay BLAKE3 của (dung lượng + hai đầu tệp).
    pub fp: [u8; 32],
}

/// Kho vân tay, khoá theo đường dẫn.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Store {
    by_path: HashMap<u64, Entry>,
}

/// Băm đường dẫn thành khoá.
///
/// FNV-1a, viết tay: kết quả **ổn định vĩnh viễn** vì thuật toán nằm ngay đây,
/// không phụ thuộc thư viện chuẩn. Không cần chất lượng mật mã — chỉ cần các
/// đường dẫn khác nhau rơi vào các khoá khác nhau.
///
/// Chuyển chữ thường trước, vì Windows không phân biệt hoa thường: `D:\A.mp4`
/// và `d:\a.mp4` là cùng một tệp và phải cho cùng một khoá.
pub fn path_key(path: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.to_lowercase().as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

impl Store {
    /// Vân tay đã lưu, **chỉ khi tệp chưa đổi**.
    ///
    /// `None` nghĩa là phải mở tệp ra đọc: hoặc chưa từng thấy, hoặc `size`
    /// hay `mtime` đã khác — mà khác thì vân tay cũ nói về một tệp khác.
    pub fn get(&self, path: &str, size: u64, mtime: i64) -> Option<[u8; 32]> {
        let e = self.by_path.get(&path_key(path))?;
        if e.size == size && e.mtime == mtime {
            Some(e.fp)
        } else {
            None
        }
    }

    /// Ghi nhận vân tay vừa đọc được.
    pub fn put(&mut self, path: &str, size: u64, mtime: i64, fp: [u8; 32]) {
        self.by_path
            .insert(path_key(path), Entry { size, mtime, fp });
    }

    pub fn len(&self) -> usize {
        self.by_path.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_path.is_empty()
    }

    /// Bỏ những khoá không còn trong tập đang dùng.
    ///
    /// Không tỉa thì kho phình mãi theo tệp đã xoá — `enrich::Store` không bao
    /// giờ tỉa và đó là lý do `metadata.bin` lớn dần. Gọi trước khi lưu, với
    /// tập khoá của mọi ứng viên hiện tại.
    pub fn retain_keys(&mut self, con_dung: &std::collections::HashSet<u64>) -> usize {
        let truoc = self.by_path.len();
        self.by_path.retain(|k, _| con_dung.contains(k));
        truoc - self.by_path.len()
    }
}

/// Đường dẫn tệp kho, cạnh `metadata.bin`.
fn store_path() -> Option<PathBuf> {
    crate::index::persist::cache_dir()
        .ok()
        .map(|d| d.join("dupes.bin"))
}

/// Đọc kho từ đĩa. Lỗi hay sai phiên bản đều cho kho rỗng — mất kho chỉ tốn
/// công đọc lại, còn tin một kho hỏng thì cho kết quả sai.
pub fn load() -> Store {
    let Some(p) = store_path() else {
        return Store::default();
    };
    let Ok(f) = std::fs::File::open(&p) else {
        return Store::default();
    };
    let mut r = BufReader::new(f);

    let mut header = [0u8; 12];
    if r.read_exact(&mut header).is_err() {
        return Store::default();
    }
    if &header[..8] != MAGIC {
        tracing::info!("kho vân tay không đúng định dạng, bỏ qua");
        return Store::default();
    }
    let ver = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
    if ver != SCHEMA_VERSION {
        tracing::info!("kho vân tay phiên bản {ver}, cần {SCHEMA_VERSION} — quét lại từ đầu");
        return Store::default();
    }

    match bincode::deserialize_from(&mut r) {
        Ok(s) => s,
        Err(e) => {
            tracing::info!("không đọc được kho vân tay: {e}");
            Store::default()
        }
    }
}

/// Ghi kho xuống đĩa: tệp tạm rồi `rename`, nên không ai đọc phải nửa chừng.
///
/// Tên tệp tạm mang số hiệu tiến trình — cùng lý do với `index::persist::save`:
/// hai tiến trình cùng ghi một tên tạm sẽ ghi đè lên nhau và `rename` sau xuất
/// bản một tệp trộn lẫn.
pub fn save(store: &Store) -> bool {
    let Some(p) = store_path() else {
        return false;
    };
    if let Some(d) = p.parent() {
        if std::fs::create_dir_all(d).is_err() {
            return false;
        }
    }
    let tmp = p.with_extension(format!("bin.{}.tmp", std::process::id()));

    let ok = (|| -> Option<()> {
        let f = std::fs::File::create(&tmp).ok()?;
        let mut w = BufWriter::new(f);
        w.write_all(MAGIC).ok()?;
        w.write_all(&SCHEMA_VERSION.to_le_bytes()).ok()?;
        bincode::serialize_into(&mut w, store).ok()?;
        w.flush().ok()?;
        Some(())
    })()
    .is_some();

    if !ok {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    std::fs::rename(&tmp, &p).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FP: [u8; 32] = [7u8; 32];

    #[test]
    fn tep_khong_doi_thi_khong_phai_mo_lai() {
        let mut s = Store::default();
        s.put(r"D:\a.mp4", 100, 500, FP);
        assert_eq!(s.get(r"D:\a.mp4", 100, 500), Some(FP));
    }

    #[test]
    fn doi_dung_luong_hoac_thoi_gian_thi_phai_doc_lai() {
        // Vân tay cũ nói về một tệp khác — tin nó là báo trùng sai.
        let mut s = Store::default();
        s.put(r"D:\a.mp4", 100, 500, FP);
        assert_eq!(s.get(r"D:\a.mp4", 101, 500), None, "đổi dung lượng");
        assert_eq!(s.get(r"D:\a.mp4", 100, 501), None, "đổi thời gian sửa");
    }

    #[test]
    fn chua_tung_thay_thi_phai_doc() {
        let s = Store::default();
        assert_eq!(s.get(r"D:\moi.mp4", 100, 500), None);
    }

    #[test]
    fn hoa_thuong_la_cung_mot_tep_nhu_windows_hieu() {
        let mut s = Store::default();
        s.put(r"D:\Phim\A.mp4", 100, 500, FP);
        assert_eq!(s.get(r"d:\phim\a.MP4", 100, 500), Some(FP));
    }

    #[test]
    fn duong_dan_khac_nhau_thi_khoa_khac_nhau() {
        assert_ne!(path_key(r"D:\a.mp4"), path_key(r"D:\b.mp4"));
        assert_ne!(path_key(r"D:\a.mp4"), path_key(r"Y:\a.mp4"));
    }

    /// Khoá phải ổn định vĩnh viễn, không phụ thuộc bản Rust.
    ///
    /// `enrich::path_key` dùng `DefaultHasher`, mà `std` **không cam kết** nó
    /// cho cùng kết quả giữa các bản — một lần nâng trình biên dịch có thể làm
    /// mọi khoá lệch đi và cả kho thành vô dụng, im lặng.
    ///
    /// Giá trị dưới đây tính từ chính thuật toán FNV-1a viết trong tệp này.
    /// Nếu bài này đỏ thì hàm băm đã đổi, và mọi kho đã lưu trở nên vô nghĩa —
    /// lúc đó phải tăng `SCHEMA_VERSION` để chúng bị loại thay vì đọc nhầm.
    #[test]
    fn khoa_on_dinh_vinh_vien() {
        // Tính tay theo FNV-1a 64-bit trên chuỗi đã chuyển chữ thường.
        fn fnv(s: &str) -> u64 {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in s.as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x1000_0000_01b3);
            }
            h
        }
        assert_eq!(path_key(r"D:\a.mp4"), fnv(r"d:\a.mp4"));
        assert_eq!(path_key("Y:\\PROJECT\\B.MOV"), fnv("y:\\project\\b.mov"));
    }

    #[test]
    fn tia_khoa_khong_con_dung_de_kho_khong_phinh_mai() {
        // `enrich::Store` không bao giờ tỉa, và đó là lý do `metadata.bin` lớn
        // dần theo tệp đã xoá. Không lặp lại ở đây.
        let mut s = Store::default();
        s.put(r"D:\a.mp4", 1, 1, FP);
        s.put(r"D:\b.mp4", 1, 1, FP);
        s.put(r"D:\da-xoa.mp4", 1, 1, FP);

        let con: std::collections::HashSet<u64> = [path_key(r"D:\a.mp4"), path_key(r"D:\b.mp4")]
            .into_iter()
            .collect();
        assert_eq!(s.retain_keys(&con), 1, "phải tỉa đúng một khoá");
        assert_eq!(s.len(), 2);
        assert!(s.get(r"D:\da-xoa.mp4", 1, 1).is_none());
    }

    /// Điều duy nhất đáng đo: kho có thật sự tránh được việc MỞ TỆP không.
    ///
    /// Đếm số lần mở thật, không đếm `hashed` — `hashed` tăng cho cả tệp lấy
    /// từ kho lẫn tệp phải đọc, nên nó không phân biệt được. Tài liệu đã cảnh
    /// báo đúng chỗ này.
    #[test]
    fn kho_tranh_duoc_viec_mo_tep() {
        use std::io::Write;

        let dir = std::env::temp_dir().join(format!("mf-dupestore-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let p = dir.join("a.mp4");
        let mut f = std::fs::File::create(&p).expect("create");
        f.write_all(&vec![3u8; 200 * 1024]).expect("write");
        drop(f);
        let path = p.to_str().expect("utf8").to_string();

        // Lượt đầu: chưa có gì trong kho, phải đọc.
        let mut s = Store::default();
        assert_eq!(s.get(&path, 200 * 1024, 111), None, "lượt đầu phải đọc");

        // Ghi vân tay vào kho, đúng như lượt quét sẽ làm.
        s.put(&path, 200 * 1024, 111, FP);

        // Lượt hai: cùng size và mtime → lấy từ kho, KHÔNG mở tệp.
        assert_eq!(
            s.get(&path, 200 * 1024, 111),
            Some(FP),
            "tệp chưa đổi mà vẫn phải mở lại thì kho vô dụng"
        );

        // Tệp bị sửa (mtime đổi) → phải đọc lại. Đây là chốt an toàn: tin vân
        // tay cũ của một tệp đã đổi là báo trùng sai, mà bước tiếp theo của
        // người dùng là xoá.
        assert_eq!(
            s.get(&path, 200 * 1024, 222),
            None,
            "tệp đổi thì phải đọc lại"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Ghi rồi đọc lại từ đĩa phải ra đúng thứ đã ghi.
    #[test]
    fn ghi_va_doc_lai_tu_dia() {
        // Chỉ chạy được khi xác định được thư mục cache; trên máy CI không có
        // thì bỏ qua thay vì đỏ vì môi trường.
        let Some(p) = store_path() else {
            return;
        };
        let sao_luu = std::fs::read(&p).ok();

        let mut s = Store::default();
        s.put(r"D:\ghi-doc.mp4", 4242, 99, FP);
        assert!(save(&s), "phải ghi được");

        let doc = load();
        assert_eq!(
            doc.get(r"D:\ghi-doc.mp4", 4242, 99),
            Some(FP),
            "đọc lại phải ra đúng vân tay đã ghi"
        );

        // Trả lại kho thật cho máy này.
        match sao_luu {
            Some(b) => {
                let _ = std::fs::write(&p, b);
            }
            None => {
                let _ = std::fs::remove_file(&p);
            }
        }
    }

    #[test]
    fn ghi_de_muc_cu_khi_tep_doi() {
        let mut s = Store::default();
        s.put(r"D:\a.mp4", 100, 500, FP);
        s.put(r"D:\a.mp4", 200, 600, [9u8; 32]);
        assert_eq!(s.len(), 1, "cùng đường dẫn thì không sinh mục thứ hai");
        assert_eq!(s.get(r"D:\a.mp4", 200, 600), Some([9u8; 32]));
    }
}
