//! Chốt chặn cho hai đường không thể chạy thử trong vòng kiểm tra: việc nâng
//! lịch tác vụ, và những tệp tạm mà hai tiến trình có thể cùng chạm vào.
//!
//! Cả hai đều hỏng theo cùng một kiểu — **im lặng, trên máy người dùng, và chỉ
//! khi có hai thứ chạy cùng lúc** — nên không có bài chạy thật nào bắt được
//! chúng ở đây. Đọc thẳng mã nguồn là cách trung thực duy nhất, và dự án đã có
//! tiền lệ: `installer_hooks.rs` đọc `nsis-hooks.nsh` vì cùng lý do.
//!
//! Chuyện đã dạy ra bài này (P28): đề xuất sửa `upgrade_schedule_if_stale`
//! bằng cách bỏ `/Delete` rồi vẫn gọi `ensure_scheduled_task` — nghe như một
//! dòng, nhưng `ensure_scheduled_task` thoát ngay ở `scheduled_task_exists()`,
//! nên XML mới **không bao giờ được ghi**. Máy mang lịch cũ sẽ ghi log "nâng
//! lên lịch v2" ở mọi lượt chạy, mãi mãi, mà lịch không đổi. Đúng hình dạng
//! của lỗi `SCHEDULE_MARK` đã trả giá ở P22.

const LIB: &str = include_str!("../src/lib.rs");
const SETUP: &str = include_str!("../src/setup.rs");
const PERSIST: &str = include_str!("../src/index/persist.rs");
const ELEVATE: &str = include_str!("../src/ipc/elevate.rs");

/// Cắt lấy thân một hàm Rust ở mức trên cùng: từ chữ ký cho tới dòng `}` đầu
/// tiên nằm ở cột 0.
fn than_ham(nguon: &str, chu_ky: &str) -> String {
    let start = nguon
        .find(chu_ky)
        .unwrap_or_else(|| panic!("khong tim thay {chu_ky:?}"));
    let rest = &nguon[start..];
    let end = rest
        .find("\n}")
        .unwrap_or_else(|| panic!("ham {chu_ky:?} khong duoc dong o cot 0"));
    rest[..end].to_string()
}

/// Bất biến quan trọng nhất: **đường nâng lịch không được đi qua chốt tồn tại.**
///
/// `ensure_scheduled_task` trả về sớm khi tác vụ đã có. Nâng lịch thì luôn luôn
/// gặp một tác vụ đã có — đó là tiền đề của việc nâng. Nên nếu nâng lịch gọi
/// hàm ấy, nó không ghi gì cả.
#[test]
fn nang_lich_khong_duoc_di_qua_chot_ton_tai() {
    let than = than_ham(SETUP, "pub fn upgrade_schedule_if_stale()");

    assert!(
        !than.contains("ensure_scheduled_task()"),
        "upgrade_schedule_if_stale goi ensure_scheduled_task, ma ham do thoat ngay khi \
         task da ton tai — nghia la XML moi KHONG BAO GIO duoc ghi va lich khong bao gio doi"
    );
    assert!(
        than.contains("write_task_definition()"),
        "nang lich phai goi thang duong ghi (write_task_definition), khong qua trung gian"
    );
}

/// Và chốt tồn tại vẫn phải còn ở chỗ của nó — nếu ai đó gỡ nó khỏi
/// `ensure_scheduled_task` thì hai đường lại nhập làm một, và bài trên hết ý
/// nghĩa mà vẫn xanh.
#[test]
fn ensure_van_giu_chot_ton_tai_de_hai_duong_that_su_khac_nhau() {
    let than = than_ham(SETUP, "pub fn ensure_scheduled_task()");
    assert!(
        than.contains("scheduled_task_exists()"),
        "ensure_scheduled_task phai con chot ton tai — do la thu phan biet no voi write_task_definition"
    );
    assert!(
        than.contains("write_task_definition()"),
        "ensure_scheduled_task phai uy quyen phan ghi cho write_task_definition, khong chep lai"
    );
}

/// Nâng lịch không được để lại một khoảng nào mà máy **không có tác vụ**.
///
/// `/Create /XML /F` tự ghi đè. Xoá trước rồi tạo lại thì giữa hai lệnh có một
/// cửa sổ: bước tạo hỏng ở đó (không đọc được `current_exe`, không ghi được XML
/// tạm, phần mềm diệt virus chặn `schtasks`) là chỉ mục thôi tự làm mới hẳn —
/// nặng hơn chính cái bệnh đang chữa.
#[test]
fn nang_lich_khong_duoc_xoa_truoc_khi_tao() {
    let than = than_ham(SETUP, "pub fn upgrade_schedule_if_stale()");
    assert!(
        !than.contains("/Delete"),
        "nang lich khong duoc xoa task truoc: /Create /XML /F da ghi de duoc, con xoa-roi-tao \
         de lai mot cua so ma may khong co tac vu nao"
    );
}

/// Đường ghi phải thật sự dùng cờ ghi đè, nếu không thì bỏ `/Delete` là hỏng.
#[test]
fn duong_ghi_phai_dung_co_ghi_de() {
    let than = than_ham(SETUP, "fn write_task_definition()");
    assert!(
        than.contains("\"/Create\""),
        "write_task_definition phai goi /Create"
    );
    assert!(
        than.contains("\"/F\""),
        "thieu /F thi schtasks tu choi ghi de len task da co, va viec nang lich that bai lang le"
    );
}

/// Mọi tệp tạm dùng chung phải mang định danh tiến trình.
///
/// Ba chỗ, cùng một kiểu hỏng: tác vụ định kỳ (15 phút một lượt) và tiến trình
/// giao diện (lượt quét ổ mạng 140–175 giây) có thể cùng đi qua. Với một đường
/// dẫn tạm cố định, tiến trình sau cắt cụt hoặc xoá mất tệp của tiến trình
/// trước. Với `index.bin.tmp` thì hậu quả nặng nhất: `rename` đưa một tệp lai
/// vào chỗ chỉ mục, header 12 byte vẫn hợp lệ nên nó qua được chốt kiểm tra,
/// `load()` trả `Corrupt`, và lượt quét đầy đủ theo sau xoá sạch mục ổ mạng vì
/// indexer nâng quyền không nhìn thấy ổ ánh xạ (CHECK-007).
#[test]
fn tep_tam_phai_rieng_theo_tien_trinh() {
    for (ten, nguon, chu_ky) in [
        (
            "index.bin.tmp",
            PERSIST,
            "pub fn save(index: &Index, volumes: Vec<VolumeStamp>)",
        ),
        ("progress json.tmp", ELEVATE, "fn write_atomic(&self)"),
        ("mediafinder-task.xml", SETUP, "fn write_task_definition()"),
    ] {
        let than = than_ham(nguon, chu_ky);
        assert!(
            than.contains("std::process::id()"),
            "tep tam cua {ten} dung duong dan co dinh — hai tien trinh ghi cung luc se dam nhau"
        );
    }
}

/// **Lượt kiểm không đổi gì vẫn phải để lại dấu vết.**
///
/// `run_incremental` cố ý không ghi lại cache khi journal không có gì đáng áp —
/// đó là quyết định đúng, nó tránh ~4,5 GB ghi SSD mỗi ngày cho việc không làm
/// gì. Nhưng `built_at_unix` chỉ được đóng dấu trong `persist::save()`, nên
/// nhánh ấy đi qua mà không để lại bất kỳ bằng chứng nào là tác vụ có chạy.
///
/// Hậu quả trên máy người dùng: một máy hoàn toàn khoẻ, tác vụ vừa chạy hai
/// phút trước, nhưng buổi tối không ai đụng vào tệp nào — và giao diện tô vàng
/// "Ổ trong máy: 4 giờ trước" ngay trên màn hình "Không tìm thấy kết quả nào".
/// Cảnh báo sai đúng vào lúc người dùng đang cố hiểu vì sao không ra kết quả
/// còn tệ hơn không cảnh báo: nó chỉ họ đi sai hướng, và vài lần như thế thì họ
/// thôi đọc mọi cảnh báo.
///
/// Bài kiểm đơn vị của `lastcheck` chứng minh hàm `record` chạy đúng, nhưng
/// không thể chứng minh nó ĐƯỢC GỌI ở đúng nhánh — đó là việc của bài này.
#[test]
fn nhanh_khong_doi_gi_van_phai_dong_dau_da_kiem() {
    let than = than_ham(LIB, "pub fn run_incremental()");

    let moc = than
        .find("không ghi lại cache")
        .expect("khong tim thay nhanh 'khong doi gi' trong run_incremental");
    // Khối kết thúc của nhánh ấy: từ dòng log tới `return true;` gần nhất.
    let phan_con_lai = &than[moc..];
    let het = phan_con_lai
        .find("return true;")
        .expect("nhanh 'khong doi gi' phai ket thuc bang return true");
    let khoi = &phan_con_lai[..het];

    assert!(
        khoi.contains("lastcheck::record"),
        "nhanh khong-doi-gi thoat ma khong dong dau 'da kiem' — giao dien se \
         tuong chi muc da cu hang gio tren mot may hoan toan khoe"
    );
}

/// Và đường có thay đổi cũng phải đóng dấu — nếu không, mốc chỉ tươi trên
/// những máy yên tĩnh, tức đúng ngược với thứ ta muốn đo.
#[test]
fn duong_co_thay_doi_cung_phai_dong_dau() {
    let than = than_ham(LIB, "pub fn run_incremental()");
    let so_lan = than.matches("lastcheck::record").count();
    assert!(
        so_lan >= 2,
        "moi ket thuc thanh cong cua run_incremental deu phai dong dau; \
         dem duoc {so_lan} loi goi"
    );
}
