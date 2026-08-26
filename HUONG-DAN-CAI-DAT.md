# MediaFinder — Hướng dẫn cài đặt

Dành cho người dùng. Đọc hết mất khoảng ba phút, cài đặt mất khoảng hai phút.

MediaFinder tìm video, ảnh, nhạc trong máy bạn **tức thì** — gõ tới đâu ra kết quả tới đó, không
phải chờ. Nó làm được vậy vì đọc thẳng bảng tệp của ổ đĩa thay vì mở từng thư mục ra xem.

---

## Máy cần gì

| | |
|---|---|
| Hệ điều hành | Windows 10 hoặc Windows 11, bản 64-bit |
| Ổ đĩa | Định dạng **NTFS** (mặc định của Windows — gần như chắc chắn máy bạn đã đúng) |
| Quyền | Tài khoản của bạn phải là **Administrator của máy** |

> **Nếu máy bạn do công ty quản lý và bạn không có quyền Administrator**, MediaFinder sẽ không dùng
> được. Đọc bảng tệp của ổ đĩa bắt buộc phải có quyền đó — đây là quy định của Windows, không phải
> lựa chọn của phần mềm.

Không cần cài thêm gì khác. Bộ cài đã mang sẵn mọi thứ, kể cả phần chạy giao diện, nên máy vừa cài
lại Windows và **không có mạng** vẫn cài được.

---

## Bước 1 — Tải bộ cài

Vào trang phát hành:

**https://github.com/khanhhoang050120-lang/mediafinder/releases/latest**

Trong mục **Assets**, bấm vào tệp có tên dạng **`MediaFinder_<phiên bản>_x64-setup.exe`**
để tải về. Tệp khoảng 200 MB nên tuỳ mạng có thể mất vài phút.

Trang này luôn là bản mới nhất. Muốn cập nhật về sau, quay lại đúng địa chỉ trên,
tải bản mới rồi cài đè lên — không cần gỡ bản cũ, chỉ mục đã quét vẫn giữ nguyên.

## Bước 2 — Chạy bộ cài

Mở tệp vừa tải về.

## Bước 3 — Vượt qua cảnh báo của Windows

Windows sẽ hiện một màn hình xanh:

> **Windows protected your PC**
> Microsoft Defender SmartScreen prevented an unrecognised app from starting.

**Đây là chuyện bình thường và không có nghĩa là phần mềm có hại.** Windows hiện màn này với mọi
phần mềm chưa mua chứng chỉ ký số — một khoản phí thường niên mà phần mềm nội bộ như thế này thường
không mua.

Cách qua:

1. Bấm dòng chữ nhỏ **`More info`** (Thông tin thêm)
2. Bấm nút **`Run anyway`** (Vẫn chạy) vừa hiện ra

Nếu muốn tự kiểm tra trước cho yên tâm: chuột phải vào tệp cài → **Scan with Microsoft Defender**.

## Bước 4 — Cài

Bộ cài chạy thẳng, không hỏi gì. Nó cài vào thư mục cá nhân của bạn nên **không hỏi quyền
Administrator** ở bước này.

Xong thì MediaFinder tự mở.

---

## Lần chạy đầu — quét ổ đĩa

Lần đầu mở, cửa sổ sẽ hiện:

> **Chưa có chỉ mục — cần quét ổ đĩa một lần**

Bấm **`Quét lần đầu`**.

**Windows sẽ hỏi quyền Administrator.** Bấm **Yes**. Đây là **lần duy nhất** bạn phải bấm — sau lần
này phần mềm tự lo, không hỏi lại nữa.

Quá trình quét mất khoảng **nửa phút tới vài phút**, tuỳ số tệp trong máy. Có thanh tiến độ.

Xong là dùng được ngay.

### Nếu bạn có ổ mạng / NAS

Ở màn hình đầu còn một nút nữa: **`Quét cả ổ mạng`**, kèm chữ cái các ổ mạng máy bạn đang có.

Quét ổ mạng **lâu hơn nhiều lần** (có thể vài phút tới hàng chục phút, tuỳ dung lượng và tốc độ
mạng). Bỏ qua lúc đầu cũng được — nút **`+ ổ mạng`** ở thanh trên làm đúng việc đó bất cứ lúc nào.

---

## Dùng hằng ngày

| Việc | Cách làm |
|---|---|
| Gọi cửa sổ tìm kiếm từ bất kỳ đâu | **`Ctrl` + `Alt` + `Space`** |
| Tìm | Gõ thẳng. Không dấu vẫn ra có dấu: gõ `tieng viet` ra `Tiếng Việt.mp4` |
| Xem trước ngay trong ứng dụng | **Nháy đúp** vào kết quả, hoặc `Shift`+`Enter` |
| Mở tệp bằng ứng dụng mặc định | `Enter` |
| Mở thư mục chứa tệp | `Ctrl`+`Enter`, hoặc chuột phải → *Mở thư mục chứa tệp* |
| Chọn nhiều tệp | `Ctrl`+click từng cái, hoặc `Shift`+click chọn cả dải |
| Kéo tệp vào CapCut / ô upload | Kéo thẳng từ kết quả ra, như kéo từ File Explorer |
| Đóng cửa sổ | Bấm `X` — **chỉ ẩn đi**, phím tắt vẫn dùng được |
| Tắt hẳn | Chuột phải biểu tượng ở khay hệ thống → **Thoát** |

### Vì sao bấm X lại không tắt hẳn

Phím tắt `Ctrl`+`Alt`+`Space` chỉ hoạt động khi phần mềm đang chạy. Nếu bấm X mà tắt hẳn thì lần
sau bạn sẽ phải mở lại bằng tay, và phím tắt thành vô dụng. Nên X chỉ ẩn cửa sổ đi; biểu tượng ở
khay hệ thống (góc phải dưới, cạnh đồng hồ) là dấu hiệu nó vẫn ở đó.

### Chỉ mục tự cập nhật khi nào

- Mỗi lần bạn **đăng nhập Windows**
- **Mỗi ngày một lần** lúc 13:00
- Bất cứ lúc nào bạn bấm nút **`Quét lại`** — mất chưa tới một giây

**Tệp vừa tải về mà chưa thấy?** Bấm `Quét lại`. Chỉ mục chỉ tự cập nhật theo lịch ở trên, nên tệp
tải lúc 14:00 sẽ không tự xuất hiện cho tới hôm sau nếu bạn không bấm.

Riêng **ổ mạng không nằm trong lịch tự động** — phải bấm `+ ổ mạng` khi cần.

---

## Gặp vấn đề

**Gõ mà không ra gì, dù chắc chắn có tệp đó.**
Nhìn dòng chữ nhỏ dưới đáy cửa sổ. Nếu ghi *"chưa có cache"* thì máy chưa quét — bấm `Quét lại`.
Nếu tệp nằm trên ổ mạng thì bấm `+ ổ mạng`.

**Phím tắt không gọi được cửa sổ.**
Một phần mềm khác đang chiếm tổ hợp đó. Cửa sổ MediaFinder sẽ nói rõ điều này ở màn hình trống. Mở
MediaFinder bằng lối tắt ở Start Menu, và tắt phần mềm kia nếu muốn dùng phím tắt.

**Ổ mạng không thấy trong kết quả dù đã bấm `+ ổ mạng`.**
Kiểm tra ổ mạng có đang kết nối không (mở File Explorer, xem chữ cái ổ có dấu X đỏ không). Ổ mạng
mất kết nối thì MediaFinder không đọc được và sẽ nói rõ lý do.

**Máy có ổ định dạng khác NTFS (thẻ nhớ, USB định dạng FAT32/exFAT).**
Những ổ đó bị bỏ qua, và phần mềm nói rõ ổ nào bị bỏ qua vì lý do gì. Đây là giới hạn thật: cách
đọc nhanh mà MediaFinder dùng chỉ có trên NTFS.

**Mở lên hiện hộp thoại "MediaFinder chưa chạy được".**
Máy thiếu WebView2 Runtime — hầu như luôn là do **chép tệp chương trình từ máy khác sang** thay vì
chạy bộ cài. Chạy bộ cài `-setup.exe`; nó mang sẵn thành phần đó bên trong, không cần mạng.

**Muốn tắt phần tự khởi động cùng Windows.**
Mở PowerShell rồi dán:

```powershell
Remove-Item (Join-Path ([Environment]::GetFolderPath('Startup')) 'MediaFinder.lnk')
```

Lưu ý: sau đó phím tắt chỉ dùng được khi bạn đã tự mở MediaFinder.

**Muốn tắt phần tự cập nhật chỉ mục.**
Mở PowerShell **bằng quyền Administrator** rồi dán:

```powershell
Unregister-ScheduledTask -TaskName 'MediaFinder - cap nhat chi muc' -Confirm:$false
```

---

## Gỡ cài đặt

Settings → Apps → Installed apps → **MediaFinder** → Uninstall.

Bộ gỡ cài đặt tự dọn luôn lối tắt tự khởi động và dữ liệu chỉ mục. Riêng tác vụ cập nhật định kỳ
cần quyền Administrator để xoá — nếu nó còn sót lại, dùng lệnh ở mục trên để xoá hẳn.

---

## Phần mềm này có gửi gì ra ngoài không

Không. MediaFinder **không kết nối mạng** để làm bất cứ việc gì: không gửi thống kê, không kiểm tra
cập nhật, không tải gì về. Chỉ mục nằm trong thư mục cá nhân của bạn và không rời khỏi máy.

Ổ mạng là ngoại lệ hiển nhiên duy nhất — và chỉ khi bạn tự bấm nút quét nó.

Hệ quả: phần mềm **không tự báo khi có bản mới**. Muốn biết, bạn tự vào trang phát hành ở
[Bước 1](#bước-1--tải-bộ-cài) xem — đây là cái giá của việc không cho phần mềm tự gọi ra ngoài.
