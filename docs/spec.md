# LỖI CỦA ĐẶC TẢ — MediaFinder
> **Thuộc file này:** Code làm **đúng y** những gì đặc tả yêu cầu, nhưng chính yêu cầu đó sai. Loại này chỉ lộ ra khi chạy trên dữ liệu thật.
> **KHÔNG thuộc file này:** code làm sai so với đặc tả — đó là bug.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `SPEC-002`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [SPEC-001](#spec-001) | 🔴 | Đặc tả chỉ tìm trong tên file → vô dụng với dữ liệu thật | P2 | ĐÃ SỬA |

---

## SPEC-001 🔴 — Đặc tả chỉ tìm trong tên file → vô dụng với dữ liệu thật

**Giai đoạn:** P2 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Sau khi dựng index từ lượt quét thật (117.123 tệp media), chạy thử tìm kiếm:

```
"tieng viet" → 0 kết quả
"da nang"    → 0 kết quả
"bai"        → 5 kết quả  ✓
```

Ban đầu trông như lỗi fold tiếng Việt. Nhưng `bai` **có** tìm ra `bài 10.mp3` và
`BÀI 75____The BEST and WORST Forms of Magnesium.mp3` — fold hoạt động hoàn hảo.

**Nguyên nhân thật.** Đặc tả gốc mục 3.3 quy định:

> *"Thuật toán lọc dựa trên việc kiểm tra chuỗi con chứa trong chuỗi tên tệp"*

Chỉ tên tệp. Nhưng thư viện thật được tổ chức thế này:

```
D:\Sounds Edit\HƯNG\WISE\DATA TẠO VID HƯNG\HAN QUOC\13\BÀI 13_ UROLOGIST_...\154.mp3
   └───────────────── mọi từ khoá tìm được đều nằm ở đây ─────────────────┘  └─ tên tệp
```

Tên tệp là `154.mp3`, `27.mp3`, `seg_116.wav`, `b000_why-giant-squids.mp4`. **Toàn bộ ý nghĩa
nằm trong tên thư mục.** Với cách tổ chức này, tìm theo tên tệp trả về gần như không gì cả.

**Vì sao đây là lỗi đặc tả chứ không phải lỗi code.** Code làm đúng y những gì đặc tả yêu cầu.
Chỉ có dữ liệu thật mới phơi ra rằng yêu cầu đó sai. Everything cũng tìm cả đường dẫn — đó là
hành vi đúng, và đặc tả gốc đã bỏ sót.

**Cách sửa.** Tìm cả trong đường dẫn thư mục, nhưng có ba ràng buộc:

1. **Điểm thư mục luôn thấp hơn mọi điểm tên tệp** (`DIR_WORD_START` 250 / `DIR_SUBSTRING` 200
   so với 400–1000 của tên tệp). Một tệp thật sự tên `holiday.mp4` không bao giờ bị đẩy xuống
   dưới một tệp chỉ nằm trong thư mục tên `holiday videos`.
2. **Chấm điểm thư mục một lần cho cả truy vấn**, không phải một lần cho mỗi tệp. 116k tệp dùng
   chung 4k thư mục → tiết kiệm khoảng **28 lần** công việc. Kết quả lưu trong bảng phẳng
   `dir_count × token_count`, tra cứu O(1) trong vòng lặp nóng.
3. **Chuỗi folded của thư mục lưu theo thư mục**, không theo tệp — vài trăm KB thay vì hàng chục MB.

**Lợi ích kèm theo.** Truy vấn nhiều từ khoá giờ có thể trải giữa thư mục và tên tệp:
`avatar 2024` khớp `D:\Phim\2024\avatar.mkv` — `2024` lấy từ thư mục, `avatar` từ tên tệp.

**Bài học.** Đây là lỗi nghiêm trọng nhất tìm được từ đầu dự án, và **không một unit test nào có
thể bắt được** — vì test do tôi tự nghĩ ra dữ liệu, và tôi đặt tên tệp có nghĩa như người ta
thường làm. Chỉ có dữ liệu thật của người dùng mới lộ ra cách tổ chức khác hẳn.
