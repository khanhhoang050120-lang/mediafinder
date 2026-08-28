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
6. **Mọi bước git đều chờ lệnh** (bổ sung 27/08): làm xong để ở working tree cho người dùng
   duyệt; commit / push `edit` / merge `master` / tag phát hành — từng nấc chỉ làm khi được nói rõ.

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
| **P13** | Chọn nhiều, kéo nhiều, sắp theo thời gian | ✅ **XONG** — Ctrl/Shift chọn dải, kéo cả tập |
| **P14** | Xem trước ngay trong ứng dụng | ✅ **XONG** — `media://`, ảnh/video/nhạc |
| **P15** | Đóng gói phát hành 20–40 máy | ✅ **XONG** — máy trắng, SmartScreen thật, UAC một lần |
| **P16** | Tách App.svelte + bộ kiểm thử vitest | ✅ **XONG** — 8 component, `npm test`, 2 bug bắt tại chỗ |
| **P17** | Tuỳ chọn bền & làm chủ bàn phím | ✅ **XONG** — prefs qua phiên, chế độ trùng có bàn phím, Home/End/Ctrl+A |
| **P18** | Đường ống thumbnail | ✅ **XONG** — hết "ảnh mất vĩnh viễn", nhịp đứng yên 120 ms |
| **P19** | Xem trước: chuột và phím Space | ✅ **XONG** — viết nốt khoá `armed` bị thiếu |
| **P20** | Chuỗi tự cập nhật + v1.0.2→v1.0.5 | ✅ **XONG** — người dùng kiểm chứng trọn vòng trên máy thật |
| **P21** | Ghi chú dài & quyền ở lại bản cũ | 🟡 **một nửa chờ duyệt** — A+C+D đã lên master; phần skip nằm ở `edit` |
| **P22** | Bốn mảng backend: nhật ký file, đo 0-kết-quả, xác minh tầng-3, lịch 15 phút | 🟡 **chờ duyệt ở working tree** — nghiệm thu tay trên máy thật, tìm & sửa BUG-P22-01 |
| **P23** | Lọc kết quả theo ổ đĩa (chip + nhãn dòng) | 🟡 **chờ duyệt ở working tree** — 112 test JS, 5 đột biến (1 từng lọt, đã vá) |
| **P24** | Hỏi trước khi quét lại ổ mạng | 🟡 **chờ duyệt ở working tree** — 121 test JS, 242 Rust, 4 đột biến (1 từng lọt, đã vá) |
| **P25** | "Quét lại" nói nó vừa làm gì | 🟡 **chờ duyệt ở working tree** — 129 test JS, 246 Rust, 4/4 đột biến bị bắt |
| **P26** | Rà soát độ phủ toàn bộ, bù nhóm t14 | 🟡 **chờ duyệt ở working tree** — tìm 3 nhánh không ai canh, 142 test JS |
| **P27** | Sửa BUG-024: cài đè tay xoá sạch chỉ mục | 🟡 **chờ duyệt ở working tree** — 250 test Rust, chốt chặn cho `nsis-hooks.nsh` |
| **P28** | Điều tra "10/16 từ" trên ổ cục bộ | 🔵 **điều tra xong, chưa sửa** — lái app thật 8 truy vấn, tái hiện cả hai vế, ghi BUG-025 |
| **P29** | Vá chốt chặn cho v1.0.6 + nói thật tuổi chỉ mục | 🟡 **chờ duyệt ở working tree** — phản biện bắt lỗi trong chính lộ trình; 259 test Rust, 161 test JS, 2 bài admin trên máy thật |
| **P30–P31** | Rà soát toàn bộ trước v1.0.6 + sửa 8 lỗi tìm được | 🟡 **chờ duyệt ở working tree** — 272 test Rust, 170 test JS; bộ cài thật đã chạy, ảnh thu nhỏ hết hiện sai tệp |
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

## P14 — Xem trước ngay trong ứng dụng ✅

**Người dùng chọn:** *"Xem trước nhanh trong ứng dụng"*.

**Vì sao đáng làm với người dựng phim:** thumbnail tĩnh cho biết *đúng tên tệp*, không cho biết
*đúng cảnh quay*. Với 360.655 tệp mà 87% nằm trên NAS, mở từng clip bằng ứng dụng ngoài để xem thử
là vòng lặp chậm nhất trong cả quy trình.

### Đo trước khi hứa

Câu hỏi chặn: phát một tệp trên NAS qua SMB có mượt không, hay sẽ giật và tua không nổi. Đem đo
trên chính ổ `F:` của người dùng, tệp 1.987 MB:

| | |
|---|---|
| Byte đầu tiên | **66 ms** |
| Thông lượng | **84,7 MB/s** (678 Mbps) |
| Nhảy tới cuối tệp rồi đọc 1 MB | **18 ms** |

Cao hơn hẳn bitrate của bất kỳ video nào, nên tệp NAS phát như tệp cục bộ. Con số này mới là thứ
quyết định làm hay không.

### `media://` — và vì sao không dùng đường dẫn

Trang web cần **byte** của tệp. Đưa qua data URL thì phải nạp cả video vào bộ nhớ rồi base64 —
clip 2 GB thành 2,7 GB chữ. Nên byte đi qua URL, và trình duyệt tự lấy phần nó cần.

URL có dạng `media://localhost/{epoch}_{index}` — **đúng cách định danh mà `thumb://` đã dùng**.
Điểm quan trọng: **không đường dẫn nào xuất hiện trong URL**. Trang chỉ gọi được một vị trí trong
chỉ mục, nên chỉ chạm được tới tệp mà chỉ mục đã có. Nếu phục vụ theo đường dẫn thì trang đọc được
mọi tệp trên máy.

**Range request là phần cốt lõi.** Trình phát của Chromium không tải cả video rồi mới phát — nó xin
từng đoạn byte, và xin lại ở mỗi chỗ người dùng kéo thanh tua. Không có `206 Partial Content` thì
phải tải hết mới hiện được khung hình đầu, và tua thì hỏng hẳn. Một yêu cầu `bytes=0-` được trả
lời **có giới hạn 8 MB** thay vì trả nguyên tệp: trả ít hơn mức được xin là hợp lệ, và nó ngăn một
yêu cầu kéo cả bộ phim vào bộ nhớ.

### Ba cách mở, một cách đóng

Nháy đúp · `Shift+Enter` · menu chuột phải. `Esc` đóng, `↑↓` đổi tệp mà không rời lớp phủ,
`Enter` giao tệp cho Windows như cũ.

**Nháy đúp trước đây mở bằng ứng dụng ngoài — trùng với Enter.** Nay nó mở xem trước, còn Enter
giữ nguyên vai trò. Thao tác nhanh và không rời ứng dụng là thứ dùng nhiều hơn hẳn.

### Hai lỗi tìm được khi tự kiểm chứng

**[BUG-021](docs/bug.md#bug-021) 🟠 — nháy đúp mở xem trước làm cửa sổ bung toàn màn hình.** Đo
được: mở bằng bàn phím thì cửa sổ 880×620, mở bằng nháy đúp thì 1920×1080. Cử chỉ mở lớp phủ rơi
luôn vào thẻ `<video>` vừa hiện ra dưới con trỏ. Chặn `dblclick` **trên chính thẻ video** thì tới
muộn — trình điều khiển media của Chromium đã xử lý xong trước khi sự kiện nổi lên. Chặn ở **pha
capture trên `window`** thì sạch 5/5.

Giữa hai lần sửa có một lượt kiểm chứng hỏng đáng ghi hơn cả lỗi: lệnh cài của tôi chạy trên bộ cài
cũ, nên tôi đo một bản không chứa bản vá, kết luận sai về nguyên nhân, và suýt để lời giải thích
sai đó nằm lại trong mã — [CHECK-008](docs/check.md#check-008).

**[BUG-022](docs/bug.md#bug-022) 🟡 — `D:\` hiện thành `:D`.** Mẹo `direction: rtl` của tôi để cắt
đầu đường dẫn dài đã bị thuật toán bidi chuyển dấu câu lên trước. Chỉ lộ ra với tệp nằm ngay gốc ổ.

**[BUG-023](docs/bug.md#bug-023) 🟠 — video 1080p tràn khỏi khung, đè lên dòng chân.** Người dùng
báo, không phải kịch bản test tìm ra. Khung chứa dùng hàng lưới `auto` nên `max-height: 100%` không
có mốc quy chiếu và bị bỏ qua; video vẽ ở kích thước gốc. Mọi ảnh kiểm chứng của tôi đều là clip
720p — **dữ liệu thử nhỏ hơn dữ liệu thật**, đúng bài học của [BUG-020](docs/bug.md#bug-020) ở một
chỗ khác.

### Một giả định của tôi bị chính phép đo bác bỏ

Tôi cố ý **không** khai báo `.mkv`/`.avi` là video, nghĩ rằng làm vậy sẽ khiến trình phát báo lỗi
ngay thay vì hiện ô đen. Thử thật thì một tệp `.mkv` **vẫn phát bình thường**: Chromium tự đánh hơi
nội dung chứ không tin phần mở rộng. Nên khai báo kiểu ở đây là **gợi ý, không phải cửa chặn** —
và kết quả còn tốt hơn dự tính: nhiều tệp xem trước được hơn. Chú thích trong mã đã sửa lại theo
đúng hành vi quan sát được, không giữ lời giải thích cũ.

---

## P15 — Đóng gói để phát hành cho 20–40 máy ✅

**Người dùng yêu cầu:** đóng gói lại toàn bộ để phát cho khoảng 20–40 người, và **test thật kỹ từng
bước** để sang máy khác vẫn chạy ổn.

**Ba câu hỏi đã chốt trước khi làm:** người dùng đều là Administrator máy mình · chấp nhận cảnh báo
SmartScreen thay vì mua chứng chỉ · máy họ **có** dùng chung NAS.

### Rà soát trước, phá sau

Rà soát tìm ra bốn thứ chỉ đúng trên máy này, tất cả trước khi chạy phép thử nào:

| # | Vấn đề | Vì sao chỉ lộ ra khi phát hành |
|---|---|---|
| 1 | **Bộ cài không thiết lập gì cả** | Lối tắt tự khởi động và tác vụ định kỳ trên máy gốc là do tôi tạo tay bằng PowerShell. Máy mới: không tự khởi động, và "Quét lại" hỏi UAC **mỗi lần bấm** |
| 2 | **WebView2 chưa cấu hình** | Mặc định tải bootstrapper lúc cài → máy thiếu WebView2 mà không có mạng thì ra ứng dụng mở lên trắng, không báo gì |
| 3 | **Không tên nhà phát hành, không mô tả** | Mục trong Apps & features ghi `mediafinder` viết thường, không có gì để nhận diện |
| 4 | **Màn hình lần đầu trông như phần mềm hỏng** | Giữa màn hình ghi *"Không tìm thấy kết quả nào"*; lý do thật nằm ở một dòng xám nhỏ dưới đáy |

Điểm 4 chỉ thấy được bằng cách **mô phỏng máy chưa từng chạy**: cất chỉ mục đi rồi mở lên xem người
lạ thấy gì.

### Một lần UAC cho tất cả

Thiết kế xoay quanh một ràng buộc cứng: đọc bảng tệp NTFS **bắt buộc** có quyền Administrator
([CHECK-004](docs/check.md#check-004)), và đăng ký tác vụ với `HighestAvailable` cũng vậy — đã đo
lại: `schtasks /Create` không elevate trả về `Access is denied`.

Nên không có cách nào tránh được một lần xin quyền. Nhưng có cách để **chỉ tốn đúng một lần**:

- **Tác vụ định kỳ** do chính tiến trình quét tạo ra — tiến trình đó vốn đã elevate vì người dùng
  vừa bấm "Quét lần đầu". Không tốn thêm hộp thoại nào.
- **Lối tắt tự khởi động** không cần quyền gì: một tệp trong hồ sơ người dùng. Giao diện tự ghi.

Sau lần đó tác vụ mang quyền, nên "Quét lại" vĩnh viễn không hỏi nữa.

Cả hai đều **kiểm tra trước khi tạo**: ai xoá lối tắt vì không muốn chạy lúc đăng nhập, hoặc sửa
tác vụ sang giờ khác, thì giữ được lựa chọn đó.

### Gỡ cài đặt phải dọn sạch

Đo thật: gỡ xong thì thư mục cài, chỉ mục 45 MB, lối tắt và mục trong Apps & features **đều biến
mất**. Riêng tác vụ định kỳ **còn sót** vì xoá nó cần quyền Administrator.

Trên một máy thì bỏ qua được. Trên bốn mươi máy đó là bốn mươi tác vụ chạy mỗi ngày để khởi động
một chương trình không còn tồn tại. Nên bộ gỡ nay **tự xin quyền đúng cho việc đó** — từ chối cũng
không sao, việc gỡ vẫn tiếp tục và hướng dẫn có nêu câu lệnh dọn nốt.

Gỡ im lặng thì truyền `--quiet` và bỏ qua bước xin quyền: một hộp thoại không ai bấm sẽ treo cả
tiến trình gỡ.

### Bộ cài 208 MB, và vì sao chấp nhận

Nhúng hẳn bộ cài WebView2 vào (`offlineInstaller`) làm bộ cài phình từ **2,4 MB lên 208 MB**.

Đổi lại: cài được ở mọi nơi, kể cả máy vừa cài lại Windows và không có mạng. Kiểu hỏng mà nó loại
bỏ là kiểu tệ nhất — ứng dụng mở lên không thấy gì và **không có thông báo nào giải thích**. Với
một đội chuyển hàng gigabyte footage mỗi ngày thì 208 MB chuyển một lần không phải vấn đề.

Đổi lại được nếu muốn: `embedBootstrapper` cho bộ cài ~4 MB nhưng cần mạng trên máy thiếu WebView2.

### Đã kiểm chứng

| # | Nội dung | Kết quả |
|---|---|---|
| 1 | Không còn đường dẫn cứng của máy gốc trong mã chạy thật | ✅ chỉ có trong test |
| 2 | Nâng cấp 0.1.0 → 1.0.0 | ✅ một mục duy nhất trong Apps, đúng tên nhà phát hành |
| 3 | Ứng dụng tự tạo lối tắt tự khởi động | ✅ đúng đích, đúng `--minimized`, có mô tả |
| 4 | Màn hình lần đầu | ✅ nói rõ cần gì, mất bao lâu, và tự phát hiện 4 ổ mạng |
| 5 | Gỡ cài đặt dọn sạch | ✅ 3/4 tự động; tác vụ định kỳ nay có bước xin quyền riêng |
| 6 | Windows Defender | ✅ không bị báo là mối đe doạ |
| 7 | **Phần mềm có gửi gì ra ngoài không** | ✅ **0** gói mạng trong toàn bộ cây phụ thuộc, 0 lời gọi HTTP trong mã |
| 8 | Vòng kiểm tra | ✅ **206 test**, sạch cả bốn |

### Đóng gói thư viện: đo trước, và câu trả lời ngắn hơn dự tính

**Ý định của người dùng:** gói sẵn mọi thư viện cần thiết; máy nào thiếu thì cài, máy nào có bản cũ
thì nâng lên bằng bản của máy gốc.

Trước khi xây bất cứ cơ chế nào, phải biết **danh sách thư viện thật**. Đọc thẳng bảng nhập PE của
tệp exe — không đoán từ `Cargo.toml` — ra 22 DLL, và **không có thư viện nào phải cài thêm**
([CHECK-009](docs/check.md#check-009)):

- `api-ms-win-crt-*` là UCRT, **nằm sẵn trong Windows 10 và 11**.
- Phần còn lại đều là thành phần của Windows.
- **`vcruntime140.dll` không có mặt** — Rust đã liên kết tĩnh phần đó. Nếu nó có, mỗi máy sẽ cần
  gói Visual C++ Redistributable và máy thiếu sẽ báo `vcruntime140.dll not found`.

Thứ duy nhất ở ngoài là **WebView2 Runtime**, và nó đã được nhúng hẳn vào bộ cài. Vế *"máy có bản
cũ thì nâng lên"* **tự xảy ra**: WebView2 là loại evergreen, Microsoft tự cập nhật qua trình cập
nhật của Edge.

Nên cơ chế cần xây nhỏ hơn nhiều so với hình dung ban đầu — không phải một trình quản lý thư viện,
mà **một bước kiểm tra và một câu nói đúng lúc**.

### Kiểm tra trước khi mở cửa sổ

Nếu WebView2 vắng mặt — thường gặp nhất là **chép tệp exe từ máy khác sang thay vì chạy bộ cài** —
thì thứ người dùng thấy là một cửa sổ trắng, không câu nào giải thích. Với người không biết code,
đó là chỗ họ dừng lại.

Nay có bước kiểm tra chạy **trước khi Tauri khởi động**, hiện một hộp thoại của Windows nói đúng
một việc cần làm. Đã ép hiện ra để nhìn tận mắt, và phải sửa câu chữ **hai lần**: hộp thoại của
Windows rộng khoảng 50 ký tự và dòng nào dài hơn thì nó **cắt vào giữa chữ** — bản đầu hiện ra
*"kết thúc bằ / ng"*, bản sau *"nó sẽ tự c / ài"*. Với tiếng Việt thì một chữ bị cắt đôi trông như
phần mềm lỗi phông. Nay mỗi dòng dưới 46 ký tự, và có test giữ ngưỡng đó khỏi trôi.

### Phép thử cuối: chạy thật trên máy trắng, người dùng bấm UAC

Đường "máy chưa có gì → bấm Quét lần đầu → UAC → tiến trình quét tự tạo tác vụ" không tự kiểm chứng
được: nó cần một cú bấm vào hộp thoại UAC, và lách hộp thoại đó là việc không nên làm. Nên nó được
thử bằng tay, một lần, đúng như người dùng mới sẽ gặp.

**Dựng cảnh cho giống thật.** Bộ cài được chép ra Desktop và **gắn Mark-of-the-Web** (`ZoneId=3`) —
dấu Windows dành cho tệp tải từ mạng. Không có bước này thì Windows tin sẵn một tệp vừa dựng tại
chỗ, và màn SmartScreen — thứ cần kiểm chứng nhất — sẽ không hiện ra.

**Người dùng thao tác ba bước:** gỡ cài đặt (kèm UAC để xoá tác vụ) · cài lại qua màn SmartScreen
(`More info` → `Run anyway`) · bấm `Quét lần đầu` rồi `Yes` ở UAC.

**Kết quả đo được sau đó:**

| Kiểm chứng | Kết quả |
|---|---|
| Tác vụ định kỳ — do chính tiến trình quét tạo, không có hộp thoại riêng | ✅ `Highest`, chạy dưới tên người dùng, lệnh `mediafinder.exe --index`, đủ **2 trigger** |
| Lối tắt tự khởi động — do giao diện tạo, không cần quyền | ✅ đúng đích, đúng `--minimized` |
| **"Quét lại" còn hỏi quyền nữa không** | ✅ `schtasks /Run` từ tiến trình **không** elevate → `SUCCESS`, tác vụ chạy **kết quả 0** |
| Chỉ mục dựng được từ con số không | ✅ **48.291 tệp · 3.204 thư mục**, tìm **0,4 ms**, đã có sẵn độ phân giải và thời lượng |

Con số 48.291 chính là thứ một máy mới nhận được: **ổ trong máy thôi**. NAS nằm sau nút `+ ổ mạng`,
đúng như thiết kế ở [P10](#p10--quét-ổ-mạng--nas-theo-yêu-cầu-).

**Một lần UAC cho tất cả — đã kiểm chứng chứ không còn là thiết kế trên giấy.**

---

## P16 — Tách App.svelte + bộ kiểm thử vitest ✅

**Người dùng nêu:** *App.svelte 1.636 dòng là God Component — vi phạm đơn nhiệm, khó bảo trì.*

### Tách theo ranh giới trách nhiệm, không theo ranh giới đề xuất

Tám file mới: `SearchBar`, `FilterPanel`, `UpdateBanner` (sau này thành `UpdateNotice`),
`ScanStatusBar`, `DuplicateFinder`, `FirstRun`, `MediaRow`, và module `scanState.svelte.ts`.
App còn ~730 dòng điều phối. Hai chỗ **cố ý làm khác** đề xuất ban đầu:

- **State tìm kiếm ở lại App.** `epoch` là số hiệu bản chỉ mục, `hit.index` là vị trí *trong* bản
  đó — `thumbUrl(epoch, hit.index)` chỉ đúng khi hai giá trị đến từ cùng một lần tìm. Đẩy ra module
  dùng chung là biến bất biến trình biên dịch giữ được thành quy ước phải nhớ.
- **CSS chế độ lưới thành prop** thay vì selector `.grid .thumb` từ cha: CSS của Svelte bị giới hạn
  theo file. Đã đối chiếu bundle build ra — 7 quy tắc lưới chuyển đổi nguyên vẹn từng thuộc tính.

### Hai bug lộ ra ngay trong lúc tách

- Bộ lọc trễ một nhịp — tự gây: đổi `$derived` thành `$effect` khiến mỗi cú bấm lọc tìm theo *lần
  bấm trước*; "Bỏ lọc" trả về danh sách bị lọc chặt nhất. Sửa bằng callback mang giá trị mới đi
  cùng lệnh chạy lại.
- Nhịp đọc-thuộc-tính (3 s) sống lâu hơn cửa sổ — có sẵn từ trước, vá cho nhất quán.

### Bộ kiểm thử thành tài sản của repo

Harness tạm 137 phép kiểm được chuyển thành vitest chính thức (`npm test`): stub Tauri IPC đếm được
từng lệnh, vá jsdom (chiều cao viewport, ResizeObserver, DragEvent). Từ giai đoạn này trở đi, mọi
đợt việc kết thúc bằng **kiểm thử đột biến** — cố tình phá từng nhánh code để chứng minh test bắt
được; phép nào lọt lưới thì viết thêm test cho tới khi bắt.

---

## P17 — Tuỳ chọn bền & làm chủ bàn phím ✅

**Người dùng chốt các mục 2→5 của lộ trình tối ưu.**

- **Tuỳ chọn qua các phiên** (`prefs.ts`): lưới / sắp xếp / chip loại vào `localStorage`, kiểm hợp
  lệ **từng trường** — dữ liệu của bản cũ không thể đưa app vào trạng thái không tồn tại. Chính
  tính năng này làm các test cũ nhiễm nhau (phiên sau "nhớ" phiên trước) — lưới test bắt được ngay,
  cách ly bằng `localStorage.clear()` mỗi ca.
- **Nút "Xem trước" chết trong menu trùng lặp** — có sẵn: hit của chế độ trùng không nằm trong
  `hits`, `indexOf` trả −1, bấm không có gì xảy ra. Mục menu giờ chỉ hiện khi thật sự dùng được.
- **Bàn phím cho chế độ trùng lặp**: con trỏ đếm theo *tệp* và nhảy qua dòng tiêu đề nhóm; Enter
  mở, Ctrl+Enter mở thư mục, Escape thoát chế độ. Trước đó các phím này rơi thẳng xuống danh sách
  tìm kiếm **đang ẩn** — Enter mở một tệp không có trên màn hình. Kiến trúc: một chủ bàn phím duy
  nhất ở App (bài học hai-listener-anh-em đã ghi trong code), component chỉ đưa thao tác.
- **Home / End / Ctrl+A** với quy tắc phân xử kiểu Everything: con trỏ đang trong ô tìm kiếm thì
  nhường phím cho việc sửa chữ; giữ Ctrl thì luôn là lệnh của danh sách. Viết test phát hiện
  jsdom 30 thực thi `autofocus` — giả định của test sai, không phải của code.

---

## P18 — Đường ống thumbnail ✅

**Người dùng hỏi:** *có nên làm Infinite Scroll để ảnh load kịp?* — Chẩn đoán trước khi trả lời:
virtualizer sẵn có đã mạnh hơn Infinite Scroll; nghẽn không nằm ở danh sách kết quả mà ở đường ống
ảnh, và ở đó có một chuỗi bug thật.

### Chuỗi "ảnh mất vĩnh viễn" — có sẵn

Hàng đợi backend đầy → lời từ chối bị trộn chung mã `404` với "tệp không có thumbnail" → frontend
ẩn ảnh **vĩnh viễn**. Cái "dòng sẽ hỏi lại khi đứng yên" mà chú thích backend trông đợi chưa bao
giờ được viết ở phía UI.

### Ba tầng sửa + một bài học đắt

- **Rust:** tách `Busy` (503) khỏi `Unavailable` (404); miss-cache 60 s để hỏi-lại với tệp không
  ảnh gần như miễn phí; câu trả lời lỗi mang `no-store`. Trạng thái hàng-đầy ép được **tất định**
  trong test nhờ `with_limits(0, 1)` — đóng khoảng trống từng để một phép đột biến lọt lưới.
- **Cửa xoay frontend** (`thumbQueue.ts`): tối đa 8 ảnh đồng thời, ô vừa lọt vào mắt đi trước
  (LIFO); prefetch theo hướng cuộn qua hook `onviewport` bỏ không. Probe thực nghiệm tự vạch ra lỗ
  hổng của chính thiết kế mới — prefetch treo có thể bỏ đói ô đang nhìn → trần riêng 4/8 chỗ.
- **Bug người dùng báo (cuộn nhanh rồi giật lên → ô trống vĩnh viễn):** chỗ tải buộc vào vòng đời
  component, cú kéo nhanh bơm đầy hàng backend bằng job rác của dòng đã chết. Sửa gốc: **nhịp đứng
  yên 120 ms** — dòng bị lướt qua không bắn ra yêu cầu nào; giãn thử-lại 300/1000/3000 ms.

---

## P19 — Xem trước: chuột và phím Space ✅

**Người dùng báo:** *bấm xem video thì không dừng, không tua được bằng chuột.*

Loại bug hiếm gặp: cơ chế chống-tự-phóng-to của P14 được chú thích tỉ mỉ — khoá chuột 800 ms rồi
mở, có cả số liệu đo — nhưng **dòng code bật khoá chưa bao giờ tồn tại**: `armed` khai báo `false`,
kiểm tra ở hai nơi, không nơi nào gán `true`. Sân khấu từ chối chuột vĩnh viễn. Viết nốt đúng một
effect. Kèm theo yêu cầu tiếp: **Space tạm dừng video** thay vì đóng overlay (ảnh/nhạc giữ nghĩa
cũ; video hỏng cũng đóng — không còn gì để dừng). Nhóm test `t8` khoá cả cơ chế khoá-mở lẫn Space.

---

## P20 — Chuỗi tự cập nhật tin được + v1.0.2 → v1.0.5 ✅

**Mục tiêu người dùng:** máy đã cài bản cũ phải *tự biết* có bản mới — và kiểm chứng bằng máy thật.

### Hai nửa của một bug có sẵn

Người dùng restart máy, không thấy gì. Nửa 1: app hỏi máy chủ **đúng một lần lúc khởi động cùng
Windows** — trước khi mạng kịp lên; lỗi bị nuốt, không bao giờ hỏi lại. Nửa 2 (tooltip khay đúng mà
cửa sổ im): webview hỏi backend một lần **quá sớm**. Sửa: thử-lại giãn dần 30 s → 10 ph tới khi hỏi
được, hỏi lại mỗi 24 h, và sự kiện `update-available` đánh thức cửa sổ đang mở.

### Trải nghiệm cập nhật theo yêu cầu

Hộp thoại kèm ghi chú "có gì mới" lấy từ `latest.json` (chỉ máy chủ biết bản mới có gì); hai nút
Cập nhật / Để sau; "Để sau" thu về mũi tên giữa chân cửa sổ (icon do người dùng chọn, đã lọc sạch
Sketch-export). Quy trình phát hành mới: changelog viết vào `RELEASE_NOTES.md`, workflow ghép với
hướng dẫn cài qua vạch `---`, hộp thoại cắt ở vạch. Footer hiện **số phiên bản đang chạy** — bằng
chứng tại chỗ sau mỗi lần cập nhật.

### Bốn lần cắt bản

v1.0.2 (đợt sửa lớn — nhưng cơ chế báo của chính nó còn lỗi) → v1.0.3 (nháp, bỏ — bị 1.0.4 gộp
trọn) → v1.0.4 (người dùng cài tay làm bản nền, vì các bản cũ không tin được vào thông báo của
chúng) → **v1.0.5: người dùng xác nhận chuỗi chạy trọn vòng** — hộp thoại đúng ghi chú → Cập nhật
→ phần trăm tải → khởi động lại → footer ghi v1.0.5.

Giữa giai đoạn có một lần **sập nguồn máy dev** — kiểm chứng bằng `git fsck` + chạy lại toàn bộ
test: không mất một byte.

---

## P21 — Ghi chú dài & quyền ở lại bản cũ 🟡 (một nửa chờ duyệt)

**Người dùng hỏi hai chuyện:** ghi chú dài có tràn khung không, và người *thích* bản cũ thì sao —
"chưa chắc có bản mới thì user thích bản mới".

### Đã lên master (A + C + D)

Ô ghi chú có trần chiều cao và **tự cuộn** — tiêu đề và hai nút đứng yên, hành động chính không bao
giờ trốn khỏi màn hình; dải mờ "còn nữa" chỉ hiện khi thật sự còn chữ; chuỗi liền mạch dài bẻ được.
Tại nguồn: `scripts/check-release-notes.sh` chặn build khi `RELEASE_NOTES.md` rỗng hoặc quá 1.200
ký tự. Link "Xem đầy đủ" mở trang Releases qua lệnh backend URL cố định.

### Đang ở `edit` (60efce1) — chờ người dùng duyệt

"**Bỏ qua bản này**" ghi bền vào prefs: bỏ lời nhắc chứ không bỏ lối vào (mũi tên vẫn đó; bản mới
hơn được quay lại). Badge `[quan trọng]` cho vá mất-dữ-liệu/bảo mật: vượt qua bỏ-qua, không có nút
bỏ qua, vẫn còn Để sau — thông tin mạnh hơn, vẫn không ép. Lời hứa ghi cạnh `SCHEMA_VERSION`:
**chỉ nâng ở bản major** — trong cùng dòng 1.x, quay về bản cũ không mất gì; footer Releases chỉ
đường quay về.

---

## P22 — Bốn mảng backend 🟡 (chờ duyệt ở working tree)

**Người dùng chốt cả bốn đề xuất A–D**, kèm quy tắc mới thành luật: *tính năng mới = module riêng.*
Bốn mảng, bốn module mới — không mảng nào phình vào file có sẵn.

### A — `diag.rs`: bản đã cài cũng kể lại được chuyện gì xảy ra

Bài học trực tiếp từ P20: chẩn đoán trên máy người dùng toàn suy luận chay vì `tracing` chỉ ra
stderr. Giờ log đi hai ngả (stderr + file trong thư mục dữ liệu), xoay theo dung lượng 5 MB × 5
file — cố ý không kéo dependency thời gian chỉ để đặt tên file. Menu khay thêm "Xem nhật ký".

### B — `misslog.rs`: đo chất lượng tìm kiếm bằng truy vấn 0-kết-quả

Triển khai đúng đề xuất nằm sẵn trong mục BT, với ràng buộc cứng của nó: mặc định **tắt**, bật là
hành động có chủ ý ngay trong màn "Không tìm thấy kết quả nào" (component `MissLogControls` riêng),
dữ liệu là một file văn bản cạnh cache có nút Xem/Xoá, **không gửi đi đâu**. Cái khó duy nhất là
tiếng ồn khi gõ từng phím — giải bằng ô *chờ lắng* 2 giây: truy vấn 0-kết-quả chỉ được ghi khi nó
đứng yên đủ lâu; chặng gõ dở bị thay thế, chưa từng chạm đĩa.

### C — `media/verify.rs`: tầng 3 của tìm-trùng

Tầng 2 tự thú "đối chiếu hai đầu tệp — xem lại trước khi xoá". Tầng 3 trả món nợ đó: hash trọn
từng byte (blake3 sẵn có), **theo yêu cầu từng nhóm** — vài giây cho nhóm người dùng sắp hành động
thay vì hàng giờ cho cả thư viện. Nút "Xác minh" trên tiêu đề nhóm; kết quả chỉ mặt đúng tệp khác
nội dung, và tệp không-đọc-được thì *không kết luận gì* — không đọc được không phải là "khác".
Đây là điều kiện tiên quyết của mục 7 (Thùng rác).

### D — Lịch chỉ mục mỗi 15 phút (P9 giai đoạn 2, phiên bản thực dụng)

Realtime thật cần đọc USN trong GUI, mà GUI cố ý chạy `asInvoker` — bức tường đó không đáng phá vì
bản vá gia tăng chỉ tốn 0,45 s: thêm `Repetition PT15M` vào trigger là tệp mới hiện sau tối đa một
khắc thay vì "ngày mai". Di trú cho 20–40 máy đã cài: chính task cũ chạy indexer elevated, nên
indexer tự nhận lịch v1 (qua marker trong Description) và tự thay lịch cho mình ở lần chạy kế —
không ai phải bấm gì. Test khoá marker và XML kể cùng một câu chuyện; clippy bắt được đúng một chỗ
hớ hênh (hằng số marker không phải nguồn sự thật duy nhất) — đã sửa.

---

## P23 — Lọc kết quả theo ổ đĩa 🟡 (chờ duyệt ở working tree)

**Người dùng nêu:** kết quả trộn lẫn các ổ nhìn rối mắt, và sẽ rối gấp bội khi quét thêm NAS.

Đề xuất ba lớp, người dùng xem bản mô phỏng bấm-được rồi chốt **lớp 1 + 2**, bỏ lớp 3.

### Lớp 1 — hàng chip ổ đĩa (`DriveChips.svelte`, `drives.ts`)

`Tất cả 6 · C: 1 · D: 3 · NAS 2` — con số trả lời "ổ nào có thứ tôi cần" trước cả khi bấm. Hàng
này **chỉ hiện khi kết quả trải trên nhiều ổ**; một ổ duy nhất thì nó không nói thêm gì mà lại
chiếm mất một dòng của thứ đang thực sự cần chỗ.

Nhận diện ổ làm ở giao diện, suy từ `path`, chứ không thêm trường vào `SearchHit` — trả giá đụng
vào cấu trúc mà mọi đường tìm kiếm và bộ kiểm thử đang dựa vào, để lấy một thứ suy ra được trong
vài ký tự, là không đáng. Đường dẫn UNC giữ nguyên hai gạch (`\NAS`) để một ổ tình cờ tên `N:`
không lẫn với máy chủ tên `NAS`.

### Lớp 2 — nhãn ổ trên mỗi dòng (`MediaRow`)

`D:` xanh, `NAS` cam — cùng ngôn ngữ màu với hàng chip. Ổ mạng mang màu riêng không phải để trang
trí: nó chậm hơn và thường là kho lưu trữ chứ không phải nơi đang làm việc.

### Lớp 3 — gom nhóm theo ổ: **đã bỏ, có lý do**

Chia kết quả thành cụm nghe hợp lý nhưng **phá vỡ thứ tự xếp theo độ khớp**: tệp khớp nhất có thể
nằm ở NAS, và gom nhóm đẩy nó xuống dưới hàng nghìn kết quả ổ D. Chip lọc giải quyết đúng vấn đề
đó mà không phải trả cái giá này.

### Một chỗ lọc duy nhất, không rắc khắp nơi

`hits` trở thành `$derived` của `allHits` đã lọc, nên bàn phím, kéo-thả, xem trước, tải trước ảnh
đều tự nhìn cùng một danh sách. Rắc `filterByDrive` ở từng nơi tiêu thụ thì chỉ cần một chỗ quên
là con trỏ bàn phím trỏ vào một tệp không có trên màn hình.

**Không lưu lựa chọn ổ qua các phiên** — khác lưới/sắp xếp/loại tệp. Một bộ lọc vô hình đang chặn
kết quả là màn hình khó hiểu nhất; mở app thấy "không tìm thấy gì" chỉ vì phiên trước lỡ lọc ổ Z
là cái bẫy không đáng đặt. Và khi lần tìm mới không còn ổ đang lọc, tự về "Tất cả" thay vì bỏ
người dùng lại trước một danh sách rỗng.

---

## P24 — Hỏi trước khi quét lại ổ mạng 🟡 (chờ duyệt ở working tree)

**Người dùng hỏi:** bấm "+ ổ mạng" lần hai thì nó quét lại từ đầu — vậy nút đó rốt cuộc là gì, và
sao không hỏi trước?

### Một đính chính, ghi lại vì nó quan trọng

Lượt trước tôi phỏng đoán "Quét lại" thường sẽ xoá mất dữ liệu NAS. **Sai.** Đọc code cho thấy cả
hai chiều hợp nhất đều bỏ qua ổ mà lượt quét không chạm tới ([`lib.rs:1509`] cho quét nội bộ,
[`lib.rs:526`] cho quét mạng) — dữ liệu NAS an toàn qua mọi lần "Quét lại". Cái "mức 3" tôi định
đề xuất hoá ra đã tồn tại sẵn từ P10. Bài học: đọc trước khi đoán, kể cả khi câu chuyện nghe rất
hợp lý.

### Cái thật sự hỏng: nút không mang trạng thái

Bấm lần nào cũng chạy trọn cả hai giai đoạn — vài phút và tranh băng thông. Người dùng bấm lần hai
vì tưởng nó là một nút khác, rồi ngồi chờ một việc mình không định làm.

**Mức 1 — hộp thoại xác nhận** (`NetScanConfirm.svelte`): hỏi trước khi quét, hai nút Quét lại /
Không, Escape cũng là Không. Nút "Quét lại" thường **không** hỏi — nó chỉ tốn vài giây, hỏi là
phiền vô ích.

**Mức 2 — dấu vết lần quét gần nhất** (`netscan_mark.rs`): lần trước quét lúc nào, ra bao nhiêu
tệp, trên mấy ổ, mất bao lâu. Không có ba con số đó thì lời hỏi rỗng tuếch. Đổi giây ra phút vì
người dùng đang quyết định có bỏ ra chừng ấy thời gian hay không — họ nghĩ bằng phút.

Ghi ra một tệp JSON nhỏ cạnh cache, **không** nhét vào `index.bin`: đụng vào định dạng chỉ mục là
phải nâng `SCHEMA_VERSION`, mà lời hứa ghi cạnh hằng số đó (P21) nói rõ chỉ được nâng ở bản major.
Một dấu vết tiện nghi không đáng để bẻ gãy lời hứa ấy.

Lượt bị huỷ giữa chừng **không** để lại dấu vết: nó không phải một câu trả lời, và nói "đã quét lúc
14:32" về một lượt dừng dở là nói dối với người đang cân nhắc bỏ ra vài phút nữa.

---

## P25 — "Quét lại" nói nó vừa làm gì 🟡 (chờ duyệt ở working tree)

**Người dùng hỏi:** nút "Quét lại" cũng nên có thông báo như "+ ổ mạng" chứ?

### Cùng câu hỏi, khác câu trả lời — vì cái giá khác nhau

"+ ổ mạng" tốn **vài phút** và tranh băng thông: bấm nhầm là mất chừng ấy thời gian, nên hỏi trước
là đáng. "Quét lại" tốn **vài giây** (bản vá gia tăng P9 ~0,45s), và chỉ mục vốn đã tự làm mới mỗi
15 phút từ P22 — nút này giờ chủ yếu dành cho lúc vừa chép tệp vào và muốn thấy ngay. Đúng lúc mà
một hộp thoại xen vào là phiền nhất.

Đã trình bày cả hai phương án kèm cái giá; người dùng chọn **B: nói mà không chặn đường.**

### Hai nửa

**Tooltip** nút Quét lại kèm "Lần gần nhất: 14:32 hôm nay · 48.320 tệp". Ai muốn biết thì rê chuột;
ai không thì không bị cản. Chưa từng quét thì không bịa ra lần nào cả.

**Một dòng sau khi xong** ở thanh trạng thái: "Đã quét lại · thêm 12 tệp". So `meta` trước và sau —
`onreload` là chỗ duy nhất còn giữ cả hai. Cả ca "không có gì đổi" cũng nói: im lặng thì người dùng
không biết nó đã chạy hay chưa, mà đó chính là câu hỏi khiến họ bấm nút lần hai. Tự tắt sau tám
giây (tin, không phải trạng thái) và bị dọn ngay khi lượt mới bắt đầu.

---

## P26 — Rà soát độ phủ toàn bộ 🟡 (chờ duyệt ở working tree)

**Người dùng yêu cầu:** test lại toàn bộ, cần thì viết thêm test case.

### Chạy lại thì dễ; tìm chỗ *không* được canh mới là việc

Chạy lại toàn bộ chỉ chứng minh những gì đã canh vẫn đúng. Phép thử thật là **cố tình phá từng
nhánh rồi xem có ai kêu không** — nhánh nào phá mà mọi test vẫn xanh thì nhánh đó chưa được canh,
chỉ tình cờ đúng.

Rà hết các module và component. Sáu chỗ bị phá thử; ba chỗ **không ai bắt**, cả ba đều ở
`ContextMenu` — component dùng ở cả danh sách tìm kiếm lẫn chế độ trùng lặp mà chưa có nhóm nào
canh:

| Nhánh bị phá | Trước | Sau (nhóm t14) |
|---|---|---|
| Bấm ra ngoài không đóng menu | ⚠️ lọt | 🔴 bắt |
| Không kê menu vào trong khi mở sát mép | ⚠️ lọt | 🔴 bắt |
| Bấm một mục xong menu không đóng | ⚠️ lọt | 🔴 bắt |
| Menu trôi ra ngoài mép trái (toạ độ âm) | — | 🔴 bắt |

Nhóm 14 gồm 13 ca: nội dung và biểu tượng, bấm mục thì chạy đúng hành động **và** đóng, ba đường
đóng (bấm ngoài / chuột phải ngoài / Escape), và bốn ca kê-vào-trong-màn-hình.

### Một test viết sai lại hoá hữu ích

Ca "Escape không lọt xuống app" của tôi đỏ — và nó đúng: `stopPropagation` **không** chặn được
trình nghe anh em trên cùng `window`. Đó chính là lý do App phải có chốt riêng
(`if (menu || preview) return`), điều mà chú thích trong App đã nêu từ P16. Ca này được viết lại
thành lời ghi chép về giới hạn ấy, trỏ sang TC-3.16b — nơi thật sự canh chốt đó.

### Một phép kiểm quá lỏng

Ca "không lùi ra ngoài mép trái" dùng toạ độ `(2, 2)` và kiểm `>= 0` — bỏ `Math.max` đi thì 2 vẫn
qua. Siết lại bằng toạ độ **âm** (có thật: màn hình phụ đặt bên trái cho `clientX` âm) và kiểm
`> 0`; giờ đột biến đỏ đúng chỗ.

---

## P27 — Sửa BUG-024: cài đè tay xoá sạch chỉ mục 🟡 (chờ duyệt ở working tree)

**Người dùng báo:** tìm một tệp có thật trên NAS mà không ra; và **ai ở v1.0.4 thì không gặp, ai lên
v1.0.5 thì gặp**.

### Chẩn đoán đầu của tôi đã sai, và chi tiết người dùng nêu mới lật được vụ này

Tôi kết luận đó là chỉ mục ổ mạng cũ — đúng triệu chứng, nhưng **không giải thích được tương quan
với phiên bản**. Nếu chỉ là chỉ mục cũ thì v1.0.4 phải bị y hệt. Đào tiếp mới ra nguyên nhân thật.

### Bằng chứng, theo thứ tự thu thập

1. Tệp có thật trên đĩa; 0/368.866 mục trong chỉ mục có tên chứa `a-lady-enjoying`.
2. Thư mục đó: 125 tệp trên đĩa, 51 trong chỉ mục, 74 tệp mới hơn mốc chỉ mục — `51+74=125`.
3. `src-tauri/src/` **không đổi một dòng** giữa v1.0.4 và v1.0.5; cây `index/` cùng SHA
   `9d6facaf…` ở cả hai tag. Về mặt toán học, v1.0.5 không thể gây lỗi tìm kiếm.
4. Nhật ký sáng 28/8: chỉ mục chỉ còn **48.319 tệp** (ổ cục bộ), **320.505 mục ổ mạng đã biến mất**.
5. `nsis-hooks.nsh` xoá `index.bin`; trong template Tauri, móc được chèn **vô điều kiện** — ngoài
   chốt mà chính Tauri dùng cho dữ liệu ứng dụng của nó.
6. Cập nhật trong app truyền `/UPDATE` → bỏ qua uninstaller (an toàn). **Cài tay đè lên** thì không
   → trang chọn hiện ra với "Uninstall before installing" **tích sẵn** → móc chạy → mất chỉ mục.

### Sửa

Cả hai móc tính một cờ chung trước mọi việc phá huỷ, dựa trên `$EXEDIR` vs `$INSTDIR` (NSIS chỉ chạy
uninstaller tại chỗ khi bộ cài gọi kèm `_?=`), `$UpdateMode`, và ô xoá-dữ-liệu người dùng tự tích.
Cài đè nay giữ cả chỉ mục lẫn tác vụ định kỳ.

Ghi chú phát hành bỏ câu sai "Chỉ mục đã quét vẫn giữ nguyên", nói rõ các bản **từ v1.0.5 trở về
trước** vẫn xoá khi cài tay đè lên, và chỉ đường phục hồi.

`src-tauri/tests/installer_hooks.rs` khoá bốn bất biến của tệp `.nsh` — tệp mà vòng kiểm tra không
có trình biên dịch nào kiểm hộ.

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


---

## P28 — Điều tra tiếp: lỗi "10/16 từ" cũng xảy ra trên ổ cục bộ 🔵 (điều tra xong, chưa sửa)

**Người dùng nói thêm:** *"không chỉ có ở trên nas mà vấn đề này còn bị ngay ở trên ổ"*, và yêu cầu
test cực kỳ chi tiết, có đọc log, và **lái chính bản đã cài** chứ không phải bản dev.

Câu bổ sung đó loại BUG-024 khỏi vai trò nguyên nhân duy nhất: cài đè tay chỉ xoá dữ liệu rồi lượt
quét đầu dựng lại **ổ cục bộ**, nên vế ổ cục bộ phải có nguyên nhân khác.

### Lái app thật

`SendKeys` không tới được nội dung WebView2 (lượt đầu ô tìm kiếm rỗng trong ảnh chụp). Đổi sang đặt
clipboard rồi bắn `Ctrl+V` bằng `keybd_event`; chụp bằng `PrintWindow(PW_RENDERFULLCONTENT)`. Đối
tượng: bản đã cài, FileVersion 1.0.5.

Tám truy vấn, chi tiết ở `docs/test-log.md` (lượt P28). Hai kết quả cốt lõi:

* **Tái hiện đúng triệu chứng người dùng báo** — dán nguyên tên tệp: băng vàng "khớp đủ 16 từ…
  10/16", 22 kết quả sai.
* **Tái hiện được cả vế ổ cục bộ** — tạo `D:\mf-test-p28\…zxqw.mp4` rồi tìm ngay: *"Không tìm thấy
  kết quả nào"*. Chạy tay tác vụ làm mới (+90 tệp) rồi tìm lại: ra ngay, 2 kết quả · 3,2 ms.

Cặp đối chứng đó khoá chặt nguyên nhân vào **tuổi chỉ mục**. Năm truy vấn khó khác (tên 29 từ tiếng
Pháp có dấu, tên có khoảng trắng và chữ hoa, tên viết lẫn hoa thường, cả ổ cục bộ lẫn NAS) đều ra
đúng 1 kết quả trong dưới 6 ms — bộ tìm kiếm không hỏng.

### Nguyên nhân (BUG-025)

| Loại ổ | Đường làm mới | Khoảng mù |
|---|---|---|
| Ổ mạng | **không có đường tự động nào** — `scan_network_volumes()` chỉ có 1 nơi gọi: lệnh IPC sau nút **+ Ổ mạng** | vô hạn, tới khi có người bấm |
| Ổ cục bộ (bản đã phát hành) | tác vụ `--index`, lịch `DaysInterval 1` **không có `Repetition`** | tới 24 giờ |
| Ổ cục bộ (nhánh `edit`) | thêm `Repetition PT15M` — **chưa gộp `master`, chưa phát hành** | 15 phút |

Đo trên thư mục người dùng đang tìm: 125 tệp trên đĩa · 51 trong chỉ mục · 74 thiếu, ranh giới nằm
đúng ở mốc quét ổ mạng 11:23:05; tệp họ tìm đến ổ lúc 13:48:49.

### Hai chỗ mù phát hiện thêm

* **Bản đã phát hành không ghi log.** `src-tauri/src/diag.rs` có trên `edit` nhưng không có
  trong `master` lẫn tag `v1.0.5`, nên
  `logs/mediafinder.log` trên máy người dùng trống. Trên máy này log đứng im ở 15:28 trong khi tác
  vụ vẫn chạy 16:00 / 16:15 / 16:21. Chẩn đoán từ xa đang mù.
* **`stats.unresolved` có thể mất dữ liệu vĩnh viễn.** Thay đổi tra không ra thư mục cha bị bỏ; nếu
  cùng lượt có tệp khác được thêm thì cache được ghi và con trỏ journal tiến qua, mất luôn tới lượt
  quét đầy đủ kế tiếp. Log cho thấy nó bắn thật: 2 / 3 / 26 / 73. Chưa dựng được ca tái hiện — ghi
  làm đầu mối.

### Chưa sửa gì

Lượt này chỉ điều tra và ghi chép, theo đúng yêu cầu "đưa ra kết quả cho tôi". Bốn hướng sửa đề xuất
nằm ở cuối BUG-025 trong `docs/bug.md`, chờ chốt.

### Dọn hiện trường

`D:\mf-test-p28\` đã xoá, chạy lại tác vụ làm mới thấy `+5 −2` đúng như mong đợi. Bốn ví dụ dò tạm
(`probe_find`, `probe_local`, `probe_search`, `probe_walk`) đã xoá khỏi `src-tauri/examples/`.



---

## P29 — Vá chốt chặn cho v1.0.6, và nói thật về tuổi chỉ mục 🟡 (chờ duyệt ở working tree)

**Chủ dự án phân vân giữa bốn hướng sửa BUG-025**, nên lượt này cân nhắc trước khi viết mã: bốn
lăng kính độc lập (người dùng cuối · rủi ro kỹ thuật · chi phí vận hành · khả năng chẩn đoán), một
lượt soi hướng bị bỏ sót, một lượt phản biện chính lộ trình vừa dựng.

### Bốn lăng kính mâu thuẫn nhau, và ba mâu thuẫn tự tan khi xếp đúng thứ tự

| Lăng kính | Xếp hạng | H1 / H2 / H3 / H4 |
|---|---|---|
| Người dùng cuối | 1 › 2 › 3 › 4 | **8** / 7 / 5 / 1 |
| Rủi ro kỹ thuật | 3 › 4 › 2 › 1 | 2 / 6 / **9** / 8 |
| Chi phí vận hành | 2 › 4 › 3 › 1 | 6 / **9** / 7 / 8 |
| Khả năng chẩn đoán | 4 › 3 › 2 › 1 | 1 / 2 / 7 / **9** |

Không lấy trung bình cộng. Lăng kính chẩn đoán chấm H2 thấp vì "phát hành PT15M một mình biến một
lỗi tái hiện được trong 2 phút thành lỗi hiếm không còn dụng cụ đuổi theo" — tan biến khi H4 đi
**cùng chuyến**. Lăng kính người dùng chấm H4 thấp vì editor không thấy gì — cũng tan biến, vì H4
đi nhờ chuyến phát hành BUG-024 vốn bắt buộc phải đi. H3 bị chấm 5/9 vì hôm nay nó "chỉ vào một
cánh cửa khoá": nút "+ ổ mạng" lần nào cũng bật UAC rồi bắt chờ trọn lượt quét cục bộ.

### Phản biện bắt được lỗi trong chính lộ trình

Lộ trình định bỏ bước `/Delete` trong `upgrade_schedule_if_stale`, gọi là "một dòng". Sai:
`ensure_scheduled_task()` mở đầu bằng `if scheduled_task_exists() { return true; }`, mà nâng lịch
thì **luôn** gặp một tác vụ đã tồn tại. Bỏ `/Delete` mà vẫn gọi hàm ấy nghĩa là XML mới không bao
giờ được ghi — máy mang lịch v1 ghi log "nâng lên lịch v2" mãi mãi, lịch không đổi. Đúng hình dạng
lỗi `SCHEDULE_MARK` đã trả giá ở P22.

### Đã làm

1. **Tách `write_task_definition()`** khỏi chốt `exists`; `upgrade_schedule_if_stale` gọi thẳng nó
   và **không xoá trước** (`/Create /XML /F` tự ghi đè, còn xoá-rồi-tạo để lại một cửa sổ máy không
   có tác vụ nào).
2. **Tệp tạm mang PID** ở cả ba chỗ: `index.bin.tmp`, `progress json.tmp`, `mediafinder-task.xml`.
3. **`#[serde(default)]` cho `NetScanMark`** — thêm trường sau này không được xoá mốc quét NAS trên
   mọi máy.
4. **`.gitattributes` cho `persist.rs`** — tệp chứa 3 byte NUL thật trong `MAGIC` nên git coi là
   nhị phân; `git diff --stat` đi từ `Bin 9365 → 10316 bytes` thành `12 +++++++++++-`.
5. **`src-tauri/src/taskhealth.rs`** (module riêng) + lệnh IPC `task_health` — trả lời "tác vụ định
   kỳ còn không". Cố ý **không** phân tích `schtasks /Query /FO LIST /V`: tên trường trong đầu ra đó
   được bản địa hoá, nên trên máy chạy Windows tiếng Việt phép so chuỗi sẽ trượt — im lặng đúng
   trên những máy cần chẩn đoán nhất. Chỉ dùng mã thoát.
6. **`src/lib/freshness.ts` + `src/lib/FreshnessNote.svelte`** (component riêng) — nói tuổi chỉ mục
   ở **cả hai** trạng thái hỏng, và chỉ lên tiếng khi có gì đáng nói.
7. **Chân cửa sổ thôi nói dối** — in hai mốc tách bạch thay vì một mốc `builtAtUnix` gọi là "quét
   lúc". `persist.rs` đóng dấu `built_at_unix = now_unix()` ở mọi lần ghi, kể cả lượt vá gia tăng
   cục bộ, nên câu cũ sai 96 lần/ngày sau khi có PT15M.
8. **`RELEASE_NOTES.md`** viết lại cho v1.0.6, nói thẳng rằng bản này **chưa** sửa vế ổ mạng.

### Một quyết định của chủ dự án từng bị lật im lặng

Chủ dự án nhắc lại rằng họ đã yêu cầu **1 ngày chỉ quét 2 lần**. Tra lại `docs/test-log.md` (lượt
P9) thì đúng: *"Lịch cuối cùng (theo lựa chọn của chủ máy): khi đăng nhập (trễ 1 phút) và 13:00
hằng ngày — tức 1–2 lần mỗi ngày."* **P22 đã thay nó bằng `PT15M` mà không hỏi lại.**

Đưa số liệu ra để chốt lại thay vì tranh luận: một lượt PT15M là đọc nhật ký USN (0,2–2 giây),
không phải quét đầy đủ; nhưng khi có thay đổi thật thì ghi lại trọn 46 MB cache — đo ngày 28/8:
16:00, 16:15, 16:21, 18:00, 20:45. Đổi lại, khoảng mù ổ cục bộ là 15 phút thay vì ~12 giờ, tức
chính cái đã gây ra khiếu nại mở đầu BUG-025.

**Chủ dự án xem số liệu rồi chốt: giữ PT15M.** Ghi chú phát hành nói "mỗi 15 phút" là đúng sự thật.
Bài học không phải ở con số mà ở chỗ: một lựa chọn đã chốt của chủ dự án chỉ được đổi khi hỏi lại
họ, kể cả khi kỹ thuật thấy nên đổi.

### Chưa làm, có chủ đích

Hướng 1 (quét NAS nền định kỳ) **không được phát hành ở dạng hiện tại**. Trong
`scan_network_volumes`, `walked.push()` chạy vô điều kiện và tập `touched` dựng từ `walked`, nên một
ổ trả về tập rỗng hoặc dở vẫn **xoá sạch mục cũ của ổ đó** rồi báo thành công. Chạy tay thì người
dùng ngồi nhìn thấy; chạy nền thì 320.000 mục biến mất trong im lặng. Cứng hoá chỗ đó là bước bắt
buộc trước bất kỳ việc gì làm lượt quét NAS rẻ hơn hoặc thường xuyên hơn.

### Nghiệm thu

Vòng trước-commit xanh: `cargo test` 259 · clippy 0 warning · fmt sạch · `npm run check` 0 lỗi/125
tệp · `npm test` 161/161 (thêm nhóm t15). Sáu đột biến, mỗi cái đỏ đúng một bài. Lái bản dựng thật
bằng chuột: cả ba chỗ đều hiện hai mốc, đúng ca người dùng báo.

Hai đường cần quyền Administrator đã chạy trên máy thật, **cả hai đạt**:

* **Nâng lịch v1 → v2**: dựng tác vụ đúng hình dạng máy người dùng (`PT15M: False | marker: False`),
  chạy `--index` của bản dựng mới → `PT15M: True | marker: True`. Nếu làm theo đề xuất ban đầu, bước
  này sẽ in `False` và v1.0.6 ra đời với tính năng chính chết lặng.
* **Cảnh báo mất tác vụ**: xoá hẳn tác vụ → ứng dụng hiện *"Không còn tác vụ làm mới định kỳ trên máy
  này — chỉ mục sẽ không tự cập nhật nữa. Bấm Quét lại một lần để tạo lại nó."*

Tác vụ được khôi phục sạch sau cả hai bài (`hop le: True | tro vao: ban da cai`).


## Sai lệch so với kế hoạch

| Ngày | Mục | Kế hoạch | Thực tế | Lý do |
|------|-----|----------|---------|-------|
| 2026-08-24 | Phạm vi tìm kiếm | chỉ **tên tệp** (đặc tả mục 3.3) | tìm cả **đường dẫn thư mục** | Dữ liệu thật tổ chức theo kiểu tên thư mục mang toàn bộ ý nghĩa, tên tệp chỉ là số. Tìm theo tên tệp trả về gần như rỗng. Xem `SPEC-001`. |
| 2026-08-24 | Lọc thư mục | danh sách tên cố định | thêm quy tắc `skip_dot_directories` | Một quy tắc thay cho danh sách phải bổ sung mãi mãi; bao phủ cả công cụ cài sau này. Xem `ISSUE-001`. |
| 2026-08-24 | `MediaKind` | thuộc P2 | kéo sớm sang P1 | Bộ lọc phần mở rộng chạy ngay lúc quét MFT, viết bảng này hai lần là vô lý. |
| 2026-08-27 | Phát hành bản vá update-check | cắt v1.0.3 và publish ngay | cắt tag nhưng bỏ nháp, gộp vào v1.0.4 | Đợt hộp thoại cập nhật hoàn thành trước khi kịp publish; gộp lại thì người dùng chỉ phải cài tay một lần |
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

### 2026-08-26 → 27 — P16–P21

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Toàn bộ kiểm thử JS | `npm test` (9 nhóm t1–t9) | **85/85 pass** |
| Toàn bộ kiểm thử Rust | `cargo test --lib` | **226 pass, 0 fail** (3 test thumbnail + 4 test update mới) |
| Type-check | `svelte-check` + `tsc --noEmit` | 0 lỗi, 0 warning, 119 file |
| Kiểm thử đột biến | cố tình phá từng nhánh (LIFO, retry, skip, guard, armed, settle…) | **>20 phép, tất cả bị bắt**; 1 phép từng lọt (Busy↔Unavailable) đã vá bằng test Rust ép hàng-đầy tất định |
| CSS sau refactor | đối chiếu bundle cũ/mới, bỏ hash scope | 0 selector mất; 7 quy tắc lưới chuyển prop **khớp từng thuộc tính** |
| App thật sau refactor | `npm run tauri dev`, thư viện thật | 48.303 tệp, log sạch, ổn định >15 phút |
| Chốt chặn ghi chú | `scripts/check-release-notes.sh` với file thật / dài 2.460 ký tự / rỗng | pass / **chặn đúng lý do** / chặn |
| v1.0.2 phát hành | tag → CI 16 ph → publish → `latest.json` từ ngoài | version 1.0.2 + chữ ký đủ, `.exe` HTTP 200 |
| v1.0.5 trọn vòng | người dùng thao tác trên máy chạy v1.0.4 | ✅ hộp thoại đúng ghi chú → Cập nhật → tải % → tự khởi động lại → footer `v1.0.5` |
| Sau sập nguồn (27/08) | `git fsck` + chạy lại toàn bộ test | repo nguyên vẹn, 76/76 (thời điểm đó) pass — không mất byte nào |

### 2026-08-27 (chiều) — P22

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Vòng trước-commit đủ bốn lệnh | `cargo test` · `clippy --all-targets` · `fmt --check` · `npm run check` | 236 pass · **0 warning** · sạch · 0 lỗi/120 file |
| Kiểm thử JS toàn cục | `npm test` (10 nhóm, thêm t10) | **94/94 pass** |
| Ô chờ-lắng của misslog | test đồng hồ tuỳ ý: chặng gõ dở bị thay, đứng yên 30 ms được ghi, tìm-thấy xoá ô chờ | 5/5 pass |
| Kẻ giả dạng cùng-hai-đầu | tệp 4 KiB khác đúng 1 byte ở bụng | tầng 3 tách đúng cụm; UI chỉ mặt đúng tệp |
| Xoay vòng nhật ký | file >5 MB dịch xuống `.1`, bản `.5` cũ rơi khỏi mép | pass |
| Nghiệm thu tay A | file log trên đĩa sau một phiên chạy | ghi liên tục, không màu ANSI, đủ dòng |
| Nghiệm thu tay B | chạy suốt phiên chưa bật bộ ghi | **không tạo file `misses.*` nào** — mặc-định-tắt đúng ngoài đời |
| Nghiệm thu tay C | 3 tệp thật × 8 MB, kẻ giả dạng khác 1 byte ở bụng | **30 ms** (~800 MB/s), tách đúng cụm |
| Nghiệm thu tay D | người dùng chạy task elevated trên máy thật | **lộ BUG-P22-01** (marker có dấu → vòng lặp xoá-tạo-lại); sửa xong, chạy lại 2 lần: log chỉ 1 dòng nâng cấp |
| Đột biến P22 | 7 phép: bỏ chờ-lắng · hash-hằng · tắt xoay · gỡ Repetition · đọc cụm sai · nút không gọi backend · marker có dấu | **7/7 bị bắt** (2/1/1/1/3/1/1 ca đỏ), khôi phục giống hệt từng byte |

### 2026-08-28 — P23

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Vòng trước-commit | `cargo test` · `clippy` · `fmt --check` · `npm run check` | 241 pass · 0 warning · sạch · 0 lỗi/122 file |
| Kiểm thử JS | `npm test` (11 nhóm, thêm t11) | **112/112 pass** |
| Đột biến P23 | 5 phép: UNC mất hai gạch · ổ mạng không xuống cuối · bàn phím đọc danh sách thô · không reset con trỏ · chip hiện khi chỉ một ổ | 4 bị bắt ngay; **phép "ổ mạng xuống cuối" LỌT** |
| Vá khoảng trống | dữ liệu cũ có C: < D: < NAS nên xếp chữ cái tình cờ trùng kết quả đúng | thêm ca `\ALPHA` vs `Z:` ép quy tắc lộ ra → đột biến đỏ đúng chỗ; **5/5** |
| Bug tự tìm ra khi chạy test | `scrollToTop()` gọi `viewport.scrollTo` — jsdom không có, effect ném lỗi giữa chừng sau khi `selectOnly(0)` đã chạy | đổi sang gán `scrollTop = 0`: cùng kết quả, không phụ thuộc một phương thức có thể vắng mặt; 8 lỗi ngầm biến mất |

### 2026-08-28 (chiều) — P24

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Vòng trước-commit | `cargo test` · `clippy` · `fmt --check` · `npm run check` | 242 pass · 0 warning · sạch · 0 lỗi/123 file |
| Kiểm thử JS | `npm test` (12 nhóm, thêm t12) | **121/121 pass** |
| Đột biến P24 | 4 phép: bỏ qua hộp thoại · không nhường bàn phím · giây không đổi ra phút · lượt huỷ vẫn ghi dấu vết | 3 bị bắt ngay; **phép "lượt huỷ" LỌT** |
| Vá khoảng trống | quy tắc nằm trong một `if` ở `lib.rs` nên test chỉ *mô phỏng* lại được — mô phỏng không bao giờ đỏ khi bản thật bị sửa | tách thành `netscan_mark::record_outcome`, test gọi thẳng hàm thật → đột biến đỏ đúng chỗ; **4/4** |
| Test cũ vỡ theo thiết kế mới | TC-2.13 giả định bấm "+ ổ mạng" là quét ngay | cập nhật để đi qua hộp thoại — giả định cũ giờ sai, không phải code sai |

### 2026-08-28 (tối) — P25

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Vòng trước-commit | `cargo test` · `clippy` · `fmt --check` · `npm run check` | 246 pass · 0 warning · sạch · 0 lỗi/123 file |
| Kiểm thử JS | `npm test` (13 nhóm, thêm t13) | **129/129 pass** |
| Đột biến P25 | 4 phép: so metadata sau khi đã thay · dòng tin không tự tắt · lượt mới không dọn tin cũ · tooltip bịa lần quét | **4/4 bị bắt** ngay lượt đầu (2/1/1/8 ca đỏ) |

### 2026-08-28 (đêm) — P26 rà soát toàn bộ

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Vòng trước-commit | `cargo test` · `clippy` · `fmt --check` · `npm run check` | 246 pass · 0 warning · sạch · 0 lỗi/123 file |
| Kiểm thử JS | `npm test` (14 nhóm, thêm t14) | **142/142 pass** |
| Rà độ phủ bằng đột biến | phá 6 nhánh khắp các module: ContextMenu ×3, FirstRun, ScanStatusBar, VirtualList | 3 bị bắt sẵn; **3 lọt — đều ở ContextMenu** |
| Sau khi bù t14 | phá lại đúng 3 nhánh đó + 1 nhánh biên mới | **4/4 bị bắt** |
| Rà tiếp các module lõi | ScanState (không dừng timer), prefs (sai khoá lưu), drives (không viết hoa chữ ổ) | 3/3 bị bắt sẵn — không có khoảng trống |

### 2026-08-28 (khuya) — P27

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Vòng trước-commit | `cargo test` · `clippy` · `fmt --check` · `npm run check` | **250 pass** · 0 warning · sạch · 0 lỗi/123 file |
| Kiểm thử JS | `npm test` | 142/142 pass |
| Tệp có thật mà chỉ mục không biết | tra thẳng chỉ mục 368.866 mục + đọc đĩa | 0 khớp; 51+74=125 khớp tuyệt đối |
| v1.0.5 có gây lỗi không | `git diff v1.0.4 v1.0.5 -- src-tauri/src/` | **trống**; cây `index/` cùng SHA → không thể |
| Đường cập nhật nào an toàn | đọc template NSIS của tauri-bundler 2.7.1 + nguồn tauri-plugin-updater 2.10.1 | in-app truyền `/UPDATE` → an toàn; cài tay → chạy uninstaller → mất dữ liệu |
| Đột biến bản móc cũ | khôi phục `nsis-hooks.nsh` từ HEAD | **4/4 bài đỏ** — chốt chặn bắt đúng lỗi |

**Trạng thái git lúc ghi:** `master` = hết A+C+D (P21 nửa đầu); `edit` = +`60efce1` (P21 nửa sau,
chờ duyệt). `RELEASE_NOTES.md` còn là nội dung 1.0.5 — **phải viết lại trước khi cắt v1.0.6**
(quên thì CI tự chặn). Mục 7 lộ trình (Thùng rác cho tệp trùng) chưa làm, chờ lệnh.

### 2026-08-28 (chiều muộn) — P28 lái app thật

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Tái hiện triệu chứng người dùng báo | dán nguyên tên tệp vào bản đã cài v1.0.5 | băng "khớp đủ 16 từ… **10/16**", 22 kết quả sai · 13,6 ms |
| Tái hiện vế **ổ cục bộ** | tạo `.mp4` trên `D:` rồi tìm ngay | "Không tìm thấy kết quả nào" |
| Đối chứng | chạy tay tác vụ làm mới (+90 tệp), tìm lại cùng truy vấn | ra ngay, 2 kết quả · 3,2 ms |
| Bộ tìm kiếm có hỏng không | 5 truy vấn tên đầy đủ: 14 từ, 29 từ tiếng Pháp có dấu, có khoảng trắng + chữ hoa, viết lẫn hoa thường, tệp NAS | **5/5 đúng 1 kết quả**, 3,7–5,7 ms, không lần nào hiện băng |
| Tệp người dùng tìm có thật không | `find` trên `Y:` | có: đến ổ 13:48:49; quét ổ mạng gần nhất 11:23:05 |
| Thư mục đó lệch bao nhiêu | đối chiếu đĩa với chỉ mục | 125 trên đĩa · 51 trong chỉ mục · 74 thiếu (51+74=125) |
| Ổ mạng có tự làm mới không | truy mọi nơi gọi `scan_network_volumes()` | đúng **1** nơi: lệnh IPC sau nút "+ Ổ mạng" → **không có đường tự động** |
| Lịch ổ cục bộ bản đã phát hành | `git show v1.0.5:src-tauri/src/setup.rs` | `DaysInterval 1`, **không có `Repetition`** → mù tới 24 giờ |
| Bản đã phát hành có ghi log không | so `LastWriteTime` của log với các lượt chạy tác vụ | log đứng im 15:28 trong khi tác vụ chạy 16:00 / 16:15 / 16:21 → **không ghi** |
| Dọn hiện trường | xoá tệp thử, chạy lại tác vụ | `+5 −2` — hai tệp thử rời chỉ mục đúng như mong đợi |

### 2026-08-28 (tối) — P29 vá chốt chặn

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Vòng trước-commit | `cargo test` · `clippy --all-targets` · `fmt --check` · `npm run check` | **259 pass** · 0 warning · sạch · 0 lỗi/125 tệp |
| Kiểm thử JS | `npm test` (15 nhóm, thêm t15) | **161/161 pass** |
| Bẫy nâng lịch có thật không | đọc `setup.rs:197` — `ensure_scheduled_task` thoát sớm ở `scheduled_task_exists()` | có: bỏ `/Delete` mà vẫn gọi hàm đó thì `/Create /XML /F` không bao giờ chạy |
| Chốt chặn có bắt được không | khôi phục lời gọi `ensure_scheduled_task()` trong đường nâng lịch | **đỏ đúng 1/5 bài**, 4 bài kia xanh |
| Tệp tạm | đưa `index.bin.tmp` về đường dẫn cố định | **đỏ đúng 1 bài** |
| `serde(default)` | nạp tệp `netscan.json` chỉ có 2 trường kiểu bản cũ | đọc được, trường thiếu nhận mặc định |
| Diff `persist.rs` có xem được không | `git diff --stat` trước/sau `.gitattributes` | `Bin 9365 → 10316 bytes` **⇒** `12 +++++++++++-` |
| Đột biến giao diện | gỡ note khỏi nhánh 0 kết quả · chân cửa sổ về một mốc · doạ nhầm khi `health = null` · luôn lên tiếng | **4/4 đỏ đúng bài**, mỗi lần đúng một bài |
| Test cũ đỏ vì đếm tuyệt đối | `t12` đổi sang đo độ tăng, rồi bỏ lượt đọc lại trong `beginNetScan` | vẫn **đỏ** — sửa mà không làm mềm |
| Bản dựng thật, trạng thái 0 kết quả | lái chuột trên `tauri build --no-bundle` | "Ổ trong máy: 1 phút trước · Ổ mạng: **6 giờ trước**" |
| Bản dựng thật, băng 10/16 | dán đúng truy vấn người dùng báo | băng vàng kèm hai mốc, phần cũ tô hổ phách |
| Bản dựng thật, chân cửa sổ | đọc dòng chân | `ổ trong máy 18:00:01 · ổ mạng 11:23:05 28/8/2026` — hết nói dối |
| Đường nâng lịch v1 → v2 chạy thật | dựng task lịch v1, chạy `--index` bản mới | `PT15M: False` → **`PT15M: True`** — **ĐẠT** |
| Cảnh báo mất tác vụ | xoá hẳn task, mở app, tìm tệp không tồn tại | hiện đúng câu cảnh báo + nút cần bấm — **ĐẠT** |
| Máy có bị bỏ lại ở trạng thái hỏng không | đọc lại task sau mỗi bài | `ton tai: True · hop le: True · tro vao: ban da cai` |
| Chính script kiểm thử có đúng không | thử hàm kiểm tồn tại với một tên task bịa ra | **trả `True`** — lỗi `%ERRORLEVEL%`; sửa rồi mới tin kết quả |

### 2026-08-29 (rạng sáng) — P30–P31 rà soát và sửa

| Kiểm chứng | Cách đo | Kết quả |
|---|---|---|
| Vòng trước-commit | `cargo test` · clippy · fmt · `npm run check` | **272 pass** · 0 warning · sạch · **0 lỗi 0 cảnh báo**/125 tệp |
| Kiểm thử JS | `npm test` (15 nhóm) | **170/170** (từ 142 ở P26) |
| Bộ cài NSIS thật | `npm run tauri build`, cài 1.0.6 đè 1.0.5 | tái hiện đúng bẫy BUG-024; **thủ phạm là bộ gỡ CŨ**, không phải móc mới |
| Bản sửa BUG-024 có chạy không | bộ gỡ MỚI, `uninstall.exe /S _?=<thư mục>` | `index.bin` 48.074.384 + `metadata.bin` 14.811.580 **còn nguyên** |
| Bản sửa có bảo vệ quá đà không | gỡ THẬT (không `_?=`) | thư mục xoá sạch, registry mất — đúng |
| Ảnh thu nhỏ hiện sai tệp | đọc `thumbnail.rs:103` + `get()` | khoá theo **vị trí**, `get()` không đọc `path` — có thật |
| Chốt chặn khoá cache | khoá không phụ thuộc đường dẫn | **đỏ đúng 1/11 bài** |
| Ổ mạng ánh xạ có được nhận không | `net use` + `driveKey("Y:\…")` | `isNetworkDrive("Y")` = **false** — cả nhánh là mã chết |
| Tầng 3 có nói sai không | đọc `allSame()` + bài t10 | nói sai, **và bài t10 đang khoá hành vi sai** |
| `npm test` có trong CI không | đọc cả hai workflow | **không** — đã thêm |
| Lớp phủ khi chỉ mục đổi | bỏ `preview = false; menu = null;` | **cả hai vế** đỏ đúng thông điệp |
| Vết hỏng bám sang tệp sau | bỏ việc đặt lại trạng thái | đỏ đúng bài |
| Bản dựng thật | lái chuột trên `--no-bundle` | "Ổ trong máy: 11 phút trước · Ổ mạng: 11 giờ trước" |
| Đóng dấu `lastcheck.json` từ `--index` thật | cần Administrator | **chưa chạy** — 6 bài đơn vị + 2 chốt đọc mã canh điểm gọi |
