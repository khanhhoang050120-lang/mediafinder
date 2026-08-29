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

// jsdom không cài đặt Element.scrollTo — gọi vào là ném TypeError. Trong
// trình duyệt thật nó tồn tại và không ném. Không vá thì mọi bài đụng tới
// việc cuộn danh sách về đầu đều hỏng vì một lỗ hổng của môi trường, chứ
// không phải vì mã sai.
if (!HTMLElement.prototype.scrollTo) {
  HTMLElement.prototype.scrollTo = function () {};
}
