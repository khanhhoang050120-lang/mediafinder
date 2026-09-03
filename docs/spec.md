# LỖI CỦA ĐẶC TẢ — MediaFinder
> **Thuộc file này:** Code làm **đúng y** những gì đặc tả yêu cầu, nhưng chính yêu cầu đó sai. Loại này chỉ lộ ra khi chạy trên dữ liệu thật.
> **KHÔNG thuộc file này:** code làm sai so với đặc tả — đó là bug.
> Mục lục: [docs/README.md](./README.md) · [bug](./bug.md) · [config](./config.md) · [risk](./risk.md) · [perf](./perf.md) · [check](./check.md) · [issue](./issue.md) · [spec](./spec.md) · [test-log](./test-log.md) · [test-log-v2](./test-log-v2.md)

**Mức độ:** 🔴 Nặng (chặn / sai kết quả) · 🟠 Vừa (ảnh hưởng trải nghiệm) · 🟡 Nhẹ (khó chịu / công cụ) · ⚪ Rủi ro (chưa xảy ra) · ✅ Đã xong / không phải lỗi

**Trạng thái:** `MỞ` · `ĐANG SỬA` · `ĐÃ SỬA` · `WORKAROUND` · `CẦN XÁC MINH` · `CẦN QUYẾT ĐỊNH` · `KHÔNG SỬA` · `KHÔNG PHẢI LỖI`

**Cấp ID tiếp theo:** `SPEC-003`

## Bảng tổng hợp

| ID | Mức | Tiêu đề | GĐ | Trạng thái |
|----|-----|---------|----|-----------|
| [SPEC-001](#spec-001) | 🔴 | Đặc tả chỉ tìm trong tên file → vô dụng với dữ liệu thật | P2 | ĐÃ SỬA |
| [SPEC-002](#spec-002) | 🔴 | Bắt buộc khớp MỌI từ khoá → truy vấn dài trả về rỗng | P3 | ĐÃ SỬA |

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

---

## SPEC-002 🔴 — Bắt buộc khớp MỌI từ khoá → truy vấn dài trả về rỗng

**Giai đoạn:** P3 · **Trạng thái:** ĐÃ SỬA · **Ngày:** 2026-08-24

**Người dùng báo.** Tìm `The anglerfish` thì **ra** tệp cần tìm. Nhưng dán nguyên tiêu đề
`The anglerfish: The original approach to deep-sea fishing` thì **0 kết quả** — dù tệp đó nằm
ngay trong danh sách vừa tìm được.

Tức là: **gõ ít thì tìm ra, gõ đầy đủ hơn lại không.** Nghịch lý này là dấu hiệu của lỗi thiết kế.

**Tái hiện.** Đối chiếu từng từ khoá với tên tệp thật:

```
TÊN TỆP : ytsave_youtube_the-anglerfish-the-original-approach-to-_media_vqpmp9x-89o_001_1080p.mp4
TRUY VẤN: the anglerfish: the original approach to deep-sea fishing

  "the"          -> KHỚP
  "anglerfish:"  -> *** KHÔNG KHỚP ***
  "the"          -> KHỚP
  "original"     -> KHỚP
  "approach"     -> KHỚP
  "to"           -> KHỚP
  "deep-sea"     -> *** KHÔNG KHỚP ***
  "fishing"      -> *** KHÔNG KHỚP ***
```

**Hai nguyên nhân tách biệt.**

*Thứ nhất — dấu câu dính vào từ khoá.* Tách token chỉ theo khoảng trắng, nên `anglerfish:` giữ
nguyên dấu hai chấm. Tên tệp có `anglerfish-`, không có `anglerfish:`. **Từ đặc trưng nhất của cả
câu bị mất vì đúng một dấu hai chấm.** Tương tự, `deep-sea` là một token nên không khớp
`deep sea` hay `deep_sea`.

*Thứ hai — và đây mới là gốc rễ — tên tệp đã bị cắt cụt:*

```
Tiêu đề thật : The anglerfish: The original approach to deep-sea fishing
Tên trên đĩa : ...The-anglerfish-The-original-approach-to-_Media_VqPMP9X-89o_001_1080p.mp4
                                                         ↑ trình tải cắt ở đây
```

Ba chữ `deep`, `sea`, `fishing` **không hề tồn tại trong tên tệp**. Không thuật toán nào tìm được
chữ không có ở đó.

**Vì sao là lỗi đặc tả.** Đặc tả mục 3.3 quy định lọc bằng "kiểm tra chuỗi con chứa trong chuỗi
tên tệp", và triển khai bắt buộc **mọi** từ khoá phải khớp. Code làm đúng y yêu cầu. Nhưng yêu cầu
đó ngầm giả định **tên tệp là bản sao trung thực của thứ nó mô tả** — điều gần như không đúng với
tệp tải về: trình tải cắt tiêu đề, thay dấu câu, chèn ID và hậu tố của riêng nó.

**Cách sửa — ba phần.**

1. **Tách token theo mọi ký tự không phải chữ-số**, không chỉ khoảng trắng.
   `anglerfish:` → `anglerfish` · `deep-sea` → `deep` + `sea`.

2. **Tự lùi về khớp một phần** khi không có tệp nào khớp đủ. Xếp hạng theo **số từ khoá khớp
   được**, và chỉ giữ những tệp khớp **nhiều nhất** — không phải mọi tệp khớp kha khá.

3. **Nói rõ cho người dùng biết.** Kết quả một phần mà trông như khớp chính xác còn tệ hơn không
   có kết quả, vì người dùng sẽ ngừng tìm. Giao diện hiện băng thông báo *"Không có tệp nào khớp
   đủ 9 từ. Đang hiện các tệp khớp nhiều nhất — 6/9 từ"*, kèm huy hiệu `6/9` trên từng dòng.

**Ba ranh giới được đặt ra để không nới lỏng quá tay.**

| Ranh giới | Giá trị | Vì sao |
|---|---|---|
| Số từ tối thiểu để được nới lỏng | 3 | Một hai từ là truy vấn có chủ đích — người dùng biết rõ mình gõ gì. Âm thầm nới rộng sẽ trả lời một câu hỏi họ không hỏi. |
| Sàn số từ phải khớp | một nửa | Không có sàn này, truy vấn 9 từ sẽ trả về **mọi tệp chứa chữ "the"**. |
| Chỉ giữ nhóm khớp nhiều nhất | — | Đo trên thư viện thật: 2 tệp khớp 6/9, rồi **171 tệp khớp 5/9** toàn thứ không liên quan. Hai câu trả lời đúng bị chôn dưới gấp trăm lần rác. |

**Kết quả trên dữ liệu thật.** Truy vấn của người dùng giờ trả về đúng tệp cần tìm ở nhóm 6/9,
kèm thông báo rõ đây là khớp một phần.

**Bài học.** Không một unit test nào bắt được lỗi này, vì dữ liệu test do chính tôi nghĩ ra —
và tôi luôn đặt tên tệp khớp với thứ tôi định tìm. Chỉ có **người dùng thật, với dữ liệu thật,
gõ theo cách thật** mới phơi ra được. Cùng một bài học với `SPEC-001`, ở một khía cạnh khác:
lần đó là *chỗ chứa* thông tin, lần này là *độ trung thực* của thông tin.
