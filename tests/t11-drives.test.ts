// Nhóm 11 — lọc theo ổ đĩa: hàng chip (lớp 1) và nhãn ổ trên mỗi dòng (lớp 2).
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import {
  bucketsFor,
  driveKey,
  driveLabel,
  filterByDrive,
  isNetworkDrive,
  networkLetters,
} from "../src/lib/drives";
import App from "../src/App.svelte";
import type { SearchHit } from "../src/lib/search";

function mkHit(i: number, name: string, dir: string): SearchHit {
  return {
    index: i,
    name,
    dir,
    path: `${dir}\\${name}`,
    kind: "video",
    matched: 1,
    size: 4096,
    width: 1920,
    height: 1080,
    durationMs: 60_000,
  } as SearchHit;
}

// Hai ổ trong máy + một ổ mạng — đúng tình huống người dùng mô tả.
const HITS = [
  mkHit(1, "a1.mp4", "D:\\1363\\video"),
  mkHit(2, "a2.mp4", "D:\\1363\\video"),
  mkHit(3, "a3.mp4", "D:\\DS3_006\\Gen"),
  mkHit(4, "b1.mp4", "C:\\Users\\Padoma1\\Videos"),
  mkHit(5, "n1.mov", "\\\\NAS\\media\\2025"),
  mkHit(6, "n2.mov", "\\\\NAS\\media\\2025"),
];

// ================================================================ đơn vị

describe("drives — nhận diện ổ từ đường dẫn", () => {
  it("ổ có chữ cái: lấy chữ cái, viết hoa", () => {
    expect(driveKey("D:\\a\\b.mp4")).toBe("D");
    expect(driveKey("c:\\a.mp4")).toBe("C");
    expect(driveLabel("D")).toBe("D:");
    expect(isNetworkDrive("D")).toBe(false);
  });

  it("đường dẫn UNC: lấy tên máy chủ, giữ hai gạch để không lẫn với ổ tên NAS", () => {
    expect(driveKey("\\\\NAS\\media\\a.mp4")).toBe("\\\\NAS");
    expect(driveLabel("\\\\NAS")).toBe("NAS");
    expect(isNetworkDrive("\\\\NAS")).toBe(true);
    // Một ổ có chữ cái mà tên là NAS phải KHÁC máy chủ tên NAS.
    expect(driveKey("N:\\as\\x.mp4")).not.toBe(driveKey("\\\\NAS\\x.mp4"));
  });

  it("Ổ MẠNG ÁNH XẠ (Y:) phải được nhận ra là ổ mạng, không phải đĩa trong máy", () => {
    // Bất biến này từng bị bỏ sót và nó làm CẢ NHÁNH ổ-mạng thành mã chết trên
    // mọi máy của studio: cả bốn ổ NAS ở đó đều là ổ ánh xạ (F:, H:, Y:, Z: —
    // đo bằng `net use`), và chỉ mục lưu chúng dưới dạng `Y:\PROJECT…` chứ
    // không phải UNC. Chỉ nhận dạng `\\` nghĩa là: không chip cam, không nhãn
    // cam, ổ mạng không bị đẩy xuống cuối hàng chip. Tính năng lặng lẽ giải
    // đúng một nửa vấn đề nó sinh ra để giải, và không ai báo lỗi vì phần lọc
    // và đếm vẫn chạy đúng.
    const net = networkLetters([{ letter: "Y" }, { letter: "Z:" }]);
    expect(isNetworkDrive("Y", net)).toBe(true);
    expect(isNetworkDrive("Z", net)).toBe(true);
    expect(isNetworkDrive("D", net)).toBe(false);
    // Vẫn phải nhận UNC kể cả khi không ai đưa danh sách ổ.
    expect(isNetworkDrive("\\\\NAS", net)).toBe(true);
    expect(isNetworkDrive("\\\\NAS")).toBe(true);
  });

  it("chữ cái ổ mạng nhận vào kiểu nào cũng khớp, và so không phân biệt hoa thường", () => {
    // Backend trả `letter` — đã từng thấy cả "Y" lẫn "Y:". Đừng để một dấu hai
    // chấm quyết định một tính năng có chạy hay không.
    const net = networkLetters([{ letter: "y:" }]);
    expect(isNetworkDrive("Y", net)).toBe(true);
  });

  it("ổ mạng ánh xạ bị đẩy xuống cuối hàng chip, đúng như ổ UNC", () => {
    const hits = [
      mkHit(1, "a.mp4", "D:\\du_an"),
      mkHit(2, "b.mp4", "Y:\\PROJECT DEEP SEA 5"),
      mkHit(3, "c.mp4", "C:\\Users\\x\\Videos"),
    ];
    const b = bucketsFor(hits, networkLetters([{ letter: "Y" }]));
    expect(b.map((x) => x.id)).toEqual(["C", "D", "Y"]);
    expect(b.map((x) => x.network)).toEqual([false, false, true]);
  });

  it("đường dẫn lạ thì trả rỗng — không dựng chip vô nghĩa", () => {
    expect(driveKey("")).toBe("");
    expect(driveKey("khong-phai-duong-dan")).toBe("");
    expect(driveKey("\\\\")).toBe("");
  });

  it("gom nhóm: đếm đúng, ổ trong máy trước, ổ mạng xuống cuối", () => {
    const b = bucketsFor(HITS);
    expect(b.map((x) => x.label)).toEqual(["C:", "D:", "NAS"]);
    expect(b.map((x) => x.count)).toEqual([1, 3, 2]);
    expect(b.map((x) => x.network)).toEqual([false, false, true]);
  });

  it("ổ mạng xuống cuối NGAY CẢ khi tên nó đứng trước theo chữ cái", () => {
    // Dữ liệu ép quy tắc lộ ra: xếp thuần chữ cái sẽ cho "ALPHA" trước "Z:",
    // nên nếu ổ mạng không bị đẩy xuống thì thứ tự này sai ngay.
    const mixed = [
      mkHit(1, "x.mp4", String.raw`\\ALPHA\share`),
      mkHit(2, "y.mp4", String.raw`Z:\video`),
    ];
    const b = bucketsFor(mixed);
    expect(
      b.map((x) => x.label),
      "ổ mạng phải xuống cuối bất kể tên — nó ít khi là nơi đang làm việc",
    ).toEqual(["Z:", "ALPHA"]);
  });

  it("lọc: null là tất cả, khoá ổ là đúng ổ đó", () => {
    expect(filterByDrive(HITS, null).length).toBe(6);
    expect(filterByDrive(HITS, "D").map((h) => h.name)).toEqual(["a1.mp4", "a2.mp4", "a3.mp4"]);
    expect(filterByDrive(HITS, "\\\\NAS").map((h) => h.name)).toEqual(["n1.mov", "n2.mov"]);
  });
});

// ================================================================ tích hợp

let ipc: IpcRecorder;
let served: SearchHit[] = HITS;
let opened: string[] = [];

function baseHandlers(): IpcRecorder {
  const r = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = r;
  opened = [];
  r.on("index_status", { loaded: true, fileCount: 100, dirCount: 5, builtAtUnix: 1_700_000_000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", { checked: true, available: null, current: "1.0.5" })
    .on("search", (a: { id: number }) => ({
      id: a.id,
      hits: served,
      epoch: 3,
      relaxed: null,
      elapsedMs: 1,
      total: served.length,
    }))
    .on("miss_log_status", { enabled: false, count: 0 })
    .on("open_file", (a: { path: string }) => {
      opened.push(a.path);
      return null;
    })
    .on("start_file_drag", null)
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

async function search(div: Element, q = "a") {
  const input = div.querySelector("input.search") as HTMLInputElement;
  input.value = q;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await settle(320);
}

const $$ = (root: Element, sel: string) => [...root.querySelectorAll(sel)];
const dchips = (root: Element) => $$(root, ".dchip");
const chipText = (root: Element) => dchips(root).map((c) => c.textContent!.replace(/\s+/g, " ").trim());
const rowNames = (root: Element) => $$(root, ".row .name").map((n) => n.textContent!.trim());
const click = (el: Element | undefined) => {
  expect(el, "phần tử cần bấm không tồn tại").toBeTruthy();
  el!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
};
const chipBy = (root: Element, label: string) =>
  dchips(root).find((c) => c.textContent!.trim().startsWith(label));

beforeEach(() => {
  localStorage.clear();
  served = HITS;
  ipc = baseHandlers();
});

describe("hàng chip ổ đĩa", () => {
  it("nhiều ổ: hiện chip kèm số đếm, mặc định 'Tất cả' đang chọn", async () => {
    const { div } = await mountApp();
    await search(div);
    expect(chipText(div)).toEqual(["Tất cả 6", "C: 1", "D: 3", "NAS 2"]);
    expect(dchips(div)[0].classList.contains("on"), "'Tất cả' phải sáng lúc đầu").toBe(true);
  });

  it("một ổ duy nhất: KHÔNG hiện hàng chip — không chiếm chỗ khi chẳng nói thêm gì", async () => {
    served = HITS.filter((h) => h.path.startsWith("D:"));
    const { div } = await mountApp();
    await search(div);
    expect(div.querySelector(".drives"), "một ổ mà vẫn dựng hàng chip").toBeNull();
  });

  it("bấm chip D: chỉ còn kết quả ổ D", async () => {
    const { div } = await mountApp();
    await search(div);
    expect(rowNames(div).length).toBe(6);
    click(chipBy(div, "D:"));
    await settle(60);
    expect(rowNames(div)).toEqual(["a1.mp4", "a2.mp4", "a3.mp4"]);
    expect(chipBy(div, "D:")!.classList.contains("on")).toBe(true);
  });

  it("bấm lại chip đang chọn thì bỏ lọc, về Tất cả", async () => {
    const { div } = await mountApp();
    await search(div);
    click(chipBy(div, "NAS"));
    await settle(60);
    expect(rowNames(div).length).toBe(2);
    click(chipBy(div, "NAS"));
    await settle(60);
    expect(rowNames(div).length).toBe(6);
  });

  it("chip ổ mạng mang lớp riêng — màu khác ổ trong máy", async () => {
    const { div } = await mountApp();
    await search(div);
    expect(chipBy(div, "NAS")!.classList.contains("nas")).toBe(true);
    expect(chipBy(div, "D:")!.classList.contains("nas")).toBe(false);
  });

  it("thanh trạng thái đếm theo danh sách ĐANG hiện, không phải tổng thô", async () => {
    const { div } = await mountApp();
    await search(div);
    click(chipBy(div, "D:"));
    await settle(60);
    expect(div.querySelector(".timing")!.textContent).toContain("3");
  });
});

describe("bàn phím và thao tác chạy trên danh sách đã lọc", () => {
  it("Enter mở đúng tệp của ổ đang lọc, không phải tệp đầu danh sách gốc", async () => {
    const { div } = await mountApp();
    await search(div);
    click(chipBy(div, "NAS"));
    await settle(60);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    await settle(80);
    // Không lọc thì dòng đầu là a1.mp4 (ổ D) — lọc rồi phải là n1.mov.
    expect(opened).toEqual(["\\\\NAS\\media\\2025\\n1.mov"]);
  });

  it("đổi ổ thì con trỏ về đầu — không trỏ vào vị trí của danh sách cũ", async () => {
    const { div } = await mountApp();
    await search(div);
    // đi xuống dòng 3 của danh sách đầy đủ
    for (let i = 0; i < 3; i++) {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
    }
    await settle(60);
    click(chipBy(div, "C:")); // ổ này chỉ có 1 kết quả
    await settle(60);
    const rows = $$(div, ".row");
    expect(rows.length).toBe(1);
    expect(rows[0].classList.contains("focused"), "con trỏ không về đầu sau khi đổi ổ").toBe(true);
  });

  it("lần tìm mới không còn ổ đang lọc: tự về Tất cả thay vì hiện danh sách rỗng", async () => {
    const { div } = await mountApp();
    await search(div);
    click(chipBy(div, "NAS"));
    await settle(60);
    expect(rowNames(div).length).toBe(2);

    // Lần tìm sau chỉ ra kết quả ổ D — ổ NAS biến mất khỏi kết quả.
    served = HITS.filter((h) => h.path.startsWith("D:"));
    await search(div, "khac");
    expect(rowNames(div).length, "người dùng bị bỏ lại trước một danh sách rỗng").toBe(3);
  });
});

describe("nhãn ổ trên mỗi dòng", () => {
  it("nhiều ổ: mỗi dòng mang nhãn đúng ổ của nó", async () => {
    const { div } = await mountApp();
    await search(div);
    const labels = $$(div, ".row .drive").map((d) => d.textContent!.trim());
    expect(labels.slice(0, 4)).toEqual(["D:", "D:", "D:", "C:"]);
  });

  it("dòng của ổ mạng mang lớp nas — cùng ngôn ngữ màu với chip", async () => {
    const { div } = await mountApp();
    await search(div);
    const nasRows = $$(div, ".row .drive.nas").map((d) => d.textContent!.trim());
    expect(nasRows).toEqual(["NAS", "NAS"]);
  });

  it("một ổ duy nhất: KHÔNG gắn nhãn — mực in không nói gì thì đừng in", async () => {
    served = HITS.filter((h) => h.path.startsWith("D:"));
    const { div } = await mountApp();
    await search(div);
    expect($$(div, ".row .drive").length).toBe(0);
  });
});
