# NHẬT KÝ TEST — MediaFinder, nhánh `version2`
> **Thuộc file này:** các lượt test từ **P34** trở đi, tức toàn bộ công việc trên nhánh
> `version2`. Nhánh này dựng lại từ tag `v1.0.4` sau khi bỏ nhánh `edit`, nên nền mã khác
> với các lượt P0–P33.
>
> Lịch sử tới P33 (nhánh `master`): [test-log.md](./test-log.md).
>
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md) · [test-log-v2](./test-log-v2.md)

> **Quy tắc:** xong mỗi giai đoạn phải chạy một lượt test **chủ động đi tìm lỗi**,
> không chỉ chạy cho có. Giai đoạn chỉ được coi là xong khi lượt test đã chạy và
> mọi phát hiện đã được ghi vào file phân loại.

## P34 — Lọc kết quả theo ổ đĩa (nhánh `version2`)

Tính năng đầu tiên dựng trên nền v1.0.4 sau khi quay về mốc đó. Bản thiết kế đặt ra ba lớp;
lớp thứ ba (gom nhóm kết quả theo ổ) **bị chính bản thiết kế bác bỏ** vì nó phá thứ tự liên
quan — cái quý nhất mà phần tìm kiếm đang có. Nên chỉ dựng hai lớp:

| Lớp | Là gì | Tệp |
|---|---|---|
| 1 | Hàng chip trên đầu danh sách: `Tất cả 4` · `D: 2` · `Y: 2` | [DriveChips.svelte](../src/lib/DriveChips.svelte) |
| 2 | Nhãn ổ nhỏ trên từng dòng kết quả | [MediaRow.svelte](../src/lib/MediaRow.svelte) |
| — | Phần nhận diện ổ, đếm, lọc | [drives.ts](../src/lib/drives.ts) |

Ràng buộc cốt lõi: **lọc chạy hoàn toàn ở giao diện, trên danh sách kết quả đã có** — không
thêm một lần tìm kiếm nào, không đụng tới `search.rs`. Đây không phải chi tiết kỹ thuật mà là
toàn bộ lý do tính năng đáng làm: bấm chip phải cho kết quả tức thì, không phải chờ NAS.
Bộ kiểm thử canh đúng điều này bằng `expect(ipc.count("search")).toBe(0)`.

### Lỗi nghiêm trọng nhất được chặn trước khi nó kịp ra bản phát hành

Cách nhận dạng ổ mạng hiển nhiên là "đường dẫn bắt đầu bằng `\\`". Đo bằng `net use` trên máy
studio thì **cả bốn ổ NAS đều là ổ ánh xạ** — `F:`, `H:`, `Y:`, `Z:` — và chỉ mục lưu chúng
dưới dạng `Y:\PROJECT…`. Cách nhận dạng hiển nhiên kia sẽ khiến toàn bộ nhánh phân biệt ổ mạng
thành **mã chết trên mọi máy**: không chip cam, không nhãn cam, ổ mạng không bị đẩy xuống cuối
hàng.

Điều khiến nó nguy hiểm là **không ai báo lỗi**: phần đếm và phần lọc vẫn chạy đúng: tính năng
chỉ lặng lẽ giải đúng một nửa vấn đề nó sinh ra để giải. Cách chặn: `isNetworkDrive` nhận thêm
tập chữ cái ổ mạng, lấy từ lệnh `network_drives` mà backend đã có sẵn từ v1.0.4.

### Kiểm thử — [t10-drives.test.ts](../tests/t10-drives.test.ts), 12 ca

Hai tầng, cố ý. Chín ca đầu kiểm `drives.ts` thuần. Ba ca sau **dựng App thật rồi bấm chip** —
vì các hàm thuần chạy đúng không chứng minh được rằng chúng đã được *nối* vào danh sách kết
quả. Bản v1.0.6 từng có đúng lỗi kiểu này.

### Thử bằng cách phá mã (mutation testing)

Một bài kiểm thử xanh chỉ có giá trị nếu nó đỏ khi mã sai. Ba phép phá:

| Phá cái gì | Kết quả |
|---|---|
| `filterByDrive` trả nguyên danh sách, không lọc | **3 ca đỏ** ✅ |
| `isNetworkDrive` bỏ qua danh sách ổ ánh xạ (đúng lỗi v1.0.6) | **2 ca đỏ** ✅ |
| Hàng chip hiện cả khi chỉ có một ổ | **1 ca đỏ** ✅ |

Cả ba đều bị bắt. Phép thứ hai là phép đáng giá nhất: nó chứng minh bộ kiểm thử bắt được đúng
loại lỗi *im lặng* đã mô tả ở trên.

### Vòng kiểm trước khi giao

| Bước | Kết quả |
|---|---|
| `cargo test` | **231 pass**, 0 fail — bằng đúng mốc trước khi sửa |
| `cargo clippy --all-targets` | **0 warning** |
| `cargo fmt --check` | sạch |
| `npm run check` | **0 lỗi / 121 tệp** |
| `npm test` | **88 pass** (76 cũ + 12 mới), 0 fail |

Rust không đổi một dòng nào — đúng như thiết kế, tính năng này sống trọn ở giao diện.

### Hai quyết định đáng ghi lại

**Bộ lọc ổ KHÔNG được lưu qua các phiên**, khác với lưới / sắp xếp / loại tệp. Mở app ra thấy
"không tìm thấy gì" chỉ vì phiên trước lỡ lọc ổ `Z:` là cái bẫy không đáng đặt. Cùng lý do đó,
khi một lần tìm mới không còn kết quả nào ở ổ đang chọn thì bộ lọc **tự buông** — màn hình rỗng
trong khi tệp vẫn nằm ngay đó, chỉ ở ổ khác, là kiểu hỏng khó hiểu nhất.

Điều này là bài học rút thẳng từ chính báo cáo của người dùng ở P33: "tìm kiếm bản 6 kém hơn
bản 4" hoá ra là **chip lọc Video đang bật** trong khi tệp cần tìm là ảnh `.avif`. Một bộ lọc
đang âm thầm chặn kết quả mà màn hình không nói ra là lỗi đắt hơn nhiều so với việc quên mất
lựa chọn của người dùng giữa hai phiên.

**Đổi ổ thì đưa con trỏ bàn phím về đầu danh sách.** Không làm thì `selected` còn trỏ vào vị
trí của danh sách cũ, và Enter mở nhầm tệp — hoặc mở một tệp không hề có trên màn hình.

### Một lỗ hổng của môi trường kiểm thử, không phải của mã

`$effect` cuộn danh sách về đầu gọi `Element.scrollTo` — **jsdom không cài đặt hàm này**, gọi
vào là ném `TypeError`, trong khi trình duyệt thật thì không. Đã vá trong
[vitest.setup.ts](../tests/vitest.setup.ts), cùng chỗ với các lỗ hổng jsdom đã biết khác
(`ResizeObserver`, `clientHeight`, `DragEvent`).

### P34b — Số hiệu phiên bản ở chân cửa sổ

Thêm `v1.0.7` ở góc phải cùng của thanh trạng thái, mờ hơn mọi thứ khác. Nó không phải thứ ai
cần đọc khi đang làm việc, nhưng là câu hỏi **đầu tiên** của mọi lần báo lỗi — "anh đang dùng
bản nào?". Không có nó thì câu trả lời phải đi vòng qua Control Panel.

Nguồn số hiệu là `update.current`, tức `CARGO_PKG_VERSION` lúc biên dịch. Lệnh `update_status`
**không gọi mạng** (`status()` chỉ đọc thứ đã ghi sẵn), nên số này luôn đúng với bản đang chạy
kể cả khi máy mất mạng. Chưa về thì không hiện gì — hơn là hiện một số sai.

Bốn ca kiểm thử. Ba ca đầu dựng App và đọc `.status .ver`, trong đó một ca kiểm rằng số hiện ra
**ngay khi app vừa mở, chưa gõ gì** — câu hỏi "bản nào" thường được hỏi đúng lúc đó, nên bắt
người dùng tìm một truy vấn mới thấy được số hiệu thì tính năng hỏng đúng lúc nó cần nhất.

Ca thứ tư là ca đáng giá nhất, và nó không đụng tới giao diện: **ba tệp khai báo phiên bản phải
nói cùng một số**. Ba ca trên dùng `"1.0.4"` cứng trong mock, nên chúng chỉ chứng minh giao diện
vẽ đúng cái backend đưa cho — *không* chứng minh cái backend đưa là đúng. Mà backend lấy từ
`Cargo.toml`, còn bộ cài và trình cập nhật lấy từ `tauri.conf.json`. Hai tệp lệch nhau thì app
nói một đằng, bộ cài một nẻo, và **không có gì bắt được** vì mỗi bên tự nó đều đúng.

| Phá cái gì | Kết quả |
|---|---|
| `package.json` lệch với `Cargo.toml` (1.0.6 vs 1.0.7) | **1 ca đỏ** ✅ |
| Bỏ thẻ `.ver` khỏi chân cửa sổ | **2 ca đỏ** ✅ |

Vòng kiểm: `npm run check` 0 lỗi/121 tệp · `npm test` **92 pass** (88 + 4 mới) · `cargo test`
231 pass. Rust không đổi dòng nào.

## P35 — BUG-025: ổ mạng tự quét lại theo lịch

### Nguyên nhân gốc, xác định bằng bằng chứng chứ không phải suy đoán

`scan_network_volumes()` trước bản vá này có **đúng một chỗ gọi**: nút "+ Ổ mạng". Tác vụ Windows
chạy mỗi ngày dùng `--index`, chạy **elevated** — mà ổ mạng ánh xạ thuộc về phiên đăng nhập nên
tiến trình elevated **không nhìn thấy chúng** (CHECK-007).

Nên đây không phải "ai đó quên gọi": đường quét tự động đang có về mặt kỹ thuật **không thể**
chạm tới ổ mạng. Hệ quả: ổ trong máy cập nhật mỗi ngày trong 1 mili giây, còn phần NAS của chỉ
mục đứng im vĩnh viễn cho tới khi có người tự bấm nút.

### Chi phí thật, đo từ log của máy studio

| Ổ | Máy chủ | Thời gian | Thư mục | Tệp media |
|---|---|---|---|---|
| `F:` | .214 | 13,1s | 4.108 | 146.662 |
| `H:` | .214 | 18,7s | 8.394 | 31.909 |
| **`Y:`** | .213 | **477,0s** | 7.686 | 159.095 |
| `Z:` | .213 | 15,1s | 958 | 18.953 |

Tổng **8 phút 44**, riêng `Y:` chiếm **91%**. Con số đó quyết định lịch thưa (12 tiếng) chứ
không dày: quét bốn lần/ngày là 35 phút đọc mạng mỗi máy mỗi ngày, nhân 20–40 máy thì cái giá
đổ hết lên chính NAS mọi người đang cần dùng.

### Một giả thuyết đã bác bỏ — và cách nó suýt lọt

Thấy `rayon` chỉ dùng 12 luồng (bằng số CPU), tôi nghi đó là nguyên nhân: với I/O mạng thì luồng
chủ yếu là *chờ*, không tốn CPU. Phép đo đầu ủng hộ rất mạnh — 64 luồng nhanh hơn **11 lần**.

Phép đo đó **tự lừa mình**: nó chạy 12 luồng trên thư mục lạnh, rồi chạy 64 luồng trên chính thư
mục vừa được Windows cache. Đo lại bằng hai mẫu riêng biệt cùng lạnh thì **12 luồng còn nhanh
hơn 64** — 987 so với 586 thư mục/giây.

Nếu không đo lại, tôi đã đề xuất một bản sửa dựa trên phép đo sai, và nó sẽ "hoạt động" đúng
nghĩa không làm gì cả. Bài học: một phép đo mà thứ tự chạy quyết định kết quả thì không phải
phép đo.

Phần chưa giải thích được: 8 lần kích thước chỉ lý giải ~121 giây, còn thực tế 477. **Còn hệ số
~4 lần chưa rõ.** Không đoán — ghi lại để điều tra riêng.

### Bản sửa — [netsched.rs](../src-tauri/src/netsched.rs), module riêng

Lịch chạy **trong tiến trình GUI**, vì đó là nơi duy nhất thấy được ổ mạng.

| Tham số | Giá trị | Vì sao |
|---|---|---|
| Lượt đầu | 5 phút sau khởi động | Lúc đăng nhập máy bận nhất; chen 8 phút đọc mạng vào đó là tranh băng thông với chính việc người dùng đang mở |
| Giữa hai lượt | 12 tiếng | ~2 lượt/ngày, xem bảng chi phí |
| Giãn giữa các máy | 0–20 phút, theo tên máy | 40 máy cùng đăng nhập 8h sáng mà không giãn thì cùng nện vào NAS một lúc |
| Nhịp kiểm tra | 1 phút | Ngủ thẳng 12 tiếng thì máy ngủ đông 8 tiếng sẽ đẩy lượt quét lùi 8 tiếng |

Độ giãn băm từ `COMPUTERNAME` chứ không dùng số ngẫu nhiên: không phải thêm thư viện chỉ để lấy
một con số, và quan trọng hơn — nó **ổn định**, máy A luôn quét sớm hơn máy B thay vì hai máy
bốc thăm lại mỗi lần khởi động rồi thỉnh thoảng trùng nhau.

Lịch **không bắn sự kiện** cho giao diện: `watch_cache` đã theo dõi đúng tệp mà
`scan_network_volumes` ghi và tự bắn `index-reloaded`. Bắn thêm là hai đường cùng nói một tin,
và đường thứ hai sẽ lặng lẽ sai đi khi cách lưu đổi.

### Kiểm thử — 9 ca, đều là ca về hành vi sai chứ không phải ca trang trí

Thử bằng cách phá mã:

| Phá cái gì | Kết quả |
|---|---|
| Bỏ phần giãn giờ (40 máy cùng nện vào NAS) | **1 ca đỏ** ✅ |
| Lượt đầu tính từ mốc 0 (quét ngay lúc đăng nhập) | **3 ca đỏ** ✅ |
| Bỏ `saturating_sub` (đồng hồ lùi → quét mỗi phút) | **1 ca đỏ** ✅ |

### Chạy thật — thứ mà kiểm thử đơn vị không thay được

Lần chạy đầu **không thấy dòng log nào của lịch**. Nguyên nhân hoá ra không phải mã sai: một bản
dev build cũ đang chạy và **single-instance** khiến bản mới thoát ngay, nên log tôi đọc là của
tiến trình cũ. Tắt bản cũ rồi chạy lại:

```
INFO mediafinder::netsched: lịch quét ổ mạng: lượt đầu sau 5 phút
     (giãn thêm 16 phút cho máy này), rồi mỗi 12 tiếng
```

Máy này bốc được 16 phút, nên lượt đầu rơi vào phút thứ 21. Cả móc `setup` lẫn phần giãn giờ đều
chạy đúng trên bản build thật.

### Vòng kiểm

`cargo test` **235 pass** (226 + 9) · clippy **0 warning** · fmt sạch · `cargo build` sạch.

### Chạy thật trọn một lượt — và con số lật lại kết luận

Lượt quét tự động đầu tiên, đo trên bản `tauri dev` của người dùng:

```
10:03:45  lịch quét ổ mạng: lượt đầu sau 5 phút (giãn thêm 16 phút cho máy này)
10:25:45  quét ổ mạng: F:, H:, Y:, Z:
10:27:35  lịch quét ổ mạng xong: 4 ổ · 358772 tệp · 109.9s
10:27:38  cache đã thay đổi bên ngoài — nạp lại: 408548 tệp
```

Chạy lúc 10:25, đúng dự đoán 10:24. Toàn chuỗi hoạt động: lịch → quét → hợp nhất → `watch_cache`
phát hiện → nạp lại. Chỉ mục từ 400.024 lên **408.548** — **8.524 tệp mới** mà trước bản vá này
sẽ không ai tìm ra cho tới khi có người tự bấm nút.

**Con số buộc phải sửa lại kết luận:**

| Ổ | Lần đo đầu | Lần này | |
|---|---|---|---|
| `F:` | 13,1s | **0,7s** | |
| `H:` | 18,7s | **2,7s** | |
| `Y:` | **477,0s** | **87,6s** | ↓ 5,4× |
| `Z:` | 15,1s | 15,7s | |
| **Tổng** | **523,9s** | **109,9s** | ↓ 4,8× |

Không phải 8 phút 44 mà **1 phút 50**.

Và đây chính là "hệ số ~4 lần chưa giải thích được" đã ghi ở trên. Câu trả lời: 477 giây là lần
quét **lạnh hoàn toàn**, chưa có gì trong cache thư mục của Windows. Lần này cây đã ấm nên còn
87,6 giây.

Điều đáng giữ lại về cách làm: hệ số đó được ghi là "chưa biết" thay vì được lấp bằng một phỏng
đoán nghe hợp lý. Câu trả lời đến từ phép đo thứ hai, không từ suy luận.

**Hệ quả cho lịch:** `GIUA_HAI_LUOT = 12 tiếng` đặt dựa trên số liệu xấu nhất, nên nó **thưa hơn
mức cần thiết**. Với 110 giây thực tế, 3–4 lượt/ngày là khả thi.

Chưa đổi, có chủ đích. Mới hai phép đo mà chênh nhau 5 lần — chưa đủ để biết cái nào là thường
lệ. Và 110 giây này đo trên máy vừa quét xong nên cache còn ấm; máy vừa đăng nhập buổi sáng gần
với trường hợp lạnh hơn. Để lịch chạy vài ngày, có 5–6 phép đo ở các thời điểm khác nhau rồi mới
quyết — dựa trên dữ liệu chứ không trên một lần may mắn.

### Một sai sót trong cách tôi tự kiểm

Phép thử "chạy thật" ở phần trên đọc `%LOCALAPPDATA%\mediafinder\logs\mediafinder.log`, nhưng
bản `tauri dev` **không ghi vào tệp đó** — nó chỉ in ra terminal, và tệp log dừng ở một phiên
v1.0.6 cũ. Nên thứ tôi tưởng là bằng chứng thực ra là log của phiên khác. Bằng chứng thật là
terminal của người dùng.

Cùng loại với lỗi cache ở trên: cả hai đều là phép đo trông giống phép đo đúng nhưng đang nhìn
vào nhầm thứ.

## P36 — Màn hình trống nói đúng nguyên nhân

Trước bản này, app có **một** câu cho bốn tình huống khác hẳn nhau: *"Không tìm thấy kết quả
nào"*. Ba trong bốn là app đang **tự che mất câu trả lời** — nhưng câu chữ đổ hết cho người gõ.

Thiệt hại đã xảy ra thật: một người tìm tệp `.avif` trong lúc chip lọc *Video* đang bật, không
thấy gì, rồi kết luận **công cụ tìm kiếm kém đi**. Công cụ không sai; màn hình nói sai.

### Bốn câu, hỏi theo thứ tự chắc chắn

| # | Tình huống | Câu nói | Mức tin |
|---|---|---|---|
| 1 | Bộ lọc đang che | "Bộ lọc đang ẩn **3 kết quả**" + nút Bỏ lọc | chắc chắn |
| 2 | Ổ mạng chưa quét lần nào | "Ổ mạng Y:, Z: **chưa được quét lần nào**" | chắc chắn |
| 3 | Ổ mạng đã lâu chưa quét | "Quét lần cuối **6 tiếng trước**" + nút (~2 phút) | có thể |
| 4 | Đã loại trừ hết | "Không tìm thấy" + **nói rõ đã loại trừ gì** | chắc chắn |

Thứ tự **chính là** thứ tự chắc chắn. Đảo nó là để một phỏng đoán che mất một sự thật — và
`reasonFor` có ca kiểm thử riêng canh đúng điều đó.

Câu số 1 là câu duy nhất đưa ra con số: app giữ sẵn `allHits` (danh sách **trước khi lọc**) nên
nó **đếm được thật**. "Bỏ lọc thì có 3 kết quả" là sự thật, không phải lời hứa.

Câu số 3 chỉ nói **"có thể"**, cố ý. Ổ mạng không có nhật ký thay đổi để hỏi — đó chính là gốc
của BUG-025 — nên app chỉ biết *chỉ mục của mình cũ tới đâu*, không biết trên NAS có gì mới.
Viết "tệp của bạn vừa được tải lên" là khẳng định một điều app chưa hề xác minh.

### Hai ý tưởng bị chính phép đo bác bỏ

**Quét đĩa tìm tệp mới.** Đo trong ngân sách 300ms: chỉ với tới **2 tầng thư mục** trên cả `D:`
lẫn `Y:`. Tệp thật của người dùng nằm ở tầng 4 (`Y:\PROJECT CAPCUT\TÀI NGUYÊN DEEP SEA\TEST\`).
Nó sẽ **không bao giờ tìm thấy**, chỉ tạo cảm giác đã kiểm tra rồi im lặng.

**Hỏi USN journal để nói "ổ D: có 1.352 thay đổi".** [CHECK-004](check.md) đã đo dứt khoát:
`FSCTL_READ_USN_JOURNAL` đòi `FILE_READ_DATA` trên volume, tức **quyền Administrator**, mà bất
biến kiến trúc là GUI không bao giờ chạy elevated. Con số 1.352 trong bản thiết kế ban đầu đến
từ log của *tiến trình quét elevated*, không phải từ app — tôi đã trích nó vào bản minh hoạ mà
chưa kiểm đường lấy. Phát hiện lúc đọc mã, trước khi viết dòng nào.

Đổi lại: cache lưu sẵn chữ cái và số tệp **cho từng ổ**, không cần quyền gì. Mất phần "bao nhiêu
thay đổi", giữ phần quyết định — *chỉ mục cũ tới đâu*.

### Một bài kiểm thử xanh vô nghĩa, và cách nó lộ ra

Ca "đồng hồ chạy lùi" ban đầu chỉ kiểm `kind`. Phá mã (bỏ `Math.max(0, …)`) thì **cả 14 ca vẫn
xanh** — ca đó không kiểm được gì.

Đào tiếp thì lý do sâu hơn dự đoán ban đầu: tuổi âm luôn nhỏ hơn mọi ngưỡng, nên không nhánh nào
chạy và `agoText` **không bao giờ** nhận số âm. `Math.max` là lớp bảo vệ cho tình huống hôm nay
**không thể xảy ra** — không mã nào phá được nó, nên không bài kiểm thử nào bắt được.

Đã viết lại ca đó để canh đúng thứ nó canh được (`agoText` với số âm — hàm công khai, gọi được
từ chỗ khác), và ghi thẳng vào cả mã lẫn kiểm thử rằng phần kia không canh được. Giữ `Math.max`
vì nó rẻ và vì hạ một ngưỡng xuống 0 sẽ làm nó cần thiết ngay.

Phá mã kiểm chứng, bốn phép:

| Phá cái gì | Kết quả |
|---|---|
| Đổ cho bộ lọc kể cả khi bỏ lọc ra vẫn rỗng | **1 ca đỏ** ✅ |
| Bỏ ưu tiên của bộ lọc (để suy đoán chen lên trước) | **3 ca đỏ** ✅ |
| Gộp "ổ chưa quét" vào nhánh "ổ mạng cũ" | **2 ca đỏ** ✅ |
| `agoText` không xử lý số âm | **1 ca đỏ** ✅ |

### Tệp

| Tệp | Vai trò |
|---|---|
| [freshness.rs](../src-tauri/src/freshness.rs) *(mới)* | Lệnh IPC: chỉ mục cũ tới đâu, tách ổ trong máy khỏi ổ mạng |
| [emptyReason.ts](../src/lib/emptyReason.ts) *(mới)* | Logic quyết định câu nào — thuần, kiểm thử được tách khỏi giao diện |
| [EmptyReason.svelte](../src/lib/EmptyReason.svelte) *(mới)* | Hiển thị, kèm nút hành động |
| [App.svelte](../src/App.svelte) | Thay khối cũ; chỉ hỏi độ mới **khi đã không có kết quả** |

Việc hỏi độ mới nằm **ngoài** đường tìm kiếm, nên nó không thể làm chậm việc tìm.

### Vòng kiểm

`cargo test` **240 pass** (235 + 5) · clippy 0 · fmt sạch · `npm run check` 0 lỗi/123 tệp ·
`npm test` **108 pass** (94 + 14) · `cargo build` và `npm run build` đều sạch.

### Hai lỗi người dùng bắt được ngay lượt thử đầu

**Lỗi 1 — quét ổ mạng xong vẫn báo "chưa được quét lần nào", vĩnh viễn.**

Bản đầu đọc số tệp từ trường `volumes` của cache. Nhưng ổ mạng **cố ý không có dòng nào** trong
đó — chú thích ngay tại chỗ hợp nhất đã nói rõ: *"Network drives get no stamp — there is no
journal to record a position in, and inventing one would make an incremental update think it
could follow them."* Tôi đọc sót chú thích ấy và hiểu "không có dòng" thành "chưa quét".

Sửa: đếm tệp theo ổ **từ chính chỉ mục**. `volume_of` suy ra chữ cái từ đường dẫn của từng tệp,
nên nó đúng cho mọi loại ổ.

Điều đáng nói: **cả 5 ca kiểm thử đều xanh trước lẫn sau khi sửa** — không ca nào canh lỗi này.
Đã thêm 2 ca, và kiểm chứng bằng cách quay lại đúng cách làm sai (`stamps` lọc bỏ ổ mạng): ca mới
đỏ ngay.

**Lỗi 2 — dòng báo lý do dạt hẳn sang lề phải.**

`.results` là hàng flex, và khối `<p class="empty">` mang `flex: 1`. Nó **luôn** được dựng, nên
khi có truy vấn nó thành một ô RỖNG vẫn chiếm hết chiều ngang và đẩy phần báo lý do sang bên.

Sửa: hai khối thành hai nhánh loại trừ nhau (`{:else if …}` / `{:else}`), cộng `flex: 1` cho
component để nó tự căn giữa.

**Bài kiểm thử đầu tiên tôi viết cho lỗi này lại là một bài xanh vô nghĩa nữa.** Nó đếm "ô rỗng
đứng cạnh", nhưng phá một nhánh không tái hiện được lỗi vì cấu trúc `if/else` vẫn giữ tính loại
trừ — bài xanh cả khi đã phá. Viết lại để canh đúng bất biến là bản sửa: **hàng đó có đúng một
khối, và nó không rỗng**. Phá bằng cách dựng cả hai khối cùng lúc thì nó đỏ: *"hàng có 2 khối"*.

Đây là lần thứ hai trong cùng một phiên một bài kiểm thử của tôi xanh mà không canh gì. Cả hai
lần đều chỉ lộ ra khi phá mã — chạy bộ kiểm thử và thấy nó xanh không nói lên điều gì cả.

### Vòng kiểm sau khi sửa

`cargo test` **242 pass** · clippy 0 · fmt sạch · `npm run check` 0 lỗi/123 tệp ·
`npm test` **109 pass**.

## P37 — Phím tắt có phương án dự phòng

### Vấn đề

App thử **đúng một** tổ hợp `Ctrl+Alt+Space`, thất bại thì bỏ cuộc. Và câu nó nói với người dùng
là một lời khuyên gần như không dùng được:

> *"đang bị ứng dụng khác chiếm — **đóng ứng dụng đó rồi mở lại MediaFinder** để dùng được phím
> tắt"*

Thứ chiếm phím thường là bộ gõ tiếng Việt, phần mềm chụp màn hình, hay công cụ của studio — những
thứ người ta cần chạy suốt ngày. "Đóng nó đi" không phải một lựa chọn, nên người dùng mất hẳn
phím tắt: đúng thứ chính để gọi cửa sổ, vì app khởi động ẩn.

### Bản sửa — [hotkey.rs](../src-tauri/src/hotkey.rs), module riêng

Thử lần lượt bốn tổ hợp, lấy cái đầu tiên đăng ký được:

`Ctrl+Alt+Space` → `Ctrl+Alt+F` → `Ctrl+Shift+Space` → `Ctrl+Alt+M`

`Ctrl+Alt+Space` giữ vị trí đầu vì người đã quen thì không nên bị đổi. Ba tổ hợp dự phòng chọn
theo cùng nguyên tắc với tổ hợp gốc: **không** đụng `Alt+Space` (menu hệ thống Windows) và
**không** đụng `Ctrl+Space` (chuyển bộ gõ ở nhiều ngôn ngữ, *kể cả tiếng Việt*).

Tự chọn chứ không hỏi người dùng, vì phần mềm chạy trên 20–40 máy: một tuỳ chọn thủ công nghĩa là
ai đó phải đi đặt trên từng máy. Phần cài đặt để đổi phím riêng — **hoãn có chủ đích**, cho tới
khi có người thật sự cần; hôm nay app chưa có màn hình cài đặt nào, và dựng cả một màn hình để
chứa đúng một mục là dựng cái khung đắt hơn thứ nó chứa.

### Ba trạng thái, không phải hai

Giao diện trước đây chỉ biết "có phím" và "không có phím". Nay:

| Trạng thái | Màn hình nói gì |
|---|---|
| Giành được tổ hợp ưu tiên | Hiện phím, như cũ |
| Đang dùng phím dự phòng | Hiện **tổ hợp thật**, kèm "Ctrl+Alt+Space đang bị chiếm nên dùng tổ hợp trên thay thế" |
| Không giành được cái nào | Không vẽ phím nào, chỉ đường mở bằng biểu tượng khay hệ thống |

Trạng thái thứ hai là thứ trả lời câu "sao phím quen của tôi không còn tác dụng?" — phải nói cả
cái mất lẫn cái thay thế, nếu không người dùng tưởng app hỏng.

Sáu chỗ hiện tổ hợp (log khởi động, menu khay, tooltip khay, thông báo lỗi khay, chân cửa sổ,
lệnh IPC) nay đều đọc **cùng một nguồn**. Một chỗ viết cứng `Ctrl+Alt+Space` là chỗ đó nói dối
ngay khi phải dùng phím dự phòng.

### Kiểm thử — 8 ca backend, 3 ca giao diện

Phá mã, cả ba đều bị bắt:

| Phá cái gì | Kết quả |
|---|---|
| Chỉ thử một tổ hợp rồi bỏ cuộc (đúng lỗi gốc) | **3 ca đỏ** ✅ |
| Giao diện in tổ hợp *mong muốn* thay vì tổ hợp *thật* | **1 ca đỏ** ✅ |
| Không có phím nào mà vẫn vẽ ô phím rỗng | **1 ca đỏ** ✅ |

Ca thứ hai canh một lỗi im lặng đáng giá: màn hình mời người dùng bấm một tổ hợp không có tác
dụng gì, mà mọi thứ khác trông vẫn bình thường.

Bài kiểm thử cũ `the_hotkey_avoids_combinations_windows_and_the_ime_already_use` được giữ nhưng
nay áp cho **cả bốn** tổ hợp — một phương án dự phòng đụng menu hệ thống thì tệ hơn là không có.

### Vòng kiểm

`cargo test` **249 pass** (242 + 7) · clippy 0 · fmt sạch · `npm run check` 0 lỗi/123 tệp ·
`npm test` **112 pass** (109 + 3) · `cargo build` sạch.

**Chưa kiểm được trên máy thật:** phần đăng ký phím tắt cần một tiến trình chạy thật, mà bản dev
của người dùng đang giữ single-instance. Cách kiểm: mở một ứng dụng khác giữ `Ctrl+Alt+Space`
trước, rồi khởi động MediaFinder và xem log có dòng *"đang bị ứng dụng khác chiếm — dùng
Ctrl+Alt+F thay thế"* không.

## P38 — Kiểm toán nâng cấp v1.0.4 → v1.0.8: có xung đột không?

Câu hỏi của người dùng: máy đang ở v1.0.4 ổn định, nâng thẳng lên bản cắt từ HEAD `version2` thì
có xung đột không. Trả lời bằng mã và phép đo, không bằng trí nhớ: `git diff v1.0.4..HEAD`, đọc
bộ gỡ cài của v1.0.4, đọc mã bộ cập nhật, rồi cho 31 agent **cố phản bác** từng kết luận và quét
tìm thứ bị bỏ sót (3 triệu token, 874 lượt đọc mã). Năm phát hiện nặng nhất tôi tự kiểm lại.

### Câu trả lời

**Không có xung đột dữ liệu** — định dạng `index.bin`/`metadata.bin` (`SCHEMA_VERSION 3`,
`MFMETA01`), tuỳ chọn localStorage, XML tác vụ nền, endpoint và khoá ký: **không đổi một dòng**.
Nhảy cóc qua v1.0.5–7 là bình thường, bộ cập nhật chỉ so sánh số hiệu.

Nhưng kết quả tuỳ **đường** nâng cấp:

| Đường | Kết quả | Vì sao |
|---|---|---|
| Nút **Cập nhật** trong app | An toàn — giữ chỉ mục, tác vụ, tuỳ chọn | updater.rs:812 luôn nối `/UPDATE`; installer.nsi:314 `$UpdateMode = 1 → reinst_done`, bộ gỡ cũ không chạy |
| Tải `.exe` **cài đè tay** | **Mất `index.bin`, `metadata.bin`**; mất tác vụ nền nếu bấm Yes ở UAC | Bộ gỡ đang nằm trên máy là của **v1.0.4**, hai móc xoá vô điều kiện (`v1.0.4:nsis-hooks.nsh:20-35`). Chốt chặn ở HEAD nằm trong gói mới, ghi ra *sau* khi cài |

### Ba thứ sẽ làm hỏng lần phát hành nếu cắt từ HEAD như hiện tại

Cả ba đều **không phải lỗi mã**, mà là hồ sơ phát hành — và cả ba đều tự kiểm lại được:

1. **`release.yml:87` vẫn in câu sai của BUG-024** lên trang Releases: *"Cài đè lên bản cũ được…
   Chỉ mục đã quét vẫn giữ nguyên."* Câu này đã bị sửa ở v1.0.6, nhưng `version2` dựng từ v1.0.4
   nên nó **quay lại** — và chỉ thẳng người dùng v1.0.4 vào đúng cái bẫy ở bảng trên.
2. **Dòng 1 của `RELEASE_NOTES.md`** là *"⚠️ Bản thử… Các máy đang dùng v1.0.4 sẽ không nhận được
   thông báo"*. Nó nằm **trước** vạch `---`, tức phần app giữ lại để hiện trong hộp thoại. Ngày bỏ
   dấu pre-release để 20–40 máy nhận, hộp thoại trên máy họ sẽ mở đầu bằng chính câu đó — tự mâu
   thuẫn, và không nói gì về tự quét ổ mạng hay phím tắt có thể đổi.
3. **Số hiệu vẫn là `1.0.7`** ở `tauri.conf.json`, `Cargo.toml`, `package.json` — trong khi tag
   `v1.0.7` đã tồn tại trên tổ tiên `c2ec7d5`. Cắt "v1.0.8" mà quên nâng thì bộ cài tự xưng 1.0.7
   và tauri-action gắn vào Release cũ. `package-lock.json` còn `1.0.4` (không có trong diff; CI vẫn
   qua nên chỉ là lệch hồ sơ, nhưng nên sửa cùng lúc).

### Hai lỗi mới do chính `version2` gây ra cho máy v1.0.4 sau khi nâng cấp bằng nút

**Lịch quét ổ mạng (BUG-025) đua với tác vụ nền cũ.** Tác vụ của v1.0.4 (đăng nhập+1 phút, 13:00)
được giữ nguyên qua cập nhật — đúng như mong muốn. Nhưng `netsched` (5–25 phút sau đăng nhập, rồi
mỗi 12 tiếng) và tác vụ elevated là **hai tiến trình**, còn `is_scanning()` là `AtomicBool` trong
tiến trình GUI — không nhìn thấy tác vụ. `persist::save` ghi `index.bin.tmp` rồi `rename`
(persist.rs:131-150) nên không hỏng tệp, nhưng **kẻ ghi sau thắng**: netsched nạp cache lúc T0, đi
NAS 2–9 phút; tác vụ chạy `--index` lúc T0.5 ghi thay đổi ổ trong máy; netsched ghi đè lúc T1 với
bản cục-bộ-của-T0 + NAS mới → **thay đổi ổ trong máy từ T0→T0.5 mất** tới lượt tác vụ kế tiếp.
Cửa sổ va chạm có thật: đăng nhập 12:40–12:55.

**Sau khi cập nhật bằng nút, cửa sổ không hiện lại.** `.onInstSuccess` (installer.nsi:714-721)
chạy lại app với đúng tham số cũ qua `/ARGS` (updater.rs:797). Trên máy studio app khởi động từ
lối tắt Startup với `--minimized` → app mở lại **ẩn ở khay**, dù hộp thoại hứa "tự khởi động
lại". Người dùng tưởng cập nhật hỏng.

Kèm theo, mức trung bình: giao diện **không biết** netsched đang quét — bấm *Quét lại* lúc đó nhận
"Đang có một lượt quét chạy rồi." mà không có thanh tiến trình, không có nút Dừng. Và nút *Cập
nhật* không khoá khi đang quét: bộ cài passive giết theo tên exe, có thể nhắm cả tiến trình
`--index` elevated.

### Hai kết luận của tôi bị bác — và đúng là đáng bác

* **C4** *"tác vụ chỉ được tạo lại khi bấm Quét lại, ổ trong máy âm thầm ngừng cập nhật"* — sai ở
  hai chỗ. Mọi nút dẫn tới `run_indexer()` (kể cả *+ Ổ mạng*) đều tạo lại. Và không "âm thầm":
  chỉ mục cũng bị xoá cùng lúc nên màn hình FirstRun **lộ ra ngay**, lượt quét bắt buộc đầu tiên
  tạo lại tác vụ. Thêm nữa: cài đè tay bật một hộp **UAC của exe cũ** giữa chừng
  (`--remove-setup` không `--quiet`, main.rs:20-25); bấm **No** thì tác vụ còn nguyên.
* **C5** *"prefs / XML tác vụ / endpoint không đổi → không xung đột"* — đúng về mã, **thiếu** về
  phạm vi: xung đột nằm ở `release.yml`, `RELEASE_NOTES.md` và số hiệu — chính ba mục ở trên.

### Những gì bị bác hoặc mức thấp

Bị bác 1/21: "mốc `built_at_unix` chung bị netsched làm mới" — không đứng vững. Mức thấp, ghi để
biết: lối tắt Startup ghi cứng "(Ctrl+Alt+Space)" trong mô tả `.lnk` (setup.rs:250) nên trên máy
dùng phím dự phòng nó nói sai phím; máy đã mất `index.bin` (cài đè tay) vẫn bị netsched đọc NAS
2–9 phút mỗi 12 tiếng rồi **vứt kết quả** vì không có cache để hợp nhất; móc `.nsh` ở HEAD xoá
`netscan.json`/`misses.*`/`logs\` mà không mã nào tạo (vô hại); `docs/bug.md` thiếu BUG-024/025;
`tests/installer_hooks.rs` (4 bài canh `.nsh`) không có trên nhánh này — `.nsh` HEAD giống v1.0.6
từng byte, nên hổng lưới kiểm thử chứ không hổng hành vi.

### Kết luận

Nâng cấp **bằng nút** thì không xung đột. Nhưng **không được cắt tag từ HEAD như hiện tại**: ba
mục hồ sơ phát hành phải sửa trước (câu sai ở `release.yml`, dòng "Bản thử" ở `RELEASE_NOTES.md`,
nâng số hiệu ở 4 chỗ). Hai lỗi mới (đua ghi cache, mở lại ẩn) nên sửa trước khi bản này tới 20–40
máy — cả hai đều do tính năng mới gây ra, không phải v1.0.4.
