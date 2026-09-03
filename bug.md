# → Đã chuyển sang thư mục [`docs/`](./docs/)

File này từng chứa mọi loại vấn đề trộn lẫn. Khi lên tới 586 dòng thì tra cứu quá khổ, nên đã
tách thành các file riêng theo phân loại.

**Bắt đầu ở [`docs/README.md`](./docs/README.md)** — có bảng "ghi vào file nào" và toàn cảnh.

| Loại | File | Nội dung |
|---|---|---|
| Lỗi | [docs/bug.md](./docs/bug.md) | Code cho ra kết quả sai, crash, treo |
| Cấu hình | [docs/config.md](./docs/config.md) | Hai thiết lập đụng nhau, sai phiên bản thư viện |
| Rủi ro | [docs/risk.md](./docs/risk.md) | Chưa gây hại, sẽ gây hại nếu bỏ qua |
| Hiệu năng | [docs/perf.md](./docs/perf.md) | Chậm hoặc tốn RAM — bắt buộc kèm số đo |
| Kiểm chứng | [docs/check.md](./docs/check.md) | Nghi ngờ đã đem đi đo, kể cả khi hoá ra không phải lỗi |
| Sản phẩm | [docs/issue.md](./docs/issue.md) | Chạy đúng nhưng kết quả không dùng được |
| Đặc tả | [docs/spec.md](./docs/spec.md) | Code đúng y yêu cầu, nhưng yêu cầu sai |
| Nhật ký test (P0–P33, `master`) | [docs/test-log.md](./docs/test-log.md) | Kết quả từng lượt test theo giai đoạn |
| Nhật ký test (P34+, `version2`) | [docs/test-log-v2.md](./docs/test-log-v2.md) | Nhánh hiện tại — ghi vào đây |

> File con trỏ này giữ lại để không ai ghi nhầm vào đây. Đừng thêm nội dung mới vào file này.
