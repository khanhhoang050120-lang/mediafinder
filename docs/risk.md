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
| [RISK-001](#risk-001) | ⚪ | `panic = "abort"` khiến panic trong IPC giết cả app | P0 | MỞ |
| [RISK-002](#risk-002) | ⚪ | Dự án chưa có version control | P0 | MỞ |

---

## RISK-001 ⚪ — `panic = "abort"` khiến panic trong IPC giết cả app

**Giai đoạn:** P0 · **Trạng thái:** MỞ · **Ngày:** 2026-08-24

**Mô tả.** `src-tauri/Cargo.toml` đặt `panic = "abort"` trong `[profile.release]` để binary nhỏ và
nhanh hơn. Hệ quả: không có stack unwinding, nên `catch_unwind` vô hiệu.

**Rủi ro.** Khi có Tauri command thật ở P3, một panic trong handler (ví dụ index truy cập ngoài phạm vi,
`unwrap()` trên `None`) sẽ **giết toàn bộ ứng dụng** thay vì trả lỗi về frontend. Với bản release
người dùng cuối, app sẽ biến mất không thông báo.

**Chưa gây hại vì.** P0 chưa có command nào; hiện chỉ chạy bản debug (`panic = unwind`).

**Việc cần làm ở P3.** Quyết định một trong hai:
1. Bỏ `panic = "abort"`, chấp nhận binary lớn hơn, đổi lấy khả năng phục hồi.
2. Giữ `abort`, nhưng **cấm tuyệt đối `unwrap()` / `expect()` / index trực tiếp trong mọi command
   handler** — bắt buộc trả `Result<_, String>`.

Khuyến nghị hiện tại: **phương án 2**, vì code index/search đằng nào cũng nên trả `Result`.
Cần kiểm chứng lại bằng test khi có command thật.

---

## RISK-002 ⚪ — Dự án chưa có version control

**Giai đoạn:** P0 · **Trạng thái:** MỞ · **Ngày:** 2026-08-24

**Mô tả.** `d:\tool_finding` **không phải git repository**. File `.gitignore` đã được tạo nhưng hiện
không có tác dụng gì.

**Rủi ro.** Kế hoạch có 9 giai đoạn. Không có VCS thì:
- Không thể xem đã đổi gì giữa các giai đoạn.
- Một thay đổi ở P5 làm hỏng P2 thì không có cách nào quay lại.
- Không có điểm khôi phục an toàn để thử nghiệm.

**Đề xuất.** `git init` + commit một mốc sau mỗi giai đoạn hoàn thành.
**Chưa thực hiện** vì khởi tạo repo là quyết định của người dùng, cần được đồng ý trước.
