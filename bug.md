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
| RISK-002 | ⚪ | Dự án chưa có version control | P0 | MỞ |

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
