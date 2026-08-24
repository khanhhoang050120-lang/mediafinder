# VẤN ĐỀ SẢN PHẨM — MediaFinder
> **Thuộc file này:** Code chạy đúng nhưng kết quả không phục vụ được người dùng. Thường **cần người dùng quyết định** chứ không tự sửa được.
> **KHÔNG thuộc file này:** lỗi kỹ thuật.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `ISSUE-002`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [ISSUE-001](#issue-001) | 🟠 | Kết quả trên C: toàn tài nguyên công cụ, không phải media người dùng | P1 | **ĐÃ SỬA** (P2) |

---

## ISSUE-001 🟠 — Kết quả trên C: toàn tài nguyên công cụ, không phải media người dùng

**Giai đoạn:** P1 (đã xử lý ở P2) · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Hiện tượng.** Trong 20 đường dẫn mẫu lấy trải đều trên ổ C:, phần lớn là tài nguyên của công cụ
lập trình chứ không phải media người dùng:

```
C:\Users\Padoma1\.gradle\caches\...\res\drawable-xxhdpi-v4\ic_call_answer_low.png
C:\Users\Padoma1\.rustup\toolchains\...\doc\rust\html\cargo\images\...png
C:\Users\Padoma1\.vscode\extensions\ms-vscode.powershell-...\media\PowerShell_Icon.png
C:\Users\Padoma1\.antigravity-ide\extensions\...\doc-assets\complete.png
```

**Vì sao đáng quan tâm.** Đây không phải lỗi kỹ thuật — chúng đúng là file ảnh. Nhưng sản phẩm
này là **công cụ tìm media**, và icon 16×16 của một extension VS Code thì không bao giờ là thứ
người dùng đi tìm. Chúng làm loãng xếp hạng ở P2 và tốn công sinh thumbnail ở P5.

**Hai hướng xử lý.**

1. **Mở rộng danh sách cấm** thêm thư mục công cụ/cache: `.gradle` `.rustup` `.cargo` `.npm`
   `.nuget` `.cache` `.vscode` `.git` `__pycache__` `site-packages` `vendor` `target` `dist`
   `build`. → Kết quả sạch hơn hẳn. Rủi ro: media thật để trong thư mục tên `build` sẽ mất.
2. **Bỏ `svg` và `ico`** khỏi bảng phần mở rộng. → Hai đuôi này gần như luôn là tài nguyên giao
   diện ứng dụng, không phải media người dùng.

**Chưa tự quyết** vì đây là quyết định về sản phẩm chứ không phải sửa lỗi — nó thay đổi thứ
người dùng tìm thấy được.

**→ Đã giải quyết (2026-08-24).** Người dùng cho biết **không lưu ảnh/video trên ổ C:**, nên có
thể lọc mạnh tay mà không sợ mất dữ liệu thật.

Cách xử lý: thay vì liệt kê từng thư mục công cụ (rồi phải bổ sung mãi mãi), thêm **một quy tắc
tổng quát** — `skip_dot_directories`: bỏ qua mọi thư mục có tên bắt đầu bằng dấu chấm.

Vì sao quy tắc này đúng:
- Trên Windows, dotfolder là quy ước dành cho công cụ và cấu hình; media người dùng gần như
  không bao giờ nằm ở đó.
- Nó bao phủ **mọi công cụ cài trong tương lai** mà không cần biết tên trước.
- Nó bắt luôn rác do ứng dụng tự tạo: CapCut giấu bản nháp đã xoá trong `.recycle_bin` **ngay
  bên trong thư mục dự án của người dùng** trên ổ D:.

Kiểm chứng bằng chính các đường dẫn có thật từ lượt quét P1: `.gradle` `.rustup` `.vscode`
`.antigravity-ide` `.cache` `.recycle_bin` — tất cả đều bị loại.

Bổ sung thêm 3 tên rõ nghĩa không bắt đầu bằng dấu chấm: `bower_components` `__pycache__`
`site-packages`.

**Cố ý KHÔNG cấm** `build` `dist` `target` `bin` `obj` `vendor` `packages`. Chúng phổ biến trong
cây mã nguồn, nhưng cũng là tên thư mục hoàn toàn bình thường mà người ta có thể để media trong
đó. Rủi ro không cân xứng: **mất file của người dùng trong im lặng tệ hơn nhiều so với hiện thừa
vài file.** Có test `ordinary_folder_names_are_never_excluded` khoá lại quyết định này.
