// Nhóm 6 — Home/End/Ctrl+A ở danh sách tìm kiếm, và bàn phím cho chế độ
// trùng lặp (thứ trước đây rơi thẳng xuống danh sách tìm kiếm đang ẩn).
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import App from "../src/App.svelte";
import type { SearchHit } from "../src/lib/search";

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

const N = 8;
const HITS = Array.from({ length: N }, (_, i) => mkHit(i + 1, `tim${i}.mp4`));

let ipc: IpcRecorder;
let opened: string[];
let revealed: string[];
let dragged: string[][];

function baseHandlers(): IpcRecorder {
  const r = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = r;
  opened = [];
  revealed = [];
  dragged = [];
  r.on("index_status", { loaded: true, fileCount: 100, dirCount: 5, builtAtUnix: 1_700_000_000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", { available: null, current: "1.0.1" })
    .on("search", (a: { id: number }) => ({
      id: a.id,
      hits: HITS,
      epoch: 3,
      relaxed: null,
      elapsedMs: 1,
      total: N,
    }))
    // Hai nhóm × hai tệp: đủ để mũi tên phải NHẢY QUA một dòng tiêu đề.
    .on("dupe_progress", { running: false, completed: true, groups: 2, wasted: 1024, hashed: 4, candidates: 4 })
    .on("dupe_groups", [
      { size: 512, wasted: 512, files: [mkHit(11, "g1a.mp4"), mkHit(12, "g1b.mp4")] },
      { size: 256, wasted: 256, files: [mkHit(21, "g2a.mp4"), mkHit(22, "g2b.mp4")] },
    ])
    .on("cancel_duplicates", null)
    .on("open_file", (a: { path: string }) => {
      opened.push(a.path);
      return null;
    })
    .on("reveal_in_explorer", (a: { path: string }) => {
      revealed.push(a.path);
      return null;
    })
    .on("start_file_drag", (a: { paths: string[] }) => {
      dragged.push(a.paths);
      return null;
    })
    .on("find_duplicates", null);
  return r;
}

// Moi App con song sau mot ca do van nghe phim tren window va lam nhiem cac
// ca sau — afterEach don sach de mot that bai chi la MOT that bai.
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

/// Dua con tro ra khoi o tim kiem — jsdom 30 thuc thi autofocus, nen sau khi
/// mount thi o nhap dang giu focus, va cac phim Home/End/Ctrl+A theo thiet ke
/// se nhuong cho viec sua chu. Focus vao mot dong ket qua de mo phong nguoi
/// dung vua bam chuot vao danh sach.
function focusRow(div: Element, i = 0) {
  ($$(div, ".row")[i] as HTMLElement).focus();
}

async function search(div: Element, q = "tim") {
  const input = div.querySelector("input.search") as HTMLInputElement;
  input.value = q;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await settle(320);
}

const $$ = (root: Element, sel: string) => [...root.querySelectorAll(sel)];
const chip = (root: Element, text: string) =>
  $$(root, "button").find((b) => b.textContent!.trim().startsWith(text)) as HTMLButtonElement;
const rows = (root: Element) => $$(root, ".row");
const selIndexes = (root: Element) =>
  rows(root)
    .map((r, i) => (r.classList.contains("sel") ? i : -1))
    .filter((i) => i >= 0);

function key(k: string, opts: KeyboardEventInit = {}): KeyboardEvent {
  const e = new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...opts });
  window.dispatchEvent(e);
  return e;
}

beforeEach(() => {
  localStorage.clear();
  ipc = baseHandlers();
});

// ================================================================ mục 5

describe("danh sách tìm kiếm — Home/End/Ctrl+A", () => {
  it("End nhảy tới dòng cuối, Home quay về dòng đầu", async () => {
    const { div, cleanup } = await mountApp();
    await search(div);
    focusRow(div);
    key("End");
    await settle(40);
    expect(selIndexes(div)).toEqual([N - 1]);
    key("Home");
    await settle(40);
    expect(selIndexes(div)).toEqual([0]);
    cleanup();
  });

  it("Shift+End mở rộng dải từ chỗ neo tới cuối", async () => {
    const { div, cleanup } = await mountApp();
    await search(div);
    focusRow(div);
    key("ArrowDown");
    key("ArrowDown"); // neo = 2
    await settle(40);
    key("End", { shiftKey: true });
    await settle(40);
    expect(selIndexes(div)).toEqual([2, 3, 4, 5, 6, 7]);
    cleanup();
  });

  it("con trỏ trong ô tìm kiếm: Home KHÔNG bị cướp — đó là 'về đầu dòng chữ'", async () => {
    const { div, cleanup } = await mountApp();
    await search(div);
    key("End", { ctrlKey: true }); // Ctrl+End hop le ca khi o nhap giu con tro
    await settle(40);
    (div.querySelector("input.search") as HTMLInputElement).focus();
    const e = key("Home");
    await settle(40);
    expect(e.defaultPrevented, "phím Home của ô nhập bị chặn").toBe(false);
    expect(selIndexes(div), "danh sách tự nhảy dù người dùng đang sửa chữ").toEqual([N - 1]);
    cleanup();
  });

  it("con trỏ trong ô tìm kiếm: Ctrl+Home vẫn là lệnh của danh sách", async () => {
    const { div, cleanup } = await mountApp();
    await search(div);
    key("End", { ctrlKey: true });
    await settle(40);
    (div.querySelector("input.search") as HTMLInputElement).focus();
    key("Home", { ctrlKey: true });
    await settle(40);
    expect(selIndexes(div)).toEqual([0]);
    cleanup();
  });

  it("Ctrl+A ngoài ô nhập: chọn hết kết quả, con trỏ bàn phím đứng yên", async () => {
    const { div, cleanup } = await mountApp();
    await search(div);
    focusRow(div);
    key("ArrowDown"); // dung o dong 1
    await settle(40);
    key("a", { ctrlKey: true });
    await settle(40);
    expect(selIndexes(div).length).toBe(N);
    const focused = rows(div).findIndex((r) => r.classList.contains("focused"));
    expect(focused, "con trỏ bàn phím bị Ctrl+A dời đi").toBe(1);
    cleanup();
  });

  it("Ctrl+A khi ô nhập giữ con trỏ: nhường cho việc chọn chữ", async () => {
    const { div, cleanup } = await mountApp();
    await search(div);
    (div.querySelector("input.search") as HTMLInputElement).focus();
    const e = key("a", { ctrlKey: true });
    await settle(40);
    expect(e.defaultPrevented).toBe(false);
    expect(selIndexes(div), "danh sách bị chọn hết trong khi người dùng muốn chọn chữ").toEqual([0]);
    cleanup();
  });

  it("Ctrl+A rồi kéo một dòng trong tập: kéo đi TẤT CẢ", async () => {
    const { div, cleanup } = await mountApp();
    await search(div);
    focusRow(div);
    key("a", { ctrlKey: true });
    await settle(40);
    rows(div)[3].dispatchEvent(new MouseEvent("dragstart", { bubbles: true, cancelable: true }));
    await settle(80);
    expect(dragged.length).toBe(1);
    expect(dragged[0].length).toBe(N);
    cleanup();
  });
});

// ================================================================ mục 4

async function enterDupes(div: Element) {
  chip(div, "Trùng lặp").dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  await settle(150);
}

describe("chế độ trùng lặp — bàn phím", () => {
  it("vào chế độ: tệp đầu tiên được chọn sẵn", async () => {
    const { div, cleanup } = await mountApp();
    await enterDupes(div);
    const dupeRows = $$(div, ".row.dupe");
    expect(dupeRows.length).toBe(4);
    expect(dupeRows[0].classList.contains("sel")).toBe(true);
    cleanup();
  });

  it("ArrowDown nhảy QUA dòng tiêu đề nhóm, không dừng lại trên nó", async () => {
    const { div, cleanup } = await mountApp();
    await enterDupes(div);
    key("ArrowDown"); // g1a -> g1b
    key("ArrowDown"); // g1b -> g2a (băng qua tiêu đề nhóm 2)
    await settle(40);
    const dupeRows = $$(div, ".row.dupe");
    expect(dupeRows.findIndex((r) => r.classList.contains("sel"))).toBe(2);
    cleanup();
  });

  it("mũi tên bị kẹp ở hai đầu danh sách", async () => {
    const { div, cleanup } = await mountApp();
    await enterDupes(div);
    key("ArrowUp");
    await settle(40);
    expect($$(div, ".row.dupe")[0].classList.contains("sel")).toBe(true);
    for (let i = 0; i < 10; i++) key("ArrowDown");
    await settle(40);
    expect($$(div, ".row.dupe")[3].classList.contains("sel")).toBe(true);
    cleanup();
  });

  it("End/Home nhảy tới tệp cuối/đầu", async () => {
    const { div, cleanup } = await mountApp();
    await enterDupes(div);
    key("End");
    await settle(40);
    expect($$(div, ".row.dupe")[3].classList.contains("sel")).toBe(true);
    key("Home");
    await settle(40);
    expect($$(div, ".row.dupe")[0].classList.contains("sel")).toBe(true);
    cleanup();
  });

  it("Enter mở đúng tệp trùng lặp đang chọn — KHÔNG phải tệp của danh sách tìm kiếm đang ẩn", async () => {
    const { div, cleanup } = await mountApp();
    await search(div); // danh sách ẩn giờ có tim0.mp4 đứng đầu
    await enterDupes(div);
    key("ArrowDown");
    await settle(40);
    key("Enter");
    await settle(80);
    // Trước bản sửa, phím này mở tim0.mp4 — một tệp không hề có trên màn hình.
    expect(opened).toEqual(["D:\\m\\g1b.mp4"]);
    cleanup();
  });

  it("Ctrl+Enter mở thư mục chứa tệp đang chọn", async () => {
    const { div, cleanup } = await mountApp();
    await enterDupes(div);
    key("Enter", { ctrlKey: true });
    await settle(80);
    expect(revealed).toEqual(["D:\\m\\g1a.mp4"]);
    expect(opened).toEqual([]);
    cleanup();
  });

  it("bấm chuột vào một dòng thì con trỏ chuyển tới dòng đó", async () => {
    const { div, cleanup } = await mountApp();
    await enterDupes(div);
    $$(div, ".row.dupe")[2].dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await settle(40);
    expect($$(div, ".row.dupe")[2].classList.contains("sel")).toBe(true);
    key("ArrowDown");
    await settle(40);
    expect(
      $$(div, ".row.dupe")[3].classList.contains("sel"),
      "mũi tên không tiếp nối từ chỗ vừa bấm",
    ).toBe(true);
    cleanup();
  });

  it("Escape thoát chế độ trùng lặp; Escape tiếp theo mới xoá truy vấn", async () => {
    const { div, cleanup } = await mountApp();
    await search(div);
    await enterDupes(div);
    expect(div.querySelector(".dupebar")).toBeTruthy();
    key("Escape");
    await settle(120);
    expect(div.querySelector(".dupebar"), "Escape không thoát chế độ trùng lặp").toBeFalsy();
    expect((div.querySelector("input.search") as HTMLInputElement).value, "truy vấn bị xoá oan").toBe("tim");
    key("Escape");
    await settle(60);
    expect((div.querySelector("input.search") as HTMLInputElement).value).toBe("");
    cleanup();
  });

  it("đang có băng lỗi thì Escape đóng lỗi trước, chưa thoát chế độ", async () => {
    ipc.on("dupe_progress", () => {
      throw new Error("hong");
    });
    ipc.on("find_duplicates", () => {
      throw new Error("Không đọc được chỉ mục.");
    });
    const { div, cleanup } = await mountApp();
    await enterDupes(div);
    expect(div.querySelector(".error")).toBeTruthy();
    key("Escape");
    await settle(60);
    expect(div.querySelector(".error"), "băng lỗi không đóng").toBeFalsy();
    expect(div.querySelector(".dupebar"), "thoát chế độ ngay trong lần Escape đầu").toBeTruthy();
    cleanup();
  });

  it("danh sách trống (chưa quét xong) thì mũi tên và Enter im lặng, không nổ", async () => {
    ipc.on("dupe_progress", { running: true, completed: false, groups: 0, wasted: 0, hashed: 1, candidates: 9 });
    const { div, cleanup } = await mountApp();
    await enterDupes(div);
    key("ArrowDown");
    key("Enter");
    await settle(60);
    expect(opened).toEqual([]);
    cleanup();
  });
});
