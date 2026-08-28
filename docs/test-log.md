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


---

### 2026-08-25 — Lượt test P10 (quét ổ mạng / NAS)

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Unit test toàn bộ | `cargo test` | ✅ **186/186 pass** |
| 2 | Chất lượng code | `cargo clippy --all-targets` | ✅ sạch |
| 3 | Type-check frontend | `npm run check` | ✅ 106 tệp, 0 lỗi |
| 4 | Tiến trình elevated có thấy ổ mạng không | so hai mức quyền | ✅ **không** — quyết định cả kiến trúc ([CHECK-007](./check.md#check-007)) |
| 5 | Tìm media, bỏ qua thứ khác | thư mục tạm | ✅ chỉ `.mp4`/`.jpg`, không lấy `.txt`/`.exe` |
| 6 | Đi sâu nhiều tầng, đường dẫn đúng | thư mục tạm | ✅ `a\b\c\sâu.mp4` |
| 7 | Bỏ qua thư mục cấm và mọi thứ dưới nó | `node_modules`, `.recycle_bin` | ✅ |
| 8 | Thư mục không đọc được không làm dừng cả lượt | thư mục không tồn tại | ✅ đánh dấu rồi đi tiếp |
| 9 | Cây rỗng | thư mục tạm | ✅ trả kết quả rỗng, không lỗi |
| 10 | Mục ổ mạng không có FRN | unit test | ✅ toàn bộ bằng 0 |
| 11 | Dung lượng lấy ngay trong lúc duyệt | unit test | ✅ song song với danh sách tệp, đúng 4096 byte |
| 12 | **Duyệt NAS thật** | ba ổ, 37,9 TB | ✅ **313.945 tệp media / 272,6 s** |
| 13 | Huỷ giữa chừng | bấm dừng sau 200 thư mục | ✅ dừng trong **0,1 s** |
| 14 | **Hợp nhất vào cache thật** | quét NAS rồi đọc lại cache | ✅ 46.700 → **360.646** mục |
| 15 | Mục ổ cục bộ có bị đụng không | so từng ổ trước/sau | ✅ C: 583 và D: 46.117 **không đổi một con số** |
| 16 | Mốc journal | kiểm tra sau khi hợp nhất | ✅ chỉ C: và D: có; ổ mạng **không** được cấp |
| 17 | Nạp cache 47,4 MB lúc khởi động | bản release | ✅ **128 ms** |
| 18 | Tìm kiếm trên 360.646 mục | gõ `shamrock` | ✅ **21 kết quả · 2,0 ms**, toàn bộ từ ổ F: |
| 19 | Dung lượng hiện đúng cho tệp NAS | nhìn ảnh chụp | ✅ 79 MB … 405,7 MB |
| 20 | Nút "+ ổ mạng" chỉ hiện khi có ổ mạng | nhìn ảnh chụp | ✅ |
| 21 | Enrichment với 313.946 tệp NAS | đo tốc độ ghi `metadata.bin` | ❌ **tìm ra vấn đề** — 11 tệp/giây → **7,8 giờ** liên tục hành NAS |

**Kết luận:** 20/21 pass, 1 vấn đề tìm ra và đã xử lý.

**Mục 21 là mục quan trọng nhất, và không test nào nhắm vào nó.** Mọi thứ đều "chạy đúng": quét
xong, hợp nhất xong, tìm kiếm 2 ms. Nhưng ngay sau khi khởi động, tiến trình enrichment nền lặng lẽ
bắt đầu **mở từng tệp một qua mạng** để đọc độ phân giải — 313.946 tệp ở tốc độ đo được là 11
tệp/giây, tức **7,8 giờ** hành NAS liên tục. Không lỗi, không cảnh báo, chỉ là một tiến trình nền
chăm chỉ làm việc sai.

Nó lộ ra vì tôi đọc **một dòng log không liên quan tới thứ đang test**:
`enrichment: 46700/360646 mục đã có sẵn`. Con số 46.700 đúng bằng số tệp ổ cục bộ — nghĩa là 313.946
tệp còn lại sắp được xử lý, mà chúng đều nằm trên mạng.

**Đã sửa:** enrichment bỏ qua ổ mạng. Đánh đổi được nêu thẳng trong log — lọc theo độ phân giải và
thời lượng không áp dụng cho tệp NAS, còn lọc theo **dung lượng vẫn được**, vì dung lượng đã lấy
miễn phí trong lúc duyệt. Bộ đếm tiến độ cũng trừ đi phần bỏ qua, nếu không thanh tiến độ sẽ đứng
mãi ở một con số không bao giờ tới đích.

**Mục 15 là thứ suýt gây mất dữ liệu.** Tiến trình elevated không nhìn thấy ổ mạng, nên nó dựng lại
chỉ mục **không có ổ Z: nào trong đó**. Nếu không có quy tắc "giữ nguyên mục của ổ không quét", thì
quét NAS 4,5 phút rồi bấm "Quét lại" là mất sạch — và mất một cách hoàn toàn im lặng, vì đứng từ
phía tiến trình đó thì nó đã làm đúng mọi thứ.

---

### 2026-08-25 — Lượt test CONF-005 (`cargo fmt`)

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Cấu hình nào khớp mã nguồn nhất | đo 4 cấu hình | ❌ **giả thuyết của tôi sai** — nới ra làm tệ gấp đôi |
| 2 | `cargo fmt` có đụng vào chuỗi không | `cargo test` sau khi format | ✅ **186/186 pass**, y nguyên |
| 3 | Có đụng vào nội dung chú thích không | đọc diff | ✅ chỉ thụt lại, không viết lại |
| 4 | Có đụng tệp ngoài `.rs` không | `git diff --name-only` | ✅ 23 tệp, toàn `.rs` |
| 5 | Chất lượng code sau khi format | `cargo clippy --all-targets` | ✅ sạch |
| 6 | Chỗ nào đọc kém hẳn đi | đọc từng hunk lớn | ❌ **4 chỗ** — bảng dữ liệu bị nổ tung |
| 7 | Hai bảng giữ được bằng `#[rustfmt::skip]` | đọc lại tệp | ✅ nguyên vẹn |
| 8 | `cargo fmt --check` sau khi xong | | ✅ **0 điểm lệch** |

**Kết luận:** 6/8 pass, 2 mục tìm ra vấn đề — cả hai đều đã xử lý.

**Mục 1: tôi đã ghi một khuyến nghị sai vào tài liệu, và số đo lật lại nó.** CONF-005 viết từ P8
nói *"phần lớn điểm lệch là do độ rộng dòng… nới `max_width` thì diff nhỏ hơn nhiều"*. Đó là suy
đoán. Đem đo:

| Cấu hình | Điểm lệch |
|---|---|
| Mặc định | **81** |
| `use_small_heuristics = "Max"` | 157 |
| `Max` + `max_width = 110` | 199 |

Nới ra làm **tệ gấp đôi**, vì rustfmt khi đó quay sang *nối* những dòng đã tự tách. Ba ví dụ đầu
tiên nó đưa ra đều là dòng **dưới 100 ký tự** — nên `max_width` chưa bao giờ là thủ phạm.

**Mục 6: định dạng tự động làm hỏng bảng.** `is_word_boundary` từ 2 dòng gọn thành 21 dòng, mỗi
ký tự một dòng; `mod pkey` từ 5 dòng thẳng hàng thành 20. Cả hai là bảng dữ liệu — `pkey` chỉ khác
nhau ở GUID nào và chỉ số nào, và điều đó chỉ nhìn ra được khi chúng thẳng hàng. Giữ lại bằng
`#[rustfmt::skip]`, đúng hai chỗ trên toàn dự án, mỗi chỗ kèm lý do.

Điều này chỉ thấy được bằng cách **đọc từng hunk lớn**. `cargo fmt --check` không phân biệt "khác"
với "tệ hơn"; nó chỉ nói là khác.

---

### 2026-08-25 — Cài đặt, tự khởi động, tự cập nhật

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Vòng kiểm tra | test · clippy · fmt · npm | ✅ 186 pass, sạch cả bốn |
| 2 | Bộ cài chạy được | `/S` (im lặng), không quyền Admin | ✅ cài vào `%LOCALAPPDATA%\MediaFinder` |
| 3 | Có mục trong Start Menu | đọc thư mục Programs | ✅ `MediaFinder.lnk` |
| 4 | Mở thường vẫn hiện cửa sổ | `IsWindowVisible` | ✅ `visible=True` |
| 5 | `--minimized` thì ẩn | như trên | ✅ `visible=False`, log ghi *"khởi động ẩn"* |
| 6 | Bấm phím tắt khi đang ẩn | `keybd_event` + đọc lại | ✅ `visible=True` |
| 7 | **Bộ theo dõi cache** | đổi `mtime` của `index.bin` | ✅ tự nạp lại sau **4 giây** |
| 8 | Trình gỡ cài đặt có xoá dữ liệu không | đọc `installer.nsi` do Tauri sinh | ✅ `RMDir` **không** `/r` → dữ liệu sống sót ([RISK-004](./risk.md#risk-004)) |
| 9 | Tạo Scheduled Task | `Register-ScheduledTask` elevated | ⚠️ **chưa làm được** — UAC bị từ chối |

**Kết luận:** 8/9 pass, 1 mục chưa chạy được vì cần một lần UAC.

**Mục 7 là mắt xích không có thì cả tính năng vô nghĩa.** Tác vụ nền ghi cache lúc đăng nhập, còn
cửa sổ có thể đã nạp cache từ trước đó vài giây — hoặc đã mở suốt mấy ngày. Không có bộ theo dõi
thì bản cập nhật vẫn diễn ra nhưng **không ai nhìn thấy**, cho tới lần khởi động lại. Theo dõi
`mtime` chứ không theo dõi nội dung: cache được ghi ra tệp tạm rồi đổi tên vào chỗ, nên dấu thời
gian chỉ nhích khi đã có một bản hoàn chỉnh nằm đó.

**Mục 4 đáng giữ vì nó suýt thành lỗi nặng.** Để `--minimized` không nháy cửa sổ mỗi lần đăng nhập,
`visible` trong `tauri.conf.json` đổi thành `false` và cửa sổ được hiện bằng mã. Nếu nhánh hiện ấy
sai thì ứng dụng **vĩnh viễn vô hình** — mở lên không thấy gì, không có cửa sổ để mà đóng. Nên nó
được thử riêng, trước khi thử `--minimized`.

**Một điểm chưa giải thích được.** Cache hiện tại được dựng lúc **09:06:44** với 360.649 mục
(313.946 ổ mạng + 46.703 ổ cục bộ), trong khi lượt hợp nhất tôi chủ động chạy kết thúc ở 360.646.
`progress.json` cho thấy một lượt quét ổ mạng đã hoàn tất đúng thời điểm đó, nhưng **tôi không quy
được nó về lệnh nào mình đã chạy**. Dữ liệu đã kiểm chứng là nhất quán và tìm kiếm được; ghi lại
đây vì một chỉ mục tự thay đổi là thứ không nên bỏ qua, kể cả khi kết quả trông đúng.

---

### 2026-08-25 — Tác vụ tự cập nhật, và lượt test đầu tiên đi qua chính nút bấm

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Vòng kiểm tra | test · clippy · fmt · npm | ✅ 186 pass, sạch cả bốn |
| 2 | Tạo Scheduled Task | `Register-ScheduledTask` elevated | ✅ `RunLevel = Highest`, trễ 1 phút sau đăng nhập |
| 3 | **Tác vụ chạy có hiện UAC không** | `Start-ScheduledTask` rồi quan sát | ✅ **không** — đúng mục đích của cả cách làm này |
| 4 | Tác vụ có cập nhật thật không | đọc `built_at` trong cache | ✅ 09:06:44 → 10:25:00, mã trả về 0 |
| 5 | **Bấm nút "Quét lại" trong giao diện** | UI Automation | ❌ **tìm ra lỗi** — báo *"kết thúc bất thường"* dù thành công ([BUG-019](./bug.md#bug-019)) |
| 6 | Sau khi sửa: bấm lại | như trên | ✅ không còn lỗi giả, thanh trạng thái đổi sang *"quét lúc 10:32:43"* |
| 7 | Quét ổ cục bộ có xoá mất NAS không | đếm lại sau khi bấm nút | ✅ **360.650 tệp** — 313.946 mục NAS còn nguyên |
| 8 | Luồng hai pha của nút "+ ổ mạng" | đọc mã | ❌ **tìm ra lỗi** — pha một giương cờ kết thúc sớm |

**Kết luận:** 6/8 pass, 2 lỗi tìm ra và đã sửa. Cả hai cùng một gốc.

**Mục 5 là lần đầu tiên có ai bấm nút đó.** Suốt P9 tôi chỉ chạy `--index` từ dòng lệnh. Mọi phép
đo đều đúng — 0,45 giây, đúng số tệp, cache ghi xong — nhưng **không phép đo nào đi qua con đường
người dùng đi**. Chỉ cần một lần bấm nút là lỗi lộ ra ngay.

**Mục 3 là thứ biện minh cho cả cách làm.** Nếu tác vụ vẫn hiện UAC thì nó chẳng hơn gì việc tự
bấm nút, và toàn bộ lý do dùng Task Scheduler sụp đổ. Đã chạy thử và xác nhận: không có lời nhắc
nào.

**Một ghi chú về công cụ test.** Kịch bản bấm nút đầu tiên báo *"không thấy nút nào"*. Sai ở kịch
bản: nó bấm phím tắt để gọi cửa sổ **trước**, mà cửa sổ đang hiện và đang focus — nên chính phím
tắt đó **ẩn nó đi**, và UIA không còn gì để tìm. Cùng họ với [BUG-016](./bug.md#bug-016): công cụ
kiểm chứng cũng cần được kiểm chứng.


---

### 2026-08-25 — Lượt test P11 (chạy nền ở khay hệ thống)

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Vòng kiểm tra | test · clippy · fmt · npm | ✅ 186 pass, sạch cả bốn |
| 2 | Khay hệ thống dựng được | đọc log | ✅ *"khay hệ thống: sẵn sàng"* |
| 3 | Bấm X thì ẩn hay thoát | gửi `WM_CLOSE` | ✅ ẩn, **tiến trình vẫn sống** |
| 4 | Gọi lại sau khi ẩn | phím tắt | ✅ cửa sổ hiện lại |
| 5 | Biểu tượng có thật trong khay không | UI Automation | ✅ trong vùng ẩn, đúng tooltip |
| 6 | Chuột phải → Thoát | UIA tìm biểu tượng, chuột phải, bàn phím chọn | ✅ **tiến trình kết thúc** |
| 7 | Có cản trở tắt máy không | gửi `WM_QUERYENDSESSION` | ✅ trả về **1** — không cản trở |
| 8 | Người dùng có biết X không phải thoát không | nhìn ảnh chụp | ✅ có một dòng nói rõ trên màn hình trống |

**Kết luận:** 8/8 pass.

**Mục 7 là mục dễ bỏ qua nhất và hậu quả thì chỉ hiện ra lúc tắt máy.** Một ứng dụng chặn sự kiện
đóng cửa sổ có thể chặn nhầm luôn tín hiệu kết thúc phiên, khiến Windows treo ở màn hình
*"ứng dụng này đang ngăn tắt máy"*. Không cần tắt máy thật để biết: gửi đúng thông điệp Windows
dùng để hỏi (`WM_QUERYENDSESSION`) và đọc câu trả lời.

**Mục 6 phải đi đường vòng.** UIA **không đọc được** menu khay của Tauri — nó là menu Win32 gốc
(lớp `#32768`), và khi hỏi UIA thì thứ trả về lại là thanh menu của VS Code ở cửa sổ khác. Cách làm
sau cùng: xác nhận đúng một cửa sổ `#32768` vừa hiện ra, rồi điều khiển bằng bàn phím — menu gốc
luôn nhận phím mũi tên. Ghi lại vì lần sau gặp menu gốc sẽ vấp đúng chỗ này.

**Hai lần đầu kịch bản chạy sai, cả hai đều là lỗi của kịch bản.** Lần một: nó bấm nút "Show Hidden
Icons" trong khi vùng ẩn **đang mở**, mà Invoke là phép bật/tắt nên nó đóng lại. Lần hai: chuột
phải rơi vào toạ độ cũ sau khi flyout đã đóng. Cùng bài học với [BUG-016](./bug.md#bug-016) — thao
tác ở cấp hệ thống không tự biết nó đang nhắm vào cái gì, nên phải xác nhận trạng thái ngay trước
mỗi bước, và gộp cả chuỗi vào một tiến trình.

---

### 2026-08-25 — Tệp vừa tải về có được tìm thấy không

Câu hỏi của người dùng: tải một video từ web về ổ thì lúc tải xong có tìm được ngay không.
Trả lời: **không**, và việc đo đã chỉ ra hai chỗ cần sửa trước khi làm cho nó gần "ngay".

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Vòng kiểm tra | test · clippy · fmt · npm | ✅ 186 pass, sạch cả bốn |
| 2 | Chạy khi **không có gì mới** | chạy tác vụ hai lần liên tiếp | ✅ sau khi sửa: cache **không** bị ghi lại |
| 3 | Chi phí một lần chạy không có gì mới | đo 3 lần | ✅ **1,2–1,8 giây**, không ghi đĩa |
| 4 | Tạo một `.mp4` 6 MB rồi chạy | như "tải về" | ✅ cache ghi lại sau **7 giây**, tệp có trong index đúng dung lượng |
| 5 | **Tiến trình quyền thường kích hoạt được tác vụ elevated không** | `schtasks /Run` | ✅ **được**, mã trả về 0 |
| 6 | Nút "Quét lại" qua tác vụ | bấm nút qua UIA | ✅ tác vụ chạy lúc 11:15:32 mã 0 — **không có UAC** |
| 7 | Kết quả trên giao diện | nhìn ảnh chụp | ✅ *"quét lúc 11:15:32"*, không lỗi |

**Kết luận:** 7/7 pass sau khi sửa hai chỗ.

**Mục 2 là chỗ suýt gây hại lâu dài.** Khi journal không có gì đáng áp, mã cũ vẫn ghi lại **toàn
bộ cache 47 MB** chỉ để đẩy con trỏ. Chạy mỗi 5 phút sẽ thành **13,5 GB ghi xuống SSD mỗi ngày cho
việc không làm gì**. Nay chỉ ghi khi thực sự có mục thay đổi; con trỏ đứng yên và lần sau đọc lại
cùng đoạn journal — đọc cả vòng journal chỉ mất 0,2 giây, rẻ hơn hẳn cái ghi mà nó tránh được.

**Mục 5 mở ra thứ tôi không ngờ.** Nút "Quét lại" trước đây luôn hiện UAC vì nó tự khởi chạy tiến
trình elevated. Nhưng **kích hoạt** một tác vụ đã lên lịch thì không cần quyền gì — chính tác vụ
mới mang quyền. Nên nút nay gọi tác vụ trước, chỉ khi không có tác vụ mới quay về cách cũ. Kết quả:
thao tác làm mới thường ngày **không còn hỏi quyền một lần nào**.

Không có handle tiến trình để chờ (tác vụ chạy ở phiên riêng), nên hoàn tất được đọc từ chính
`progress.json` — thứ vừa được sửa ở [BUG-019](./bug.md#bug-019) để đường cập nhật nhanh cũng phải
báo xong. Hai việc tưởng rời nhau hoá ra là một.

**Lịch cuối cùng (theo lựa chọn của chủ máy):** khi đăng nhập (trễ 1 phút) và **13:00 hằng ngày**
— tức 1–2 lần mỗi ngày.

> **Cập nhật 28/08/2026 (P29).** Đoạn trên đã bị P22 thay bằng `Repetition PT15M` (96 lượt/ngày) mà
> **không hỏi lại chủ dự án** — một quyết định của họ bị lật im lặng. Chỗ này lộ ra khi họ nhắc lại
> "1 ngày chỉ được quét 2 lần". Đã đưa số liệu ra để chốt lại: một lượt PT15M là **đọc nhật ký USN**
> (0,2–2 giây), không phải quét đầy đủ, nhưng khi có thay đổi thật thì nó ghi lại trọn **46 MB**
> cache — quan sát ngày 28/8 trên máy này: ghi lúc 16:00, 16:15, 16:21, 18:00, 20:45, tức vài lần
> mỗi giờ chứ không phải mọi tick. Đổi lại, khoảng mù của ổ cục bộ là 15 phút thay vì ~12 giờ.
> **Chủ dự án xem số liệu rồi chốt: giữ PT15M.** Lịch hiện hành là 96 lượt/ngày, và đó là lựa chọn
> có chủ đích — đừng ai "sửa lại cho đúng tài liệu cũ". Đã kiểm chứng sau khi đổi lịch: hành động và quyền của tác vụ vẫn nguyên
(`--index`, `RunLevel = Highest`), chạy thử từ tiến trình quyền thường trả về mã 0, lần kế tiếp
được lên lịch đúng 13:00.

Tần suất thấp là lựa chọn hợp lý **vì** nút "Quét lại" không còn hỏi quyền: tải xong một tệp thì
bấm nút là có sau vài giây. Nếu nút vẫn hỏi UAC như trước thì lựa chọn này sẽ khó chịu — hai thay
đổi đó phụ thuộc nhau.


---

### 2026-08-25 — Lượt test P12 (kéo tệp ra ngoài)

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Vòng kiểm tra | test · clippy · fmt · npm | ✅ 186 pass, sạch cả bốn |
| 2 | Crate có thật dùng `CF_HDROP` không | đọc mã nguồn crate | ✅ `IDataObject` + `DROPFILES` + `DoDragDrop` |
| 3 | `DoDragDrop` có chặn luồng gọi không | đọc mã nguồn crate | ✅ có — nên phải chạy trên luồng giao diện |
| 4 | Cái giá của phụ thuộc mới | đo build và kích thước | ✅ 25,5 s · +456 KB ([CONF-006](./config.md#conf-006)) |
| 5 | **Thả thật vào ứng dụng khác** | cửa sổ nhận thả tự dựng | ✅ **`CF_HDROP: CÓ`**, đúng đường dẫn và dung lượng |
| 6 | Các định dạng kèm theo | như trên | ✅ Shell IDList Array, FileNameW, FileContents, FileGroupDescriptorW |

**Kết luận:** 6/6 pass.

**Mục 5 phải làm lại năm lần, và cả năm lỗi đều nằm ở kịch bản test.** Đáng ghi vì nó lặp lại đúng
[BUG-016](./bug.md#bug-016) theo một cách mới:

1. Explorer mở lên chiếm foreground, phím tắt không gọi được cửa sổ lên.
2. Hai cửa sổ chồng nhau nên điểm bắt đầu kéo rơi vào nhầm cửa sổ.
3. Đặt vị trí tường minh rồi, nhưng điểm thả vẫn báo sai.
4. **Kiểm tra theo tên lớp cửa sổ là sai từ gốc:** VS Code và WebView2 **dùng chung** lớp
   `Chrome_RenderWidgetHostHWND`, nên điều kiện `-like 'Chrome*'` cho qua nhầm cửa sổ. Chuyển sang
   kiểm theo **PID** thì lộ ra ngay: điểm thả thuộc tiến trình `Code`, không phải Explorer.
5. VS Code đang toàn màn hình 1920×1032 và nằm trên tất cả; giành z-order với nó là cuộc chiến
   không cần thiết.

Cách giải sau cùng tốt hơn cách ban đầu: **tự dựng cửa sổ nhận thả**. Nó `TopMost` nên không phải
tranh z-order, và quan trọng hơn — nó ghi lại **đúng những định dạng dữ liệu nhận được**. Explorer
chỉ trả lời được "tệp có sang không"; câu hỏi thật là "tệp đến dưới dạng `CF_HDROP` hay không", và
chỉ cửa sổ tự dựng mới trả lời được.

**Bài học lặp lại lần thứ ba:** nhận diện cửa sổ theo tên lớp là không đủ. Lần này là VS Code đội
lốt WebView2; [BUG-001](./bug.md#bug-001) là cửa sổ ẩn của tao đội lốt cửa sổ chính. PID và handle
thì không nói dối.

### 2026-08-25 — Lượt test P13 (chọn nhiều, kéo nhiều, sắp theo thời gian)

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Vòng kiểm tra | test · clippy · fmt · npm | ✅ **196 pass**, sạch cả bốn |
| 2 | Sắp theo thời gian không bị cắt trước khi sắp | unit test | ✅ `newest_order_still_ranks_by_time_when_the_limit_cuts_the_list` |
| 3 | Kéo tệp NAS **không** làm sập ứng dụng | kéo thật, tệp trên F: | ✅ 1 tệp sang đúng, tiến trình còn sống |
| 4 | Kéo nhiều tệp cùng lúc | Ctrl+click 3 hàng rồi kéo | ✅ đủ 3 tệp, đúng dung lượng |
| 5 | Trộn ổ cục bộ với ổ mạng trong một lượt kéo | 2 tệp D: + 1 tệp F: | ✅ đủ 3 tệp — **ba lần chạy liên tiếp** |
| 6 | Đường dẫn có dấu tiếng Việt | `D:\TÀI NGUYÊN DEEP SEA\Sinh vật phù du\` | ✅ sang đúng, không hỏng ký tự |
| 7 | Shell dựng được data object cho đường dẫn mạng | test Rust, không cần chuột | ✅ `the_shell_builds_a_data_object_for_a_file_on_a_network_drive` |
| 8 | Thời gian dựng data object | test Rust, đo 2 lần mỗi ca | ✅ cục bộ 9,4 → 0,2 ms · ổ mạng **4,3 → 0,2 ms** |
| 9 | Chọn dải bằng Shift+click | click hàng 0, Shift+click hàng 2 | ✅ đúng 3 tệp |
| 10 | Bỏ crate rồi graph có sạch không | `cargo tree --duplicates` | ✅ **0 dòng `windows`** trùng, exe 10,14 MB |

**Kết luận:** 10/10 pass. Tìm ra [BUG-020](./bug.md#bug-020) 🔴 — mục 3 chính là ca đã làm sập
ứng dụng hai lần trước khi sửa.

#### Cách tìm ra BUG-020, và vì sao P12 không thấy

Không phải do đọc code. Đang **kiểm chứng tính năng chọn nhiều** thì ứng dụng biến mất giữa chừng,
hai lần liên tiếp. Console để lại đúng một dấu vết: `panic in a function that cannot unwind`.

Tách nguyên nhân bằng một test Rust chứ không đoán — hỏi shell trực tiếp:

```
gốc:        F:\132 mốc  168 commit, từ 2026-07-01.txt   → shell nhận: true
chuẩn hoá:  \?\UNC\192.168.1.214\f\132 mốc  ...        → shell nhận: false
```

Lượt test P12 pass 6/6 và mục quan trọng nhất ("thả thật vào ứng dụng khác") pass thật. Nó chỉ dùng
tệp tự tạo trong `C:\Users\Padoma1\Videos`. **Dữ liệu thử do tôi tạo không bao giờ nằm trên NAS,
trong khi 87% dữ liệu thật thì có.** Đây là lần thứ hai đúng cái sai này lọt lưới — lần trước là
[BUG-018](./bug.md#bug-018), ổ mạng bị bỏ qua im lặng.

#### Bốn giả thuyết sai, ghi lại vì mỗi cái đều bị một phép đo bác bỏ

Sau khi sửa, ca "2 tệp D: + 1 tệp F:" **vẫn không thả được gì** hai lần. Không sập, không bảng lỗi,
không dòng log nào. Lần lượt:

| Giả thuyết | Phép đo | Kết quả |
|---|---|---|
| Shell từ chối đường dẫn mạng | test dựng data object cho tệp F: | ❌ dựng được |
| Shell từ chối tập **trộn nhiều ổ** | test dựng cho `[D:…, F:…]` | ❌ dựng được |
| Dựng quá chậm nên cử chỉ chuột đã kết thúc | đo thời gian | ❌ 4,3 ms |
| Chuyển hướng stdout/stderr phá thao tác kéo | chạy lại có chuyển hướng | ❌ vẫn chạy tốt |

Cả bốn sai. Điều dứt điểm được: nhật ký cho thấy `start_file_drag` **chưa từng được gọi** trong hai
lần đó — nên lỗi nằm ở cử chỉ/`dragstart`, không phải ở tầng shell. Sau khi dựng lại và cài lại,
đúng ca đó chạy đúng **năm lần liên tiếp** (3 lần bản cài + 2 lần có chuyển hướng).

**Tôi không giải thích được hai lần hỏng đó**, nên ghi thành [RISK-005](./risk.md#risk-005) thay vì
coi như đã xong. Tỉ lệ quan sát được: 2 hỏng / ~12 lần kéo, toàn bộ đều bằng chuột tổng hợp.

#### Hai lỗi của chính kịch bản test, không phải của ứng dụng

**Toạ độ hàng cố định gặp khung cảnh báo.** Khi truy vấn chỉ khớp một phần, khung *"Không có tệp
nào khớp đủ 3 từ"* đẩy cả danh sách xuống ~46px; điểm bắt đầu kéo rơi vào chính khung đó nên không
có thao tác kéo nào bắt đầu. Kịch bản báo "không nhận được gì" — trông y hệt lỗi ứng dụng.

**Bản gỡ lỗi không có giao diện.** Chạy `cargo run` để lấy nhật ký thì cửa sổ chỉ hiện
*"localhost refused to connect"* — bản debug trỏ vào máy chủ vite. Hai lượt chạy sau đó kiểm tra
đúng một trang lỗi trắng, và tôi suýt kết luận "bản gỡ lỗi cũng hỏng". Phải `npm run tauri dev`.

Cùng một bài học với [BUG-016](./bug.md#bug-016): **kịch bản test phải tự chứng minh nó đang nhìn
đúng thứ cần nhìn.** Ảnh chụp màn hình là thứ bác bỏ cả hai — không có nó thì cả hai đều bị ghi
thành lỗi ứng dụng.

#### Một chỗ hụt do phím tổng hợp, không phải do logic

Shift+↓ hai lần chỉ chọn được 2 hàng thay vì 3, ổn định qua nhiều lần chạy. Ba phép đo:

1. `keybd_event` gửi Shift+↓ → WebView2 nhận phím ↓ nhưng **không thấy Shift**, chỉ nhảy tiêu điểm.
2. Đổi sang `SendKeys('+{DOWN}')` → lựa chọn mở rộng đúng, nhưng `N` lần nhấn cho `N` hàng.
3. **Shift+click** cùng dải (chuột, đường vào đáng tin hơn) → **đúng `N+1` hàng**.

Phép đo 3 cho thấy logic chọn dải đúng; chỗ hụt là phím tổng hợp đầu tiên bị rơi sau khi tiêu điểm
vừa đổi. Đã ghi vào kịch bản test để lần sau không truy lại từ đầu.

### 2026-08-25 — Quit hẳn rồi mở lại có quét lại không

**Người dùng hỏi:** vừa tải một video về ổ, nếu Quit hẳn ứng dụng rồi mở lại thì nó có quét lại
không.

Trả lời bằng đo chứ không bằng đọc code. Tạo `D:\video-moi-tai-ve-99887766.mp4`, rồi:

| # | Bước | Quan sát |
|---|---|---|
| 1 | Quit hẳn, mở lại, tìm `99887766` | ❌ **Không tìm thấy** — dòng trạng thái vẫn *"quét lúc 13:00:00"*, y nguyên lần quét cũ |
| 2 | Chạy tác vụ `MediaFinder - cap nhat chi muc` | ✅ Kết quả `0`; tìm thấy ngay, **360.655 → 360.656 tệp**, dấu thời gian đổi thành *15:28:19* |
| 3 | Xoá tệp thử, chạy lại tác vụ | ✅ Về lại **360.655**, tìm không ra nữa |

**Kết luận:** mở lại ứng dụng **không** kích hoạt quét. Khối `setup()` chỉ hiện cửa sổ, đăng ký
phím tắt, dựng khay và theo dõi tệp cache — không có lệnh quét nào. Chỉ mục mới đến từ ba nguồn:
tác vụ lúc **đăng nhập**, tác vụ **hằng ngày 13:00**, và nút **"Quét lại"**.

**Đây là thiết kế cố ý, không phải thiếu sót.** Mở ứng dụng phải gõ được ngay (nạp cache ~0,13 s);
nếu mỗi lần mở đều quét thì mất đúng cái tính chất đó. Nhưng nó tạo ra một khoảng hở thật: tệp tải
về lúc 14:00 sẽ không tìm thấy cho tới 13:00 hôm sau, trừ khi bấm "Quét lại" — nên khoảng hở này
được ghi vào [ISSUE](./issue.md) chứ không để ngầm hiểu.

**Điều mục 2 cũng chứng minh luôn:** ứng dụng đang mở **tự nạp lại** chỉ mục mới do tác vụ nền ghi
ra, không cần khởi động lại — `watch_cache` phát hiện cache đổi trong 5 giây.

### 2026-08-25 — Lượt test P14 (xem trước ngay trong ứng dụng)

**Đo trước khi viết một dòng code nào**, vì cả tính năng phụ thuộc vào một câu chưa ai kiểm tra:
phát tệp NAS qua SMB có mượt không.

| Phép đo trên `F:`, tệp 1.987 MB | Kết quả |
|---|---|
| Byte đầu tiên | **66 ms** |
| Thông lượng | **84,7 MB/s** (678 Mbps) |
| Nhảy tới cuối tệp rồi đọc 1 MB | **18 ms** |

Cao hơn hẳn bitrate của bất kỳ video nào → làm được. Nếu số ra ngược lại thì tính năng này đã
không nên làm.

| # | Nội dung test | Cách làm | Kết quả |
|---|---|---|---|
| 0 | Vòng kiểm tra | test · clippy · fmt · npm | ✅ **203 pass**, sạch cả bốn |
| 1 | Phân tích `Range` | unit test | ✅ 7 test: đoạn thường, mở đuôi, đuôi tệp, quá cỡ, cắt trần, rác |
| 2 | **Phát video NAS trong ứng dụng** | nháy đúp thật, chụp màn hình | ✅ hiện và phát sau **1,2 giây** |
| 3 | Có phát thật hay đứng hình | chụp lại ở 4 giây | ✅ khung hình đổi, thanh điều khiển tự ẩn |
| 4 | `↑↓` đổi tệp mà không rời lớp phủ | gửi `{DOWN}`, đọc tiêu đề | ✅ tên đổi, bộ đếm **2 / 40**, hiện `2.6 MB · 1280×720` |
| 5 | `Esc` đóng và giữ đúng chỗ | chụp sau `Esc` | ✅ về danh sách, tiêu điểm nằm ở hàng vừa dừng |
| 6 | Xem trước ảnh | truy vấn `462222` | ✅ ảnh vừa khung, không phóng to vỡ nét |
| 7 | Định dạng không giải mã được | tự tạo `.mkv` chứa chữ | ✅ hiện *"Không xem trước được định dạng này"* + nút mở bằng ứng dụng ngoài |
| 8 | Không rò rỉ cử chỉ mở | 3 lần chạy, 6 ảnh | ✅ **tất cả 880×620** (trước khi sửa: 1920×1080) |

**Kết luận:** 9/9 pass, sau khi sửa 2 lỗi tìm được trong chính lượt test.

#### Hai lỗi, cả hai đều do tự kiểm chứng chứ không do đọc lại code

**[BUG-021](./bug.md#bug-021) 🟠 — nháy đúp mở xem trước làm cửa sổ bung toàn màn hình.** Thứ phát
hiện ra nó là **kích thước ảnh chụp**: `PrintWindow` trả về 1920×1080 thay vì 880×620. Nếu chỉ nhìn
nội dung ảnh thì không thấy gì bất thường — video vẫn hiện, vẫn phát.

Lần sửa đầu (chặn `dblclick` trên `<video>`) **có lần hết, có lần không** — và chính sự không ổn
định đó là dữ liệu: nếu chặn đúng sự kiện gây lỗi thì phải hết hẳn. Cách sửa đúng là lớp phủ từ
chối mọi thao tác chuột trong 250 ms đầu, không cần biết sự kiện nào rò rỉ.

**[BUG-022](./bug.md#bug-022) 🟡 — `D:\` hiện thành `:D`.** Chỉ lộ ra vì tệp thử tình cờ nằm ngay
gốc ổ. Mọi ảnh chụp trước đó đều là tệp nằm sâu nhiều cấp, nên mẹo `direction: rtl` trông vẫn ổn.

#### Một giả định của tôi bị phép đo bác bỏ

Tôi cố ý **không** khai báo `.mkv`/`.avi` là video, tin rằng như vậy trình phát sẽ báo lỗi ngay
thay vì hiện ô đen. Thử thật: một `.mkv` **vẫn phát bình thường** — Chromium đánh hơi nội dung chứ
không tin phần mở rộng. Kiểu MIME ở đây là **gợi ý, không phải cửa chặn**.

Kết quả còn tốt hơn dự tính (xem trước được nhiều tệp hơn), nhưng **chú thích trong mã đã sai** và
đã sửa lại theo đúng hành vi đo được. Một chú thích mô tả hành vi tưởng tượng còn tệ hơn không có
chú thích, vì người sau sẽ tin nó.

#### Hai lỗi của kịch bản test

**Lớp phủ còn mở từ lần chạy trước nuốt hết phím gõ vào.** Kịch bản gõ truy vấn mới, nhưng lớp phủ
đang giữ bàn phím nên chẳng có gì tới ô tìm kiếm — và ảnh chụp ra là tệp của *lần trước*. Suýt ghi
thành "tính năng không hoạt động". Đã thêm `Esc` vào đầu mọi kịch bản.

**Bộ gõ Telex đổi truy vấn.** Gõ `An_extremely` thì ô tìm kiếm nhận `An_ẽtremely` — Telex biến `ex`
thành `ẽ`. Ba lần chạy liên tiếp "thành công" thật ra không mở nổi lớp phủ nào vì truy vấn không có
kết quả. Từ nay kịch bản dùng truy vấn **toàn chữ số**. Đây là bộ gõ của máy, không phải lỗi ứng
dụng — nhưng nó làm hỏng phép thử y như một lỗi thật.

#### Một quan sát chưa khép lại: số tệp lệch 1

Trước lượt test: **360.655 tệp · 15.298 thư mục**. Sau khi dọn sạch 3 tệp thử tôi tự tạo:
**360.654 tệp · 15.302 thư mục**. Lệch **−1 tệp, +4 thư mục** trong khoảng 1,5 giờ.

**Nhiều khả năng là biến động bình thường của một ổ đang được dùng** — D: là ổ làm việc, CapCut
sinh và xoá tệp tạm liên tục. Chỉ mục chỉ gỡ một mục khi **journal nói tệp đó đã bị xoá**, chứ
không tự bỏ mục nào; và quy tắc "một lần quét không có thẩm quyền với ổ nó không quét" vẫn giữ
nguyên mọi mục trên NAS.

**Chưa xác nhận được bằng đo.** Chế độ `--audit` dựng lại đúng lịch sử xoá từ USN journal và trả
lời được câu này, nhưng nó cần quyền Administrator: chạy ẩn thì mất kết quả, còn đăng ký một tác vụ
quyền cao thì bản thân việc đăng ký cũng cần quyền cao. Cả hai đường đều dẫn tới một lời nhắc UAC,
nên tôi dừng lại thay vì tự bật nó lên.

**Cách khép lại khi cần:** chạy `mediafinder.exe --audit D` trong một cửa sổ dòng lệnh mở bằng
quyền Administrator. Nó liệt kê từng tệp bị xoá kèm thời điểm, dựng lại từ journal chứ không từ
chỉ mục — nên nó thấy được cả những gì chỉ mục chưa kịp biết.

### 2026-08-25 — Người dùng báo: video tràn ra ngoài khung xem trước

**Nguồn phát hiện là người dùng, không phải kịch bản test** — đúng dạng của giai đoạn BT. Kèm hai
ảnh chụp: video 1920×1080 tràn xuống đè lên dòng chân.

| # | Nội dung | Cách làm | Kết quả |
|---|---|---|---|
| 1 | Tái hiện | mở đúng thư mục người dùng đang xem, video 1080p | ✅ tái hiện được |
| 2 | Sửa bố cục | `grid-template-rows: minmax(0,1fr)` + `object-fit: scale-down` | ✅ video nằm gọn, dòng chân hiện đủ |
| 3 | Không phóng to tệp nhỏ | `scale-down` thay `contain` | ✅ giữ đúng ý định ban đầu |
| 4 | Vòng kiểm tra | test · clippy · fmt · npm | ✅ **203 pass**, sạch cả bốn |
| 5 | Tệp lớn trên ổ mạng | video **12 GB, 42:58** trên `Y:` | ✅ phát từ đầu, thanh tua chạy |

#### Và lỗi cũ của tôi lộ ra là chưa sửa xong

Trong lúc tái hiện, [BUG-021](./bug.md#bug-021) (cửa sổ tự bung toàn màn hình) **xuất hiện lại** —
thứ tôi đã tuyên bố sửa xong ở lượt test P14.

**Vì sao lượt P14 tưởng là hết.** Tôi chạy 3 lần trên truy vấn trả về **2 kết quả**, đều sạch. Lỗi
này chỉ lộ dưới tải: truy vấn **5.000 kết quả** với video 1080p thì hiện lại ngay, **2/5 lần**.

**Hai lần sửa, và một lượt kiểm chứng hỏng nằm giữa:**

| Cách sửa | Kết quả đo |
|---|---|
| Chặn `dblclick` trên `<video>` | ❌ 2/5 vẫn bung |
| `pointer-events: none` + chặn `dblclick` ở **pha capture trên `window`** | ✅ **5/5 sạch** |

Trình điều khiển media của Chromium xử lý cú nháy đúp trước khi sự kiện nổi lên tới thẻ `<video>`,
nên trình xử lý ở tầng phần tử luôn tới muộn; ở pha capture thì thấy trước tất cả.

**Nhưng tôi suýt kết luận ngược lại.** Sau lần sửa thứ hai, phép đo vẫn báo "2/5 vẫn bung", nên tôi
đã chuyển sang chặn ở *kết quả* và ghi lời giải thích ấy vào mã. Hoá ra lệnh cài của tôi chạy trên
**bộ cài cũ** — lệch 16 phút so với bản vừa dựng. Cài đúng bản rồi đo lại: **5/5 sạch**. Tắt riêng
lớp chặn kết quả rồi đo lại lần nữa, đối chiếu mã băm: **vẫn 5/5 sạch** → lớp chặn ở tầng cửa sổ tự
nó đã đủ, và lớp kia đã bị gỡ. Chi tiết: [CHECK-008](./check.md#check-008).

**Hai bài học.** Một: ba lần chạy sạch trên dữ liệu nhẹ không chứng minh được gì về dữ liệu nặng.
Hai, và nặng hơn: **một phép đo trên nhị phân sai còn tệ hơn không đo** — nó không chỉ bỏ sót lỗi
mà còn dựng lên một lý thuyết nhân quả sai, rồi lý thuyết ấy được ghi vào tài liệu như tri thức.

### 2026-08-25 — Đóng gói thư viện: đo trước khi xây

**Người dùng muốn:** bộ cài tự lo phần thư viện — thiếu thì cài, cũ thì nâng lên — để người dùng
chỉ việc mở phần mềm ra dùng.

| # | Nội dung | Cách làm | Kết quả |
|---|---|---|---|
| 1 | **Thật sự cần thư viện nào** | đọc bảng nhập PE của exe đã dựng | ✅ 22 DLL, **không cái nào phải cài thêm** ([CHECK-009](./check.md#check-009)) |
| 2 | Có cần Visual C++ Redistributable không | tìm `vcruntime140.dll` trong bảng nhập | ✅ **không có** — Rust liên kết tĩnh |
| 3 | WebView2 trên máy này | đọc registry đúng khoá bộ cài kiểm | ✅ `151.0.4129.107`, cấp máy |
| 4 | Hộp thoại khi thiếu WebView2 | ép hiện qua khe thử, chụp màn hình | ✅ sau **2 lần sửa câu chữ** |
| 5 | Hộp thoại khi Windows quá cũ | như trên | ✅ |
| 6 | Đường bình thường không bị ảnh hưởng | mở ứng dụng, tìm thật | ✅ 360.654 tệp, 1,7 ms |
| 7 | Vòng kiểm tra | test · clippy · fmt · npm | ✅ **212 test**, sạch cả bốn |

**Kết luận:** 7/7 pass. Cơ chế cần xây **nhỏ hơn nhiều** so với hình dung ban đầu — không phải một
trình quản lý thư viện, mà một bước kiểm tra và một câu nói đúng lúc.

#### Câu chữ phải sửa hai lần, và chỉ ảnh chụp mới cho biết

Test đã kiểm nội dung thông báo (có "Cách xử lý:", không lộ tên API). Test **không** thấy được thứ
hỏng thật: hộp thoại của Windows rộng khoảng 50 ký tự, và dòng nào dài hơn thì nó **cắt vào giữa
chữ** thay vì lùi về khoảng trắng.

```
bản 1:  "... tệp có tên kết thúc bằ
         ng -setup.exe ..."
bản 2:  "... nên nó sẽ tự c
         ài giúp bạn."
```

Với tiếng Việt, một chữ bị cắt đôi trông như phần mềm lỗi phông — và đây là màn hình **duy nhất**
người dùng thấy khi máy họ chưa chạy được. Nay mỗi dòng dưới 46 ký tự, và có một test giữ ngưỡng đó
khỏi trôi khi ai đó sửa câu sau này.

**Bài học:** test kiểm được *nội dung* thông báo, không kiểm được *hình dạng* của nó. Thứ duy nhất
trả lời được câu "người dùng nhìn thấy gì" là nhìn thử.

#### Dính bẫy bản dựng cũ thêm hai lần nữa

Cùng họ với [CHECK-008](./check.md#check-008), lần này nguyên nhân mới: **đang chạy ứng dụng thì bộ
cài không ghi đè được exe, mà vẫn thoát với mã 0.** `Stop-Process` trả về ngay trong khi tiến trình
còn đang thoát và vẫn giữ tệp.

Cả hai lần đều do chính kịch bản chụp hộp thoại khởi động ứng dụng lên rồi để đó. Công cụ cài đặt
nay **chờ tiến trình biến mất khỏi danh sách tiến trình** chứ không chỉ gọi lệnh tắt, và tự thử lại
một lần nếu lần đầu không ghi đè được.

### 2026-08-25 — Chạy thật trên máy trắng, có người bấm UAC

**Phép thử duy nhất không tự động hoá được.** Cần một cú bấm vào hộp thoại UAC, và lách hộp thoại
đó là việc không nên làm. Người dùng ngồi trước máy và thao tác; tôi dựng cảnh và đo kết quả.

**Dựng cảnh cho giống thật:** bộ cài được chép ra Desktop và **gắn Mark-of-the-Web** (`ZoneId=3`).
Không có bước này thì Windows tin sẵn một tệp vừa dựng tại chỗ, và **màn SmartScreen sẽ không hiện
ra** — tức là thứ cần kiểm chứng nhất sẽ bị bỏ qua mà không ai biết.

| # | Bước | Người làm | Kết quả |
|---|---|---|---|
| 1 | Gỡ cài đặt, kèm UAC để xoá tác vụ | người dùng | ✅ máy sạch |
| 2 | Cài qua màn SmartScreen (`More info` → `Run anyway`) | người dùng | ✅ cài xong |
| 3 | `Quét lần đầu` → `Yes` ở UAC | người dùng | ✅ quét xong |
| 4 | Tác vụ định kỳ có được **tự tạo** không | đo | ✅ `Highest` · tên người dùng đúng · lệnh `--index` · **2 trigger** |
| 5 | Lối tắt tự khởi động có được **tự tạo** không | đo | ✅ đúng đích, đúng `--minimized` |
| 6 | **"Quét lại" còn hỏi quyền không** | `schtasks /Run` không elevate | ✅ `SUCCESS`, tác vụ chạy **kết quả 0** |
| 7 | Chỉ mục dựng từ con số không | mở ứng dụng, tìm thật | ✅ **48.291 tệp · 3.204 thư mục**, **0,4 ms** |
| 8 | Trả lại chỉ mục đầy đủ của người dùng | sao lưu + tác vụ cập nhật | ✅ **362.237 tệp**, có cả NAS |

**Kết luận: 8/8 pass.** Mục 6 là mục quan trọng nhất — nó quyết định 20–40 người kia có bị hỏi
quyền mỗi ngày hay không. Câu trả lời: **một lần duy nhất, lúc quét đầu tiên.**

**Điều đáng ghi về cách dựng phép thử.** Nếu chỉ chạy bộ cài từ thư mục dựng, mọi bước vẫn "pass"
mà **không hề đi qua SmartScreen** — vì Windows không chặn tệp do chính máy tạo ra. Phép thử khi đó
trông giống hệt phép thử thật nhưng bỏ sót đúng cái màn hình mà người dùng mới sẽ khựng lại. Một
dòng gắn `ZoneId=3` là khác biệt giữa hai điều đó.

#### Hai điều chỉ người dùng xác nhận được

Sau lượt thử trên máy trắng, hai đường tôi không tự kiểm được đã được người dùng xác nhận:

| Nội dung | Xác nhận |
|---|---|
| Màn SmartScreen hiện **đúng như hướng dẫn đã viết** (`More info` → `Run anyway`) | ✅ khớp |
| Bộ gỡ cài đặt **có hiện UAC** để xoá tác vụ chạy nền, và bấm `Yes` thì xoá được | ✅ hoạt động |

Điều thứ hai đóng lại phần còn dở ở lượt trước: khi đó tôi chỉ đo được bằng gỡ im lặng — đường đó
**cố ý bỏ qua** bước xin quyền, nên tác vụ còn sót lại và tôi không biết đường xin quyền có chạy
được không. Nay biết là chạy được.

## 2026-08-27 — Lượt test P22 (bốn mảng backend)

Nguồn phát hiện: tự kiểm chủ động sau khi viết, theo phương pháp đột biến.

| # | Phát hiện | Xử lý |
|---|---|---|
| 1 | `SCHEDULE_MARK` khai báo nhưng `schedule_is_current` so bằng chuỗi gõ tay — hai nơi có thể trôi lệch, và khi lệch thì task hoặc nâng cấp mãi hoặc không bao giờ nâng | clippy chỉ ra; sửa cho hàm dùng chính hằng số làm nguồn sự thật duy nhất, kèm mẩu ASCII suy ra từ nó cho đầu ra mã trang OEM |
| 2 | Lịch v2 chưa có test — ai gỡ `Repetition` khỏi XML thì marker thành nói dối mà không gì đỏ | thêm `lich_v2_marker_va_xml_khop_nhau`; đột biến gỡ Repetition → đỏ đúng chỗ |
| 3 | setup.rs đã có sẵn `mod tests` — mod mới đặt tên trùng gây E0428 | đổi `schedule_v2_tests`; bài học: grep trước khi append mod test |

### Nghiệm thu tay trên máy thật (cùng ngày)

Bốn mảng chạy trên bản dev, thư viện thật 48.319 tệp:

| Mảng | Kết quả |
|---|---|
| A — nhật ký file | ✅ `%LOCALAPPDATA%\MediaFinder\logs\mediafinder.log` ghi liên tục, không màu ANSI, đủ mọi dòng như stderr |
| B — bộ ghi 0-kết-quả | ✅ chạy suốt phiên **không tạo file `misses.*` nào** — lời hứa mặc-định-tắt giữ đúng ngoài đời |
| C — xác minh tầng 3 | ✅ 3 tệp × 8 MB → **30 ms** (~800 MB/s), tách đúng kẻ giả dạng khác 1 byte ở giữa bụng |
| D — lịch 15 phút | ⚠️ **lộ một bug thật** — xem dưới |

#### BUG-P22-01: marker phiên bản lịch có dấu tiếng Việt → vòng lặp xoá-tạo-lại vô tận

Người dùng chạy task elevated trên máy thật: `PT15M` = CO (lịch **đã** nâng cấp) nhưng marker = KHONG
(hàm nhận diện **không** đọc thấy marker trong XML nó vừa tự ghi). Hậu quả nếu phát hành: indexer
xoá và tạo lại scheduled task ở **mỗi** lần chạy — mỗi 15 phút, mãi mãi.

Nguyên nhân: `schtasks /XML` in ra **UTF-8** (không phải UTF-16 như dự đoán), và marker `"[lịch v2"`
chứa `ị` — ký tự nhiều byte. Phép so trên "mẩu ASCII lọc ra" tìm chuỗi `"ch v2]"`, nhưng giữa `ch`
và ` v2]` thực tế là `ị` + `h`. Đoán sai hình dạng dữ liệu, và chỉ máy thật mới phơi ra.

Sửa: marker thành **thuần ASCII** `[schedule-v2:`; hàm nhận diện thử UTF-8 trước rồi UTF-16. Thêm
`assert!(SCHEDULE_MARK.is_ascii())` vào test — đột biến trả marker về chuỗi có dấu làm test đỏ đúng chỗ.

Kiểm chứng lại trên máy thật: lần chạy 1 → marker CO, PT15M CO; lần chạy 2 → y hệt, và log chỉ có
**đúng một** dòng "lịch định kỳ là bản cũ". Vòng lặp đã chấm dứt.

Kết luận: 236 test Rust + 94 test JS xanh; 7/7 đột biến bị bắt. Toàn bộ nằm ở working tree chờ
duyệt theo quy tắc git-theo-lệnh.

## 2026-08-28 — Lượt test P23 (lọc theo ổ đĩa)

| # | Phát hiện | Xử lý |
|---|---|---|
| 1 | `$effect` reset con trỏ chạy cả ở lần mount đầu, khi `listRef` chưa tồn tại — 13 test đỏ và 8 lỗi ngầm | so với giá trị trước đó thay vì chạy mỗi lần effect kích |
| 2 | `VirtualList.scrollToTop()` dùng `viewport.scrollTo`, jsdom không có → ném lỗi giữa effect, giết phần việc đứng sau | gán `scrollTop = 0`; cùng kết quả, không phụ thuộc phương thức có thể vắng |
| 3 | **Đột biến "ổ mạng xuống cuối" lọt lưới** — dữ liệu test có C: < D: < NAS nên xếp thuần chữ cái tình cờ ra đúng thứ tự | thêm ca `\ALPHA` vs `Z:`: xếp chữ cái sẽ cho ALPHA trước, nên quy tắc buộc phải lộ ra |

Bài học lặp lại lần thứ hai trong hai ngày: **dữ liệu kiểm thử tình cờ đúng cũng là một kiểu mù**.
Đột biến là thứ duy nhất phát hiện ra nó.

Kết luận: 241 test Rust + 112 test JS xanh; 5/5 đột biến bị bắt sau khi vá.

## 2026-08-28 (chiều) — Lượt test P24 (hỏi trước khi quét ổ mạng)

| # | Phát hiện | Xử lý |
|---|---|---|
| 1 | Tôi phỏng đoán "Quét lại" xoá mất dữ liệu NAS — **sai**. Cả hai chiều hợp nhất đều bỏ qua ổ không chạm tới, có từ P10 | đính chính với người dùng ngay; bài học: đọc code trước khi đoán, nhất là khi câu chuyện nghe hợp lý |
| 2 | `btn(document, "Quét lại")` bắt nhầm nút trên thanh chính thay vì nút trong hộp thoại — test xanh vì lý do sai | thêm `dlgBtn()` giới hạn phạm vi tìm trong `[role=dialog]` |
| 3 | **Đột biến "lượt huỷ vẫn ghi dấu vết" lọt lưới** — quy tắc nằm trong một `if` ở `lib.rs`, test chỉ mô phỏng lại được điều kiện | tách thành `record_outcome()`; test gọi thẳng hàm mà `lib.rs` dùng |

Bài học #3 là biến thể của bài học P23 (dữ liệu tình cờ đúng): **một bài kiểm thử mô phỏng lại logic
thay vì gọi nó thì không canh gì cả.** Quy tắc muốn được canh thì phải có một cái tên để gọi tới.

Kết luận: 242 test Rust + 121 test JS xanh; 4/4 đột biến bị bắt sau khi vá.

## 2026-08-28 (tối) — Lượt test P25 ("Quét lại" nói nó vừa làm gì)

Không phát hiện lỗi mới. 8 ca của nhóm t13 xanh ngay lần chạy đầu, 4/4 đột biến bị bắt.

Đáng ghi lại một quyết định thiết kế thay vì một lỗi: người dùng đề nghị áp cùng cách xử lý của
"+ ổ mạng" (hộp thoại xác nhận) cho "Quét lại". Hai nút trông giống nhau nhưng **cái giá khác nhau
một bậc độ lớn** — vài phút so với vài giây — nên cùng một giải pháp sẽ đúng ở nút này và sai ở nút
kia. Trình bày cả hai phương án kèm cái giá, để người dùng chọn.

Kết luận: 246 test Rust + 129 test JS xanh.

## 2026-08-28 (đêm) — Lượt rà soát độ phủ P26

Không chạy lại cho có: phá từng nhánh rồi xem có ai kêu không.

| # | Phát hiện | Xử lý |
|---|---|---|
| 1 | **`ContextMenu` không có nhóm test nào** — ba nhánh (đóng khi bấm ngoài, kê vào trong màn hình, bấm mục thì đóng) bị phá mà 129 test vẫn xanh | viết nhóm t14, 13 ca; 4/4 đột biến bị bắt |
| 2 | Ca "Escape không lọt xuống app" tôi viết sai giả định — `stopPropagation` không chặn listener anh em trên cùng `window` | viết lại thành lời ghi chép về giới hạn đó, trỏ sang TC-3.16b nơi canh chốt thật |
| 3 | Ca kiểm mép trái quá lỏng: `(2,2)` với `>= 0` — bỏ `Math.max` vẫn qua | đổi sang toạ độ âm và kiểm `> 0` |

Bài học lần thứ ba trong hai ngày, mỗi lần một dạng: dữ liệu tình cờ đúng (P23), test mô phỏng thay
vì gọi (P24), và giờ là **cả một component không ai canh**. Cách phát hiện luôn giống nhau: phá rồi
xem có ai kêu.

Kết luận: 246 test Rust + 142 test JS xanh.

## 2026-08-28 (khuya) — Điều tra BUG-024 do người dùng báo

Nguồn phát hiện: **người dùng**, không phải kịch bản test. Mô tả ban đầu mơ hồ như thường lệ ("tìm
file mà không ra"), và việc đầu tiên là tái hiện — ở đây là tra thẳng chỉ mục thật trên máy.

| # | Phát hiện | Xử lý |
|---|---|---|
| 1 | Tệp có thật trên NAS nhưng 0/368.866 mục chỉ mục biết tới nó | không phải lỗi xếp hạng tìm kiếm — bộ tìm không trả về được thứ nó không biết |
| 2 | **Chẩn đoán đầu của tôi (chỉ mục NAS cũ) giải thích được triệu chứng nhưng KHÔNG giải thích được tương quan phiên bản người dùng nêu** | đào tiếp thay vì đóng vụ; chi tiết không khớp mới là chỗ đáng đào |
| 3 | `nsis-hooks.nsh` xoá `index.bin` vô điều kiện; cài tay đè lên bản cũ chạy qua uninstaller | thêm chốt chặn ba tín hiệu; 4 bài kiểm thử đọc thẳng tệp `.nsh` |
| 4 | Ghi chú phát hành hứa "Chỉ mục đã quét vẫn giữ nguyên" — sai với đường cài tay | sửa lời, nói rõ bản nào còn dính và cách phục hồi |

Bài học bổ sung vào ba bài của P23–P26: **một tương quan người dùng nêu ra là dữ liệu, kể cả khi nó
mâu thuẫn với chẩn đoán đang có.** Chẩn đoán giải thích được triệu chứng mà không giải thích được
tương quan thì chưa xong.

Kết luận: 250 test Rust + 142 test JS xanh; đột biến khôi phục bản móc cũ làm 4/4 bài đỏ.


## 2026-08-28 (chiều muộn) — Lượt P28: lái app thật bằng chuột, truy vấn thật

Người dùng yêu cầu: *"test cực kỳ chi tiết… phải test log và test trên phiên bản thật (nghĩa là bạn
tự ở app tự điều khiển chuột tự tìm kiếm và đưa ra kết quả cho tôi)"*, sau khi bổ sung rằng lỗi
"10/16 từ" **cũng xảy ra trên ổ cục bộ**, không riêng NAS.

### Cách lái

`SendKeys` không tới được nội dung WebView2 — lượt đầu ô tìm kiếm vẫn rỗng trong ảnh chụp. Đổi sang
đặt clipboard rồi bắn `Ctrl+V` bằng `keybd_event`, đi qua đúng đường xử lý phím của trình duyệt.
Chụp bằng `PrintWindow(PW_RENDERFULLCONTENT)` chứ không `CopyFromScreen`, để một cửa sổ khác đè lên
không làm hỏng bằng chứng.

Đối tượng thử: bản **đã cài** `C:\Users\Padoma1\AppData\Local\MediaFinder\mediafinder.exe`,
FileVersion 1.0.5 — không phải bản dev.

### Kết quả

| # | Truy vấn | Mong đợi | Thực tế | Đạt |
|---|---|---|---|---|
| P28-1 | Tên tệp người dùng báo, dán nguyên | tìm thấy | băng "khớp đủ **16** từ… **10/16**", 22 kết quả sai · 13,6 ms | ❌ tái hiện |
| P28-2 | `.mp4` vừa tạo trên `D:` 2 phút trước | tìm thấy | "Không tìm thấy kết quả nào" | ❌ tái hiện vế ổ cục bộ |
| P28-3 | Y hệt P28-2, sau khi chạy tay tác vụ làm mới | tìm thấy | 2 kết quả · 3,2 ms, đúng thư mục | ✅ |
| P28-4 | Tên 14 từ trên `C:` | 1 kết quả | 1 kết quả · 5,4 ms | ✅ |
| P28-5 | Tên 29 từ tiếng Pháp có dấu, `Z:` | 1 kết quả | 1 kết quả · 5,7 ms | ✅ |
| P28-6 | Tên có khoảng trắng + gạch dưới + chữ hoa, `F:` | 1 kết quả | 1 kết quả · 5,6 ms | ✅ |
| P28-7 | P28-4 viết HOA/thường lẫn lộn | 1 kết quả | 1 kết quả · 4,3 ms | ✅ |
| P28-8 | Tên 12 từ trên NAS `Y:` | 1 kết quả | 1 kết quả · 3,7 ms | ✅ |

P28-2 và P28-3 là cặp đối chứng có kiểm soát: cùng tệp, cùng truy vấn, chỉ khác chỗ chỉ mục đã làm
mới hay chưa. Đó là bằng chứng thẳng rằng nguyên nhân nằm ở **tuổi chỉ mục**, không ở bộ tìm kiếm.

### Đo trên đĩa và trong chỉ mục

Thư mục chứa tệp người dùng tìm, `Y:\PROJECT DEEP SEA 5\DS1_118\Whale Shark`:

```
trên đĩa                        : 125 tệp
chỉ mục biết                    :  51 tệp
thiếu                           :  74 tệp   (51 + 74 = 125, khớp tuyệt đối)
tệp mới nhất chỉ mục biết       : đến ổ 11:12:45
lần quét ổ mạng gần nhất        : 11:23:05  (netscan.json)
tệp người dùng tìm, đến ổ lúc   : 13:48:49  — sau lần quét 2 giờ 25 phút
```

### Test log — và một phát hiện về chính nó

`%LOCALAPPDATA%\MediaFinder\logs\mediafinder.log` **không lớn thêm** trong suốt lượt thử, dù tác vụ
định kỳ chạy lúc 16:00:01, 16:15:01 và 16:21:41 (đối chiếu bằng `LastWriteTime` của `index.bin`).
Lý do: `src-tauri/src/diag.rs` có trên nhánh `edit` nhưng không có trong `master` lẫn tag
`v1.0.5`, nên **bản v1.0.5 người dùng đang chạy không ghi log**.
Toàn bộ nội dung tệp log trên máy này do các bản dev tạo ra. Chẩn đoán từ xa hiện đang mù.

Nội dung log cũ vẫn có giá trị: nó cho thấy `run_incremental()` bỏ ổ mạng ở **mọi** lượt, và cho
thấy `stats.unresolved` bắn thật 2 / 3 / 26 / 73 lần ở các lượt khác nhau.

### Dọn hiện trường

Tệp thử `D:\mf-test-p28\` đã xoá; chạy lại tác vụ làm mới, chỉ mục báo `+5 −2` — hai tệp thử rời
khỏi chỉ mục đúng như mong đợi. Bốn ví dụ dò tạm (`probe_find`, `probe_local`, `probe_search`,
`probe_walk`) đã xoá khỏi `src-tauri/examples/`.

### Kết luận

Bộ tìm kiếm đạt 5/5 ở mọi kiểu tên khó. Hai ca hỏng đều là chỉ mục thiếu tệp, và thiếu vì không có
đường làm mới nào với tới: ổ mạng chỉ được quét khi có người bấm nút, ổ cục bộ mỗi ngày một lần trên
bản đã phát hành. Ghi thành **BUG-025**.


## 2026-08-28 (tối) — Lượt P29: vá chốt chặn trước khi cắt v1.0.6

Chủ dự án phân vân giữa bốn hướng sửa BUG-025, nên lượt này cân nhắc bằng bốn lăng kính độc lập
(người dùng cuối · rủi ro kỹ thuật · chi phí vận hành · khả năng chẩn đoán), một lượt soi hướng bị
bỏ sót, và một lượt phản biện chính lộ trình vừa dựng. Kết quả đáng giá nhất **không phải** thứ
hạng mà là lượt phản biện: nó bắt được một lỗi trong chính đề xuất, thứ suýt làm cả bản phát hành
thành công cốc.

### Lỗi trong đề xuất, bắt được trước khi viết một dòng nào

Lộ trình định vá `upgrade_schedule_if_stale` bằng cách bỏ bước `/Delete`, gọi đó là "một dòng, rủi
ro gần bằng không". Đọc lại mã thì sai: [`setup.rs`](../src-tauri/src/setup.rs) —
`ensure_scheduled_task()` mở đầu bằng `if scheduled_task_exists() { return true; }`. Nâng lịch thì
**luôn luôn** gặp một tác vụ đã tồn tại, đó là tiền đề của việc nâng. Nên bỏ `/Delete` mà vẫn gọi
hàm ấy nghĩa là `/Create /XML /F` **không bao giờ chạy tới**: máy mang lịch v1 sẽ ghi log "nâng lên
lịch v2" ở mọi lượt, mãi mãi, mà lịch không đổi.

Đúng hình dạng của lỗi `SCHEDULE_MARK` đã trả giá ở P22 — cùng một cách hỏng, chỗ khác. Và bài đo
mà lộ trình đề xuất ("xác nhận tác vụ vẫn còn VÀ Repetition = PT15M") rất dễ bị tick xanh chỉ dựa
trên vế đầu.

### Bốn vá, và đột biến chứng minh từng cái

| # | Vá | Đo bằng | Đột biến | Kết quả |
|---|---|---|---|---|
| 1 | Tách `write_task_definition()` ra khỏi chốt `exists`; nâng lịch gọi thẳng nó | `tests/refresh_guards.rs` ×4 | khôi phục lời gọi `ensure_scheduled_task()` | **đỏ đúng 1 bài**, 4 bài kia xanh |
| 2 | Tệp tạm mang PID ở cả ba chỗ (`index.bin.tmp`, `progress json.tmp`, `mediafinder-task.xml`) | cùng tệp trên | đưa `index.bin.tmp` về đường dẫn cố định | **đỏ đúng 1 bài** |
| 3 | `#[serde(default)]` cho `NetScanMark` | `tep_thieu_truong_van_doc_duoc_thay_vi_mat_trang` | — | đọc được tệp 2 trường của bản cũ |
| 4 | `.gitattributes` cho `persist.rs` | `git diff --stat` | — | `Bin 9365 → 10316 bytes` **⇒** `12 +++++++++++-` |

Vá 4 không phải chuyện thẩm mỹ: `persist.rs` khai báo `const MAGIC: &[u8; 8] = b"MFIDX\0\0\0";` —
ba byte NUL thật nằm trong nguồn (byte NUL đầu ở offset 2443), nên git coi cả tệp là nhị phân và
người duyệt **không đọc được diff** của đúng tệp giữ định dạng chỉ mục.

### Bốn đột biến phía giao diện

| Đột biến | Bài phải đỏ | Kết quả |
|---|---|---|
| Gỡ `FreshnessNote` khỏi nhánh 0 kết quả | "không còn im lặng về tuổi" | ✅ đỏ đúng bài |
| Chân cửa sổ quay về một mốc `quét lúc` | "nói HAI mốc, không còn nói dối" | ✅ đỏ đúng bài |
| `matTacVu = !health?.taskExists` | "chưa hỏi được thì không doạ nhầm" | ✅ đỏ đúng bài |
| `coGiDeNoi = true` | "mọi thứ đều tươi thì im lặng" | ✅ đỏ đúng bài |

Mỗi lần chỉ một bài đỏ — không bài nào đang canh hộ bài khác.

### Một test cũ đỏ, và vì sao nó đáng sửa chứ không đáng nới

`t12` đếm **số tuyệt đối** lượt gọi `net_scan_mark` (1 rồi 2). Từ lượt này có thêm một lượt đọc hợp
lệ lúc mở cửa sổ, nên con số thành 2 rồi 3. Bất biến thật của bài ấy là "**mỗi lần mở hộp thoại
phải hỏi lại**", nên nó được viết lại để đo **độ tăng**. Kiểm chứng rằng sửa như vậy không làm nó
mềm đi: bỏ lượt đọc lại trong `beginNetScan` → bài vẫn **đỏ**.

### Nghiệm thu trên bản dựng thật

Dựng bằng `npm run tauri build -- --no-bundle` (bản `cargo build --release` trần vẫn trỏ vào
`devUrl`, ảnh chụp đầu ra `ERR_CONNECTION_REFUSED` — ghi lại để lần sau khỏi mất thời gian), rồi
lái bằng chuột và clipboard như lượt P28.

| Ca | Trước | Sau |
|---|---|---|
| Không tìm thấy kết quả nào | im lặng hoàn toàn về tuổi chỉ mục | **"Ổ trong máy: 1 phút trước · Ổ mạng: 6 giờ trước"**, phần cũ tô hổ phách |
| Băng "10/16 từ" (đúng ca người dùng báo) | chỉ có câu "khớp nhiều nhất" | thêm **"Ổ trong máy: 2 phút trước · Ổ mạng: 6 giờ trước"** |
| Chân cửa sổ | `quét lúc 18:00:01 28/8/2026` — nói dối về nửa NAS | `ổ trong máy 18:00:01 · ổ mạng 11:23:05 28/8/2026` |

### Vòng trước-commit

`cargo test` **259 pass** · `clippy --all-targets` **0 warning** · `fmt --check` sạch ·
`npm run check` **0 lỗi / 125 tệp** · `npm test` **161/161** (15 nhóm, thêm t15).

### Hai đường cần quyền Administrator — đã chạy, cả hai đạt

Chủ dự án mở một PowerShell Administrator; hai script tự chạy trọn vẹn và tự khôi phục tác vụ ở
khối `finally`.

**Bài A — đường nâng lịch v1 → v2.** Dựng một tác vụ đúng hình dạng máy người dùng đang mang (cắt
khối `<Repetition>`, đổi marker thành `[v1]`), chạy `--index` của bản dựng mới, rồi đọc lại:

```
1. da dung task kieu lich v1 -> PT15M: False | marker: False
2. indexer thoat voi ma 0
3. sau khi indexer chay      -> PT15M: True  | marker: True
   KET QUA: DAT
4. da khoi phuc task goc     -> ton tai: True | hop le: True | tro vao: ban da cai (dung)
```

Đây là bài đắt nhất của cả lượt: nếu làm theo đề xuất ban đầu (chỉ bỏ `/Delete`, vẫn gọi
`ensure_scheduled_task`), bước 3 sẽ in `PT15M: False` — và v1.0.6 ra đời với tính năng chính chết
lặng trên mọi máy.

**Bài B — cảnh báo mất tác vụ.** Xoá hẳn tác vụ (`con ton tai: False`), mở ứng dụng, tìm một tệp
không tồn tại. Màn hình hiện đúng câu cần nói:

> Không còn tác vụ làm mới định kỳ trên máy này — chỉ mục sẽ không tự cập nhật nữa. Bấm **Quét lại**
> một lần để tạo lại nó.

kèm "Ổ trong máy: 6 phút trước · Ổ mạng: 9 giờ trước", và chân cửa sổ
`ổ trong máy 20:45:01 · ổ mạng 11:23:05 28/8/2026`. Tác vụ được khôi phục sạch sau đó
(`hop le: True | tro vao: ban da cai`).

**Một chi tiết trong ảnh, không phải lỗi.** Nút **+ ổ mạng** biến mất khỏi thanh công cụ ở ảnh này.
Nguyên nhân là bài thử: script chạy trong PowerShell Administrator nên `Start-Process` mở ứng dụng ở
tiến trình **nâng quyền**, mà tiến trình nâng quyền thuộc phiên đăng nhập khác và không nhìn thấy ổ
ánh xạ — đúng CHECK-007 đã ghi trong mã. Người dùng thật mở ứng dụng theo lối tắt Startup
(`asInvoker`) nên không gặp. Dòng tuổi chỉ mục vẫn đúng, vì mốc ổ mạng đọc từ `netscan.json` trên
đĩa chứ không từ việc liệt kê ổ.

### Ba lỗi trong chính script kiểm thử, sửa trước khi tin kết quả

Lần chạy đầu cả hai script đều hỏng giữa chừng (khối `finally` vẫn khôi phục đúng). Ghi lại vì lỗi
thứ ba là loại nguy hiểm nhất — nó không làm script đổ, nó làm script **nói dối**:

1. **Tên biến PowerShell không phân biệt hoa–thường.** `$v1` (nội dung XML) đè lên `$V1` (đường dẫn
   tệp) → `WriteAllText` nhận cả đống XML làm đường dẫn.
2. **`2>$null` trên chương trình ngoài.** PS 5.1 bọc stderr thành `ErrorRecord`; gặp
   `ErrorActionPreference='Stop'` là ném — đúng lúc `schtasks /Query` báo không tìm thấy tác vụ vừa
   xoá. Mọi lệnh `schtasks` nay đi qua `cmd /c` để cmd tự nuốt stderr.
3. **`& echo %ERRORLEVEL%` luôn nói dối.** cmd khai triển biến cả dòng **trước khi** chạy, nên nó in
   mã lỗi của lệnh *trước đó*. Hàm kiểm tra tồn tại trả `True` cho cả một tên tác vụ bịa ra — thử
   bằng `khong-he-co-task-nay` mới lộ. Nếu không bắt, bài B sẽ báo "xoá rồi mà vẫn còn tồn tại" và
   kết luận sẽ sai. Nay dùng `&& echo CO || echo KHONG`.

Bài học lặp lại của cả dự án: **dụng cụ đo cũng phải được đo.** Cùng một họ với lỗi `SCHEDULE_MARK`
ở P22 và với đột biến "cancelled scan" ở P26 — một phép kiểm mô phỏng lại điều kiện thay vì hỏi
thẳng hệ thống thì không bao giờ đỏ khi bản thật hỏng.


## 2026-08-28 (khuya) — Lượt P30: rà soát toàn bộ tính năng trước khi cắt v1.0.6

Chủ dự án hỏi thẳng: *"đã fix xong hoàn tất chưa, liệu version mới đến tay user có bị mắc lỗi"*, và
yêu cầu test lần lượt toàn bộ tính năng, viết test case cho bài bản.

Chia làm 6 mảng, mỗi mảng một người soi độc lập, mỗi rủi ro bị một người khác cố bác bỏ: **55 rủi
ro nêu ra, 52 sống sót**. Tỉ lệ sống sót cao bất thường nên **không chuyển thẳng** — tự đối chiếu
từng khẳng định nặng bằng mã trước khi tin.

### Bốn khẳng định tự kiểm chứng — cả bốn đều đúng

**1. Cache ảnh thu nhỏ khoá theo VỊ TRÍ, không theo tệp.** `thumbnail.rs` khai
`type CacheKey = (u64, u32)` = (chỉ số trong chỉ mục, kích thước), và `get()` trả cache hit **mà
không hề đọc tham số `path`**. Chỉ số là một vị trí, nên sau một lượt dựng lại chỉ mục thì số 42
chỉ vào tệp khác. Chốt epoch ở `protocol.rs` **không cứu được**: nó chỉ từ chối yêu cầu *đang bay*
mang epoch cũ; yêu cầu mới mang epoch *đúng* thì đi lọt rồi trúng cache.

Chính chú thích trong mã tự tố cáo: `protocol.rs:14-17` viết *"an in-flight request would quietly
paint the wrong picture next to the right name"*, và `:82-83` viết *"a cached response can never go
stale"*. Cả hai đều không đúng với cache nằm sau nó.

Vì sao là chuyện của v1.0.6: lịch cũ mỗi ngày một lượt thì cửa sổ này gần như không tồn tại; lịch
`PT15M` mới làm chỉ mục nạp lại vài lần mỗi giờ **ngay giữa phiên làm việc**. Với người dựng phim
chọn clip bằng khung hình, ảnh sai cạnh tên đúng là kiểu hỏng tệ nhất — nó không báo lỗi, nó khiến
người ta chọn nhầm.

**Cách sửa, và vì sao không chọn cách hiển nhiên.** Thêm epoch vào khoá cũng chặn được lỗi, nhưng
mỗi lượt nạp lại sẽ làm mọi khoá đổi hết — vứt sạch cache vài lần mỗi giờ ngay dưới tay người đang
cuộn, tức sửa một lỗi bằng cách tạo một lỗi khác. Đã khoá theo **thứ bức ảnh mô tả**:
`(băm đường dẫn viết thường, mtime, cạnh px)`. Hai tệp khác nhau không bao giờ chung khoá, **và**
cùng một tệp giữ nguyên khoá qua mọi lượt dựng lại — cache sống sót. `mtime` bắt ca xuất lại một
bản dựng đè lên chính nó.

**2. Cả nhánh "ổ mạng" của tính năng lọc-theo-ổ là mã chết trên mọi máy studio.**
`isNetworkDrive` chỉ nhận tiền tố UNC `\\`. Nhưng `driveKey("Y:\PROJECT…")` trả `"Y"`, và cả bốn ổ
NAS của studio đều là **ổ ánh xạ** — `net use` trên máy này: `F:`, `H:`, `Y:`, `Z:`. Nên
`isNetworkDrive("Y")` = `false`: không chip cam, không nhãn cam, ổ mạng không bị đẩy xuống cuối
hàng chip. Không ai báo lỗi vì phần lọc và đếm vẫn đúng — tính năng lặng lẽ giải đúng một nửa vấn
đề nó sinh ra để giải. Phần còn thiếu vốn đã có sẵn: lệnh `network_drives` trả danh sách chữ cái,
và `App.svelte` đã gọi nó từ trước cho màn hình chạy lần đầu; chỉ chưa ai truyền nó vào chỗ cần.

**3. Tầng 3 kết luận sai khi có tệp không đọc được.** `allSame()` đòi
`groups.length === 1 && unreadable.length === 0`, nên nhóm có a+b trùng nhau cộng c **chưa đọc
được** rơi vào nhánh else và hiện cảnh báo đỏ *"⚠ có tệp khác nội dung"* — trong khi không tệp nào
khác nội dung. Ca cả nhóm nằm trên NAS vừa rớt mạng còn tệ hơn: `groups` rỗng, tiêu đề đỏ, mọi
dòng bên dưới mang nhãn "không đọc được". Trái đúng nguyên tắc `verify.rs` tự đặt ra, và người dùng
đang định **xoá tệp** dựa trên câu trả lời này. Nay có ba trạng thái: trùng / khác / **chưa kết
luận**.

Đáng nói hơn: bài `t10` đang **khoá hành vi sai**. Tên bài nói "một tệp khác nội dung" nhưng dữ
liệu là `groups:[["a","b"]], unreadable:["c"]` — a và b trùng nhau. Chú thích của chính bài thừa
nhận dữ liệu đã bị đổi mà khẳng định thì không đổi theo. Một bài kiểm thử sai còn nguy hơn không có
bài nào: nó cấp giấy chứng nhận cho lỗi.

**4. `npm test` chưa từng chạy trong CI.** Cả `check.yml` lẫn `release.yml` đều chạy
`npm run check` (chỉ kiểm kiểu), `cargo fmt`, `cargo clippy`, `cargo test` — **không** có
`npm test`. 165 bài canh hợp đồng giao diện chỉ sống nếu có người nhớ gõ tay.

### Một quả mìn tìm thấy dọc đường

`thumbnail.rs` chứa một **ký tự backspace thật** (`\x08`) nằm lẫn trong mã nguồn — di chứng heredoc
của một phiên trước. Nó biên dịch được vì nằm trong raw string, nên không ai biết. Đã dọn, và thêm
một khẳng định chặn: không còn ký tự điều khiển nào trong tệp.

### Đột biến

| Đột biến | Bài phải đỏ | Kết quả |
|---|---|---|
| Khoá cache không phụ thuộc đường dẫn | `hai_tep_khac_nhau_khong_bao_gio_dung_chung_khoa` | ✅ đỏ đúng 1/11 bài |

### Vòng trước-commit

`cargo test` **264 pass** · clippy **0 warning** · fmt sạch · `npm run check` 0 lỗi/125 tệp ·
`npm test` **165/165** (thêm 4 bài) · `check-release-notes.sh` 1074/1200.

### Chưa sửa — ghi ra để không ai tưởng là đã xong

Còn khoảng tám mục mức "đáng kể" chưa xử lý, đáng chú ý nhất:

* `"Ổ mạng: chưa quét lần nào"` sẽ hiện **sai trên mọi máy nâng cấp**: `netscan.json` là tệp mới
  của v1.0.6 nên `load()` trả `None`, dù chỉ mục đang có 320.505 mục NAS. Sửa bằng câu chữ:
  `null` thì nói "chưa rõ lần trước".
* **Ngưỡng 30 phút báo động giả**: `run_incremental` cố ý không ghi cache khi không có gì đổi, mà
  `built_at_unix` chỉ đóng dấu lúc `save()`. Máy yên tĩnh buổi tối sẽ bị tô vàng "Ổ trong máy: 4
  giờ trước" trong khi tác vụ vừa chạy hai phút trước.
* `index-reloaded` không đóng lớp xem trước và menu chuột phải đang mở — lịch 15 phút đưa chuyện
  này vào giữa phiên làm việc.
* Lớp xem trước giữ vết hỏng của tệp trước (`failed`/`loading` không reset khi `hit` đổi).
* Tầng 3 không có đường huỷ và không giới hạn song song.


## 2026-08-29 (rạng sáng) — Lượt P31: làm nốt bốn mục còn treo của P30

### 1. `"Ổ mạng: chưa quét lần nào"` nói sai trên mọi máy nâng cấp

`netscan.json` là tệp mới của bản này, nên `load()` trả `None` trên **mọi** máy nâng cấp — kể cả
máy đang có 320.505 mục ổ mạng trong chỉ mục. Nói "chưa quét lần nào" ở đó là khẳng định một điều
mình không biết, và khẳng định sai đúng trên màn hình được dựng lên để *"thôi để họ kết luận sai"*.

Không có đường di trú nào khả dĩ: chỉ mục cố ý không cấp `VolumeStamp` cho ổ mạng nên không suy
ngược ra mốc cũ. Sửa bằng câu chữ — **chưa biết thì nói là chưa biết**: `"chưa rõ lần trước"`.
Chân cửa sổ cũng thôi rơi về câu một-mốc `"quét lúc …"` mà nói rõ `"ổ trong máy …"`.

Thêm: máy không gắn ổ mạng nào thì **đừng nhắc tới ổ mạng**.

### 2. Ngưỡng 30 phút báo động giả — sửa bằng một mốc mới, không bằng cách nới ngưỡng

`run_incremental` cố ý không ghi lại cache khi journal không có gì đáng áp; đó là quyết định đúng,
nó tránh ~4,5 GB ghi SSD mỗi ngày. Nhưng `built_at_unix` chỉ được đóng dấu trong `persist::save()`,
nên **một máy hoàn toàn khoẻ** — tác vụ vừa chạy hai phút trước — vẫn bị tô vàng *"Ổ trong máy: 4
giờ trước"* chỉ vì buổi tối không ai đụng vào tệp nào.

Cách rẻ là chỉ cảnh báo khi mất tác vụ. Cách đúng là nhận ra **hai câu hỏi khác nhau cần hai con số
khác nhau**:

* `built_at_unix` — chỉ mục **đổi** lần cuối lúc nào.
* mốc mới — cỗ máy làm mới **chạy** lần cuối lúc nào.

Con số thứ hai mới trả lời được câu giao diện thật sự hỏi. Module riêng `src-tauri/src/lastcheck.rs`
(theo quy tắc chia nhỏ), tệp JSON vài chục byte cạnh cache — không đụng `SCHEMA_VERSION`, và không
rơi lại vào chính cái bẫy ghi-47-MB vừa tránh. Đóng dấu ở **cả hai** lối ra thành công của
`run_incremental`; giao diện lùi về `builtAtUnix` khi chưa có mốc, cho máy vừa nâng cấp.

### 3. Chỉ mục bị thay khi lớp phủ đang mở

Lớp xem trước bám theo `selected` — vừa bị đưa về 0 — nên nó ở lại và lặng lẽ chiếu một tệp người
dùng chưa từng chọn. Menu chuột phải thì tính lại `menuItems`, `hits.indexOf(hit)` thành -1, và mục
"Xem trước" biến mất khỏi menu **đang mở**: bốn mục co xuống ba ngay dưới con trỏ. Hai dòng.

Trước v1.0.6 chuyện này gần như không xảy ra vì chỉ mục làm mới mỗi ngày một lần.

### 4. Lớp xem trước giữ vết hỏng của tệp trước

`Preview.svelte` mang sẵn chú thích *"Reset per file, not per open"* — nhưng **không có dòng mã nào
làm việc đó**. Gặp một `.mkv` không giải mã được rồi bấm mũi tên là mọi tệp sau đó đều báo "Không
xem trước được định dạng này". Cùng kiểu hỏng như `armed` từng mắc: cơ chế sống trong lời văn chứ
không trong mã.

**Bản sửa đầu của tôi sai, và bộ kiểm thử bắt được.** `$effect` chạy cả lần đầu, mà lượt flush đầu
tiên xảy ra *sau* khi component dựng xong — nên một tệp hỏng ngay lúc mở bị chính effect ấy xoá mất
trạng thái hỏng. Sửa lại: chỉ đặt lại khi tệp **thật sự đổi**, `null` là "chưa chạy lần nào".

Để kiểm được nó phải đổi prop trên một component **đang gắn** — `mount()` trả về exports chứ không
phải proxy props, nên thêm `tests/runes.svelte.ts` (rune chỉ dùng được trong tệp `.svelte.ts`).

### Đột biến — mỗi phép đỏ đúng một bài

| Đột biến | Bài phải đỏ | Kết quả |
|---|---|---|
| Bỏ hẳn việc đặt lại trạng thái xem trước | "bước sang tệp khác thì QUÊN vết hỏng cũ" | ✅ đỏ đúng bài |
| Bỏ `preview = false; menu = null;` | TC-2.9b (cả hai vế) | ✅ **cả hai vế** đỏ với đúng thông điệp |

Phép thứ hai quan trọng hơn vẻ ngoài của nó: bài TC-2.9b có nhánh "bỏ qua nếu jsdom không mở được
lớp phủ", và một bài xanh nhờ bỏ qua thì vô dụng. Đột biến chứng minh cả hai vế **thật sự chạy**.

### Vòng trước-commit

`cargo test` **272 pass** · clippy **0 warning** · fmt sạch · `npm run check` **0 lỗi 0 cảnh báo**
/125 tệp · `npm test` **170/170**.

Cảnh báo `state_referenced_locally` xuất hiện một lúc rồi được sửa — Svelte nói đúng: khởi tạo biến
theo dõi bằng `hit.index` chỉ bắt được giá trị đầu.

### Nghiệm thu trên bản dựng thật

Dựng `npm run tauri build -- --no-bundle`, lái bằng chuột:

* Trạng thái 0 kết quả: **"Ổ trong máy: 11 phút trước · Ổ mạng: 11 giờ trước"**, phần cũ tô hổ phách.
* Chân cửa sổ: `ổ trong máy 22:30:02 · ổ mạng 11:23:05 28/8/2026`.
* `lastcheck.json` chưa tồn tại nên tuổi ổ cục bộ lùi về `builtAtUnix` — **đúng đường dự phòng đã
  thiết kế cho máy vừa nâng cấp**, và quan sát được đúng như vậy.

**Chưa nghiệm thu được:** việc đóng dấu `lastcheck.json` từ tiến trình `--index` thật, vì nó cần
quyền Administrator và bản đang cài trên máy được dựng trước khi module này tồn tại. Logic có 6 bài
đơn vị cộng 2 chốt đọc thẳng mã nguồn canh đúng điểm gọi.
