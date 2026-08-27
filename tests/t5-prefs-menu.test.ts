// Nhóm 5 — tuỳ chọn được lưu giữa các phiên, và menu chuột phải ở chế độ
// trùng lặp không còn mục chết.
//
// Khác bốn nhóm chuyển từ harness cũ, nhóm này viết mới: mỗi ca một `it`.
import { beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import { loadPrefs, savePrefs, type Prefs } from "../src/lib/prefs";
import App from "../src/App.svelte";
import type { SearchHit } from "../src/lib/search";

const KEY = "mediafinder:prefs";

// ---------------------------------------------------------------- phần đơn vị

describe("prefs — đọc/ghi và phòng dữ liệu hỏng", () => {
  beforeEach(() => localStorage.clear());

  it("kho rỗng thì trả mặc định", () => {
    expect(loadPrefs()).toEqual({ grid: false, order: "relevance", activeKinds: [] });
  });

  it("lưu rồi đọc lại thì y nguyên", () => {
    const p: Prefs = { grid: true, order: "newest", activeKinds: ["video", "audio"] };
    savePrefs(p);
    expect(loadPrefs()).toEqual(p);
  });

  it("JSON hỏng thì về mặc định, không ném lỗi", () => {
    localStorage.setItem(KEY, "{khong phai json");
    expect(loadPrefs()).toEqual({ grid: false, order: "relevance", activeKinds: [] });
  });

  it("JSON hợp lệ nhưng không phải object thì về mặc định", () => {
    localStorage.setItem(KEY, "42");
    expect(loadPrefs()).toEqual({ grid: false, order: "relevance", activeKinds: [] });
  });

  it("order lạ thì rơi về relevance — backend chưa từng hứa xử lý giá trị khác", () => {
    localStorage.setItem(KEY, JSON.stringify({ grid: true, order: "oldest", activeKinds: [] }));
    expect(loadPrefs().order).toBe("relevance");
    expect(loadPrefs().grid).toBe(true); // trường hỏng không kéo trường lành theo
  });

  it("kinds lạ bị lọc bỏ, kinds hợp lệ giữ lại", () => {
    localStorage.setItem(
      KEY,
      JSON.stringify({ activeKinds: ["video", "document", 7, null, "audio"] }),
    );
    expect(loadPrefs().activeKinds).toEqual(["video", "audio"]);
  });

  it("activeKinds không phải mảng thì về rỗng", () => {
    localStorage.setItem(KEY, JSON.stringify({ activeKinds: "video" }));
    expect(loadPrefs().activeKinds).toEqual([]);
  });

  it("grid không phải boolean thì về false", () => {
    localStorage.setItem(KEY, JSON.stringify({ grid: "yes" }));
    expect(loadPrefs().grid).toBe(false);
  });
});

// ---------------------------------------------------------------- phần tích hợp

function mkHit(i: number, name: string, kind: SearchHit["kind"] = "video"): SearchHit {
  return {
    index: i,
    name,
    dir: "D:\\m",
    path: `D:\\m\\${name}`,
    kind,
    matched: 1,
    size: 4096,
    width: 1920,
    height: 1080,
    durationMs: 60_000,
  } as SearchHit;
}

let ipc: IpcRecorder;

function baseHandlers(): IpcRecorder {
  const r = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = r;
  r.on("index_status", { loaded: true, fileCount: 100, dirCount: 5, builtAtUnix: 1_700_000_000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", { available: null, current: "1.0.1" })
    .on("search", (a: { id: number }) => ({
      id: a.id,
      hits: [mkHit(1, "a.mp4"), mkHit(2, "b.mp4")],
      epoch: 3,
      relaxed: null,
      elapsedMs: 1,
      total: 2,
    }))
    .on("dupe_progress", { running: false, completed: true, groups: 1, wasted: 512, hashed: 2, candidates: 2 })
    .on("dupe_groups", [
      { size: 512, wasted: 512, files: [mkHit(11, "x.mp4"), mkHit(12, "y.mp4")] },
    ])
    .on("cancel_duplicates", null)
    .on("open_file", null)
    .on("reveal_in_explorer", null);
  return r;
}

async function mountApp() {
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  await settle(90);
  return {
    div,
    cleanup: () => {
      unmount(app);
      div.remove();
    },
  };
}

const $$ = (root: Element, sel: string) => [...root.querySelectorAll(sel)];
const chip = (root: Element, text: string) =>
  $$(root, "button").find((b) => b.textContent!.trim().startsWith(text)) as HTMLButtonElement | undefined;
const click = (el: Element | undefined) => {
  expect(el, "selector không khớp nút nào").toBeTruthy();
  el!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
};

describe("prefs — App áp dụng và ghi lại", () => {
  beforeEach(() => {
    localStorage.clear();
    ipc = baseHandlers();
  });

  it("phiên mới với prefs đã lưu: lưới bật, xếp Mới nhất, chip Video sáng", async () => {
    savePrefs({ grid: true, order: "newest", activeKinds: ["video"] });
    const { div, cleanup } = await mountApp();
    const gridBtn = $$(div, "button").find((b) =>
      (b.getAttribute("aria-label") ?? "").includes("danh sách"),
    );
    // aria-label nói hành động kế tiếp: đang ở lưới thì nút mời "Chuyển sang danh sách"
    expect(gridBtn, "nút lưới không ở trạng thái bật").toBeTruthy();
    expect(chip(div, "Mới nhất"), "chip sắp xếp không hiện 'Mới nhất'").toBeTruthy();
    expect(chip(div, "Video")?.classList.contains("on"), "chip Video không sáng").toBe(true);
    cleanup();
  });

  it("tìm kiếm ngay sau khi mở dùng đúng kinds và order đã lưu", async () => {
    savePrefs({ grid: false, order: "newest", activeKinds: ["image"] });
    let seen: { kinds: string[]; order: string } | null = null;
    ipc.on("search", (a: { id: number; req: { kinds: string[]; order: string } }) => {
      seen = { kinds: a.req.kinds, order: a.req.order };
      return { id: a.id, hits: [], epoch: 1, relaxed: null, elapsedMs: 1, total: 0 };
    });
    const { div, cleanup } = await mountApp();
    const input = div.querySelector("input.search") as HTMLInputElement;
    input.value = "anh";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await settle(320);
    expect(seen).toEqual({ kinds: ["image"], order: "newest" });
    cleanup();
  });

  it("bật lưới thì được ghi xuống ngay, không đợi đóng app", async () => {
    const { div, cleanup } = await mountApp();
    const gridBtn = $$(div, "button").find((b) =>
      (b.getAttribute("aria-label") ?? "").includes("lưới"),
    );
    click(gridBtn);
    await settle(60);
    expect(loadPrefs().grid).toBe(true);
    cleanup();
  });

  it("bật chip Nhạc rồi đổi sang Mới nhất: cả hai cùng nằm trong prefs", async () => {
    const { div, cleanup } = await mountApp();
    click(chip(div, "Nhạc"));
    await settle(60);
    click(chip(div, "Liên quan"));
    await settle(60);
    expect(loadPrefs()).toEqual({ grid: false, order: "newest", activeKinds: ["audio"] });
    cleanup();
  });

  it("phiên sau mở lại thấy đúng lựa chọn của phiên trước", async () => {
    // Phiên 1: người dùng bật lưới và lọc Ảnh, rồi đóng app.
    const one = await mountApp();
    click(
      $$(one.div, "button").find((b) => (b.getAttribute("aria-label") ?? "").includes("lưới")),
    );
    click(chip(one.div, "Ảnh"));
    await settle(60);
    one.cleanup();

    // Phiên 2: mở lại — không bấm gì cả, lưới và chip Ảnh phải đã sẵn sàng.
    const two = await mountApp();
    expect(
      $$(two.div, "button").some((b) =>
        (b.getAttribute("aria-label") ?? "").includes("danh sách"),
      ),
      "lưới không được khôi phục",
    ).toBe(true);
    expect(chip(two.div, "Ảnh")?.classList.contains("on"), "chip Ảnh không sáng lại").toBe(true);
    two.cleanup();
  });

  it("kho prefs chứa rác thì app vẫn mở được với mặc định", async () => {
    localStorage.setItem(KEY, "%%%");
    const { div, cleanup } = await mountApp();
    expect(div.querySelector("input.search"), "app không dựng được giao diện").toBeTruthy();
    expect(chip(div, "Liên quan"), "order không về mặc định").toBeTruthy();
    cleanup();
  });
});

describe("tin cập nhật về SAU khi cửa sổ đã mở", () => {
  beforeEach(() => {
    localStorage.clear();
    ipc = baseHandlers();
  });

  it("sự kiện update-available làm băng cập nhật hiện ra không cần mở lại", async () => {
    // Mở cửa sổ khi backend còn chưa hỏi được máy chủ (mạng lên chậm sau
    // đăng nhập) — không có băng nào cả.
    const { div, cleanup } = await mountApp();
    expect(document.querySelector("[role=dialog]")).toBeNull();

    // Backend thử lại thành công, ghi kết quả rồi bắn sự kiện.
    ipc.on("update_status", {
      checked: true,
      available: { version: "1.0.2", notes: null },
      current: "1.0.1",
    });
    await ipc.emit("update-available", null);
    await settle(80);

    const dlg = document.querySelector("[role=dialog]");
    expect(dlg, "cửa sổ đang mở phải nhận được tin, không chờ lần mở sau").toBeTruthy();
    expect(dlg!.textContent).toContain("1.0.2");
    cleanup();
  });
});

describe("menu chuột phải — mục Xem trước", () => {
  beforeEach(() => {
    localStorage.clear();
    ipc = baseHandlers();
  });

  const menuLabels = () =>
    $$(document.body, "[role=menu] .label").map((l) => l.textContent!.trim());

  it("chế độ tìm kiếm: menu CÓ 'Xem trước'", async () => {
    const { div, cleanup } = await mountApp();
    const input = div.querySelector("input.search") as HTMLInputElement;
    input.value = "a";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await settle(320);
    const row = div.querySelector(".row");
    expect(row, "không có dòng kết quả").toBeTruthy();
    row!.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
    await settle(60);
    expect(menuLabels()).toContain("Xem trước");
    cleanup();
  });

  it("chế độ trùng lặp: menu KHÔNG có 'Xem trước' — mục đó từng bấm vào mà không có gì xảy ra", async () => {
    const { div, cleanup } = await mountApp();
    click(chip(div, "Trùng lặp"));
    await settle(150);
    const row = div.querySelector(".row.dupe");
    expect(row, "không có dòng trùng lặp").toBeTruthy();
    row!.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
    await settle(60);
    const labels = menuLabels();
    expect(labels).not.toContain("Xem trước");
    expect(labels).toContain("Mở tệp");
    expect(labels).toContain("Mở thư mục chứa tệp");
    expect(labels).toContain("Sao chép đường dẫn");
    cleanup();
  });

  it("chế độ trùng lặp: 'Mở tệp' trong menu mở đúng tệp được bấm", async () => {
    const opened: string[] = [];
    ipc.on("open_file", (a: { path: string }) => {
      opened.push(a.path);
      return null;
    });
    const { div, cleanup } = await mountApp();
    click(chip(div, "Trùng lặp"));
    await settle(150);
    const second = $$(div, ".row.dupe")[1];
    expect(second, "thiếu dòng trùng lặp thứ hai").toBeTruthy();
    second.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
    await settle(60);
    const item = $$(document.body, "[role=menu] .label").find(
      (l) => l.textContent!.trim() === "Mở tệp",
    );
    click(item?.closest("button") ?? undefined);
    await settle(80);
    expect(opened).toEqual(["D:\\m\\y.mp4"]);
    cleanup();
  });
});
