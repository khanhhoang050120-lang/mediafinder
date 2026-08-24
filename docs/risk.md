# RỦI RO — MediaFinder
> **Thuộc file này:** Chưa gây hại, nhưng sẽ gây hại nếu không xử lý. Mỗi mục phải ghi rõ điều kiện kích hoạt và hạn xử lý.
> **KHÔNG thuộc file này:** lỗi đã xảy ra rồi (thuộc bug.md).
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

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

**Giai đoạn:** P9 · **Trạng thái:** CẦN QUYẾT ĐỊNH · **Ngày:** 2026-08-24

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

**Cách nhận biết nếu nó đang xảy ra.** Con số `unresolved` trong log. Hiện tại nó cũng đếm cả những
thay đổi dưới thư mục **cố ý bị loại** (`Windows`, `AppData`…), vốn hoàn toàn bình thường — nên nếu
làm tiếp thì nên tách hai loại này ra, chứ một con số gộp thì không nói lên điều gì.
