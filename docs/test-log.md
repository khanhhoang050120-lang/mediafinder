# NHẬT KÝ TEST — MediaFinder
> **Thuộc file này:** kết quả từng lượt test sau mỗi giai đoạn — chạy gì, ra sao, tìm được lỗi nào.
> Chi tiết từng lỗi nằm ở file phân loại tương ứng, đây chỉ ghi lượt test và kết luận.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

> **Quy tắc:** xong mỗi giai đoạn phải chạy một lượt test **chủ động đi tìm lỗi**,
> không chỉ chạy cho có. Giai đoạn chỉ được coi là xong khi lượt test đã chạy và
> mọi phát hiện đã được ghi vào file phân loại.

### 2026-08-24 — Lượt test sau P0

| # | Nội dung test | Lệnh | Kết quả |
|---|---|---|---|
| 1 | Unit test chạy được | `cargo test` | ✅ pass (0 test — chưa có test nào ở P0) |
| 2 | Không có cảnh báo chất lượng | `cargo clippy --all-targets` | ✅ **sạch**, 0 warning |
| 3 | Dispatch chế độ `--index` | `mediafinder.exe --index` | ✅ log `indexer mode`, **không** mở GUI, dừng đúng ở `unimplemented!` (exit 101) |
| 4 | Manifest nhúng đúng vào exe | `grep` chuỗi trong binary | ✅ có `asInvoker`, `longPathAware`, `PerMonitorV2`; **không** có `requireAdministrator` |
| 5 | Cửa sổ mở và render đúng | `PrintWindow` + xem ảnh | ✅ 916x659, title bar + icon đúng, tiếng Việt đủ dấu, dark theme đúng |
| 6 | Frontend type-check | `npm run check` | ✅ 0 lỗi, 0 warning |
| 7 | Frontend build production | `npm run build` | ✅ JS 28.37 kB (gzip 11.16 kB) |
| 8 | Rust type-check toàn bộ target | `cargo check --all-targets` | ✅ exit 0, 0 lỗi, 0 warning |

**Kết luận lượt test P0:** 8/8 pass. Phát hiện 3 bug + 2 xung đột cấu hình — đã sửa 3, workaround 1,
còn 1 cần người dùng xác minh thủ công (BUG-002).

**Chưa test được ở P0 (để lại giai đoạn sau):**
- Build release (`cargo tauri build`) — để P8, vì tốn thời gian mà chưa có gì để kiểm chứng.
- Hành vi khi từ chối UAC — để P4, khi có luồng elevate.
- Rò rỉ bộ nhớ / giữ lock — để P3, khi có index thật và command thật.

### 2026-08-24 — Lượt test sau P1

| # | Nội dung test | Lệnh / cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **29/29 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch, 0 warning |
| 3 | Thiếu quyền Admin | chạy `--index --dry-run` không elevate | ✅ báo lỗi rõ theo từng ổ, đi tiếp ổ khác, không sập |
| 4 | Ổ non-NTFS | cùng lần chạy trên | ✅ phát hiện đúng `G: (FAT32)`, cảnh báo rõ |
| 5 | Quét MFT thật | chạy elevated qua UAC | ✅ C: 3.559.309 bản ghi / 18,5s · D: 530.731 / 20,4s |
| 6 | Không có bản ghi hỏng | thống kê pha 1 | ✅ 0 malformed, 0 sai phiên bản trên cả 2 ổ |
| 7 | Không trùng tên 8.3 | kiểm tra path mẫu | ✅ không thấy tên dạng `ABCDEF~1.MP4` |
| 8 | Resolve path sâu | đường dẫn sâu nhất | ✅ đúng ở 15+ cấp (`...gradle-8.4\subprojects\...\mipmap-mdpi\ic_launcher.png`) |
| 9 | Tên tiếng Việt | path mẫu ổ D: | ✅ `D:\Sounds Edit\HƯNG\WISE\DATA TẠO VID HƯNG\...` đủ dấu |
| 10 | Không mồ côi / vòng lặp | thống kê pha 2 | ✅ 0 mồ côi, 0 vòng lặp, 0 quá sâu trên cả 2 ổ |
| 11 | Số liệu pha 2 có đúng không | đếm chéo bằng PowerShell | ✅ xem CHECK-001 — khớp, không phải lỗi |
| 12 | Bảng phân loại phần mở rộng | soi 20 path mẫu | ❌ **tìm ra BUG-004** (`.ts` → video) — đã sửa |
| 13 | Đọc lại code tìm lỗi tiềm ẩn | review thủ công | ❌ **tìm ra BUG-005 + PERF-001** — đã sửa |

**Kết luận lượt test P1:** 11/13 pass ngay, 2 mục tìm ra lỗi và đã sửa xong (test lại 29/29 pass).
Phát hiện thêm 1 vấn đề sản phẩm cần người dùng quyết định (ISSUE-001).

**Điểm đáng chú ý:** BUG-004 chỉ lộ ra khi **quét dữ liệu thật** — bộ 28 unit test không hề bắt
được, vì test dùng dữ liệu tổng hợp do chính tôi nghĩ ra và tôi không nghĩ tới `.ts`.
BUG-005 và PERF-001 thì ngược lại, không test nào bắt được vì hàm cần volume thật — chỉ đọc lại
code mới thấy. Bài học: **test tổng hợp, chạy dữ liệu thật, và đọc lại code là ba việc khác nhau,
không thay thế được cho nhau.**

### 2026-08-24 — Lượt test sau P2

| # | Nội dung test | Lệnh / cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **72/72 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch, 0 warning |
| 3 | Fold tiếng Việt | 12 test riêng cho `fold` | ✅ `đ Đ ơ ư ế ự ằ ỗ`, dấu chồng, bảng chữ cái đầy đủ |
| 4 | Fold không phá hệ chữ khác | test Nhật/Hàn/Đức/Ba Lan | ❌ **tìm ra BUG-006** (Hangul) — đã sửa |
| 5 | Fold có idempotent không | fold hai lần | ✅ không đổi — query và index luôn khớp cách fold |
| 6 | Xếp hạng đúng thứ tự | test acceptance của kế hoạch | ✅ `avatar.mkv` > `avatar_extended` > `my_avatar_backup` |
| 7 | Kết quả có tất định không | chạy 12 lần cùng truy vấn | ✅ giống hệt nhau — thứ tự `(score, index)` là toàn phần |
| 8 | Ranh giới chunk | index > 2×CHUNK | ❌ **tìm ra BUG-007** (cấp phát) — đã sửa |
| 9 | Bench 500k entry | `cargo bench` | ✅ worst case **3,01 ms** / mục tiêu 20 ms |
| 10 | Rayon có đáng không | `RAYON_NUM_THREADS=1` so sánh | ✅ song song nhanh hơn **2,6–4,5×** → giữ |
| 11 | Chi phí chọn lọc | bench `selection_cost` | ❌ **tìm ra PERF-002** — tối ưu **−39,5%** |
| 12 | Quét thật + dựng index | chạy elevated trên C: + D: | ✅ 117.123 tệp, index **7,6 MB** |
| 13 | Tìm kiếm trên dữ liệu thật | thử truy vấn sau khi quét | ❌ **tìm ra SPEC-001** — chỉ tìm tên tệp là vô dụng — đã sửa |
| 14 | Truy vấn 0 kết quả là đúng hay sai | đối chiếu bằng PowerShell | ✅ `tieng viet` → 0 là **đúng**, không có thư mục nào như vậy trên D: |
| 15 | Tên thư mục tiếng Việt thật | test dùng tên lấy nguyên văn từ D: | ✅ `nhac nen` `nang dong` `tao vid` `han quoc` đều khớp |

**Kết luận lượt test P2:** 11/15 pass ngay, 4 mục tìm ra vấn đề — đã sửa hết
(`BUG-006`, `BUG-007`, `SPEC-001`, `PERF-002`).

**Vấn đề nghiêm trọng nhất từ đầu dự án là `SPEC-001`**, và nó đáng để rút kinh nghiệm:

- 67 unit test **không bắt được**, vì dữ liệu test do tôi tự nghĩ ra, mà tôi đặt tên tệp có
  nghĩa như người ta thường làm (`holiday.mp4`, `avatar.mkv`).
- Thư viện thật lại tổ chức ngược hẳn: tên thư mục mang toàn bộ ý nghĩa
  (`DATA TẠO VID HƯNG\HAN QUOC\13\BÀI 13...`), tên tệp chỉ là số (`154.mp3`).
- Chỉ khi chạy trên **dữ liệu thật của người dùng** mới lộ ra rằng yêu cầu trong đặc tả là sai.

Bài học bổ sung ở mục 14: **kết quả rỗng cũng phải kiểm chứng.** `tieng viet → 0` trông y hệt
một lỗi fold. Chỉ đối chiếu bằng công cụ độc lập mới phân biệt được "không tìm thấy vì hỏng" và
"không tìm thấy vì không tồn tại". Và câu truy vấn thử nghiệm phải lấy từ dữ liệu thật, không
được tự nghĩ ra — nếu không thì không phân biệt nổi hai trường hợp đó.
