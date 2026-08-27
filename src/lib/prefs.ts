import type { MediaKind, Order } from "./search";

/// Những lựa chọn đáng nhớ giữa các lần mở: cách xem, cách xếp, và loại tệp
/// đang lọc. Người thích xem lưới không nên phải bấm lại nút lưới mỗi sáng.
///
/// Chỉ ba thứ này — có chủ ý. Ô tìm kiếm, bộ lọc chi tiết và bảng lọc đang
/// mở đều là chuyện của *một phiên*: mở lại app mà thấy truy vấn cũ cùng bộ
/// lọc cũ thì màn hình đầu tiên là một câu hỏi ("sao ít kết quả vậy?") thay
/// vì một khởi đầu sạch.
export interface Prefs {
  grid: boolean;
  order: Order;
  activeKinds: MediaKind[];
}

const KEY = "mediafinder:prefs";

const DEFAULTS: Prefs = { grid: false, order: "relevance", activeKinds: [] };

const VALID_KINDS: MediaKind[] = ["video", "image", "audio"];

/// Đọc tuỳ chọn đã lưu, lọc bỏ mọi giá trị không hợp lệ.
///
/// Từng trường được kiểm riêng chứ không tin cả cụm: dữ liệu trong
/// localStorage sống lâu hơn phiên bản đã ghi nó, và một bản cũ (hoặc một tay
/// chỉnh sửa thủ công) không được phép đưa app vào trạng thái không tồn tại —
/// `order: "oldest"` mà lọt qua thì backend nhận một giá trị nó chưa từng hứa
/// xử lý.
export function loadPrefs(): Prefs {
  let raw: string | null = null;
  try {
    raw = localStorage.getItem(KEY);
  } catch {
    return { ...DEFAULTS };
  }
  if (!raw) return { ...DEFAULTS };

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return { ...DEFAULTS };
  }
  if (typeof parsed !== "object" || parsed === null) return { ...DEFAULTS };
  const p = parsed as Record<string, unknown>;

  return {
    grid: typeof p.grid === "boolean" ? p.grid : DEFAULTS.grid,
    order: p.order === "newest" || p.order === "relevance" ? p.order : DEFAULTS.order,
    activeKinds: Array.isArray(p.activeKinds)
      ? VALID_KINDS.filter((k) => (p.activeKinds as unknown[]).includes(k))
      : [...DEFAULTS.activeKinds],
  };
}

/// Ghi đè toàn bộ, nuốt lỗi: localStorage có thể bị tắt hoặc đầy, và một tuỳ
/// chọn không lưu được thì đáng để bỏ qua chứ không đáng một băng lỗi đỏ.
export function savePrefs(p: Prefs): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(p));
  } catch {
    // thiếu chỗ lưu thì phiên sau quay về mặc định — chấp nhận được
  }
}
