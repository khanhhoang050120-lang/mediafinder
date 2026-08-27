/// Cửa xoay cho việc tải thumbnail: tối đa một nhúm ảnh chạy cùng lúc, ô vừa
/// lọt vào mắt được đi trước.
///
/// Không có cái này, một cú cuộn nhanh ở chế độ lưới bắn hàng chục yêu cầu
/// cùng lúc vào hàng đợi 64 chỗ của backend — đầy hàng thì các ô *đang nhìn*
/// bị từ chối oan. Giới hạn ở đây thấp hơn nhiều so với sức chứa bên kia, nên
/// hàng bên backend gần như không bao giờ đầy nữa; mã thử-lại trong MediaRow
/// trở thành lưới an toàn thay vì đường đi thường xuyên.
///
/// Hai mức ưu tiên, hai kỷ luật xếp hàng khác nhau — có chủ đích:
/// * Ô đang hiện: **LIFO**. Người dùng cuộn tới đâu nhìn tới đó; ô mới thấy
///   nhất đáng đi trước ô đã bị cuộn qua.
/// * Tải trước (prefetch): **FIFO**, và chỉ khi không còn ô đang hiện nào
///   chờ. Đoán trước không bao giờ được tranh chỗ của cái đang nhìn.

const MAX_CONCURRENT = 8;

/// Trần riêng cho tải trước: một nửa cửa xoay.
///
/// Không có trần này, một loạt prefetch bị treo (đường trả lời tắc ở đâu đó)
/// sẽ chiếm đủ 8 chỗ và bỏ đói chính các ô đang hiện — thứ mà toàn bộ hàng
/// đợi này tồn tại để phục vụ. Đoán trước được phép chậm, không được phép
/// chặn cái đang nhìn.
const MAX_PREFETCH = 4;

interface Entry {
  grant: (done: () => void) => void;
  low: boolean;
  cancelled: boolean;
}

let active = 0;
let activeLow = 0;
const visible: Entry[] = []; // dùng như stack — LIFO
const ahead: Entry[] = []; // dùng như queue — FIFO

function pump(): void {
  while (active < MAX_CONCURRENT) {
    let e = visible.pop();
    if (!e) {
      if (activeLow >= MAX_PREFETCH) return;
      e = ahead.shift();
      if (!e) return;
    }
    if (e.cancelled) continue;
    active++;
    if (e.low) activeLow++;
    const low = e.low;
    let finished = false;
    e.grant(() => {
      // done() phải lũy đẳng: onload và teardown của component có thể cùng
      // gọi, và đếm trả chỗ hai lần sẽ cho phép 9, 10… ảnh chạy song song.
      if (finished) return;
      finished = true;
      active--;
      if (low) activeLow--;
      pump();
    });
  }
}

/// Xin một chỗ tải. `onGrant` được gọi (đồng bộ nếu còn chỗ) kèm theo hàm
/// `done` phải gọi khi ảnh xong việc — tải xong, lỗi, hay không cần nữa.
///
/// Trả về hàm huỷ: gọi trước khi được cấp thì rút khỏi hàng, gọi sau thì
/// tương đương `done`. Lũy đẳng, gọi bao nhiêu lần cũng được — teardown của
/// Svelte không phải nhớ mình đang ở trạng thái nào.
export function acquireThumbSlot(
  onGrant: (done: () => void) => void,
  low = false,
): () => void {
  let doneFn: (() => void) | null = null;
  const entry: Entry = {
    low,
    cancelled: false,
    grant: (done) => {
      doneFn = done;
      onGrant(done);
    },
  };
  (low ? ahead : visible).push(entry);
  pump();
  return () => {
    entry.cancelled = true;
    doneFn?.();
  };
}

/// Tải trước một URL với ưu tiên thấp. Trả về hàm huỷ.
///
/// Ảnh đi qua chính `Image()` của trình duyệt nên kết quả nằm lại trong HTTP
/// cache — lúc ô thật sự xuất hiện, thẻ `<img>` của nó nhận ảnh ngay mà không
/// hỏi đĩa lần nữa.
export function prefetchThumb(url: string): () => void {
  if (prefetched.has(url)) return () => {};
  remember(url);
  return acquireThumbSlot((done) => {
    const img = new Image();
    img.onload = () => done();
    img.onerror = () => done();
    img.src = url;
  }, true);
}

/// Các URL đã từng xếp hàng tải trước — chặn trùng lặp khi các sự kiện
/// viewport bắn liên tiếp về cùng một dải.
const prefetched = new Set<string>();

function remember(url: string): void {
  prefetched.add(url);
  // Chặn phình vô hạn trong một phiên dài; xoá cả tập rẻ hơn và đơn giản hơn
  // là đuổi từng phần tử, còn cái giá duy nhất là vài lần tải trước lặp lại.
  if (prefetched.size > 2048) prefetched.clear();
}

/// Chỉ dành cho kiểm thử: trả bộ đếm và hàng đợi về trạng thái ban đầu.
export function resetThumbQueueForTest(): void {
  active = 0;
  activeLow = 0;
  visible.length = 0;
  ahead.length = 0;
  prefetched.clear();
}
