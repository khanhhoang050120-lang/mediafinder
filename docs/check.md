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

---

## CHECK-003 ⚪ — Bộ đọc journal chưa chạy trên ổ NTFS thật

**Giai đoạn:** P9 · **Trạng thái:** ĐÃ XÁC MINH · **Ngày:** 2026-08-24

**Tình trạng.** `ntfs/usn_journal.rs` có 17 test, trong đó có một test đi hết chặng: byte thô của
journal vào → `Change` → `rebuild_with` → index mới, kiểm tra đúng đường dẫn. Tất cả đều pass.

Nhưng **mọi test đều chạy trên bản ghi tôi tự dựng bằng tay**, theo layout `USN_RECORD_V2` đọc từ
tài liệu. Chúng chứng minh mã đọc đúng cái tôi *nghĩ* là đúng — không chứng minh Windows thật sự
sinh ra những bản ghi như thế.

Đây chính là điều đã sai ở [BUG-013](./bug.md#bug-013): mã đúng theo cách nhìn của nó, dữ liệu đúng
theo cách nhìn của nó, và cả hai không gặp nhau ở đâu cả.

**Vì sao chưa đo được.** `FSCTL_READ_USN_JOURNAL` cần quyền Administrator. Đã bật lời nhắc UAC để
chạy `--watch`, nhưng lời nhắc không được chấp nhận (`The operation was canceled by the user`) —
nhiều khả năng không có ai ở máy lúc đó.

**Cách đo, khi có người ở máy.**

```powershell
# terminal chạy với quyền Administrator
D:\tool_finding\src-tauri\target\debug\mediafinder.exe --watch C
```

Rồi tạo / đổi tên / xoá một tệp `.mp4`. Bốn thứ cần nhìn tận mắt:

1. Đổi tên có thật sự sinh ra **cặp** `RENAME_OLD_NAME` + `RENAME_NEW_NAME` không, và theo thứ tự nào.
2. Xoá có sinh `FILE_DELETE`, và bản ghi đó có tới **sau** mọi bản ghi khác của cùng tệp không —
   quy tắc "bản ghi cuối thắng" dựa hoàn toàn vào điều này.
3. `next_usn` trả về có thật sự tiến lên không. Vòng lặp đọc dừng dựa vào nó; nếu nó đứng yên mà
   vẫn trả về bản ghi thì vòng lặp sẽ quay mãi.
4. Ghi một tệp lớn sinh ra bao nhiêu bản ghi — quyết định có cần chặn trần kích thước lô hay không.

## Kết quả — đã chạy trên ổ thật

Không cần chạy `--watch` thủ công. Chỗ đo tốt hơn nằm ngay trong chính lần quét đầy đủ: con trỏ
journal được ghi lại **trước** khi một ổ được duyệt, mà duyệt hết mọi ổ mất hàng chục giây — nên
đến lúc quét xong, chắc chắn đã có hoạt động tệp thật để đọc. Thêm `check_journal_cursors()` chạy
ở cuối mỗi lần quét, trong đúng tiến trình vốn đã có quyền Administrator.

Chạy thật, ổ C:

```
ổ C: tự kiểm tra journal — 570 thay đổi kể từ usn=20623105360 (nay 20623167576) [1ms]
    C: CÓ MẶT  frn=19984723347872345 cha=1688849862689891 progress.json.tmp
ổ D: tự kiểm tra journal — 0 thay đổi kể từ usn=2088815312 (nay 2088815312) [0ms]
```

| Cần xác nhận | Kết quả |
|---|---|
| `FSCTL_READ_USN_JOURNAL` chạy được với con trỏ đã lưu | ✅ 570 bản ghi, **1 ms** |
| Bản ghi giải mã ra đúng hình dạng | ✅ FRN, FRN cha, tên tệp đều hợp lệ |
| Tên tệp đọc đúng | ✅ `progress.json.tmp` — chính là tệp indexer vừa ghi |
| `next_usn` tiến lên, vòng lặp dừng được | ✅ 20623105360 → 20623167576 rồi dừng |
| Ổ không có thay đổi thì trả về rỗng, không treo | ✅ ổ D: trả 0 thay đổi, `next_usn` giữ nguyên |
| `journal_id` khớp nên không bị hiểu nhầm là journal mới | ✅ không có cảnh báo `Restart` nào |

Ổ C: sinh ra 570 thay đổi trong 11 giây quét — đúng như dự đoán rằng một máy đang chạy thì luôn có
hoạt động tệp. Ổ D: yên tĩnh vì không ai đụng tới nó lúc đó.

**Còn chưa đo:** cặp `RENAME_OLD_NAME` + `RENAME_NEW_NAME` do Windows tự sinh, và số bản ghi khi
ghi một tệp lớn. Cả hai cần thao tác tệp có chủ đích trong lúc `--watch` đang chạy. Phần đọc,
giải mã và con trỏ thì đã chắc chắn.

---

## CHECK-004 ✅ — Đọc USN journal có thật sự cần quyền Administrator không?

**Giai đoạn:** P9 · **Trạng thái:** KHÔNG PHẢI LỖI (đã có câu trả lời dứt khoát) · **Ngày:** 2026-08-24

**Nghi ngờ.** Toàn bộ kế hoạch Windows Service dựa trên một câu khẳng định: *"`FSCTL_READ_USN_JOURNAL`
cần quyền Administrator"*. Câu đó được suy ra từ việc `open_volume` xin `FILE_GENERIC_READ` và bị
từ chối khi không elevate — **không phải từ việc thử các mức quyền thấp hơn**.

Nếu một handle mở với quyền thấp mà vẫn gọi được FSCTL, thì GUI theo dõi journal được mà không
elevate lần nào, và **cả dự án con Windows Service trở thành thừa**. Câu hỏi này đáng vài phút.

**Cách đo.** Tiến trình **không** elevated, mở `\.\C:` với bốn mức quyền rồi gọi
`FSCTL_QUERY_USN_JOURNAL` trên handle nhận được:

| Quyền xin | Mở volume | `FSCTL_QUERY_USN_JOURNAL` |
|---|---|---|
| `0` (không xin gì) | **OK** | lỗi 1 — `ERROR_INVALID_FUNCTION` |
| `FILE_READ_ATTRIBUTES` (0x80) | **OK** | lỗi 1 — `ERROR_INVALID_FUNCTION` |
| `FILE_READ_DATA` (0x01) | lỗi 5 — `ACCESS_DENIED` | — |
| `GENERIC_READ` | lỗi 5 — `ACCESS_DENIED` | — |

**Kết luận.** Mở volume thì **không** cần quyền — điều này trước đây tôi cũng tưởng là cần. Nhưng
FSCTL của journal bị từ chối trên handle quyền thấp, và mức quyền duy nhất cho phép nó
(`FILE_READ_DATA` trên volume) thì đòi Administrator.

Lỗi `ERROR_INVALID_FUNCTION` ở đây **không** có nghĩa là hàm không tồn tại — nó là cách hệ điều
hành nói "handle này không mang đủ quyền cho thao tác đó". Nếu đọc mã lỗi theo nghĩa đen thì sẽ đi
tìm sai hướng hoàn toàn.

Vậy khẳng định ban đầu đúng, nhưng giờ nó đúng **vì đã đo**, không phải vì suy đoán. Bất biến
["GUI không bao giờ chạy elevated"](../README.md#bất-biến-kiến-trúc) và "cập nhật realtime" thật sự
loại trừ nhau, trừ khi có một tiến trình phụ có quyền.

**Giá trị của việc đo.** Nếu kết quả ngược lại, nó đã xoá bỏ được toàn bộ hạng mục lớn nhất còn lại
của dự án. Bốn phút để biết chắc thay vì xây một Windows Service dựa trên một câu chưa ai kiểm tra.


---

## CHECK-005 ✅ — Số tệp tụt từ 117.128 xuống 46.700: hồi quy hay là thật?

**Giai đoạn:** P9 · **Trạng thái:** KHÔNG PHẢI LỖI · **Ngày:** 2026-08-24

**Nghi ngờ.** Sau khi thêm FRN và nâng schema, lần quét đầu tiên cho **46.700** tệp media, trong
khi mọi số liệu ghi từ P6 tới P8 đều là **117.128**. Giảm 60%. Nếu là hồi quy do thay đổi của tôi
thì đây là loại hỏng tệ nhất: chỉ mục thiếu mất hơn một nửa thư viện mà không có lỗi nào.

**Bước 1 — loại trừ khả năng tự gây ra.** `git diff` phần quét so với trước P9: `tree.rs` chỉ thêm
trường FRN, `usn_enum.rs` không đổi một dòng, `DEFAULT_EXCLUDED` và `ResolveOptions::default()`
nguyên vẹn. Luật quét không hề thay đổi.

**Bước 2 — đếm độc lập.** Viết một bộ đếm riêng bằng PowerShell dùng **đúng** 71 đuôi tệp lấy
thẳng từ `model.rs` và **đúng** danh sách loại trừ, rồi so:

| Ổ | Bộ quét MFT | Đếm độc lập | Chênh |
|---|---|---|---|
| D: | 46.117 | **46.116** | 1 |
| C: | 583 | **521** | 62 |

Chênh 1 tệp ở D: là bình thường — máy vẫn đang chạy giữa hai phép đo. Chênh 62 ở C: cũng có lời
giải: bộ quét chạy **elevated** nên đọc được thư mục hệ thống, còn phép đếm chạy quyền thường và
báo đúng **62 thư mục không đọc được**.

**Bẫy trong chính phép kiểm chứng.** Lần đếm C: **đầu tiên** ra 1.635 — nhiều gấp ba lần bộ quét,
tức là ngược hướng với mọi giả thuyết. Nguyên nhân nằm ở phép đếm chứ không ở bộ quét:
`Directory.GetDirectories` **đi xuyên qua junction**. `C:{bs}Users{bs}All Users` trỏ tới
`C:{bs}ProgramData`, `C:{bs}Documents and Settings` trỏ tới `C:{bs}Users` — nên phép đếm chui vào
đúng những cây đã bị loại, bằng một đường dẫn không chứa tên bị cấm nào.

Bộ quét MFT không bao giờ mắc lỗi này: nó đọc **chuỗi cha thật** từ bảng MFT, nơi không tồn tại
đường vòng. Bỏ qua reparse point thì con số rơi từ 1.635 xuống 521, và số lỗi cũng về 0 — vì không
còn chui vào thư mục hệ thống nữa.

**Kết luận.** Không có hồi quy. 46.700 là số thật của máy tại thời điểm này; 117.128 phản ánh một
trạng thái đĩa đã không còn. Tổng dung lượng cũng đi cùng chiều: 3.014,6 GB → **2.749,5 GB**, tức
70.428 tệp biến mất chỉ mang theo 265 GB — trung bình 3,8 MB mỗi tệp, đúng cỡ của tệp vật liệu dự
án chứ không phải video. Cache cũ đã bị ghi đè nên không dựng lại được trạng thái trước để so.

**Bài học cho bộ quét ổ mạng sắp tới.** Một bộ duyệt thư mục **bắt buộc** phải bỏ qua reparse
point, nếu không nó vừa đếm trùng vừa chui vào những cây lẽ ra phải loại — và trên NAS thì junction
còn có thể tạo thành vòng lặp thật sự. Đây chính là thứ khiến duyệt thư mục khó đúng hơn đọc MFT,
và nó vừa tự chứng minh ngay trong công cụ kiểm chứng của tôi.

---

## CHECK-006 ✅ — 70.000 tệp biến mất: MediaFinder có xoá gì không?

**Giai đoạn:** P9 · **Trạng thái:** KHÔNG PHẢI LỖI · **Ngày:** 2026-08-24

**Câu hỏi.** Người dùng hỏi thẳng: chênh lệch 117.128 → 46.700 có phải là mất dữ liệu không, và
phần mềm có xoá gì không. Với câu hỏi này thì trấn an là vô trách nhiệm — phải có bằng chứng.

**Bước 1 — MediaFinder có đường mã nào xoá tệp không.** `grep` toàn bộ mã nguồn tìm `remove_file`,
`remove_dir`, `DeleteFile`, `SHFileOperation`, `IFileOperation`, `File::create`, `OpenOptions`:

| Nơi | Đối tượng |
|---|---|
| `ipc/elevate.rs` | `progress.json` — tệp của chính MediaFinder |
| `index/persist.rs` | `index.bin` — cache |
| `media/enrich.rs` | `metadata.bin` — kho metadata |
| `media/dupes.rs` | trong `mod tests`, tạo tệp tạm để test |

**Không có một lệnh nào chạm tới tệp của người dùng.** Tính năng tìm trùng lặp chỉ đọc 64 KB đầu
và cuối để so vân tay; không có nút xoá.

**Bước 2 — hỏi USN journal.** Chỉ mục nói cái gì *đang* có, không bao giờ nói cái gì *đã từng* có.
Nguồn duy nhất trả lời được là journal — nó ghi từng lần xoá kèm tên và thời điểm. Viết chế độ
`--audit`, chỉ đọc journal và không ghi gì.

Kết quả trên ổ D:

```
70.461 tệp media và 4.430 thư mục đã bị xoá
    2026-08-24 08h (UTC): 70.460 tệp
    .mp3: 67.305 · .mp4: 2.793 · .mov: 187 · .png: 151
thư mục GỐC bị xoá (thư mục cha vẫn còn):
    D:\Sounds Edit\HƯNG
```

**Đúng một thư mục bị xoá**, trong đúng một giờ. 966 thư mục con mất tệp, tất cả đều nằm dưới nó.
Thời điểm 08h UTC = **15h giờ địa phương**, và lượt quét trùng lặp của MediaFinder chạy 09h11–09h20
UTC — **sau** khi việc xoá đã xong.

**Bước 3 — có phải chuyển chỗ không?** Di chuyển trong cùng một volume sinh bản ghi `RENAME`, không
sinh `FILE_DELETE`. Journal ghi `FILE_DELETE`, nên thư mục không bị chuyển đi đâu trong D:. Tìm
trên cả ba ổ NAS tới độ sâu 3 cấp: không có thư mục nào tên `HƯNG`.

**Bước 4 — còn khôi phục được không?** `D:\$RECYCLE.BIN` giữ 43.011 tệp nhưng chỉ 1.116 là media
(16,6 GB). Một cây thư mục 265 GB thì vượt hạn ngạch thùng rác, nên Windows xoá thẳng.

**Kết luận.** Không phải hồi quy, không phải lỗi phần mềm, và MediaFinder không xoá gì. Đây là một
thao tác xoá thư mục xảy ra lúc 15h ngày 24/8, trước cả lượt quét trùng lặp.

**Người dùng xác nhận (2026-08-24):** *"tôi đã xóa folder HƯNG lúc chiều"*. Khớp chính xác với
thời điểm 15h giờ địa phương mà journal ghi lại. Không có dữ liệu nào mất ngoài dự tính, và
46.700 là con số đúng của máy.

**Vì sao đáng giữ lại chế độ `--audit`.** Nó dựng lại được đường dẫn đầy đủ của những thư mục
**đã bị xoá** — bằng cách đi ngược chuỗi cha qua chính các bản ghi xoá, cho tới khi gặp một thư mục
còn tồn tại trong index. Không có nó thì câu trả lời dừng ở "70.000 tệp mất ở đâu đó"; có nó thì
câu trả lời là một đường dẫn cụ thể và một mốc giờ.

---

## CHECK-007 ✅ — Tiến trình elevated có nhìn thấy ổ mạng không?

**Giai đoạn:** P10 · **Trạng thái:** KHÔNG PHẢI LỖI (một sự thật về Windows) · **Ngày:** 2026-08-25

**Nghi ngờ.** Trước khi thiết kế phần quét NAS: đặt nó vào đâu? Đường tự nhiên nhất là nhét vào
tiến trình `--index` sẵn có — nó đã quét ổ đĩa rồi. Nhưng tiến trình đó chạy elevated.

**Cách đo.** Cùng một hàm `list_volumes()`, chạy ở hai mức quyền:

| | Ổ nhìn thấy |
|---|---|
| Elevated (`--index --full`) | C:, D:, G: |
| Quyền thường | C:, D:, **F:**, G:, **Y:**, **Z:** |

Ba ổ mạng **không xuất hiện** trong lần chạy elevated — không phải bị bỏ qua, mà là không tồn tại
đối với nó. Cả dòng cảnh báo "bỏ qua ổ mạng" cũng không có gì để in.

**Nguyên nhân.** Ổ mạng được gắn theo **phiên đăng nhập**, không phải theo máy. Tiến trình elevated
chạy dưới một token khác, nên nó không thừa hưởng các ánh xạ ổ đĩa của người dùng. Đây là hành vi
cố ý của Windows, không phải lỗi cấu hình.

**Hệ quả với kiến trúc, và nó lớn.**

1. Tiến trình quét MFT **vĩnh viễn** không quét được NAS, dù có sửa gì đi nữa. Không phải vì SMB
   không có MFT (điều đó cũng đúng, xem [ISSUE-003](./issue.md#issue-003)) mà vì nó còn không thấy
   ổ đĩa để mà thử.
2. Ngược lại, việc đó **không sao cả**: duyệt thư mục qua SMB không cần quyền gì. Nên bộ quét NAS
   thuộc về **tiến trình GUI**, nơi vốn chạy quyền thường và nhìn đủ ổ.
3. Và đây là điều dễ gây mất dữ liệu nhất: **một lần quét ổ cục bộ không được phép xoá phần NAS
   trong chỉ mục.** Tiến trình elevated không thấy ổ Z: nên sẽ dựng lại chỉ mục không có nó — nếu
   không giữ lại, người dùng quét NAS 20 phút rồi bấm "Quét lại" là mất sạch.

Quy tắc đã chọn: **giữ nguyên mọi mục thuộc ổ đĩa không được quét trong lần chạy này.** Không cần
biết ổ đó là mạng hay là USB vừa rút ra — nếu không quét thì không có thẩm quyền nói gì về nó.

**Nếu đã tin vào linh cảm thay vì đo.** Tôi đã định đặt bộ quét NAS vào tiến trình `--index`. Nó sẽ
chạy, không báo lỗi, và im lặng không tìm thấy ổ mạng nào — đúng loại hỏng mà dự án này đã gặp
nhiều lần.

## CHECK-008 ✅ — Bản tôi đang đo có đúng là bản tôi vừa dựng không?

**Giai đoạn:** P14 · **Ngày:** 2026-08-25 · **Kết luận:** KHÔNG — và nó đã đẻ ra một lý thuyết sai

**Vì sao phải kiểm.** Sau khi sửa [BUG-021](./bug.md#bug-021) lần hai, phép đo vẫn báo "2/5 lần vẫn
bung". Tôi kết luận rằng chặn theo nguyên nhân là bất khả, chuyển sang chặn ở kết quả, và **ghi lời
giải thích đó vào mã lẫn tài liệu**.

**Nghi ngờ đến từ một con số lẻ.** Đối chiếu dấu thời gian:

```
installer vừa dựng : 17:52:22
exe đang cài       : 17:36:40
```

Lệch **16 phút**. Bản tôi đo không phải bản tôi vừa sửa.

**Nguyên nhân.** Vòng chờ của tôi là `until ! tasklist cargo`. Nhưng `tauri build` chạy **`vite`
trước**, nên tại thời điểm bắt đầu chờ thì `cargo` còn chưa khởi động — điều kiện đúng ngay lập
tức, vòng chờ thoát, và lệnh cài chạy trên bộ cài của lần dựng **trước**.

**Đã đo lại cho đúng.** Đối chiếu **mã băm SHA-256** thay vì dấu thời gian, và chạy thẳng exe vừa
dựng thay vì qua bộ cài:

| Bản chạy | Xác nhận | Kết quả 5 lần |
|---|---|---|
| Có cả hai lớp chặn | băm khớp | **5/5 sạch** |
| **Tắt riêng lớp chặn kết quả** | băm `8EE678CC…` khớp bản dựng | **5/5 sạch** |

Lần đo thứ hai mới là lần trả lời được câu hỏi: lớp chặn ở tầng cửa sổ **tự nó đã đủ**. Lớp chặn
kết quả đã bị gỡ, và lời giải thích sai trong mã đã được viết lại.

**Bài học.** Một phép đo trên nhị phân sai còn tệ hơn không đo. Nó không chỉ bỏ sót lỗi — nó **dựng
lên một lý thuyết nhân quả sai**, rồi dẫn tới một cách sửa nhân danh lý thuyết ấy, và cách sửa đó
được ghi vào tài liệu như tri thức. Ba thứ sai chồng lên nhau, tất cả từ một vòng `until` sai.

**Đã thành quy trình:** [`cai-dat.sh`](../src-tauri/) trong thư mục làm việc dừng hẳn nếu bản đang
chạy không khớp bản vừa dựng; và với các phép đo quan trọng thì chạy thẳng
`target/release/mediafinder.exe`, bỏ bộ cài ra khỏi vòng kiểm chứng.

### Hai cái bẫy nữa, tìm ra khi đang sửa chính quy trình này

**Bẫy 1 — đang chạy exe thì không dựng đè lên nó được, và bản dựng *không* báo lỗi.**
Tôi chạy thẳng `target/release/mediafinder.exe` để bỏ bộ cài ra khỏi vòng kiểm chứng. Nhưng khi
tiến trình đó còn sống, trình liên kết không ghi đè được tệp — `npm run tauri build` **thoát với mã
0** và exe vẫn nguyên bản cũ. Hai lần dựng liên tiếp cho ra **đúng cùng một mã băm**, kể cả sau khi
`touch` vào `lib.rs` để ép liên kết lại. Từ đó: **tắt ứng dụng trước khi dựng**, và luôn kiểm mã
băm có đổi không.

**Bẫy 2 — mã băm giữa bản dựng và bản cài *luôn* khác nhau một cách chính đáng.**
Sau khi đóng gói, Tauri "vá thông tin loại gói" vào exe trong `target/release`, nên bản trên đĩa và
bản nằm trong bộ cài lệch nhau vài byte:

```
10.654.208 byte  18:29:31  target/release/mediafinder.exe   (đã vá)
10.654.208 byte  18:29:28  ...\MediaFinder\mediafinder.exe  (bản đóng gói)
```

Phép kiểm bằng mã băm vì thế báo "KHÁC NHAU" cho một lần cài hoàn toàn đúng. Tiêu chí đúng là
**cùng kích thước và lệch thời gian dưới một phút**.

**Kết quả cuối, trên bản đã xác minh:** 5/5 lần mở đều giữ cửa sổ 880×620, và video 1080p nằm gọn
trong khung.

## CHECK-009 ✅ — Phần mềm này thật sự cần cài thêm thư viện nào?

**Giai đoạn:** P15 · **Ngày:** 2026-08-25 · **Kết luận:** đúng **một** thứ, và bộ cài đã mang sẵn

**Vì sao phải kiểm.** Người dùng muốn bộ cài "quét xem máy có thư viện chưa, thiếu thì cài, cũ thì
nâng lên". Xây một cơ chế như vậy mà không biết **danh sách thư viện thật** thì sẽ đi cài thứ không
cần và bỏ sót thứ cần.

**Cách đo.** Đọc thẳng **bảng nhập PE** của tệp exe đã dựng — không phải đoán từ danh sách phụ
thuộc trong `Cargo.toml`. Ra 22 DLL:

| Nhóm | Kết luận |
|---|---|
| `api-ms-win-crt-*` (8 tệp) | **UCRT — nằm sẵn trong Windows 10 và 11**, không phải thứ phải cài |
| `kernel32` `user32` `shell32` `ole32` `gdi32` `comctl32` `dwmapi` `mpr` `ntdll` `propsys` `shlwapi` `advapi32` `oleaut32` `bcryptprimitives` | Thành phần của Windows |
| `vcruntime140.dll` | **KHÔNG có mặt** — Rust đã liên kết tĩnh phần đó vào exe |

**Kết quả đáng chú ý nhất là thứ *không* có:** `vcruntime140.dll`. Nếu nó có mặt thì mỗi máy đều
cần gói Visual C++ Redistributable, và máy thiếu nó sẽ báo *"vcruntime140.dll not found"* — đúng
kiểu lỗi khó hiểu mà người dùng không tự xử lý được. Đã kiểm và không cần lo.

**Thứ duy nhất ở ngoài: WebView2 Runtime.** Nó **không nằm trong bảng nhập** vì được nạp lúc chạy,
nên chỉ phát hiện được qua registry. Máy này: `151.0.4129.107`, cài ở cấp máy (`HKLM`).

**Vế "máy có bản cũ thì nâng lên" tự xảy ra.** WebView2 là loại *evergreen* — Microsoft tự cập
nhật nó qua trình cập nhật của Edge. Bộ cài chỉ cần lo vế "máy chưa có", và đã lo bằng cách nhúng
hẳn bộ cài WebView2 vào trong (208 MB).

**Vẫn cần một lớp nữa: nói cho người dùng biết khi thiếu.** Nếu WebView2 vắng mặt vì lý do nào đó
— thường gặp nhất là **chép tệp exe từ máy khác sang thay vì chạy bộ cài** — thì thứ người dùng
thấy là cửa sổ trắng, không câu nào giải thích. Nên nay có bước kiểm tra chạy **trước khi mở cửa
sổ**, hiện một hộp thoại nói đúng một việc họ cần làm.

### Bẫy thứ ba của cùng một họ với [CHECK-008](#check-008)

**Đang chạy ứng dụng thì bộ cài không ghi đè được — mà vẫn thoát với mã 0.** `Stop-Process` trả về
ngay lập tức trong khi tiến trình còn đang thoát và **vẫn giữ tệp exe**. Bộ cài NSIS gặp tệp bị giữ
thì bỏ qua, báo thành công, và mọi phép đo sau đó đo nhầm bản cũ.

Đã dính bẫy này **hai lần liên tiếp** trong cùng một buổi, và cả hai lần đều do chính công cụ chụp
hộp thoại khởi động ứng dụng lên rồi để đó. Công cụ cài đặt nay **chờ tiến trình thật sự biến mất**
khỏi danh sách tiến trình, chứ không chỉ gọi lệnh tắt rồi đi tiếp — và nếu lần cài đầu không ghi đè
được thì tự thử lại một lần.
