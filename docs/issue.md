# VẤN ĐỀ SẢN PHẨM — MediaFinder
> **Thuộc file này:** Code chạy đúng nhưng kết quả không phục vụ được người dùng. Thường **cần người dùng quyết định** chứ không tự sửa được.
> **KHÔNG thuộc file này:** lỗi kỹ thuật.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `ISSUE-003`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [ISSUE-001](#issue-001) | 🟠 | Kết quả trên C: toàn tài nguyên công cụ, không phải media người dùng | P1 | **ĐÃ SỬA** (P2) |
| [ISSUE-002](#issue-002) | 🟡 | Huy hiệu loại tệp dùng chữ cái tiếng Anh trong giao diện tiếng Việt | P5 | ĐÃ SỬA |

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

---

## ISSUE-002 🟡 — Huy hiệu loại tệp dùng chữ cái tiếng Anh trong giao diện tiếng Việt

**Giai đoạn:** P5 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Cách phát hiện.** Người dùng gửi ảnh chụp màn hình và hỏi: *"I và V trong bức ảnh này là sao?"*

Câu hỏi đó chính là câu trả lời.

**Vấn đề.** Mỗi kết quả có một huy hiệu màu ghi chữ cái đầu của **tên tiếng Anh** loại tệp:
`V` (Video), `I` (Image), `A` (Audio). Nhưng toàn bộ giao diện là tiếng Việt, và **ngay phía trên
danh sách** là ba chip lọc ghi rõ **"Video / Ảnh / Nhạc"**.

Nên người dùng nhìn thấy:

| Chip lọc | Huy hiệu | Khớp? |
|---|---|---|
| Video | `V` | có |
| **Ảnh** | **`I`** | **không** |
| **Nhạc** | **`A`** | **không** |

Hai trong ba nhãn không liên quan gì tới chữ cái bên cạnh chúng.

**Vì sao là `ISSUE-` chứ không phải `BUG-`.** Code chạy đúng y những gì tôi viết. Chuỗi `"image"`
lấy chữ cái đầu ra `I` — không sai chỗ nào. Chỉ là **thiết kế sai** cho người sẽ đọc nó.

**Cách sửa.** Thay chữ cái bằng **biểu tượng**: nút play cho video, khung ảnh cho ảnh, nốt nhạc
cho nhạc. Hình khối không cần dịch. Giữ nguyên mã màu, và thêm `title`/`aria-label` lấy đúng từ
danh sách chip nên huy hiệu và chip bật/tắt nó không bao giờ lệch nhau.

**Bài học.** Chuỗi hằng số trong code thì bằng tiếng Anh — đó là chuyện bình thường và đúng.
Nhưng **bất cứ thứ gì lọt ra màn hình đều là giao diện**, kể cả một chữ cái. Tôi rút chữ cái đó
thẳng từ định danh nội bộ mà không hỏi nó có nghĩa gì với người đọc.

Đáng chú ý: tôi đã dịch mọi câu, mọi nhãn nút, mọi thông báo lỗi sang tiếng Việt — rồi để lọt
đúng ba chữ cái. Chỗ dễ lọt nhất là chỗ trông không giống văn bản.
