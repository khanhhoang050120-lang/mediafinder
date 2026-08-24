# KIỂM CHỨNG — MediaFinder
> **Thuộc file này:** Nghi ngờ đã được đem đi đo bằng công cụ độc lập. Ghi lại **cả khi hoá ra không phải lỗi** — biết một thứ đã được kiểm chứng cũng quan trọng như biết nó hỏng.
> **KHÔNG thuộc file này:** lỗi đã xác định rõ (thuộc bug.md).
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `CHECK-002`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [CHECK-001](#check-001) | ✅ | Nghi ngờ pha 2 loại nhầm 99,7% file trên C: → KHÔNG PHẢI LỖI | P1 | KHÔNG PHẢI LỖI (đã kiểm chứng) |

---

## CHECK-001 ✅ — Nghi ngờ pha 2 loại nhầm 99,7% file trên C: → KHÔNG PHẢI LỖI

**Giai đoạn:** P1 · **Trạng thái:** KHÔNG PHẢI LỖI (đã kiểm chứng) · **Ngày:** 2026-08-24

**Nghi ngờ ban đầu.** Thống kê quét ổ C: trông rất đáng ngờ:

```
pha 1: 3.559.309 bản ghi → giữ 872.803 tệp media
pha 2: giữ 2.437 / loại 870.366 (thư mục cấm)   ← loại 99,7%
```

Loại bỏ 99,7% là con số đủ bất thường để phải nghi ngờ logic loại trừ khớp quá rộng.

**Cách kiểm chứng.** Đếm độc lập bằng PowerShell `Get-ChildItem -Recurse`, hoàn toàn không dùng
lại code của dự án, trên các thư mục **không** nằm trong danh sách cấm:

| Thư mục | Trong danh sách cấm? | PowerShell đếm được |
|---|---|---|
| `C:\Users\Padoma1\.gradle` | không | 1.119 |
| `C:\Users\Padoma1\.rustup` | không | 127 |
| `C:\Users\Padoma1\Pictures` | không | 168 |
| **Cộng (phải được GIỮ)** | | **1.414** |
| `C:\Users\Padoma1\AppData` | **có** | **829.251** |

**Kết luận — khớp cả hai chiều.**

*Chiều giữ lại:* 1.414 trong tổng 2.437 mà scanner giữ đến từ ba thư mục trên; phần còn lại từ
`MSI`, `Downloads`, `.vscode`, `.antigravity-ide`, `.cache`, `Users\Public`.

*Chiều loại bỏ:* riêng `AppData` đã có **829.251** file media — chiếm 95% trong số 870.366 file
bị loại. Phần còn lại (~41.000) đến từ `Windows`, `Program Files`, `ProgramData`.

**Logic loại trừ hoạt động chính xác.** Trên máy có nhiều công cụ lập trình, `AppData` (cache
trình duyệt, ứng dụng Electron, cache npm/pip/gradle) chứa gần một triệu ảnh là hoàn toàn bình
thường — và đó đúng là thứ cần loại bỏ khỏi một công cụ tìm media.

**Bài học.** Một con số trông "sai" chưa chắc là lỗi — nhưng cũng không được cho qua chỉ vì code
trông có vẻ đúng. Phải đo bằng một công cụ độc lập.

---

## CHECK-002 ✅ — "Phím tắt tự nhiên ngừng hoạt động" — hoá ra là do kịch bản test

**Giai đoạn:** P8 · **Trạng thái:** KHÔNG PHẢI LỖI · **Ngày:** 2026-08-24

**Nghi ngờ.** Sau vài lượt test, `Ctrl+Alt+Space` không còn gọi được cửa sổ nữa. Cửa sổ nằm ở
trạng thái thu nhỏ và không phản ứng. Nếu đúng là phím tắt "hỏng dần theo thời gian" thì đây là
lỗi nặng — người dùng sẽ gặp nó sau vài giờ dùng máy.

**Cách đo.** Ba câu hỏi, mỗi câu một phép đo độc lập:

| Câu hỏi | Cách đo | Kết quả |
|---|---|---|
| Có phím bổ trợ nào bị kẹt không? | `GetAsyncKeyState` cho Shift/Ctrl/Alt/Win | Không, cả bốn đều nhả |
| Ứng dụng còn giữ đăng ký không? | Tiến trình khác thử `RegisterHotKey` cùng tổ hợp | Trả `1409` — **vẫn giữ** |
| `unminimize` có làm việc không? | `SW_MINIMIZE` rồi bấm phím tắt, đọc `IsIconic` | `True` → `False`, **phục hồi được** |

Ba chu kỳ thu nhỏ ↔ gọi lại liên tiếp đều thành công.

**Kết luận.** Phím tắt không hỏng. Lần "không phản ứng" là do cửa sổ đang ở trạng thái mà kịch bản
test không lường: bấm phím trong lúc foreground thuộc về cửa sổ khác thì `toggle` gọi `summon` —
đúng như thiết kế — nhưng phép đo của tôi lại đọc trạng thái quá sớm / sai cửa sổ. Cùng gốc với
[BUG-016](./bug.md#bug-016).

**Chi phí thật.** Đo cả ba câu hỏi mất khoảng bốn phút. Bốn phút này đổi lấy việc **không** đi sửa
một lỗi không tồn tại trong `register_hotkey` — nơi mà mọi thay đổi đều có thể làm hỏng nhánh đang
chạy đúng.
