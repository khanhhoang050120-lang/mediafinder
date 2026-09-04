# Quét trùng lặp: vì sao lượt đầu lâu, và làm gì để nhanh hơn

> Phân tích trên dòng mã `origin/version2` (v1.0.8). Tham chiếu tệp:dòng theo cây đó.
> Mọi nhận định đã qua một lượt phản biện độc lập (4 agent, 172 lượt đọc mã); chỗ nào bị bác
> hoặc hạ mức tin cậy đều ghi lại. **Chưa sửa gì.** Đây là tài liệu để đọc và quyết.

## Tóm tắt

Lượt quét đầu lâu vì phần việc nặng nhất là **bản chất của thư viện**: cứ 10 tệp thì 6 tệp có bản
sao cùng dung lượng (CapCut nhân bản asset vào từng draft), và để nói "đây là bản sao" thì phải mở
từng tệp ra. Mã hiện tại đọc rất ít mỗi tệp (128 KB) nhưng trả giá **theo số tệp phải mở và số lần
đọc mỗi tệp**, chứ không theo byte: trên NAS đã đo, mở và lấy byte đầu tiên mất 66 ms trong khi
đọc thêm 1 MB chỉ mất 18 ms. Ngoài ra nó không nhớ gì giữa các lần, không cho xem gì cho tới khi
xong hẳn, và chiếm luôn pool luồng của tìm kiếm.

**Không có số đo nào cho mã hiện tại trên thư viện hiện tại.** Con số 584 giây trong PROGRESS.md
đo ngày 24/8 trên ổ cục bộ D:, bằng mã cũ chia việc theo dung lượng, trên một chỉ mục còn chứa
70.461 tệp vừa bị xoá. Ngoại suy từ phép đo NAS (85–100 ms mỗi tệp mỗi luồng) với ~240.000 ứng
viên hôm nay thì lượt đầu vào khoảng **30 phút**, nhưng đó là ước lượng.

Có tám việc làm được, xếp theo lợi ích trên công sức. Ba việc rẻ (hiện kết quả dần, giữ kết quả
sống sót qua nạp lại chỉ mục, không nói dối khi ổ rớt) đổi trải nghiệm ngay. Hai việc trung bình
(pool I/O theo ổ, vân tay bền) mới rút thời gian thật, và phải đo trước khi tin. Kèm theo là bốn
lỗi tìm thấy khi đọc mã, trong đó một lỗi đúng/sai đáng sửa trước mọi thứ.

## 1. Cơ chế hiện tại

| Tầng | Kiểm tra | Đọc đĩa | Ở đâu |
|---|---|---|---|
| 1 | Cùng dung lượng, bỏ tệp dưới 64 KB | **không đọc gì**, chỉ mục đã có `sizes()` | [dupes.rs:197-203](src-tauri/src/media/dupes.rs#L197-L203) |
| 2 | BLAKE3 của (dung lượng + 64 KB đầu + 64 KB cuối) | 128 KB mỗi tệp, một lần mở, hai lần đọc | [dupes.rs:224-234](src-tauri/src/media/dupes.rs#L224-L234), [fingerprint()](src-tauri/src/media/dupes.rs#L276-L298) |
| 3 | Hash toàn tệp | toàn bộ tệp | [full_hash()](src-tauri/src/media/dupes.rs#L304-L316) |

Một điều tài liệu chưa nói: trên v1.0.8, **tầng 3 là mã chết**. `full_hash` không có nơi nào gọi,
không có lệnh IPC, không có nút. Tầng 3 theo nhóm (`verify.rs`, hàm `verify_paths`) chỉ có ở dòng
`edit` cũ. Mô tả "ba tầng" ở đầu `dupes.rs` là của dòng cũ. Hệ quả thực tế: hôm nay không có cách
nào trong app để chắc một nhóm là trùng thật trước khi xoá.

Kết quả chỉ sống trong RAM của tiến trình ([DupeService.result](src-tauri/src/media/dupes.rs#L96-L97)),
chỉ được ghi một lần khi quét xong ([dupes.rs:171-173](src-tauri/src/media/dupes.rs#L171-L173)),
và huỷ giữa chừng thì vứt toàn bộ ([dupes.rs:236-238](src-tauri/src/media/dupes.rs#L236-L238)).

## 2. Số đo có gì, và không có gì

| Số | Nguồn | Điều kiện đo | Dùng được để nói gì |
|---|---|---|---|
| 70.576 / 117.128 tệp là ứng viên sau tầng 1 (60 %) | PROGRESS.md P7 | 24/8, chỉ mục còn chứa 70.461 tệp đã xoá lúc 08:00 UTC (docs/check.md CHECK-002) | tỷ lệ ứng viên; **không** nói được bao nhiêu là bản sao thật vì tệp đã mất thì mở hỏng và bị bỏ lặng |
| 584 giây, "~9 GB" | PROGRESS.md P7 | ổ cục bộ D: (docs/issue.md:160), mã cũ commit 8d1fcaa chia việc theo dung lượng; byte không đếm, 9 GB là 70.576 × 128 KB tính ra | chỉ nói "bị giới hạn bởi số thao tác, không bởi byte"; **không** dùng làm mốc so sánh |
| Đầu tệp chỉ tách được 166 / 29.053 ứng viên (0,6 %) | chú thích [dupes.rs:219-223](src-tauri/src/media/dupes.rs#L219-L223) | không ngày, không rõ chỉ mục nào; dẫn tới "PERF-003" mà docs/perf.md chưa có mục đó | bằng chứng thật cho "đa số ứng viên là bản sao thật" |
| NAS F: qua gigabit: byte đầu 66 ms, nhảy cuối + đọc 1 MB 18 ms, tuần tự 84,7 MB/s | [media_stream.rs:23-30](src-tauri/src/ipc/media_stream.rs#L23-L30), docs/test-log.md P14 | đo cho xem trước, không phải quét trùng | mô hình chi phí trên NAS: mở tệp đắt gấp 50 lần đọc thêm byte |
| Duyệt thư mục NAS: 12 luồng nhanh hơn 64 luồng | netsched.rs | đo `read_dir`, BFS theo mức | **không** áp dụng cho đọc nội dung tệp; chỉ mang sang được bài học "cache Windows làm lượt sau nhanh giả" |

Ba bộ đếm chưa có, nên mọi phép đo sau này sẽ lại là số suy ra nếu không thêm: byte đã đọc, số
lần mở thất bại, thời gian theo từng ổ. `hashed` tăng **trước** khi mở tệp
([dupes.rs:231](src-tauri/src/media/dupes.rs#L231)), nên "đã đối chiếu N tệp" đếm cả tệp không mở được.

## 3. Vì sao lâu

### 3.1 Phần việc là bản chất của thư viện

Đa số ứng viên tầng 1 **là** bản sao thật, nên tầng 2 không thể loại chúng, chỉ có thể xác nhận.
Bộ lọc miễn phí duy nhất còn lại là `mtime` (chỉ mục có sẵn, kể cả ổ mạng), nhưng nó bỏ sót bản
sao qua công cụ không giữ thời gian sửa, nên chỉ dùng được để **xếp thứ tự đọc**, không dùng để bỏ
đọc nếu vẫn muốn chắc.

### 3.2 Bị giới hạn bởi số thao tác mỗi tệp, không bởi byte

Mỗi tệp ở tầng 2 là mở + đọc đầu + đọc cuối + đóng ([fingerprint()](src-tauri/src/media/dupes.rs#L276-L298)).
Trên SMB, `seek` không phải một vòng đi-về (SMB2 READ mang sẵn offset), nên là ~3 vòng: CREATE,
READ, READ. Phép đo xem trước cho thấy mở-và-byte-đầu 66 ms còn đọc thêm 1 MB chỉ 18 ms: **độ trễ
nằm ở đĩa của NAS, không ở mạng**, nên tăng số I/O đang bay có trần, phải đo. Trên ổ cơ, nhảy tới
cuối một tệp video vài GB là một lần seek, và nhiều luồng cùng seek ngẫu nhiên làm đầu đọc chỉ chạy
qua chạy lại.

Hệ quả ngược với trực giác: `SMALL_FILE_LIMIT = 128 KB` ([dupes.rs:44](src-tauri/src/media/dupes.rs#L44))
đang **thấp hơn điểm hoà vốn**. Với tệp 128 KB đến ~1 MB, đọc trọn tệp một lần rẻ hơn đọc hai đầu,
vì bỏ được một READ mà chỉ thêm vài mili giây băng thông. Đọc **nhiều byte hơn** để **ít thao tác
hơn**.

### 3.3 Thứ tự đọc là ngẫu nhiên và số luồng là số CPU

Danh sách việc dựng từ duyệt `HashMap` ([dupes.rs:209-216](src-tauri/src/media/dupes.rs#L209-L216)),
thứ tự băm, khác nhau mỗi lần. Rayon chia thành các đoạn liên tiếp cho pool toàn cục có số luồng
bằng số CPU logic của **từng máy** (không có `ThreadPoolBuilder` nào trong mã). Nghĩa là trên 40
máy studio, độ song song đọc đĩa đổi theo CPU chứ không theo thiết bị I/O, và ứng viên ổ trong máy
trộn với ứng viên NAS nên một lần mở chậm trên NAS giữ chân luồng lẽ ra đang đọc đĩa cục bộ.

### 3.4 Không nhớ gì giữa các lần

Mỗi `start()` tính lại từ tầng 1 và đọc lại toàn bộ ứng viên, kể cả khi thư viện gần như không đổi
so với hôm qua. Khởi động lại máy, cập nhật phần mềm, Thoát ở khay, hay chỉ mục nạp lại: lượt sau
lại là "lần đầu". Ngay trong cùng tiến trình, vân tay từng tệp cũng không được giữ, chỉ giữ nhóm.

### 3.5 Không hiện gì cho tới khi xong hẳn

Giao diện chỉ có dòng đếm và một thanh hoạt ảnh cố định
([DuplicateFinder.svelte:157-162](src/lib/DuplicateFinder.svelte#L157-L162), [249-267](src/lib/DuplicateFinder.svelte#L249-L267)).
Người dọn ổ chỉ cần vài nhóm lớn nhất nhưng phải chờ hết lượt mới thấy nhóm đầu tiên. Thanh hoạt
ảnh còn lừa theo chiều ngược lại: lượt quét đang kẹt ở một lần mở tệp treo vẫn trông như đang chạy.

## 4. Bốn lỗi phát hiện kèm theo

Không phải lỗi tốc độ, nhưng đụng cùng đoạn mã. Lỗi đầu là lỗi đúng/sai.

### 4.1 Kết quả trỏ vào vị trí cũ sau khi chỉ mục nạp lại

`DuplicateGroup.entries` là **vị trí** trong chỉ mục ([dupes.rs:63](src-tauri/src/media/dupes.rs#L63)).
Chỉ mục được dựng lại sau mỗi lượt cập nhật gia tăng có xoá/di chuyển và sau **mỗi** lượt quét NAS
có thay đổi (netsched, 12 giờ một lần, đánh số lại toàn bộ mục NAS), và khi dựng lại thì vị trí không
được giữ: *"Entry positions are not preserved — they cannot be"*
([update.rs:142-143](src-tauri/src/index/update.rs#L142-L143)). `DupeService` không được đặt lại khi
chỉ mục đổi; `watch_cache` và `reload_index` chỉ thay chỉ mục và tăng `epoch`, không đụng tới nó.
`dupe_groups` chỉ chặn vị trí vượt độ dài rồi tra tên và đường dẫn trên **snapshot mới**
([commands.rs:572-593](src-tauri/src/ipc/commands.rs#L572-L593)).

Kịch bản: quét trùng lúc 9:00, rời màn hình, 10:25 netsched quét NAS xong và chỉ mục nạp lại, 11:00
quay lại. `completed` vẫn `true` nên không quét lại ([DuplicateFinder.svelte:120-123](src/lib/DuplicateFinder.svelte#L120-L123)),
mỗi nhóm hiện tên và đường dẫn của **tệp khác**, Enter mở đúng tệp sai đó, ảnh thu nhỏ cũng của tệp
khác. Không có cảnh báo nào. Với một tính năng mà bước tiếp theo là xoá tệp, đây là lỗi phải sửa
trước mọi thứ khác. Phản biện xác nhận, mức tin cậy cao.

### 4.2 Quét trùng chiếm pool luồng của tìm kiếm

Tầng 2 chạy `into_par_iter()` trên pool rayon toàn cục ([dupes.rs:225](src-tauri/src/media/dupes.rs#L225));
tìm kiếm chạy trên chính pool đó ([search.rs:362](src-tauri/src/index/search.rs#L362)), và duyệt NAS
nền cũng vậy ([walk.rs:95](src-tauri/src/walk.rs#L95)). Theo cách rayon chia việc, một truy vấn mới không
chen được vào đoạn đang chạy; nó chờ tới khi một luồng xong đoạn hiện tại, mà mỗi đoạn là hàng trăm
tới hàng nghìn tệp, mỗi tệp cả trăm mili giây. Phép kiểm `generation` để huỷ truy vấn cũ nằm **trong**
closure nên chỉ chạy sau khi đã được nhặt ([search.rs:364](src-tauri/src/index/search.rs#L364)).

Ước lượng của phản biện: tìm kiếm trong lúc quét trùng có thể đơ **hàng chục giây tới vài phút**,
không phải "hàng giây" như bản nháp đầu. Chưa đo. Rời màn Trùng lặp thì huỷ, nên phạm vi bị giới
hạn, nhưng enrichment, thumbnail và netsched nền vẫn chịu.

### 4.3 Ổ rớt thì kết quả thiếu mà vẫn báo hoàn tất

Tệp không mở được bị bỏ lặng lẽ ([dupes.rs:232](src-tauri/src/media/dupes.rs#L232)). NAS rớt giữa
lượt quét thì mọi ứng viên trên đó biến mất khỏi kết quả, `completed = true`, và giao diện có thể nói
*"Không tìm thấy tệp trùng lặp nào"*, một khẳng định sai. Không có số "không đọc được" nào lên giao
diện (`verify.rs` ở dòng cũ có `unreadable`; `dupes.rs` không). Thêm nữa, cờ `stop` được kiểm **trước**
khi mở nên không ngắt được một lần mở đang treo; `start()` từ chối tới khi các lần mở treo trả về.

### 4.4 Quét trùng và quét NAS nền chạy chồng nhau

`netsched` chỉ hỏi `is_scanning()`, cờ đó được định nghĩa là "có tiến trình `--index` elevated đang
chạy" ([state.rs:60-64](src-tauri/src/state.rs#L60-L64)); `DupeService.running` là cờ riêng. Chip
"Trùng lặp" không bao giờ bị khoá. Hai lượt cùng nện Y: và Z: (cùng máy chủ .213), và cùng tranh
pool rayon. Mức nhẹ, nhưng là lý do nữa để tách pool.

## 5. Giải pháp

Tám việc, xếp theo lợi ích trên công sức. Mỗi việc làm riêng được; phụ thuộc ghi rõ.

### A. Giữ kết quả sống sót qua nạp lại chỉ mục

**Làm gì.** `DupeService` giữ luôn `Arc<Index>` mà lượt quét đã dùng, cạnh `result`. `dupe_groups`
phân giải tên và đường dẫn bằng chính snapshot đó, không lấy từ `AppState`. Giao diện hiện "kết quả
từ chỉ mục lúc HH:MM" và một nút quét lại. Khi có vân tay bền (việc D), lượt quét mới ánh xạ sang
chỉ mục mới bằng đường dẫn + size + mtime thay vì đọc lại đĩa.

**Lợi.** Hết lỗi 4.1. Không phải quét lại sau mỗi lần cache đổi (ổ trong máy mỗi ngày, NAS hai lần
mỗi ngày). Tốn thêm ~10 MB RAM cho một `Arc<Index>`.

**Rủi ro.** Tệp đã xoá từ lúc quét vẫn hiện, nhưng hôm nay cũng vậy, và app đã có cách nói "chỉ mục
cũ tới đâu" (`freshness.rs`).

**Công sức.** S, một ngày. Nghiệm thu: test "chỉ mục thay bằng bản có vị trí khác thì nhóm vẫn trỏ
đúng đường dẫn cũ"; phá mã bằng cách lấy index từ `AppState` thì test đỏ.

### B. Hiện kết quả dần, cho phép dừng và giữ

**Làm gì.** Sau tầng 1, mỗi lớp dung lượng đã biết số tệp, nên **tiềm năng lớp** = size × (số tệp − 1)
là cận trên chính xác của mọi nhóm trong lớp. Sắp lớp theo tiềm năng giảm dần, gộp các lớp thành
đợt vài trăm tệp (không chạy rayon từng lớp, vì đuôi phân bố là hàng nghìn lớp 2–3 tệp), `par_iter`
trong đợt, công bố nhóm sau mỗi đợt. Mọi nhóm đã công bố có `wasted` ≥ tiềm năng lớp đang xử lý là
chung cuộc **và đã đúng thứ hạng**, nên giao diện nói được "các nhóm từ X GB trở lên đã chốt".

Đi kèm: định nghĩa lại huỷ. Hôm nay huỷ là vứt sạch (test `a_cancelled_scan_returns_nothing`); mới:
đợt đã công bố là chung cuộc, kết quả mang cờ "dừng ở mức X GB, chưa kiểm phần dưới". Giao diện:
gọi `dupeGroups()` khi `stat.groups` đổi thay vì chờ `running == false`; bỏ effect đặt `cursor = 0`
mỗi khi `rows` đổi ([DuplicateFinder.svelte:70-73](src/lib/DuplicateFinder.svelte#L70-L73)), nếu không
con trỏ nhảy lên đầu mỗi 400 ms; backend giữ `result` luôn sắp theo `wasted`; tiến độ hiện "đã kiểm
X GB / Y GB lãng phí tối đa" thay vì số tệp.

**Lợi.** Nhóm 3 × 16,6 GB (tiềm năng 33,3 GB, đứng đầu) hiện ra sau vài giây trên ổ trong máy; qua
SMB chưa đo. Người dọn ổ dừng khi đã đủ. Tổng thời gian không đổi.

**Công sức.** M, hai ngày kể cả test và giao diện. Nghiệm thu: test "nhóm của đợt đã xử lý xuất hiện
trước khi lượt quét kết thúc" và "nhóm đã công bố không bao giờ đổi"; chạy thật thấy nhóm đầu trước
10 giây trên ổ trong máy.

### C. Pool I/O riêng theo ổ, sắp việc theo (ổ, FRN), chọn phạm vi ổ

**Làm gì.** Bỏ `into_par_iter()` toàn cục. Chia `work` theo `volume_of(i)`; mỗi ổ một hàng đợi và một
nhóm luồng riêng, dùng `std::thread` với `SetThreadPriority(BELOW_NORMAL)` như enrichment. Trong ổ
trong máy, sắp theo `frn` (số bản ghi MFT, xấp xỉ thứ tự tạo tệp, chỉ mục đã có
[model.rs:143-153](src-tauri/src/index/model.rs#L143-L153)); trong ổ mạng, theo (thư mục, vị trí). Số
luồng ban đầu: ổ cơ 4, ổ mạng 8 mỗi máy chủ, rồi chỉnh theo phép đo (phản biện lưu ý HDD SATA có NCQ
nên 1–2 luồng có thể quá ít, và thứ tự đường dẫn chỉ là proxy của vị trí vật lý). Đồng thời thêm hai
tuỳ chọn trên thanh Trùng lặp, nhớ qua `prefs.ts`: "chỉ ổ trong máy" và chọn từng ổ, theo đúng tinh
thần hai nút "Quét lại" / "+ ổ mạng".

**Lợi.** Sửa 4.2 và 4.4 ngay. Ổ rớt không giữ chân luồng của ổ khác. Thời gian thật trên ổ cơ và trên
NAS **có thể** giảm vài lần, **chưa đo**; phép đo netsched không áp dụng vì đo thao tác khác. Cho phép
studio quét trùng ổ máy mình trong vài phút mà không đụng NAS.

**Rủi ro.** Số luồng cao trên NAS lúc nhiều máy cùng quét là tự nện máy chủ; giới hạn theo máy chủ
(`remote` từ `network_drives`) chứ không theo chữ ổ. Mã dài hơn một dòng `into_par_iter` khoảng
100 dòng; phải giữ hành vi huỷ cho từng hàng đợi.

**Công sức.** M, ba đến bốn ngày kể cả đo. Nghiệm thu: bảng đo lạnh trước/sau theo từng ổ; tìm kiếm
trong lúc quét vẫn dưới 20 ms (thêm phép đo này vào `dupes_real.rs`).

### D. Vân tay bền giữa các lần

**Làm gì.** Tệp `dupes.bin` cạnh `metadata.bin`, mượn **phần bền hoá** của mẫu `Store` trong
enrichment ([enrich.rs:70-86](src-tauri/src/media/enrich.rs#L70-L86)): khoá hash đường dẫn, giá trị
(size, mtime, vân tay 32 byte), header có magic + `SCHEMA_VERSION`, ghi tệp tạm mang PID rồi rename.
**Không** mượn mô hình 2 luồng ưu tiên thấp (người dùng đang đứng chờ ở màn này) và **không** lưu mỗi
500 tệp (`save_store` tuần tự hoá cả map; 70k ứng viên là ~140 lần ghi lại 13 MB). Lưu khi xong và khi
huỷ; huỷ giữ lại phần đã băm thay vì vứt. Chỉ lưu ứng viên tầng 1, ~56 byte một mục, khoảng 13 MB cho
chỉ mục 400k. Tỉa khoá không còn trong chỉ mục khi lưu (store của enrichment không bao giờ tỉa, nên
phình theo tệp đã xoá). Dùng hàm băm ổn định (không dùng `DefaultHasher`, tài liệu std không cam kết
giữa các bản Rust). Với ổ trong máy, khoá thêm theo (ổ, FRN) để tệp đổi tên không bị đọc lại; ổ mạng
FRN = 0 nên vẫn theo đường dẫn.

**Lợi.** Lượt sau chỉ mở tệp mới hoặc đổi: vài giây thay vì hàng chục phút, kể cả sau khởi động lại.
Về lâu dài đây là việc đổi trải nghiệm nhiều nhất, và với ổ mạng là nhiều nhất trong tất cả.

**Rủi ro.** Sao chép giữ mtime **không** phải rủi ro (khoá là đường dẫn, mỗi bản sao có vân tay
riêng). Rủi ro thật: sửa tại chỗ mà giữ nguyên size lẫn mtime (mtime độ phân giải 1 giây, công cụ như
`exiftool -P`) thì vân tay cũ được tin nhầm, có thể báo trùng sai. Đó là lý do tầng 3 trước khi xoá
vẫn bắt buộc (việc H). Tệp offline (NAS rớt) vẫn "có vân tay" và vẫn hiện trong nhóm; phải chọn chính
sách và nói rõ trên giao diện. Vô hiệu hoá: tăng `SCHEMA_VERSION` khi đổi `SAMPLE_BYTES`; nút "quét lại
từ đầu".

**Công sức.** M, hai đến ba ngày. Nghiệm thu: test "tệp có trong store với đúng size + mtime thì không
được mở" (đếm số lần mở, không phải `hashed`); "size đổi thì đọc lại"; "khoá không còn trong chỉ mục
thì bị tỉa"; chạy thật hai lượt qua khởi động lại app, lượt hai dưới 30 giây.

Hai biến thể mở rộng của D, để sau khi D chạy:

- **D2. Lấy vân tay ngay trong worker enrichment.** Enrichment đã mở từng tệp ổ trong máy qua shell
  ([enrich.rs:288-289](src-tauri/src/media/enrich.rs#L288-L289), 4–80 ms một tệp, 50 phút cho 117k
  tệp). Gọi `fingerprint()` ngay sau, khi 64 KB đầu gần chắc còn trong cache Windows. Sau khi enrichment
  xong, lượt quét trùng đầu tiên trên ổ trong máy gần như không đọc đĩa. Giá: enrichment dài thêm 1 đến
  15 phút tuỳ đuôi ảnh có lạnh không, **cần đo**. Không giúp NAS (enrichment bỏ qua ổ mạng, đúng đắn).
- **D3. Chia sẻ vân tay NAS giữa các máy.** Nội dung NAS giống nhau trên 40 máy; hôm nay mỗi máy tự
  đọc lại. Đặt tệp vân tay của ổ mạng (khoá theo UNC gốc + đường dẫn tương đối, không theo chữ ổ) ở một
  thư mục ẩn trên chính share, một máy ghi, các máy khác đọc. Tải NAS cho tính năng này giảm từ N máy
  xuống 1. Cần thử rename nguyên tử qua SMB trên đúng NAS trước, và cần quyền ghi lên share. Xa hơn.

### E. Sàn dung lượng và điểm đọc-trọn-tệp

**Làm gì.** Nâng `SMALL_FILE_LIMIT` từ 128 KB lên ~1 MB: tệp trong khoảng đó đọc trọn một lần thay vì
hai đầu, bỏ được một READ mỗi tệp mà không thêm độ trễ đáng kể (mục 3.2). Và cân nhắc sàn
`MIN_INTERESTING_SIZE` cao hơn 64 KB, dưới dạng tuỳ chọn có nhớ, **chỉ sau khi đo**.

**Vì sao phải đo trước.** Dữ liệu có sẵn cho thấy hai chiều ngược nhau: tệp "vật liệu dự án" của
studio trung bình 3,8 MB, và có 67.305 tệp `.mp3` (docs/check.md CHECK-002), nên sàn 4 MB cắt đúng
lớp tệp dự án; ngược lại, phần thư viện còn lại trung bình 59 MB một tệp thiên về video, nên tệp nhỏ
có thể **không** phải phần lớn ứng viên. Rủi ro thật không phải RAW (20–100 MB, không bị ảnh hưởng) mà
là SFX ngắn, PNG/JPG sticker nhân bản hàng nghìn lần: mất ít GB nhưng nhiều nhóm. Phép đo gần như miễn
phí: mỗi nhóm đã có `size` và `wasted`, gom histogram theo dải dung lượng từ kết quả một lượt quét.

**Công sức.** S, nửa ngày cho `SMALL_FILE_LIMIT`; sàn tuỳ chọn thêm nửa ngày.

### F. Ổ rớt: kiểm tra trước, đếm lỗi, không nói dối

**Làm gì.** Trước tầng 2, với mỗi chữ ổ trong `work`, kiểm tra ổ còn gắn bằng `GetDriveTypeW` /
`WNetGetConnectionW` (đã có trong `volume.rs` và `network_drives`; **không** dùng
`GetVolumeInformationW`, phản biện nghi nó cũng treo trên share chết). Ổ không đáp thì loại toàn bộ
tệp của ổ đó và ghi vào tiến độ. Thêm bộ đếm "không đọc được" vào `DupeProgress`; kết quả có tệp không
đọc được thì giao diện nói "thiếu N tệp trên Y:" thay vì "không tìm thấy tệp trùng lặp nào".

**Lợi.** Hết 4.3. Về thời gian treo: phản biện hạ mức tin cậy xuống "lần mở đầu của mỗi luồng treo tới
timeout (SMB SessTimeout mặc định 60 giây), các lần sau có thể hỏng nhanh nhờ negative cache", chưa
xác minh trên máy studio; nhưng cái giá của kiểm tra là không đáng kể.

**Công sức.** S, một ngày.

### G. Ba hướng chỉ làm sau khi đo

- **Tầng 2 chỉ đọc đầu.** Nếu đầu tệp đã tách 99,4 % ứng viên, lượt đọc đuôi hầu như chỉ xác nhận.
  Bỏ đuôi là bỏ một READ và một seek mỗi tệp (trên ổ cơ có thể 30–45 %, phải đo). Nhưng là hạ độ tin
  cậy có chủ ý; chỉ chấp nhận nếu phép đo trên thư viện thật cho số nhóm "chỉ khác đuôi" bằng 0 **và**
  có nút Xác minh (việc H). Lưu ý ghi chú PERF-003 trong mã nói về thử nghiệm **hai lần mở**, khác với
  đề xuất này là một lần mở một lần đọc.
- **Khoá (size, mtime, tên) làm tầng 1½.** Nếu CapCut sao chép bằng CopyFile thì bản sao giữ nguyên
  mtime, và cặp cùng size + mtime + tên là "gần chắc trùng" không cần đọc. Không biết CapCut có giữ
  mtime không; đo được miễn phí từ chỉ mục + kết quả quét cũ. Nếu đúng, dùng để xếp thứ tự đọc (an
  toàn) hoặc làm chế độ "nhanh" có nhãn rõ (không bao giờ là căn cứ xoá).
- **Cờ mở tệp Windows.** `FILE_FLAG_RANDOM_ACCESS` hoặc `NO_BUFFERING` cho tầng 2 để 9 GB mẫu không
  đuổi cache của người dùng ra khỏi RAM. Tốc độ kỳ vọng thấp (0–15 %); lợi thật là máy không chậm đi sau
  lượt quét, thứ thanh trạng thái không đo được. Loại trừ với D2 (D2 muốn cache nóng).

### H. Đưa tầng 3 xác minh theo nhóm về lại v2

Không phải việc tốc độ, nhưng là điều kiện của mọi thứ dẫn tới xoá: D chấp nhận rủi ro vân tay cũ,
G chấp nhận bỏ đuôi, và mục "Thùng rác cho tệp trùng" trong lộ trình cũ đều cần một nút "Xác minh"
hash trọn tệp cho đúng nhóm sắp hành động. Dòng `edit` đã có `verify.rs` (`verify_paths`, tuần tự có
chủ ý, đệm 1 MiB, báo `unreadable`) và lệnh `verify_dupe_group`; ghép sang v2 là việc S đến M, và
việc này sẽ nằm trong tài liệu hợp nhất hai dòng mã đang làm.

Việc rẻ đi kèm C hoặc D: hai mục cùng (ổ, FRN ≠ 0) là cùng một tệp (hardlink), không phải trùng, không
đọc. Và nếu hai chữ ổ ánh xạ chồng nhau (F: và H: cùng máy chủ .214, một share là thư mục con của share
kia), cùng một tệp vật lý xuất hiện hai lần trong chỉ mục thành "nhóm" không thu hồi được gì. Chưa biết
có xảy ra không; đo được từ chỉ mục trong vài giây.

## 6. Đo trước khi quyết

Mọi phép đo thêm vào `src-tauri/tests/dupes_real.rs`, chạy bằng
`cargo test --test dupes_real -- --ignored --nocapture` trên một máy studio đã quét cả ổ trong máy lẫn
bốn ổ NAS. Hai bẫy phải tránh: (1) cache Windows làm lượt sau nhanh giả, mọi so sánh A/B chạy trên hai
mẫu ứng viên rời nhau cùng lạnh, đảo thứ tự khi lặp; (2) `hashed` đếm cả tệp không mở được, test phải
in riêng số mở lỗi.

| # | Câu hỏi | Cách đo | Đọc đĩa | Quyết định gì |
|---|---|---|---|---|
| 0 | Ứng viên nằm ở đâu: theo loại × dải dung lượng × ổ; tiềm năng lãng phí mỗi ô; tỷ lệ nhóm cùng mtime / cùng tên; cặp cùng (ổ, FRN); ổ ánh xạ chồng nhau | tái tạo tầng 1 từ `index.sizes()`, in bảng | **không** | E (sàn), B (thứ tự), G (khoá mtime), phần hardlink/chồng ổ |
| 1 | Nền lạnh theo từng ổ với mã hiện tại | lọc `work` theo `volume_of`, chạy lạnh D:, F:, H:, Y:, Z:; ghi giây, tệp/giây, số mở lỗi | có | chia công sức giữa ổ trong máy và NAS; mốc so sánh cho mọi bước sau |
| 2 | Thứ tự và số luồng | ổ trong máy: hai mẫu lạnh 10k tệp, A theo HashMap, B theo FRN; lặp với 2/4/8/mặc định luồng. Một ổ NAS (Z:, nhỏ nhất): 1/4/8/16 luồng | có | C: nhận nếu ≥ 1,5× trên HDD hoặc NAS tăng gần tuyến tính; bỏ nếu chênh dưới 20 % |
| 3 | Chỉ đọc đầu có an toàn không | song song, hai mẫu lạnh, đếm nhóm "trùng đầu khác đuôi" và liệt kê chúng | có | G.1: chỉ nhận khi tiết kiệm ≥ 30 % và số nhóm chỉ-khác-đuôi = 0 |
| 4 | Vân tay trong enrichment tốn thêm bao nhiêu | 500 video + 500 ảnh chưa có metadata, đo `media_props` rồi `fingerprint` ngay sau, trung vị và p90 | có | D2: ghép cả hai đầu nếu video dưới 2 ms và ảnh dưới 10 ms |
| 5 | Tìm kiếm chậm bao nhiêu trong lúc quét | chạy `search` mỗi 500 ms trong khi `DupeService` chạy, ghi p50/p99 | có | mức khẩn của C |
| 6 | Cờ mở tệp | ba biến thể trên ba mẫu lạnh 5k tệp, đo thời gian và `Standby Cache` trước/sau | có | G.3 |

Thứ tự khuyến nghị: 0 (miễn phí, chốt được E và phần chồng ổ ngay) → 1 → 2 → 5 → 3 → 4 → 6. Mỗi
bước ghi vào docs/perf.md; mở mục PERF-003 trước để ghi lại số 166 / 29.053 đang chỉ nằm trong một
dòng chú thích.

## 7. Đề xuất gói, và câu hỏi cho bạn

| Gói | Gồm | Công sức | Được gì |
|---|---|---|---|
| Nhỏ | A, B, F, E (phần `SMALL_FILE_LIMIT`) | 4 đến 5 ngày | hết lỗi sai tệp; thấy nhóm lớn sau vài giây và dừng được; ổ rớt không nói dối; bớt một READ cho tệp nhỏ |
| Đủ | Nhỏ + C + D, sau phép đo 0, 1, 2, 5 | thêm 1 đến 1,5 tuần | lượt sau vài giây; lượt đầu nhanh hơn X lần với X đo được; tìm kiếm không đơ khi quét; chọn được phạm vi ổ |
| Sau | D2, D3, G, H | từng việc riêng | trùng lặp gần tức thì trên ổ trong máy; NAS một máy đọc cho cả studio; xác minh trước khi xoá |

Ba câu hỏi chỉ bạn trả lời được:

1. **Quét trùng có nên đọc ổ mạng mặc định không**, hay mặc định chỉ ổ trong máy và có nút bật từng ổ
   NAS. Ảnh hưởng thẳng tới thời gian lượt đầu và tải lên NAS khi nhiều máy cùng dùng. Khuyến nghị:
   mặc định chỉ ổ trong máy, nhớ lựa chọn.
2. **CapCut sao chép asset bằng cách nào.** Nếu bạn biết nó giữ nguyên thời gian sửa của tệp, khoá
   (size, mtime, tên) ở mục G trở nên rất đáng làm. Nếu không biết, phép đo 0 sẽ trả lời.
3. **Có muốn tôi viết phần đo (bước 0 và 1) trước không.** Đó là mã test, không đụng mã sản phẩm, và
   phải chạy trên một máy studio có chỉ mục thật; máy này chỉ mục chỉ 767 KB nên số đo không đại diện.
