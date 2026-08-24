# HIỆU NĂNG — MediaFinder
> **Thuộc file này:** Chậm, tốn RAM, cấp phát thừa. Mỗi mục **bắt buộc kèm số đo** trước và sau.
> **KHÔNG thuộc file này:** lỗi làm sai kết quả — dù có chậm đi nữa.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `PERF-003`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [PERF-001](#perf-001) | 🟡 | Cấp phát `String` cho mỗi thành phần đường dẫn khi lọc | P1 | ĐÃ SỬA |
| [PERF-002](#perf-002) | ✅ | Chọn lọc tốn hơn quét ở `limit` lớn | P2 | ĐÃ TỐI ƯU |

---

## PERF-001 🟡 — Cấp phát `String` cho mỗi thành phần đường dẫn khi lọc

**Giai đoạn:** P1 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Bộ lọc thư mục cấm dùng `HashSet<String>`, so sánh bằng
`excluded.contains(&node.name.to_lowercase())`.

**Vấn đề.** `to_lowercase()` **cấp phát một `String` mới** cho mỗi thành phần đường dẫn được xét.
Trên ổ C: với hơn một triệu thư mục, đó là hàng trăm nghìn lần cấp phát rồi vứt đi ngay.

**Cách sửa.** Đổi sang `Vec<String>` + `eq_ignore_ascii_case` — không cấp phát. Danh sách chỉ 15
mục nên quét tuyến tính còn nhanh hơn băm, và mọi tên trong danh sách đều là ASCII nên so sánh
ASCII-insensitive đúng ngữ nghĩa.

---

## PERF-002 ✅ — Chọn lọc tốn hơn quét ở `limit` lớn

**Giai đoạn:** P2 · **Trạng thái:** ĐÃ TỐI ƯU · **Ngày:** 2026-08-24

**Phát hiện.** Bench `selection_cost` tách riêng chi phí chọn lọc khỏi chi phí quét:

| `limit` | Thời gian |
|---|---|
| 1 | 944 µs |
| 100 | 965 µs |
| 5.000 | **2,25 ms** |

Quét thực sự chỉ tốn ~944 µs. Ở `limit=5000`, **chọn lọc tốn 1,3 ms — nhiều hơn cả việc quét.**

**Nguyên nhân.** Mỗi chunk song song đóng góp tối đa `limit` kết quả. Với index 500k chia thành
31 chunk, vector gộp chứa tới **155.000 hit**, rồi `sort_unstable` toàn bộ để lấy ra 5.000.
Đó là khoảng 2,6 triệu phép so sánh để giữ lại 3% dữ liệu.

**Cách tối ưu.** Dùng `select_nth_unstable_by` phân hoạch trong O(n) **trước khi** sort, rồi chỉ
sort phần sống sót: ~215.000 phép so sánh thay vì 2,6 triệu.

**Kết quả đo được.** `limit=5000`: **2,25 ms → 1,35 ms (−39,5%)**. `limit=1` và `limit=100` không
đổi, đúng như dự đoán. Toàn bộ 71 test vẫn pass — thứ tự kết quả không thay đổi.

**Ghi chú.** Đây là tối ưu tìm ra **nhờ đo**, không phải nhờ đoán. Trực giác ban đầu là chi phí
nằm ở việc quét chuỗi; số liệu chỉ ra ngược lại.
