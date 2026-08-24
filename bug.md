# NHẬT KÝ LỖI & XUNG ĐỘT — MediaFinder

> Ghi lại **mọi** bug, xung đột cấu hình, và rủi ro phát hiện trong quá trình phát triển.
> Sau mỗi giai đoạn phải chạy một lượt test và cập nhật file này.
>
> Liên quan: `PROGRESS.md` (tiến độ) · `README.md` (bất biến kiến trúc)

## Quy ước

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra, cần theo dõi)

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `KHÔNG SỬA`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| BUG-001 | 🟡 | `MainWindowHandle` bắt nhầm cửa sổ event-loop của tao | P0 | WORKAROUND |
| BUG-002 | 🟠 | Cửa sổ mở ở trạng thái minimize | P0 | CẦN XÁC MINH |
| BUG-003 | 🟡 | `SetForegroundWindow` bị chặn → chụp nhầm màn hình | P0 | ĐÃ SỬA |
| CONF-001 | 🟠 | Xung đột phiên bản crate `windows` (0.58 vs 0.61.3) | P0 | ĐÃ SỬA |
| CONF-002 | 🟡 | `tsconfig.node.json`: `composite` xung đột `noEmit` | P0 | ĐÃ SỬA |
| RISK-001 | ⚪ | `panic = "abort"` khiến panic trong IPC giết cả app | P0 | MỞ |
| RISK-002 | ⚪ | Dự án chưa có version control | P0 | ĐÃ SỬA |
| BUG-004 | 🔴 | `.ts` (TypeScript) bị phân loại thành **video** | P1 | ĐÃ SỬA |
| BUG-005 | 🟡 | Tiến độ quét báo trùng bản ghi cuối | P1 | ĐÃ SỬA |
| PERF-001 | 🟡 | Cấp phát `String` cho mỗi thành phần đường dẫn khi lọc | P1 | ĐÃ SỬA |
| CHECK-001 | ✅ | Nghi ngờ pha 2 loại nhầm 99,7% file trên C: | P1 | KHÔNG PHẢI LỖI |
| ISSUE-001 | 🟠 | Kết quả trên C: toàn tài nguyên công cụ, không phải media | P1 | CẦN QUYẾT ĐỊNH |

---

## BUG-001 🟡 — `MainWindowHandle` bắt nhầm cửa sổ event-loop của tao

**Giai đoạn:** P0 · **Trạng thái:** WORKAROUND · **Ngày:** 2026-08-24

**Hiện tượng.** Kiểm tra cửa sổ app bằng `.NET Process.MainWindowHandle` cho ra kết quả vô lý:
tiêu đề rỗng, kích thước `16x16` tại `(0,0)`.

**Nguyên nhân.** Tiến trình Tauri có **4 cửa sổ top-level**, và `MainWindowHandle` chọn nhầm:

```
hwnd=855616   vis=True   16x16    (0,0)          class='Tao Thread Event Target'   title=''
hwnd=1966312  vis=True   160x28   (-32000,-32000) class='Tauri Window'             title='MediaFinder'   <- cửa sổ thật
hwnd=5049608  vis=False  0x0                      class='MSCTFIME UI'
hwnd=3409892  vis=False  0x0                      class='IME'
```

`Tao Thread Event Target` là cửa sổ message-only mà event loop của tao dùng để nhận thông điệp.
Nó được tạo trước nên `MainWindowHandle` vớ phải nó.

**Ảnh hưởng.** Không ảnh hưởng sản phẩm. Nhưng **mọi kiểm tra tự động về cửa sổ ở các giai đoạn sau
sẽ đo nhầm đối tượng** nếu dùng `MainWindowHandle`.

**Cách xử lý.** Luôn dùng `EnumWindows` rồi lọc theo **class name `Tauri Window`**, không dùng
`MainWindowHandle`. Đã áp dụng và cho kết quả đúng.

---

## BUG-002 🟠 — Cửa sổ mở ở trạng thái minimize

**Giai đoạn:** P0 · **Trạng thái:** CẦN XÁC MINH · **Ngày:** 2026-08-24

**Hiện tượng.** Ngay sau khi `npm run tauri dev` khởi động xong, cửa sổ `Tauri Window` có
`IsIconic = True`, `GetWindowRect` trả `160x28` tại `(-32000,-32000)` — chữ ký kinh điển của
cửa sổ đang thu nhỏ. Log không có panic, tiến trình sống bình thường.

Sau khi gọi `ShowWindow(hwnd, SW_RESTORE)`: `IsIconic = False`, kích thước `916x659` tại `(502,190)` —
đúng như cấu hình `900x620` cộng viền cửa sổ. Nội dung render hoàn toàn đúng.

**Giả thuyết nguyên nhân.** App được khởi chạy từ một shell nền **không tương tác**, nên tiến trình
không có quyền foreground; Windows không cho cửa sổ mới hiện lên. Nhiều khả năng đây là **hệ quả của
cách tôi khởi chạy**, không phải lỗi của ứng dụng.

**Vì sao chưa đóng.** Chưa loại trừ được khả năng đây là bug thật. Nếu người dùng bấm đúp vào exe mà
app mở ra ở trạng thái minimize thì đó là lỗi nghiêm trọng về trải nghiệm.

**Cách xác minh (cần người dùng làm).**
Chạy `npm run tauri dev` từ terminal thông thường và quan sát: cửa sổ MediaFinder có **tự hiện lên
trước mặt** không, hay nằm im dưới thanh taskbar?

- Nếu **tự hiện lên** → đóng bug này, ghi `KHÔNG SỬA (do môi trường chạy nền)`.
- Nếu **vẫn minimize** → là bug thật; sửa bằng cách gọi `window.set_focus()` / `unminimize()` trong
  `setup()` của Tauri, hoặc đặt lại thuộc tính cửa sổ trong `tauri.conf.json`.

---

## BUG-003 🟡 — `SetForegroundWindow` bị chặn → chụp nhầm màn hình

**Giai đoạn:** P0 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Chụp ảnh cửa sổ bằng `SetForegroundWindow` + `Graphics.CopyFromScreen` cho ra ảnh của
**ứng dụng khác** đang che phía trên, không phải MediaFinder.

**Nguyên nhân.** Windows có cơ chế foreground lock: tiến trình nền không được phép cướp focus.
`SetForegroundWindow` trả về mà không làm gì, nên `CopyFromScreen` chụp đúng toạ độ nhưng lớp trên cùng
là cửa sổ khác.

**Ảnh hưởng phụ.** Ảnh chụp lẫn nội dung màn hình riêng tư của người dùng. Ảnh đã được xoá ngay.

**Cách sửa.** Dùng `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT /* = 2 */)` — API này yêu cầu chính
cửa sổ đó tự vẽ nội dung vào device context, **không phụ thuộc z-order và không đụng tới màn hình**.
Cho ảnh đúng, và an toàn về riêng tư. Cờ `PW_RENDERFULLCONTENT` là bắt buộc với WebView2 vì nó
render qua DirectComposition.

**Ghi nhớ cho các giai đoạn sau:** mọi lần chụp cửa sổ đều phải dùng `PrintWindow`, tuyệt đối không
dùng `CopyFromScreen`.

---

## CONF-001 🟠 — Xung đột phiên bản crate `windows`

**Giai đoạn:** P0 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Log `cargo check` cho thấy biên dịch **hai bản** của crate `windows`:

```
Checking windows-core v0.61.2      <- Tauri 2.11 / webview2-com kéo về
Checking windows v0.61.3
Checking windows-core v0.58.0      <- Cargo.toml của mình khai báo
Checking windows v0.58.0
```

**Ảnh hưởng.** Tốn thời gian build gấp đôi cho phần Win32; và nguy hiểm hơn — nếu sau này cần truyền
một type Win32 (ví dụ `HWND`) giữa code của mình và API của Tauri thì **hai bản là hai type khác nhau**,
trình biên dịch sẽ từ chối dù tên giống hệt.

**Cách sửa.** Bump `windows` từ `0.58` lên `0.61` trong `src-tauri/Cargo.toml`.

**Vì sao sửa ngay.** Thời điểm phát hiện chưa có dòng code Win32 nào được viết → bump là miễn phí.
Để đến P1/P5 mới bump thì phải sửa lại toàn bộ code đã viết (API giữa 0.58 và 0.61 có thay đổi).

**Kết quả.** `cargo check --all-targets` exit 0. Thời gian check giảm **2m35s → 29.28s**.

---

## CONF-002 🟡 — `tsconfig.node.json`: `composite` xung đột `noEmit`

**Giai đoạn:** P0 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** `npm run check` báo:
`WARNING "tsconfig.json" 23:18 "Referenced project 'tsconfig.node.json' may not disable emit."`

**Nguyên nhân.** `"composite": true` bắt buộc project phải sinh file khai báo, nhưng `"noEmit": true`
lại cấm sinh bất cứ thứ gì. Hai tuỳ chọn loại trừ nhau.

**Cách sửa.** Thay `noEmit` bằng `declaration: true` + `emitDeclarationOnly: true` +
`outDir: "./node_modules/.tmp/tsconfig-node"`. Thoả mãn `composite` mà không sinh rác vào dự án.

**Kết quả.** `npm run check` → 0 lỗi, 0 warning.

---

## RISK-001 ⚪ — `panic = "abort"` khiến panic trong IPC giết cả app

**Giai đoạn:** P0 · **Trạng thái:** MỞ · **Ngày:** 2026-08-24

**Mô tả.** `src-tauri/Cargo.toml` đặt `panic = "abort"` trong `[profile.release]` để binary nhỏ và
nhanh hơn. Hệ quả: không có stack unwinding, nên `catch_unwind` vô hiệu.

**Rủi ro.** Khi có Tauri command thật ở P3, một panic trong handler (ví dụ index truy cập ngoài phạm vi,
`unwrap()` trên `None`) sẽ **giết toàn bộ ứng dụng** thay vì trả lỗi về frontend. Với bản release
người dùng cuối, app sẽ biến mất không thông báo.

**Chưa gây hại vì.** P0 chưa có command nào; hiện chỉ chạy bản debug (`panic = unwind`).

**Việc cần làm ở P3.** Quyết định một trong hai:
1. Bỏ `panic = "abort"`, chấp nhận binary lớn hơn, đổi lấy khả năng phục hồi.
2. Giữ `abort`, nhưng **cấm tuyệt đối `unwrap()` / `expect()` / index trực tiếp trong mọi command
   handler** — bắt buộc trả `Result<_, String>`.

Khuyến nghị hiện tại: **phương án 2**, vì code index/search đằng nào cũng nên trả `Result`.
Cần kiểm chứng lại bằng test khi có command thật.

---

## RISK-002 ⚪ — Dự án chưa có version control

**Giai đoạn:** P0 · **Trạng thái:** MỞ · **Ngày:** 2026-08-24

**Mô tả.** `d:\tool_finding` **không phải git repository**. File `.gitignore` đã được tạo nhưng hiện
không có tác dụng gì.

**Rủi ro.** Kế hoạch có 9 giai đoạn. Không có VCS thì:
- Không thể xem đã đổi gì giữa các giai đoạn.
- Một thay đổi ở P5 làm hỏng P2 thì không có cách nào quay lại.
- Không có điểm khôi phục an toàn để thử nghiệm.

**Đề xuất.** `git init` + commit một mốc sau mỗi giai đoạn hoàn thành.
**Chưa thực hiện** vì khởi tạo repo là quyết định của người dùng, cần được đồng ý trước.

---

## BUG-004 🔴 — `.ts` (TypeScript) bị phân loại thành video

**Giai đoạn:** P1 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Lượt quét thật ổ C: trả về:

```
[video] C:\Users\Padoma1\Downloads\cal-ai-tutorial-master\...\src\trigger\analyze-meal.ts
```

**Nguyên nhân.** `ts` nằm trong bảng phần mở rộng video vì nó là đuôi hợp lệ của
**MPEG Transport Stream**. Nhưng nó cũng là đuôi của **TypeScript** — và trên bất kỳ máy nào
có mã nguồn, số file TypeScript nhiều hơn transport stream hàng nghìn lần.

**Mức độ nghiêm trọng.** Nặng. Không chỉ sai về phân loại: nó bơm hàng nghìn file rác vào index,
làm loãng kết quả xếp hạng và tốn RAM. Người dùng gõ tên video sẽ nhận về lẫn lộn file mã nguồn.

**Cách sửa.** Bỏ `ts` khỏi bảng. Video máy quay và Blu-ray vẫn được bao phủ bởi `m2ts` và `mts` —
hai đuôi này không nhập nhằng với thứ gì.

**Chống tái phát.** Thêm test `source_code_extensions_are_never_media` kiểm tra 18 đuôi mã nguồn
và cấu hình (`ts` `tsx` `js` `json` `md` `toml` `py` `go` `rs` …) đều **không** được coi là media.
Ai thêm đuôi mới chạm phải nhóm này sẽ bị test chặn ngay.

---

## BUG-005 🟡 — Tiến độ quét báo trùng bản ghi cuối

**Giai đoạn:** P1 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Log quét ổ C: kết thúc bằng hai dòng giống hệt nhau:

```
02:42:10.480354Z  … đã đọc 3559309 bản ghi
02:42:10.480402Z  … đã đọc 3559309 bản ghi
```

**Nguyên nhân.** Sau khi thoát vòng lặp, hàm gọi `on_progress` lần cuối để chốt tổng số, nhưng
không kiểm tra lần báo trong vòng lặp có vừa báo đúng con số đó chưa.

**Ảnh hưởng.** Nhẹ ở P1 (thừa một dòng log). Nhưng ở P4 mỗi lần gọi là **một sự kiện IPC** đẩy
lên giao diện — thanh tiến độ sẽ nhấp nháy ở mốc 100%.

**Cách sửa.** Chỉ gọi lần cuối khi `records_seen != last_reported`.

**Phát hiện kèm theo (nặng hơn).** Trong lúc sửa còn tìm ra một lỗi logic ở cùng đoạn code, chưa
từng chạy nên chưa lộ ra: biến đếm throttle bị gán chồng lên chính nó
(`since_progress = records_seen - since_progress`), khiến ngưỡng 20.000 bản ghi tính sai hoàn
toàn từ vòng lặp thứ hai trở đi. Đã viết lại bằng mốc `last_reported` tường minh.
Bug này **không thể bắt bằng unit test** vì hàm cần volume thật — chỉ đọc lại code mới thấy.

---

## PERF-001 🟡 — Cấp phát `String` cho mỗi thành phần đường dẫn khi lọc

**Giai đoạn:** P1 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Bộ lọc thư mục cấm dùng `HashSet<String>`, so sánh bằng
`excluded.contains(&node.name.to_lowercase())`.

**Vấn đề.** `to_lowercase()` **cấp phát một `String` mới** cho mỗi thành phần đường dẫn được xét.
Trên ổ C: với hơn một triệu thư mục, đó là hàng trăm nghìn lần cấp phát rồi vứt đi ngay.

**Cách sửa.** Đổi sang `Vec<String>` + `eq_ignore_ascii_case` — không cấp phát. Danh sách chỉ 15
mục nên quét tuyến tính còn nhanh hơn băm, và mọi tên trong danh sách đều là ASCII nên so sánh
ASCII-insensitive đúng ngữ nghĩa.

---

## CHECK-001 ✅ — Nghi ngờ pha 2 loại nhầm 99,7% file trên C: → KHÔNG PHẢI LỖI

**Giai đoạn:** P1 · **Trạng thái:** KHÔNG PHẢI LỖI (đã kiểm chứng) · **Ngày:** 2026-08-24

**Nghi ngờ ban đầu.** Thống kê quét ổ C: trông rất đáng ngờ:

```
pha 1: 3.559.309 bản ghi → giữ 872.803 tệp media
pha 2: giữ 2.437 / loại 870.366 (thư mục cấm)   ← loại 99,7%
```

Loại bỏ 99,7% là con số đủ bất thường để phải nghi ngờ logic loại trừ khớp quá rộng.

**Cách kiểm chứng.** Đếm độc lập bằng PowerShell `Get-ChildItem -Recurse`, hoàn toàn không dùng
lại code của dự án, trên các thư mục **không** nằm trong danh sách cấm:

| Thư mục | Trong danh sách cấm? | PowerShell đếm được |
|---|---|---|
| `C:\Users\Padoma1\.gradle` | không | 1.119 |
| `C:\Users\Padoma1\.rustup` | không | 127 |
| `C:\Users\Padoma1\Pictures` | không | 168 |
| **Cộng (phải được GIỮ)** | | **1.414** |
| `C:\Users\Padoma1\AppData` | **có** | **829.251** |

**Kết luận — khớp cả hai chiều.**

*Chiều giữ lại:* 1.414 trong tổng 2.437 mà scanner giữ đến từ ba thư mục trên; phần còn lại từ
`MSI`, `Downloads`, `.vscode`, `.antigravity-ide`, `.cache`, `Users\Public`.

*Chiều loại bỏ:* riêng `AppData` đã có **829.251** file media — chiếm 95% trong số 870.366 file
bị loại. Phần còn lại (~41.000) đến từ `Windows`, `Program Files`, `ProgramData`.

**Logic loại trừ hoạt động chính xác.** Trên máy có nhiều công cụ lập trình, `AppData` (cache
trình duyệt, ứng dụng Electron, cache npm/pip/gradle) chứa gần một triệu ảnh là hoàn toàn bình
thường — và đó đúng là thứ cần loại bỏ khỏi một công cụ tìm media.

**Bài học.** Một con số trông "sai" chưa chắc là lỗi — nhưng cũng không được cho qua chỉ vì code
trông có vẻ đúng. Phải đo bằng một công cụ độc lập.

---

## ISSUE-001 🟠 — Kết quả trên C: toàn tài nguyên công cụ, không phải media người dùng

**Giai đoạn:** P1 · **Trạng thái:** CẦN QUYẾT ĐỊNH · **Ngày:** 2026-08-24

**Hiện tượng.** Trong 20 đường dẫn mẫu lấy trải đều trên ổ C:, phần lớn là tài nguyên của công cụ
lập trình chứ không phải media người dùng:

```
C:\Users\Padoma1\.gradle\caches\...\res\drawable-xxhdpi-v4\ic_call_answer_low.png
C:\Users\Padoma1\.rustup\toolchains\...\doc\rust\html\cargo\images\...png
C:\Users\Padoma1\.vscode\extensions\ms-vscode.powershell-...\media\PowerShell_Icon.png
C:\Users\Padoma1\.antigravity-ide\extensions\...\doc-assets\complete.png
```

**Vì sao đáng quan tâm.** Đây không phải lỗi kỹ thuật — chúng đúng là file ảnh. Nhưng sản phẩm
này là **công cụ tìm media**, và icon 16×16 của một extension VS Code thì không bao giờ là thứ
người dùng đi tìm. Chúng làm loãng xếp hạng ở P2 và tốn công sinh thumbnail ở P5.

**Hai hướng xử lý.**

1. **Mở rộng danh sách cấm** thêm thư mục công cụ/cache: `.gradle` `.rustup` `.cargo` `.npm`
   `.nuget` `.cache` `.vscode` `.git` `__pycache__` `site-packages` `vendor` `target` `dist`
   `build`. → Kết quả sạch hơn hẳn. Rủi ro: media thật để trong thư mục tên `build` sẽ mất.
2. **Bỏ `svg` và `ico`** khỏi bảng phần mở rộng. → Hai đuôi này gần như luôn là tài nguyên giao
   diện ứng dụng, không phải media người dùng.

**Chưa tự quyết** vì đây là quyết định về sản phẩm chứ không phải sửa lỗi — nó thay đổi thứ
người dùng tìm thấy được.

---

## Nhật ký test

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
