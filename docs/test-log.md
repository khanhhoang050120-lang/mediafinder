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

### 2026-08-24 — Lượt test sau P3

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **85/85 pass** (84 unit + 1 integration) |
| 2 | Chất lượng code Rust | `cargo clippy --all-targets` | ✅ sạch, 0 warning |
| 3 | Type-check frontend | `npm run check` | ✅ 0 lỗi, 0 warning |
| 4 | Indexer ghi cache | chạy `--index` (không `--dry-run`) | ✅ `index.bin` **8.331.574 byte**, 117.124 tệp |
| 5 | GUI nạp cache **không cần UAC** | mở app, đọc log | ✅ nạp trong ~300ms, 117.124 tệp / 4.196 thư mục |
| 6 | Giao diện render đúng | `PrintWindow` | ✅ ô nhập, 3 chip lọc, status bar hiện số liệu index |
| 7 | Gõ phím → ra kết quả | UI Automation `ValuePattern.SetValue` | ✅ gõ `nhac nen` → **55 kết quả trong 4,6 ms** |
| 8 | Tìm không dấu qua toàn bộ luồng | đọc lại cây UIA | ✅ ra `nhạc nền.mp3`, `nhac nen tho.MP3`, `nhạc nền hàn.MP3` |
| 9 | Xếp hạng hiển thị đúng | nhìn ảnh chụp | ✅ khớp tên tệp lên đầu, khớp tên thư mục (`NHẠC NỀN`) xếp dưới |
| 10 | `ShellExecuteW` báo lỗi đúng chiều | integration test | ✅ đường dẫn không mở được → `Err`, không giả vờ thành công |
| 11 | **Mở thư mục chứa tệp** | integration test có tác dụng phụ | ✅ Explorer mở đúng thư mục |
| 12 | **Tệp có được bôi đen không** | `Shell.Application` đọc `SelectedItems()` | ✅ `>>> DANG BOI DEN: Bài 13, Tiếng Việt — Đà Nẵng (thử)` |
| 13 | Tên tệp có dấu phẩy | dùng chính tên trên | ✅ hoạt động — đây là ca làm hỏng `explorer.exe /select` |
| 14 | Đường dẫn không tồn tại | integration test | ✅ trả `Err`, không dereference PIDL null |
| 15 | Xung đột phím khi menu mở | đọc lại code | ❌ **tìm ra lỗi** — `Esc` vừa đóng menu vừa xoá ô tìm kiếm — đã sửa |
| 16 | CSP có chặn IPC không | đọc lại cấu hình | ❌ **tìm ra rủi ro** — thiếu `connect-src ipc: http://ipc.localhost` — đã thêm |
| 17 | Chạy lại dev sau khi dừng | `npm run tauri dev` | ❌ **tìm ra `CONF-004`** — vite còn sót giữ port 1420 |

**Kết luận lượt test P3:** 14/17 pass ngay, 3 mục tìm ra vấn đề — đã sửa 2, 1 để `WORKAROUND`.

**Mục 12 là bằng chứng quan trọng nhất.** Test tự báo "thành công" là chưa đủ: `SHOpenFolderAndSelectItems`
có thể mở thư mục mà **không** bôi đen gì cả, và test vẫn pass. Phải hỏi ngược lại Explorer bằng
`Shell.Application` xem nó đang chọn cái gì. Tên tệp trong test cố ý chứa **dấu phẩy** — thứ làm
hỏng cách `explorer.exe /select,"path"` mà phần lớn hướng dẫn trên mạng khuyên dùng, và là lý do
dự án này chọn COM API.

**Mục 15 và 16 đều tìm ra bằng cách đọc lại code, không phải bằng chạy.** Mục 15 là xung đột giữa
hai `svelte:window` cùng nghe trên `window` — `stopPropagation` không chặn được listener anh em.
Mục 16 là CSP `default-src 'self'` sẽ chặn `connect-src`, mà IPC của Tauri v2 trên Windows đi qua
`http://ipc.localhost`.

**Cách kiểm chứng giao diện đáng ghi lại.** UI Automation nhìn **không** thấy nội dung WebView2 từ
cửa sổ gốc — phải tìm cửa sổ con class `WRY_WEBVIEW` rồi `AutomationElement.FromHandle` trên đó.
Từ đó `ValuePattern.SetValue` gõ được vào ô tìm kiếm và **kích hoạt đúng sự kiện `input`** của
Svelte, nên kiểm chứng được toàn bộ luồng Svelte → IPC → Rust → kết quả mà không cần chuột.

### 2026-08-24 — Người dùng báo lỗi tìm kiếm (sau P3)

Người dùng thử trên dữ liệu thật và báo: tìm `The anglerfish` thì **ra** tệp cần tìm, nhưng dán
nguyên tiêu đề `The anglerfish: The original approach to deep-sea fishing` thì **0 kết quả**.

| # | Việc làm | Kết quả |
|---|---|---|
| 1 | Tái hiện trước khi kết luận | ✅ viết test tạm đối chiếu từng từ khoá với tên tệp thật |
| 2 | Xác định nguyên nhân | ✅ **hai** nguyên nhân tách biệt, không phải một |
| 3 | Sửa tách token theo dấu câu | ✅ `anglerfish:` → `anglerfish`, `deep-sea` → `deep`+`sea` |
| 4 | Thêm cơ chế lùi về khớp một phần | ✅ `SPEC-002` |
| 5 | Chạy thật lần 1 | ⚠️ tìm ra tệp, nhưng **173 kết quả** — 2 đúng, 171 rác |
| 6 | Siết lại: chỉ giữ nhóm khớp nhiều nhất | ✅ **2 kết quả**, cả hai đều đúng |
| 7 | Toàn bộ test | ✅ **96/96 pass**, clippy sạch |
| 8 | Hồi quy `nhac nen` | ✅ 55 kết quả, khớp chính xác — không đổi |
| 9 | Hồi quy `The anglerfish` | ✅ 10 kết quả, khớp chính xác — không đổi |
| 10 | Không bịa kết quả | ✅ `avatar 1999 khongcogi` → không có kết quả |
| 11 | Giao diện báo rõ khớp một phần | ✅ băng thông báo + huy hiệu `6/9` mỗi dòng |

**Bước 5 là bước quan trọng nhất của lượt này.** Sau lần sửa đầu, tệp cần tìm **đã ra** và nằm ở
vị trí đầu — về mặt kỹ thuật là "đã sửa xong". Nhưng chạy thật cho thấy nó kèm theo 171 kết quả
khớp 5/9 toàn thứ không liên quan. Đúng về thuật toán, vô dụng khi nhìn bằng mắt.

Nếu chỉ kiểm bằng unit test — "tệp cần tìm có trong kết quả không?" — thì đã pass và tôi đã dừng
lại ở đó. Phải **nhìn vào toàn bộ danh sách trên màn hình thật** mới thấy vấn đề.

### 2026-08-24 — Lượt test sau P4

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **100/100 pass** |
| 2 | Chất lượng code Rust | `cargo clippy --all-targets` | ✅ sạch |
| 3 | Type-check frontend | `npm run check` | ✅ 0 lỗi, 0 warning |
| 4 | Nút "Quét lại" có trên giao diện | UIA tìm control Button | ✅ tìm thấy, bấm được |
| 5 | **Từ chối UAC** | bấm No trên hộp thoại | ✅ báo *"Chưa quét gì cả — dữ liệu cũ vẫn nguyên"* |
| 6 | Từ chối UAC không mất dữ liệu | đọc status bar sau đó | ✅ vẫn `117.124 tệp · quét lúc 13:34:30` |
| 7 | Từ chối UAC không kẹt nút | đọc trạng thái nút | ✅ "Quét lại" trở lại bình thường |
| 8 | `progress.json` ghi đúng định dạng | đọc file, parse JSON | ✅ hợp lệ, camelCase, đủ trường |
| 9 | Lượt quét thất bại vẫn đặt `finished` | chạy `--index` không elevate | ✅ `finished: true` — GUI không poll vô tận |
| 10 | **Quét thất bại có phá cache không** | sao lưu → chạy → đo lại | ❌ **tìm ra `BUG-008`** — đã sửa |
| 11 | Guard bảo vệ cache | chạy lại sau khi sửa | ✅ cache **8.331.574 → 8.331.574 byte**, nguyên vẹn |
| 12 | Test chống tái phát | `tests/cache_safety.rs --ignored` | ✅ 2/2 pass |
| 13 | **Chấp nhận UAC → quét thật** | cần người dùng bấm Yes | ⏳ **chưa kiểm chứng được** |

**Kết luận lượt test P4:** 11/13 pass, 1 mục tìm ra lỗi nặng và đã sửa, 1 mục còn chờ.

**`BUG-008` không tìm ra bằng cách chạy hỏng, mà bằng cách đặt câu hỏi.** Đang tìm cách kiểm chứng
luồng quét mà không cần UAC, tôi tự hỏi *"chạy `--index` không có quyền Admin thì sao?"* — và lần
theo code thì thấy mọi ổ đều `continue`, index rỗng đi thẳng tới `persist::save()` và **ghi đè
cache đang dùng tốt**.

Loại lỗi này không lượt chạy bình thường nào phơi ra được, vì đường hạnh phúc luôn có đủ quyền.
Nó chỉ lộ khi hỏi *"nếu bước này thất bại thì sao?"* ở từng chỗ có `continue` hoặc bỏ qua lỗi.

**Mục 5 lại là may mắn.** UAC bị từ chối ngoài ý muốn, nhưng nhờ đó **đường xử lý khó nhất được
kiểm chứng trước** — và nó đúng hoàn toàn.

### 2026-08-24 — Lượt test sau P5

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **108/108 pass** |
| 2 | Chất lượng code Rust | `cargo clippy --all-targets` | ✅ sạch |
| 3 | Type-check frontend | `npm run check` | ✅ 0 lỗi, 0 warning |
| 4 | Dựng thumbnail thật | integration test trên thư viện người dùng | ✅ video + ảnh, kèm số byte và thời gian |
| 5 | Ảnh có bị lật ngược không | **nhìn ảnh chụp** | ✅ trời trên, đường dưới — thứ tự dòng top-down đúng |
| 6 | Màu có bị đảo không | **nhìn ảnh chụp** | ✅ trời xanh, cỏ xanh — hoán BGR→RGB đúng |
| 7 | Cache thumbnail | đo lần 1 vs lần 2 | ✅ 13,65ms → **0,003ms**, cùng con trỏ `Arc` |
| 8 | Kích thước thumbnail | đọc header PNG | ❌ **tìm ra `BUG-010`** — 1280×720 thay vì 192 — đã sửa |
| 9 | Thumbnail hiện trong lưới | **nhìn ảnh chụp** | ❌ **tìm ra `BUG-009`** — ô trống hoàn toàn — đã sửa |
| 10 | Video có khung hình thật không | **nhìn ảnh chụp** | ❌ **tìm ra `BUG-011`** — icon chung — đã sửa |
| 11 | Ảo hoá: số node theo số kết quả | đếm phần tử UIA | ✅ 38 kết quả → 118 node · **5.000 kết quả → 118 node** |
| 12 | Chuyển chế độ danh sách ↔ lưới | bấm nút qua UIA | ✅ hoạt động |
| 13 | Giới hạn kích thước phía server | unit test `parse_size` | ✅ `s=999999` bị chặn ở 512 |

**Kết luận lượt test P5:** 10/13 pass, 3 mục tìm ra lỗi — cả ba đều đã sửa.

**Điểm đáng ghi nhất của P5: cả ba lỗi đều tìm ra bằng cách NHÌN, không phải bằng cách đọc kết quả
test.**

- `BUG-010` — test **pass**, nhưng con số `1269526 byte` in ra đập vào mắt. Nếu test chỉ
  `assert!(png.len() > 200)` thì lỗi đã lọt tới khi người dùng thấy app ăn 650 MB RAM.
- `BUG-009` — không có lỗi nào ở đâu cả. Trình duyệt im lặng khi `<img>` nhận 400, `onerror` của
  tôi lại **giấu** ảnh hỏng đi. Chỉ ảnh chụp mới cho thấy ô trống. Sau đó thêm **một dòng log**
  là ra ngay nguyên nhân.
- `BUG-011` — thumbnail đã hiện, test vẫn pass. Nhưng nhìn kỹ thì mọi video đều cùng một icon xám.

Bài học chung: **"có ảnh" không có nghĩa là "đúng ảnh".** Ba lỗi này đều lọt qua mọi assertion về
kiểu dữ liệu và kích thước; chỉ có mắt người mới phân biệt được khung hình thật với icon chung.

### 2026-08-24 — Lượt test sau P6

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **118/118 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch (4 assertion hằng số → chuyển sang `const _: () = assert!`) |
| 3 | Type-check frontend | `npm run check` | ✅ 0 lỗi, 0 warning |
| 4 | Lượt nhanh dung lượng | quét thật | ✅ 117.128 tệp / **3.014,6 GB** trong 13,1s |
| 5 | `IPropertyStore` trên tệp thật | integration test in ra số đo | ✅ video 1920×1080 + thời lượng, 7/9 tệp đọc được |
| 6 | Cache cũ sau khi đổi schema | nạp lại | ❌ **tìm ra `BUG-012`** — báo "hỏng" thay vì "phiên bản cũ" |
| 7 | Enrichment lưu bền | khởi động lại app | ✅ nạp ngay **50.947 mục có sẵn** |
| 8 | **Bộ lọc `≥1080p`** | bấm qua UIA | ❌ **tìm ra `BUG-013`** — 0 kết quả — đã sửa |
| 9 | Bộ lọc sau khi sửa | bấm lại | ✅ **5.000 kết quả trong 9,1 ms** |
| 10 | Hiển thị thuộc tính trên dòng | nhìn ảnh chụp | ✅ `4K · 0:04 · 3.2 MB` |
| 11 | Chỉ báo tiến độ | nhìn ảnh chụp | ✅ *"Đã đọc thuộc tính 54.822/117.128 tệp"* |

**Kết luận lượt test P6:** 9/11 pass, 2 mục tìm ra lỗi — cả hai đã sửa.

**`BUG-013` là lỗi im lặng nhất từ đầu dự án.** Không có lỗi ở đâu cả: enrichment chạy đúng, đếm
đúng, lưu đúng, chỉ báo tăng đều. Bộ lọc trả về 0 — mà 0 là **câu trả lời hợp lệ** cho một bộ lọc.
Cả hai bên đều đúng theo cách nhìn riêng của chúng.

Chỉ vì tôi **đã biết chắc** `1.mp4` là 1920×1080 từ lượt test trước nên con số 0 mới trở thành mâu
thuẫn. Nếu không có phép đo độc lập đó, tôi đã kết luận "chưa enrich tới" và đi tiếp.

**`BUG-012` chỉ lộ ở lần đổi schema đầu tiên.** Nếu không gặp bây giờ thì sẽ gặp ở P7 hoặc P8 —
lúc đó là dữ liệu thật của người dùng, và thông báo "cache hỏng" sẽ khiến họ nghĩ đĩa có vấn đề.

### 2026-08-24 — Lượt test sau P7

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **124/124 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch |
| 3 | Type-check frontend | `npm run check` | ✅ 0 lỗi, 0 warning |
| 4 | Hai tệp giống nhau | unit test | ✅ cùng vân tay |
| 5 | Khác ở đầu tệp | unit test | ✅ khác vân tay |
| 6 | **Khác ở giữa tệp** | unit test | ✅ tầng 2 **không** phân biệt được — giới hạn đã biết, tầng 3 thì được |
| 7 | Dung lượng có trong vân tay | unit test | ✅ cùng nội dung + khác dung lượng → khác vân tay |
| 8 | Tệp nhỏ đọc toàn bộ | unit test | ✅ vẫn phân biệt được |
| 9 | Quét thật 3 TB | bấm nút qua UIA | ✅ **6.780 nhóm · 520,7 GB** trong 584s |
| 10 | Thanh tiến độ khi quét | nhìn ảnh chụp | ✅ "Đang đối chiếu 51.098/70.576 tệp" |
| 11 | Đơn vị dung lượng | nhìn ảnh chụp | ❌ **tìm ra lỗi** — "17048.5 MB" thay vì "16.6 GB" |
| 12 | Tiêu đề có trung thực không | nhìn ảnh chụp | ❌ **tìm ra lỗi** — trộn 500 nhóm đã tải với tổng của 6.780 nhóm |
| 13 | Bấm lại lần hai | đo thời gian | ❌ **tìm ra lỗi** — quét lại 10 phút → sửa thành 3,5s |

**Kết luận lượt test P7:** 10/13 pass, 3 mục tìm ra vấn đề — cả ba đều **chỉ lộ ra khi nhìn ảnh
chụp màn hình**, không mục nào bị test bắt.

**Mục 12 đáng ghi nhất.** Tiêu đề ghi *"500 nhóm trùng lặp · có thể thu hồi 533214.6 MB"*.
Cả hai con số đều **đúng** — nhưng chúng mô tả hai tập khác nhau: 500 là số nhóm đã tải về giao
diện, còn dung lượng là tổng của cả 6.780 nhóm. Ghép lại thành một câu, nó nói sai gấp hơn mười
lần. Không assertion nào bắt được loại lỗi này, vì từng thành phần đều chính xác.

---

### 2026-08-24 — Lượt test sau P8 (chạy trên **bản release**, không phải dev)

Toàn bộ mục 4–16 chạy trên `target/release/mediafinder.exe` đã build bằng `cargo tauri build`,
khởi động bằng `Start-Process` (tách khỏi tiến trình cha) để tránh [BUG-002](./bug.md#bug-002).

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **125/125 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch |
| 3 | Type-check frontend | `npm run check` | ✅ 106 tệp, 0 lỗi, 0 warning |
| 4 | Build bản phát hành | `cargo tauri build` | ✅ exe **9,9 MB** + bộ cài NSIS, 3m23s |
| 5 | Khởi động bản release | đọc log | ✅ nạp cache **117.128 tệp** trong **27 ms** |
| 6 | Đăng ký phím tắt | đọc log | ✅ `phím tắt toàn cục: Ctrl+Alt+Space` |
| 7 | Phím tắt có thật sự chiếm được không | tiến trình khác thử `RegisterHotKey` | ✅ trả `1409` — hệ thống xác nhận |
| 8 | Gọi cửa sổ khi **không** có focus | `keybd_event` + `GetForegroundWindow` | ✅ lên foreground |
| 9 | Bấm lại khi **đang** có focus | như trên | ✅ ẩn đi |
| 10 | Bấm lại khi đang ẩn | như trên | ✅ hiện + focus |
| 11 | Gọi lại khi đang **thu nhỏ** | `SW_MINIMIZE` rồi bấm, đọc `IsIconic` | ✅ 3/3 chu kỳ phục hồi |
| 12 | **Gọi rồi gõ luôn** | bấm phím tắt rồi `SendKeys` ngay | ❌ **tìm ra lỗi** — chữ không vào ô nhập ([BUG-015](./bug.md#bug-015)) |
| 13 | Gọi lại lần hai rồi gõ | gõ `avatar` khi ô đang có `anglerfish` | ✅ sau khi sửa: thay thế, không nối thêm |
| 14 | Tìm kiếm trên bản release | gõ `anglerfish` | ✅ **84 kết quả · 0,5 ms** |
| 15 | Thumbnail trên bản release | nhìn ảnh chụp | ✅ hiện đủ ảnh, độ phân giải, thời lượng, dung lượng |
| 16 | **Mở thư mục chứa tệp** | `Ctrl+Enter`, rồi hỏi Explorer đang chọn gì | ✅ chọn đúng `D:\TÀI NGUYÊN DEEP SEA\anglerfish\img\anglerfish.webp` |
| 17 | **Mở tệp** | `Enter`, rồi đọc tiến trình mới | ✅ `Photos — 'anglerfish.webp'` |
| 18 | Phím tắt bị ứng dụng khác chiếm | chiếm trước rồi mở app | ❌ **tìm ra lỗi** — vẫn mời gọi phím tắt đã mất ([BUG-014](./bug.md#bug-014)) |
| 19 | Ứng dụng có chịu khởi động không khi mất phím tắt | như trên | ✅ mở cửa sổ bình thường, chỉ ghi `WARN` |
| 20 | Kịch bản test có gõ nhầm chỗ không | `GetForegroundWindow` trước mỗi lần gõ | ❌ **tìm ra lỗi** — đã gõ vào VS Code ([BUG-016](./bug.md#bug-016)) |

**Kết luận lượt test P8:** 17/20 pass, 3 mục tìm ra vấn đề — cả ba đều đã sửa hoặc đã có quy tắc
phòng tránh.

**Mục 12 đáng ghi nhất.** Phím tắt đạt mọi tiêu chí đã đặt ra: gọi được, ẩn được, phục hồi được từ
trạng thái thu nhỏ, và log nói nó đăng ký thành công. Nhưng thứ người dùng thật sự cần —
*bấm rồi gõ* — thì hỏng. Không tiêu chí nào trong danh sách nghiệm thu mô tả **hành động tiếp theo**
sau khi cửa sổ hiện ra, nên tất cả đều xanh trong khi tính năng vô dụng.

**Mục 16 và 17 chạy trên bản release là có chủ ý.** Hai tính năng này người dùng nêu là *bắt buộc
phải có*, và cả hai đều gọi COM. Bản debug chạy đúng không bảo đảm bản release cũng vậy, nên cả
hai được kiểm chứng lại bằng cách **hỏi hệ điều hành** — Explorer đang chọn tệp nào, và tiến trình
nào vừa xuất hiện — chứ không tin vào việc lệnh trả về `Ok`.

---

### 2026-08-24 — Lượt test P9 bước 1 (FRN + `rebuild_with`)

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **151/151 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch |
| 3 | FRN đi hết chặng | test tổng hợp `tree.rs` → `Index` | ✅ tệp và thư mục đều giữ đúng FRN |
| 4 | Hai bảng `dirs`/`dir_frns` không lệch nhau | thư mục bị loại giữa chừng | ✅ cùng độ dài, ghép đúng cặp |
| 5 | FRN trùng nhau giữa hai ổ | `C:` và `D:` cùng FRN 100 | ✅ chỉ mục đúng ổ bị ảnh hưởng |
| 6 | Cache cũ bị từ chối đúng cách | chạy app với cache phiên bản 2 | ✅ *"cache thuộc phiên bản 2, phần mềm cần 3 — bấm Quét lại"* |
| 7 | `metadata.bin` sống sót qua đổi schema | đọc log khởi động | ✅ 117.128 mục vẫn nạp được |
| 8 | Xoá tệp | `rebuild_with` | ✅ biến mất, các tệp khác nguyên vẹn |
| 9 | Đổi tên tệp | như trên | ✅ tìm được tên mới, không còn tên cũ, số mục không đổi |
| 10 | Di chuyển tệp | như trên | ✅ giữ nguyên dung lượng và thời gian đã đo |
| 11 | Tệp mới trong thư mục mới lồng nhau | như trên | ✅ dựng cả chuỗi thư mục |
| 12 | Thứ tự thay đổi đến lộn xộn | con trước cha | ✅ vẫn resolve đúng |
| 13 | Đổi tên thư mục | như trên | ✅ toàn bộ thư mục con và tệp đi theo |
| 14 | `D:\Phim` vs `D:\Phim2` | như trên | ✅ thư mục cùng tiền tố **không** bị kéo theo |
| 15 | Xoá thư mục | như trên | ✅ mọi tệp và thư mục con bên dưới biến mất |
| 16 | Tệp ở gốc ổ đĩa | FRN gốc có sequence number | ✅ khớp về cùng một thư mục, không thành mồ côi |
| 17 | Thay đổi dưới thư mục chưa từng index | `C:\Windows` | ✅ bỏ qua, đếm vào `unresolved`, không panic |
| 18 | Tạo rồi xoá trong cùng một lô | như trên | ✅ không xuất hiện lần nào |
| 19 | Đổi tên ba lần trong cùng một lô | như trên | ✅ chỉ còn tên cuối |
| 20 | Index mới có tìm kiếm được không | `search("tieng viet")` | ✅ khớp `Tiếng Việt.mp4` vừa thêm |
| 21 | **Chi phí dựng lại thật** | `cargo bench -- rebuild_with` | ❌ **tìm ra lỗi** — 100 thay đổi nhanh hơn 0 thay đổi 7 lần ([BUG-017](./bug.md#bug-017)) |
| 22 | Chi phí sau khi sửa | như trên | ✅ 160 ms / 170 ms / 170 ms cho 0 / 100 / 10.000 thay đổi |

**Kết luận:** 21/22 pass, 1 mục tìm ra lỗi — và mục đó là lỗi **nặng nhất** trong cả bước này.

**Mục 21 đáng ghi nhất.** Hai mươi test ở trên đều pass, kể cả những test cố tình nhắm vào định
danh (mục 5 kiểm tra FRN trùng giữa hai ổ). Không test nào bắt được việc `frn = 0` khớp mọi mục,
vì mọi test đều dựng dữ liệu với FRN mà tôi **tự đặt là hợp lệ**. Bench thì không assert gì cả —
nó chỉ in ra một con số, và con số đó vô lý.

**Việc còn lại của giai đoạn 1:** bộ đọc `FSCTL_READ_USN_JOURNAL` để dịch bản ghi thô thành
`Change`, cùng với phát hiện journal bị tạo lại và journal cuộn vòng.

---

### 2026-08-24 — Lượt test P9 bước 3 (bộ đọc USN journal)

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **168/168 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch |
| 3 | Tệp được tạo | bản ghi tự dựng | ✅ thành `Present` |
| 4 | Tệp bị xoá | như trên | ✅ thành `Gone` |
| 5 | Đổi tên: chỉ nửa "tên mới" được dùng | như trên | ✅ nửa "tên cũ" bị bỏ, đếm vào `rename_halves` |
| 6 | Lô kết thúc giữa hai nửa của một lần đổi tên | như trên | ✅ không phát gì — không đổi tên ngược |
| 7 | `FILE_DELETE` cộng `CLOSE` trong cùng bản ghi | như trên | ✅ xoá thắng |
| 8 | Thư mục được đánh dấu đúng | như trên | ✅ `is_dir: true` |
| 9 | Đổi `.mp4` thành `.txt` | như trên | ✅ vẫn báo lên, `rebuild_with` gỡ mục cũ |
| 10 | Nhiều bản ghi trong một buffer | như trên | ✅ đọc hết |
| 11 | Bản ghi khai độ dài bằng 0 | như trên | ✅ dừng, không lặp vô hạn |
| 12 | Tên chạy quá cuối bản ghi | như trên | ✅ từ chối, không đọc tràn |
| 13 | Bản ghi phiên bản 3 | như trên | ✅ bỏ qua, không đoán |
| 14 | Đi hết chặng: byte journal → index mới | bản ghi tự dựng + `rebuild_with` | ✅ đổi tên và thêm tệp đều đúng |
| 15 | Journal cuộn vòng (`ERROR_JOURNAL_ENTRY_DELETED`) | dựng lỗi Win32 thật | ✅ hiểu là "cần quét lại", không phải lỗi |
| 16 | Journal tắt / đang bị xoá | như trên | ✅ nhận diện đúng |
| 17 | `journal_id` sai (`ERROR_INVALID_PARAMETER`) | như trên | ✅ hiểu là journal đã bị tạo lại |
| 18 | Lỗi thật (`ERROR_ACCESS_DENIED`) | như trên | ✅ **không** bị nuốt thành "quét lại" |
| 19 | Thông báo có nói người dùng cần làm gì không | như trên | ✅ cả ba đều nêu tên ổ và hành động |
| 20 | **Chạy trên ổ NTFS thật** | `--watch C` với quyền Admin | ⚠️ **chưa chạy được** — UAC không được chấp nhận ([CHECK-003](./check.md#check-003)) |

**Kết luận:** 19/20 pass, 1 mục **chưa đo được** chứ không phải hỏng.

**Mục 20 là mục quan trọng nhất và nó chưa chạy.** Mười chín test kia chứng minh mã đọc đúng cái
tôi *nghĩ* là layout của `USN_RECORD_V2` — vì chính tôi dựng ra những bản ghi đó theo tài liệu.
Chúng không chứng minh Windows sinh ra bản ghi như thế. Đúng khoảng cách đã tạo ra
[BUG-013](./bug.md#bug-013): mã đúng theo cách nhìn của nó, dữ liệu đúng theo cách nhìn của nó, và
hai bên không gặp nhau ở đâu cả.

Vì vậy hai ô trong `PROGRESS.md` giữ dấu `[~]` — đã viết, chưa kiểm chứng — chứ không phải `[x]`.

**Mục 18 nhỏ nhưng đáng giữ.** Ba mã lỗi được dịch thành "cần quét lại". Nếu `ERROR_ACCESS_DENIED`
lọt vào nhóm đó thì một tiến trình thiếu quyền sẽ được bảo đi quét lại — mà quét lại cũng cần đúng
quyền ấy. Người dùng rơi vào vòng lặp không có lối ra, và không có thông báo nào nói vì sao.


---

### 2026-08-24 — Lượt test P9 bước 4 (cập nhật nhanh, chạy thật trên máy người dùng)

Toàn bộ chạy trên ổ NTFS thật với quyền Administrator, qua chính USN journal của máy.

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **170/170 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch |
| 3 | Đọc journal có cần quyền Admin không | thử 4 mức quyền, tiến trình không elevate | ✅ có — `ERROR_INVALID_FUNCTION` trên handle quyền thấp ([CHECK-004](./check.md#check-004)) |
| 4 | Đọc journal trên ổ thật | tự kiểm tra cuối lần quét | ✅ **570 thay đổi trong 1 ms**, FRN/tên đều đúng |
| 5 | Ổ không có thay đổi | như trên, ổ D: | ✅ 0 thay đổi, con trỏ giữ nguyên, không treo |
| 6 | FRN vào được cache thật | đọc cache | ✅ **46.700/46.700** mục đều có FRN |
| 7 | Số tệp tụt 60% sau khi đổi schema | đếm độc lập cả hai ổ | ✅ **không hồi quy** — bộ quét lệch 1 tệp trên D: ([CHECK-005](./check.md#check-005)) |
| 8 | Tệp mới trong thư mục đã có | tạo `.mp4` rồi chạy `--index` | ✅ vào index, đúng dung lượng 4096 byte |
| 9 | Tệp mới trong thư mục **vừa tạo** | thư mục mới + tệp trong đó | ✅ dựng được cả chuỗi thư mục |
| 10 | Đổi tên tệp | đổi trước khi index biết tới | ✅ vào index dưới tên mới |
| 11 | Tệp không phải media | tạo `.txt` | ✅ không vào index |
| 12 | Xoá tệp | xoá cả hai rồi chạy lại | ✅ **-2 tệp**, biến khỏi index |
| 13 | Xoá thư mục | xoá thư mục thử | ✅ **-1 thư mục**, 3.196 → 3.195 |
| 14 | **Tốc độ cập nhật nhanh** | đo | ✅ **0,43–0,45 s** so với **13,2 s** quét đầy đủ |
| 15 | Ổ mạng có được báo không | liệt kê ổ đĩa | ✅ cả ba NAS đều được nêu tên máy chủ ([BUG-018](./bug.md#bug-018)) |
| 16 | Tệp trong thư mục cũ chưa từng có media | đọc `unresolved` trong log | ⚠️ **giới hạn đã biết** ([RISK-003](./risk.md#risk-003)) |

**Kết luận:** 15/16 pass, 1 giới hạn đã biết và đã ghi lại.

**Mục 7 đáng ghi nhất, và nó suýt đi sai đường.** Lần đếm độc lập đầu tiên trên ổ C: ra 1.635 so
với 583 của bộ quét — gấp gần ba lần, đủ để kết luận là có hồi quy nặng. Sai nằm ở **công cụ kiểm
chứng**: `GetDirectories` đi xuyên junction, nên nó chui vào `ProgramData` qua
`C:\Users\All Users` và đếm đúng những tệp mà bộ quét loại đúng. Bỏ qua reparse point thì
con số về 521, và phần chênh còn lại đúng bằng 62 thư mục mà phép đếm không có quyền đọc.

Bài học không chỉ nằm ở con số: **công cụ dùng để kiểm chứng cũng cần được kiểm chứng.** Nếu tin
ngay lần đếm đầu, tôi đã đi sửa một bộ quét vốn không hỏng.

**Mục 14.** Nút "Quét lại" trong giao diện gọi cùng một `--index`, nên nó cũng đi đường cập nhật
nhanh — từ 13 giây xuống dưới nửa giây, vẫn chỉ một lần UAC.


---

### 2026-08-24 — Lượt test RISK-003 (tra thư mục cha qua NTFS)

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **179/179 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch |
| 3 | Tệp đầu tiên trong thư mục cũ rỗng | `DirLookup` giả | ✅ không có lookup thì mất, có lookup thì tìm ra |
| 4 | Thư mục bị hệ thống từ chối | như trên | ✅ đếm vào `excluded`, **không** vào `unresolved` |
| 5 | Thư mục không tra được tên | như trên | ✅ vẫn đếm vào `unresolved` |
| 6 | 50 tệp cùng một thư mục mới | đếm số lần gọi lookup | ✅ **1 lần hỏi**, không phải 50 |
| 7 | Không hỏi về thư mục đã biết | lookup panic khi bị gọi | ✅ không bị gọi |
| 8 | Thư mục mới nằm dưới thư mục lạ | journal + lookup | ✅ resolve qua cả hai |
| 9 | Luật loại trừ áp cho đường dẫn ghép sẵn | `excludes_path` | ✅ `C:\Windows\Media`, `AppData`, `.recycle_bin`, `node_modules` đều bị loại |
| 10 | Bỏ tiền tố `\\?\` | unit test | ✅ `\\?\D:\Phim` → `D:\Phim` |
| 11 | **Kịch bản thật trên máy** | thư mục có trước lần quét đầy đủ, sau đó mới bỏ `.mp4` vào | ✅ **+1 tệp, 1 lần hỏi hệ thống tệp**, 0,60s |
| 12 | Xoá thư mục vừa tra được | xoá rồi cập nhật nhanh | ✅ -1 tệp, -1 thư mục, về đúng 46.700 |

**Kết luận:** 12/12 pass.

**Mục 9 là mục dễ bỏ sót nhất.** Đường dẫn lấy thẳng từ NTFS **đi vòng qua** `tree.rs` — nơi vốn
lọc từng thành phần trong lúc đi ngược chuỗi cha. Nếu không áp lại luật loại trừ ở đây thì mọi tệp
media mới trong `C:\Windows` hay `AppData` sẽ chui thẳng vào index, và bộ lọc mà cả dự án dựa vào
sẽ có một lỗ hổng rộng đúng bằng tính năng vừa thêm. Không có test nào **hiện có** bắt được điều
đó, vì đường dẫn mới này chưa từng tồn tại.

**Một lỗi trình bày tự tôi gây ra rồi tự bắt được.** Dòng log cảnh báo in ra một dãy dấu cách giữa
câu: ký tự nối dòng `\` trong chuỗi Rust bị Python nuốt mất lúc tôi ghi file. Chỉ lộ ra khi **đọc
dòng log thật**, không assertion nào chạm tới. Nhân đó tách luôn cảnh báo `unresolved` ra khỏi dòng
`excluded` — gộp hai con số vào một câu chính là cách giấu con số quan trọng sau con số bình thường.
