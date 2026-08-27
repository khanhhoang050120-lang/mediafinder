// Nhóm 7 — đường ống thumbnail: cửa xoay ưu tiên (GĐ2), thử lại lỗi tạm
// (GĐ1, phía giao diện), và tải trước theo hướng cuộn (GĐ3).
//
// Phần Rust của GĐ1 (miss-cache, mã 503) nằm ngoài tầm jsdom — được kiểm bằng
// `cargo check` và đọc lại; phần giao diện ở đây kiểm đúng hành vi mà backend
// trông đợi: "hỏi lại khi đứng yên".
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import {
  acquireThumbSlot,
  prefetchThumb,
  resetThumbQueueForTest,
} from "../src/lib/thumbQueue";
import App from "../src/App.svelte";
import type { SearchHit } from "../src/lib/search";

// ================================================================ đơn vị

describe("thumbQueue — cửa xoay", () => {
  beforeEach(() => resetThumbQueueForTest());

  /// Xin một chỗ và ghi lại thứ tự được cấp.
  function tracked(log: string[], name: string, low = false) {
    let done: (() => void) | null = null;
    const cancel = acquireThumbSlot((d) => {
      log.push(name);
      done = d;
    }, low);
    return { cancel, done: () => done?.() };
  }

  it("tối đa 8 ảnh chạy cùng lúc; trả chỗ thì người kế tiếp được vào", () => {
    const log: string[] = [];
    const holders = Array.from({ length: 10 }, (_, i) => tracked(log, `h${i}`));
    expect(log.length).toBe(8);
    holders[0].done();
    expect(log.length).toBe(9);
    holders[1].done();
    expect(log.length).toBe(10);
  });

  it("ô đang hiện xếp LIFO: cái vừa lọt vào mắt đi trước", () => {
    const log: string[] = [];
    const holders = Array.from({ length: 8 }, (_, i) => tracked(log, `h${i}`));
    tracked(log, "cũ");
    tracked(log, "giữa");
    tracked(log, "mới nhất");
    expect(log.length).toBe(8); // ba người mới đều phải chờ
    holders[0].done();
    holders[1].done();
    holders[2].done();
    expect(log.slice(8)).toEqual(["mới nhất", "giữa", "cũ"]);
  });

  it("tải trước không bao giờ tranh chỗ của ô đang hiện", () => {
    const log: string[] = [];
    const holders = Array.from({ length: 8 }, (_, i) => tracked(log, `h${i}`));
    tracked(log, "đoán trước", true); // xếp hàng TRƯỚC ô đang hiện
    tracked(log, "đang nhìn");
    holders[0].done();
    expect(log[8], "prefetch chen lên trước ô đang hiện").toBe("đang nhìn");
    holders[1].done();
    expect(log[9]).toBe("đoán trước");
  });

  it("huỷ khi còn chờ thì không bao giờ được cấp", () => {
    const log: string[] = [];
    const holders = Array.from({ length: 8 }, (_, i) => tracked(log, `h${i}`));
    const waiting = tracked(log, "sẽ bị huỷ");
    waiting.cancel();
    holders[0].done();
    holders[1].done();
    expect(log).not.toContain("sẽ bị huỷ");
  });

  it("huỷ hai lần chỉ trả chỗ một lần — không mở khoá cho ảnh thứ 9", () => {
    const log: string[] = [];
    const first = tracked(log, "h0");
    Array.from({ length: 7 }, (_, i) => tracked(log, `h${i + 1}`));
    tracked(log, "chờ 1");
    tracked(log, "chờ 2");
    expect(log.length).toBe(8);
    first.cancel();
    first.cancel(); // lần hai phải là vô hại
    expect(log.length, "double-release cấp thừa một chỗ").toBe(9);
  });

  it("tải trước chiếm tối đa 4 chỗ — ô đang hiện đến sau vẫn được cấp ngay", () => {
    const log: string[] = [];
    for (let i = 0; i < 10; i++) tracked(log, `p${i}`, true);
    expect(log.length, "prefetch tràn quá nửa cửa xoay").toBe(4);
    tracked(log, "đang nhìn");
    expect(log).toContain("đang nhìn"); // không phải chờ prefetch nhả chỗ
  });

  it("prefetchThumb không tải cùng một URL hai lần", () => {
    const made: string[] = [];
    const RealImage = globalThis.Image;
    (globalThis as { Image: unknown }).Image = class {
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      set src(v: string) {
        made.push(v);
      }
    };
    try {
      prefetchThumb("thumb://1_5?s=64");
      prefetchThumb("thumb://1_5?s=64");
      prefetchThumb("thumb://1_6?s=64");
      expect(made).toEqual(["thumb://1_5?s=64", "thumb://1_6?s=64"]);
    } finally {
      (globalThis as { Image: unknown }).Image = RealImage;
    }
  });
});

// ================================================================ tích hợp

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

const N = 30;
const HITS = Array.from({ length: N }, (_, i) => mkHit(i + 1, `t${i}.mp4`));

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
      hits: HITS,
      epoch: 9,
      relaxed: null,
      elapsedMs: 1,
      total: N,
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

async function search(div: Element) {
  const input = div.querySelector("input.search") as HTMLInputElement;
  input.value = "t";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await settle(320);
}

const firstThumb = (div: Element) => div.querySelector(".thumb") as HTMLImageElement;

/// jsdom không tự bắn sự kiện load, nên các ô được cấp chỗ sẽ giữ chỗ mãi và
/// cửa xoay không quay. Mô phỏng ảnh về đích theo từng đợt — mỗi đợt trả chỗ
/// lại cấp cho đợt kế — cho tới khi mọi ô đang hiện đều xong, như trình duyệt
/// thật làm trong vài chục mili giây.
async function drainVisible(div: Element) {
  for (let round = 0; round < 8; round++) {
    const loading = [...div.querySelectorAll("img.thumb[src]:not(.ready)")];
    if (!loading.length) return;
    for (const img of loading) img.dispatchEvent(new Event("load"));
    await settle(20);
  }
}

describe("MediaRow — thử lại lỗi tạm, buông lỗi thật", () => {
  beforeEach(() => {
    localStorage.clear();
    resetThumbQueueForTest();
    baseHandlers();
  });

  it("ảnh về thì hiện dần (class ready)", async () => {
    const { div } = await mountApp();
    await search(div);
    const img = firstThumb(div);
    expect(img.getAttribute("src"), "ô đầu tiên phải được cấp chỗ ngay").toContain("9_1");
    expect(img.classList.contains("ready")).toBe(false);
    img.dispatchEvent(new Event("load"));
    await settle(20);
    expect(img.classList.contains("ready")).toBe(true);
  });

  it("lỗi thì rút src, chờ một nhịp rồi hỏi lại với cache-buster", async () => {
    const { div } = await mountApp();
    await search(div);
    await drainVisible(div); // chỗ trống sẵn — lượt thử lại được cấp ngay
    const img = firstThumb(div);
    img.dispatchEvent(new Event("error"));
    await settle(20);
    expect(img.getAttribute("src"), "src phải được rút trong lúc chờ").toBeNull();
    await settle(400); // lượt thử 1 tới sau ~300ms
    expect(img.getAttribute("src")).toContain("&r=1");
  });

  it("hết ba lượt thử thì buông: gỡ ảnh, nhãn màu đứng thay, KHÔNG thử nữa", async () => {
    const { div } = await mountApp();
    await search(div);
    const row = div.querySelector(".row")!;
    await drainVisible(div);
    const img = firstThumb(div);
    img.dispatchEvent(new Event("error"));
    await settle(400);
    expect(img.getAttribute("src")).toContain("&r=1");
    img.dispatchEvent(new Event("error"));
    await settle(1150); // lượt 2 sau ~1000ms
    expect(img.getAttribute("src")).toContain("&r=2");
    img.dispatchEvent(new Event("error"));
    await settle(3150); // lượt 3 sau ~3000ms
    expect(img.getAttribute("src")).toContain("&r=3");
    img.dispatchEvent(new Event("error"));
    await settle(3600); // đủ lâu cho một lượt 4 KHÔNG-được-phép xảy ra
    expect(row.querySelector(".thumb"), "ảnh phải bị gỡ hẳn sau lượt cuối").toBeNull();
    expect(row.querySelector(".kind"), "nhãn màu phải còn đứng thay").toBeTruthy();
  });

  it("ô hỏng trả chỗ cho ô khác — lỗi không giam slot", async () => {
    const { div } = await mountApp();
    await search(div);
    const imgs = [...div.querySelectorAll("img.thumb")] as HTMLImageElement[];
    const withSrc = imgs.filter((i) => i.getAttribute("src"));
    const without = imgs.filter((i) => !i.getAttribute("src"));
    expect(withSrc.length).toBe(8); // đúng bằng số chỗ của cửa xoay
    expect(without.length).toBeGreaterThan(0);
    withSrc[0].dispatchEvent(new Event("error")); // trả chỗ ngay khi lỗi
    await settle(30);
    const nowWithSrc = imgs.filter((i) => i.getAttribute("src"));
    expect(nowWithSrc.length, "chỗ vừa trả phải được cấp cho ô đang chờ").toBe(8);
  });
});

describe("MediaRow — nhịp đứng yên trước khi hỏi ảnh", () => {
  beforeEach(() => {
    localStorage.clear();
    resetThumbQueueForTest();
    baseHandlers();
  });

  it("dòng phải đứng yên ~120ms rồi mới xin ảnh", async () => {
    const { div } = await mountApp();
    const input = div.querySelector("input.search") as HTMLInputElement;
    input.value = "t";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await settle(80); // kết quả đã về, dòng đã vẽ, nhưng CHƯA tới nhịp
    const img = firstThumb(div);
    expect(img, "dòng phải vẽ ngay không đợi ảnh").toBeTruthy();
    expect(img.getAttribute("src"), "hỏi ảnh trước khi đứng yên đủ lâu").toBeNull();
    await settle(150); // qua nhịp 120ms
    expect(img.getAttribute("src")).toContain("9_1");
  });

  it("dòng bị cuộn lướt qua chết trước nhịp thì KHÔNG bắn ra yêu cầu nào", async () => {
    // Tái hiện đúng lỗi ngoài đời: kéo nhanh xuống rồi kéo ngược lên — các
    // dòng sống dưới 120ms từng bơm đầy hàng đợi backend bằng job rác, và
    // các dòng đứng yên sau đó bị 503 tới hết lượt thử lại.
    const { div, cleanup } = await mountApp();
    const input = div.querySelector("input.search") as HTMLInputElement;
    input.value = "t";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await settle(60); // dòng đã mount…
    cleanup(); // …và biến mất trước nhịp 120ms, như bị cuộn lướt qua
    await settle(200);
    // Không dòng nào kịp xin chỗ — 8 chỗ của cửa xoay phải còn nguyên vẹn.
    let grantedNow = 0;
    for (let i = 0; i < 8; i++) acquireThumbSlot(() => grantedNow++);
    expect(grantedNow, "cửa xoay bị dòng đã chết chiếm chỗ").toBe(8);
  });
});

describe("prefetch — tải trước theo hướng cuộn", () => {
  let made: string[];
  let RealImage: typeof Image;

  beforeEach(() => {
    localStorage.clear();
    resetThumbQueueForTest();
    baseHandlers();
    made = [];
    RealImage = globalThis.Image;
    (globalThis as { Image: unknown }).Image = class {
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      set src(v: string) {
        made.push(v);
        // ảnh "về" ngay để chỗ được trả và cả loạt prefetch chạy hết
        queueMicrotask(() => this.onload?.());
      }
    };
  });

  afterEach(() => {
    (globalThis as { Image: unknown }).Image = RealImage;
  });

  it("viewport đứng yên thì tải trước các dòng KẾ TIẾP, không phải dòng đang hiện", async () => {
    const { div } = await mountApp();
    await search(div);
    // Trả chỗ cho toàn bộ ô đang hiện — hàng ưu tiên thấp chỉ được cấp khi
    // không còn ô đang hiện nào chờ, và đó chính là điều đang kiểm.
    await drainVisible(div);
    await settle(250); // qua nhịp debounce 150ms + microtask của Image giả
    expect(made.length, "không có gì được tải trước").toBeGreaterThan(0);
    // Viewport 600px / dòng 46px + dự phòng 4 → 18 dòng đầu (hit.index 1..18)
    // đang hiện; mọi URL tải trước phải trỏ ra NGOÀI dải đó.
    const lastVisibleIndex = Math.ceil(600 / 46) + 4;
    for (const url of made) {
      const m = url.match(/9_(\d+)\?/);
      expect(m, `URL lạ: ${url}`).toBeTruthy();
      expect(Number(m![1]), `tải trước một dòng đang hiện: ${url}`).toBeGreaterThan(
        lastVisibleIndex,
      );
    }
  });

  it("cuộn chồng lấn dải cũ: phần giao KHÔNG bị tải lại", async () => {
    const { div } = await mountApp();
    await search(div);
    await drainVisible(div);
    await settle(250);
    const before = made.length;
    expect(before).toBeGreaterThan(0);
    // Cuộn xuống một dòng: dải mới giao gần trọn dải cũ.
    const vp = div.querySelector(".viewport") as HTMLElement;
    vp.scrollTop = 46;
    vp.dispatchEvent(new Event("scroll"));
    await drainVisible(div);
    await settle(250);
    expect(new Set(made).size, "có URL bị tải trước hai lần").toBe(made.length);
    expect(made.length, "dải mới không tải thêm phần chưa có").toBeGreaterThan(before);
  });
});
