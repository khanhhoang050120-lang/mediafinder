# LỖI — MediaFinder
> **Thuộc file này:** Code chạy ra kết quả sai so với thiết kế: sai kết quả, crash, treo, cấp phát hỏng.
> **KHÔNG thuộc file này:** xung đột cấu hình/phiên bản, rủi ro chưa xảy ra, vấn đề tốc độ, nghi ngờ chưa kiểm chứng, quyết định sản phẩm, lỗi của bản đặc tả.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `BUG-008`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [BUG-001](#bug-001) | 🟡 | `MainWindowHandle` bắt nhầm cửa sổ event-loop của tao | P0 | WORKAROUND |
| [BUG-002](#bug-002) | 🟠 | Cửa sổ mở ở trạng thái minimize | P0 | CẦN XÁC MINH |
| [BUG-003](#bug-003) | 🟡 | `SetForegroundWindow` bị chặn → chụp nhầm màn hình | P0 | ĐÃ SỬA |
| [BUG-004](#bug-004) | 🔴 | `.ts` (TypeScript) bị phân loại thành video | P1 | ĐÃ SỬA |
| [BUG-005](#bug-005) | 🟡 | Tiến độ quét báo trùng bản ghi cuối | P1 | ĐÃ SỬA |
| [BUG-006](#bug-006) | 🟠 | Fold tách âm tiết Hangul thành Jamo, không ghép lại | P2 | ĐÃ SỬA |
| [BUG-007](#bug-007) | 🔴 | `BinaryHeap::with_capacity(limit+1)` cấp phát không giới hạn | P2 | ĐÃ SỬA |

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

**Quan sát bổ sung ở P3 (2026-08-24).** Chạy lại nhiều lần, vẫn từ tiến trình nền:

| Thời điểm đo | `IsIconic` |
|---|---|
| Ngay sau khi cửa sổ mở | **False** |
| Sau vài thao tác UI Automation | **True** |

Lần này cửa sổ mở ra **không** bị thu nhỏ — khác hẳn lần đo ở P0. Nó chỉ chuyển sang trạng thái
thu nhỏ về sau, sau khi mất focus.

Điều này **củng cố** giả thuyết ban đầu: đây là hệ quả của việc tiến trình nền không giữ được
foreground, chứ không phải ứng dụng tự thu nhỏ lúc khởi động. Nhưng vẫn chưa đủ để đóng bug —
cần chính người dùng chạy từ terminal của mình và quan sát.

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

## BUG-006 🟠 — Fold tách âm tiết Hangul thành Jamo, không ghép lại

**Giai đoạn:** P2 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Test thất bại với thông báo trông như vô lý:

```
assertion `left == right` failed
  left: "한국어"
 right: "한국어"
```

Hai chuỗi hiển thị **giống hệt nhau** nhưng khác nhau ở mức byte.

**Nguyên nhân.** Hàm fold chạy NFD rồi lọc bỏ combining mark. Nhưng NFD tách **nhiều hơn** dấu
phụ Latin: một âm tiết Hangul như `한` bị tách thành ba Jamo riêng biệt (`ᄒ` + `ᅡ` + `ᆫ`), mà
Jamo **không phải** combining mark nên chúng sống sót qua bộ lọc. Kết quả là chuỗi ở dạng phân
rã — nhìn thì giống, byte thì khác.

**Ảnh hưởng.** Không làm sai kết quả tìm kiếm (cả query lẫn index đều phân rã như nhau nên vẫn
khớp). Nhưng: chuỗi folded của tên CJK **phình gấp 3 lần** trong arena, và giá trị trả về không
còn khớp với thứ người dùng nhìn thấy — sẽ gây rắc rối khi cần highlight đoạn khớp ở P3.

**Cách sửa.** Thêm bước ghép lại **NFC** ở cuối: NFD → bỏ mark → map ký tự không phân rã →
lowercase → **NFC**. Bước này khôi phục `한`, còn `ế` vẫn là `e` vì dấu đã bị bỏ, không còn gì để
ghép lại.

**Bài học.** NFD được chọn để tách dấu phụ tiếng Việt, nhưng nó là phép biến đổi **toàn cục** —
nó cũng làm những việc mình không hề yêu cầu với các hệ chữ khác. Test tiếng Hàn được viết chỉ
để "kiểm tra fold không phá tên nước ngoài", vậy mà lại bắt được đúng vấn đề này.

---

## BUG-007 🔴 — `BinaryHeap::with_capacity(limit+1)` cấp phát không giới hạn

**Giai đoạn:** P2 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Test `spans_many_chunks_correctly` (truyền `limit` rất lớn để lấy hết kết quả)
thất bại khi cấp phát.

**Nguyên nhân.** Mỗi chunk song song khởi tạo heap bằng
`BinaryHeap::with_capacity(opts.limit + 1)`. Với `limit` lớn, **mỗi chunk** cố cấp phát một vùng
nhớ khổng lồ — và có hàng chục chunk chạy song song.

**Mức độ nghiêm trọng.** Nặng. Đây không phải lỗi chỉ xảy ra trong test: bất kỳ lời gọi nào từ
tầng IPC truyền `limit` lớn (dù vô tình) đều làm ứng dụng cạn RAM. Và vì `panic = "abort"` ở bản
release (xem `RISK-001`), nó sẽ **giết cả app** chứ không trả lỗi.

**Cách sửa.** Chặn trần capacity theo kích thước chunk: `opts.limit.min(end - start) + 1`.
Một chunk không thể sinh ra nhiều kết quả hơn số mục nó chứa, nên đây vừa là cận đúng vừa là cận
chặt.

**Ghi chú.** Bug này chỉ lộ ra vì test cố tình dùng `limit` cực lớn để kiểm tra ranh giới chunk —
tức là **một test viết cho mục đích khác** lại bắt được lỗi. Nếu chỉ test bằng `limit` mặc định
5.000 thì nó sẽ nằm im tới tận khi có người dùng thật gặp phải.
