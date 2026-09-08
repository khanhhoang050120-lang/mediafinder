## Tính năng mới: Quét trùng lặp

**Tìm những tệp bị nhân bản, để dọn bớt chỗ.** Một video 11 GB nằm bốn nơi thì ba bản là thừa. Ứng dụng đi tìm giúp bạn, gom thành từng nhóm, và nói rõ mỗi nhóm đang chiếm dư bao nhiêu — `thừa 33,7 GB`.

**Hỏi quét ở đâu trước khi bắt đầu.** Quét ổ mạng đọc trên chính NAS cả studio đang dùng, nên bạn được chọn: chỉ ổ trong máy, hay cả ổ mạng. Hộp thoại cho biết mỗi lựa chọn phải xem bao nhiêu tệp.

**Kết quả hiện dần, tệp đáng giá nhất lên trước.** Không phải chờ hết mới thấy gì. Những nhóm nặng nhất — thứ dọn được nhiều chỗ nhất — hiện ra trước, nên thường bạn dừng sớm được mà vẫn thu về gần hết dung lượng.

**Đối chiếu trước khi xoá.** Hai video khác nhau vẫn có thể trùng dung lượng và giống nhau ở đoạn đầu, nên trước khi xoá bất cứ thứ gì, hãy bấm **Đối chiếu** trên nhóm đó — ứng dụng mở cả nhóm ra so nội dung thật.

* **Đối chiếu** so hàng trăm điểm rải khắp tệp. Một nhóm 50 GB xong sau khoảng mười giây.
* **Toàn bộ** so từng byte một. Chắc chắn tuyệt đối, nhưng lâu hơn nhiều — dành cho khi bạn muốn yên tâm hoàn toàn.

Trong lúc chạy có phần trăm và nút **Dừng**. Bấm nhiều nhóm liền tay cũng được, chúng tự xếp hàng. Kết quả nói thẳng bằng lời: *"trùng từng byte — an toàn để xoá bớt"* hay *"KHÔNG phải bản sao — đừng xoá"*.

**Quét nền lúc máy rảnh**, nên thường mở màn hình lên là đã có sẵn kết quả.

---

## Sửa lỗi

**Quét nền không còn tự ý đọc ổ mạng.** Lỗi nặng nhất của bản này: dù đã đặt "chỉ quét ổ trong máy", lượt quét nền vẫn đọc trọn NAS — trên tất cả 20–40 máy, mỗi sáng. Nó làm chậm NAS cho cả studio vào đúng giờ mọi người bắt đầu làm việc.

**Danh sách trùng lặp không còn trỏ nhầm sang tệp khác.** Quét lúc 9 giờ, để đó, quay lại lúc 11 giờ — sau khi chỉ mục tự làm mới, mỗi nhóm hiện tên và đường dẫn của tệp khác hẳn, không một lời cảnh báo. Trên màn hình mà việc kế tiếp là xoá, đây là lỗi nguy hiểm nhất có thể có.

**Ổ mạng rớt giữa chừng thì nói thật.** Trước đây mất kết nối giữa lượt quét sẽ cho ra câu *"Không tìm thấy tệp trùng lặp nào"* — nghe như đã tìm xong và không có gì. Nay nó nói rõ: *"Thiếu Y: — ổ không còn kết nối"*.

**Gõ tìm kiếm trong lúc đang quét không còn bị đơ.** Lượt quét nay nhường đường cho việc bạn đang làm.

---

## Nhanh hơn

**Quét trùng lặp nhanh hơn khoảng 3 lần.** Đo trên chính thư viện của studio, sau khi bỏ được một thao tác đọc tốn kém trên mỗi tệp.

**Các máy chia sẻ kết quả cho nhau.** Nội dung ổ mạng giống hệt nhau trên mọi máy, nên máy đầu tiên quét xong sẽ để lại kết quả ngay trên ổ mạng. Những máy sau đọc lại kết quả đó thay vì đọc lại gần 90 nghìn tệp — vừa nhanh hơn hẳn, vừa đỡ tải cho NAS.

**Bấm đối chiếu lại một nhóm đã xem thì trả lời ngay**, không đọc lại đĩa.
