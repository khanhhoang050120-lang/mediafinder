// Nhóm 10 — hai mảnh giao diện của đợt backend A–D: bộ ghi truy-vấn-0-kết-quả
// (hiện đúng lúc "Không tìm thấy kết quả nào") và nút Xác minh tầng-3 của
// chế độ trùng lặp.
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

let ipc: IpcRecorder;
let missEnabled: boolean;
let missCount: number;

function baseHandlers(): IpcRecorder {
  const r = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = r;
  missEnabled = false;
  missCount = 0;
  r.on("index_status", { loaded: true, fileCount: 100, dirCount: 5, builtAtUnix: 1_700_000_000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
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
    .on("miss_log_status", () => ({ enabled: missEnabled, count: missCount }))
    .on("miss_log_set_enabled", (a: { enabled: boolean }) => {
      missEnabled = a.enabled;
      return null;
    })
    .on("miss_log_clear", () => {
      missCount = 0;
      return null;
    })
    .on("miss_log_open", null)
    .on("dupe_progress", { running: false, completed: true, groups: 1, wasted: 8192, hashed: 3, candidates: 3 })
    .on("dupe_groups", [
      {
        size: 4096,
        wasted: 8192,
        files: [mkHit(11, "a.mp4"), mkHit(12, "b.mp4"), mkHit(13, "c.mp4")],
      },
    ])
    .on("cancel_duplicates", null)
    .on("find_duplicates", null);
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

const $$ = (root: Element, sel: string) => [...root.querySelectorAll(sel)];
const btn = (root: Element, text: string) =>
  $$(root, "button").find((b) => b.textContent!.trim().startsWith(text)) as
    | HTMLButtonElement
    | undefined;
const click = (el: Element | undefined) => {
  expect(el, "phần tử cần bấm không tồn tại").toBeTruthy();
  el!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
};

async function searchZero(div: Element) {
  const input = div.querySelector("input.search") as HTMLInputElement;
  input.value = "khong co gi";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await settle(320);
}

beforeEach(() => {
  localStorage.clear();
  ipc = baseHandlers();
});

// ================================================================ miss-log

describe("bộ ghi truy-vấn-0-kết-quả — sống trong màn không-tìm-thấy", () => {
  it("mặc định tắt: mời Bật ngay tại chỗ đau, nói rõ chỉ-lưu-trên-máy", async () => {
    const { div } = await mountApp();
    await searchZero(div);
    const block = div.querySelector(".misslog");
    expect(block, "khối điều khiển không hiện ở màn không-tìm-thấy").toBeTruthy();
    expect(block!.textContent).toContain("chỉ lưu trên máy này");
    expect(btn(div, "Bật ghi"), "thiếu nút Bật").toBeTruthy();
    expect(btn(div, "Xem"), "chưa bật mà đã có nút Xem").toBeFalsy();
  });

  it("Bật ghi → backend nhận enabled=true, khối chuyển sang trạng thái đang-ghi", async () => {
    const { div } = await mountApp();
    await searchZero(div);
    click(btn(div, "Bật ghi"));
    await settle(60);
    expect(ipc.count("miss_log_set_enabled")).toBe(1);
    expect(div.querySelector(".misslog")!.textContent).toContain("Đang ghi");
    expect(btn(div, "Tắt")).toBeTruthy();
  });

  it("đang ghi, đã có 3 mục: Xem gọi mở file, Xoá gọi xoá rồi đếm về 0", async () => {
    missEnabled = true;
    missCount = 3;
    const { div } = await mountApp();
    await searchZero(div);
    expect(div.querySelector(".misslog b")!.textContent).toBe("3");

    click(btn(div, "Xem"));
    await settle(40);
    expect(ipc.count("miss_log_open")).toBe(1);

    click(btn(div, "Xoá"));
    await settle(60);
    expect(ipc.count("miss_log_clear")).toBe(1);
    expect(div.querySelector(".misslog b")!.textContent, "đếm không về 0 sau khi xoá").toBe("0");
  });

  it("đang ghi nhưng chưa có gì: Xem và Xoá bị vô hiệu — nút bấm-mà-không-làm-gì là nói dối", async () => {
    missEnabled = true;
    missCount = 0;
    const { div } = await mountApp();
    await searchZero(div);
    expect(btn(div, "Xem")?.disabled).toBe(true);
    expect(btn(div, "Xoá")?.disabled).toBe(true);
  });
});

// ================================================================ verify

async function enterDupes(div: Element) {
  click(btn(div, "Trùng lặp"));
  await settle(150);
}

describe("tầng 3 trùng lặp — nút Xác minh trên từng nhóm", () => {
  it("nhóm chưa xác minh có nút; bấm → gửi đúng danh sách đường dẫn của nhóm", async () => {
    let got: string[] | null = null;
    ipc.on("verify_dupe_group", (a: { paths: string[] }) => {
      got = a.paths;
      return { groups: [a.paths], unreadable: [] };
    });
    const { div } = await mountApp();
    await enterDupes(div);
    click(btn(div, "Xác minh"));
    await settle(60);
    expect(got).toEqual(["D:\\m\\a.mp4", "D:\\m\\b.mp4", "D:\\m\\c.mp4"]);
  });

  it("tất cả trùng từng byte: badge xanh thay chỗ nút, không tệp nào bị gắn tag", async () => {
    ipc.on("verify_dupe_group", (a: { paths: string[] }) => ({
      groups: [a.paths],
      unreadable: [],
    }));
    const { div } = await mountApp();
    await enterDupes(div);
    click(btn(div, "Xác minh"));
    await settle(60);
    expect(div.querySelector(".vok")?.textContent).toContain("trùng từng byte");
    expect(btn(div, "Xác minh"), "đã có phán quyết mà nút vẫn mời bấm").toBeFalsy();
    expect($$(div, ".vtag").length).toBe(0);
  });

  it("một tệp khác nội dung + một tệp không đọc được: badge đỏ, tag đúng từng tệp", async () => {
    ipc.on("verify_dupe_group", () => ({
      groups: [["D:\\m\\a.mp4", "D:\\m\\b.mp4"]],
      unreadable: ["D:\\m\\c.mp4"],
    }));
    // Kịch bản gộp: b thật ra khác nội dung → cụm lớn chỉ còn a… dùng dữ liệu
    // sát thực hơn: a+b trùng, c không đọc được — rồi kịch bản thứ hai bên dưới.
    const { div } = await mountApp();
    await enterDupes(div);
    click(btn(div, "Xác minh"));
    await settle(60);
    expect(div.querySelector(".vbad")?.textContent).toContain("khác nội dung");
    const tags = $$(div, ".vtag").map((t) => t.textContent!.trim());
    expect(tags).toEqual(["không đọc được"]);
  });

  it("kẻ giả dạng bị tách cụm: đúng tệp đó mang tag 'khác nội dung'", async () => {
    ipc.on("verify_dupe_group", () => ({
      groups: [
        ["D:\\m\\a.mp4", "D:\\m\\c.mp4"],
        ["D:\\m\\b.mp4"],
      ],
      unreadable: [],
    }));
    const { div } = await mountApp();
    await enterDupes(div);
    click(btn(div, "Xác minh"));
    await settle(60);
    const rows = $$(div, ".row.dupe");
    const tagOf = (i: number) => rows[i].querySelector(".vtag")?.textContent?.trim() ?? null;
    expect(tagOf(0), "a là bản thật, không được nghi oan").toBeNull();
    expect(tagOf(1), "b là kẻ giả dạng, phải bị chỉ mặt").toBe("khác nội dung");
    expect(tagOf(2)).toBeNull();
  });

  it("đang xác minh: nút vô hiệu và nói rõ đang chạy", async () => {
    let release!: (v: { groups: string[][]; unreadable: string[] }) => void;
    ipc.on("verify_dupe_group", () => new Promise((res) => (release = res)));
    const { div } = await mountApp();
    await enterDupes(div);
    click(btn(div, "Xác minh"));
    await settle(40);
    const b = btn(div, "Đang xác minh");
    expect(b, "không thấy trạng thái đang chạy").toBeTruthy();
    expect(b!.disabled).toBe(true);
    release({ groups: [[]], unreadable: [] });
    await settle(40);
  });
});
