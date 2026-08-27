// Nhóm 9 — hộp thoại cập nhật: tự mở khi có tin, ghi chú "có gì mới", hai nút
// Cập nhật / Để sau, và mũi tên giữa chân cửa sổ để quay lại.
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import App from "../src/App.svelte";
import type { SearchHit, UpdateStatus } from "../src/lib/search";

function mkHit(i: number, name: string): SearchHit {
  return {
    index: i,
    name,
    dir: "D:\\m",
    path: `D:\\m\\${name}`,
    kind: "video",
    matched: 1,
    size: 4096,
    width: 1920,
    height: 1080,
    durationMs: 60_000,
  } as SearchHit;
}

const NOTES =
  "- Sửa lỗi cuộn nhanh\n- Space tạm dừng video\n\n---\n\n**Cài lần đầu:** tải tệp .exe bên dưới rồi chạy.";

const FOUND: UpdateStatus = {
  checked: true,
  available: { version: "1.0.4", notes: NOTES },
  current: "1.0.3",
};
const NONE: UpdateStatus = { checked: true, available: null, current: "1.0.3" };

let ipc: IpcRecorder;

function baseHandlers(update: UpdateStatus): IpcRecorder {
  const r = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = r;
  r.on("index_status", { loaded: true, fileCount: 100, dirCount: 5, builtAtUnix: 1_700_000_000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", update)
    .on("search", (a: { id: number }) => ({
      id: a.id,
      hits: [mkHit(1, "a.mp4"), mkHit(2, "b.mp4"), mkHit(3, "c.mp4")],
      epoch: 3,
      relaxed: null,
      elapsedMs: 1,
      total: 3,
    }))
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

const dialog = () => document.querySelector("[role=dialog]");
const arrow = (div: Element) => div.querySelector(".update-arrow") as HTMLButtonElement | null;
const click = (el: Element | null | undefined) => {
  expect(el, "phần tử cần bấm không tồn tại").toBeTruthy();
  el!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
};
const btn = (label: string) =>
  [...document.querySelectorAll("[role=dialog] button")].find((b) =>
    b.textContent!.trim().startsWith(label),
  );

beforeEach(() => localStorage.clear());

describe("hộp thoại cập nhật", () => {
  it("không có bản mới: không hộp thoại, không mũi tên", async () => {
    ipc = baseHandlers(NONE);
    const { div } = await mountApp();
    expect(dialog()).toBeNull();
    expect(arrow(div)).toBeNull();
  });

  it("số phiên bản đang chạy luôn hiện ở chân cửa sổ", async () => {
    ipc = baseHandlers(NONE);
    const { div } = await mountApp();
    expect(div.querySelector(".ver")?.textContent, "không thấy số phiên bản").toBe("v1.0.3");
  });

  it("có bản mới lúc mở app: hộp thoại TỰ hiện, đủ số hiệu và hai nút", async () => {
    ipc = baseHandlers(FOUND);
    await mountApp();
    const d = dialog();
    expect(d, "hộp thoại không tự mở").toBeTruthy();
    expect(d!.textContent).toContain("1.0.4");
    expect(d!.textContent).toContain("1.0.3"); // bản đang dùng
    expect(btn("Cập nhật"), "thiếu nút Cập nhật").toBeTruthy();
    expect(btn("Để sau"), "thiếu nút Để sau").toBeTruthy();
  });

  it("ghi chú hiện phần CHANGELOG, cắt bỏ phần hướng dẫn cài sau vạch ---", async () => {
    ipc = baseHandlers(FOUND);
    await mountApp();
    const text = dialog()!.textContent!;
    expect(text).toContain("Sửa lỗi cuộn nhanh");
    expect(text).toContain("Space tạm dừng video");
    expect(text, "hướng dẫn cài dành cho trang Releases lọt vào hộp thoại").not.toContain(
      "Cài lần đầu",
    );
  });

  it("máy chủ không gửi ghi chú: nói thẳng, không để trống", async () => {
    ipc = baseHandlers({ ...FOUND, available: { version: "1.0.4", notes: null } });
    await mountApp();
    expect(dialog()!.textContent).toContain("không kèm ghi chú");
  });

  it("Để sau: hộp thoại đóng, mũi tên hiện giữa chân cửa sổ, bấm vào là quay lại", async () => {
    ipc = baseHandlers(FOUND);
    const { div } = await mountApp();
    click(btn("Để sau"));
    await settle(40);
    expect(dialog(), "hộp thoại chưa đóng").toBeNull();
    const a = arrow(div);
    expect(a, "mũi tên không xuất hiện").toBeTruthy();
    expect(a!.title).toContain("1.0.4");
    click(a);
    await settle(40);
    expect(dialog(), "bấm mũi tên không mở lại hộp thoại").toBeTruthy();
    expect(arrow(div), "mũi tên vẫn hiện trong lúc hộp thoại đang mở").toBeNull();
  });

  it("Escape đóng hộp thoại như bấm Để sau — và KHÔNG lọt xuống app bên dưới", async () => {
    ipc = baseHandlers(FOUND);
    const { div } = await mountApp();
    // gõ truy vấn để có thứ mà Escape-của-App sẽ xoá nếu phím bị lọt
    const input = div.querySelector("input.search") as HTMLInputElement;
    input.value = "abc";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await settle(320);
    expect(dialog()).toBeTruthy();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    await settle(40);
    expect(dialog()).toBeNull();
    expect(input.value, "Escape lọt xuống và xoá mất truy vấn").toBe("abc");
    expect(arrow(div)).toBeTruthy();
  });

  it("hộp thoại đang mở thì mũi tên bàn phím không điều khiển danh sách", async () => {
    ipc = baseHandlers(FOUND);
    const { div } = await mountApp();
    const input = div.querySelector("input.search") as HTMLInputElement;
    input.value = "a";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await settle(320);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
    await settle(40);
    const sel = [...div.querySelectorAll(".row")].findIndex((r) => r.classList.contains("sel"));
    expect(sel, "phím lọt xuống danh sách trong lúc hộp thoại mở").toBe(0);
  });

  it("nhịp hỏi-lại hằng ngày báo CÙNG một bản: không dựng người ta dậy lần nữa", async () => {
    ipc = baseHandlers(FOUND);
    const { div } = await mountApp();
    click(btn("Để sau"));
    await settle(40);
    await ipc.emit("update-available", null); // backend hỏi lại, vẫn 1.0.4
    await settle(80);
    expect(dialog(), "hộp thoại tự bật lại cho bản đã Để sau").toBeNull();
    expect(arrow(div)).toBeTruthy();
  });

  it("bản MỚI HƠN nữa xuất hiện: hộp thoại được phép quay lại", async () => {
    ipc = baseHandlers(FOUND);
    await mountApp();
    click(btn("Để sau"));
    await settle(40);
    ipc.on("update_status", {
      checked: true,
      available: { version: "1.0.5", notes: null },
      current: "1.0.3",
    });
    await ipc.emit("update-available", null);
    await settle(80);
    expect(dialog(), "bản mới hơn không được mời").toBeTruthy();
    expect(dialog()!.textContent).toContain("1.0.5");
  });

  it("tin về SAU khi cửa sổ đã mở (mạng lên chậm): hộp thoại tự hiện", async () => {
    ipc = baseHandlers(NONE);
    await mountApp();
    expect(dialog()).toBeNull();
    ipc.on("update_status", FOUND);
    await ipc.emit("update-available", null);
    await settle(80);
    expect(dialog(), "cửa sổ đang mở không nhận được tin").toBeTruthy();
  });

  it("bấm Cập nhật: hộp thoại chuyển sang trạng thái đang tải", async () => {
    ipc = baseHandlers(FOUND);
    await mountApp();
    click(btn("Cập nhật"));
    await settle(60);
    expect(dialog()!.textContent).toContain("Đang tải bản 1.0.4");
    expect(btn("Để sau"), "đang tải dở mà vẫn mời Để sau").toBeFalsy();
  });
});
