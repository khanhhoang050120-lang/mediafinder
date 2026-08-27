// Nhóm 3 — bàn phím, chọn nhiều dòng, lưới, và các trường hợp biên.
//
// Phần logic tinh vi nhất, và cũng là phần tôi cố ý GIỮ NGUYÊN ở App thay vì
// đẩy ra tệp riêng. Nếu lập luận đó sai thì nó sẽ vỡ ở đây.
// @ts-nocheck — kịch bản chuyển nguyên trạng từ harness cũ; các hàm
// trợ giúp cục bộ chưa gán kiểu. Bài mới viết thì phải có kiểu đầy đủ.
import { it, expect } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle, makeCollector } from "./helpers";
import App from "../src/App.svelte";

const ipc = new IpcRecorder();
globalThis.__ipc = ipc;

const { check, finish } = makeCollector();

it("Nhóm 3 — tương tác & biên", async () => {

const N = 12;
const HITS = Array.from({ length: N }, (_, i) => ({
  index: i + 1,
  name: `tep${i}.mp4`,
  dir: "D:\\media",
  path: `D:\\media\\tep${i}.mp4`,
  kind: i % 3 === 0 ? "video" : i % 3 === 1 ? "image" : "audio",
  matched: 2,
  size: 1024 * 1024,
  width: 1920,
  height: 1080,
  durationMs: 60000,
}));

let dragPaths = null;
let opened = [];
let revealed = [];
function baseHandlers(r, relaxed = null) {
  // Prefs gio duoc luu qua cac lan mount; moi TC phai mo phien sach.
  localStorage.clear();
  dragPaths = null;
  opened = [];
  revealed = [];
  r.on("index_status", { loaded: true, fileCount: 100, dirCount: 5, builtAtUnix: 1700000000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", { available: null, current: "1.0.1" })
    .on("search", (a) => ({ id: a.id, hits: HITS, epoch: 9, relaxed, elapsedMs: 1, total: N }))
    .on("dupe_progress", { running: false, completed: false, groups: 0, wasted: 0, hashed: 0, candidates: 0 })
    .on("dupe_groups", [])
    .on("find_duplicates", null)
    .on("cancel_duplicates", null)
    .on("open_file", (a) => { opened.push(a.path); return null; })
    .on("reveal_in_explorer", (a) => { revealed.push(a.path); return null; })
    .on("start_file_drag", (a) => { dragPaths = a.paths; return null; });
  return r;
}

async function mountWithHits() {
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  await settle(80);
  const input = div.querySelector("input.search");
  input.value = "tep";
  input.dispatchEvent(new window.Event("input", { bubbles: true }));
  await settle(350);
  return { div, cleanup: () => { unmount(app); div.remove(); } };
}

const $$ = (root, sel) => [...root.querySelectorAll(sel)];
const rows = (root) => $$(root, ".row");
const selRows = (root) => $$(root, ".row.sel");
const focusedRow = (root) => root.querySelector(".row.focused");

function key(k, opts = {}) {
  const e = new window.KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...opts });
  window.dispatchEvent(e);
  return e;
}
function clickRow(root, i, opts = {}) {
  const r = rows(root)[i];
  if (!r) return false;
  r.dispatchEvent(new window.MouseEvent("click", { bubbles: true, cancelable: true, ...opts }));
  return true;
}
/// Chỉ số của các dòng đang được chọn, đọc từ DOM.
function selIndexes(root) {
  const all = rows(root);
  return all.map((r, i) => (r.classList.contains("sel") ? i : -1)).filter((i) => i >= 0);
}

// ---------------------------------------------------------------- TC-3.1
// Sau khi tìm xong, dòng đầu phải được chọn sẵn.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  check("TC-3.1a có kết quả để thao tác", rows(div).length > 0, `${rows(div).length} dòng`);
  check("TC-3.1b dòng đầu được chọn sẵn", selIndexes(div).join(",") === "0", `chọn=[${selIndexes(div)}]`);
  check("TC-3.1c dòng đầu giữ con trỏ bàn phím", !!focusedRow(div) && rows(div)[0] === focusedRow(div));
  cleanup();
}

// ---------------------------------------------------------------- TC-3.2
// Mũi tên xuống/lên di chuyển đúng một dòng ở chế độ danh sách.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  key("ArrowDown");
  await settle(40);
  check("TC-3.2a ArrowDown xuống dòng 1", selIndexes(div).join(",") === "1", `chọn=[${selIndexes(div)}]`);
  key("ArrowDown");
  await settle(40);
  check("TC-3.2b ArrowDown xuống dòng 2", selIndexes(div).join(",") === "2", `chọn=[${selIndexes(div)}]`);
  key("ArrowUp");
  await settle(40);
  check("TC-3.2c ArrowUp lên dòng 1", selIndexes(div).join(",") === "1", `chọn=[${selIndexes(div)}]`);
  cleanup();
}

// ---------------------------------------------------------------- TC-3.3
// Mũi tên không được vượt ra ngoài hai đầu danh sách.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  key("ArrowUp");
  await settle(40);
  check("TC-3.3a ArrowUp ở dòng đầu thì đứng yên", selIndexes(div).join(",") === "0", `chọn=[${selIndexes(div)}]`);
  for (let i = 0; i < N + 5; i++) key("ArrowDown");
  await settle(60);
  const sel = selIndexes(div);
  check(
    "TC-3.3b ArrowDown quá cuối thì dừng ở dòng cuối",
    sel.length === 1 && sel[0] === N - 1,
    `chọn=[${sel}] (mong đợi [${N - 1}])`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.4
// Shift + mũi tên mở rộng dải tính từ chỗ neo.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  key("ArrowDown");
  key("ArrowDown"); // đang ở dòng 2, neo = 2
  await settle(40);
  key("ArrowDown", { shiftKey: true });
  key("ArrowDown", { shiftKey: true });
  await settle(60);
  check(
    "TC-3.4a Shift+Down mở rộng dải 2..4",
    selIndexes(div).join(",") === "2,3,4",
    `chọn=[${selIndexes(div)}]`,
  );
  key("ArrowUp", { shiftKey: true });
  await settle(40);
  check(
    "TC-3.4b Shift+Up thu dải lại còn 2..3",
    selIndexes(div).join(",") === "2,3",
    `chọn=[${selIndexes(div)}]`,
  );
  // Đi ngược qua chỗ neo: dải phải lật chiều, không phải cộng dồn.
  key("ArrowUp", { shiftKey: true });
  key("ArrowUp", { shiftKey: true });
  await settle(60);
  check(
    "TC-3.4c Shift+Up vượt qua chỗ neo thì dải lật chiều (1..2)",
    selIndexes(div).join(",") === "1,2",
    `chọn=[${selIndexes(div)}]`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.5
// Shift+click chọn dải từ chỗ neo tới chỗ bấm.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  clickRow(div, 1);
  await settle(40);
  clickRow(div, 5, { shiftKey: true });
  await settle(60);
  check(
    "TC-3.5a Shift+click chọn dải 1..5",
    selIndexes(div).join(",") === "1,2,3,4,5",
    `chọn=[${selIndexes(div)}]`,
  );
  // Bấm ngược lên trên chỗ neo.
  clickRow(div, 0, { shiftKey: true });
  await settle(60);
  check(
    "TC-3.5b Shift+click ngược lên thì dải là 0..1",
    selIndexes(div).join(",") === "0,1",
    `chọn=[${selIndexes(div)}]`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.6
// Ctrl+click bật/tắt từng dòng, và không cho bỏ dòng cuối cùng.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  clickRow(div, 0);
  await settle(40);
  clickRow(div, 3, { ctrlKey: true });
  await settle(40);
  check("TC-3.6a Ctrl+click thêm dòng 3", selIndexes(div).join(",") === "0,3", `chọn=[${selIndexes(div)}]`);
  clickRow(div, 3, { ctrlKey: true });
  await settle(40);
  check("TC-3.6b Ctrl+click lần nữa thì bỏ dòng 3", selIndexes(div).join(",") === "0", `chọn=[${selIndexes(div)}]`);
  clickRow(div, 0, { ctrlKey: true });
  await settle(40);
  check(
    "TC-3.6c không bỏ được dòng cuối cùng đang chọn",
    selIndexes(div).length === 1,
    `chọn=[${selIndexes(div)}] — bỏ hết thì không còn gì để kéo`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.7
// Enter mở tệp đang chọn; Ctrl+Enter mở thư mục chứa.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  key("ArrowDown"); // dòng 1
  await settle(40);
  key("Enter");
  await settle(80);
  check(
    "TC-3.7a Enter mở đúng tệp đang chọn",
    opened.length === 1 && opened[0].endsWith("tep1.mp4"),
    `opened=${JSON.stringify(opened)}`,
  );
  key("Enter", { ctrlKey: true });
  await settle(80);
  check(
    "TC-3.7b Ctrl+Enter mở thư mục chứa tệp",
    revealed.length === 1 && revealed[0].endsWith("tep1.mp4"),
    `revealed=${JSON.stringify(revealed)}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.8
// Shift+Enter mở lớp xem trước, và khi lớp phủ mở thì bàn phím thuộc về nó.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  key("ArrowDown");
  await settle(40);
  const before = selIndexes(div).join(",");
  key("Enter", { shiftKey: true });
  await settle(80);
  // Lớp xem trước vẽ ra .backdrop > .sheet, không phải một class tên "preview".
  const overlay = document.querySelector(".backdrop");
  check("TC-3.8a Shift+Enter mở lớp xem trước", !!overlay, "không thấy lớp phủ nào");
  check("TC-3.8b Shift+Enter không mở tệp bằng Windows", opened.length === 0, `opened=${JSON.stringify(opened)}`);
  // Trong lúc lớp phủ mở, mũi tên thuộc về lớp phủ: nó bước sang kết quả kế
  // tiếp và kéo cả danh sách bên dưới đi theo, để đóng lớp phủ ra là người
  // dùng đứng đúng chỗ họ đã xem tới. App phải im, nhưng dòng chọn thì VẪN
  // đổi — đúng một bước, do lớp phủ điều khiển.
  key("ArrowDown");
  await settle(40);
  check(
    "TC-3.8c lớp phủ mở thì mũi tên bước đúng một kết quả",
    selIndexes(div).join(",") === String(Number(before) + 1),
    `trước=[${before}] sau=[${selIndexes(div)}] — mong đợi [${Number(before) + 1}]`,
  );
  // Và chỉ một lần: nếu App cũng xử lý cùng phím thì sẽ nhảy hai bước.
  key("ArrowDown");
  await settle(40);
  check(
    "TC-3.8d mỗi phím chỉ bước một lần (App không xử lý trùng)",
    selIndexes(div).join(",") === String(Number(before) + 2),
    `sau=[${selIndexes(div)}] — nhảy hai bước nghĩa là cả App lẫn lớp phủ cùng bắt phím`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.9
// Escape xoá ô tìm kiếm và danh sách.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  check("TC-3.9a đang có kết quả", rows(div).length > 0);
  key("Escape");
  await settle(80);
  check("TC-3.9b Escape xoá ô tìm kiếm", div.querySelector("input.search").value === "", `value="${div.querySelector("input.search").value}"`);
  check("TC-3.9c Escape xoá danh sách kết quả", rows(div).length === 0, `còn ${rows(div).length} dòng`);
  check(
    "TC-3.9d Escape lấy con trỏ về ô tìm kiếm",
    document.activeElement === div.querySelector("input.search"),
    `activeElement=${document.activeElement?.className}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.10
// Kéo một dòng nằm TRONG tập chọn thì kéo cả tập.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  clickRow(div, 1);
  await settle(40);
  clickRow(div, 3, { ctrlKey: true });
  await settle(40);
  const r = rows(div)[3];
  r.dispatchEvent(new window.MouseEvent("dragstart", { bubbles: true, cancelable: true }));
  await settle(80);
  check(
    "TC-3.10 kéo dòng trong tập thì kéo cả tập (2 tệp)",
    Array.isArray(dragPaths) && dragPaths.length === 2,
    `dragPaths=${JSON.stringify(dragPaths)}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.11
// Kéo một dòng NGOÀI tập chọn thì chỉ kéo đúng dòng đó.
// Đây là bất biến an toàn: kéo nhầm sẽ mang đi tệp người dùng không nhìn thấy.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  clickRow(div, 0);
  await settle(40);
  clickRow(div, 1, { ctrlKey: true });
  await settle(40);
  clickRow(div, 2, { ctrlKey: true });
  await settle(40);
  const outside = rows(div)[7];
  outside.dispatchEvent(new window.MouseEvent("dragstart", { bubbles: true, cancelable: true }));
  await settle(80);
  check(
    "TC-3.11a kéo dòng ngoài tập thì chỉ kéo một tệp",
    Array.isArray(dragPaths) && dragPaths.length === 1,
    `dragPaths=${JSON.stringify(dragPaths)} — kéo nhầm tệp người dùng không thấy`,
  );
  check(
    "TC-3.11b kéo đúng tệp đó",
    dragPaths && dragPaths[0].endsWith("tep7.mp4"),
    `dragPaths=${JSON.stringify(dragPaths)}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.12
// Chế độ lưới: bấm nút lưới thì .results có class grid, và MediaRow nhận grid.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  const gridBtn = $$(div, "button").find((b) => (b.getAttribute("aria-label") ?? "").includes("lưới"));
  check("TC-3.12a có nút chuyển lưới", !!gridBtn);
  gridBtn?.dispatchEvent(new window.MouseEvent("click", { bubbles: true, cancelable: true }));
  await settle(80);
  const res = div.querySelector(".results");
  check("TC-3.12b .results có class grid", res?.classList.contains("grid"), `class="${res?.className}"`);
  const thumb = div.querySelector(".thumb");
  check(
    "TC-3.12c ảnh trong dòng nhận class grid",
    thumb?.classList.contains("grid"),
    `thumb class="${thumb?.className}" — thiếu thì ảnh giữ cỡ 40x30 của danh sách`,
  );
  const kind = div.querySelector(".kind");
  check("TC-3.12d nhãn loại nhận class grid", kind?.classList.contains("grid"), `kind class="${kind?.className}"`);
  const facts = div.querySelector(".facts");
  check("TC-3.12e cụm thông số nhận class grid", facts?.classList.contains("grid"), `facts class="${facts?.className}"`);
  cleanup();
}

// ---------------------------------------------------------------- TC-3.13
// Ảnh dùng kích thước lớn ở chế độ lưới (256) và nhỏ ở danh sách (64).
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  const listSrc = div.querySelector(".thumb")?.getAttribute("src") ?? "";
  check("TC-3.13a danh sách dùng ảnh 64px", listSrc.includes("s=64"), `src=${listSrc}`);
  const gridBtn = $$(div, "button").find((b) => (b.getAttribute("aria-label") ?? "").includes("lưới"));
  gridBtn?.dispatchEvent(new window.MouseEvent("click", { bubbles: true, cancelable: true }));
  // Đổi cỡ ảnh là đổi URL — dòng chờ đủ nhịp đứng yên 120ms rồi mới hỏi lại.
  await settle(250);
  const gridSrc = div.querySelector(".thumb")?.getAttribute("src") ?? "";
  check("TC-3.13b lưới dùng ảnh 256px", gridSrc.includes("s=256"), `src=${gridSrc}`);
  // epoch phải đi cùng với chỉ số tệp trong cùng một URL — đây chính là bất
  // biến khiến tôi giữ state tìm kiếm ở App.
  check(
    "TC-3.13c URL ảnh mang đúng epoch của lần tìm (9)",
    gridSrc.includes("9_"),
    `src=${gridSrc} — sai epoch nghĩa là ảnh của tệp khác`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.14
// Mũi tên trái/phải chỉ có tác dụng ở chế độ lưới.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  const before = selIndexes(div).join(",");
  key("ArrowRight");
  await settle(40);
  check(
    "TC-3.14a chế độ danh sách: ArrowRight không đổi dòng chọn",
    selIndexes(div).join(",") === before,
    `trước=[${before}] sau=[${selIndexes(div)}]`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.15
// Cột "khớp mấy từ" chỉ hiện khi kết quả bị nới lỏng.
{
  baseHandlers(ipc, null);
  const a = await mountWithHits();
  check("TC-3.15a không nới lỏng thì không hiện cột khớp", !a.div.querySelector(".matched"));
  check("TC-3.15b không nới lỏng thì không có băng thông báo", !a.div.querySelector(".partial"));
  a.cleanup();

  baseHandlers(ipc, { totalTokens: 3, bestMatched: 2 });
  const b = await mountWithHits();
  check("TC-3.15c có nới lỏng thì hiện băng thông báo", !!b.div.querySelector(".partial"));
  const m = b.div.querySelector(".matched");
  check("TC-3.15d có nới lỏng thì hiện cột khớp", !!m, "thiếu cột khớp");
  check(
    "TC-3.15e cột khớp ghi đúng dạng 'khớp/tổng'",
    m && m.textContent.trim() === "2/3",
    `thấy "${m?.textContent.trim()}" — mong đợi "2/3"`,
  );
  b.cleanup();
  baseHandlers(ipc, null);
}

// ---------------------------------------------------------------- TC-3.16
// Chuột phải mở trình đơn ngữ cảnh, và khi nó mở thì bàn phím thuộc về nó.
{
  baseHandlers(ipc);
  const { div, cleanup } = await mountWithHits();
  const before = selIndexes(div).join(",");
  rows(div)[2].dispatchEvent(new window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await settle(80);
  const menu = document.querySelector("[class*=menu], [role=menu]");
  check("TC-3.16a chuột phải mở trình đơn", !!menu, "không thấy trình đơn");
  key("ArrowDown");
  await settle(40);
  check(
    "TC-3.16b trình đơn mở thì App không tự đổi dòng chọn",
    selIndexes(div).join(",") === "2" || selIndexes(div).join(",") === before,
    `chọn=[${selIndexes(div)}]`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-3.17
// Tìm kiếm rỗng thì hiện lời mời, không phải "không tìm thấy".
{
  baseHandlers(ipc);
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  await settle(120);
  const empty = div.querySelector(".empty");
  check("TC-3.17a chưa gõ gì thì hiện lời mời", !!empty && empty.textContent.includes("Gõ để tìm kiếm"), `"${empty?.textContent?.trim().slice(0, 50)}"`);
  check("TC-3.17b có nhắc phím tắt", !!div.querySelector(".hint"), "thiếu gợi ý phím tắt");
  unmount(app);
  div.remove();
}

// ---------------------------------------------------------------- TC-3.18
// Máy chưa quét bao giờ thì hiện màn hình chào, không phải "không tìm thấy".
{
  baseHandlers(ipc);
  ipc.on("index_status", { loaded: false, fileCount: 0, dirCount: 0, builtAtUnix: 0, problem: null });
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  await settle(150);
  check("TC-3.18a chưa quét thì hiện màn hình chào", !!div.querySelector(".firstrun"), "không thấy .firstrun");
  const primary = div.querySelector(".firstrun button.primary");
  check("TC-3.18b có nút Quét lần đầu", !!primary && primary.textContent.includes("Quét lần đầu"));
  unmount(app);
  div.remove();
  baseHandlers(ipc);
}

finish();
});
