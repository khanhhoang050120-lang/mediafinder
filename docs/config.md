# XUNG ĐỘT CẤU HÌNH — MediaFinder
> **Thuộc file này:** Hai thiết lập loại trừ nhau, phiên bản thư viện đụng nhau, tuỳ chọn build sai — code không sai, cấu hình sai.
> **KHÔNG thuộc file này:** lỗi logic trong code, vấn đề tốc độ.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `CONF-004`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [CONF-001](#conf-001) | 🟠 | Xung đột phiên bản crate `windows` | P0 | ĐÃ SỬA |
| [CONF-002](#conf-002) | 🟡 | `tsconfig.node.json`: `composite` xung đột `noEmit` | P0 | ĐÃ SỬA |
| [CONF-003](#conf-003) | 🟡 | Terminal mở trước khi cài Rust không thấy `cargo` | P2 | ĐÃ SỬA |

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
