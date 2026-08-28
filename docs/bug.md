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

---

## BUG-014 🟠 — Màn hình trống mời gọi một phím tắt mà ứng dụng có thể không sở hữu

**Giai đoạn:** P8 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** `register_hotkey` đã được viết để "cảnh báo rồi chạy tiếp" khi phím tắt bị chiếm —
nhưng nhánh đó chưa bao giờ được chạy thử. Đem chạy thật (một tiến trình khác `RegisterHotKey`
`Ctrl+Alt+Space` trước) thì ứng dụng khởi động bình thường, ghi log:

```
WARN không đăng ký được phím tắt Ctrl+Alt+Space: HotKey already registered
```

Vấn đề nằm ở chỗ **người dùng không bao giờ thấy dòng log đó** — họ mở app từ Explorer, không có
cửa sổ terminal nào cả. Trong khi đó màn hình trống vẫn in nguyên:

> `Ctrl` + `Alt` + `Space` để gọi cửa sổ này từ bất kỳ đâu

Một câu hướng dẫn **sai sự thật**. Người dùng bấm, không có gì xảy ra, và không có cách nào biết
tại sao. Với loại lỗi này thì không có hint còn đỡ hơn có.

**Nguyên nhân.** Kết quả đăng ký chỉ đi vào log, không đi vào trạng thái nào mà giao diện đọc được.
Cộng thêm: tổ hợp phím được viết **hai nơi** — hằng `HOTKEY` trong Rust và chuỗi `<kbd>` cứng
trong `App.svelte` — nên kể cả khi đăng ký thành công, đổi phím ở một nơi sẽ làm nơi kia nói dối.

**Cách sửa.** Một cờ toàn tiến trình `HOTKEY_ACTIVE` (đúng bản chất: đăng ký phím tắt là tài nguyên
cấp tiến trình của hệ điều hành) cộng lệnh `hotkey_status` trả về **cả tổ hợp lẫn tình trạng**:

```rust
pub struct HotkeyStatus { pub combo: String, pub active: bool }
```

Giao diện tự tách `combo` theo dấu `+` thành các phím, nên chỉ còn **một** nơi trong mã nguồn quyết
định phím tắt là gì. Khi `active == false`, dòng hint đổi sang màu hổ phách và đổi nội dung thành
*"đang bị ứng dụng khác chiếm — đóng ứng dụng đó rồi mở lại MediaFinder để dùng được phím tắt"*.

Màu hổ phách chứ không phải đỏ: ứng dụng vẫn chạy đủ chức năng, chỉ mất mỗi phím tắt.

**Bài học.** Một nhánh xử lý lỗi chưa từng chạy thì chưa phải là đã xử lý xong. Ở đây nhánh đó
**hoạt động đúng** — nó chỉ báo cho sai người: cho người viết code, không cho người dùng.

---

## BUG-015 🔴 — Phím tắt gọi được cửa sổ nhưng không đặt được con trỏ vào ô tìm kiếm

**Giai đoạn:** P8 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Bấm `Ctrl+Alt+Space` → cửa sổ hiện lên, đúng như thiết kế. Gõ luôn `anglerfish` →
**không có chữ nào vào ô tìm kiếm**. Ảnh chụp cho thấy ô nhập không có viền focus và vẫn còn
placeholder.

Đây là lỗi nặng nhất có thể có với một launcher: toàn bộ lý do tồn tại của phím tắt là *bấm rồi
gõ ngay*. Gọi được cửa sổ mà phải với tay ra chuột bấm vào ô nhập thì phím tắt gần như vô nghĩa.

**Nguyên nhân.** `summon()` gọi `unminimize` → `show` → `set_focus`. Cả ba đều thao tác trên **cửa
sổ**, không cái nào chạm tới **phần tử DOM** bên trong WebView. Phía frontend chỉ đặt focus đúng
hai chỗ: `autofocus` lúc tải trang, và khi bấm `Escape`. Không có gì chạy khi cửa sổ được gọi lại.

Con trỏ vì thế ở lại chỗ nó đang ở — một hàng kết quả, một nút bấm, hoặc không đâu cả nếu cửa sổ
vừa bị ẩn.

**Cách sửa.** `summon()` phát thêm một sự kiện `summon`; giao diện lắng nghe và gọi
`inputEl.focus()` + `inputEl.select()`.

Dùng sự kiện riêng chứ **không** dùng `window.addEventListener("focus", …)`: sự kiện focus của cửa
sổ cũng nổ mỗi lần người dùng alt-tab quay lại giữa chừng, và khi đó `select()` sẽ bôi đen truy vấn
họ đang gõ dở — phím tiếp theo xoá sạch. Sự kiện riêng chỉ nổ khi người dùng **chủ động gọi** cửa
sổ: phím tắt, hoặc lần chạy thứ hai bị `single_instance` chuyển hướng về.

`select()` chứ không chỉ `focus()`: đó là quy ước của mọi launcher — lần gọi sau bắt đầu một truy
vấn mới, không nối thêm vào truy vấn cũ. Đã kiểm chứng: gõ `anglerfish` → ẩn → gọi lại → gõ
`avatar` → ô nhập chỉ còn `avatar`, không phải `anglerfishavatar`.

**Vì sao lọt tới tận P8.** Điều hướng bàn phím được nghiệm thu ở P3/P5 bằng cách bấm phím **trong
lúc cửa sổ đang mở sẵn và đang có focus** — trạng thái mà `autofocus` lúc tải trang đã lo xong.
Phím tắt mới là thứ đầu tiên tạo ra tình huống "cửa sổ vừa từ trạng thái ẩn quay lại", và lỗi chỉ
tồn tại trong đúng tình huống đó.

---

## BUG-016 🟡 — Bơm phím để test có thể rơi vào cửa sổ của người dùng

**Giai đoạn:** P8 · **Trạng thái:** WORKAROUND · **Ngày:** 2026-08-24

**Hiện tượng.** Kịch bản test bơm `Down` rồi `Ctrl+Enter` để thử "mở thư mục chứa tệp". Không có
cửa sổ Explorer nào mở ra. Kiểm tra lại thì cửa sổ đang ở foreground **không phải MediaFinder mà
là VS Code của người dùng** — hai phím đó đã đi vào trình soạn thảo của họ.

**Nguyên nhân.** Giữa hai lần gọi công cụ của tôi, VS Code giành lại foreground. `keybd_event`
không gửi phím tới một cửa sổ cụ thể — nó bơm vào **hàng đợi đầu vào của hệ thống**, và hệ thống
giao cho cửa sổ nào đang ở trước tại đúng khoảnh khắc đó.

**Cách xử lý.** Hai quy tắc, áp dụng cho mọi kịch bản test bơm phím về sau:

1. **Gộp cả chuỗi thao tác vào một tiến trình PowerShell duy nhất.** Mỗi lần trả quyền về cho tôi
   là một lần foreground có thể đổi chủ.
2. **Kiểm tra foreground ngay trước khi gõ, và huỷ nếu sai:**

   ```powershell
   if ((Fg) -ne $appPid) { Write-Output "HUY: cua so khong o foreground"; exit 1 }
   ```

   Thà không test được còn hơn gõ nhầm vào máy người khác.

**Liên quan.** Cùng họ với [BUG-003](#bug-003) — ở đó là *chụp* nhầm màn hình người dùng, ở đây là
*gõ* nhầm vào cửa sổ người dùng. Cùng một gốc: thao tác ở cấp hệ thống thì không tự biết nó đang
nhắm vào ai.

**Ghi chú trung thực.** Lần chạy hỏng đó đã thực sự gửi `Down` và `Ctrl+Enter` vào VS Code trước
khi tôi phát hiện. Hai phím này không sửa nội dung tệp, nhưng có thể đã đổi vị trí con trỏ hoặc mở
thêm một khung soạn thảo.

---

## BUG-017 🔴 — Số 0 được coi là một định danh hợp lệ, một thay đổi xoá sạch cả index

**Giai đoạn:** P9 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Bench đo `rebuild_with` trên index tổng hợp 500.000 mục cho ra:

| Số thay đổi áp vào | Thời gian |
|---|---|
| 0 | **165 ms** |
| 100 | **21,9 ms** |

Áp một trăm thay đổi **nhanh hơn bảy lần** so với áp không thay đổi nào. Một kết quả không thể
đúng được.

**Nguyên nhân.** Khi thêm tham số `frn` vào `IndexBuilder`, mọi lời gọi trong bench được sửa hàng
loạt để truyền `0`. Nên cả 500.000 mục trong index tổng hợp đều mang `frn = 0`.

`rebuild_with` khoá theo `(volume, frn)`. Thay đổi đầu tiên trong bench là
`Gone { volume: b'D', frn: 0 }` — và nó khớp **toàn bộ 500.000 mục cùng lúc**. Index bị xoá sạch,
nên phần dựng lại gần như không còn gì để làm. Đó chính là 21,9 ms.

**Vì sao nguy hiểm hơn vẻ ngoài của nó.** Đây không phải lỗi của bench. Bench chỉ là thứ **duy
nhất** tình cờ chạm vào nó. Trong `rebuild_with`, `0` là một khoá tra cứu bình thường như mọi số
khác, và `Index::frn()` trả về `0` cho bất kỳ mục nào không có FRN. Một index dựng từ mã cũ, hoặc
một bản ghi journal dị dạng, sẽ khiến **một** thay đổi xoá **mọi** mục như vậy — im lặng, không lỗi,
không cảnh báo.

**Cách sửa.** Đặt tên cho hằng số đó và chặn nó ở cả ba nơi: lúc gộp thay đổi, lúc dựng bảng tra
vị trí cũ, và lúc đối chiếu từng mục.

```rust
/// Not a reference number NTFS ever hands out, so it is safe as "no identity".
const NO_FRN: u64 = 0;
```

NTFS không bao giờ cấp `0`: record 0 là chính `$MFT` và luôn có sequence number ở 16 bit cao, còn
gốc ổ đĩa là record 5. Nên `0` an toàn để dùng làm giá trị "không có định danh".

**Chống tái phát.** Hai test: `zero_is_not_an_identity_and_matches_nothing` (index có mục `frn = 0`
lẫn mục có FRN thật — thay đổi mang `frn = 0` không được đụng vào mục nào, còn FRN thật vẫn phải
hoạt động) và `a_change_carrying_no_reference_number_is_dropped`. Bench cũng được sửa để mỗi mục
tổng hợp mang một FRN riêng.

**Bài học.** Bench bắt được thứ 149 test không bắt được, và bắt được **không phải bằng một assertion
nào cả** — chỉ bằng một con số vô lý. Test kiểm tra thứ tôi nghĩ ra để kiểm tra; số đo thì phơi ra
thứ tôi không nghĩ tới. Cùng họ với [BUG-010](#bug-010) (thumbnail 1,27 MB — test pass, chỉ số byte
in ra là sai) và [BUG-013](#bug-013) (hàng đợi sắp xếp ngược — mọi chỉ báo đều xanh).

---

## BUG-018 🔴 — Ổ mạng bị bỏ qua hoàn toàn im lặng, 37,9 TB không có trong kết quả

**Giai đoạn:** P9 · **Trạng thái:** ĐÃ SỬA (phần thông báo) · **Ngày:** 2026-08-24

**Hiện tượng.** Người dùng hỏi vì sao phải quét lại. Kiểm tra ổ đĩa trên máy thì thấy:

| Ổ | Loại | Dung lượng | Tình trạng trước khi sửa |
|---|---|---|---|
| C:, D: | local NTFS | 4,2 TB | được index |
| G: | local FAT32 | 3,7 TB | bỏ qua — **có báo** |
| F: | mạng → `\192.168.1.214\f` | 9,3 TB | **im lặng** |
| Y: | mạng → `\192.168.1.213\padoma 8` | 14,3 TB | **im lặng** |
| Z: | mạng → `\192.168.1.213\padoma 1` | 14,3 TB | **im lặng** |

Gần **38 TB** không có mặt trong kết quả tìm kiếm, và không một dòng thông báo nào ở đâu cả.

**Nguyên nhân.** `list_volumes()` lọc `GetDriveTypeW` chỉ lấy `DRIVE_FIXED` và `DRIVE_REMOVABLE`.
Ổ mạng là `DRIVE_REMOTE`, nên chúng bị loại **trước** cả vòng lặp báo cáo "volume bị bỏ qua" — vòng
đó chỉ duyệt những ổ đã lọt vào danh sách. G: (FAT32) lọt vào nên được báo; F:, Y:, Z: thì không.

Vi phạm thẳng [bất biến số 9](../README.md#bất-biến-kiến-trúc): *"được phát hiện và báo rõ cho
người dùng, không im lặng bỏ qua"*. Bất biến đó được viết khi nghĩ tới USB FAT32 — không ai nghĩ
tới ổ mạng, nên nó không bao giờ được áp dụng ở đó.

**Một chi tiết khiến chuyện này dễ sai hơn nữa.** Cả ba ổ mạng đều khai filesystem là **`NTFS`**:

```
F: NTFS   Network [\192.168.1.214\f]
Y: NTFS   Network [\192.168.1.213\padoma 8]
Z: NTFS   Network [\192.168.1.213\padoma 1]
```

SMB báo lại filesystem của **máy chủ**. Nên kiểm tra `is_ntfs()` sẽ **chấp nhận** cả ba, rồi hỏng ở
bước `open_volume` với một thông báo nói về NTFS — trong khi vấn đề chẳng liên quan gì tới NTFS.

**Cách sửa.** Thêm `VolumeKind { Local, Removable, Network }` — hỏi *cách ổ đĩa được gắn*, không
hỏi tên filesystem. `is_scannable()` = NTFS **và** không phải ổ mạng. `skip_reason()` trả về lý do
bằng lời của người dùng, và với ổ mạng thì nêu luôn địa chỉ máy chủ (`WNetGetConnectionW`): khi có
ba ổ mạng thì "bỏ qua Z:" là vô dụng, còn "bỏ qua Z: (`\192.168.1.213\padoma 1`)" chỉ đúng máy
cần đi xem.

**Phần chưa sửa.** Đây mới chỉ là **nói ra sự thật**, chưa phải hỗ trợ ổ mạng. Kiến trúc MFT/USN
không thể đọc được NAS — xem [ISSUE-003](./issue.md#issue-003).

**Bài học.** Bất biến số 9 nói "không im lặng bỏ qua" và tôi tin là đã làm đúng, vì đã có sẵn vòng
lặp báo cáo. Nhưng vòng lặp đó chỉ báo cáo những thứ **đã lọt qua bộ lọc trước nó**. Thứ bị loại ở
bộ lọc sớm hơn thì không có gì báo cáo cả — và không có cách nào biết, vì thiếu vắng không tạo ra
dấu vết.

---

## BUG-019 🟠 — Cập nhật nhanh chạy thành công nhưng giao diện báo "kết thúc bất thường"

**Giai đoạn:** P9 → P10 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-25

**Hiện tượng.** Bấm "Quét lại". Chỉ mục **được cập nhật đúng**, cache ghi xong, mọi thứ trong log
đều bình thường. Nhưng giao diện hiện:

> Tiến trình quét kết thúc bất thường. Dữ liệu cũ vẫn nguyên.

Câu đó sai ở cả hai vế: tiến trình kết thúc bình thường, và dữ liệu **đã** được cập nhật.

**Nguyên nhân.** Giao diện chỉ theo dõi đúng một thứ: `progress.json`. Nó dừng khi thấy
`finished: true`, và có một nhánh dự phòng — nếu tiến trình con chết mà chưa báo gì thì hiện lỗi,
để thanh tiến độ không quay mãi.

Đường **cập nhật nhanh** thêm ở P9 chạy trước `run_indexer` và trả về sớm, nên nó **không ghi
`progress.json`** một lần nào. Với giao diện, một lượt chạy xong mà chưa bao giờ ghi `finished`
trông y hệt một lần crash — và nhánh dự phòng làm đúng việc của nó.

**Vì sao lọt.** Suốt P9 tôi chỉ chạy `--index` từ **dòng lệnh**, nơi không có giao diện nào theo
dõi. Mọi phép đo đều đúng: 0,45 giây, đúng số tệp, cache ghi xong. Chỉ có điều không phép đo nào
đi qua đúng con đường mà người dùng đi.

**Cách sửa.** Cập nhật nhanh nay ghi tiến độ như bản quét đầy đủ: báo từng ổ khi đọc journal, báo
`saving` khi ghi cache, và `finished` sau khi cache đã nằm an toàn trên đĩa. Đường ghi cache thất
bại **cố ý không** báo `finished` — nó trả về `false` để chuyển sang quét đầy đủ, mà quét đầy đủ sẽ
tự ghi tiến độ của nó.

## Lỗi thứ hai cùng họ, tìm ra khi đang sửa lỗi thứ nhất

Nút **"+ ổ mạng"** chạy hai pha: tiến trình con quét ổ cục bộ, rồi tiến trình GUI duyệt ổ mạng.
Tiến trình con ghi `finished: true` khi xong **pha một** — nên giao diện tưởng đã xong toàn bộ,
ngừng theo dõi, và 4,5 phút quét NAS chạy âm thầm không có gì trên màn hình nói rằng nó đang chạy.

Sửa bằng cờ `--no-finish`: pha một không được phép giương cờ kết thúc khi còn pha hai phía sau.
Nhánh dự phòng vẫn bắt được nếu tiến trình con chết thật, vì cờ `scanning` chỉ hạ khi cả hai pha
xong.

**Bài học.** Hai lỗi, cùng một gốc: **`progress.json` là hợp đồng, không phải nhật ký.** Ai ghi vào
đó cũng phải hoàn thành hợp đồng — báo xong đúng một lần, và đúng lúc thật sự xong. Cả hai đều
không thể tìm ra bằng cách chạy từ dòng lệnh, vì ở đó không có ai đang đọc hợp đồng ấy.

## BUG-020 🔴 — Kéo một tệp trên NAS làm **tắt phăng cả ứng dụng**

**Giai đoạn:** P13 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-25

**Hiện tượng.** Kéo một tệp bất kỳ nằm trên ổ mạng. Ứng dụng biến mất — không bảng lỗi, không
cửa sổ báo, không kịp thấy gì. Trong console:

```
thread 'main' panicked at drag-2.1.1\src\platform_impl\windows\mod.rs:370:60:
called `Option::unwrap()` on a `None` value
panic in a function that cannot unwind → aborting
```

**Nguyên nhân.** Crate `drag` gọi `dunce::canonicalize` lên mọi đường dẫn trước khi đưa cho shell.
Với ổ mạng đã ánh xạ, hàm đó trả về dạng UNC verbatim — và `ILCreateFromPathW` **từ chối** dạng ấy.
Đo bằng một test Rust:

```
gốc:        F:\132 mốc  168 commit, từ 2026-07-01.txt   → shell nhận: true
chuẩn hoá:  \?\UNC\192.168.1.214\f\132 mốc  ...        → shell nhận: false
```

`dunce` làm vậy **có chủ đích**: `is_safe_to_strip_unc` chỉ rút gọn `Prefix::VerbatimDisk`, vì
đường dẫn UNC hiểu `..` theo nghĩa đen nên rút gọn có thể đổi ý nghĩa. Đây là chỗ hai thư viện
đúng riêng lẻ nhưng sai khi ghép: dunce giữ nguyên UNC cho an toàn, shell không nhận UNC verbatim,
và crate `.unwrap()` cái `None` ở giữa.

Điều biến nó từ lỗi thành thảm hoạ là **chỗ** panic xảy ra: bên trong window procedure, nơi Windows
gọi ngược vào mã Rust qua ranh giới FFI. Panic ở đó **không unwind được**, nên runtime `abort()`
toàn tiến trình. Không có `catch_unwind` nào cứu được, và không có cách nào bắt lỗi từ bên gọi.

**Mức nghiêm trọng thật.** 87% thư viện của người dùng nằm trên NAS — 311.951 trên 360.655 tệp.
Nên đây không phải trường hợp biên: với người dùng này, **kéo tệp là thao tác làm sập ứng dụng**.

**Vì sao lọt qua P12.** Lượt test P12 tự tạo tệp thử trong `C:\Users\Padoma1\Videos` — ổ cục bộ.
Sáu mục test đều pass, và mục quan trọng nhất ("thả thật vào ứng dụng khác") pass thật. Cả sáu chỉ
chưa bao giờ đi qua một đường dẫn mạng. Đây là lần thứ hai một lỗi lọt vì dữ liệu thử là dữ liệu
tôi tự tạo chứ không phải dữ liệu người dùng có — lần trước là [BUG-018](#bug-018).

**Cách sửa: bỏ crate, tự viết** — [`ipc/drag_source.rs`](../src-tauri/src/ipc/drag_source.rs).

Khi mổ ra thì phần thật sự phải tự viết **nhỏ hơn nhiều** so với ước lượng ở [CONF-006](./config.md#conf-006):
`SHCreateShellItemArrayFromIDLists` + `BindToHandler(BHID_DataObject)` cho ra một `IDataObject`
đầy đủ do **chính shell dựng** — `CF_HDROP`, `Shell IDList Array`, `FileNameW`, `FileContents`.
Không phải tự cấp phát `HGLOBAL`, không phải tự xếp `DROPFILES`. Chỉ còn `IDropSource` với ba
phương thức ngắn.

Và bản tự viết **không chuẩn hoá đường dẫn** — nó đưa shell đúng dạng `F:\…` mà chỉ mục đang giữ,
tức đúng dạng shell biết phân giải.

**Bài học.** Một `.unwrap()` trong thư viện của người khác vẫn là rủi ro của mình, và mức rủi ro
phụ thuộc vào **nơi nó chạy**: cùng một panic, ở luồng thường thì mất một thao tác, trong window
procedure thì mất cả ứng dụng. Khi đánh giá "dùng crate hay tự viết" ở CONF-006 tôi đã cân kích
thước binary và số dòng `unsafe`, mà không hỏi câu quan trọng hơn: *code này chạy ở đâu, và nếu nó
panic ở đó thì mất gì.*

## BUG-021 🟠 — Mở xem trước bằng nháy đúp làm cửa sổ tự bung toàn màn hình

**Giai đoạn:** P14 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-25

**Hiện tượng.** Nháy đúp một hàng để xem trước. Lớp phủ hiện ra — và cả cửa sổ ứng dụng bung ra
toàn màn hình, che hết mọi thứ khác.

**Đo được, không phải cảm nhận.** `PrintWindow` trả về kích thước cửa sổ thật:

| Cách mở | Kích thước cửa sổ sau khi mở |
|---|---|
| Bàn phím (Shift+Enter) | **880×620** — bình thường |
| Nháy đúp | **1920×1080** — toàn màn hình |

**Nguyên nhân.** Cử chỉ mở lớp phủ cũng rơi vào chính thứ mà lớp phủ vừa đặt xuống dưới con trỏ.
Nháy đúp vào một hàng thì thẻ `<video>` hiện ra ngay tại chỗ con trỏ vừa nháy đúp, và Chromium đọc
phần đuôi của cử chỉ đó như cử chỉ của riêng nó — nháy đúp vào video nghĩa là toàn màn hình.

**Lần sửa đầu chưa đủ.** Chặn `dblclick` ngay trên thẻ `<video>` thì có lần hết, có lần không —
nghĩa là sự kiện tới được trình phát **không phải lúc nào cũng là sự kiện đang bị chặn**. Đoán xem
đó là sự kiện gì rồi chặn đúng cái đó là cách sửa dựa trên phỏng đoán.

**Hai lần sửa, và một lượt kiểm chứng hỏng nằm giữa.**

| # | Cách sửa | Kết quả đo |
|---|---|---|
| 1 | Chặn `dblclick` ngay trên thẻ `<video>` | ❌ vẫn bung **2/5 lần** |
| 2 | `pointer-events: none` trên khung + chặn `dblclick` ở **tầng cửa sổ, pha capture** | ✅ **5/5 sạch** |

**Vì sao lần 1 trượt còn lần 2 được.** Trình điều khiển media của Chromium xử lý cú nháy đúp
**trước khi** sự kiện kịp nổi lên tới trình xử lý gắn trên chính thẻ `<video>` — nên trình xử lý ở
tầng phần tử luôn tới muộn. Một trình lắng nghe ở **pha capture trên `window`** thì thấy sự kiện
trước tất cả. Đó là toàn bộ khác biệt.

**Tôi đã tuyên bố sửa xong ở lần 1 — và sai.** Ba lần chạy đầu đều sạch nên tôi kết luận là hết.
Chỉ khi người dùng báo một lỗi khác (video tràn khỏi khung, [BUG-023](#bug-023)) và tôi tái hiện
bằng truy vấn nặng hơn — 5.000 kết quả, video 1080p — thì nó mới hiện lại. **Truy vấn nhẹ không đủ
tải để lộ ra lỗi này.**

**Rồi tôi suýt ghi một lời giải thích sai vào chính tệp này.** Sau lần sửa 2 tôi vẫn đo ra "2/5 vẫn
bung", nên kết luận rằng chặn theo nguyên nhân là bất khả và chuyển sang **chặn ở kết quả** — nghe
`fullscreenchange` rồi hoàn tác. Kết luận ấy dựa trên một phép đo hỏng: lệnh cài của tôi đã chạy
trên **bộ cài cũ**, nên tôi đo một bản không hề chứa bản vá mình đang muốn kiểm chứng. Chi tiết:
[CHECK-008](./check.md#check-008).

Khi cài đúng bản và đo lại thì lần sửa 2 sạch **5/5**. Kiểm chứng chéo bằng cách **tắt riêng lớp
chặn kết quả** rồi đo lại: vẫn **5/5 sạch** — chứng minh lớp chặn ở tầng cửa sổ tự nó đã đủ. Lớp
chặn kết quả đã được **gỡ bỏ**: giữ lại một lớp phòng thủ không bao giờ kích hoạt, kèm chú thích
nói rằng nguyên nhân chưa rõ, là để lại một lời giải thích sai trong mã.

**Bài học 1 — về cách test.** Lỗi đua chỉ lộ ra dưới tải. Ba lần chạy sạch trên truy vấn 2 kết quả
không chứng minh được gì về truy vấn 5.000 kết quả.

**Bài học 2 — về việc đo cái gì.** Một phép đo trên nhị phân sai còn tệ hơn không đo: nó không chỉ
bỏ sót lỗi, nó **dựng lên một lý thuyết sai về nguyên nhân** rồi dẫn tới một cách sửa nhân danh lý
thuyết ấy. Từ nay mọi lượt kiểm chứng phải đối chiếu mã băm giữa bản vừa dựng và bản đang chạy.

## BUG-022 🟡 — Đường dẫn thư mục gốc hiện ngược thành `:D`

**Giai đoạn:** P14 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-25

**Hiện tượng.** Xem trước một tệp nằm ngay ở gốc ổ D:. Dòng đường dẫn hiện **`:D`** thay vì `D:\`.

**Nguyên nhân.** Của chính tôi. Để cắt bớt đầu một đường dẫn dài mà vẫn thấy được thư mục ở cuối,
tôi đặt `direction: rtl` cho dòng đó. Thuật toán bidi khi đó chuyển dấu câu ở cuối lên đầu, nên
`D:\` thành `:D`. Với đường dẫn dài nhiều cấp thì không lộ ra, chỉ đúng trường hợp tệp nằm ở gốc ổ
mới thấy — và tôi chỉ thấy vì tình cờ tạo một tệp thử ở `D:\`.

**Cách sửa.** Bỏ `direction: rtl`. Danh sách kết quả vốn đã hiện đường dẫn xuôi từ trái sang phải;
làm giống nó vừa đúng vừa nhất quán.

**Bài học.** Một mẹo CSS thuần trình bày vẫn có thể **đổi nội dung người dùng đọc được**. `rtl` ở
đây không chỉ căn lề khác đi — nó sắp xếp lại ký tự.


## BUG-023 🟠 — Video tràn ra ngoài khung xem trước, đè lên dòng chân

**Giai đoạn:** P14 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-25 · **Người báo:** người dùng

**Hiện tượng.** Xem trước một video 1920×1080. Khung hình tràn xuống quá đáy khung chứa, đè lên
dòng chân (dung lượng, độ phân giải, các phím tắt). Người dùng gửi ảnh chụp.

**Nguyên nhân.** Của tôi, trong CSS. Khung chứa dùng `display: grid` với hàng ngầm định `auto` —
tức **kích thước hàng do nội dung quyết định**. Khi đó `max-height: 100%` của thẻ video không có gì
xác định để quy chiếu, nên trình duyệt **bỏ qua hẳn** thuộc tính ấy và vẽ video ở kích thước gốc.
Một clip 1080p vẽ cao 1080 pixel trong một khung cao vài trăm pixel, phần thừa rơi xuống dòng chân.

**Cách sửa.** Cho hàng một kích thước thật: `grid-template-rows: minmax(0, 1fr)`. `1fr` lấy kích
thước từ khung chứa mà flex đã tính xong, nên phần trăm có mốc để quy chiếu; số `0` ở cận dưới là
thứ cho phép nó co lại — nếu không, hàng lưới từ chối nhỏ hơn nội dung, đúng cái bẫy vừa gặp ở một
tầng trên. Thêm `overflow: hidden` làm chốt chặn cuối.

Đổi luôn `max-*: 100%` thành `width/height: 100%` cộng **`object-fit: scale-down`**. `contain` cũng
vừa khung nhưng sẽ **phóng to** một tệp nhỏ cho đầy khung; `scale-down` không bao giờ vẽ lớn hơn
kích thước thật của tệp — đúng ý định ban đầu.

**Vì sao lọt qua lượt test P14.** Mọi ảnh chụp kiểm chứng của tôi đều là clip **720p** trong một
cửa sổ đủ cao để 720 pixel vẫn lọt. Người dùng mở 1080p. Cùng một lỗi, chỉ khác dữ liệu — và lần
này dữ liệu thật lớn hơn dữ liệu thử.

**Bài học.** Lặp lại đúng bài học của [BUG-020](#bug-020) ở một chỗ khác: **dữ liệu thử do tôi chọn
nhỏ hơn dữ liệu thật.** Với bố cục, "thử bằng thứ lớn nhất người dùng có" phải là một mục kiểm tra,
không phải may rủi.

## BUG-024 🔴 — Cài tay đè lên bản cũ xoá sạch chỉ mục, người dùng tưởng bản mới hỏng

**Giai đoạn:** BT (phát hiện ở P27) · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-28 · **Người báo:** người dùng

**Hiện tượng.** Người dùng gõ nguyên tên một tệp có thật trên NAS
(`a-lady-enjoying-swimming-with-the-huge-whale-shark-2025-12-17-21-17-42-utc`) và không tìm ra; ứng
dụng báo "Không có tệp nào khớp đủ 16 từ · đang hiện 10/16". Họ báo thêm một chi tiết quyết định:
**ai còn ở v1.0.4 thì không gặp, ai lên v1.0.5 thì gặp.**

**Chẩn đoán đầu tiên của tôi đã SAI.** Tôi kết luận đó chỉ là chỉ mục ổ mạng cũ (ổ mạng chỉ được
quét khi bấm tay "+ ổ mạng"), và nói với người dùng như vậy. Nhưng chẩn đoán đó không giải thích
được tương quan với phiên bản — nếu chỉ là chỉ mục cũ thì v1.0.4 phải bị y hệt. Chính chi tiết
người dùng nêu thêm mới lật lại được vụ này.

**Nguyên nhân thật.** Của tôi, trong `src-tauri/nsis-hooks.nsh`.

Hai móc `NSIS_HOOK_PREUNINSTALL` và `NSIS_HOOK_POSTUNINSTALL` chạy **mỗi lần uninstaller chạy**.
Trong template của Tauri, chúng được chèn vào `Section Uninstall` **vô điều kiện** — nằm ngoài chốt
`$DeleteAppDataCheckboxState = 1 ${AndIf} $UpdateMode <> 1` mà chính Tauri dùng để bảo vệ dữ liệu
ứng dụng của nó.

Hệ quả tuỳ đường lên bản mới:

| Đường | Cờ | Kết quả |
|---|---|---|
| Nút **Cập nhật** trong ứng dụng | truyền `/UPDATE` → `PageLeaveReinstall` nhảy thẳng `reinst_done`, không chạy uninstaller | chỉ mục an toàn |
| **Tải .exe về cài đè tay** | không có `/UPDATE` → hiện trang chọn, **nút radio đầu tiên được tích sẵn** là "Uninstall before installing" | uninstaller chạy → móc xoá `index.bin`, `metadata.bin`, **và** gỡ luôn tác vụ định kỳ |

Ghi chú phát hành lại đang hướng người dùng đi đúng vào đường thứ hai, kèm một câu **sai sự thật**:
"Cài đè lên bản cũ được, không cần gỡ trước. Chỉ mục đã quét vẫn giữ nguyên."

**Bằng chứng trên máy thật.** Nhật ký sáng 28/8 (sau khi cài tay v1.0.4 lúc 27/8 15:41):

```
01:40:19Z  nạp cache: 48.319 tệp, 3.211 thư mục      ← chỉ còn ổ cục bộ
03:51:29Z  hợp nhất: 48.335 cục bộ + 320.505 mạng    ← sau khi quét lại NAS
```

**320.505 mục ổ mạng đã biến mất** và chỉ trở lại sau một lượt quét mạng thủ công. Thư mục chứa tệp
người dùng tìm: 125 tệp trên đĩa, 51 trong chỉ mục, 74 tệp có mtime sau mốc chỉ mục — `51 + 74 =
125`, khớp tuyệt đối.

**Cách sửa.** Cả hai móc nay tính một cờ chung trước khi làm bất cứ việc phá huỷ nào, dựa trên ba
tín hiệu phân biệt "gỡ hẳn" với "gỡ để cài đè":

* `$EXEDIR` so với `$INSTDIR` — tín hiệu quyết định. NSIS chỉ chạy uninstaller **tại chỗ** khi được
  gọi kèm `_?=`, và chỉ bộ cài mới gọi kiểu đó; người dùng tự gỡ thì NSIS chép sang thư mục tạm rồi
  chạy bản sao.
* `$UpdateMode` — bản cập nhật trong ứng dụng, tuyệt đối không đụng dữ liệu.
* `$DeleteAppDataCheckboxState` — người dùng tự tay yêu cầu xoá thì tôn trọng.

Cài đè nay cũng giữ nguyên tác vụ định kỳ: nó trỏ vào đúng đường dẫn mà bản mới ghi đè lên, nên vẫn
đúng. Gỡ nó đi nghĩa là sau khi nâng cấp, chỉ mục thôi tự làm mới cho tới khi người dùng tự quét lại
một lần có hỏi quyền — một sự cố im lặng không ai báo cho họ.

Ghi chú phát hành đã sửa: nói thẳng rằng các bản **từ v1.0.5 trở về trước** vẫn xoá chỉ mục khi cài
tay đè lên, và chỉ đường phục hồi (Quét lại → + ổ mạng).

**Chốt chặn.** `src-tauri/tests/installer_hooks.rs` — 4 bài đọc thẳng tệp `.nsh`: không móc nào được
xoá gì trước một `${If}`, chốt chặn phải còn nhìn đủ ba tín hiệu, chế độ cập nhật không bao giờ được
xoá, và gỡ thật thì phải dọn hết tệp ứng dụng tự tạo. Khôi phục bản móc cũ làm cả 4 bài đỏ.

**Bài học.** Một tương quan mà người dùng nêu ra ("v1.0.4 không sao, v1.0.5 thì có") là dữ liệu, dù
nó mâu thuẫn với chẩn đoán đang có. Tôi đã suýt đóng vụ này ở chẩn đoán sai vì nó giải thích được
triệu chứng — nhưng nó không giải thích được **tương quan**. Chi tiết không khớp mới là chỗ đáng đào.


### Nghiệm thu bằng bộ cài THẬT (P29, 28/08/2026) — và một giới hạn không vượt qua được

Bốn bài đọc tệp `.nsh` không chứng minh được điều quan trọng nhất, nên đã dựng bộ cài thật
(`npm run tauri build`, phiên bản nâng lên 1.0.6) và chạy nó đè lên bản v1.0.5 đang cài trên máy.

**Việc đầu tiên bộ cài thật chứng minh:** `makensis` **biên dịch được** `nsis-hooks.nsh`. Bài đọc
tệp không bao giờ nói được điều đó — một lỗi cú pháp trong `.nsh` sẽ chỉ lộ ra lúc CI đóng gói.

**Cái bẫy tái hiện đúng nguyên văn.** Đọc thẳng từ điều khiển Win32 của hộp thoại, không đoán qua
ảnh chụp:

```
Already Installed
"An older version of MediaFinder is installed on your system. It's recommended
 that you uninstall the current version before installing."
  (•) Uninstall before installing     <- DA TICH SAN
  ( ) Do not uninstall
```

Chọn đúng cái mặc định đó thì `index.bin` (48.074.384 byte) và `metadata.bin` (14.811.580 byte)
**bị xoá**, tác vụ định kỳ và lối tắt Startup **bị gỡ**.

**Nhưng thủ phạm không phải móc mới.** `netscan.json` và `logs/` sống sót — mà móc mới xoá cả hai.
Đối chiếu với `git show v1.0.5:src-tauri/nsis-hooks.nsh`:

| Móc cũ (v1.0.5) xoá | Quan sát được |
|---|---|
| `index.bin` | ✗ mất |
| `metadata.bin` | ✗ mất |
| `progress.json` | ✗ mất |
| `--remove-setup` vô điều kiện | ✗ mất tác vụ + lối tắt |
| *(không biết `netscan.json`, `logs/`)* | ✓ còn |

Khớp từng chi tiết. Móc cũ không biết `netscan.json` và `logs/` vì hai thứ đó chưa tồn tại ở v1.0.5.

**GIỚI HẠN CẤU TRÚC, KHÔNG VƯỢT QUA ĐƯỢC.** Mẫu NSIS gọi bộ gỡ **đang nằm trên máy**:

```nsis
ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
StrCpy $R1 "$R1 _?=$4"      ; chạy TẠI CHỖ
ExecWait '$R1' $0
```

Bộ gỡ đó là của bản **cũ**, mang móc **cũ**. Bản sửa nằm trong gói mới và chỉ được ghi ra **sau** khi
cài xong. Nên:

> **Bản sửa BUG-024 không cứu được bất kỳ ai đang ở v1.0.5 trở về trước.** Nó chỉ có hiệu lực từ
> v1.0.6 → v1.0.7 trở đi. Mọi người dùng cài tay bản mới đè lên bản ≤ v1.0.5 **vẫn sẽ mất chỉ mục**.

Đường an toàn duy nhất cho lần nâng cấp này là **nút cập nhật trong ứng dụng** — nó truyền `/UPDATE`,
và `PageLeaveReinstall` nhảy thẳng `reinst_done` mà không chạy bộ gỡ.

Ghi chú phát hành đã viết lại theo đúng sự thật đó: câu đầu tiên nay là *"Hãy cập nhật bằng nút trong
ứng dụng, đừng tải tệp `.exe` về cài đè lần này."* Bản nháp trước đó nói *"cài tay đè lên bản cũ không
còn xoá chỉ mục nữa"* — đúng về lâu dài nhưng **sai cho chính lần nâng cấp này**, tức lặp lại đúng
kiểu sai đã tạo ra BUG-024.

### Bản sửa có hoạt động không — đo riêng, và có

Sau khi v1.0.6 đã cài, chạy bộ gỡ **mới** đúng cách bộ cài gọi nó (`uninstall.exe /S _?=<thư mục>`):

| Tệp | Móc cũ | Móc mới |
|---|---|---|
| `index.bin` 48.074.384 | xoá | **còn nguyên** |
| `metadata.bin` 14.811.580 | xoá | **còn nguyên** |
| `progress.json` · `netscan.json` · `logs/` | xoá / không biết | **còn nguyên** |
| `mediafinder.exe` | xoá | xoá — đúng, bản mới sắp ghi đè |

Và phép thử ngược lại, gỡ **thật** (không có `_?=`, NSIS tự chép sang thư mục tạm nên
`$EXEDIR != $INSTDIR`): thư mục dữ liệu **xoá sạch hoàn toàn**, khoá registry mất. Bản sửa không bảo
vệ quá đà.

**Một điểm còn treo.** Ở lượt gỡ thật đó, lối tắt Startup **còn sót**. Nguyên nhân là thứ tự thử
nghiệm: lượt gỡ tại-chỗ ngay trước đó đã xoá `mediafinder.exe`, nên lời gọi
`"$INSTDIR\mediafinder.exe" --remove-setup` trong móc không chạy được. Không phải hình dạng thường
gặp — gỡ thật thì tệp exe vẫn còn — nhưng nó cho thấy **móc phụ thuộc vào sự tồn tại của exe và thất
bại im lặng khi thiếu**. Đáng thêm một chốt chặn.

## BUG-025 🔴 — Chỉ mục ổ mạng không bao giờ tự làm mới; ổ cục bộ chỉ mỗi ngày một lần

Người dùng báo tiếp: cái băng "khớp nhiều nhất — 10/16 từ" **không chỉ xảy ra với tệp trên NAS mà
còn với tệp trên ổ trong máy**. BUG-024 giải thích được vế NAS sau khi cài tay đè lên, nhưng không
giải thích được vế ổ cục bộ, nên phải đào tiếp. Lượt này lái thẳng bản v1.0.5 đã cài
(`C:\Users\Padoma1\AppData\Local\MediaFinder\mediafinder.exe`) bằng chuột và bàn phím thật.

**Tái hiện được cả hai vế.**

| # | Việc làm trên app thật | Kết quả |
|---|---|---|
| 1 | Dán đúng tên tệp người dùng báo (`a-lady-enjoying-…-utc`) | băng vàng "Không có tệp nào khớp đủ **16** từ… **10/16**", 22 kết quả sai · 13,6 ms |
| 2 | Tạo `D:\mf-test-p28\…zxqw.mp4` lúc 16:19:47 rồi tìm ngay | **"Không tìm thấy kết quả nào"** — chân cửa sổ vẫn "quét lúc 16:15:01" |
| 3 | Chạy tay tác vụ `MediaFinder - cap nhat chi muc` (+90 tệp) rồi tìm lại | tìm ra cả `.mp4` lẫn `.png`, 2 kết quả · 3,2 ms |

Bước 2 và 3 là cặp đối chứng: cùng một truy vấn, cùng một tệp, chỉ khác nhau ở chỗ chỉ mục đã được
làm mới hay chưa. **Bộ tìm kiếm không hỏng.** Tệp không có trong chỉ mục thì không thể ra.

**Bộ tìm kiếm được thử riêng, và nó khoẻ.** Năm truy vấn nữa trên app thật, mỗi truy vấn là nguyên
tên một tệp *đã có* trong chỉ mục:

| # | Kiểu tên | Ổ | Kết quả |
|---|---|---|---|
| T1 | 14 từ, gạch nối | C: | 1 kết quả · 5,4 ms |
| T2 | 29 từ, tiếng Pháp có dấu | Z: | 1 kết quả · 5,7 ms |
| T3 | có khoảng trắng, gạch dưới, chữ hoa | F: | 1 kết quả · 5,6 ms |
| T4 | T1 viết HOA/thường lẫn lộn | C: | 1 kết quả · 4,3 ms |
| T5 | 12 từ | Y: (NAS) | 1 kết quả · 3,7 ms |

Không lần nào hiện băng "khớp nhiều nhất". Truy vấn 20+ từ, có dấu, lẫn hoa thường đều ra đúng một
tệp trong dưới 6 ms.

**Nguyên nhân, đo bằng số.** Tệp người dùng tìm **có thật trên đĩa**:

```
Y:\PROJECT DEEP SEA 5\DS1_118\Whale Shark\a-lady-enjoying-…-2025-12-17-21-17-42-utc.mov
  đến ổ (CreationTime) : 28/08/2026 13:48:49
lần quét ổ mạng gần nhất (netscan.json atUnix 1787890985) : 28/08/2026 11:23:05
```

Tệp đến sau lần quét **2 giờ 25 phút**. Đối chiếu nguyên thư mục đó:

```
trên đĩa           : 125 tệp
chỉ mục biết       :  51 tệp
thiếu              :  74 tệp
  đến sau 11:12:45 :  67
  còn lại 7        : mốc CreationTime giữ nguyên khi sao chép, nhưng LastWriteTime
                     là 11:09–14:53 cùng ngày — tức cũng đến trong ngày
tệp mới nhất mà chỉ mục biết trong thư mục này: đến ổ lúc 11:12:45
```

51 + 74 = 125. Ranh giới nằm đúng ở mốc quét, không sót chỗ nào.

**Vì sao không bao giờ tự khỏi.** Hai đường làm mới, cả hai đều không với tới trường hợp này:

* **Ổ mạng — không có đường tự động nào cả.** `scan_network_volumes()` (`src-tauri/src/lib.rs:440`)
  có đúng **một** nơi gọi: `src-tauri/src/ipc/commands.rs:427`, tức lệnh IPC sau nút **+ Ổ mạng**.
  Tác vụ định kỳ chạy `--index` → `run_incremental()`, mà đường này đọc MFT/USN nên
  `src-tauri/src/ntfs/volume.rs:67` gạt thẳng ổ mạng ra: *"đọc MFT/USN chỉ làm được với đĩa gắn trực
  tiếp, qua SMB thì máy này không thấy MFT của máy chủ"*. Nhật ký xác nhận điều đó ở mọi lượt chạy.
  ⇒ **Chỉ mục ổ mạng chỉ mới khi có người bấm nút.** Không ai bấm thì nó cũ mãi.
* **Ổ cục bộ — mỗi ngày một lần.** Bản v1.0.5 đã phát hành và `master` đặt lịch
  `<DaysInterval>1</DaysInterval>` **không kèm `<Repetition>`** (`git show v1.0.5:src-tauri/src/setup.rs`).
  Khối `PT15M` đã có trên nhánh `edit` (commit `207b453`) nhưng **chưa gộp lên `master`, chưa
  phát hành**. ⇒ Tệp mới bỏ vào ổ trong máy có thể **mất
  tăm tới 24 giờ**.

Máy đang thử đã được nâng lên PT15M từ bản dev, nên khoảng mù ở đây chỉ 15 phút — máy người dùng thì
là một ngày. Đó là lý do họ thấy vế ổ cục bộ nặng hơn nhiều so với những gì đo được ở đây.

**Một chỗ mù nữa, phát hiện lúc đọc nhật ký.** `run_incremental()` đếm riêng `stats.unresolved` —
số thay đổi mà journal có nhắc tới nhưng không tra ra được thư mục cha (`src-tauri/src/lib.rs:757`).
Nhật ký cho thấy nó bắn thật, nhiều lần: 2, 3, 26, **73** thay đổi. Khi cả lượt không đổi gì thì
cache không được ghi và con trỏ journal đứng yên, nên lần sau thử lại — tự khỏi. Nhưng nếu cùng lượt
đó có tệp khác được thêm, cache **được** ghi, con trỏ **tiến qua**, và những thay đổi tra không ra
kia mất luôn cho tới một lượt quét đầy đủ. Chưa dựng được ca tái hiện trên máy thật; ghi lại ở đây
làm đầu mối, chưa phải kết luận.

**Nhật ký hiện trường đang mù.** `src-tauri/src/diag.rs` (ghi log ra tệp) có trên nhánh `edit`
nhưng không có trong `master` lẫn tag `v1.0.5`, nên bản v1.0.5
người dùng đang chạy **không ghi một dòng nào**. Tệp
`%LOCALAPPDATA%\MediaFinder\logs\mediafinder.log` trên máy này chỉ có nội dung do các bản dev tạo ra,
và nó đứng im ở 15:28 trong khi tác vụ định kỳ vẫn chạy lúc 16:00, 16:15, 16:21. Muốn chẩn đoán được
máy người dùng thì phải phát hành `diag.rs`.

**Đã sửa một phần ở P29** (xem `docs/test-log.md`, lượt P29). Bản kế tiếp nói thật về tuổi chỉ mục
ở cả ba chỗ — băng "khớp nhiều nhất", trạng thái "Không tìm thấy kết quả nào", và chân cửa sổ (nơi
trước đây in **một** mốc rồi gọi là "quét lúc", trong khi mốc ấy chỉ nói về ổ cục bộ). Máy đã mất
tác vụ định kỳ vì BUG-024 nay được báo thẳng và chỉ đúng nút cần bấm. Vế **ổ mạng vẫn chưa có đường
làm mới tự động** — đó là phần còn lại của lỗi này.

Ba chốt chặn phải đi kèm, tìm ra lúc phản biện chứ không phải lúc viết mã:

* `ensure_scheduled_task()` thoát sớm ở `scheduled_task_exists()`, nên đường nâng lịch **không được**
  đi qua nó — nếu không, `PT15M` không bao giờ tới máy người dùng và log ghi "nâng lên lịch v2" mãi
  mãi. Cùng hình dạng với lỗi `SCHEDULE_MARK` ở P22.
* Ba tệp tạm dùng chung một đường dẫn cho mọi tiến trình. Nguy nhất là `index.bin.tmp`: hai lượt ghi
  chồng nhau đưa một tệp lai vào chỗ chỉ mục, header 12 byte vẫn hợp lệ nên qua được chốt, `load()`
  trả `Corrupt`, và lượt quét đầy đủ theo sau **xoá sạch mục ổ mạng**. Xác suất nhỏ, hậu quả bằng
  đúng BUG-024.
* `NetScanMark` thiếu `#[serde(default)]`: thêm một trường mới sẽ làm mọi `netscan.json` trên 20–40
  máy đọc ra `None`, tức mốc "quét ổ mạng lần cuối" biến mất đúng vào bản phát hành thêm trường ấy.

**Hướng sửa còn lại** (chưa làm, chờ chốt):

1. Cho ổ mạng một đường làm mới định kỳ — quét lại nền theo lịch riêng, thưa hơn ổ cục bộ vì một
   lượt mất 140–175 s, và phải bỏ qua khi ổ không gắn.
2. Phát hành khối `PT15M` đang nằm trên nhánh `edit`, để ổ cục bộ hết cảnh chờ 24 giờ.
3. Khi băng "khớp nhiều nhất" bật lên, nói luôn chỉ mục cũ tới mức nào và ổ mạng lần cuối quét khi
   nào — hiện người dùng không có cách nào biết mình đang nhìn dữ liệu cũ.
4. Phát hành `diag.rs` (cũng đang nằm trên `edit`).

**Bài học.** Người dùng nói "còn bị ngay ở trên ổ" là một dữ kiện thu hẹp phạm vi, không phải một lời
than. Nó loại BUG-024 khỏi vai trò nguyên nhân duy nhất và chỉ thẳng vào chỗ chung của cả hai vế:
chỉ mục có tuổi, mà app không nói tuổi đó cho ai biết.
