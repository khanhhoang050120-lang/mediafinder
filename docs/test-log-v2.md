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

## P39 — Chuẩn bị v1.0.8, bản phát hành thật cho người dùng

Khác với v1.0.7 (bản thử, nháp trên GitHub), v1.0.8 đi thẳng tới 20–40 máy. Cuộc kiểm toán ở P38
tìm ra bốn thứ chặn đường và hai lỗi mã; tất cả đã sửa.

### Bốn mục phát hành

| # | Vấn đề | Hậu quả nếu bỏ qua |
|---|---|---|
| 1 | Số hiệu còn `1.0.7` ở 3 chỗ, `package-lock.json` còn `1.0.4` | Tag `v1.0.7` đã tồn tại; bộ cài tự xưng sai số hiệu |
| 2 | `release.yml` in câu sai của BUG-024 | **Chỉ thẳng người dùng v1.0.4 vào cái bẫy mất chỉ mục** |
| 3 | Ghi chú mở đầu bằng "⚠️ Bản thử… máy v1.0.4 sẽ không nhận" | Hộp thoại trên 40 máy nói một câu tự mâu thuẫn |
| 4 | `prerelease: true` | Bản build vô hình với trình cập nhật |

Mục 2 nặng nhất. Câu *"Cài đè lên bản cũ được, không cần gỡ trước. Chỉ mục đã quét vẫn giữ
nguyên"* đã bị sửa ở v1.0.6 vì nó **sai**, nhưng `version2` dựng từ v1.0.4 nên nó quay lại. Bộ gỡ
cài đặt *chạy* khi cài tay là bộ gỡ **cũ** đang nằm trên máy, mang móc xoá `index.bin` và
`metadata.bin` vô điều kiện. Nay ghi chú nói thẳng: cập nhật bằng nút trong ứng dụng, và nếu buộc
phải cài tay thì chọn **Do not uninstall**.

Số hiệu nay đồng bộ ở **năm** chỗ: `tauri.conf.json`, `package.json`, `Cargo.toml`, `Cargo.lock`,
và `package-lock.json` (hai vị trí) — chỗ cuối đang lệch hẳn hai bản.

### Lỗi 1 — cập nhật xong cửa sổ không hiện lại

Hộp thoại hứa *"ứng dụng sẽ tự khởi động lại"*, nhưng trên máy studio lời hứa đó không giữ được:

1. Lối tắt Startup chạy app với `--minimized` — cố ý, để không bật cửa sổ mỗi lần đăng nhập.
2. `tauri-plugin-updater` khởi động lại kèm **nguyên dòng lệnh hiện tại** (`updater.rs:797`,
   `current_exe_args()[1..]` → `/ARGS`).
3. App mở lại **ẩn ở khay**. Màn hình không đổi gì.

Người vừa bấm "Cập nhật" và chờ sẽ kết luận bản cập nhật hỏng — trong khi nó cài xong hoàn toàn
bình thường.

Không sửa được ở phía thư viện (`current_exe_args` là `pub(crate)`). Bản sửa
[relaunch.rs](../src-tauri/src/relaunch.rs): ghi một tệp mốc rỗng trước khi cài, đọc-rồi-**xoá** ở
lần khởi động kế tiếp. Dùng tệp chứ không phải biến môi trường vì tiến trình cũ chết hẳn trước khi
tiến trình mới sinh ra, và bộ cài đứng giữa — không gì sống sót qua đó ngoài đĩa.

### Lỗi 2 — hai tiến trình cùng ghi một tệp tạm

Từ khi có lịch tự quét ổ mạng (P35, chạy trong tiến trình giao diện) thì có **đúng hai** tiến
trình cùng ghi cache: tiến trình này và tác vụ nền `--index` chạy elevated mỗi ngày. Cờ
`is_scanning` chỉ sống trong `AppState` của tiến trình giao diện nên nó **không hề thấy** tác vụ
kia.

Ghi cache vốn nguyên tử (ghi tệp tạm rồi `rename`), nhưng cả hai dùng **cùng một tên tạm**:

```
A: File::create(tmp)   B: File::create(tmp)   ← cùng đường dẫn, cắt sạch lẫn nhau
A: ghi 55 MB           B: ghi 55 MB           ← vào cùng một tệp
A: rename(tmp → bin)                          ← xuất bản tệp TRỘN LẪN
```

Nên hậu quả không chỉ là mất một lượt quét mà có thể là **cache hỏng**, phải quét lại từ đầu.

Sửa: tên tệp tạm mang `std::process::id()`. Ba chỗ, không chỉ một —
[persist.rs](../src-tauri/src/index/persist.rs) (`index.bin`),
[enrich.rs](../src-tauri/src/media/enrich.rs) (`metadata.bin`),
[elevate.rs](../src-tauri/src/ipc/elevate.rs) (tệp tiến độ). Kẻ `rename` sau vẫn thắng và vẫn mất
một lượt, nhưng thua một lượt thì lượt sau bù được, còn cache hỏng thì không.

### Kiểm chứng — và một bài kiểm thử xanh vô nghĩa nữa

Phá mã ba phép:

| Phá cái gì | Kết quả |
|---|---|
| Tên tạm dùng chung trở lại | **1 ca đỏ** ✅ |
| Vừa cập nhật vẫn ẩn cửa sổ | **1 ca đỏ** ✅ |
| Mốc không bị xoá sau khi đọc | **sống sót** ❌ → đã viết bài mới |

Phép thứ ba lộ ra rằng phần "đọc là xoá" — chỗ nguy hiểm nhất của module, vì hỏng thì cửa sổ bật
lên **mãi mãi** ở mọi lần đăng nhập — không có bài nào canh. Đã tách `doc_va_xoa()` ra để kiểm thử
với tệp thật, và bài mới bắt được ngay.

Đây là lần thứ ba trong dự án một bài kiểm thử của tôi xanh mà không canh gì, và cả ba lần đều chỉ
lộ ra khi phá mã.

### Vòng kiểm

`cargo test` **255 pass** (249 + 6) · clippy 0 · fmt sạch · `npm run check` 0 lỗi/123 tệp ·
`npm test` **112 pass** · `cargo build` và `npm run build` đều sạch.

### Chưa kiểm được trên máy thật

Đường cập nhật đầy đủ (v1.0.4 → bấm nút → cài → khởi động lại → cửa sổ hiện) chỉ kiểm được sau khi
bản phát hành đã lên GitHub. Đó là phép thử cuối, và nó cần một máy đang chạy v1.0.4 thật.

## P40 — Việc A: kết quả trùng lặp không còn trỏ vào tệp sai

Lỗi 4.1 của [DE-XUAT-TRUNG-LAP.md](../DE-XUAT-TRUNG-LAP.md), và là lỗi nặng nhất trong tài liệu đó.
Đã kiểm chứng lại từng mắt xích bằng mã trước khi sửa:

| Mắt xích | Bằng chứng |
|---|---|
| `entries` là **vị trí**, không phải đường dẫn | [dupes.rs:63](../src-tauri/src/media/dupes.rs#L63) |
| Vị trí không sống sót qua dựng lại chỉ mục | [update.rs:142](../src-tauri/src/index/update.rs#L142) — *"positions are not preserved — they cannot be"* |
| `dupe_groups` tra vị trí cũ trên snapshot **mới** | `let index = state.snapshot()` |
| `DupeService` không có chỗ nào đặt lại | tạo một lần ở `lib.rs:144` |

Hệ quả: quét lúc 9:00, chỉ mục nạp lại lúc 10:25, quay lại lúc 11:00 thì mỗi nhóm hiện tên và đường
dẫn của **tệp khác** — im lặng, không cảnh báo. Với màn hình mà bước tiếp theo là **xoá tệp**, đây
là kiểu hỏng đắt nhất có thể có.

**Và chính v1.0.8 làm nó nặng hẳn lên.** Trước đó chỉ mục chỉ dựng lại khi có người tự bấm quét; từ
v1.0.8 lịch quét NAS dựng lại **hai lần mỗi ngày trên mọi máy**.

### Sửa hẳn gốc, không phải vá cảnh báo

Định làm bản vá epoch nửa ngày (phát hiện lệch thì báo "kết quả đã cũ"), nhưng đọc mã thì thấy
`start()` **đã nhận sẵn `Arc<Index>`** — chỉ là thả nó sau khi quét xong. Giữ lại là sửa hẳn gốc,
không đắt hơn, và tốt hơn hẳn: kết quả vẫn dùng được thay vì bị vứt.

`DupeService` nay giữ `snapshot: Arc<Mutex<Option<Arc<Index>>>>` và `epoch` của chính lượt quét.
`dupe_groups` phân giải bằng snapshot đó và **không còn nhận `AppState`** — clippy báo tham số thừa,
đúng là bằng chứng phụ thuộc đã cắt hẳn.

Mắt xích thứ tư dễ sót: giao diện dựng URL ảnh thu nhỏ bằng `thumbUrl(epoch, index)` với `epoch`
**hiện tại**, nên ảnh cũng của tệp khác. Nay `DupeGroupView` mang theo `epoch` của lượt quét và mỗi
hàng dùng epoch của nhóm mình.

### Kiểm chứng

| Phá cái gì | Kết quả |
|---|---|
| Không giữ snapshot nữa (đúng lỗi gốc) | **1 ca đỏ** ✅ |
| Giữ snapshot nhưng epoch lấy sai | **1 ca đỏ** ✅ |

Bài kiểm thử dựng chỉ mục thật qua `index_over()`, chạy trọn một lượt quét, rồi khẳng định
`Arc::ptr_eq` với chỉ mục ban đầu — không phải "một bản giống nó", mà **đúng nó**.

### Vòng kiểm

`cargo test` **256 pass** (255 + 1) · clippy 0 · fmt sạch · `npm run check` 0 lỗi/123 tệp ·
`npm test` 112 pass · cả hai build sạch.

Hai tệp kiểm thử tích hợp (`dupes_real.rs`, `dupes_cancel_real.rs`) cập nhật theo chữ ký mới.

## P41 — Hỏi phạm vi trước khi quét trùng lặp, và thời gian còn lại

Yêu cầu của người dùng: bấm nút Trùng lặp thì hỏi "có quét cả ổ NAS không?", kèm số liệu để
quyết, và hiện thời gian còn lại trong lúc quét.

### Vì sao phải hỏi

Cái giá của việc quét NAS **không đổ lên máy người bấm nút** — nó đổ lên chính NAS mà cả studio
đang dùng để làm việc. 20–40 máy cùng quét là 20–40 luồng đọc ngẫu nhiên trên cùng vài ổ đĩa.

Không đặt mặc định im lặng theo hướng nào: chọn sẵn "có" thì người chỉ muốn dọn ổ C: phải chờ NAS;
chọn sẵn "không" thì người muốn dọn NAS tưởng app bỏ sót tệp.

### Con số hiện ra là con số ĐẾM ĐƯỢC

Tầng 1 của quét trùng — gom theo dung lượng — **không đọc đĩa một byte nào**: chỉ mục đã giữ sẵn
mọi dung lượng. Nên đếm chính xác "bao nhiêu tệp phải mở trên ổ trong máy, bao nhiêu trên NAS" là
việc vài mili giây, làm được **trước** khi hỏi.

Module riêng [dupescope.rs](../src-tauri/src/media/dupescope.rs) lặp lại đúng phép lọc của tầng 1
và tách theo loại ổ. Nó nhận danh sách chữ ổ mạng đang gắn — ổ ánh xạ trông y hệt đĩa trong máy
trong chỉ mục (`Y:\…`), nên thiếu danh sách này thì mọi ổ NAS bị đếm nhầm và câu hỏi **không bao
giờ hiện ra**. Có ca kiểm thử canh đúng chỗ đó.

### Điều CỐ Ý không làm: đoán số phút trong hộp thoại

Hộp thoại nói **số tệp phải mở**, không nói số phút. Không có phép đo nào cho mã hiện tại trên thư
viện hiện tại — con số 584 giây trong tài liệu cũ đo ngày 24/8 trên ổ khác, bằng mã khác, trên chỉ
mục còn chứa 70.461 tệp đã bị xoá.

Hứa "khoảng 30 phút" rồi chạy 8 phút hoặc 50 phút thì lần sau không ai tin con số nào nữa.

### Thời gian còn lại: tính từ tốc độ THẬT của chính lượt quét đó

`DupeProgress.eta_seconds` tính từ `hashed / elapsed` của chính máy đang chạy. Ba chốt chặn:

| Chốt | Vì sao |
|---|---|
| Im lặng cho tới khi mở đủ **200 tệp** | Tốc độ vài tệp đầu là nhiễu (cache lạnh, luồng đang khởi động); con số nhảy từ "2 phút" lên "40 phút" rồi xuống "5 phút" tệ hơn không hiện gì |
| Chặn trên **24 giờ** | "Còn 340 ngày" gần như luôn là dấu hiệu ổ vừa rớt, không phải ước lượng |
| `None` khi `hashed >= total` hoặc thời gian ≤ 0 | Không ra số âm, không chia cho 0 |

### Kiểm chứng

Backend 13 ca mới. Frontend 10 ca, phá mã bốn phép — **cả bốn đều bị bắt**:

| Phá cái gì | Kết quả |
|---|---|
| Không hỏi, quét thẳng cả NAS | **4 ca đỏ** ✅ |
| Hỏi cả khi không có ổ mạng (hộp thoại thừa) | **1 ca đỏ** ✅ |
| Hỏi thất bại thì quét cả NAS | **2 ca đỏ** ✅ |
| "Để sau" mà vẫn quét | **1 ca đỏ** ✅ |

Phép đầu là phép đáng giá nhất: nó canh đúng điều khiến tính năng này tồn tại — **không đọc một
byte nào của NAS trước khi người dùng đồng ý**.

### Vòng kiểm

`cargo test` **269 pass** (256 + 13) · clippy 0 · fmt sạch · `npm run check` 0 lỗi/124 tệp ·
`npm test` **122 pass** (112 + 10) · cả hai build sạch.

### Chưa làm, có chủ đích

Lựa chọn phạm vi **không được nhớ** qua các lần. Tài liệu đề xuất lưu vào `prefs.ts`, nhưng một
lựa chọn ẩn quyết định có đọc NAS hay không là đúng loại bẫy đã gặp với chip lọc Video: người dùng
quên mình đã chọn gì rồi kết luận app hỏng. Hỏi lại mỗi lần rẻ hơn nhiều so với một lựa chọn vô
hình — và câu hỏi chỉ hiện khi thật sự có ổ mạng.

### P41b — Hai lỗi người dùng bắt được khi thử

**Lỗi 1 — bấm quét lại trong lúc lượt cũ đang dừng thì không có gì xảy ra.**

Kịch bản: bấm Trùng lặp → chọn phạm vi → đổi ý bấm lại để huỷ → bấm lần nữa để quét.

Gốc: `DupeService::cancel()` chỉ **giương cờ** rồi trả về ngay — luồng quét vẫn kẹt giữa một lần
mở tệp, mà trên NAS một lần mở có thể treo hàng chục giây. Trong khoảng đó `start()` gặp
`running == true` và từ chối, nhưng giao diện không phân biệt được "chưa quét" với "đang dừng dở",
nên nó dựng lại như sắp chạy và trông như app hỏng.

Sửa: `DupeProgress` có thêm cờ `stopping` (`running && stop`). Giao diện theo dõi thay vì gọi một
lệnh chắc chắn bị từ chối, và nói rõ **"Đang dừng lượt quét…"** ở cả hai chỗ hiển thị.

**Lỗi 2 — nút "Để sau" không đúng nghĩa.**

Người bấm nút đó muốn thoát hẳn khỏi việc tìm trùng lặp, nhưng "Để sau" gợi ý một việc còn treo —
mà không có việc nào treo cả. Đổi thành **"Huỷ"**, và giờ nó đóng hẳn chế độ trùng lặp qua
`onclose`: nút Trùng lặp trên thanh công cụ cũng tắt sáng, màn hình về đúng trạng thái ban đầu.

**Một bài kiểm thử xanh vô nghĩa nữa — lần thứ tư.**

Ca đầu tiên viết cho lỗi 1 chỉ khẳng định "không mở lượt mới". Phá mã thì nó **vẫn xanh**: khi
`stopping` thì `running` cũng true, nên nhánh `running` cũ cũng chặn được lượt mới. Bỏ hẳn xử lý
`stopping` mà bài không đỏ.

Thứ chỉ nhánh mới làm được là **nói ra** rằng đang dừng — và đó mới là phần người dùng thiếu. Viết
lại để canh đúng điều đó, phá mã lần hai thì đỏ ngay.

| Phá cái gì | Kết quả |
|---|---|
| Bỏ hiện "Đang dừng" ở cả hai chỗ | **1 ca đỏ** ✅ |
| "Huỷ" chỉ đóng hộp thoại, không đóng chế độ | **1 ca đỏ** ✅ |
| Nút vẫn tên "Để sau" | **1 ca đỏ** ✅ |

Vòng kiểm: `cargo test` **269** · clippy 0 · fmt sạch · `npm run check` 0 lỗi/124 tệp ·
`npm test` **124 pass**.

## P42 — Số liệu thật, và hiện kết quả trùng lặp dần theo giá trị

### Bước 0: lần đầu có số liệu thật, và nó đảo ngược ba giả định

Chỉ mục trên máy này là chỉ mục **thật của studio** — 54 MB, 410.581 tệp — không phải 767 KB như
[DE-XUAT-TRUNG-LAP.md](../DE-XUAT-TRUNG-LAP.md) tưởng. Nên phép đo bước 0 chạy được ngay, miễn phí
(không đọc đĩa một byte), trong **2,5 giây**.

```
=== ỨNG VIÊN TẦNG 1: 197.301 tệp / 410.581 trong chỉ mục ===
  trên ổ mạng : 160.982 (82%)
  trên đĩa máy:  36.319 (18%)
```

| Ổ | Loại | Tệp | Tiềm năng thu hồi |
|---|---|---|---|
| `D:` | máy | 36.200 | **2.204 GB** |
| `F:` | mạng | 45.294 | 1.059 GB |
| `Y:` | mạng | 80.162 | 543 GB |
| `H:` | mạng | 22.863 | 123 GB |
| `Z:` | mạng | 12.663 | 14 GB |

| Dải dung lượng | Tệp | % số tệp | Tiềm năng | % giá trị |
|---|---|---|---|---|
| 64K–1M | 68.994 | **35,0%** | 13,4 GB | **0,4%** |
| 1M–4M | 32.778 | 16,6% | 46,7 GB | 1,2% |
| 4M–16M | 60.962 | 30,9% | 360,4 GB | 9,6% |
| 16M–64M | 24.355 | 12,3% | 452,2 GB | 12,1% |
| 64M–256M | 6.872 | 3,5% | 530,5 GB | 14,2% |
| **≥256M** | 3.340 | **1,7%** | **2.543,5 GB** | **67,9%** |

**Ba giả định bị bác bỏ:**

1. **82% công việc nằm trên NAS** — hộp thoại hỏi phạm vi (P41) hoá ra là việc đáng giá nhất đã
   làm: chọn "chỉ ổ trong máy" cắt 82% khối lượng ngay. Và ổ `D:` trong máy lại có tiềm năng lớn
   nhất (2,2 TB) với chỉ 18% công sức.
2. **Khoá (size, mtime) không dùng được** — chỉ **2,7%** cặp cùng dung lượng có cùng thời gian sửa.
   Đây cũng là câu trả lời cho câu người dùng nói không biết: **CapCut không giữ nguyên mtime khi
   sao chép**. Việc G trong tài liệu: bỏ.
3. **Không có hardlink nào** (0 cặp cùng ổ+FRN). Phần đó cũng bỏ được.

### Hiện kết quả dần theo giá trị

Tệp ≥256 MB chỉ 1,7% số lượng nhưng **68% giá trị**. Nên tầng 2 nay xếp các lớp dung lượng theo
**tiềm năng thu hồi giảm dần** (`size × (số tệp − 1)` — cận trên chính xác của mọi nhóm trong lớp),
chia thành đợt 400 tệp, và **công bố sau mỗi đợt**.

Vì lớp được xử lý theo giá trị giảm dần, nhóm đã công bố không bao giờ bị đẩy xuống bởi nhóm tìm
sau — **thứ hạng của nó là chung cuộc**. Tổng thời gian không đổi, nhưng thời gian phải *chờ* để
thấy nhóm đáng giá nhất giảm từ hàng chục phút xuống vài giây.

Kèm theo: **huỷ giữa chừng nay GIỮ phần đã chốt** thay vì vứt sạch. Vì phần đã chốt chính là những
lớp giá trị nhất, vứt đi là bắt người dùng trả lại từ đầu cái họ vừa chờ xong.

### Ba lỗi bộ kiểm thử bắt được, hai trong số đó là lỗi của chính tôi

**Lỗi mã:** đợt bị huỷ `break` **trước khi** `publish` — vân tay của đợt đó bị vứt dù đã băm xong.
Bài kiểm thử bắt ngay lần chạy đầu. Sửa: công bố trước, kiểm cờ dừng sau.

**Lỗi bài kiểm thử #1 (xanh vô nghĩa — lần thứ năm):** ca "nhóm đáng giá nhất ra trước" chỉ canh
thứ tự *sắp xếp cuối*, mà `groups.sort_unstable_by` vẫn chạy — đảo chiều sắp lớp mà bài vẫn xanh.
Thứ tự **xử lý** mới là thứ quyết định người dùng chờ bao lâu, và nó chỉ quan sát được qua kết quả
công bố giữa chừng. Viết ca mới, phá mã lần hai thì đỏ.

**Lỗi bài kiểm thử #2:** ca mới đó đỏ ngay, nhưng vì **giả định sai của tôi**, không phải mã sai:
250 tệp cùng 70 KB không phải 250 lớp mà là *một* lớp 500 tệp, tiềm năng 34 MB — lớn hơn lớp "to"
2 MB, nên mã sắp đúng. Sửa cho mỗi lớp một dung lượng riêng.

| Phá cái gì | Kết quả |
|---|---|
| Đảo chiều sắp lớp theo tiềm năng | **1 ca đỏ** ✅ |
| Không công bố dần (chỉ công bố cuối) | **1 ca đỏ** ✅ |
| Huỷ vẫn vứt sạch kết quả | **1 ca đỏ** ✅ |

### Giao diện

Lấy kết quả **trong lúc quét**, mỗi khi `stat.groups` đổi. Và sửa một lỗi tài liệu đã cảnh báo:
`cursor` nhảy về đầu mỗi khi `rows` đổi — vô hại khi kết quả chỉ hiện một lần, nhưng nay `rows`
dài ra mỗi 400 ms nên con trỏ nhảy liên tục. Nay chỉ đưa về đầu khi danh sách **ngắn đi**, tức
lượt quét mới bắt đầu.

### Vòng kiểm

`cargo test` **272 pass** (269 + 3) · clippy 0 · fmt sạch · `npm run check` 0 lỗi/124 tệp ·
`npm test` 124 pass · cả hai build sạch.

### Còn lại, xếp theo số liệu vừa đo

| Việc | Lợi | Số liệu ủng hộ |
|---|---|---|
| Nâng `SMALL_FILE_LIMIT` lên ~1 MB | cắt ~35% khối lượng | 64K–1M: 35% số tệp, 0,4% giá trị |
| Vân tay bền (việc D) | lượt sau vài giây | không giúp lần đầu |
| Pool I/O theo ổ (việc C) | chưa đo, có thể vài lần | cần bước đo 1 và 2 |

## P43 — Việc D: vân tay bền giữa các lần chạy

Yêu cầu của người dùng: khi bấm quét NAS thì phải nhanh nhất có thể. Đây là việc D của
[DE-XUAT-TRUNG-LAP.md](../DE-XUAT-TRUNG-LAP.md), và số liệu bước 0 cho thấy nó là việc đáng nhất
cho ổ mạng: **160.982 trên 197.301 ứng viên (82%) nằm trên NAS**, mỗi lần mở tệp ở đó tốn ~66 ms
chỉ để lấy byte đầu.

### Cách làm

[dupestore.rs](../src-tauri/src/media/dupestore.rs) — kho vân tay khoá theo đường dẫn, kèm `size`
và `mtime` lúc đọc. Lượt sau: tệp còn nguyên cả hai thì **không mở lại**.

Mượn phần bền hoá của `enrich::Store` (magic + `SCHEMA_VERSION`, tệp tạm mang PID rồi `rename`),
nhưng **không** mượn hai điều — cả hai đều là bài học từ chính mã cũ:

**Không dùng `DefaultHasher`.** `enrich::path_key` dùng nó, mà tài liệu `std` **không cam kết** nó
cho cùng kết quả giữa các bản Rust. Một lần nâng trình biên dịch có thể làm mọi khoá lệch đi và cả
kho thành vô dụng — im lặng. Ở đây dùng FNV-1a viết tay ngay trong tệp, và có ca kiểm thử neo giá
trị để nếu ai đó đổi thì bài đỏ ngay.

**Không lưu mỗi 500 tệp.** `save` tuần tự hoá cả map; với 197 nghìn ứng viên đó là hàng trăm lần
ghi lại vài chục MB. Lưu một lần khi xong — kể cả khi bị huỷ, vì một lượt huỷ vẫn đã đọc xong hàng
nghìn tệp.

**Tỉa khoá không còn dùng**, thứ `enrich::Store` không bao giờ làm (và đó là lý do `metadata.bin`
lớn dần theo tệp đã xoá). Chỉ tỉa khi quét **trọn vẹn cả phạm vi**: một lượt bị huỷ, hay một lượt
chỉ quét ổ trong máy, không nhìn thấy hết tệp — tỉa theo nó là vứt vân tay của tệp vẫn còn nguyên.

### Rủi ro, và vì sao tầng 3 thành bắt buộc

Sao chép **giữ nguyên** mtime không phải rủi ro: khoá là đường dẫn, mỗi bản sao có mục riêng.

Rủi ro thật là **sửa tại chỗ mà giữ nguyên cả size lẫn mtime** — `mtime` có độ phân giải một giây,
và vài công cụ (`exiftool -P`) cố ý giữ nguyên. Khi đó vân tay cũ bị tin nhầm và hai tệp khác nội
dung bị báo là trùng. Đó là lý do **tầng 3 xác minh trước khi xoá không còn là tuỳ chọn**.

### Kiểm chứng

Phá mã bốn phép, **cả bốn bị bắt**:

| Phá cái gì | Kết quả |
|---|---|
| Bỏ qua `mtime` — tin vân tay cũ của tệp đã đổi | **2 ca đỏ** ✅ |
| Bỏ qua `size` | **1 ca đỏ** ✅ |
| Không chuyển chữ thường (Windows coi là một tệp) | **2 ca đỏ** ✅ |
| Không tỉa khoá cũ | **1 ca đỏ** ✅ |

Phép đầu là phép quan trọng nhất: tin vân tay cũ của một tệp đã đổi nghĩa là **báo trùng sai**, mà
bước tiếp theo của người dùng là xoá.

Ca đo cố ý đếm **số lần mở tệp**, không đếm `hashed` — `hashed` tăng cho cả tệp lấy từ kho lẫn tệp
phải đọc, nên nó không phân biệt được. Tài liệu đã cảnh báo đúng chỗ này.

### Vòng kiểm

`cargo test` **282 pass** (272 + 10) · clippy 0 · fmt sạch.

## P44 — Quét trùng lặp ổ trong máy lúc máy rảnh

Ý tưởng của người dùng: người ta tới công ty lúc 8 giờ, mở máy, pha cà phê và đọc email — ứng dụng
đã chạy nền nhưng chưa ai dùng tới. Đó là khoảng thời gian rẻ nhất trong ngày để đọc đĩa.

### Số đo thật, và nó đắt gấp đôi tài liệu ước tính

Chạy lượt quét thật trên thư viện studio bằng chính mã hiện tại:

| | |
|---|---|
| Tốc độ đo được | **45 tệp/giây** |
| Quét trọn 197.301 ứng viên | **1,2 giờ** |
| Tài liệu cũ ước tính | ~30 phút |
| Riêng ổ trong máy (36.319 tệp) | **13 phút** |

Tài liệu nói rõ 30 phút là *ngoại suy*, và giờ đã có số thật: **đắt hơn hai lần**. Đây cũng là con
số biện minh cho cả tính năng — 13 phút chạy nền lúc 8 giờ sáng đổi lấy việc bấm nút lúc 10 giờ là
có kết quả ngay.

### Bảy ràng buộc, tất cả do người dùng duyệt

| Ràng buộc | Cách làm |
|---|---|
| Chỉ ổ trong máy, **không bao giờ** tự quét NAS | `DupeScope::LocalOnly`, có ca kiểm thử đọc thẳng mã nguồn để canh |
| Chờ enrichment xong | Hỏi `EnrichService::status().running` |
| Chờ ~10 phút không có tìm kiếm | Theo dõi `AppState::generation()` |
| 2 luồng, `BELOW_NORMAL` | Dùng lại `DupeService` — cùng mô hình enrichment đã chứng minh |
| Dừng ngay khi người dùng gõ tìm kiếm | Vòng theo dõi kiểm `generation` mỗi 2 giây, đổi thì `cancel()` |
| Màn hình nói "kết quả từ HH:MM" | `DupeProgress.started_unix` |
| Có cách tắt | Ô tích trong hộp thoại phạm vi |

**82% ứng viên nằm trên NAS**, nên ràng buộc đầu là ràng buộc quan trọng nhất: 20–40 máy cùng đọc
NAS mỗi sáng là cái giá đổ lên chính NAS mà cả studio đang dùng để làm việc. Ca kiểm thử canh nó
đọc thẳng mã nguồn của module — vì phạm vi được truyền vào một lời gọi bên trong vòng lặp, không có
cách nào quan sát mà không dựng cả ứng dụng.

Ô tắt đặt **trong hộp thoại phạm vi**, không phải trong một màn cài đặt riêng: đó là lúc người dùng
đang nghĩ về việc quét. Và dòng chữ nói rõ *"Không bao giờ tự đọc ổ mạng"* — người đọc nó vừa thấy
con số "đọc qua mạng nên chậm hơn nhiều" ở ngay trên, nên phải chặn ngay cách hiểu rằng máy đang âm
thầm đọc NAS.

### Kiểm chứng

Phá mã tám phép, **cả tám bị bắt**:

| Phá cái gì | Kết quả |
|---|---|
| **Đổi phạm vi thành cả ổ mạng** | **1 ca đỏ** ✅ |
| Bỏ qua cờ tắt | 2 ca đỏ ✅ |
| Không chờ enrichment xong | 1 ca đỏ ✅ |
| Quét ngay, không chờ máy yên | 1 ca đỏ ✅ |
| Chen vào lượt quét đang chạy | 1 ca đỏ ✅ |
| Bỏ dòng "không bao giờ tự đọc ổ mạng" | 1 ca đỏ ✅ |
| Bỏ tích mà không gửi lệnh xuống backend | 1 ca đỏ ✅ |
| Không hiện mốc quét | 1 ca đỏ ✅ |

Ca canh ràng buộc NAS suýt tự làm mình đỏ: nó nhắc tên biến thể `Everything` trong chính chú thích
của mình. Sửa bằng cách chỉ soi phần trước khối `mod tests`.

### Vòng kiểm

`cargo test` **291 pass** · clippy 0 · fmt sạch · `npm run check` 0 lỗi/124 tệp ·
`npm test` **127 pass**.

### Kho vân tay chạy thật

Lượt quét bị cắt giữa chừng vẫn ghi được `dupes.bin` đúng định dạng (magic `MFDUPE01`, version 1) —
đúng thiết kế: một lượt huỷ vẫn đã đọc xong hàng nghìn tệp, vứt phần đó là bắt lượt sau đọc lại.

## P45 — Việc H: xác minh trọn nội dung trước khi xoá

Việc H của [DE-XUAT-TRUNG-LAP.md](../DE-XUAT-TRUNG-LAP.md). Tài liệu xếp nó vào nhóm "làm sau",
nhưng hai việc vừa xong biến nó thành **bắt buộc**:

* Tầng 2 chỉ đối chiếu dung lượng và **hai đầu** tệp, nên hai video khác nhau ở giữa vẫn bị gom
  chung. Đúng để *tìm ứng viên*, sai hoàn toàn nếu lấy làm căn cứ *xoá*.
* Kho vân tay bền (P43) thêm một rủi ro nữa: tệp bị sửa tại chỗ mà giữ nguyên cả `size` lẫn `mtime`
  sẽ được tin theo vân tay cũ. Hiếm, nhưng có thật.
* Và P42 khiến kết quả hiện ra sau vài giây thay vì hàng chục phút — nên người dùng bắt đầu hành
  động sớm hơn nhiều.

### Ghép từ nhánh cũ

`verify.rs` ở `backup/edit-v1.0.6` còn nguyên và viết kỹ: hash trọn từng byte, đệm 1 MiB không kéo
cả tệp vào RAM, chạy **tuần tự có chủ ý** (một nhóm hiếm khi quá vài tệp, và các bản sao thường
nằm cùng một ổ — hai luồng cùng đọc một đĩa cơ chỉ đổi tuần tự lấy tiếng lạch cạch).

Ghép nguyên, kèm lệnh IPC `verify_dupe_group` chạy trên pool blocking.

Ba ca kiểm thử của nó cũng ghép nguyên, và ca đầu chính là ca chứng minh tầng 3 cần tồn tại: bốn
tệp 4 KiB **giống hệt hai đầu**, một tệp khác đúng một byte ở giữa bụng — tầng 2 gom cả bốn, tầng 3
tách đúng kẻ giả ra.

### Giao diện

Nút **Xác minh** trên tiêu đề từng nhóm, không phải một nút chung: chỉ đọc trọn nhóm mà người dùng
sắp hành động — vài giây cho một nhóm, thay vì hàng giờ cho cả thư viện mà tuyệt đại đa số không ai
đụng tới.

Bốn trạng thái, và ranh giới giữa chúng là chỗ dễ sai nhất:

| Kết quả | Màn hình nói |
|---|---|
| Một cụm, đọc được hết | ✓ trùng thật |
| Nhiều cụm | ⚠ **có tệp khác nội dung** |
| Có tệp không đọc được | không đọc được hết |

Trường hợp thứ ba **không khẳng định gì cả** — không đọc được không phải là "khác nội dung", cũng
không phải "trùng thật". Nói bừa một trong hai đều là khẳng định điều chưa xác minh.

Trạng thái giữ trong một `Map` khoá theo `(dung lượng, vị trí tệp đầu)`, không gắn vào `DupeGroup`:
nhóm đến từ backend và bị thay mới mỗi 400 ms trong lúc quét, nên gắn vào chúng là mất ngay nhịp
sau.

### Kiểm chứng

| Phá cái gì | Kết quả |
|---|---|
| **Nhiều cụm vẫn báo "trùng thật"** | **2 ca đỏ** ✅ |
| Bỏ qua danh sách không đọc được | 1 ca đỏ ✅ |
| Gửi sai danh sách đường dẫn của nhóm | 1 ca đỏ ✅ |

Phép đầu là phép quan trọng nhất trong cả đợt việc: báo "trùng thật" khi thực ra không phải nghĩa
là người dùng xoá mất một tệp không có bản sao nào.

### Vòng kiểm

`cargo test` **294 pass** (291 + 3) · clippy 0 · fmt sạch · `npm run check` 0 lỗi/124 tệp ·
`npm test` **132 pass** (127 + 5).

## P46 — Việc F: ổ rớt thì không nói dối

Lỗi 4.3 của [DE-XUAT-TRUNG-LAP.md](../DE-XUAT-TRUNG-LAP.md). Tệp không mở được bị bỏ **lặng lẽ**:
NAS rớt giữa lượt quét thì mọi ứng viên trên đó biến mất khỏi kết quả, `completed` vẫn thành
`true`, và màn hình nói *"Không tìm thấy tệp trùng lặp nào"* — một khẳng định sai.

Với **82% ứng viên nằm trên ổ mạng**, chuyện này không hiếm. Và đây là kiểu nói dối tệ nhất: nó
nghe như một câu trả lời dứt khoát.

### Hai đường, hai mức chắc chắn

**Đếm tệp không mở được.** `Counters::unreadable` thay cho việc `filter_map` bỏ lặng. Một tệp không
mở được có thể là tệp vừa bị xoá, hoặc cả một ổ vừa rớt — không phân biệt được, nhưng đếm được.

**Loại ổ đã rớt, TRƯỚC khi mở tệp nào.** Kiểm bằng `GetLogicalDrives` qua `list_volumes`, không mở
thử một tệp: một lần mở trên share đã chết có thể treo tới hết SMB SessTimeout (mặc định 60 giây),
và cờ dừng được kiểm **trước** khi mở nên không ngắt được nó. Cái giá của phép kiểm gần bằng
không; cái nó tránh là mấy chục luồng cùng treo một phút.

Đường thứ hai nói được **TÊN** ổ, và đó là câu hữu ích hơn hẳn: *"Thiếu Y: — ổ không còn kết nối"*
cho người dùng biết phải nối lại ổ nào, còn *"thiếu 160.982 tệp"* thì không.

### Dọn nợ kỹ thuật đi kèm

`find_duplicates` đã có 7 tham số và việc F cần thêm 2 nữa — clippy cảnh báo đúng lúc. Gom thành
`struct Counters`: một danh sách chín tham số thì ai gọi cũng dễ đặt nhầm thứ tự hai `&AtomicUsize`
cạnh nhau, và **trình biên dịch không bắt được** vì chúng cùng kiểu.

Lần thử gom đầu tiên hỏng: tôi dùng regex thay hàng loạt và nó nuốt luôn các bộ đếm mà từng bài
kiểm thử cần theo dõi. Đã `git checkout` lùi hẳn về bản commit rồi làm lại thủ công theo bốn dạng
lời gọi — mất thêm một lượt, nhưng một bản refactor "gần đúng" ở chỗ này thì mọi phép đo sau đó đều
sai mà không ai biết.

### Bộ kiểm thử cũ bắt được một lỗi thật của tôi

Bảy bài ở `t5` và `t6` đỏ sau khi thêm hai trường mới: mock cũ không có `unreadable` và
`droppedDrives`, nên `thieuTep()` đọc `.length` của `undefined` và làm **trắng cả màn hình**.

Đây là lỗi thật, không phải bài kiểm thử lỗi thời: một bản backend cũ hơn — hoặc một lượt cập nhật
dở dang, khi tệp `.exe` đã đổi mà cửa sổ chưa nạp lại — trả về đúng hình dạng đó. Sửa ở **mã**
(`?? 0` và `?? []`), không phải ở mock.

### Kiểm chứng

| Phá cái gì | Kết quả |
|---|---|
| Quay lại bỏ lặng tệp không mở được | 1 ca đỏ ✅ |
| Không loại ổ đã rớt (mở thử rồi treo) | 1 ca đỏ ✅ |
| Loại ổ nhưng không nói tên | 1 ca đỏ ✅ |
| **Quay lại nói dối "không tìm thấy" dù thiếu tệp** | **3 ca đỏ** ✅ |
| Bỏ qua ổ đã rớt, chỉ đếm tệp | 1 ca đỏ ✅ |
| Không gọi tên ổ, chỉ nói số | 2 ca đỏ ✅ |

Có một ca canh chiều ngược lại: **quét đủ thì vẫn nói "không tìm thấy" như cũ**. Bản sửa không được
biến một câu trả lời đúng thành một lời cảnh báo thừa — thư viện không có gì trùng lặp là một câu
trả lời thật.

### Vòng kiểm

`cargo test` **296 pass** (294 + 2) · clippy 0 · fmt sạch · `npm run check` 0 lỗi/124 tệp ·
`npm test` **136 pass** (132 + 4).

## P47 — Việc E, và vì sao chỉ làm một nửa

Tài liệu gọi cả hai hằng số là "việc E", nhưng chúng làm hai chuyện khác hẳn nhau — và đó chính là
chỗ gây nhầm lẫn:

| Hằng số | Quyết định | Đánh đổi |
|---|---|---|
| `SMALL_FILE_LIMIT` | **đọc thế nào** — trọn tệp hay hai đầu | **không có** |
| `MIN_INTERESTING_SIZE` | **đọc cái gì** — tệp nào bị bỏ hẳn | bỏ tệp thật |

### Đã làm: nâng `SMALL_FILE_LIMIT` 128 KB → 1 MB

Đọc hai đầu là mở + đọc + nhảy + đọc. Đọc trọn là mở + đọc. Trên NAS đo được: mở và lấy byte đầu
tiên tốn **66 ms**, đọc thêm 1 MB chỉ tốn **18 ms**. Nên với tệp tới ~1 MB, bỏ được một lượt đọc
đáng giá hơn hẳn phần băng thông thêm vào.

Ngưỡng cũ nằm **dưới điểm hoà vốn**: mọi tệp từ 128 KB đến 1 MB đang trả giá cho một thao tác
chúng không cần — và dải đó có **68.994 ứng viên, 35% tổng số tệp phải mở**.

Thêm một lợi ích không tính trước: tầng 2 nay **chính xác hơn** cho mọi tệp dưới 1 MB, vì đọc trọn
thấy được khác biệt ở giữa mà đọc hai đầu bỏ sót.

Một bài kiểm thử cũ đỏ vì điều đó — bài `a_difference_in_the_middle_is_invisible_to_tier_two` dùng
tệp 400 KB. Không phải mã hỏng: bản sửa đã **thu hẹp đúng giới hạn mà bài đang mô tả**. Đổi bài
sang tệp 3 MB để nó nói về vùng mà giới hạn còn thật, và thêm một ca mới canh hành vi mới.

Thêm một chốt lúc biên dịch: `SMALL_FILE_LIMIT >= SAMPLE_BYTES * 2`. Hạ ngưỡng xuống dưới hai lần
mẫu thì nhánh "đọc trọn" lại đọc ít byte hơn nhánh "đọc hai đầu", và vân tay của hai tệp cùng kích
thước được tính theo hai cách khác nhau. Phá mã thử: nó chặn ngay lúc **biên dịch**, không đợi tới
lúc chạy.

### KHÔNG làm: nâng `MIN_INTERESTING_SIZE`

Tài liệu đề xuất nâng sàn lên ~4 MB để cắt 35% khối lượng. Số liệu bước 0 thoạt nhìn ủng hộ mạnh:
dải 64 KB–1 MB chiếm **35,0% số tệp** nhưng chỉ **0,4% giá trị**.

Nhưng P42 đã giải quyết xong vấn đề đó theo cách tốt hơn. Xếp các lớp theo tiềm năng giảm dần thì
tiến độ tích luỹ là:

| Đọc tới dải | % số tệp | % giá trị đã thấy |
|---|---|---|
| ≥256M | 1,7% | **64,4%** |
| 64M–256M | 5,2% | 77,9% |
| 16M–64M | **17,5%** | **89,3%** |
| 4M–16M | 48,4% | 98,5% |
| 64K–1M | 100% | 100% |

Sau khi đọc **17,5% số tệp**, người dùng đã thấy **89,3% giá trị** — rồi tự bấm dừng nếu đủ.

Nên nâng sàn giờ chỉ còn là **bỏ 0,4% giá trị để tiết kiệm thời gian mà người dùng đã có quyền
không trả**. Và cái mất không chỉ là dung lượng: 68.994 tệp đó là sticker, SFX ngắn, PNG nhân bản
hàng nghìn lần trong các dự án CapCut — ai đang dọn thư viện muốn thấy chúng, kể cả khi mỗi nhóm
chỉ vài trăm KB.

Đây là lần thứ hai trong dự án một đề xuất trong tài liệu bị chính số liệu bác bỏ (lần đầu: khoá
(size, mtime) ở việc G).

### Vòng kiểm

`cargo test` **297 pass** (296 + 1) · clippy 0 · fmt sạch · `npm run check` 0 lỗi/124 tệp ·
`npm test` 136 pass.

## P48 — Bước đo 1: các ổ chênh nhau tới 25 lần

Phép đo nền lạnh theo từng ổ, 1.500 tệp mỗi ổ, tuần tự.

| Ổ | Loại | Tệp/giây | Ngoại suy cho cả ổ |
|---|---|---|---|
| `D:` | máy | **30,3** | 20 phút |
| `F:` | mạng | 11,6 | 65 phút |
| `H:` | mạng | 9,0 | 42 phút |
| **`Y:`** | mạng | **1,2** | 1.100 phút |
| **`Z:`** | mạng | **1,1** | 188 phút |

`Y:` chậm hơn `D:` **25 lần** và chậm hơn `F:` **10 lần**.

### Đây là bằng chứng cho việc C, mạnh hơn dự kiến

Hiện rayon trộn ứng viên của mọi ổ vào **một hàng đợi chung**. Nên một lần mở trên `Y:` — mất
khoảng 0,8 giây — **giữ chân một luồng lẽ ra đang đọc `D:`** ở tốc độ nhanh gấp 25 lần. Với 12
luồng và 82% việc nằm trên NAS chậm, phần lớn luồng bị `Y:` và `Z:` chiếm.

Số liệu cũng chỉnh lại thiết kế tài liệu đề xuất. Tài liệu nói "mỗi ổ một pool riêng", nhưng các ổ
tự chia thành hai nhóm rõ rệt theo **máy chủ**: `F:`/`H:` trên `.214` nhanh gấp mười `Y:`/`Z:` trên
`.213`. Tách theo máy chủ đúng hơn tách theo chữ ổ.

### Hai điều bài đo này KHÔNG nói được

**Bước 2 (thứ tự HashMap so với FRN) cho `0,01×` — con số vô nghĩa.** Mẫu A chạy trong 0,2 giây,
tức Windows đã cache sẵn 1.500 tệp đó vì chúng vừa được đọc ở bước 1. Hai mẫu không cùng lạnh như
thiết kế. Không dùng kết quả này.

**Và cả bài đo tuần tự không trả lời được câu hỏi chính của việc C.** Lượt quét thật chạy song song
12 luồng; đo tuần tự cho biết *ổ nào chậm*, không cho biết *chia luồng riêng có giúp không*.

Phát hiện ra điều đó khi bài đã chạy 44 phút mà chỉ tốn 5 giây CPU — dấu hiệu rõ ràng là đang chờ
đĩa chứ không tính toán. Bài viết ra để đo tốc độ song song mà lại dùng vòng `for` tuần tự.

Nên có thêm **bước 2b**: cùng một ổ `Y:`, bốn mức luồng (1/4/8/16), mỗi mức một mẫu **rời nhau**
chưa ai chạm. Nó phân biệt được hai nguyên nhân:

* Tăng gần tuyến tính → chậm vì **độ trễ mỗi thao tác**, pool riêng theo ổ đáng làm.
* Gần như không đổi → chậm vì **băng thông máy chủ**, pool riêng không giúp gì.

Đây là lần thứ ba trong dự án cache Windows suýt làm hỏng một phép đo — hai lần trước ở `netsched`
(64 luồng "nhanh hơn 11 lần") và ở bước 2 trên.

## P49 — Bước đo 2b: việc C đáng làm, gấp mười sáu lần ngưỡng

Cùng ổ `Y:`, bốn mức luồng, mỗi mức một mẫu 250 tệp **rời nhau** chưa ai chạm.

| Luồng | Tệp/giây | So với 1 luồng |
|---|---|---|
| 1 | 0,83 | 1,0× |
| 4 | 7,43 | **9,0×** |
| 8 | 12,45 | **15,0×** |
| 16 | 20,47 | **24,7×** |

Ngưỡng tài liệu đặt ra để nhận việc C: **≥1,5×**. Đo được **24,7×**.

Tăng còn **siêu tuyến tính** ở mức đầu — 4 luồng cho 9× chứ không phải 4×. Dấu hiệu dứt khoát rằng
`Y:` chậm vì **độ trễ mỗi thao tác**, không phải băng thông máy chủ: phần lớn thời gian là chờ NAS
trả lời, nên nhiều yêu cầu cùng bay thì tổng thông lượng tăng gần theo số luồng.

Ước lượng lại cho cả ổ `Y:` (80.162 ứng viên):

| Luồng | Thời gian |
|---|---|
| 1 | 27 giờ |
| 8 | 1 giờ 47 |
| **16** | **65 phút** |

### Cái được lớn nhất không nằm ở `Y:`

Hiện rayon trộn mọi ổ vào một hàng đợi chung, nên **một lần mở trên `Y:` giữ chân một luồng lẽ ra
đang đọc `D:`** — ổ nhanh gấp 25 lần. Tách pool ra thì `D:` chạy hết tốc độ của nó.

### Hai chỗ số liệu chỉnh lại tài liệu

**"Ổ mạng 8 luồng mỗi máy chủ" là quá ít.** 16 luồng vẫn còn tăng gần tuyến tính, chưa chạm trần.
Cần đo thêm mức 24/32 để tìm điểm bão hoà.

**Tách theo MÁY CHỦ, không theo chữ ổ.** `Y:` và `Z:` cùng `.213` và cùng chậm (1,2 và 1,1 tệp mỗi
giây); `F:` và `H:` cùng `.214` và nhanh gấp mười. Giới hạn luồng theo máy chủ, nếu không thì hai ổ
trên cùng một NAS cộng lại thành gấp đôi tải mà máy chủ đó phải chịu.
