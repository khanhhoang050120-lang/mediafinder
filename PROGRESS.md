# TIẾN ĐỘ THỰC HIỆN — MediaFinder

> **File này là nguồn sự thật duy nhất về trạng thái dự án.**
> Kế hoạch chi tiết: `C:\Users\Padoma1\.claude\plans\b-n-h-y-c-docs-transient-plum.md`
> Bất biến kiến trúc: `README.md` · Sổ ghi vấn đề: [`docs/`](./docs/)

## Quy tắc làm việc

1. **Không nhảy cóc giai đoạn.** Xong P(n) → cập nhật file này → chờ xác nhận → mới sang P(n+1).
2. **Chỉ tick `[x]` khi đã CHẠY và THẤY kết quả.** Viết code xong nhưng chưa chạy = `[~]`.
3. Mỗi tiêu chí nghiệm thu phải ghi **bằng chứng thực tế** vào Nhật ký kiểm chứng cuối file.
4. Phải làm khác kế hoạch → ghi vào mục "Sai lệch so với kế hoạch" kèm lý do.
5. **Xong mỗi giai đoạn phải đóng vai tester**: chạy một lượt test chủ động đi tìm lỗi
   (không chỉ chạy cho có), sửa những gì tìm được, rồi ghi **toàn bộ** phát hiện vào
   thư mục [`docs/`](./docs/) — mỗi loại vấn đề một file, xem bảng "ghi vào file nào"
   ở [`docs/README.md`](./docs/README.md). Kết quả lượt test ghi vào
   [`docs/test-log.md`](./docs/test-log.md).
   Giai đoạn chỉ được coi là xong khi lượt test đã chạy và `docs/` đã cập nhật.

Ký hiệu: `[ ]` chưa làm · `[~]` đã viết chưa kiểm chứng · `[x]` đã kiểm chứng chạy được · `[!]` đang vướng

**Vòng kiểm tra trước mỗi commit:** `cargo test` · `cargo clippy --all-targets` · `cargo fmt --check`
· `npm run check`. Định dạng theo mặc định rustfmt, không có `rustfmt.toml` — đã đo, không cấu hình
nào khớp tốt hơn ([CONF-005](docs/config.md#conf-005)).

---

## Bảng tổng quan

| GĐ | Nội dung | Trạng thái |
|----|----------|-----------|
| **P0** | Scaffold + kiểm tra toolchain | ✅ **XONG** — 27/27, test 8/8 pass |
| **P1** | Enumerator NTFS (USN) | ✅ **XONG** — 29/29 test, quét thật 4,1 triệu bản ghi |
| **P2** | Index + fold + search + bench | ✅ **XONG** — 72/72 test, bench 3,01 ms worst case |
| **P3** | Nối Tauri + UI tối giản | ✅ **XONG** — 96/96 test, mở tệp + mở thư mục đã kiểm chứng |
| **P4** | Cache trên đĩa + luồng elevate | ✅ **XONG** — 100/100 test, người dùng xác nhận chạy được |
| **P5** | Thumbnail + lưới ảo hoá | ✅ **XONG** — 108 test, 5.000 kết quả = 118 node |
| **P6** | Enrichment metadata + lọc | ✅ **XONG** — 118 test, lọc 1080p chạy 9,1 ms |
| **P7** | Tìm file trùng | ✅ **XONG** — 124 test, tìm ra 520,7 GB trùng lặp |
| **P8** | Hoàn thiện (hotkey, bàn phím, USN realtime) | ✅ **XONG** — 126 test, kiểm chứng trên bản release |
| **P9** | Cập nhật gia tăng qua USN journal | ✅ **giai đoạn 1 XONG** — 0,45s thay cho 13,2s, kiểm chứng trên máy thật |
| **P10** | Quét ổ mạng / NAS theo yêu cầu | ✅ **XONG** — 313.945 tệp trên NAS, 4,5 phút, có nút riêng |
| **P11** | Chạy nền ở khay hệ thống | ✅ **XONG** — đóng cửa sổ thì ẩn, chỉ Thoát mới tắt hẳn |
| **P12** | Kéo tệp ra ngoài (CapCut, Explorer, web) | ✅ **XONG** — `CF_HDROP` thật, đã kiểm chứng đầu bên kia |
| **BT** | Bảo trì sau phát hành | 🟢 **đang chạy** — ghi mọi vấn đề thực tế vào [`docs/`](./docs/) |

---

## P0 — Scaffold + kiểm tra toolchain ✅

**Tiêu chí nghiệm thu:** `npm run tauri dev` mở được cửa sổ ứng dụng.

### Môi trường
- [x] Node.js — `v24.18.0`
- [x] npm — `11.12.1`
- [x] MSVC Build Tools — `2022 (17.14.37502)` + `2026 (18.4.11626)`
- [x] Windows SDK — `10.0.26100.0`
- [x] WebView2 Runtime — `151.0.4129.101`
- [x] Rust — `1.98.0 (88d9e12ae)`, toolchain `stable-x86_64-pc-windows-msvc`

### Frontend
- [x] `package.json` / `vite.config.ts` / `svelte.config.js` / `tsconfig*.json`
- [x] `index.html`, `src/main.ts`, `src/App.svelte`, `src/app.css`
- [x] `npm install` — 51 gói
- [x] `npm run check` — 0 lỗi, 0 warning
- [x] `npm run build` — dist sinh ra, JS 28.37 kB

### Backend
- [x] `Cargo.toml` — lib + bin target (lib bắt buộc để criterion bench import được)
- [x] `build.rs` — manifest Win32 `asInvoker` + `longPathAware` + `PerMonitorV2` + UTF-8
- [x] `tauri.conf.json` (Tauri v2) + `capabilities/default.json`
- [x] `src/main.rs` — rẽ nhánh 2 chế độ (GUI / `--index`)
- [x] `src/lib.rs` — khai báo module + `init_tracing` + `run_gui` + `run_indexer`
- [x] Stub module: `ntfs/` `index/` `media/` `ipc/` `state.rs`
- [x] `ntfs::RawRecord` — seam không chứa type Win32 (để test được trên CI)
- [x] Icon: `32x32` `128x128` `128x128@2x` `icon.png` `icon.ico`
- [x] `cargo check --all-targets` — exit 0, không lỗi, không warning

### Nghiệm thu
- [x] **`npm run tauri dev` mở được cửa sổ** — `916x659`, class `Tauri Window`, title `MediaFinder`,
      render đúng: title bar + icon, placeholder tiếng Việt đủ dấu, dark theme, focus ring

### Lượt test P0 (chi tiết ở [`docs/`](./docs/))
- [x] `cargo test` — pass
- [x] `cargo clippy --all-targets` — **sạch, 0 warning**
- [x] Dispatch `--index` — vào đúng indexer mode, không mở GUI
- [x] Manifest nhúng trong exe — có `asInvoker`/`longPathAware`, **không** có `requireAdministrator`
- [x] Phát hiện 3 bug + 2 xung đột cấu hình → đã sửa 3, workaround 1, còn `BUG-002` cần xác minh tay

---

## P1 — Enumerator NTFS (USN) ✅

**Tiêu chí nghiệm thu:** `--index --dry-run` in ra số record + 20 path mẫu từ ổ C:;
**không có tên 8.3 trùng lặp**; path resolve đúng ở độ sâu ≥5 cấp.

- [x] `ntfs/volume.rs` — liệt kê volume, lọc đúng NTFS, mở handle `\\.\X:`
- [x] `ntfs/volume.rs` — phát hiện USN Journal bị tắt, báo lỗi rõ ràng
- [x] `ntfs/usn_enum.rs` — vòng lặp `FSCTL_ENUM_USN_DATA` → `Vec<RawRecord>` (PHA 1)
- [x] `ntfs/usn_enum.rs` — lọc phần mở rộng ngay lúc đọc (thứ DUY NHẤT lọc được ở pha 1)
- [x] `ntfs/tree.rs` — dựng map FRN→record, resolve path ngược lên gốc (PHA 2)
- [x] `ntfs/tree.rs` — chặn vòng lặp: giới hạn độ sâu + tập `visited`
- [x] `ntfs/tree.rs` — record mồ côi (không thấy cha) — KHÔNG panic
- [x] `ntfs/tree.rs` — bộ lọc thư mục loại trừ (Windows, AppData, $Recycle.Bin, ProgramData…)
- [x] Unit test `tree.rs` bằng `RawRecord` tổng hợp — chạy không cần Admin/ổ NTFS
- [x] Golden test: cây sâu ≥5 cấp, tên Unicode, record mồ côi, vòng lặp junction
- [x] Cờ `--dry-run` in thống kê + path mẫu
- [x] Test parser nhị phân: bản ghi hỏng, độ dài 0, độ dài vượt buffer, sai phiên bản
- [x] Xử lý lỗi thiếu quyền Admin — báo rõ, đi tiếp ổ khác, không sập tiến trình
- [x] Cảnh báo ổ non-NTFS — phát hiện đúng `G: (FAT32)`
- [x] Chạy thật: C: **3.559.309 bản ghi / 18,5s**, D: **530.731 / 20,4s** — 0 bản ghi hỏng, 0 mồ côi, 0 vòng lặp
- [x] Đối chiếu chéo bằng PowerShell — số liệu khớp cả chiều giữ lẫn chiều loại (xem `CHECK-001`)

### Lượt test P1 (chi tiết ở [`docs/`](./docs/))
- [x] 13 mục test → 11 pass ngay, 2 mục **tìm ra lỗi** và đã sửa
- [x] `BUG-004` 🔴 `.ts` (TypeScript) bị phân loại thành video — đã sửa + thêm test chống tái phát
- [x] `BUG-005` 🟡 tiến độ báo trùng, kèm lỗi logic throttle nặng hơn chưa từng chạy — đã sửa
- [x] `PERF-001` 🟡 cấp phát `String` mỗi thành phần đường dẫn — đã sửa
- [x] `CHECK-001` ✅ nghi ngờ loại nhầm 99,7% → kiểm chứng độc lập, **không phải lỗi**
- [x] `ISSUE-001` 🟠 kết quả C: toàn tài nguyên công cụ → đã giải quyết ở P2 bằng `skip_dot_directories`

⚠️ **Bẫy đã biết:** đừng lọc thư mục ở pha 1 — bất khả thi, record chưa có path.
⚠️ **Bẫy đã biết:** nếu đổi sang parse `$MFT` thô thì phải lọc namespace tên 8.3, nếu không mỗi file ra 2 lần.

---

## P2 — Index + fold + search ✅

**Tiêu chí nghiệm thu:** `fold("Tiếng Việt Đà Nẵng") == "tieng viet da nang"`;
bench 500k entry **p99 < 20ms**; `avatar` xếp `Avatar.mkv` trên `my_avatar_backup_2019.mkv`.

- [x] `index/model.rs` — `Span`, `MediaKind`, `Index` (Struct-of-Arrays + string arena)
- [x] `index/model.rs` — builder `Vec<RawRecord>` → `Index`, dedupe bảng thư mục
- [x] `index/fold.rs` — NFD → bỏ combining marks → `đ`/`Đ`→`d` → lowercase
- [x] `index/fold.rs` — **unit test tiếng Việt**: `đ Đ ơ ư ế ự ằ ỗ`, lẫn ASCII, chuỗi rỗng
- [x] `index/search.rs` — tách token, `memmem::Finder` dựng sẵn mỗi token
- [x] `index/search.rs` — AND đa token
- [x] `index/search.rs` — chấm điểm 4 bậc (exact / prefix / biên từ / substring) + thưởng
- [x] `index/search.rs` — top-K min-heap cục bộ mỗi thread, merge rồi sort
- [x] `index/search.rs` — huỷ sớm theo generation counter
- [x] `benches/search.rs` + khai báo `[[bench]]` trong Cargo.toml
- [x] `index/fold.rs` — ghép lại NFC ở cuối (NFD tách Hangul thành Jamo — xem `BUG-006`)
- [x] `index/search.rs` — thứ tự **tất định** `(score, index)`, có test chạy 12 lần liên tiếp
- [x] Nối index vào indexer: `--dry-run` dựng `Index` thật và báo dung lượng RAM
- [x] Bench 500k entry — worst case **3,01 ms**, mục tiêu 20 ms → **dư 6,6 lần**
- [x] **Quyết định dựa trên số đo: GIỮ rayon** — đo trên 12 nhân, song song nhanh hơn 2,6–4,5×
- [x] Tối ưu theo số đo: `select_nth_unstable_by` trước khi sort → **−39,5%** ở `limit=5000`

### Số liệu bench (500k entry, 12 nhân)

| Truy vấn | Song song | Đơn luồng | Rayon nhanh hơn |
|---|---|---|---|
| `avatar 1080p` | 0,99 ms | 4,83 ms | 4,5x |
| `family trip 2024` | 1,10 ms | 5,18 ms | 4,5x |
| `holiday` | 1,28 ms | 5,84 ms | 2,7x |
| `tieng viet` | 1,37 ms | 6,29 ms | 2,7x |
| `a` (worst case) | **3,01 ms** | 14,76 ms | 2,6x |

Dựng index 500k: **183 ms**. Với thư viện thật 124k file thì khoảng 45 ms — không đáng kể
so với 20 giây quét MFT.

### Kiểm chứng trên dữ liệu thật (117.123 tệp, index 7,6 MB)

Gõ **không dấu**, dữ liệu **có dấu**:

| Truy vấn | Tìm ra | Điểm |
|---|---|---|
| `nhac nen` | `…\tây ban nha\`**`nhạc nền`**`.mp3` | 1447 |
| `nhac nen` | `…\HAN QUOC\`**`nhạc nền`**` hàn.MP3` | 1446 |
| `nang dong` | `…\NHẠC NỀN\Nhạc\`**`Năng Động`**`\SLPSTRM - Fireflies.mp3` | 545 |
| `bai` | `…\DS3-BÀI 10\materials\audio\`**`bài 10`**`.mp3` | 848 |
| `hung` | `…\`**`hung`**`arian-goulash-soup…mp4` | 824 |

Thang điểm đúng thiết kế: khớp trọn tên tệp 1447 · khớp đầu tên 824–848 ·
khớp **tên thư mục** 445–545 (luôn xếp dưới).

### Sai lệch có chủ ý so với đặc tả gốc

Đặc tả mục 3.3 quy định chỉ tìm trong **tên tệp**. Dữ liệu thật chứng minh yêu cầu đó sai —
xem [`SPEC-001`](./docs/spec.md#spec-001). Đã đổi sang tìm cả đường dẫn thư mục, với điểm thấp hơn.


---

## P3 — Nối Tauri + UI tối giản ✅

**Tiêu chí nghiệm thu:** gõ phím → thấy kết quả; Enter mở file; Ctrl+Enter mở Explorer highlight.

- [x] `state.rs` — `ArcSwap<Index>` + `AtomicU64` generation
- [x] `ipc/commands.rs` — `search(query, filters)` → kết quả đã xếp hạng
- [x] `ipc/commands.rs` — `open_file(path)` qua `ShellExecuteW`
- [x] `ipc/commands.rs` — `reveal_in_explorer(path)` qua `SHOpenFolderAndSelectItems`
- [x] `ipc/commands.rs` — `index_status()`
- [x] `src/lib/search.ts` — coalesce 30ms + bỏ response cũ theo id
- [x] `src/App.svelte` — nối IPC, hiện danh sách kết quả
- [x] Xác minh **không giữ lock** khi search — `ArcSwap::load_full()` clone `Arc` rồi nhả ngay;
      có test `a_snapshot_survives_the_index_being_replaced_underneath_it`

### Bổ sung theo yêu cầu người dùng (2026-08-24)

Kế hoạch gốc chỉ có phím tắt `Ctrl+Enter` để mở thư mục chứa tệp. Đó là thiết kế tồi cho một
thao tác dùng thường xuyên — bắt người dùng phải nhớ phím tắt. Bổ sung menu chuột phải:

- [x] `src/lib/ContextMenu.svelte` — menu chuột phải, dựng theo phong cách Windows 11
- [x] Mục **"Mở tệp"** — mở bằng ứng dụng mặc định
- [x] Mục **"Mở thư mục chứa tệp"** — mở Explorer và **bôi đen đúng tệp**
- [x] Mục **"Sao chép đường dẫn"**
- [x] Chặn menu ngữ cảnh mặc định của trình duyệt
- [x] Đóng menu khi bấm ra ngoài / nhấn `Esc`; menu không tràn khỏi cửa sổ
- [x] Nháy đúp vào kết quả → mở tệp (song song với `Enter`)
- [x] Điều hướng bàn phím `↑ ↓ Enter Esc` (kéo sớm từ P8 vì nếu không thì không dùng được)

### Xử lý `RISK-001` (đến hạn ở giai đoạn này)

- [x] Quyết định về `panic = "abort"` → **BỎ** (xem `RISK-001`) — khi đã có Tauri command thật
- [x] Mọi command trả `Result<_, String>`, không `unwrap()` trên dữ liệu từ frontend

### Kéo sớm từ P4 (bắt buộc, nếu không P3 không kiểm chứng được)

`index/persist.rs` phải làm sớm: GUI không có cách nào lấy được index để tìm kiếm nếu chưa có
cache trên đĩa. Chỉ kéo phần **lưu/nạp**; luồng elevate + `progress.json` vẫn ở P4.

- [x] `index/persist.rs` — bincode lưu/nạp, ghi nguyên tử qua .tmp + rename `%LOCALAPPDATA%\MediaFinder\`
- [x] Indexer ghi cache khi chạy không có `--dry-run`
- [x] GUI nạp cache lúc khởi động, báo rõ khi chưa có cache

### Kiểm chứng P3 trên dữ liệu thật

| Hạng mục | Bằng chứng |
|---|---|
| Cache ghi được | `index.bin` 8.331.574 byte, 117.124 tệp |
| GUI nạp cache **không UAC** | ~300ms, log `nạp cache: 117124 tệp, 4196 thư mục` |
| Gõ phím → ra kết quả | `nhac nen` → **55 kết quả trong 4,6 ms** |
| Tìm không dấu qua toàn bộ luồng | ra `nhạc nền.mp3`, `nhac nen tho.MP3`, `nhạc nền hàn.MP3` |
| Xếp hạng hiển thị | khớp tên tệp lên đầu, khớp tên thư mục `NHẠC NỀN` xếp dưới |
| **Mở thư mục chứa tệp** | Explorer mở đúng thư mục |
| **Tệp được bôi đen** | `Shell.Application` xác nhận `SelectedItems()` đúng tệp |
| Tên tệp có dấu phẩy | hoạt động — ca làm hỏng `explorer.exe /select` |

### Sửa sau khi người dùng thử nghiệm (2026-08-24)

Người dùng báo: dán nguyên tiêu đề video thì 0 kết quả, dù gõ ngắn hơn lại tìm ra.
Xem [`SPEC-002`](./docs/spec.md#spec-002).

- [x] Tách token theo **mọi ký tự không phải chữ-số**, không chỉ khoảng trắng
- [x] Tự lùi về khớp một phần khi không tệp nào khớp đủ, xếp hạng theo số từ khớp
- [x] Chỉ giữ nhóm **khớp nhiều nhất** — thử lần đầu ra 173 kết quả, 171 là rác
- [x] Ba ranh giới chống nới lỏng quá tay: tối thiểu 3 từ mới nới · sàn một nửa · chỉ nhóm tốt nhất
- [x] Giao diện báo rõ khớp một phần: băng thông báo + huy hiệu `6/9` mỗi dòng
- [x] Hồi quy: `nhac nen` 55 kết quả, `The anglerfish` 10 kết quả — không đổi

Cách kiểm chứng giao diện: UI Automation không thấy nội dung WebView2 từ cửa sổ gốc, phải tìm
cửa sổ con class `WRY_WEBVIEW` rồi `FromHandle` trên đó. `ValuePattern.SetValue` kích hoạt đúng
sự kiện `input` của Svelte nên kiểm chứng được cả luồng mà không cần chuột.

---

## P4 — Cache trên đĩa + luồng elevate ✅

**Tiêu chí nghiệm thu:** mở app lần 2 **không có UAC**, kết quả < 500ms;
nút "Quét lại" đẩy UAC đúng một lần.

- [ ] `index/persist.rs` — bincode save/load vào `%LOCALAPPDATA%\MediaFinder\`
- [x] `index/persist.rs` — lưu `last_usn` + serial volume; kiểm tra phiên bản schema (làm ở P3)
- [x] `ipc/elevate.rs` — `ShellExecuteExW(verb="runas", "--index")` + handle tiến trình con
- [x] `ipc/elevate.rs` — bắt `ERROR_CANCELLED (1223)`, diễn đạt là **câu trả lời** chứ không phải lỗi
- [x] `run_indexer()` — ghi `progress.json`, throttle 10 Hz, ghi nguyên tử qua .tmp + rename
- [x] GUI poll mỗi 250ms → thanh tiến độ + nút "Quét lại"
- [x] Reload cache + `ArcSwap::store` sau khi indexer xong
- [x] Cờ `scanning` trong `AppState` — chặn hai lượt quét chồng nhau, nút tự khoá
- [x] Luồng theo dõi tiến trình con — quét sập vẫn gỡ được cờ, nút không kẹt vĩnh viễn
- [x] Thứ tự an toàn: **ghi cache xong mới đặt `finished`** — GUI không thể đọc file dở
- [x] Xác minh lần mở thứ 2 không có UAC
- [x] Xác minh bấm "Quét lại" → UAC → thanh tiến độ chạy → index tự cập nhật — **người dùng xác nhận**
- [x] Xác minh bấm **No** trên UAC → báo rõ, dữ liệu cũ nguyên vẹn

---

## P5 — Thumbnail + lưới ảo hoá ✅

**Tiêu chí nghiệm thu:** 5.000 kết quả, cuộn mượt, chỉ ~30 DOM node (đo bằng DevTools).

- [x] Thêm crate `image` (chỉ mã hoá PNG), `lru`
- [x] `media/thumbnail.rs` — `IShellItemImageFactory::GetImage`, không cần bundle ffmpeg
- [x] `media/thumbnail.rs` — thử `SIIGBF_INCACHEONLY` trước, fallback sinh mới
- [x] `media/thumbnail.rs` — pool 4 worker, mỗi thread `CoInitializeEx(APARTMENTTHREADED)` một lần
- [x] `media/thumbnail.rs` — HBITMAP → RGBA (top-down, hoán BGR) → PNG
- [x] `media/thumbnail.rs` — LRU 512 mục + hàng đợi chặn trên 64 việc
- [x] `ipc/protocol.rs` — scheme `thumb://{epoch}/{index}?s=N`, có epoch chống lẫn ảnh sau khi quét lại
- [x] `src/lib/VirtualList.svelte` — ảo hoá cả list lẫn grid, tái dùng DOM
- [x] Chỉ sinh thumbnail cho hàng đang hiển thị (`loading="lazy"` + ảo hoá)
- [x] Giới hạn kết quả 5.000 (đã đặt từ P2)
- [x] Nút chuyển chế độ xem danh sách ↔ lưới ảnh
- [x] Điều hướng bàn phím theo lưới (`←→` khi ở chế độ lưới)
- [x] Giới hạn kích thước thumbnail phía server (chống URL bịa số lớn)
- [x] Kiểm chứng sinh thumbnail thật từ thư viện người dùng (video/ảnh/nhạc)
- [x] Đo số phần tử khi cuộn — **phẳng hoàn toàn**, xem bảng dưới
- [x] Buộc thumbnail thật, không nhận icon loại tệp (`SIIGBF_THUMBNAILONLY`)

### Số đo ảo hoá (đếm phần tử UIA của WebView)

| Truy vấn | Số kết quả | Phần tử UIA |
|---|---|---|
| `Deep-sea.avif` | 38 | 118 |
| `anglerfish` | 84 | 133 |
| `deep sea` | 3.333 | **118** |
| `mp4` | 5.000 | **118** |
| `a` | 5.000 | **118** |

5.000 kết quả render đúng bằng số phần tử với 38 kết quả — số node không phụ thuộc số kết quả.

### Số đo thumbnail

| | Trước khi sửa | Sau |
|---|---|---|
| video | 1280×720 · 1.269.526 byte · 980ms | 192×108 · **37.475 byte** · **51ms** |
| image | 242×242 · 82.888 byte · 50ms | 192×192 · **51.976 byte** · **9ms** |
| cache lần 2 | — | **0,003 ms** |

Cache 512 mục: ~18 MB thay vì 650 MB.

---

## P6 — Enrichment metadata + lọc ✅

**Tiêu chí nghiệm thu:** lọc được `≥1080p` và `thời lượng > 10 phút`; metadata sống sót qua restart.

- [x] Lượt nhanh: `GetFileAttributesEx` → `size` + `mtime`, chạy song song trong indexer
- [x] `media/metadata.rs` — `SHGetPropertyStoreFromParsingName` + `PKEY_*` viết tay
- [x] `media/enrich.rs` — lượt nền 2 luồng, `THREAD_PRIORITY_BELOW_NORMAL`
- [x] Ưu tiên **video trước** — độ phân giải/thời lượng là thứ người ta lọc video theo
- [x] Store bền `metadata.bin`, key = hash đường dẫn viết thường
- [x] Tự vô hiệu khi `(size, mtime)` đổi — tệp đã thay thì đọc lại
- [x] Lưu mỗi 500 tệp — đóng app mất tối đa vài giây công việc
- [x] Bộ lọc trong `search.rs` — kiểm tra trước cả phép so chuỗi (số nguyên rẻ hơn)
- [x] UI: chip lọc độ phân giải / thời lượng + nút bỏ lọc
- [x] UI: chỉ báo *"đã đọc thuộc tính X/Y tệp"* — nói rõ vì sao kết quả ít
- [x] UI: hiện độ phân giải / thời lượng / dung lượng trên từng dòng
- [x] Sửa `BUG-012` — `SCHEMA_VERSION` nằm trong chính khối nó bảo vệ
- [x] Sửa `BUG-013` — hàng đợi sắp xếp ngược, `pop()` lấy từ cuối nên đọc nhạc trước video
- [x] Kiểm chứng trên dữ liệu thật — xem bảng dưới

### Kiểm chứng P6

| Hạng mục | Kết quả |
|---|---|
| Lượt nhanh (dung lượng, ngày sửa) | 117.128 tệp, tổng **3.014,6 GB**, trong **13,1 giây** |
| `IPropertyStore` trên tệp thật | video **1920×1080** + thời lượng · ảnh có kích thước · nhạc có thời lượng |
| Tốc độ đọc thuộc tính | 4–80 ms/tệp, thực đo ~**5,7 ms** với 2 luồng |
| Lưu bền | khởi động lại nạp ngay **50.947 mục có sẵn**, không đọc lại |
| Bộ lọc `≥1080p` | **5.000 kết quả trong 9,1 ms** |
| Hiển thị trên từng dòng | `4K · 0:04 · 3.2 MB` |
| Chỉ báo tiến độ | *"Đã đọc thuộc tính 54.822/117.128 tệp · đang tiếp tục"* |

RAM index: 7,6 → **9,4 MB** (thêm dung lượng + ngày sửa cho 117k tệp).

---

## P7 — Tìm file trùng ✅

**Tiêu chí nghiệm thu:** phát hiện đúng bản sao tạo thủ công; không có nút xoá hàng loạt không xác nhận.

- [x] Thêm crate `blake3`
- [x] Tầng 1 — nhóm theo dung lượng, **không đọc gì cả** (index đã có sẵn)
- [x] Tầng 2 — BLAKE3 64KB đầu + 64KB cuối + size, chạy song song
- [x] Tầng 3 — `full_hash()` đọc toàn tệp, chỉ gọi khi cần chắc chắn
- [x] Bỏ qua tệp dưới 64 KB — icon và thumbnail trùng dung lượng hàng nghìn cái
- [x] Xếp theo **lãng phí nhiều nhất trước** — thứ tự người dọn ổ đĩa cần
- [x] UI riêng cho kết quả trùng, dùng lại lưới ảo hoá
- [x] **Không tự động xoá bất cứ thứ gì** — chỉ báo cáo
- [x] Nói rõ giới hạn của tầng 2 ngay trên giao diện
- [x] Test thừa nhận giới hạn: hai tệp giống hai đầu khác ở giữa → tầng 2 không phân biệt được
- [x] Dùng lại kết quả đã quét thay vì quét lại 10 phút khi quay lại chế độ này
- [x] Sửa `formatBytes` — dừng ở MB nên hiện "17048.5 MB" thay vì "16.6 GB"
- [x] Sửa tiêu đề trộn số nhóm bị cắt với tổng lãng phí đầy đủ

### Kiểm chứng P7 trên thư viện 3 TB thật

| Hạng mục | Kết quả |
|---|---|
| Tầng 1 (miễn phí) | **70.576/117.128 tệp** cùng dung lượng cần kiểm tra |
| Tầng 2 (đọc 128 KB/tệp) | ~9 GB đọc đĩa, **584 giây** |
| Kết quả | **6.780 nhóm · 520,7 GB có thể thu hồi** (17% thư viện) |
| Nhóm lớn nhất | 3 bản sao × 16,6 GB — thừa **33,3 GB** |
| Bấm lại lần hai | **3,5 giây** — dùng lại kết quả, không quét lại |

Mẫu trùng lặp điển hình: CapCut nhân bản asset qua từng draft
(`CapCut Drafts/DS1_106/subdraft/…` và `CapCut Drafts/DS1_063/…` cùng một tệp 16,6 GB).

---

## P8 — Hoàn thiện ✅

- [x] Hotkey toàn cục `Ctrl+Alt+Space` gọi cửa sổ từ bất kỳ đâu
- [x] Bấm lại để ẩn — cùng một phím mở và đóng, tay không rời bàn phím
- [x] Một phiên bản duy nhất — bấm hotkey tới cửa sổ đang mở, không mở bản thứ hai
- [x] Không đăng ký được phím tắt thì cảnh báo rồi chạy tiếp, không từ chối khởi động
- [x] Gợi ý phím tắt hiện ngay trên màn hình trống
- [x] Điều hướng bàn phím `↑ ↓ ← → Enter Esc PageUp PageDown` (làm ở P3/P5)
- [x] Thông báo rõ cho volume non-NTFS bị bỏ qua (làm ở P1)
- [x] `cargo tauri build` → exe **9,9 MB** + bộ cài NSIS
- [x] Chạy chính exe release: tìm kiếm, thumbnail, mở tệp, mở thư mục — tất cả hoạt động

### Đo được trên bản release

| Việc | Số đo |
|---|---|
| Nạp cache lúc khởi động | **27 ms** cho 117.128 tệp |
| Tìm `anglerfish` | **84 kết quả · 0,5 ms** |
| Tìm `avatar` | **54 kết quả · 0,5 ms** |
| Gọi cửa sổ từ trạng thái thu nhỏ | 3/3 chu kỳ phục hồi |
| Mở thư mục chứa tệp | Explorer xác nhận đúng tệp, đường dẫn có dấu tiếng Việt |
| Mở tệp | tiến trình `Photos — 'anglerfish.webp'` xuất hiện |

Hai lỗi lộ ra ở lượt test này, cả hai đều đã sửa:
[BUG-014](docs/bug.md#bug-014) (mời gọi phím tắt mà app không sở hữu) và
[BUG-015](docs/bug.md#bug-015) (gọi được cửa sổ nhưng không đặt con trỏ vào ô tìm kiếm).

### Cập nhật realtime qua USN Journal — KHÔNG LÀM Ở P8, chuyển sang P9

`FSCTL_READ_USN_JOURNAL` cần mở handle `\\.\C:`, tức là **cần quyền Administrator**. Đo được ở
P1: chạy không elevate thì mọi volume đều trả `AccessDenied`.

Nhưng bất biến kiến trúc số 3 (xem `README.md`) là **GUI không bao giờ chạy elevated** — để giữ
kéo-thả từ Explorer và không có UAC mỗi lần mở app. Hai điều này loại trừ nhau.

Các cách vượt qua, và vì sao không chọn:

| Cách | Vấn đề |
|---|---|
| Cho GUI elevate | Phá bất biến số 3 — UAC mỗi lần mở, mất kéo-thả từ Explorer |
| Windows Service chạy nền có quyền | Đúng đắn nhất (Everything làm vậy), nhưng là một dự án con riêng: cài đặt, gỡ bỏ, IPC, quyền |
| Scheduled Task chạy `--index` định kỳ | Khả thi, nhưng vẫn là quét định kỳ chứ không phải realtime |

**Đã chọn cho P8:** giữ nút "Quét lại" — quét đầy đủ mất ~38 giây và người dùng chủ động.

Hạ tầng cho service đã có sẵn từ P4: mỗi volume được lưu một `VolumeStamp` trong cache gồm
`letter`, `serial`, **`journal_id`**, **`next_usn`**, `file_count`. `journal_id` mới là trường
quan trọng nhất — nó là thứ duy nhất phát hiện được journal đã bị xoá rồi tạo lại, trường hợp mà
`next_usn` cũ trở thành vô nghĩa. Kế hoạch chi tiết ở P9 bên dưới.

---

## P9 — Windows Service theo dõi USN realtime 🔵

**Chưa bắt đầu.** Mục này là kế hoạch, không phải mô tả thứ đã có. Mọi ô đều `[ ]`.

**Mục tiêu.** Bỏ nút "Quét lại". Tạo tệp media mới xong thì nó có mặt trong kết quả tìm kiếm ngay,
không cần người dùng làm gì.

### Vướng mắc thật sự không nằm ở việc đọc journal

Đọc `FSCTL_READ_USN_JOURNAL` là phần dễ — nó gần giống `FSCTL_ENUM_USN_DATA` đã viết ở P1. Phần khó
là **áp thay đổi vào index đang chạy**, và ở đây kiến trúc hiện tại chặn lại:

`Index` là bất biến theo thiết kế. `IndexBuilder` chỉ có `add_dir` / `add_file` rồi `finish()` —
**không có API xoá**. Chuỗi nằm chung trong một arena `Vec<u8>`, tên tệp chỉ là `Span { off, len }`
trỏ vào đó, nên xoá một mục tại chỗ là không làm được: khoảng trống để lại không ai dùng được, còn
dồn lại thì mọi span phía sau đều phải dịch.

Đó chính là thứ làm nó nhanh (xem bất biến số 6 trong `README.md`), nên không thể vứt đi. Ba đường ra:

| Cách | Đánh đổi |
|---|---|
| **Dựng lại toàn bộ `Index` trong RAM từ `Index` cũ + thay đổi** | Tưởng là đắt. Đo ra thì không |
| Lớp phủ nhỏ có thể sửa, nằm cạnh index đông cứng | Tìm kiếm phải quét hai nơi rồi hợp nhất, cần nén định kỳ, và mọi nơi đang dùng vị trí làm định danh đều phải sửa |
| Đổi `Index` sang cấu trúc có tombstone + free list | Phá vỡ SoA + arena — mất đúng thứ khiến tìm kiếm chạy 0,5 ms |

**Đã chọn cách 1, sau khi đo.** Ban đầu bản kế hoạch này nghiêng về lớp phủ, với lý do "dựng lại
mất ~38 giây". Con số đó **sai chỗ**: 38 giây là thời gian **quét đĩa**, không phải thời gian dựng
cấu trúc trong RAM. Đem `cargo bench --bench search -- build` ra đo:

| Việc | Thời gian |
|---|---|
| Dựng `Index` 100.000 mục (kể cả fold lại toàn bộ) | **37,5 ms** |
| Dựng `Index` 500.000 mục | **183 ms** |
| Nạp lại metadata enrichment cho 117.128 mục | **53 ms** (đo từ log khởi động) |

Thư viện thật là 117.128 tệp, nên một lần áp thay đổi tốn khoảng **100 ms** trên một luồng nền.
Gộp thay đổi trong vài giây rồi dựng lại một lần thì chi phí là vài phần trăm của một nhân.

`Index` không sửa được, nhưng **đọc được** — dựng cái mới từ cái cũ cộng danh sách thay đổi là
hoàn toàn khả thi. Đổi lại: không lớp phủ, không tombstone, không nén, không phải sửa `search.rs`,
và không chỗ nào trong ứng dụng phải học cách xử lý hai nguồn dữ liệu.

Enrichment không cản đường: `Store` khoá theo **hash đường dẫn**, không theo vị trí, và
`seed_from_store` vốn đã dựng lại toàn bộ vector `MediaProps` từ một `&Index` mới trong mỗi lần
khởi động. Vị trí đổi hết sau mỗi lần dựng lại cũng không sao — `epoch` của `ArcSwap` đã lo phần
thumbnail từ P5.

**Đổi tên thư mục** vẫn rẻ như phân tích ban đầu, và vì lý do đó: bảng `dirs` tách riêng từ P2 nên
đổi tên một thư mục chứa 50.000 tệp chỉ là sửa **một** chuỗi.

### Thứ thật sự còn thiếu: định danh

Bỏ được lớp phủ rồi thì chỉ còn đúng một vướng mắc kiến trúc, và nó nhỏ hơn nhiều.

Journal cho biết "tệp có FRN X vừa bị xoá". Nhưng `Index` **không lưu FRN** — nó chỉ có tên, thư
mục, loại, dung lượng, thời gian. Không có cách nào biết mục nào trong index tương ứng với FRN X.
Ghép theo đường dẫn thì không dùng được: tệp vừa bị xoá thì không còn đường dẫn để hỏi nữa.

Nên bước đầu tiên của P9 là **thêm FRN vào index**: `frn: Vec<u64>` cho tệp và `dir_frn: Vec<u64>`
cho thư mục. Tốn 8 byte mỗi mục — khoảng 940 KB cho thư viện thật, trên nền 9,4 MB hiện tại.
`RawRecord` đã mang sẵn `frn` và `parent_frn` từ P1; `tree.rs` chỉ đang **vứt đi** sau khi dùng
xong.

### Việc phải làm

**Giai đoạn 1 — Rust thuần, không cài service, test được bằng dữ liệu tổng hợp:**

- [x] `frn` cho tệp và `dir_frn` cho thư mục, xuyên suốt `tree.rs` → `IndexBuilder` → `Index` → cache
- [x] Nâng `SCHEMA_VERSION` — cache cũ báo "phiên bản 2, phần mềm cần 3", đã kiểm chứng
- [x] `rebuild_with(&Index, &[Change]) -> Index` — tạo, xoá, đổi tên, di chuyển tệp và thư mục
- [x] Đọc `FSCTL_READ_USN_JOURNAL`, dịch bản ghi thô thành `Change` — **570 bản ghi thật trong 1 ms**
- [x] Phát hiện journal bị xoá rồi tạo lại, và journal cuộn vòng
- [x] Tự kiểm tra journal ở cuối mỗi lần quét đầy đủ, trong tiến trình vốn đã có quyền
- [x] `--index` thử cập nhật nhanh trước, thất bại thì tự quét đầy đủ (`--full` để ép quét)
- [x] Gộp thay đổi rồi dựng lại một lần, thay vì dựng lại theo từng thay đổi
- [x] Chế độ `--watch` để kiểm chứng bộ đọc journal trên volume thật

**Giai đoạn 2 — cần quyền và cần cài đặt, làm sau:**

- [ ] Đọc `FSCTL_READ_USN_JOURNAL` liên tục cho từng volume NTFS, nối từ `next_usn` đã lưu
- [ ] Phát hiện **journal bị xoá rồi tạo lại**: `journal_id` khác `VolumeStamp` → buộc quét lại đầy đủ
- [ ] Phát hiện **journal cuộn vòng**: `next_usn` đã lưu cũ hơn bản ghi cũ nhất còn lại → quét lại đầy đủ
- [ ] Diễn giải `USN_REASON_*`: tạo, xoá, đổi tên, đổi dữ liệu
- [ ] Ghép cặp `RENAME_OLD_NAME` + `RENAME_NEW_NAME` — hai bản ghi rời của cùng một thao tác
- [ ] Lớp phủ có thể sửa, hợp nhất lúc tìm kiếm, nén định kỳ
- [ ] Volume gắn vào / tháo ra giữa chừng
- [ ] Service chết rồi bật lại → nối tiếp từ `next_usn` đã lưu, không quét lại từ đầu
- [ ] Cài / gỡ service, elevate **đúng một lần** lúc cài
- [ ] Gỡ ứng dụng thì **phải** gỡ luôn service
- [ ] Service là **tuỳ chọn** — không cài thì ứng dụng vẫn chạy đúng như hôm nay

### Đo được: dựng lại rẻ, và số lượng thay đổi gần như không tính phí

`cargo bench --bench search -- rebuild_with`, trên index tổng hợp 500.000 mục:

| Số thay đổi áp vào | Thời gian |
|---|---|
| 0 | 160 ms |
| 100 | 170 ms |
| 10.000 | **170 ms** |

Mười nghìn thay đổi tốn đúng bằng một trăm. Chi phí nằm hết ở việc dựng lại, không ở số thay đổi —
nghĩa là **gộp thay đổi lại rồi áp một lần là gần như miễn phí**, và không có lý do gì để áp từng
cái một.

Thư viện thật là 46.700 mục, tức khoảng **20 ms** một lần áp.

> Con số 117.128 xuất hiện ở các mục P6–P8 phía trên là trạng thái đĩa **trước ngày 24/8/2026**.
> Chiều hôm đó người dùng xoá thư mục `D:\Sounds Edit\HƯNG` (70.461 tệp media, ~265 GB) — đã xác
> nhận, không phải mất dữ liệu. Xem [CHECK-006](docs/check.md#check-006). Các số đo cũ được giữ
> nguyên chứ không sửa lại, vì chúng đúng tại thời điểm đo.

Bench này cũng là thứ tìm ra [BUG-017](docs/bug.md#bug-017): `rebuild_with/0` mất 165 ms còn
`rebuild_with/100` chỉ 21,9 ms — áp một trăm thay đổi mà nhanh hơn bảy lần so với áp không thay đổi
nào. Nguyên nhân là số `0` đang được coi là một định danh hợp lệ, nên một thay đổi xoá sạch cả
index. 149 test không bắt được; một con số vô lý thì bắt được ngay.

### Đã chạy trên ổ thật

Con trỏ journal được ghi lại **trước** khi một ổ được duyệt, mà duyệt hết mọi ổ mất hàng chục giây
— nên đến lúc quét xong chắc chắn đã có hoạt động tệp thật để đọc. `check_journal_cursors()` chạy
ở cuối mỗi lần quét, trong đúng tiến trình vốn đã có quyền Administrator, nên **không tốn thêm lời
nhắc UAC nào**:

```
ổ C: tự kiểm tra journal — 570 thay đổi kể từ usn=20623105360 (nay 20623167576) [1ms]
    C: CÓ MẶT  frn=19984723347872345 cha=1688849862689891 progress.json.tmp
ổ D: tự kiểm tra journal — 0 thay đổi kể từ usn=2088815312 (nay 2088815312) [0ms]
```

Chi tiết những gì đã và chưa xác nhận: [CHECK-003](docs/check.md#check-003).

### Đọc journal có cần quyền Administrator không — đã đo, và câu trả lời là có

Cả kế hoạch Windows Service dựa trên một câu chưa ai kiểm tra. Đem đo bằng tiến trình **không**
elevate, mở `\\.\C:` với bốn mức quyền:

| Quyền xin | Mở volume | `FSCTL_QUERY_USN_JOURNAL` |
|---|---|---|
| `0` (không xin gì) | **được** | lỗi 1 — `ERROR_INVALID_FUNCTION` |
| `FILE_READ_ATTRIBUTES` | **được** | lỗi 1 |
| `FILE_READ_DATA` | lỗi 5 — `ACCESS_DENIED` | — |
| `GENERIC_READ` | lỗi 5 | — |

Mở volume thì **không** cần quyền — điều này trước đây cũng tưởng là cần. Nhưng FSCTL của journal
bị từ chối trên handle quyền thấp. Xem [CHECK-004](docs/check.md#check-004).

### Cũ: hai ô `[~]` khi chưa chạy được trên ổ thật

Bộ đọc journal có 17 test, gồm cả một test đi hết chặng — byte thô của journal vào, index mới ra.
Nhưng tất cả đều chạy trên **bản ghi tự dựng bằng tay**. Chúng chứng minh mã đọc đúng cái layout
tôi *nghĩ* là layout của `USN_RECORD_V2`, không chứng minh một ổ NTFS thật hành xử đúng như tài
liệu mô tả.

`FSCTL_READ_USN_JOURNAL` cần quyền Administrator. Lời nhắc UAC đã bật lên nhưng không được chấp
nhận, nên phép thử thật chưa chạy. Cách chạy:

```powershell
# trong một terminal chạy với quyền Administrator
D:\tool_finding\src-tauri\target\debug\mediafinder.exe --watch C
```

Rồi tạo, đổi tên, xoá một tệp `.mp4` ở bất kỳ đâu trên ổ đó. Mỗi thao tác phải in ra một dòng.
Những thứ cần nhìn tận mắt, vì test bằng dữ liệu tự dựng không thể trả lời:

| Cần xác nhận | Vì sao test tổng hợp không đủ |
|---|---|
| Đổi tên thật sự sinh ra **cặp** `RENAME_OLD_NAME` + `RENAME_NEW_NAME` | Tôi tự dựng cặp đó theo tài liệu; chưa thấy Windows tự sinh ra |
| Xoá sinh ra `FILE_DELETE`, và nó tới **sau** các bản ghi khác của cùng tệp | Thứ tự trong journal thật là thứ quyết định "bản ghi cuối thắng" có đúng không |
| `next_usn` trả về thực sự tiến lên, không lặp vô hạn | Vòng lặp đọc dừng dựa vào điều này |
| Ghi một tệp lớn sinh ra bao nhiêu bản ghi | Quyết định một lô lớn cỡ nào, và có cần chặn trần hay không |

Cho tới khi chạy được, hai ô trên giữ dấu `[~]` — theo đúng quy ước ở đầu tài liệu này: đã viết,
chưa kiểm chứng. Đánh `[x]` bây giờ sẽ là nói dối chính mình ở phiên sau.

### Kiểm chứng cuối, trên máy thật

Tạo, đổi tên, tạo thư mục mới, rồi xoá — tất cả đi qua USN journal thật:

| Bước | Index | Thời gian |
|---|---|---|
| Trước | 46.700 mục / 3.195 thư mục | — |
| Tạo `.mp4` + đổi tên + tệp trong thư mục mới | **46.702 / 3.196** | **0,45 s** |
| Xoá cả hai và xoá thư mục | **46.700 / 3.195** | **0,43 s** |
| So với quét đầy đủ | | **13,2 s** |

Nhanh hơn khoảng **30 lần**, và nút "Quét lại" trong giao diện cũng đi đúng đường này vì nó gọi
cùng một `--index`. Tệp `.txt` tạo kèm không vào index, đúng như phải thế.

Giới hạn cuối cùng đã được sửa: tệp media **đầu tiên** đặt vào một thư mục cũ chưa từng chứa media
trước đây bị bỏ sót tới lần quét kế tiếp. Nay khi không tra được thư mục cha, hệ thống hỏi thẳng
NTFS bằng `OpenFileById` + `GetFinalPathNameByHandleW`, và áp lại luật loại trừ cho đường dẫn nhận
được — xem [RISK-003](docs/risk.md#risk-003). Kiểm chứng trên máy thật: **+1 tệp, 1 lần hỏi hệ
thống tệp, 0,60 s**.

### Giao tiếp giữa service và GUI — chọn đường ít quyền nhất trước

Service chạy dưới `LocalSystem`, tức là **đọc được mọi tệp trên máy**. GUI chạy dưới quyền người
dùng thường. Bất cứ kênh nào nối hai bên cũng là một mặt tấn công leo thang đặc quyền:

| Cách | Mặt tấn công | Độ trễ |
|---|---|---|
| **Service ghi cache, GUI theo dõi tệp bằng `ReadDirectoryChangesW`** | Gần như không có — không tồn tại kênh ra lệnh nào | Cao hơn, phải ghi lại cả tệp |
| Named pipe có ACL chặt | Phải tự viết ACL đúng; sai một chỗ là mọi tiến trình local đều ra lệnh được cho service | Thấp |
| TCP trên localhost | Không nên — mọi thứ trên máy đều nối được | Thấp |

**Bắt đầu bằng cách 1.** Chỉ chuyển sang named pipe khi **đo được** rằng độ trễ ghi tệp là vấn đề
thật, chứ không phải vì nó nghe chuyên nghiệp hơn. Nếu dùng pipe: ACL chỉ mở cho SID của phiên đăng
nhập tương tác, và service phải coi **mọi** thông điệp nhận được là dữ liệu không tin được — nó là
bên đang giữ đặc quyền.

### Rủi ro cần theo dõi

| Rủi ro | Giảm thiểu |
|---|---|
| Service treo hoặc rò bộ nhớ, chạy nền hàng tháng không ai để ý | Log xoay vòng; đo mức dùng RAM sau 24 giờ và sau 7 ngày trước khi coi là xong |
| Đổi tên thư mục gốc làm sai đường dẫn hàng loạt | Test riêng: đổi tên thư mục có ≥10.000 tệp bên dưới |
| Cặp đổi tên bị tách qua hai lượt đọc journal | Giữ bản ghi lẻ lại tới lượt sau, không xử lý vội |
| Gỡ ứng dụng để sót service `LocalSystem` | Test gỡ cài đặt là **tiêu chí nghiệm thu**, không phải việc làm thêm |
| Lớp phủ phình vô hạn nếu không nén | Ngưỡng cứng + nén cưỡng bức, có test khoá lại |

### Tiêu chí nghiệm thu

- [ ] Tạo một tệp `.mp4` mới → thấy trong kết quả **dưới 5 giây**, không bấm gì
- [ ] Xoá tệp đó → biến khỏi kết quả, cũng không bấm gì
- [ ] Đổi tên một thư mục có ≥1.000 tệp media → mọi đường dẫn con cập nhật đúng
- [ ] Tắt service → ứng dụng vẫn tìm kiếm bình thường trên cache cũ
- [ ] Gỡ ứng dụng → `sc query` không còn thấy service
- [ ] Chạy liên tục 24 giờ → RAM không tăng đơn điệu

---

## P10 — Quét ổ mạng / NAS theo yêu cầu ✅

**Vấn đề.** Ba ổ mạng, ~37,9 TB, hoàn toàn không có trong chỉ mục — và cho tới
[BUG-018](docs/bug.md#bug-018) thì còn không có một dòng thông báo nào. Kiến trúc MFT/USN không đọc
được chúng, không phải vì thiếu tính năng mà vì bản chất: ổ mạng là một phiên SMB, không phải một
volume trên máy này.

### Quyết định: nút riêng, không tự động

Người dùng nêu thẳng: *"sẽ khá là ít tôi lên NAS để kiếm file… việc cần thiết quét là quét ổ trên
máy trước, nếu user muốn quét cả trên NAS thì bấm nút."*

Số đo về sau xác nhận điều đó:

| Nút | Phạm vi | Thời gian |
|---|---|---|
| **Quét lại** | chỉ ổ gắn trong máy | ~13 s, hoặc **0,45 s** nếu journal trả lời được |
| **+ ổ mạng** | ổ trong máy **rồi mới** tới NAS | **~4,5 phút** |

Nút thứ hai chỉ hiện khi máy thật sự có ổ mạng. Thứ tự bên trong nó là cố ý: phần nhanh chạy
trước, nên kết quả thường dùng đã sẵn sàng trước khi phần chậm bắt đầu — và nếu bấm Dừng thì vẫn
giữ được nó.

Nút Dừng chỉ hiện trong pha mạng, vì chỉ pha đó dừng được: pha ổ cục bộ chạy trong một tiến trình
elevated khác mà tiến trình này không cầm handle. Một nút dừng không dừng được gì còn tệ hơn không
có nút.

### Đo được trên NAS thật

| Ổ | Thư mục | Tệp media | Thời gian | Tốc độ |
|---|---|---|---|---|
| F: | 3.832 | 144.417 | 11,6 s | 331 thư mục/giây |
| Y: | 7.581 | 150.575 | 237,3 s | 32 thư mục/giây |
| Z: | 958 | 18.953 | 23,8 s | 40 thư mục/giây |
| **Tổng** | 12.371 | **313.945** | **272,6 s** | |

**313.945 tệp trên NAS so với 46.700 trên ổ trong máy** — gấp gần 7 lần. Đây không phải phần thêm
cho đủ; đây là phần lớn nhất của thư viện.

### Ba điều quyết định kiến trúc

**1. Tiến trình elevated không nhìn thấy ổ mạng.** Đo được, không phải suy đoán: cùng một hàm
`list_volumes()`, chạy elevated thấy C:, D:, G:; chạy quyền thường thấy cả sáu ổ. Ổ mạng gắn theo
phiên đăng nhập. Nên bộ quét NAS chạy **trong tiến trình GUI** — và không mất gì, vì duyệt thư mục
không cần quyền. Xem [CHECK-007](docs/check.md#check-007).

**2. Quét ổ cục bộ không được xoá phần NAS.** Hệ quả trực tiếp của điều trên: tiến trình elevated
dựng lại chỉ mục mà không có ổ Z: nào trong đó. Không xử lý thì người dùng quét NAS 4,5 phút rồi
bấm "Quét lại" là mất sạch. Quy tắc, cố ý viết mà không nhắc tới chữ "mạng":

> **Giữ nguyên mọi mục thuộc ổ đĩa không được quét trong lần chạy này.**

**3. Tệp qua SMB không có FRN.** Gán 0, mà `index::update` vốn đã coi 0 là "không có định danh"
([BUG-017](docs/bug.md#bug-017)) — nên cập nhật nhanh qua journal không bao giờ đụng vào phần NAS.
Đúng như phải thế: journal của ổ cục bộ không biết gì về tệp trên NAS.

### Việc đã làm

- [x] `walk.rs` — duyệt thư mục song song theo từng tầng, bỏ qua reparse point, huỷ được
- [x] Dung lượng và thời gian đọc kèm ngay trong lúc duyệt (`DirEntry::metadata` miễn phí trên Windows)
- [x] Giữ nguyên mục của ổ không quét, ở cả hai chiều (quét cục bộ giữ NAS, quét NAS giữ cục bộ)
- [x] Ổ mạng không được cấp mốc journal — có test khoá lại
- [x] Hai nút tách bạch, nút "+ ổ mạng" chỉ hiện khi có ổ mạng
- [x] Nút Dừng, chỉ trong pha mạng
- [x] Kiểm chứng trên chính hai NAS của người dùng

### Còn lại

Phần NAS **không** cập nhật gia tăng — không có journal để theo. Muốn mới thì bấm lại nút.
`ReadDirectoryChangesW` *có thể* chạy qua SMB nếu máy chủ hỗ trợ change notify, nhưng đó là thứ
phải thử thật trên chính hai máy này chứ không đọc tài liệu rồi tin.

---

## P11 — Chạy nền ở khay hệ thống ✅

**Người dùng đề xuất, và nó vá một lỗ hổng thật.** Nguyên văn: *"khi ấy tôi đã tắt rồi nhưng mà
đột nhiên tôi có file muốn tìm thì tôi sẽ không sử dụng được Ctrl+Alt+Space."*

Phím tắt sống trong tiến trình. Bấm X là giết tiến trình, nên phím tắt chết theo — mà đó lại là
đường vào chính. Nghịch lý: càng dùng đúng cách (đóng cửa sổ cho gọn) thì càng làm hỏng tính năng
chính, và không có gì trên màn hình nói cho biết điều đó.

### Việc đã làm

- [x] Đóng cửa sổ thì **ẩn**, không thoát — `CloseRequested` gọi `prevent_close()` rồi `hide()`
- [x] Biểu tượng ở khay hệ thống, kèm tooltip nhắc phím tắt
- [x] Bấm trái vào biểu tượng: hiện/ẩn, y như phím tắt
- [x] Chuột phải: menu **Mở MediaFinder** · **Thoát**
- [x] Menu chỉ hiện khi chuột phải — chuột trái là thao tác nhanh, phải làm việc nhanh
- [x] Một dòng trên màn hình trống nói rõ đóng cửa sổ không phải là thoát
- [x] Không cản trở tắt máy

### Kiểm chứng trên máy thật

| Việc | Kết quả |
|---|---|
| Gửi `WM_CLOSE` (đúng thứ nút X gửi) | cửa sổ ẩn, **tiến trình vẫn sống** |
| Bấm phím tắt sau đó | cửa sổ hiện lại |
| Biểu tượng có trong khay không | có, trong vùng ẩn, tooltip *"MediaFinder — Ctrl+Alt+Space để tìm kiếm"* |
| Chuột phải → Thoát | **tiến trình kết thúc** |
| `WM_QUERYENDSESSION` | trả về **1** — không cản trở tắt máy |

Mục cuối là thứ dễ làm sai nhất mà không ai để ý cho tới lúc tắt máy: một ứng dụng chặn sự kiện
đóng có thể chặn nhầm cả tín hiệu kết thúc phiên, khiến Windows hiện *"ứng dụng này đang ngăn tắt
máy"*. Thử được an toàn bằng cách gửi đúng thông điệp Windows dùng để hỏi, thay vì phải tắt máy
thật.

---

## P12 — Kéo tệp ra ngoài ✅

**Người dùng hỏi:** tìm thấy tệp rồi thì kéo thẳng vào CapCut hoặc ô upload của trang web được
không.

### Cái bẫy ở tầng đầu tiên

Giao diện chạy trong WebView2, và kéo-thả HTML5 chỉ đặt được các kiểu dữ liệu của web —
`text/plain`, `text/uri-list`. Mọi ứng dụng nhận tệp đều đọc **`CF_HDROP`**, cấu trúc `DROPFILES`
của shell Windows. Hai thứ đó không gặp nhau: kéo bằng HTML5 vào CapCut sẽ **không ra tệp nào**, và
sẽ trông như tính năng bị hỏng chứ không như một giới hạn.

Nên phải làm **nguồn kéo OLE gốc**: `IDataObject` thật mang `DROPFILES`, chạy qua `DoDragDrop`.

### Dùng crate `drag` thay vì tự viết — *quyết định này đã bị đảo ngược ở P13*

> **Đọc mục này với một dấu sao.** Crate làm tắt phăng ứng dụng khi kéo tệp trên ổ mạng
> ([BUG-020](docs/bug.md#bug-020)) và đã bị bỏ. Phần bên dưới giữ nguyên vì lý lẽ dẫn tới quyết
> định sai cũng đáng đọc như quyết định đúng — xem [CONF-006](docs/config.md#conf-006).


Phần khó đã có sẵn: `#[implement(IDataObject)]`, `CF_HDROP` + `DROPFILES`, `IDropSource`,
`IDragSourceHelper` cho ảnh kéo. Tự viết là khoảng ba trăm dòng COM `unsafe`, và chỗ cấp phát
`HGLOBAL` sai thì hỏng âm thầm. Cái giá: +456 KB và một bản `windows` 0.52 nằm cạnh 0.61 —
đã đo và chấp nhận, xem [CONF-006](docs/config.md#conf-006).

### Ba chi tiết quyết định

**`preventDefault()` trên `dragstart`.** Nếu không chặn, WebView sẽ khởi động phép kéo của riêng
nó song song với phép kéo gốc; con trỏ kẹt và không thả được gì.

**`DoDragDrop` chặn luồng gọi** cho tới khi thả xong — đọc mã crate đã xác nhận. Nên nó chạy trên
luồng giao diện qua `run_on_main_thread`, và cửa sổ đứng yên trong lúc kéo. Explorer cũng vậy.

**Luôn là Copy, không bao giờ Move.** Một công cụ tìm kiếm không có việc gì phải di dời tệp nó tìm
được. Thả vào thư mục khác phải để nguyên bản gốc tại chỗ.

### Kiểm chứng: hỏi đầu bên kia nhận được gì

Không dùng Explorer làm đích — nó chỉ cho biết tệp có sang hay không. Dựng một cửa sổ nhận thả
riêng, ghi lại **đúng những định dạng nhận được**:

```
các định dạng: Shell IDList Array, FileDrop, FileNameW, FileName,
               FileContents, FileGroupDescriptorW, ZoneIdentifier
CF_HDROP: CÓ
  tệp: C:\Users\Padoma1\Videos\mf-keo-tha-thu.mp4  (262144 byte)
```

Một shell data object đầy đủ — đúng thứ CapCut, Explorer và ô upload đọc.

### Giới hạn đã biết

Thả vào ứng dụng chạy quyền Administrator sẽ bị Windows chặn (UIPI). Đó là ràng buộc bảo mật của
hệ điều hành, không sửa được từ phía ứng dụng.

---

## P13 — Chọn nhiều, kéo nhiều, sắp theo thời gian ✅

**Người dùng chọn:** *"Chọn nhiều + kéo nhiều, và sắp theo thời gian"*.

### Ba việc, một nguyên tắc chung

**Chọn nhiều.** Ctrl+click thêm/bớt từng hàng, Shift+click chọn cả dải, Shift+↑↓ mở rộng từ chỗ
neo. Giữ `selection` **bên cạnh** `selected` chứ không thay thế nó: `selected` là chỗ bàn phím đang
đứng, `selection` là thứ lệnh sẽ tác động. Gộp hai thứ vào một thì không diễn đạt được Shift+click.

**Kéo nhiều.** Kéo một hàng **nằm trong** lựa chọn thì mang cả lựa chọn; kéo một hàng **ngoài** lựa
chọn thì chỉ mang hàng đó — nếu không, một cú click lạc chỗ sẽ âm thầm kéo đi những tệp người dùng
còn không nhìn thấy. Explorer cư xử đúng như vậy.

**Sắp theo thời gian.** Nút "Liên quan" / "Mới nhất" cộng với chip lọc 7 / 30 / 365 ngày. Sắp theo
thời gian phải là **một khoá sắp xếp thật**, không phải sắp lại 5.000 kết quả đã cắt: nếu cắt theo
điểm liên quan rồi mới sắp theo ngày thì tệp mới nhất có thể đã bị cắt mất từ trước. Nên `Hit` mang
sẵn `key` và heap top-K so sánh bằng chính khoá đó.

### Lỗi lớn nhất của giai đoạn không nằm ở tính năng nào

Kéo bất kỳ tệp NAS nào → **ứng dụng tắt ngay lập tức**. Crate `drag` chuẩn hoá đường dẫn thành UNC
verbatim, shell từ chối dạng đó, crate `.unwrap()` cái `None` — và vì chỗ đó nằm trong window
procedure nên panic **không unwind được**, runtime `abort()` cả tiến trình.

87% thư viện của người dùng nằm trên NAS. Chi tiết: [BUG-020](docs/bug.md#bug-020).

Đã bỏ crate và tự viết [`ipc/drag_source.rs`](src-tauri/src/ipc/drag_source.rs). Hoá ra nhỏ hơn
nhiều so với ước lượng cũ, vì **`IDataObject` là do chính shell dựng** —
`SHCreateShellItemArrayFromIDLists` + `BindToHandler(BHID_DataObject)`. Chỉ còn `IDropSource` với
ba phương thức. Bỏ được luôn bản `windows` 0.52 thừa ([CONF-006](docs/config.md#conf-006)).

### Kiểm chứng

Cửa sổ nhận thả tự dựng, báo lại đúng những gì nhận được:

```
các định dạng: Shell IDList Array, FileDrop, FileNameW, FileName,
               FileContents, FileGroupDescriptorW, ZoneIdentifier
CF_HDROP: CÓ
  tệp: D:\TÀI NGUYÊN DEEP SEA\Sinh vật phù du\Gen\Create_a_highly_202601071441_5luao.mp4
  tệp: D:\TÀI NGUYÊN DEEP SEA\Sinh vật phù du\Gen\Create_a_highly_202601071453_n1nvn.mp4
  tệp: F:\AutoEdit\library\deepsea\Sinh vat phu du\Gen\Create a highly 202601071441 5luao.mp4
```

Ba tệp một lượt, **trộn ổ cục bộ với ổ mạng**, đường dẫn có dấu tiếng Việt — đúng ca trước đây làm
sập ứng dụng. Ba lần chạy liên tiếp đều đủ ba tệp.

Chi tiết cả lượt test, kể cả hai lần kéo không khởi động mà tôi **chưa giải thích được**:
[test-log](docs/test-log.md).

---

## BT — Bảo trì sau phát hành 🟢

Không phải một giai đoạn có điểm kết thúc. Quy tắc số 5 vẫn nguyên hiệu lực: **mọi vấn đề gặp trong
lúc dùng thật đều phải ghi lại**, theo đúng bảng phân loại ở [`docs/README.md`](./docs/README.md).

### Ghi vào đâu

Không đổi gì so với trước: đọc bảng "Ghi vào file nào?" từ trên xuống, dừng ở dòng khớp đầu tiên.
Mỗi phiên dùng thật có phát hiện gì thì thêm một mục có ngày vào
[`docs/test-log.md`](./docs/test-log.md).

Khác biệt duy nhất của giai đoạn này: **nguồn phát hiện là người dùng, không phải kịch bản test.**
Nghĩa là mô tả ban đầu sẽ mơ hồ ("tìm không ra"), và việc đầu tiên luôn là **tái hiện được lỗi** —
chưa tái hiện được thì chưa biết mình đang sửa cái gì.

### Những ca đã biết là dễ sai, cần thu thập thêm

`SPEC-002` xuất phát từ đúng một lần người dùng dán tiêu đề video vào và không ra kết quả. Nó phơi
ra hai chuyện cùng lúc: dấu câu dính vào token, và tên tệp bị trình tải xuống cắt ngắn. Loại ca đó
gần như chắc chắn còn nữa:

| Ca | Vì sao dễ sai |
|---|---|
| Số tập `S01E02` / `1x02` / `Tập 2` | Ba cách viết cùng một thứ, hiện được chấm điểm như ba chuỗi không liên quan |
| Viết tắt (`ĐH`, `TP.HCM`) | Fold xong còn `dh`, `tp.hcm` — dấu chấm vẫn dính vào token |
| Chỉ gõ năm (`2019`) | Khớp cả tên tệp, đường dẫn lẫn số thứ tự ngẫu nhiên — nhiễu rất nặng |
| Ý nghĩa nằm hết ở tên thư mục, tên tệp chỉ là `1.mp4` | Đã xử lý ở `SPEC-001`, nhưng **trọng số** thư mục so với tên tệp thì chưa hề kiểm chứng bằng dữ liệu thật |
| Tên riêng có `đ` gõ không dấu (`da nang` ↔ `Đà Nẵng`) | Đã có test, nhưng chỉ với vài chuỗi cố định |

### Đề xuất: đo thay vì đoán, nhưng chỉ đo tại chỗ

Muốn chỉnh điểm số cho đúng thì cần biết truy vấn nào **trả về 0 kết quả** — đó là tín hiệu rõ nhất
cho biết chỗ nào đang hỏng. Đề xuất: ghi các truy vấn 0 kết quả vào một log **cục bộ**, kèm nút xem,
nút xoá và nút tắt hẳn ngay trong giao diện.

Ràng buộc bắt buộc nếu làm: **không gửi đi đâu hết.** Truy vấn tìm kiếm cho biết người dùng có
những tệp gì trên máy — đó là dữ liệu riêng tư, không phải số liệu sử dụng. Mặc định phải là tắt,
và bật lên phải là một hành động có chủ ý.

---

## Sai lệch so với kế hoạch

| Ngày | Mục | Kế hoạch | Thực tế | Lý do |
|------|-----|----------|---------|-------|
| 2026-08-24 | Phạm vi tìm kiếm | chỉ **tên tệp** (đặc tả mục 3.3) | tìm cả **đường dẫn thư mục** | Dữ liệu thật tổ chức theo kiểu tên thư mục mang toàn bộ ý nghĩa, tên tệp chỉ là số. Tìm theo tên tệp trả về gần như rỗng. Xem `SPEC-001`. |
| 2026-08-24 | Lọc thư mục | danh sách tên cố định | thêm quy tắc `skip_dot_directories` | Một quy tắc thay cho danh sách phải bổ sung mãi mãi; bao phủ cả công cụ cài sau này. Xem `ISSUE-001`. |
| 2026-08-24 | `MediaKind` | thuộc P2 | kéo sớm sang P1 | Bộ lọc phần mở rộng chạy ngay lúc quét MFT, viết bảng này hai lần là vô lý. |
| 2026-08-24 | crate `windows` | `0.58` | **`0.61`** | Tauri 2.11 đã kéo về `windows 0.61.3`. Giữ 0.58 → 2 bản trong graph, tốn build time và dễ xung đột type. Bump lúc chưa viết dòng Win32 nào là miễn phí; để sau sẽ phải sửa code. |

---

## Nhật ký kiểm chứng

Chỉ ghi những gì đã **thực sự chạy** và kết quả quan sát được.

### 2026-08-24 — P0

| Kiểm chứng | Lệnh | Kết quả |
|---|---|---|
| Toolchain gốc | `rustc/cargo/node/npm --version` | Node `24.18.0`, npm `11.12.1`; **Rust chưa cài** |
| Build tools | `vswhere` + Windows Kits | VS BT `2022 17.14.37502` + `2026 18.4.11626`; SDK `10.0.26100.0` |
| WebView2 | registry `EdgeUpdate\Clients\{F3017226-…}` | `151.0.4129.101` |
| Cài Rust | `rustup-init -y --default-toolchain stable-x86_64-pc-windows-msvc` | exit 0 → `rustc 1.98.0 (88d9e12ae)` |
| Icon hợp lệ | đọc `icons/128x128.png` | render đúng — kính lúp xanh trên nền tối bo góc |
| Frontend deps | `npm install` | 51 gói; svelte `5.56.10`, vite `6.4.3`, tauri-cli `2.11.4` |
| Frontend type-check | `npm run check` | **0 lỗi, 0 warning** |
| Frontend build | `npm run build` | dist OK — JS 28.37 kB (gzip 11.16 kB) |
| Rust check (windows 0.58) | `cargo check --all-targets` | **exit 0**, không lỗi/warning, 2m35s |
| Rust check (windows 0.61) | `cargo check --all-targets` | **exit 0**, 29.28s, graph thống nhất 1 bản `windows` |
| Cửa sổ mở được | `npm run tauri dev` | **exit 0**, build 2m16s → `Running target\debug\mediafinder.exe` → `starting GUI`, không panic |
| Cửa sổ render đúng | `PrintWindow` + xem ảnh | `916x659`, title bar + icon, tiếng Việt đủ dấu, dark theme |
| Unit test | `cargo test` | pass (0 test ở P0) |
| Chất lượng code | `cargo clippy --all-targets` | **sạch, 0 warning** |
| Dispatch 2 chế độ | `mediafinder.exe --index` | log `indexer mode`, không mở GUI, exit 101 tại `unimplemented!` — đúng như thiết kế |
| Manifest trong binary | `grep` chuỗi trong exe | có `asInvoker` `longPathAware` `PerMonitorV2`; **không** có `requireAdministrator` |
