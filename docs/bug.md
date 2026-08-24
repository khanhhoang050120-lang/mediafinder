# LỖI — MediaFinder
> **Thuộc file này:** Code chạy ra kết quả sai so với thiết kế: sai kết quả, crash, treo, cấp phát hỏng.
> **KHÔNG thuộc file này:** xung đột cấu hình/phiên bản, rủi ro chưa xảy ra, vấn đề tốc độ, nghi ngờ chưa kiểm chứng, quyết định sản phẩm, lỗi của bản đặc tả.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `BUG-014`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [BUG-001](#bug-001) | 🟡 | `MainWindowHandle` bắt nhầm cửa sổ event-loop của tao | P0 | WORKAROUND |
| [BUG-002](#bug-002) | 🟡 | Cửa sổ mở ở trạng thái minimize | P0 | **KHÔNG SỬA** (do môi trường) |
| [BUG-003](#bug-003) | 🟡 | `SetForegroundWindow` bị chặn → chụp nhầm màn hình | P0 | ĐÃ SỬA |
| [BUG-004](#bug-004) | 🔴 | `.ts` (TypeScript) bị phân loại thành video | P1 | ĐÃ SỬA |
| [BUG-005](#bug-005) | 🟡 | Tiến độ quét báo trùng bản ghi cuối | P1 | ĐÃ SỬA |
| [BUG-006](#bug-006) | 🟠 | Fold tách âm tiết Hangul thành Jamo, không ghép lại | P2 | ĐÃ SỬA |
| [BUG-007](#bug-007) | 🔴 | `BinaryHeap::with_capacity(limit+1)` cấp phát không giới hạn | P2 | ĐÃ SỬA |
| [BUG-008](#bug-008) | 🔴 | Lượt quét thất bại ghi đè chỉ mục tốt bằng chỉ mục rỗng | P4 | ĐÃ SỬA |
| [BUG-009](#bug-009) | 🔴 | `convertFileSrc` mã hoá `/` thành `%2F` → mọi thumbnail 400 trong im lặng | P5 | ĐÃ SỬA |
| [BUG-010](#bug-010) | 🔴 | Thumbnail video trả về ảnh full-res, 1,27 MB mỗi cái | P5 | ĐÃ SỬA |
| [BUG-011](#bug-011) | 🟠 | Video hiện icon chung của Windows thay vì khung hình thật | P5 | ĐÃ SỬA |
| [BUG-012](#bug-012) | 🟠 | `SCHEMA_VERSION` nằm trong chính khối dữ liệu nó bảo vệ | P6 | ĐÃ SỬA |
| [BUG-013](#bug-013) | 🔴 | Hàng đợi enrichment sắp xếp ngược — đọc nhạc trước video | P6 | ĐÃ SỬA |

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

## BUG-002 🟡 — Cửa sổ mở ở trạng thái minimize

**Giai đoạn:** P0 · **Trạng thái:** KHÔNG SỬA (không phải lỗi ứng dụng) · **Ngày:** 2026-08-24

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
foreground, chứ không phải ứng dụng tự thu nhỏ lúc khởi động.

**→ Đã xác minh và đóng (2026-08-24, P4).** Người dùng chạy `npm run tauri dev` từ terminal của
mình: **cửa sổ tự hiện lên bình thường.** Không phải lỗi ứng dụng — chỉ là hệ quả của việc tôi
khởi chạy từ tiến trình nền không có quyền foreground.

**Giữ lại mục này** thay vì xoá, vì nó ghi lại một giới hạn của môi trường kiểm thử: bất cứ khi
nào tôi tự chạy app để kiểm chứng, trạng thái cửa sổ đo được **không phản ánh** thứ người dùng
thấy. Đo nội dung thì tin được (dùng `PrintWindow`), đo trạng thái cửa sổ thì không.

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

---

## BUG-008 🔴 — Lượt quét thất bại ghi đè chỉ mục tốt bằng chỉ mục rỗng

**Giai đoạn:** P4 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Cách phát hiện.** Không phải do chạy hỏng mà do **đặt câu hỏi**: đang tìm cách kiểm chứng luồng
quét mà không cần UAC, tôi tự hỏi *"chạy `--index` không có quyền Admin thì chuyện gì xảy ra?"*
Lần theo luồng code thì thấy đường đi rất tệ.

**Cơ chế.**

```rust
for v in ntfs_volumes {
    let handle = match volume::open_volume(v) {
        Ok(h) => h,
        Err(e) => { tracing::error!("{e}"); continue; }   // bỏ qua ổ này
    };
    …
}
let ix = builder.finish();          // RỖNG nếu mọi ổ đều thất bại
persist::save(&ix, stamps)?;        // GHI ĐÈ cache tốt bằng cái rỗng
```

Bỏ qua một ổ hỏng để các ổ còn lại vẫn quét được là **đúng**. Nhưng nếu **tất cả** đều hỏng thì
chương trình vẫn chạy tới cuối với một index rỗng, rồi lưu nó đè lên cache đang dùng tốt.

**Khi nào xảy ra thật.**

- Chạy `mediafinder.exe --index` mà quên elevate.
- Một ổ bị khoá hoặc rút ra giữa chừng.
- USN Journal bị tắt trên mọi ổ.

**Hậu quả.** Người dùng mất toàn bộ chỉ mục và phải quét lại — mất thêm ~20 giây **và một lần UAC
nữa** — chỉ để lấy lại đúng thứ họ vừa có. Tệ hơn nữa là nó **im lặng**: cache mới vẫn ghi thành
công, app vẫn khởi động bình thường, chỉ là không tìm thấy gì cả.

**Cách sửa.** Đếm số ổ quét thành công. Nếu bằng 0 (hoặc index rỗng) thì **không lưu gì hết**,
báo lỗi và giữ nguyên cache cũ. Nếu chỉ một phần ổ thành công thì vẫn lưu nhưng nói rõ
*"chỉ quét được 1/2 ổ"* — mất một nửa chỉ mục trong im lặng cũng tệ.

**Kiểm chứng.** Sao lưu cache, chạy `--index` không elevate, đo lại:

```
Cache trước: 8.331.574 bytes
  → ổ C: bị từ chối truy cập — cần chạy với quyền Administrator
  → ổ D: bị từ chối truy cập — cần chạy với quyền Administrator
  → TỔNG: 0 tệp media
  → Không quét được ổ đĩa nào (2 ổ NTFS đều thất bại). Chỉ mục cũ được giữ nguyên.
Cache sau  : 8.331.574 bytes  ← NGUYÊN VẸN
```

**Chống tái phát.** `tests/cache_safety.rs` chạy đúng kịch bản đó và so kích thước cache trước/sau.
Test thứ hai kiểm tra lượt quét thất bại vẫn đặt `finished`, nếu không thanh tiến độ sẽ quay mãi.

**Bài học.** Đây là loại lỗi mà **không lượt chạy bình thường nào phơi ra được** — đường hạnh phúc
luôn có đủ quyền. Nó chỉ lộ khi hỏi *"nếu bước này thất bại thì sao?"* ở từng chỗ có `continue`
hoặc bỏ qua lỗi. Mỗi lần bỏ qua một lỗi là một lần ngầm khẳng định *"phần còn lại vẫn có nghĩa"* —
và ở đây lời khẳng định đó sai.

---

## BUG-009 🔴 — `convertFileSrc` mã hoá `/` thành `%2F` → mọi thumbnail 400 trong im lặng

**Giai đoạn:** P5 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Lưới ảnh dựng đúng — đủ cột, ảo hoá chạy, tên tệp hiện đủ — nhưng **không ô nào có
ảnh**. Không có thông báo lỗi, không có ảnh vỡ, chỉ là ô trống.

**Vì sao im lặng.** Ba lớp che lấp nhau:

1. Trình duyệt không báo gì khi `<img>` nhận HTTP 400.
2. `onerror` của tôi gắn class `failed` → `display: none`, nên ảnh hỏng bị **giấu đi** thay vì
   hiện biểu tượng vỡ. Đó là chủ ý (tệp không có preview thì không nên hiện ảnh vỡ), nhưng nó
   cũng giấu luôn lỗi thật.
3. Handler phía Rust trả `BAD_REQUEST` rồi thôi, không ghi log gì.

**Cách tìm ra.** Không đoán mò — thêm đúng **một dòng log** vào nhánh phân tích thất bại:

```
WARN thumb: không phân tích được đường dẫn "/2%2F84341"
```

Nhìn là ra ngay.

**Nguyên nhân.** URL thumbnail dựng bằng `convertFileSrc(`${epoch}/${index}`, "thumb")`.
Hàm này **percent-encode** tham số được truyền vào, nên dấu `/` biến thành `%2F` trước khi tới
backend. Hàm `parse_path` tách theo `/` nên không tách được gì.

Không thể tránh bằng cách tự dựng URL: trên Windows, Tauri phục vụ scheme tuỳ biến qua
`http://thumb.localhost`, còn trên nền tảng khác là `thumb://localhost`. `convertFileSrc` tồn tại
chính để che khác biệt đó — viết tay URL sẽ chạy trong trình duyệt rồi 404 ở đây.

**Cách sửa.** Nối hai số bằng `_` thay vì `/`. Dấu gạch dưới là ký tự **unreserved** nên
`encodeURIComponent` không đụng tới.

**Chống tái phát.** Test `a_slash_separator_is_rejected_because_it_never_arrives_intact` dùng đúng
chuỗi quan sát được (`/2%2F84341`) và khẳng định nó **không** phân tích được — nếu ai đổi lại về
dấu `/`, test sẽ đỏ ngay.

**Bài học.** Một `onerror` xử lý "đẹp" có thể **giấu mất lỗi thật**. Ở đây việc ẩn ảnh hỏng là
đúng về mặt sản phẩm, nhưng nó khiến 100% thất bại trông y hệt 0% thất bại. Nhánh lỗi ở phía
server cần ghi log ngay cả khi phía client đã xử lý "êm".

---

## BUG-010 🔴 — Thumbnail video trả về ảnh full-res, 1,27 MB mỗi cái

**Giai đoạn:** P5 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Test tích hợp dựng thumbnail thật từ thư viện người dùng chạy **pass**. Nhưng số
liệu in ra rất bất thường:

```
video  1269526 byte   980ms
image    82888 byte    50ms
audio    13804 byte   158ms
```

Yêu cầu 192×192, nhận về:

| | Kích thước thật |
|---|---|
| video | **1280 × 720** |
| image | 242 × 242 |
| audio | 192 × 192 |

**Nguyên nhân.** Cờ `SIIGBF_BIGGERSIZEOK` — nó *mời* provider trả về kích thước tự nhiên của nó,
và provider video hiểu đúng nghĩa đen: trả nguyên khung hình 720p. Tệ hơn, `SIIGBF_RESIZETOFIT`
thực chất bằng **0**, nên `RESIZETOFIT | BIGGERSIZEOK` chỉ còn `BIGGERSIZEOK`.

**Mức độ nghiêm trọng.** Cache đặt 512 mục với giả định "10–25 KB mỗi PNG". Với 1,27 MB mỗi cái,
cache đó là **650 MB RAM** — nhiều hơn cả index gấp 80 lần. Cộng thêm 980ms mỗi ảnh và ~1,3 MB
truyền qua protocol cho **mỗi hàng** trong danh sách.

**Cách sửa.** Bỏ `SIIGBF_BIGGERSIZEOK`. Thêm lưới an toàn phía Rust: nếu provider vẫn trả về to
hơn kích thước yêu cầu thì thu nhỏ lại bằng `image::imageops::resize`. Provider thumbnail là code
của bên thứ ba — một gói codec có thể cài provider riêng — nên không thể chỉ tin vào cờ.

**Kết quả đo được.**

| | Trước | Sau | |
|---|---|---|---|
| video | 1280×720 · 1.269.526 byte · 980ms | 192×108 · **37.475 byte** · **51ms** | nhỏ hơn 34× |
| image | 242×242 · 82.888 byte · 50ms | 192×192 · **51.976 byte** · **9ms** | |

Cache 512 mục giờ khoảng 18 MB thay vì 650 MB.

**Bài học.** Test **pass** mà vẫn có lỗi nặng. Test hỏi "có dựng được ảnh không?" và câu trả lời
là có. Chỉ vì test **in ra số byte và thời gian** nên con số 1.269.526 mới đập vào mắt. Nếu chỉ
`assert!(png.len() > 200)` thì lỗi này đã lọt tới tận khi người dùng thấy app ăn 650 MB RAM.

---

## BUG-011 🟠 — Video hiện icon chung của Windows thay vì khung hình thật

**Giai đoạn:** P5 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Sau khi sửa `BUG-009`, lưới ảnh đã hiện thumbnail. Nhưng nhìn kỹ ảnh chụp:
**ảnh** (`.avif`, `.webp`, `.jpg`) có preview thật, còn **mọi video** (`.mp4`, `.webm`) đều hiện
cùng một biểu tượng nút play xám — icon loại tệp của Windows.

Nghịch lý: test tích hợp trước đó **đã trích được khung hình thật** từ một tệp `.mp4`. Nên trích
xuất là làm được.

**Nguyên nhân.** `IShellItemImageFactory::GetImage` không có cờ `SIIGBF_THUMBNAILONLY` sẽ **lùi về
icon loại tệp** khi không có thumbnail sẵn. Tệ hơn: Windows **cache cả icon đó**, nên đường
`SIIGBF_INCACHEONLY` trả về icon rồi dừng — và bước trích khung hình thật không bao giờ được chạy.

Mỗi video trong lưới vì thế đều hiện đúng một biểu tượng giống nhau.

**Vì sao đáng sửa.** Một công cụ tìm media mà hiện icon chung thì **không nói thêm điều gì** ngoài
thứ tên tệp đã nói. Chính khung hình mới là lý do người ta cần lưới ảnh thay vì danh sách.

**Cách sửa.** Thêm `SIIGBF_THUMBNAILONLY` vào **cả hai** lời gọi. Nó từ chối icon loại tệp:

- Tệp có thumbnail thật → trả về khung hình / ảnh bìa.
- Tệp không có → trả về rỗng, giao diện hiện huy hiệu màu theo loại.

**Đánh đổi có chủ ý.** Tệp mp3 không nhúng ảnh bìa giờ **không có ảnh** thay vì hiện icon nốt nhạc.
Đó là lựa chọn đúng: huy hiệu màu `A` của chúng ta đã nói "đây là nhạc" rồi, một icon nốt nhạc xám
chỉ thêm nhiễu.

**Kết quả đo được sau khi sửa.**

```
video  37.475 byte   54ms   ← khung hình thật
image  51.976 byte   10ms   ← thumbnail thật
audio  KHÔNG DỰNG ĐƯỢC     ← tệp không có ảnh bìa, huy hiệu màu thay thế
```

**Bài học.** Ảnh chụp màn hình bắt được lỗi mà test không bắt được. Test hỏi *"có dựng được ảnh
không?"* và câu trả lời là có — chỉ là **ảnh sai**. Cả ba lỗi của P5 (`BUG-009`, `BUG-010`,
`BUG-011`) đều chỉ lộ ra khi **nhìn vào thứ hiện lên màn hình**, chứ không phải khi đọc kết quả
test.

---

## BUG-012 🟠 — `SCHEMA_VERSION` nằm trong chính khối dữ liệu nó bảo vệ

**Giai đoạn:** P6 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Thêm hai trường vào `Index` khiến cache cũ không đọc được — điều đó bình thường
và đã được lường trước. Nhưng thông báo lại là:

```
không nạp được cache (cache hỏng, không giải mã được)
```

Sai bản chất. Cache **không hỏng** — nó chỉ thuộc phiên bản cũ. Hai chuyện khác hẳn nhau: một cái
nghe như dữ liệu bị lỗi hoặc đĩa có vấn đề, cái kia chỉ cần bấm "Quét lại".

**Nguyên nhân.** Cơ chế `SCHEMA_VERSION` vô dụng theo đúng định nghĩa của nó:

```rust
let cache: CacheFile = bincode::deserialize_from(&mut reader)?;   // ← chết ở đây
if cache.schema_version != SCHEMA_VERSION { … }                   // ← không bao giờ tới
```

Số phiên bản là **một trường bên trong khối bincode**. Thời điểm cần đọc nó — khi layout đổi —
cũng chính là thời điểm không giải mã được khối đó. Phép kiểm tra chưa từng chạy một lần nào.

**Cách sửa.** Đưa magic và số phiên bản ra **ngoài** khối dữ liệu: 12 byte thuần ở đầu tệp, đọc
bằng `read_exact` trước khi động tới bincode. Đọc được bất kể phía sau đổi thế nào.

Áp dụng cùng cách cho `metadata.bin` của enrichment ngay từ đầu.

**Thông báo cũng sửa theo.** Cache không có header (định dạng cũ) và tệp lạ đều dẫn tới cùng một
việc người dùng cần làm, nên thông báo nói **việc cần làm** thay vì phân loại nguyên nhân:
*"cache thuộc định dạng cũ và không đọc được — bấm Quét lại để dựng lại"*.

**Bài học.** Một phép kiểm tra phiên bản chỉ có giá trị nếu nó đọc được **ở đúng lúc phiên bản
sai**. Đặt nó bên trong thứ nó bảo vệ là tự vô hiệu hoá nó. Lỗi này chỉ lộ ở lần đổi schema **đầu
tiên** — nếu không gặp bây giờ thì sẽ gặp ở P7 hoặc P8, lúc đó có thể là dữ liệu thật của người
dùng và thông báo sai sẽ khiến họ nghĩ đĩa hỏng.

---

## BUG-013 🔴 — Hàng đợi enrichment sắp xếp ngược, đọc nhạc trước video

**Giai đoạn:** P6 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Enrichment chạy nền, chỉ báo cho biết đã đọc **31.773/117.128 tệp**. Bấm bộ lọc
`≥1080p` → **không tìm thấy kết quả nào**.

Nhưng test tích hợp đã đọc được `1.mp4` là **1920×1080**. Dữ liệu tồn tại, bộ lọc lại không thấy.

**Nguyên nhân.** Hàng đợi được sắp xếp để ưu tiên video:

```rust
queue.sort_by_key(|&i| match kind {
    MediaKind::Video => 0,   // ý định: đọc trước
    MediaKind::Image => 1,
    MediaKind::Audio => 2,
});
```

Sau khi sắp xếp, vector là `[Video…, Image…, Audio…]`. Nhưng worker lấy việc bằng **`Vec::pop()`**,
mà `pop` lấy phần tử **cuối cùng**.

Kết quả: **nhạc được đọc trước tiên**, ngược hoàn toàn với ý định. Sau 31.773 tệp, phần lớn là
nhạc — mà nhạc thì `width` và `height` luôn bằng 0. Bộ lọc độ phân giải không có gì để khớp.

**Vì sao im lặng.** Không có lỗi ở đâu cả:

- Enrichment chạy đúng, đếm đúng, lưu đúng.
- Chỉ báo hiện con số tăng đều, trông hoàn toàn khoẻ mạnh.
- Bộ lọc trả về 0 kết quả — mà 0 kết quả là **câu trả lời hợp lệ** cho một bộ lọc.
- Cả hai đều đúng theo cách nhìn riêng của chúng.

Chỉ khi biết chắc `1.mp4` là 1080p thì con số 0 mới trở thành mâu thuẫn.

**Cách sửa.** Tách phần sắp xếp thành hàm `order_queue` riêng và đảo thứ tự — **ưu tiên cao nhất
phải nằm CUỐI**, vì đó là đầu mà `pop` lấy:

```rust
queue.sort_by_key(|&i| match kind_of(i) {
    MediaKind::Audio => 0,
    MediaKind::Image => 1,
    MediaKind::Video => 2,   // pop() lấy cái này trước
});
```

**Chống tái phát.** Test `video_is_read_before_anything_else` **rút việc ra đúng cách worker làm**
(`pop` liên tiếp) rồi kiểm tra thứ tự nhận được. Kiểm tra thứ tự trong vector thôi là chưa đủ —
chính chỗ đó là chỗ tôi nhầm.

**Bài học.** `sort` rồi `pop` là một cặp dễ nhầm: thứ tự đọc **ngược** với thứ tự trong vector.
Và cái giá của nhầm lẫn này không phải là lỗi mà là **im lặng** — hệ thống làm việc chăm chỉ hàng
chục phút cho đúng thứ ít hữu ích nhất, trong khi mọi chỉ báo đều xanh.
