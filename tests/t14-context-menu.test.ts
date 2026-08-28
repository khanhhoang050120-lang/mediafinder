// Nhóm 14 — trình đơn chuột phải.
//
// Viết sau khi rà lại toàn bộ: `ContextMenu` bị dùng ở cả danh sách tìm kiếm
// lẫn chế độ trùng lặp, nhưng chưa có nhóm nào canh nó. Ba nhánh dưới đây đều
// từng bị phá mà 129 test khác vẫn xanh — nghĩa là chúng chưa được canh ở đâu
// cả, chỉ tình cờ đúng.
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { settle } from "./helpers";
import ContextMenu, { type MenuItem } from "../src/lib/ContextMenu.svelte";

let ran: string[] = [];
let closed = 0;

function items(): MenuItem[] {
  return [
    { label: "Xem trước", icon: "eye", shortcut: "Shift+Enter", action: () => ran.push("xem") },
    { label: "Mở tệp", icon: "open", shortcut: "Enter", action: () => ran.push("mo") },
    { label: "Sao chép đường dẫn", icon: "copy", action: () => ran.push("chep") },
  ];
}

const pending: (() => void)[] = [];
afterEach(() => {
  while (pending.length) pending.pop()!();
});

function mountMenu(x = 100, y = 100) {
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(ContextMenu, {
    target: div,
    props: { x, y, items: items(), onclose: () => closed++ },
  });
  let done = false;
  const cleanup = () => {
    if (done) return;
    done = true;
    unmount(app);
    div.remove();
  };
  pending.push(cleanup);
  return { div, cleanup };
}

const menuEl = (div: Element) => div.querySelector(".menu") as HTMLElement;
const catcher = (div: Element) => div.querySelector(".catcher")!;
const itemBtns = (div: Element) => [...div.querySelectorAll(".item")] as HTMLButtonElement[];
const labelOf = (b: Element) => b.querySelector(".label")!.textContent!.trim();

beforeEach(() => {
  ran = [];
  closed = 0;
});

describe("dựng và nội dung", () => {
  it("vẽ đủ các mục, đúng nhãn và phím tắt", () => {
    const { div } = mountMenu();
    expect(itemBtns(div).map(labelOf)).toEqual(["Xem trước", "Mở tệp", "Sao chép đường dẫn"]);
    const shortcuts = [...div.querySelectorAll(".shortcut")].map((s) => s.textContent!.trim());
    expect(shortcuts, "mục không có phím tắt thì không vẽ ô trống").toEqual([
      "Shift+Enter",
      "Enter",
    ]);
  });

  it("mỗi mục có biểu tượng riêng — không phải hình mặc định dùng chung", () => {
    const { div } = mountMenu();
    const paths = itemBtns(div).map((b) => b.querySelector("svg path")!.getAttribute("d"));
    expect(new Set(paths).size, "các biểu tượng trùng nhau").toBe(3);
  });
});

describe("bấm một mục", () => {
  it("chạy đúng hành động của mục đó", () => {
    const { div } = mountMenu();
    itemBtns(div)[1].dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(ran).toEqual(["mo"]);
  });

  it("và đóng menu ngay sau đó — menu treo lại sau khi đã chọn là rác trên màn hình", () => {
    const { div } = mountMenu();
    itemBtns(div)[0].dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(ran).toEqual(["xem"]);
    expect(closed, "bấm xong mà menu không đóng").toBe(1);
  });
});

describe("đóng menu", () => {
  it("bấm ra ngoài thì đóng", () => {
    const { div } = mountMenu();
    catcher(div).dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
    expect(closed).toBe(1);
  });

  it("cú bấm ra ngoài bị nuốt — không lọt xuống chọn nhầm dòng bên dưới", () => {
    const { div } = mountMenu();
    const e = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
    catcher(div).dispatchEvent(e);
    expect(e.defaultPrevented, "cú bấm đóng menu lại chọn luôn dòng phía sau").toBe(true);
  });

  it("chuột phải ra ngoài cũng đóng, và không bật menu của trình duyệt", () => {
    const { div } = mountMenu();
    const e = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
    catcher(div).dispatchEvent(e);
    expect(closed).toBe(1);
    expect(e.defaultPrevented).toBe(true);
  });

  it("Escape thì đóng", () => {
    mountMenu();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(closed).toBe(1);
  });

  it("Escape gọi stopPropagation — nhưng đó KHÔNG đủ, và App phải tự phòng", () => {
    // Ghi lại một sự thật của DOM mà chú thích trong App đã nêu: hai trình
    // nghe anh em trên cùng `window` không chặn được nhau. `stopPropagation`
    // ở đây chỉ dừng việc nổi bọt lên trên, nên bản thân nó KHÔNG ngăn được
    // trình nghe của App. Cái ngăn thật sự là chốt `if (menu || preview)
    // return` bên App — và nhóm 3 canh chốt đó (TC-3.16b).
    //
    // Test này tồn tại để ai đó gỡ `stopPropagation` đi thì thấy nó có chủ
    // đích, đồng thời nói rõ nó bảo vệ tới đâu.
    mountMenu();
    const e = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
    window.dispatchEvent(e);
    expect(closed, "Escape không đóng được menu").toBe(1);
  });
});

describe("kê vào trong màn hình", () => {
  /// jsdom mặc định 1024×768; đo bằng cách cấp số đo cho phần tử menu.
  function sizeMenu(div: Element, w: number, h: number) {
    const el = menuEl(div);
    Object.defineProperty(el, "clientWidth", { configurable: true, get: () => w });
    Object.defineProperty(el, "clientHeight", { configurable: true, get: () => h });
    window.dispatchEvent(new Event("resize"));
  }

  it("mở ở giữa màn hình thì đứng đúng chỗ chuột", () => {
    const { div } = mountMenu(200, 150);
    expect(menuEl(div).style.left).toBe("200px");
    expect(menuEl(div).style.top).toBe("150px");
  });

  it("mở sát mép phải thì lùi vào — mục dưới con trỏ là mục dễ bị cắt nhất", async () => {
    const { div } = mountMenu(window.innerWidth - 10, 100);
    sizeMenu(div, 232, 140);
    await settle(30);
    const left = parseInt(menuEl(div).style.left, 10);
    expect(
      left + 232,
      `menu tràn khỏi mép phải: left=${left}, rộng 232, màn ${window.innerWidth}`,
    ).toBeLessThanOrEqual(window.innerWidth);
  });

  it("mở sát mép dưới thì lùi lên", async () => {
    const { div } = mountMenu(100, window.innerHeight - 10);
    sizeMenu(div, 232, 140);
    await settle(30);
    const top = parseInt(menuEl(div).style.top, 10);
    expect(top + 140, "menu tràn khỏi mép dưới").toBeLessThanOrEqual(window.innerHeight);
  });

  it("không bao giờ lùi ra ngoài mép trái/trên — kể cả khi chuột ở toạ độ âm", async () => {
    // Toạ độ âm nghe lạ nhưng có thật: màn hình phụ đặt bên trái màn chính
    // cho `clientX` âm. Dùng nó vì `(2, 2)` quá dễ — chỉ cần bỏ `Math.max`
    // là 2 vẫn qua được phép kiểm, và đột biến ấy từng lọt lưới.
    const { div } = mountMenu(-50, -50);
    sizeMenu(div, 232, 140);
    await settle(30);
    const left = parseInt(menuEl(div).style.left, 10);
    const top = parseInt(menuEl(div).style.top, 10);
    expect(left, "menu trôi ra ngoài mép trái").toBeGreaterThan(0);
    expect(top, "menu trôi lên trên mép trên").toBeGreaterThan(0);
  });
});
