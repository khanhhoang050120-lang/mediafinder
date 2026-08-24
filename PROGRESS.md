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

---

## Bảng tổng quan

| GĐ | Nội dung | Trạng thái |
|----|----------|-----------|
| **P0** | Scaffold + kiểm tra toolchain | ✅ **XONG** — 27/27, test 8/8 pass |
| **P1** | Enumerator NTFS (USN) | ✅ **XONG** — 29/29 test, quét thật 4,1 triệu bản ghi |
| **P2** | Index + fold + search + bench | ✅ **XONG** — 72/72 test, bench 3,01 ms worst case |
| **P3** | Nối Tauri + UI tối giản | ✅ **XONG** — 96/96 test, mở tệp + mở thư mục đã kiểm chứng |
| **P4** | Cache trên đĩa + luồng elevate | ✅ **XONG** — 100/100 test, người dùng xác nhận chạy được |
| **P5** | Thumbnail + lưới ảo hoá | 🔵 **đang làm** |
| **P6** | Enrichment metadata + lọc | ⬜ chưa bắt đầu |
| **P7** | Tìm file trùng | ⬜ chưa bắt đầu |
| **P8** | Hoàn thiện (hotkey, bàn phím, USN realtime) | ⬜ chưa bắt đầu |

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

## P5 — Thumbnail + lưới ảo hoá ⬜

**Tiêu chí nghiệm thu:** 5.000 kết quả, cuộn mượt, chỉ ~30 DOM node (đo bằng DevTools).

- [ ] Thêm crate `image`, `lru`
- [ ] `media/thumbnail.rs` — `IShellItemImageFactory::GetImage`
- [ ] `media/thumbnail.rs` — thử `SIIGBF_INCACHEONLY` trước, fallback sinh mới
- [ ] `media/thumbnail.rs` — pool 4 worker, mỗi thread `CoInitializeEx(APARTMENTTHREADED)`
- [ ] `media/thumbnail.rs` — HBITMAP → RGBA → encode
- [ ] `media/thumbnail.rs` — LRU cache ~500 entry
- [ ] `ipc/protocol.rs` — đăng ký scheme `thumb://{file_id}?s=N`
- [ ] `src/lib/VirtualGrid.svelte` — ảo hoá list + grid, tái dùng DOM
- [ ] Chỉ sinh thumbnail cho hàng đang hiển thị
- [ ] Nâng giới hạn kết quả lên 5.000
- [ ] Đo DOM node bằng DevTools khi cuộn

---

## P6 — Enrichment metadata + lọc ⬜

**Tiêu chí nghiệm thu:** lọc được `≥1080p` và `thời lượng > 10 phút`; metadata sống sót qua restart.

- [ ] Lượt nhanh: `GetFileAttributesEx` → `size` + `mtime`
- [ ] `media/metadata.rs` — `SHGetPropertyStoreFromParsingName` + `PKEY_*`
- [ ] Lượt nền priority thấp, không chặn tìm kiếm, cho phép tắt
- [ ] Store bền, key `(file_id, size, mtime)` để tự invalidate
- [ ] UI: chip lọc độ phân giải / thời lượng + chỉ báo tiến độ
- [ ] Đối chiếu chéo với thuộc tính file trong Explorer

---

## P7 — Tìm file trùng ⬜

**Tiêu chí nghiệm thu:** phát hiện đúng bản sao tạo thủ công; không có nút xoá hàng loạt không xác nhận.

- [ ] Thêm crate `blake3`
- [ ] Tầng 1 — nhóm theo `(size, ext)`
- [ ] Tầng 2 — BLAKE3 64KB đầu + 64KB cuối + size
- [ ] Tầng 3 — BLAKE3 toàn file, chỉ khi user xác nhận
- [ ] UI riêng cho kết quả trùng
- [ ] **Không tự động xoá bất cứ thứ gì**

---

## P8 — Hoàn thiện ⬜

- [ ] Hotkey toàn cục gọi cửa sổ
- [x] Điều hướng bàn phím: `↑ ↓ Enter Esc`
- [ ] Cập nhật realtime qua `FSCTL_READ_USN_JOURNAL`
- [ ] Thông báo rõ cho volume non-NTFS bị bỏ qua
- [ ] `cargo tauri build` → chạy exe release, xác nhận toàn bộ vẫn hoạt động

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
