# XUNG ĐỘT CẤU HÌNH — MediaFinder
> **Thuộc file này:** Hai thiết lập loại trừ nhau, phiên bản thư viện đụng nhau, tuỳ chọn build sai — code không sai, cấu hình sai.
> **KHÔNG thuộc file này:** lỗi logic trong code, vấn đề tốc độ.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `CONF-005`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [CONF-001](#conf-001) | 🟠 | Xung đột phiên bản crate `windows` | P0 | ĐÃ SỬA |
| [CONF-002](#conf-002) | 🟡 | `tsconfig.node.json`: `composite` xung đột `noEmit` | P0 | ĐÃ SỬA |
| [CONF-003](#conf-003) | 🟡 | Terminal mở trước khi cài Rust không thấy `cargo` | P2 | ĐÃ SỬA |
| [CONF-004](#conf-004) | 🟡 | Tiến trình vite còn sót giữ port 1420 sau khi dừng dev | P3 | WORKAROUND |

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

## CONF-003 🟡 — Terminal mở trước khi cài Rust không thấy `cargo`

**Giai đoạn:** P2 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Người dùng chạy `npm run tauri dev` từ PowerShell của mình:

```
failed to run 'cargo metadata' command to get workspace directory:
failed to run command cargo metadata --no-deps --format-version 1: program not found
```

**Không phải lỗi dự án.** Kiểm chứng cho thấy mọi thứ đều đúng:

| Kiểm tra | Kết quả |
|---|---|
| `.cargo\bin` có trong PATH vĩnh viễn (User) không? | **CÓ** — `C:\Users\Padoma1\.cargo\bin` |
| `cargo.exe` có tồn tại và chạy được không? | **CÓ** — `cargo 1.98.0` |

**Nguyên nhân.** Rust được cài **giữa phiên làm việc**. Trên Windows, một tiến trình nhận bản sao
biến môi trường tại thời điểm khởi động và **không bao giờ thấy thay đổi sau đó**. Cửa sổ
PowerShell kia mở từ trước khi cài nên vẫn giữ PATH cũ.

**Bẫy dễ mắc.** Với terminal tích hợp trong VS Code, **mở tab mới là không đủ**. Tab mới kế thừa
môi trường từ chính tiến trình VS Code, mà VS Code lại kế thừa từ Explorer lúc nó khởi động.
Phải **đóng hẳn VS Code rồi mở lại**.

**Cách sửa.**

- Đúng và vĩnh viễn: khởi động lại VS Code (hoặc terminal).
- Tạm cho một cửa sổ: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`

**Vì sao ghi lại.** Sẽ tái diễn bất cứ khi nào mở lại một terminal cũ, và thông báo lỗi
(`program not found`) trông y hệt như Rust chưa được cài — dễ dẫn tới cài lại một cách vô ích.

**Ghi chú.** Phiên làm việc của Claude không dính lỗi này vì mọi lệnh đều thêm `.cargo/bin` vào
`PATH` ngay đầu lệnh. Đó cũng là lý do lỗi chỉ lộ ra khi người dùng tự chạy — một nhắc nhở rằng
"chạy được ở đây" không có nghĩa là "chạy được ở đó".

---

## CONF-004 🟡 — Tiến trình vite còn sót giữ port 1420 sau khi dừng dev

**Giai đoạn:** P3 · **Trạng thái:** WORKAROUND · **Ngày:** 2026-08-24

**Hiện tượng.** Chạy `npm run tauri dev` thất bại ngay:

```
Error: Port 1420 is already in use
    The "beforeDevCommand" terminated with a non-zero status code.
```

**Nguyên nhân.** Lần chạy dev trước đó đã bị dừng ở mức shell, nhưng **tiến trình con không bị
dọn theo**. `npm run tauri dev` sinh ra một cây tiến trình — npm → vite (node) → cargo →
mediafinder.exe — và dừng tiến trình cha không giết các tiến trình con. Node vẫn ôm port 1420.

Kiểm chứng: `Get-NetTCPConnection -LocalPort 1420` chỉ ra `PID=2104 node` khởi động lúc 09:17,
tức là từ lượt chạy ở giai đoạn P0.

**Vì sao `strictPort: true` khiến nó thành lỗi cứng.** `vite.config.ts` đặt `strictPort: true`
để vite không tự nhảy sang port khác. Đó là **chủ ý**: `tauri.conf.json` trỏ cứng
`devUrl: http://localhost:1420`, nên nếu vite âm thầm chuyển sang 1421 thì cửa sổ app sẽ mở ra
trắng trơn — một lỗi khó hiểu hơn nhiều so với thông báo "port đang bận".

**Cách xử lý.** Trước khi chạy lại dev, dọn tiến trình còn sót:

```powershell
Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue |
  ForEach-Object { Stop-Process -Id $_.OwningProcess -Force }
Get-Process mediafinder -ErrorAction SilentlyContinue | Stop-Process -Force
```

**Vì sao để `WORKAROUND` chứ không phải `ĐÃ SỬA`.** Đây là hành vi của công cụ dev, không phải
lỗi của sản phẩm. Sửa triệt để thì phải dùng job object hoặc `taskkill /T` mỗi lần dừng — chưa
đáng ở giai đoạn này, nhưng ghi lại để lần sau nhận ra ngay thay vì đi tìm nguyên nhân.

---

## CONF-005 🟡 — `cargo fmt` chưa từng chạy, nay lệch với toàn bộ mã nguồn

**Giai đoạn:** P8 · **Trạng thái:** CẦN QUYẾT ĐỊNH · **Ngày:** 2026-08-24

**Hiện tượng.** Chạy `cargo fmt --check` lần đầu ở P8: **51 điểm lệch trên 20 tệp**, trải khắp mọi
giai đoạn từ P1 tới P8. Không phải lỗi mới — vòng kiểm tra của dự án từ trước tới nay là
`cargo test` + `cargo clippy` + `npm run check`, chưa bao giờ có `cargo fmt`.

**Nguyên nhân.** Mã trong dự án được xuống dòng bằng tay để chú thích và biểu thức dài đọc dễ hơn.
`rustfmt` mặc định xếp lại theo `max_width = 100`, nên gần như tệp nào cũng lệch ở vài chỗ.

**Vì sao chưa sửa ở P8.** Chạy `cargo fmt` bây giờ sẽ sinh một diff khổng lồ **không liên quan gì
tới P8**, trộn lẫn vào commit cuối cùng của giai đoạn và làm hỏng khả năng truy vết `git blame` cho
toàn bộ mã đã viết từ P1. Đây là quyết định của chủ dự án, không phải việc nên tự làm kèm.

**Hai lựa chọn.**

| Cách | Đánh đổi |
|---|---|
| Chạy `cargo fmt` một lần, thêm vào vòng kiểm tra | Thống nhất về sau, nhưng một commit chạm 20 tệp và mất định dạng thủ công ở nhiều chỗ |
| Thêm `rustfmt.toml` nới `max_width`, rồi mới format | Diff nhỏ hơn nhiều, giữ được phần lớn cách xuống dòng hiện tại |

Cách thứ hai hợp với dự án này hơn: phần lớn điểm lệch là do độ rộng dòng, không phải do thụt lề
hay dấu ngoặc sai.
