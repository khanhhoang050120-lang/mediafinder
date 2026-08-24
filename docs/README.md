# Sổ ghi vấn đề — MediaFinder

Mục lục của thư mục `docs/`. Mỗi loại vấn đề có file riêng để tra cứu nhanh.

Liên quan: [`../PROGRESS.md`](../PROGRESS.md) (tiến độ 9 giai đoạn) · [`../README.md`](../README.md) (bất biến kiến trúc)

## Ghi vào file nào?

Đọc từ trên xuống, dừng ở dòng đầu tiên khớp.

| Câu hỏi | Nếu đúng → file | Tiền tố ID |
|---|---|---|
| Chính **bản đặc tả** yêu cầu sai, code làm đúng y yêu cầu đó? | [spec.md](./spec.md) | `SPEC-` |
| Code chạy đúng nhưng **kết quả không dùng được** cho người dùng? | [issue.md](./issue.md) | `ISSUE-` |
| Là **nghi ngờ** đã đem đi đo, kể cả khi hoá ra không phải lỗi? | [check.md](./check.md) | `CHECK-` |
| Là **hai thiết lập đụng nhau** / sai phiên bản thư viện? | [config.md](./config.md) | `CONF-` |
| **Chưa gây hại** nhưng sẽ gây hại nếu bỏ qua? | [risk.md](./risk.md) | `RISK-` |
| **Chậm hoặc tốn RAM**, nhưng kết quả vẫn đúng? | [perf.md](./perf.md) | `PERF-` |
| Còn lại: code cho ra **kết quả sai**, crash, treo | [bug.md](./bug.md) | `BUG-` |

Kết quả từng **lượt test** theo giai đoạn thì ghi vào [test-log.md](./test-log.md), không ghi vào các file trên.

## Quy ước chung

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Mỗi mục cần có:** Giai đoạn · Trạng thái · Ngày · Hiện tượng · Nguyên nhân · Cách sửa. Với `PERF-` thì **bắt buộc kèm số đo** trước và sau.

## Toàn cảnh

| File | Số mục | Còn mở | Nội dung |
|---|---|---|---|
| [bug.md](./bug.md) | 7 | 1 | Lỗi làm phần mềm chạy sai |
| [config.md](./config.md) | 4 | 0 | Xung đột cấu hình / phiên bản |
| [risk.md](./risk.md) | 2 | 1 | Rủi ro chưa xảy ra |
| [perf.md](./perf.md) | 2 | 0 | Hiệu năng, kèm số đo |
| [check.md](./check.md) | 1 | 0 | Nghi ngờ đã kiểm chứng |
| [issue.md](./issue.md) | 1 | 0 | Vấn đề sản phẩm |
| [spec.md](./spec.md) | 2 | 0 | Lỗi của bản đặc tả |
| **Cộng** | **19** | **1** | |

### Hai mục còn mở

| ID | Mức | Tiêu đề | Chờ gì |
|---|---|---|---|
| [BUG-002](./bug.md#bug-002) | 🟠 | Cửa sổ mở ở trạng thái minimize | **Người dùng xác minh** — chạy `npm run tauri dev` từ terminal thường, xem cửa sổ có tự hiện lên không |
| [RISK-001](./risk.md#risk-001) | ⚪ | `panic = "abort"` khiến panic trong IPC giết cả app | **Quyết định ở P3**, khi có Tauri command thật |

## Ba lỗi đáng nhớ nhất

Không phải vì nặng nhất, mà vì mỗi lỗi dạy một cách tìm lỗi khác nhau.

**[SPEC-001](./spec.md#spec-001) — đặc tả chỉ tìm trong tên tệp.** 67 unit test không bắt được, vì dữ liệu test do chính người viết nghĩ ra, mà người ta luôn đặt tên tệp có nghĩa. Thư viện thật lại tổ chức ngược hẳn: `...\DATA TẠO VID HƯNG\HAN QUOC\13\BÀI 13...\154.mp3` — tên thư mục mang toàn bộ ý nghĩa, tên tệp chỉ là số.
→ *Chỉ dữ liệu thật mới phơi ra được loại lỗi này.*

**[BUG-004](./bug.md#bug-004) — `.ts` bị phân loại thành video.** Đúng là đuôi MPEG transport stream. Nhưng trên máy có mã nguồn, TypeScript áp đảo hàng nghìn lần. Lộ ra ngay lượt quét thật đầu tiên.
→ *Bảng tra cứu do người viết tự nghĩ luôn phản ánh thế giới quan của người viết.*

**[CHECK-001](./check.md#check-001) — 99,7% file bị loại trên C:.** Con số trông sai đến mức phải nghi ngờ. Đếm chéo bằng PowerShell cho thấy logic **đúng** — riêng `AppData` có 829.251 file media.
→ *Một con số trông sai chưa chắc là lỗi, nhưng cũng không được cho qua chỉ vì code trông có vẻ đúng. Phải đo bằng công cụ độc lập.*

Và bài học từ mục 14 của [lượt test P2](./test-log.md): **kết quả rỗng cũng phải kiểm chứng.** `tieng viet → 0 kết quả` trông y hệt một lỗi fold tiếng Việt; hoá ra đơn giản là không có thư mục nào tên như vậy. Câu truy vấn thử nghiệm phải lấy từ dữ liệu thật, không được tự nghĩ ra — nếu không thì không phân biệt nổi "hỏng" với "không tồn tại".
