# RỦI RO — MediaFinder
> **Thuộc file này:** Chưa gây hại, nhưng sẽ gây hại nếu không xử lý. Mỗi mục phải ghi rõ điều kiện kích hoạt và hạn xử lý.
> **KHÔNG thuộc file này:** lỗi đã xảy ra rồi (thuộc bug.md).
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md) · [test-log-v2](./test-log-v2.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `RISK-003`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [RISK-001](#risk-001) | ⚪ | `panic = "abort"` khiến panic trong IPC giết cả app | P0 | **ĐÃ SỬA** (P3) |
| [RISK-002](#risk-002) | ⚪ | Dự án chưa có version control | P0 | **ĐÃ SỬA** (P0) |

---

## RISK-001 ⚪ — `panic = "abort"` khiến panic trong IPC giết cả app

**Giai đoạn:** P0 (đã xử lý ở P3) · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Mô tả.** `src-tauri/Cargo.toml` đặt `panic = "abort"` trong `[profile.release]` để binary nhỏ và
nhanh hơn. Hệ quả: không có stack unwinding, nên `catch_unwind` vô hiệu.

**Rủi ro.** Khi có Tauri command thật ở P3, một panic trong handler (ví dụ index truy cập ngoài phạm vi,
`unwrap()` trên `None`) sẽ **giết toàn bộ ứng dụng** thay vì trả lỗi về frontend. Với bản release
người dùng cuối, app sẽ biến mất không thông báo.

**Chưa gây hại vì.** P0 chưa có command nào; hiện chỉ chạy bản debug (`panic = unwind`).

**→ Đã quyết định ở P3 (2026-08-24): BỎ `panic = "abort"`.**

Cân nhắc thực tế: `abort` giúp binary nhỏ hơn một chút, nhưng đổi lại là **một panic ở bất kỳ
đâu — kể cả trong một Tauri command gặp phải cái tên tệp không ai lường trước — sẽ kéo sập cả
ứng dụng mà không báo gì.** Với một công cụ để mở suốt ngày thì đánh đổi đó sai.

Đã làm **cả hai** phương án chứ không chọn một:

1. Bỏ `panic = "abort"` khỏi `[profile.release]`.
2. Mọi command trả `Result<_, String>` và không `unwrap()` bất cứ thứ gì đến từ frontend hay
   hệ thống tệp. Một đường dẫn biến mất giữa lúc quét và lúc bấm sẽ thành **thông báo trên màn
   hình**, không phải panic.

Có test khoá lại điều 2: `opening_a_missing_file_is_an_error_not_a_panic` và
`revealing_a_missing_file_under_a_missing_folder_is_an_error`.

<details>
<summary>Phân tích lúc chưa quyết định (giữ lại để tham khảo)</summary>

**Việc cần làm ở P3.** Quyết định một trong hai:
1. Bỏ `panic = "abort"`, chấp nhận binary lớn hơn, đổi lấy khả năng phục hồi.
2. Giữ `abort`, nhưng **cấm tuyệt đối `unwrap()` / `expect()` / index trực tiếp trong mọi command
   handler** — bắt buộc trả `Result<_, String>`.

Khuyến nghị hiện tại: **phương án 2**, vì code index/search đằng nào cũng nên trả `Result`.
Cần kiểm chứng lại bằng test khi có command thật.

</details>

---

## RISK-002 ⚪ — Dự án chưa có version control

**Giai đoạn:** P0 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Mô tả.** `d:\tool_finding` **không phải git repository**. File `.gitignore` đã được tạo nhưng hiện
không có tác dụng gì.

**Rủi ro.** Kế hoạch có 9 giai đoạn. Không có VCS thì:
- Không thể xem đã đổi gì giữa các giai đoạn.
- Một thay đổi ở P5 làm hỏng P2 thì không có cách nào quay lại.
- Không có điểm khôi phục an toàn để thử nghiệm.

**Đề xuất.** `git init` + commit một mốc sau mỗi giai đoạn hoàn thành.

**→ Đã thực hiện (2026-08-24).** Người dùng đồng ý. Repo đã khởi tạo, `.gitattributes` chuẩn hoá
kết thúc dòng, và mỗi giai đoạn hoàn thành được commit thành một mốc riêng.


---

## RISK-003 ⚪ — Tệp media mới trong thư mục chưa từng có media sẽ bị cập nhật nhanh bỏ sót

**Giai đoạn:** P9 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Chưa xảy ra với người dùng**, nhưng đã thấy dấu vết trong log của chính lượt kiểm chứng:

```
áp thay đổi: +2 tệp, -0 tệp, ..., 5 bỏ qua (ngoài phạm vi index)
```

**Vấn đề.** Bảng `dirs` trong `Index` chỉ chứa những thư mục **có chứa tệp media đã được index** —
thư mục nào không có media thì không bao giờ được thêm vào, vì nó chỉ được tạo ra khi resolve chuỗi
cha của một tệp.

`rebuild_with` tra thư mục cha theo `(ổ, FRN)` trong bảng đó. Nên có một khoảng trống:

| Tình huống | Kết quả |
|---|---|
| Tệp mới trong thư mục **đã có** trong index | ✅ thêm đúng |
| Tệp mới trong thư mục **vừa được tạo** (thư mục đó nằm trong cùng lô journal) | ✅ dựng được cả chuỗi — đã kiểm chứng với `mf-test-newdir` |
| Tệp mới trong thư mục **có sẵn từ trước nhưng chưa từng chứa media** | ❌ **bỏ sót**, đếm vào `unresolved` |

Trường hợp thứ ba là thật và dễ gặp: một thư mục dự án tạo từ tuần trước, hôm nay mới bỏ video
đầu tiên vào. Journal báo tệp mới, nhưng FRN của thư mục cha không có trong bảng và cũng không nằm
trong lô journal — vì thư mục đó được tạo từ lâu rồi.

Tệp sẽ chỉ xuất hiện sau một lần **quét đầy đủ**.

**Vì sao chưa sửa ngay.** Cách sửa đúng đã rõ và không khó: khi gặp FRN cha không tra được, mở nó
bằng `OpenFileById` rồi lấy đường dẫn bằng `GetFinalPathNameByHandle`, sau đó thêm thư mục vào
bảng. Tiến trình indexer vốn đã có quyền Administrator và đã có sẵn handle volume, nên không cần
thêm quyền gì. Chi phí chỉ phát sinh với FRN chưa biết, tức là hiếm.

Chưa làm vì nó cần thêm một đường Win32 nữa, và phần cập nhật nhanh vừa mới được kiểm chứng xong —
thêm mã ngay lúc này sẽ trộn lẫn thứ đã kiểm chứng với thứ chưa.

## Đã sửa

**Cách sửa.** Khi không tra được FRN của thư mục cha, hỏi thẳng NTFS: `OpenFileById` mở thư mục
theo số hiệu, `GetFinalPathNameByHandleW` cho biết nó nằm ở đâu.

Ràng buộc kiến trúc: `index/update.rs` là đường nối **cố ý không có Win32** — đó là thứ khiến nó
test được trên máy bất kỳ, không cần quyền, không cần ổ NTFS. Nên phần tra cứu đi qua một trait:

```rust
pub enum DirAnswer {
    Path(String),   // có, và đây là đường dẫn
    Excluded,       // có, nhưng cố ý không index
    Unknown,        // không mở được, hoặc không còn tồn tại
}

pub trait DirLookup {
    fn path_of(&self, volume: u8, frn: u64) -> DirAnswer;
}
```

`rebuild_with` giữ nguyên chữ ký cũ (dùng `NoLookup`), thêm `rebuild_with_lookup`. Toàn bộ 21 test
cũ không phải sửa một dòng, và 6 test mới dùng một `DirLookup` giả — chạy được không cần quyền.

**Ba điều dễ sai đã xử lý.**

1. **Luật loại trừ phải áp lại.** Đường dẫn lấy từ NTFS đã **đi vòng qua** `tree.rs`, nơi vốn lọc
   từng thành phần một trong lúc đi ngược chuỗi cha. Không lọc lại thì `C:\Windows\Temp` sẽ
   chui thẳng vào index — một lỗ hổng rộng đúng bằng tính năng này. Thêm
   `ResolveOptions::excludes_path`, có test khoá lại.

2. **Hỏi một lần cho mỗi thư mục, không phải mỗi tệp.** Một thư mục mới thường xuất hiện kèm cả
   loạt tệp; hỏi theo từng tệp là mở một handle cho mỗi tệp. `discover_parents` gom và khử trùng
   lặp trước. Có test: 50 tệp cùng một thư mục → **1 lần hỏi**.

3. **Không hỏi về thư mục index đã biết.** Test dùng một `DirLookup` panic ngay khi bị gọi.

**Tách `unresolved` làm hai.** Trước đây một con số gộp cả "cố ý bỏ qua" lẫn "không tra được" thì
không nói lên điều gì. Nay `excluded` ghi ở mức `info` (đúng như thiết kế) còn `unresolved` ghi ở
mức `warn` — vì mỗi mục ở đó là một tệp có thật trên đĩa và thiếu trong chỉ mục.

**Kiểm chứng trên máy thật.** Đúng kịch bản, gộp trong một lần elevate:

```
BUOC 1: tạo thư mục + một tệp .txt, rồi quét đầy đủ
        → thư mục không có media nên KHÔNG vào index
BUOC 2: giờ mới bỏ một .mp4 vào thư mục đó
BUOC 3: cập nhật nhanh

áp thay đổi: +1 tệp, ..., +0 thư mục (1 hỏi hệ thống tệp), ...
  bỏ qua 0 thay đổi ngoài phạm vi index
cập nhật nhanh xong: 46701 mục [0.60s]
```

Đúng một lần hỏi, tệp vào index đúng đường dẫn và đúng dung lượng. Trước khi sửa, chính tệp đó bị
đếm vào `unresolved` và biến mất cho tới lần quét đầy đủ kế tiếp.

---

## RISK-004 ⚪ — Chỉ mục nằm chung thư mục với chương trình đã cài

**Giai đoạn:** P10 · **Trạng thái:** ĐÃ ĐO, CHẤP NHẬN · **Ngày:** 2026-08-25

**Tình huống.** Bộ cài NSIS của Tauri cài theo từng người dùng vào `%LOCALAPPDATA%\MediaFinder` —
**đúng thư mục** mà `persist::cache_dir()` đã dùng để chứa dữ liệu:

```
C:\Users\<user>\AppData\Local\MediaFinder\
    index.bin        47 MB   ← dữ liệu, 4,5 phút quét NAS mới có
    metadata.bin              ← dữ liệu
    progress.json             ← dữ liệu
    mediafinder.exe           ← chương trình
    uninstall.exe             ← chương trình
```

Nếu trình gỡ cài đặt xoá cả thư mục thì gỡ phần mềm sẽ lấy theo luôn chỉ mục.

**Đã đo, không suy đoán.** Đọc `installer.nsi` do Tauri sinh ra (`target/release/nsis/x64/`), mục
`Section Uninstall`:

```nsis
Delete "$INSTDIR\${MAINBINARYNAME}.exe"
Delete "$INSTDIR\uninstall.exe"
RMDir "$INSTDIR"
```

`RMDir` **không có `/r`**, nên nó chỉ xoá thư mục khi thư mục rỗng. Còn `index.bin` ở đó thì lệnh
này thất bại một cách vô hại. **Dữ liệu sống sót qua gỡ cài đặt.**

**Vì sao vẫn ghi lại.** Điều đó đúng vì *mẫu NSIS của Tauri hiện nay* viết như vậy — không phải vì
thiết kế của dự án này bảo vệ được nó. Một bản Tauri sau đổi thành `RMDir /r` là mất sạch, và mất
im lặng.

**Chưa đổi chỗ chứa cache**, vì đổi đường dẫn dữ liệu đồng nghĩa phải viết mã di chuyển tệp — mà
mỗi dòng mã chạm vào chỗ chứa chỉ mục lại là một cơ hội làm mất nó. Rủi ro hiện tại bằng 0; rủi ro
của việc sửa thì không.

**Việc cần làm khi nâng cấp Tauri:** đọc lại `Section Uninstall` trong `installer.nsi` mới sinh và
kiểm tra `RMDir` có mọc thêm `/r` không. Nếu có thì phải chuyển cache sang thư mục khác trước khi
phát hành.

## RISK-005 ⚪ — Thao tác kéo thỉnh thoảng không khởi động, chưa rõ nguyên nhân

**Giai đoạn:** P13 · **Trạng thái:** ĐANG THEO DÕI · **Ngày:** 2026-08-25

**Hiện tượng.** Trong lượt test P13, hai lần kéo không mang được gì sang cửa sổ nhận thả. Không
sập, không bảng lỗi, không dòng log nào. Sau khi dựng lại và cài lại, **đúng ca đó chạy đúng 5 lần
liên tiếp**. Tỉ lệ quan sát được: **2 hỏng / ~12 lần kéo**.

**Điều đã xác định được.** Nhật ký cho thấy `start_file_drag` **chưa từng được gọi** trong hai lần
đó. Nên lỗi nằm ở phía cửa sổ — sự kiện `dragstart` không nổ — chứ **không** ở tầng shell. Tầng
shell đã được loại trừ bằng bốn phép đo riêng, ghi ở [test-log](./test-log.md).

**Điều chưa xác định được.** Vì sao `dragstart` không nổ. Bốn giả thuyết đã bị bác bỏ bằng đo đạc:
đường dẫn mạng, tập trộn nhiều ổ, thời gian dựng data object, và chuyển hướng stdout/stderr.

**Vì sao để ở mức ⚪ chứ không nâng lên bug.** Cả hai lần đều dùng **chuột tổng hợp**
(`SetCursorPos` + `mouse_event`), và Chromium có ngưỡng khoảng cách/thời gian riêng để quyết định
một cử chỉ là kéo hay là click. Nhiều khả năng đây là giới hạn của cách test chứ không phải của
ứng dụng — nhưng **tôi chưa chứng minh được**, nên không đóng.

**Cách theo dõi.** Người dùng gặp lần nào "kéo mà không ra tệp" thì ghi lại vào
[test-log](./test-log.md): kéo từ hàng nào, đang chọn mấy hàng, đích thả là gì. Nếu xuất hiện với
chuột thật thì nâng thành bug và thêm log ở `onDragStart` phía giao diện để biết sự kiện có nổ hay
không.
