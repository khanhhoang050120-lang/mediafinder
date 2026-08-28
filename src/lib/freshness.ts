/**
 * Diễn đạt "chỉ mục cũ tới mức nào" thành câu người đọc hiểu ngay.
 *
 * Tách khỏi component vì đây là chỗ dễ sai nhất và cũng là chỗ dễ test nhất:
 * ranh giới giữa "vừa xong" và "5 phút trước", giữa "hôm nay" và "hôm qua",
 * là những thứ chỉ lộ ra khi có người ngồi thử từng mốc.
 *
 * # Vì sao phải có hai mốc, không phải một
 *
 * Chân cửa sổ trước đây in đúng một mốc, lấy từ `meta.builtAtUnix`. Nhưng
 * `persist.rs` đóng dấu `built_at_unix = now_unix()` ở **mọi** lần ghi cache —
 * kể cả lượt vá gia tăng ổ cục bộ. Nên sau một lượt vá lúc 16:15, ứng dụng nói
 * "quét lúc 16:15" trong khi nửa ổ mạng của chỉ mục vẫn là bản 11:23.
 *
 * Đó không phải thiếu thông tin, đó là **nói sai** — và nói sai đúng vào lúc
 * người dùng đang cố hiểu vì sao tệp của họ không hiện ra. Họ đọc "vừa quét
 * xong" rồi kết luận phần mềm hỏng. Với lịch 15 phút, câu sai ấy được lặp lại
 * 96 lần mỗi ngày.
 *
 * Hai mốc là hai sự thật khác nhau và phải đứng riêng: ổ trong máy làm mới
 * theo tác vụ định kỳ, ổ mạng chỉ làm mới khi có người bấm nút.
 */

/** Một giây, tính bằng đơn vị của `Date.now()`. */
const GIAY = 1000;

/**
 * "3 phút trước", "2 giờ trước", "hôm qua"…
 *
 * Trả về chuỗi rỗng khi không có mốc — chỗ gọi tự quyết định nói gì thay thế,
 * vì "chưa từng quét ổ mạng" và "chưa nạp xong chỉ mục" là hai câu khác nhau.
 *
 * @param unix mốc thời gian, tính bằng giây (0 hoặc âm = không biết)
 * @param bayGioMs thời điểm hiện tại; truyền vào để test được, mặc định là bây giờ
 */
export function moTaTuoi(unix: number, bayGioMs: number = Date.now()): string {
  if (!unix || unix <= 0) return "";

  const giay = Math.floor((bayGioMs - unix * GIAY) / GIAY);

  // Đồng hồ máy có thể lệch, hoặc mốc tới từ một máy khác. Một con số âm in ra
  // thành "-3 phút trước" thì trông như lỗi phần mềm, nên gộp về "vừa xong".
  if (giay < 60) return "vừa xong";

  const phut = Math.floor(giay / 60);
  if (phut < 60) return `${phut} phút trước`;

  const gio = Math.floor(phut / 60);
  if (gio < 24) return `${gio} giờ trước`;

  const ngay = Math.floor(gio / 24);
  if (ngay === 1) return "hôm qua";
  return `${ngay} ngày trước`;
}

/**
 * Chỉ mục đã cũ tới mức đáng nói ra chưa?
 *
 * Ngưỡng cố ý khác nhau cho hai loại ổ, vì hai đường làm mới khác hẳn nhau:
 *
 * - **Ổ trong máy** làm mới mỗi 15 phút, nên quá 30 phút là dấu hiệu tác vụ
 *   định kỳ không chạy — đáng nói.
 * - **Ổ mạng** chỉ làm mới khi có người bấm "+ ổ mạng", nên một giờ là chuyện
 *   bình thường; chỉ quá 2 giờ mới đáng nhắc, kẻo câu cảnh báo hiện suốt ngày
 *   và người ta thôi đọc nó.
 */
export const NGUONG_CUC_BO_GIAY = 30 * 60;
export const NGUONG_O_MANG_GIAY = 2 * 60 * 60;

export function daCu(unix: number, nguongGiay: number, bayGioMs: number = Date.now()): boolean {
  if (!unix || unix <= 0) return false;
  return (bayGioMs - unix * GIAY) / GIAY > nguongGiay;
}
