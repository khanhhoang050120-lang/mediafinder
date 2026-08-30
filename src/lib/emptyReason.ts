import type { Freshness } from "./search";

/// Vì sao màn hình đang trống — quyết định app nên nói câu nào.
///
/// Trước tệp này, app chỉ có **một** câu cho mọi trường hợp: "Không tìm thấy
/// kết quả nào". Câu đó gộp bốn tình huống khác hẳn nhau và ngầm đổ lỗi cho
/// người dùng ở cả bốn — trong khi ba trong số đó là app đang tự che mất câu
/// trả lời.
///
/// Nó đã gây thiệt hại thật: một người tìm tệp `.avif` trong lúc chip lọc
/// *Video* đang bật, không thấy gì, rồi kết luận **công cụ tìm kiếm kém đi**.
/// Công cụ không sai; màn hình nói sai về nguyên nhân.
///
/// Tách khỏi component để kiểm thử được từng nhánh mà không phải dựng cả ứng
/// dụng — và vì đây là phần dễ sai nhất: thứ tự ưu tiên.

/// Mức tin cậy của câu trả lời. Hiện ra thành cách dùng chữ, không phải trang
/// trí: điều app **biết chắc** thì nói thẳng, điều app chỉ suy ra thì phải nói
/// đúng mức đó.
export type Certainty = "chắc chắn" | "có thể";

export interface EmptyReason {
  /// Khoá để kiểm thử bám vào, ổn định hơn là bám vào câu chữ.
  kind: "filter" | "unscanned-network" | "stale-network" | "stale-local" | "genuinely-empty";
  certainty: Certainty;
  /// Dòng chữ lớn.
  title: string;
  /// Dòng giải thích bên dưới.
  detail: string;
  /// Nút hành động, nếu có việc gì bấm được để sửa.
  action?: { label: string; do: "clear-filters" | "rescan" | "scan-network" };
}

/// Những bộ lọc đang bật, và điều quan trọng nhất: bỏ chúng ra thì còn bao
/// nhiêu kết quả.
export interface FilterState {
  /// Tên các bộ lọc đang bật, đã sẵn sàng để hiện: `["Video", "ổ D:"]`.
  active: string[];
  /// Số kết quả nếu bỏ hết bộ lọc. Đây là con số app **đếm được thật**, không
  /// phải lời hứa — nó có sẵn danh sách trước khi lọc.
  countWithout: number;
}

/// Bao lâu thì coi một ổ trong máy là "đã cũ", tính bằng giờ.
///
/// Sáu tiếng. Tác vụ nền quét ổ trong máy mỗi ngày và mỗi lần đăng nhập, nên
/// mốc quét bình thường luôn mới. Đặt ngưỡng thấp hơn thì câu này hiện ra
/// suốt ngày và mất hết ý nghĩa; đặt cao hơn thì nó im lặng đúng lúc người
/// dùng vừa tải tệp về sáng nay.
const CU_NOI_BO_GIO = 6;

/// Bao lâu thì coi ổ mạng là "đã cũ", tính bằng giờ.
///
/// Hai tiếng, thấp hơn ổ trong máy — cố ý. Ổ mạng quét mỗi 12 tiếng chứ không
/// phải mỗi ngày vài lần, nên khoảng mù của nó rộng hơn nhiều, và một tệp
/// đồng nghiệp vừa đưa lên NAS là trường hợp thường gặp nhất trong studio.
const CU_MANG_GIO = 2;

const GIO = 3600;

/// Đọc mốc thời gian thành câu tiếng Việt: "6 tiếng trước", "hôm qua".
export function agoText(seconds: number): string {
  if (seconds < 90) return "vừa xong";
  const phut = Math.round(seconds / 60);
  if (phut < 60) return `${phut} phút trước`;
  const gio = Math.round(seconds / GIO);
  if (gio < 24) return `${gio} tiếng trước`;
  const ngay = Math.round(seconds / (24 * GIO));
  return ngay === 1 ? "hôm qua" : `${ngay} ngày trước`;
}

/// Giờ trong ngày: "09:15".
function clock(unix: number): string {
  const d = new Date(unix * 1000);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/// Quyết định câu nào hiện ra.
///
/// Hỏi theo thứ tự, dừng ở câu đầu tiên trả lời được — và thứ tự đó **chính
/// là thứ tự chắc chắn**: câu đầu app tự biết, câu cuối là khi đã loại trừ
/// hết. Đảo thứ tự này là để một phỏng đoán che mất một sự thật.
///
/// * `nowUnix` — giây Unix hiện tại, truyền vào để kiểm thử được.
export function reasonFor(
  filters: FilterState,
  fresh: Freshness | null,
  nowUnix: number,
): EmptyReason {
  // 1. Bộ lọc đang che. Ưu tiên cao nhất vì đây là điều duy nhất app biết
  //    CHẮC CHẮN — nó có sẵn danh sách trước khi lọc nên đếm được thật.
  if (filters.active.length && filters.countWithout > 0) {
    const ten = filters.active.join(" · ");
    return {
      kind: "filter",
      certainty: "chắc chắn",
      title: `Bộ lọc đang ẩn ${filters.countWithout} kết quả`,
      detail: `Đang lọc ${ten}. Bỏ lọc thì có ${filters.countWithout} kết quả khớp.`,
      action: { label: "Bỏ lọc", do: "clear-filters" },
    };
  }

  // Không có dữ liệu về chỉ mục thì không suy đoán gì thêm.
  if (!fresh || !fresh.builtAtUnix) {
    return {
      kind: "genuinely-empty",
      certainty: "chắc chắn",
      title: "Không tìm thấy kết quả nào",
      detail: "Thử bớt từ khoá hoặc kiểm tra chính tả.",
    };
  }

  // `Math.max(0, …)` cho trường hợp đồng hồ chạy lùi (đổi giờ hệ thống, hoặc
  // NTP kéo lại) khiến mốc quét nằm ở tương lai.
  //
  // Nói thẳng: hôm nay nó KHÔNG với tới được. Tuổi âm luôn nhỏ hơn mọi ngưỡng
  // bên dưới, nên `agoText` không bao giờ nhận số âm — phép thử bằng cách phá
  // mã đã chứng minh: bỏ hẳn `Math.max` mà không bài kiểm thử nào đỏ, và cũng
  // không thể viết bài nào đỏ được. Giữ nó lại vì nó rẻ và vì hạ một ngưỡng
  // xuống 0 trong tương lai sẽ làm nó thành cần thiết ngay lập tức.
  const tuoi = Math.max(0, nowUnix - fresh.builtAtUnix);

  // 2. Ổ mạng đang gắn mà chỉ mục CHƯA HỀ biết tới. Khác hẳn "ổ mạng cũ":
  //    mọi tệp trên đó đều vô hình, không phải chỉ vài tệp mới.
  if (fresh.unscannedNetwork.length) {
    const ds = fresh.unscannedNetwork.map((l) => `${l}:`).join(", ");
    return {
      kind: "unscanned-network",
      certainty: "chắc chắn",
      title: `Ổ mạng ${ds} chưa được quét lần nào`,
      detail: `Không tệp nào trên ${ds} có trong danh sách tìm kiếm.`,
      action: { label: "Quét ổ mạng · ~2 phút", do: "scan-network" },
    };
  }

  // 3. Ổ mạng đã lâu chưa quét. Chỉ "có thể", và câu chữ phải nói đúng mức
  //    đó: ổ mạng không có nhật ký thay đổi để hỏi (chính là gốc của
  //    BUG-025), nên app chỉ biết chỉ mục của mình cũ tới đâu — không biết
  //    trên NAS có gì mới. Khẳng định "tệp của bạn vừa được tải lên" là nói
  //    một điều app chưa hề xác minh.
  if (fresh.network.length && tuoi >= CU_MANG_GIO * GIO) {
    return {
      kind: "stale-network",
      certainty: "có thể",
      title: `Ổ mạng quét lần cuối ${agoText(tuoi)}`,
      detail: `Lúc ${clock(fresh.builtAtUnix)}. Tệp đưa lên ổ mạng sau lúc đó chưa có trong danh sách.`,
      action: { label: "Quét lại ổ mạng · ~2 phút", do: "scan-network" },
    };
  }

  // 4. Ổ trong máy đã lâu chưa quét. Hiếm hơn — tác vụ nền quét mỗi ngày —
  //    nên nếu nó xảy ra thì đáng nói.
  if (fresh.local.length && tuoi >= CU_NOI_BO_GIO * GIO) {
    return {
      kind: "stale-local",
      certainty: "có thể",
      title: `Chỉ mục quét lần cuối ${agoText(tuoi)}`,
      detail: `Lúc ${clock(fresh.builtAtUnix)}. Tệp lưu về sau lúc đó chưa có trong danh sách.`,
      action: { label: "Quét lại · vài giây", do: "rescan" },
    };
  }

  // 5. Đã loại trừ hết. Giờ câu này mới đúng — và nó NÓI RA rằng đã loại trừ,
  //    để người dùng tin được thay vì tự hỏi mình đã bỏ sót gì.
  return {
    kind: "genuinely-empty",
    certainty: "chắc chắn",
    title: "Không tìm thấy kết quả nào",
    detail: `Chỉ mục vừa quét lúc ${clock(fresh.builtAtUnix)}, không có bộ lọc nào đang bật. Thử bớt từ khoá hoặc kiểm tra chính tả.`,
  };
}
