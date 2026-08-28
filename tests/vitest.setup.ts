// Vá những chỗ jsdom khác trình duyệt thật, trước khi bất kỳ bài nào chạy.

// VirtualList đo viewport bằng ResizeObserver; jsdom không có.
class RO {
  observe() {}
  unobserve() {}
  disconnect() {}
}
(globalThis as any).ResizeObserver = RO;

// jsdom báo mọi phần tử cao 0, nên bộ ảo hoá chỉ vẽ mấy dòng dự phòng và bài
// kiểm thử không với tới được dòng thứ tám. Cho viewport một chiều cao như
// cửa sổ thật để danh sách vẽ đủ.
Object.defineProperty(HTMLElement.prototype, "clientHeight", {
  configurable: true,
  get(this: HTMLElement) {
    return this.classList?.contains("viewport") ? 600 : 0;
  },
});

// jsdom không có DragEvent; MouseEvent đủ cho ondragstart của app.
(globalThis as any).DragEvent = MouseEvent;

// Cửa xoay tải ảnh là trạng thái ở tầng module, dùng chung cho cả tiến trình
// test. Một nhóm dựng App rồi gỡ đi vẫn để lại các chỗ đang bị chiếm — nhóm
// chạy sau bị đói slot và đỏ vì lý do chẳng liên quan gì tới nó.
//
// Trước khi có dòng này, chạy riêng từng nhóm thì xanh, chạy chung thì đỏ, và
// SỐ ca đỏ đổi mỗi lần — dấu hiệu kinh điển của trạng thái rò rỉ giữa các
// nhóm. Dọn ở một chỗ duy nhất, thay vì bắt mười nhóm cùng nhớ.
import { beforeEach } from "vitest";
import { resetThumbQueueForTest } from "../src/lib/thumbQueue";

beforeEach(() => resetThumbQueueForTest());
