// Nhóm 12 — hỏi trước khi quét lại ổ mạng.
//
// Nút "+ ổ mạng" không mang trạng thái: bấm lần nào cũng chạy lại trọn cả hai
// giai đoạn, tốn vài phút. Hộp thoại phải nói được lần trước quét lúc nào và
// ra bao nhiêu — không có hai con số đó thì lời hỏi rỗng tuếch.
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import App from "../src/App.svelte";
import type { NetScanMark } from "../src/lib/search";

const MARK: NetScanMark = {
  atUnix: 1_772_000_000,
  files: 313_945,
  drives: 2,
  seconds: 271.5,
};

let ipc: IpcRecorder;
let mark: NetScanMark | null = MARK;

function baseHandlers(): IpcRecorder {
  const r = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = r;
  r.on("index_status", { loaded: true, fileCount: 100, dirCount: 5, builtAtUnix: 1_700_000_000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [{ letter: "Z", remote: "\\\\NAS\\media" }])
    .on("update_status", { checked: true, available: null, current: "1.0.5" })
    .on("search", (a: { id: number }) => ({
      id: a.id,
      hits: [],
      epoch: 3,
      relaxed: null,
      elapsedMs: 1,
      total: 0,
    }))
    .on("net_scan_mark", () => mark)
    .on("request_scan", null)
    .on("request_scan_with_network", null)
    .on("miss_log_status", { enabled: false, count: 0 })
    .on("dupe_progress", { running: false, completed: false, groups: 0, wasted: 0, hashed: 0, candidates: 0 })
    .on("dupe_groups", [])
    .on("cancel_duplicates", null);
  return r;
}

const pending: (() => void)[] = [];
afterEach(() => {
  while (pending.length) pending.pop()!();
});

async function mountApp() {
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  await settle(90);
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

const $$ = (root: ParentNode, sel: string) => [...root.querySelectorAll(sel)];
const btn = (root: ParentNode, text: string) =>
  $$(root, "button").find((b) => b.textContent!.trim().startsWith(text)) as
    | HTMLButtonElement
    | undefined;
/// Nút BÊN TRONG hộp thoại. Thanh chính cũng có nút mở đầu bằng "Quét lại",
/// và `document` trả về nó trước — tìm nhầm thì test xanh vì lý do sai.
const dlgBtn = (text: string) => {
  const d = dialog();
  expect(d, "chưa có hộp thoại nào để tìm nút trong đó").toBeTruthy();
  return btn(d!, text);
};
const click = (el: Element | undefined) => {
  expect(el, "phần tử cần bấm không tồn tại").toBeTruthy();
  el!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
};
const dialog = () => document.querySelector("[role=dialog]");

beforeEach(() => {
  localStorage.clear();
  mark = MARK;
  ipc = baseHandlers();
});

describe("hỏi trước khi quét lại ổ mạng", () => {
  it("bấm '+ ổ mạng' KHÔNG quét ngay — hỏi trước đã", async () => {
    const { div } = await mountApp();
    click(btn(div, "+ ổ mạng"));
    await settle(80);
    expect(dialog(), "không có hộp thoại nào hiện ra").toBeTruthy();
    expect(
      ipc.count("request_scan_with_network"),
      "quét luôn mà chưa hỏi — đúng cái làm người dùng chờ vài phút oan",
    ).toBe(0);
  });

  it("hộp thoại nêu đủ ba con số của lần trước: lúc nào, bao nhiêu tệp, mất bao lâu", async () => {
    const { div } = await mountApp();
    click(btn(div, "+ ổ mạng"));
    await settle(80);
    const text = dialog()!.textContent!;
    expect(text, "thiếu số tệp lần trước").toContain("313.945");
    expect(text, "thiếu số ổ mạng").toContain("2 ổ mạng");
    expect(text, "271,5 giây phải nói thành 'khoảng 5 phút'").toContain("khoảng 5 phút");
  });

  it("bấm 'Quét lại' thì mới thật sự quét", async () => {
    const { div } = await mountApp();
    click(btn(div, "+ ổ mạng"));
    await settle(80);
    click(dlgBtn("Quét lại"));
    await settle(80);
    expect(ipc.count("request_scan_with_network")).toBe(1);
    expect(dialog(), "hộp thoại chưa đóng sau khi đồng ý").toBeNull();
  });

  it("bấm 'Không' thì không quét gì cả", async () => {
    const { div } = await mountApp();
    click(btn(div, "+ ổ mạng"));
    await settle(80);
    click(dlgBtn("Không"));
    await settle(80);
    expect(ipc.count("request_scan_with_network")).toBe(0);
    expect(dialog()).toBeNull();
  });

  it("Escape cũng là 'Không' — và KHÔNG lọt xuống app bên dưới", async () => {
    const { div } = await mountApp();
    const input = div.querySelector("input.search") as HTMLInputElement;
    input.value = "abc";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await settle(320);

    click(btn(div, "+ ổ mạng"));
    await settle(80);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    await settle(80);
    expect(dialog()).toBeNull();
    expect(ipc.count("request_scan_with_network")).toBe(0);
    expect(input.value, "Escape lọt xuống và xoá mất truy vấn").toBe("abc");
  });

  it("chưa từng quét ổ mạng: hỏi bằng lời khác, không bịa số liệu", async () => {
    mark = null;
    const { div } = await mountApp();
    click(btn(div, "+ ổ mạng"));
    await settle(80);
    const text = dialog()!.textContent!;
    expect(text).toContain("Quét cả ổ mạng?");
    expect(text, "chưa quét bao giờ mà vẫn nói 'lần trước'").not.toContain("Lần trước");
    expect(dlgBtn("Quét"), "nút phải là 'Quét', không phải 'Quét lại'").toBeTruthy();
  });

  it("nút 'Quét lại' thường KHÔNG hỏi — nó chỉ tốn vài giây", async () => {
    const { div } = await mountApp();
    click(btn(div, "Quét lại"));
    await settle(80);
    expect(dialog(), "quét ổ trong máy mà cũng hỏi thì phiền vô ích").toBeNull();
    expect(ipc.count("request_scan")).toBe(1);
  });

  it("hộp thoại đang mở thì bàn phím của danh sách bị khoá", async () => {
    const { div } = await mountApp();
    click(btn(div, "+ ổ mạng"));
    await settle(80);
    const e = new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true });
    window.dispatchEvent(e);
    await settle(40);
    expect(e.defaultPrevented, "phím mũi tên lọt xuống danh sách sau lưng hộp thoại").toBe(false);
  });

  it("đọc lại dấu vết mỗi lần mở — lượt quét vừa xong đã ghi số mới", async () => {
    const { div } = await mountApp();
    // Đo theo ĐỘ TĂNG, không theo số tuyệt đối. Bất biến ở đây là "mỗi lần mở
    // hộp thoại thì hỏi lại", chứ không phải "trong cả vòng đời ứng dụng chỉ
    // có đúng ngần này lượt đọc" — kể từ P28 còn một lượt đọc hợp lệ nữa lúc
    // mở cửa sổ, để dòng tuổi chỉ mục có số mà nói. Đếm tuyệt đối sẽ đỏ mỗi
    // khi thêm một chỗ đọc chính đáng, và đó là bài kiểm thử giòn chứ không
    // phải bài kiểm thử chặt.
    const nen = ipc.count("net_scan_mark");
    click(btn(div, "+ ổ mạng"));
    await settle(80);
    click(dlgBtn("Không"));
    await settle(40);
    expect(ipc.count("net_scan_mark") - nen).toBe(1);
    click(btn(div, "+ ổ mạng"));
    await settle(80);
    expect(
      ipc.count("net_scan_mark") - nen,
      "dùng lại số liệu cũ thay vì hỏi lại",
    ).toBe(2);
  });
});
