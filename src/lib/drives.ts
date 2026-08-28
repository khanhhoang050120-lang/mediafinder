import type { SearchHit } from "./search";

/// Nhận diện ổ đĩa từ đường dẫn, và đếm kết quả theo từng ổ.
///
/// Làm ở phía giao diện chứ không thêm trường vào `SearchHit`: đường dẫn đã
/// mang sẵn câu trả lời, và một trường mới nghĩa là đụng vào cấu trúc mà mọi
/// đường tìm kiếm cùng bộ kiểm thử đang dựa vào — trả giá đó để lấy một thứ
/// suy ra được trong vài ký tự là không đáng.

/// Một ổ trong hàng chip, kèm số kết quả của nó.
export interface DriveBucket {
  /// Khoá lọc: chữ cái ổ viết hoa (`"D"`), hoặc tên máy chủ với đường dẫn
  /// UNC (`"\\\\NAS"`).
  id: string;
  /// Chữ hiện trên chip: `"D:"` hoặc `"NAS"`.
  label: string;
  /// Ổ mạng — chậm hơn và thường là kho lưu trữ, nên đáng được phân biệt
  /// ngay từ cái liếc mắt.
  network: boolean;
  count: number;
}

/// Khoá ổ của một đường dẫn.
///
/// Hai dạng đường dẫn Windows, hai cách đọc:
/// * `D:\thư mục\tệp` → `"D"`
/// * `\\NAS\share\tệp` → `"\\\\NAS"` (giữ cả hai gạch để không lẫn với một ổ
///   nào đó tên là `NAS`)
///
/// Đường dẫn lạ (rỗng, không khớp dạng nào) trả chuỗi rỗng — người gọi coi
/// như "không biết ổ nào" và bỏ qua, thay vì dựng một chip vô nghĩa.
export function driveKey(path: string): string {
  if (path.startsWith("\\\\")) {
    const host = path.slice(2).split("\\")[0];
    return host ? "\\\\" + host.toUpperCase() : "";
  }
  if (path.length >= 2 && path[1] === ":") return path[0].toUpperCase();
  return "";
}

/// Chữ hiện trên chip và trên nhãn mỗi dòng.
export function driveLabel(id: string): string {
  return id.startsWith("\\\\") ? id.slice(2) : id + ":";
}

/// Ổ này có phải ổ mạng không.
///
/// Đường dẫn UNC (`\\NAS\share`) thì tự nó đã nói ra. Nhưng **ổ mạng ánh xạ
/// thì không**: nó xuất hiện dưới dạng `Y:\…` y hệt một đĩa cắm trong máy, và
/// bản đầu chỉ nhận dạng UNC nên nhánh "ổ mạng" là **mã chết trên mọi máy của
/// studio** — cả bốn ổ NAS ở đó đều là ổ ánh xạ (`F:`, `H:`, `Y:`, `Z:`, đo
/// bằng `net use`), và chỉ mục lưu chúng là `Y:\PROJECT…`. Không chip cam,
/// không nhãn cam, ổ mạng không bị đẩy xuống cuối hàng chip. Tính năng lặng lẽ
/// giải đúng một nửa vấn đề nó sinh ra để giải.
///
/// Phần còn thiếu vốn đã có sẵn: lệnh `network_drives` trả về danh sách chữ
/// cái ổ mạng, và `App.svelte` đã gọi nó từ trước cho màn hình chạy lần đầu.
/// Ở đây chỉ cần nhận vào và dùng.
///
/// `netLetters` bỏ trống thì lùi về cách nhận dạng cũ — đúng cho kiểm thử
/// đơn vị và cho khoảnh khắc trước khi danh sách ổ về tới nơi.
export function isNetworkDrive(id: string, netLetters?: Set<string>): boolean {
  if (id.startsWith("\\\\")) return true;
  return netLetters ? netLetters.has(id.toUpperCase()) : false;
}

/// Dựng tập chữ cái ổ mạng từ câu trả lời của backend.
export function networkLetters(drives: { letter: string }[]): Set<string> {
  return new Set(drives.map((d) => d.letter.replace(":", "").toUpperCase()));
}

/// Gom kết quả theo ổ, xếp ổ nội bộ trước rồi tới ổ mạng, mỗi nhóm theo
/// thứ tự chữ cái.
///
/// Ổ mạng xuống cuối có chủ đích: nó thường đông kết quả nhất mà lại ít khi
/// là nơi người ta đang làm việc, nên để nó chiếm chỗ đầu hàng chip là đặt
/// sai trọng tâm.
export function bucketsFor(hits: SearchHit[], netLetters?: Set<string>): DriveBucket[] {
  const counts = new Map<string, number>();
  for (const h of hits) {
    const id = driveKey(h.path);
    if (!id) continue;
    counts.set(id, (counts.get(id) ?? 0) + 1);
  }

  return [...counts.entries()]
    .map(([id, count]) => ({
      id,
      label: driveLabel(id),
      network: isNetworkDrive(id, netLetters),
      count,
    }))
    .sort((a, b) => {
      if (a.network !== b.network) return a.network ? 1 : -1;
      return a.label.localeCompare(b.label);
    });
}

/// Lọc kết quả theo ổ đang chọn. `null` nghĩa là "Tất cả".
export function filterByDrive(hits: SearchHit[], drive: string | null): SearchHit[] {
  if (!drive) return hits;
  return hits.filter((h) => driveKey(h.path) === drive);
}
