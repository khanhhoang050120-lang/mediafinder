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
| [bug.md](./bug.md) | 18 | 0 | Lỗi làm phần mềm chạy sai |
| [config.md](./config.md) | 5 | 0 | Xung đột cấu hình / phiên bản |
| [risk.md](./risk.md) | 3 | 0 | Rủi ro chưa xảy ra |
| [perf.md](./perf.md) | 2 | 0 | Hiệu năng, kèm số đo |
| [check.md](./check.md) | 7 | 0 | Nghi ngờ đã kiểm chứng |
| [issue.md](./issue.md) | 3 | 0 | Vấn đề sản phẩm |
| [spec.md](./spec.md) | 2 | 0 | Lỗi của bản đặc tả |
| **Cộng** | **40** | **0** | |

### Mục còn mở

_Không còn mục nào đang chờ xử lý._

Ba mục ở trạng thái `WORKAROUND` — đã có cách xử lý, chưa sửa triệt để:
[BUG-001](./bug.md#bug-001) (dùng `EnumWindows` thay `MainWindowHandle`),
[BUG-016](./bug.md#bug-016) (kiểm tra foreground trước khi bơm phím) và
[CONF-004](./config.md#conf-004) (dọn tiến trình giữ port 1420 trước khi chạy dev).

## Vì sao tìm trùng lặp không băm toàn bộ

Thư viện này 3 TB. Băm hết là hàng giờ đọc đĩa liên tục. Ba tầng, mỗi tầng chỉ nhìn thứ sống sót từ tầng trước:

| Tầng | Phép thử | Đọc | Còn lại |
|---|---|---|---|
| 1 | Cùng số byte | **không đọc gì** — index đã có sẵn | 60% (thư viện này) |
| 2 | Cùng 64 KB đầu, 64 KB cuối, và dung lượng | 128 KB mỗi tệp | gần như chắc chắn trùng |
| 3 | Cùng nội dung toàn bộ | tất cả | chắc chắn |

Tầng 1 làm phần nặng nhất mà **miễn phí**: hai tệp khác dung lượng thì không thể giống nhau.

Tầng 2 là nơi dừng lại trong thực tế. Hai tệp khác nhau mà trùng cả dung lượng, cả 64 KB đầu **và** 64 KB cuối thì gần như không xảy ra ngẫu nhiên — định dạng media đặt header ở đầu và bảng chỉ mục ở cuối, đúng hai chỗ này nhìn vào.

**Nhưng đó vẫn là một giới hạn thật.** Hai tệp giống hai đầu mà khác ở giữa sẽ bị coi là trùng. Có test thừa nhận điều đó (`a_difference_in_the_middle_is_invisible_to_tier_two`), giao diện nói rõ điều đó, và **không có gì tự động xoá**. Tầng 3 tồn tại cho lúc cần chắc chắn trước khi xoá.

## Ba lỗi đáng nhớ nhất

Không phải vì nặng nhất, mà vì mỗi lỗi dạy một cách tìm lỗi khác nhau.

**[SPEC-001](./spec.md#spec-001) — đặc tả chỉ tìm trong tên tệp.** 67 unit test không bắt được, vì dữ liệu test do chính người viết nghĩ ra, mà người ta luôn đặt tên tệp có nghĩa. Thư viện thật lại tổ chức ngược hẳn: `...\DATA TẠO VID HƯNG\HAN QUOC\13\BÀI 13...\154.mp3` — tên thư mục mang toàn bộ ý nghĩa, tên tệp chỉ là số.
→ *Chỉ dữ liệu thật mới phơi ra được loại lỗi này.*

**[BUG-004](./bug.md#bug-004) — `.ts` bị phân loại thành video.** Đúng là đuôi MPEG transport stream. Nhưng trên máy có mã nguồn, TypeScript áp đảo hàng nghìn lần. Lộ ra ngay lượt quét thật đầu tiên.
→ *Bảng tra cứu do người viết tự nghĩ luôn phản ánh thế giới quan của người viết.*

**[CHECK-001](./check.md#check-001) — 99,7% file bị loại trên C:.** Con số trông sai đến mức phải nghi ngờ. Đếm chéo bằng PowerShell cho thấy logic **đúng** — riêng `AppData` có 829.251 file media.
→ *Một con số trông sai chưa chắc là lỗi, nhưng cũng không được cho qua chỉ vì code trông có vẻ đúng. Phải đo bằng công cụ độc lập.*

Và bài học từ mục 14 của [lượt test P2](./test-log.md): **kết quả rỗng cũng phải kiểm chứng.** `tieng viet → 0 kết quả` trông y hệt một lỗi fold tiếng Việt; hoá ra đơn giản là không có thư mục nào tên như vậy. Câu truy vấn thử nghiệm phải lấy từ dữ liệu thật, không được tự nghĩ ra — nếu không thì không phân biệt nổi "hỏng" với "không tồn tại".
