# VẤN ĐỀ SẢN PHẨM — MediaFinder
> **Thuộc file này:** Code chạy đúng nhưng kết quả không phục vụ được người dùng. Thường **cần người dùng quyết định** chứ không tự sửa được.
> **KHÔNG thuộc file này:** lỗi kỹ thuật.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md) · [test-log-v2](./test-log-v2.md)

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

---

## ISSUE-003 🔴 — Kiến trúc MFT/USN không đọc được NAS, mà thư viện lớn nhất lại nằm ở đó

**Giai đoạn:** P9 → P10 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-25

**Vấn đề.** Máy người dùng có ba ổ mạng tổng ~37,9 TB, so với 4,2 TB đĩa cục bộ đang được index.
Toàn bộ nền tảng kỹ thuật của dự án — đọc MFT qua `FSCTL_ENUM_USN_DATA` — **không áp dụng được**
cho chúng, và không phải vì thiếu tính năng mà vì bản chất:

`\.\Z:` mở một **volume**. Ổ mạng không phải volume trên máy này — nó là một phiên SMB. Máy khách
nói chuyện với máy chủ bằng giao thức tệp, không thấy đĩa, không thấy MFT, không thấy USN journal.
Máy chủ có chạy NTFS hay không cũng không đổi được điều đó.

Nói cách khác: mọi thứ khiến MediaFinder nhanh đều là thứ **chỉ tồn tại với đĩa gắn trực tiếp**.

**Đã đo.** Duyệt thư mục theo chiều rộng, một luồng, 20 giây mỗi ổ:

| Ổ | Mục/giây | Tệp thấy được | Trong đó là media |
|---|---|---|---|
| Z: (mạng) | 1.584 | 29.865 | 18.941 |
| Y: (mạng) | 1.308 | 24.210 | 21.400 |
| F: (mạng) | 1.530 | 26.792 | 22.237 |
| **D: (cục bộ)** | **3.219** | 48.770 | 9.147 |

Hai điều đáng chú ý:

1. **Qua mạng chỉ chậm hơn cục bộ khoảng 2 lần** khi dùng cùng một phương pháp duyệt. Con số này
   nhỏ hơn nhiều so với dự đoán thông thường về SMB.
2. Nút cổ chai là **độ trễ vòng lặp yêu cầu**, không phải băng thông — nên chạy song song nhiều
   luồng sẽ ăn thẳng vào phần chờ. Đây là loại việc song song hoá rất tốt.

Cũng đáng chú ý: trên NAS, **63–88% tệp là media**, so với 19% trên ổ D:. Đúng như dự đoán — NAS là
nơi để chứa thư viện, còn ổ cục bộ chứa lẫn cả phần mềm và mã nguồn.

**Hướng giải quyết.** Một **bộ quét thứ hai** phía sau **cùng một index**: duyệt thư mục song song
cho ổ mạng, thay vì đọc MFT. Phần còn lại của hệ thống không cần biết:

- `ResolvedSet { dirs, dir_frns, files }` chính là đường nối. Bộ duyệt sinh ra đúng hình dạng đó.
- Tìm kiếm, xếp hạng, fold tiếng Việt, thumbnail, lọc metadata, tìm trùng lặp — không sửa gì.
- `rebuild_with` vừa viết ở P9 nhận `Change` từ **bất kỳ** nguồn nào, không chỉ từ USN journal.

**Ba điều phải quyết trước khi làm.**

| Vấn đề | Chi tiết |
|---|---|
| **Định danh** | Tệp qua SMB **không có FRN**. `rebuild_with` đang khoá theo `(ổ, FRN)`. Ổ mạng buộc phải khoá theo **đường dẫn**. Nghĩa là `Index` cần một khái niệm định danh rộng hơn, và đó là một lần đổi schema nữa |
| **Cập nhật** | `ReadDirectoryChangesW` **có** hoạt động qua SMB nếu máy chủ hỗ trợ change notify. Cần thử thật trên chính hai NAS này, không được tin vào tài liệu |
| **Tốc độ của các tính năng khác** | Thumbnail và tìm trùng lặp phải **đọc nội dung tệp qua mạng**. Tìm trùng lặp đọc 64 KB đầu + 64 KB cuối mỗi ứng viên — trên 38 TB qua mạng thì đây là việc hoàn toàn khác về quy mô so với 584 giây đã đo trên đĩa cục bộ |

## Đã làm — quét NAS theo yêu cầu, không tự động

**Quyết định của người dùng, và nó đúng:** *"sẽ khá là ít tôi lên NAS để kiếm file… việc cần thiết
quét là quét ổ trên máy trước, nếu user muốn quét cả trên NAS thì bấm nút."*

Số đo về sau xác nhận: quét ổ trong máy mất **13 giây**, quét NAS mất **4,5 phút**. Bắt trả 4,5
phút cho mọi lần quét, trong khi phần lớn tìm kiếm là tệp trên máy, là vô lý. Nên có **hai nút**:

| Nút | Phạm vi | Thời gian |
|---|---|---|
| **Quét lại** | chỉ ổ gắn trong máy | ~13 giây, hoặc **0,45 s** nếu journal trả lời được |
| **+ ổ mạng** | ổ trong máy **rồi mới** tới NAS | ~4,5 phút |

Nút thứ hai chỉ hiện khi máy thật sự có ổ mạng được gắn, và tooltip nêu đích danh từng ổ.

Thứ tự trong nút thứ hai là cố ý: làm phần nhanh trước, nên kết quả thường dùng đã có sẵn trước
khi phần chậm bắt đầu — và nếu người dùng bấm Dừng thì họ vẫn giữ được nó.

### Điều bất ngờ nhất nằm ở chỗ khác: tiến trình elevated không thấy ổ mạng

Đường tự nhiên nhất là nhét bộ quét NAS vào tiến trình `--index` sẵn có. Đem đo thì hỏng ngay:
ổ mạng gắn theo **phiên đăng nhập**, mà tiến trình elevated chạy dưới token khác — nó không thấy
F:, Y:, Z: nào cả, kể cả để mà báo là bỏ qua. Xem [CHECK-007](./check.md#check-007).

Nên bộ quét NAS chạy **trong chính tiến trình GUI**, quyền thường. Không mất gì: duyệt thư mục
không cần quyền nào.

Và nó kéo theo một ràng buộc suýt gây mất dữ liệu: tiến trình elevated dựng lại chỉ mục mà **không
có ổ mạng trong đó**. Nếu không xử lý, người dùng quét NAS 4,5 phút rồi bấm "Quét lại" là mất
sạch. Quy tắc đã chọn, viết mà không cần nhắc tới chữ "mạng":

> **Giữ nguyên mọi mục thuộc ổ đĩa không được quét trong lần chạy này.**
> Lần quét này không có thẩm quyền nói gì về chúng — dù đó là NAS, USB vừa rút, hay ổ mở hỏng.

### Đo được trên NAS thật

| Ổ | Thư mục | Tệp media | Thời gian |
|---|---|---|---|
| F: (`\\192.168.1.214\f`) | 3.832 | 144.417 | 11,6 s |
| Y: (`\\192.168.1.213\padoma 8`) | 7.581 | 150.575 | 237,3 s |
| Z: (`\\192.168.1.213\padoma 1`) | 958 | 18.953 | 23,8 s |
| **Tổng** | 12.371 | **313.945** | **272,6 s** |

**313.945 tệp media trên NAS so với 46.700 trên ổ trong máy** — gấp gần **7 lần**. Thư viện thật
của người dùng nằm chủ yếu ở đó, nên tính năng này không phải phần thêm cho đủ mà là phần lớn nhất
của thư viện.

Hai NAS chênh nhau rõ rệt: `.214` cho 331 thư mục/giây, `.213` chỉ 32–40. Cùng một đoạn mã, cùng
một mạng — khác nhau ở máy chủ.

### Ba chi tiết dễ sai

**Bỏ qua reparse point.** Đúng cái bẫy đã hạ gục công cụ kiểm chứng ở [CHECK-005](./check.md#check-005):
junction biến cây thành đồ thị, khiến vừa đếm trùng vừa chui vào những cây lẽ ra phải loại. Trên
NAS nó còn có thể tạo vòng lặp thật.

**Tệp qua SMB không có số hiệu (FRN).** Chúng được gán FRN 0, mà `index::update` vốn đã coi 0 là
"không có định danh" ([BUG-017](./bug.md#bug-017)) nên không bản ghi journal nào khớp được. Đó
chính là hành vi đúng: journal của ổ cục bộ không biết gì về tệp trên NAS, nên cập nhật nhanh phải
để nguyên phần NAS.

**Dung lượng đọc kèm ngay trong lúc duyệt.** Trên Windows, `DirEntry::metadata()` dùng lại dữ liệu
mà chính lần liệt kê thư mục đã trả về — không tốn thêm lời gọi hệ thống nào. Qua SMB thì khác biệt
đó là quyết định: đo riêng từng tệp sẽ nhân đôi số vòng round trip cho hơn ba trăm nghìn tệp.

### Một cái bẫy chỉ lộ ra sau khi mọi thứ đã chạy đúng

Quét xong, hợp nhất xong, tìm kiếm 2 ms. Nhưng ngay khi khởi động lại, tiến trình enrichment nền
lặng lẽ bắt đầu **mở từng tệp qua mạng** để đọc độ phân giải — 313.946 tệp ở tốc độ đo được là
**11 tệp/giây**, tức **7,8 giờ** hành NAS liên tục. Không lỗi, không cảnh báo.

Đã sửa: enrichment bỏ qua ổ mạng, và nói thẳng lý do trong log. Đánh đổi:

| Lọc theo | Tệp ổ cục bộ | Tệp NAS |
|---|---|---|
| Dung lượng | ✅ | ✅ (lấy miễn phí lúc duyệt) |
| Loại (video/ảnh/nhạc) | ✅ | ✅ (từ phần mở rộng) |
| Độ phân giải, thời lượng | ✅ | ❌ phải mở từng tệp qua mạng |

### Còn lại

Phần NAS **không** được cập nhật gia tăng — không có journal để theo. Muốn cập nhật thì bấm lại
nút "+ ổ mạng". Với thư viện mà phần lớn là tư liệu đã hoàn thành thì đó là đánh đổi hợp lý; nếu
sau này thấy phiền, `ReadDirectoryChangesW` **có thể** hoạt động qua SMB khi máy chủ hỗ trợ
change notify — cần thử thật trên chính hai NAS này, không được tin tài liệu.

## ISSUE-004 🟡 — Tệp vừa tải về không tìm thấy cho tới lần quét kế tiếp

**Giai đoạn:** BT · **Trạng thái:** ĐÃ ĐO, CHƯA SỬA · **Ngày:** 2026-08-25

**Người dùng hỏi:** vừa tải video về ổ, Quit hẳn ứng dụng rồi mở lại thì có quét lại không.

**Đã đo:** không. Mở lại ứng dụng **không** kích hoạt quét — xem [test-log](./test-log.md). Chỉ mục
mới chỉ đến từ ba nguồn: tác vụ lúc **đăng nhập**, tác vụ **hằng ngày 13:00**, và nút **"Quét lại"**.

**Khoảng hở thật.** Tải một video lúc 14:00 thì tìm không ra cho tới 13:00 hôm sau, trừ khi tự bấm
"Quét lại". Với người dùng tải footage về liên tục trong ngày, đây là khoảng hở đáng kể.

**Vì sao chưa sửa bằng cách quét lúc khởi động.** Đánh đổi sai chiều: mở ứng dụng phải gõ được ngay
(nạp cache ~0,13 s), quét lúc mở sẽ giết đúng tính chất đó. Và nó cũng **không giải quyết được vấn
đề**: ứng dụng chạy nền ở khay hệ thống suốt ngày, hiếm khi bị mở lại — nên "quét lúc mở" gần như
không bao giờ chạy đúng lúc cần.

**Hướng sửa đúng: [P9](../PROGRESS.md) — theo dõi USN realtime.** Journal đã có sẵn cursor cho mỗi
ổ; việc còn lại là đọc liên tục thay vì đọc theo lịch. Khi đó tệp vừa tải về xuất hiện trong vài
giây, không cần lịch cũng không cần bấm nút.

**Cách xoay xở hiện tại:** bấm **"Quét lại"** — cập nhật nhanh qua journal mất khoảng 0,45 giây,
không phải quét lại toàn bộ.
