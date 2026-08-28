// Nhóm 13 — "Quét lại" nói cho biết nó vừa làm gì.
//
// Khác nút "+ ổ mạng" (hỏi trước, vì tốn vài phút): quét ổ trong máy chỉ tốn
// vài giây, nên chặn bằng hộp thoại là bắt bấm hai lần cho một việc chớp mắt.
// Thay vào đó: tooltip nói lần gần nhất, và một dòng ngắn sau khi xong nói
// lượt vừa rồi đổi những gì.
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import App from "../src/App.svelte";

/// Backend trả metadata này ở lần `index_status` đầu tiên.
let metaBefore = {
  loaded: true,
  fileCount: 48_320,
  dirCount: 3_211,
  memoryBytes: 0,
  builtAtUnix: 1_772_000_000,
  problem: null as string | null,
};
/// …và cái này sau khi `reload_index` chạy xong.
let metaAfter = { ...metaBefore, fileCount: 48_332 };
/// Lượt quét đã xong chưa — điều khiển nhịp poll của ScanState.
let scanDone = false;

let ipc: IpcRecorder;

function baseHandlers(): IpcRecorder {
  const r = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = r;
  scanDone = false;
  r.on("index_status", () => metaBefore)
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("network_drives", [])
    .on("update_status", { checked: true, available: null, current: "1.0.5" })
    .on("search", (a: { id: number }) => ({
      id: a.id,
      hits: [],
      epoch: 3,
      relaxed: null,
      elapsedMs: 1,
      total: 0,
    }))
    .on("request_scan", () => {
      scanDone = true;
      return null;
    })
    .on("scan_progress", () =>
      scanDone
        ? {
            scanning: true,
            progress: {
              message: "xong",
              phase: "done",
              volumesDone: 2,
              volumesTotal: 2,
              finished: true,
              error: null,
            },
          }
        : { scanning: false, progress: null },
    )
    .on("reload_index", () => metaAfter)
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
const rescanBtn = (root: ParentNode) =>
  $$(root, "button").find((b) => b.textContent!.trim().startsWith("Quét lại")) as
    | HTMLButtonElement
    | undefined;
const outcome = (root: ParentNode) => root.querySelector(".outcome");

/// Bấm Quét lại rồi chờ ScanState poll xong (nhịp 250ms) và reload.
async function rescanAndSettle(div: Element) {
  rescanBtn(div)!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  await settle(500);
}

beforeEach(() => {
  localStorage.clear();
  metaBefore = {
    loaded: true,
    fileCount: 48_320,
    dirCount: 3_211,
    memoryBytes: 0,
    builtAtUnix: 1_772_000_000,
    problem: null,
  };
  metaAfter = { ...metaBefore, fileCount: 48_332 };
  ipc = baseHandlers();
});

describe("tooltip nút Quét lại — nói lần gần nhất, không chặn đường", () => {
  it("tooltip kèm thời điểm và số tệp của lần quét gần nhất", async () => {
    const { div } = await mountApp();
    const title = rescanBtn(div)!.title;
    expect(title, "mất lời mô tả gốc của nút").toContain("vài giây");
    expect(title, "thiếu nhắc lần gần nhất").toContain("Lần gần nhất");
    expect(title).toContain("48.320");
  });

  it("chưa từng quét: tooltip không bịa ra lần nào cả", async () => {
    metaBefore = { ...metaBefore, loaded: false, fileCount: 0, builtAtUnix: 0 };
    const { div } = await mountApp();
    const title = rescanBtn(div)!.title;
    expect(title).toContain("vài giây");
    expect(title, "chưa quét bao giờ mà vẫn nói 'lần gần nhất'").not.toContain("Lần gần nhất");
  });

  it("bấm Quét lại KHÔNG hỏi — nó chỉ tốn vài giây", async () => {
    const { div } = await mountApp();
    rescanBtn(div)!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    await settle(100);
    expect(document.querySelector("[role=dialog]"), "chặn một việc chớp mắt bằng hộp thoại").toBeNull();
    expect(ipc.count("request_scan")).toBe(1);
  });
});

describe("dòng kết quả sau khi quét xong", () => {
  it("có tệp mới: nói thêm bao nhiêu", async () => {
    const { div } = await mountApp();
    expect(outcome(div), "chưa quét mà đã khoe kết quả").toBeNull();
    await rescanAndSettle(div);
    expect(outcome(div)?.textContent).toBe("Đã quét lại · thêm 12 tệp");
  });

  it("tệp bị xoá bớt: nói bớt bao nhiêu", async () => {
    metaAfter = { ...metaBefore, fileCount: 48_300 };
    const { div } = await mountApp();
    await rescanAndSettle(div);
    expect(outcome(div)?.textContent).toBe("Đã quét lại · bớt 20 tệp");
  });

  it("không có gì đổi: vẫn nói — im lặng thì người dùng không biết nó đã chạy", async () => {
    metaAfter = { ...metaBefore };
    const { div } = await mountApp();
    await rescanAndSettle(div);
    expect(outcome(div)?.textContent).toBe("Đã quét lại · không có gì đổi");
  });

  it("dòng này tự tắt — nó là tin, không phải trạng thái thường trực", async () => {
    const { div } = await mountApp();
    await rescanAndSettle(div);
    expect(outcome(div)).toBeTruthy();
    await settle(8200);
    expect(outcome(div), "tin cũ đứng mãi ở thanh trạng thái").toBeNull();
  });

  it("bắt đầu lượt mới thì dọn tin của lượt trước", async () => {
    const { div } = await mountApp();
    await rescanAndSettle(div);
    expect(outcome(div)).toBeTruthy();
    // Lượt hai: bấm xong, trước khi nó kịp xong, tin cũ phải biến mất ngay.
    scanDone = false;
    rescanBtn(div)!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    await settle(30);
    expect(outcome(div), "tin của lượt trước còn treo trong lúc lượt mới chạy").toBeNull();
  });
});
