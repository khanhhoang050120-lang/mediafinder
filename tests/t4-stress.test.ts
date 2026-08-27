// Nhóm 4 — kịch bản khắc nghiệt và hồi quy nhắm vào đúng những chỗ đã sửa.
//
// Ba câu hỏi: (1) chuyển qua lại giữa các chế độ có rò rỉ gì không, (2) các
// bất biến mà tôi viện dẫn để GIỮ state ở App có thật sự được giữ không, và
// (3) lỗi backend có được nói ra tử tế không.
// @ts-nocheck — kịch bản chuyển nguyên trạng từ harness cũ; các hàm
// trợ giúp cục bộ chưa gán kiểu. Bài mới viết thì phải có kiểu đầy đủ.
import { it, expect } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle, makeCollector } from "./helpers";
import App from "../src/App.svelte";

const ipc = new IpcRecorder();
globalThis.__ipc = ipc;

const { check, finish } = makeCollector();

it("Nhóm 4 — khắc nghiệt & hồi quy", async () => {

const mk = (i, name, epoch) => ({
  index: i, name, dir: "D:\\m", path: `D:\\m\\${name}`, kind: "video",
  matched: 1, size: 4096, width: 1920, height: 1080, durationMs: 1000,
});

let epochNow = 5;
let hitsNow = [mk(1, "a.mp4"), mk(2, "b.mp4"), mk(3, "c.mp4")];
let searchCalls = 0;

function baseHandlers(r) {
  // Prefs gio duoc luu qua cac lan mount; moi TC phai mo phien sach.
  localStorage.clear();
  searchCalls = 0;
  r.on("index_status", { loaded: true, fileCount: 100, dirCount: 5, builtAtUnix: 1700000000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", { available: null, current: "1.0.1" })
    .on("search", (a) => {
      searchCalls++;
      return { id: a.id, hits: hitsNow, epoch: epochNow, relaxed: null, elapsedMs: 1, total: hitsNow.length };
    })
    .on("dupe_progress", { running: false, completed: false, groups: 0, wasted: 0, hashed: 0, candidates: 0 })
    .on("dupe_groups", [])
    .on("find_duplicates", null)
    .on("cancel_duplicates", null)
    .on("cancel_scan", null)
    .on("request_scan", null)
    .on("request_scan_with_network", null)
    .on("open_file", null)
    .on("reveal_in_explorer", null)
    .on("start_file_drag", null)
    .on("reload_index", { loaded: true, fileCount: 200, dirCount: 9, builtAtUnix: 1700000009, problem: null });
  return r;
}

const $ = (r, s) => r.querySelector(s);
const $$ = (r, s) => [...r.querySelectorAll(s)];
const chip = (r, t) => $$(r, "button").find((b) => b.textContent.trim().startsWith(t));
const click = (el) => el?.dispatchEvent(new window.MouseEvent("click", { bubbles: true, cancelable: true }));

async function mountApp() {
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  await settle(90);
  return { div, cleanup: () => { unmount(app); div.remove(); } };
}
async function search(div, q = "abc") {
  const input = $(div, "input.search");
  input.value = q;
  input.dispatchEvent(new window.Event("input", { bubbles: true }));
  await settle(320);
}

// ---------------------------------------------------------------- TC-4.1
// Vào/ra chế độ trùng lặp nhiều lần không được rò rỉ nhịp hẹn giờ hay bỏ sót
// lệnh huỷ. Mỗi lần vào phải có đúng một lần huỷ khi ra.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  for (let i = 0; i < 5; i++) {
    click(chip(div, "Trùng lặp"));
    await settle(80);
    click(chip(div, "Trùng lặp"));
    await settle(80);
  }
  check(
    "TC-4.1a năm lần vào thì có năm lần quét",
    ipc.count("find_duplicates") === 5,
    `find_duplicates = ${ipc.count("find_duplicates")}`,
  );
  check(
    "TC-4.1b năm lần ra thì có ít nhất năm lần huỷ",
    ipc.count("cancel_duplicates") >= 5,
    `cancel_duplicates = ${ipc.count("cancel_duplicates")}`,
  );
  const before = ipc.count("dupe_progress");
  await settle(900);
  check(
    "TC-4.1c ra hẳn rồi thì không còn nhịp hẹn giờ nào chạy",
    ipc.count("dupe_progress") === before,
    `sau khi ra còn ${ipc.count("dupe_progress") - before} lần hỏi tiến độ — nhịp bị rò rỉ`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-4.2
// Gỡ App khỏi cây thì mọi nhịp hẹn giờ phải chết: nhịp đọc thuộc tính (3s),
// nhịp quét, nhịp trùng lặp.
{
  baseHandlers(ipc);
  ipc.reset();
  ipc.on("enrich_status", { running: true, done: 5, total: 100 });
  const { div, cleanup } = await mountApp();
  await settle(200);
  cleanup();
  const after = ipc.count("enrich_status");
  await settle(3500); // qua một chu kỳ 3s
  check(
    "TC-4.2 gỡ App thì nhịp đọc thuộc tính dừng",
    ipc.count("enrich_status") === after,
    `sau khi gỡ còn hỏi thêm ${ipc.count("enrich_status") - after} lần — nhịp sống lâu hơn giao diện`,
  );
  baseHandlers(ipc);
}

// ---------------------------------------------------------------- TC-4.3
// BẤT BIẾN CHÍNH: epoch và chỉ số tệp phải luôn đến từ cùng một lần tìm.
// Đây là lý do tôi giữ state tìm kiếm ở App thay vì đẩy ra module dùng chung.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountApp();
  epochNow = 5;
  hitsNow = [mk(11, "x.mp4"), mk(12, "y.mp4")];
  await search(div, "x");
  const src1 = $(div, ".thumb")?.getAttribute("src") ?? "";
  check("TC-4.3a lần tìm đầu: epoch 5 đi với chỉ số 11", src1.includes("5_11"), `src=${src1}`);

  // Chỉ mục bị thay dưới chân: epoch mới, chỉ số mới.
  epochNow = 6;
  hitsNow = [mk(77, "z.mp4")];
  await ipc.emit("index-reloaded", {});
  await settle(350);
  const src2 = $(div, ".thumb")?.getAttribute("src") ?? "";
  check(
    "TC-4.3b sau khi chỉ mục đổi: epoch 6 đi với chỉ số 77",
    src2.includes("6_77"),
    `src=${src2} — lệch nghĩa là ảnh của tệp này gắn lên tên tệp khác`,
  );
  check(
    "TC-4.3c không còn sót epoch cũ trong URL nào",
    $$(div, ".thumb").every((t) => !(t.getAttribute("src") ?? "").includes("5_")),
    $$(div, ".thumb").map((t) => t.getAttribute("src")).join(" "),
  );
  cleanup();
  epochNow = 5;
  hitsNow = [mk(1, "a.mp4"), mk(2, "b.mp4"), mk(3, "c.mp4")];
}

// ---------------------------------------------------------------- TC-4.4
// Chỉ mục đổi trong lúc ô tìm kiếm rỗng thì phải xoá danh sách, không giữ
// kết quả cũ trỏ vào chỉ số đã lỗi thời.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountApp();
  await search(div, "abc");
  check("TC-4.4a đang có kết quả", $$(div, ".row").length === 3);
  // Xoá ô tìm kiếm bằng Escape rồi mới nạp lại chỉ mục.
  window.dispatchEvent(new window.KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await settle(80);
  await ipc.emit("index-reloaded", {});
  await settle(250);
  check(
    "TC-4.4b ô rỗng + chỉ mục đổi thì danh sách trống",
    $$(div, ".row").length === 0,
    `còn ${$$(div, ".row").length} dòng trỏ vào chỉ số đã lỗi thời`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-4.5
// Dòng chọn phải trở về đầu sau mỗi lần tìm mới — nếu không, dòng chọn có thể
// trỏ ra ngoài danh sách ngắn hơn.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountApp();
  hitsNow = Array.from({ length: 10 }, (_, i) => mk(i + 1, `f${i}.mp4`));
  await search(div, "nhieu");
  for (let i = 0; i < 8; i++)
    window.dispatchEvent(new window.KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
  await settle(80);
  const deep = $$(div, ".row").findIndex((r) => r.classList.contains("focused"));
  check("TC-4.5a đã đi xuống sâu trong danh sách", deep >= 5, `đang ở dòng ${deep}`);

  // Lần tìm mới trả về ít kết quả hơn hẳn.
  hitsNow = [mk(1, "chi-mot.mp4")];
  await search(div, "it");
  const rowsNow = $$(div, ".row");
  const focusedIdx = rowsNow.findIndex((r) => r.classList.contains("focused"));
  check(
    "TC-4.5b tìm lại thì dòng chọn về đầu danh sách",
    rowsNow.length === 1 && focusedIdx === 0,
    `${rowsNow.length} dòng, con trỏ ở ${focusedIdx} — trỏ ra ngoài danh sách là hỏng`,
  );
  cleanup();
  hitsNow = [mk(1, "a.mp4"), mk(2, "b.mp4"), mk(3, "c.mp4")];
}

// ---------------------------------------------------------------- TC-4.6
// Lỗi từ backend phải hiện ra và đóng được.
{
  baseHandlers(ipc);
  ipc.on("search", () => { throw new Error("Chỉ mục hỏng"); });
  const { div, cleanup } = await mountApp();
  await search(div, "loi");
  const err = $(div, ".error");
  check("TC-4.6a lỗi tìm kiếm thì hiện băng lỗi", !!err, "không thấy .error");
  check("TC-4.6b băng lỗi nêu nội dung lỗi", (err?.textContent ?? "").includes("Chỉ mục hỏng"), `"${err?.textContent}"`);
  click($(div, ".error .dismiss"));
  await settle(60);
  check("TC-4.6c bấm Đóng thì băng lỗi biến mất", !$(div, ".error"));
  baseHandlers(ipc);
  cleanup();
}

// ---------------------------------------------------------------- TC-4.7
// Escape ưu tiên đóng lỗi trước, chỉ xoá ô tìm kiếm ở lần bấm sau.
{
  baseHandlers(ipc);
  ipc.on("search", () => { throw new Error("Sự cố tạm thời"); });
  const { div, cleanup } = await mountApp();
  await search(div, "loi");
  check("TC-4.7a đang có băng lỗi", !!$(div, ".error"));
  window.dispatchEvent(new window.KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await settle(60);
  check("TC-4.7b Escape lần đầu đóng lỗi", !$(div, ".error"));
  check(
    "TC-4.7c Escape lần đầu KHÔNG xoá ô tìm kiếm",
    $(div, "input.search").value === "loi",
    `value="${$(div, "input.search").value}" — mất chữ đang gõ là phiền`,
  );
  window.dispatchEvent(new window.KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await settle(60);
  check("TC-4.7d Escape lần hai xoá ô tìm kiếm", $(div, "input.search").value === "");
  baseHandlers(ipc);
  cleanup();
}

// ---------------------------------------------------------------- TC-4.8
// Lỗi cũ phải được xoá khi bắt đầu một lần quét mới.
{
  baseHandlers(ipc);
  ipc.on("search", () => { throw new Error("Lỗi cũ"); });
  const { div, cleanup } = await mountApp();
  await search(div, "loi");
  check("TC-4.8a có lỗi cũ trên màn hình", !!$(div, ".error"));
  baseHandlers(ipc);
  click(chip(div, "Quét lại"));
  await settle(200);
  check(
    "TC-4.8b bắt đầu quét thì lỗi cũ được dọn đi",
    !$(div, ".error"),
    "lỗi cũ còn nằm đó trong khi việc mới đang chạy",
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-4.9
// Bộ lọc phải sống sót qua việc chuyển sang chế độ trùng lặp rồi quay lại.
// (FilterPanel bị gỡ khỏi cây khi dupeMode bật? Nếu có, lựa chọn sẽ mất.)
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountApp();
  await search(div, "abc");
  click(chip(div, "Lọc"));
  await settle(60);
  click(chip(div, "4K"));
  await settle(200);
  check("TC-4.9a đã bật bộ lọc 4K", chip(div, "4K")?.classList.contains("on"), "chip 4K không sáng");
  click(chip(div, "Trùng lặp"));
  await settle(150);
  click(chip(div, "Trùng lặp"));
  await settle(150);
  const chip4k = chip(div, "4K");
  check(
    "TC-4.9b quay lại từ chế độ trùng lặp thì bộ lọc 4K vẫn còn",
    !!chip4k && chip4k.classList.contains("on"),
    "bộ lọc bị mất — người dùng tưởng đang lọc mà thực ra không",
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-4.10
// Hồi quy cho đúng lỗi đã sửa: giao diện và kết quả không được lệch nhau.
// Bấm lần lượt nhiều bộ lọc rồi kiểm tra lần tìm CUỐI dùng đúng thứ đang sáng.
{
  baseHandlers(ipc);
  let lastFilters = null;
  ipc.on("search", (a) => {
    lastFilters = JSON.parse(JSON.stringify(a.req.filters));
    return { id: a.id, hits: hitsNow, epoch: epochNow, relaxed: null, elapsedMs: 1, total: hitsNow.length };
  });
  const { div, cleanup } = await mountApp();
  await search(div, "abc");
  click(chip(div, "Lọc"));
  await settle(60);

  click(chip(div, "≥720p"));
  await settle(250);
  check(
    "TC-4.10a chọn ≥720p thì tìm ngay với minHeight=720",
    lastFilters?.minHeight === 720,
    `filters=${JSON.stringify(lastFilters)}`,
  );

  click(chip(div, "1–10 phút"));
  await settle(250);
  check(
    "TC-4.10b thêm thời lượng thì cả hai điều kiện cùng có hiệu lực",
    lastFilters?.minHeight === 720 && lastFilters?.minDurationMs === 60000,
    `filters=${JSON.stringify(lastFilters)}`,
  );

  click(chip(div, "30 ngày"));
  await settle(250);
  check(
    "TC-4.10c thêm mốc thời gian thì cả ba cùng có hiệu lực",
    lastFilters?.minHeight === 720 && lastFilters?.minDurationMs === 60000 && lastFilters?.withinDays === 30,
    `filters=${JSON.stringify(lastFilters)}`,
  );

  click(chip(div, "Bỏ lọc"));
  await settle(250);
  check(
    "TC-4.10d Bỏ lọc thì lần tìm ngay sau đó KHÔNG còn bộ lọc nào",
    lastFilters &&
      lastFilters.minHeight === 0 &&
      lastFilters.minDurationMs === 0 &&
      lastFilters.maxDurationMs === 0 &&
      lastFilters.withinDays === 0,
    `filters=${JSON.stringify(lastFilters)} — đây chính là lỗi đã sửa`,
  );
  // Và giao diện phải đồng ý với kết quả.
  check(
    "TC-4.10e sau Bỏ lọc thì không chip nào còn sáng",
    !$$(div, ".filters .chip.on").length,
    `còn ${$$(div, ".filters .chip.on").length} chip sáng trong khi kết quả không lọc`,
  );
  cleanup();
  baseHandlers(ipc);
}

// ---------------------------------------------------------------- TC-4.11
// Gõ liên tiếp nhiều phím chỉ nên sinh một lần tìm (coalesce).
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  const input = $(div, "input.search");
  for (const s of ["p", "ph", "phi", "phim"]) {
    input.value = s;
    input.dispatchEvent(new window.Event("input", { bubbles: true }));
  }
  await settle(350);
  check(
    "TC-4.11 gõ bốn phím liền chỉ sinh một lần tìm",
    searchCalls === 1,
    `search gọi ${searchCalls} lần — mỗi phím một lần là quá nhiều`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-4.12
// Chế độ lưới giữ nguyên khi chuyển qua trùng lặp rồi quay lại, và danh sách
// trùng lặp KHÔNG được vẽ theo lưới (nó là danh sách nhóm).
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountApp();
  await search(div, "abc");
  const gridBtn = $$(div, "button").find((b) => (b.getAttribute("aria-label") ?? "").includes("lưới"));
  click(gridBtn);
  await settle(80);
  check("TC-4.12a đã bật lưới", $(div, ".results")?.classList.contains("grid"));
  click(chip(div, "Trùng lặp"));
  await settle(150);
  const dupeResults = $(div, ".results");
  check(
    "TC-4.12b danh sách trùng lặp không vẽ theo lưới",
    dupeResults && !dupeResults.classList.contains("grid"),
    `class="${dupeResults?.className}" — nhóm trùng lặp mà xếp lưới thì không đọc được`,
  );
  click(chip(div, "Trùng lặp"));
  await settle(200);
  check(
    "TC-4.12c quay lại thì vẫn ở chế độ lưới như trước",
    $(div, ".results")?.classList.contains("grid"),
    `class="${$(div, ".results")?.className}"`,
  );
  cleanup();
}

finish();
});
