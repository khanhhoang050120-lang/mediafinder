// Nhóm 2 — App tích hợp.
//
// Kiểm những đường đi xuyên qua nhiều component: chip gọi ngược lên App, App
// truyền xuống FilterPanel, sự kiện của backend chạy qua cả cây.
// @ts-nocheck — kịch bản chuyển nguyên trạng từ harness cũ; các hàm
// trợ giúp cục bộ chưa gán kiểu. Bài mới viết thì phải có kiểu đầy đủ.
import { it, expect } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle, makeCollector } from "./helpers";
import App from "../src/App.svelte";

const ipc = new IpcRecorder();
globalThis.__ipc = ipc;

const { check, finish } = makeCollector();

it("Nhóm 2 — App tích hợp", async () => {

function hit(i, name, kind = "video", extra = {}) {
  return {
    index: i,
    name,
    dir: "D:\\media",
    path: `D:\\media\\${name}`,
    kind,
    matched: 1,
    size: 1024 * 1024,
    width: 1920,
    height: 1080,
    durationMs: 120000,
    ...extra,
  };
}

let searchArgs = [];
function baseHandlers(r) {
  // Prefs gio duoc luu qua cac lan mount; moi TC phai mo phien sach.
  localStorage.clear();
  searchArgs = [];
  r.on("index_status", { loaded: true, fileCount: 1000, dirCount: 50, builtAtUnix: 1700000000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 10, total: 10 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", { available: null, current: "1.0.1" })
    .on("search", (a) => {
      // Backend nhận { id, req: {...} }; bài kiểm thử quan tâm phần `req`.
      searchArgs.push(a.req);
      // `id` phải dội lại: searchFiles bỏ qua câu trả lời nào không mang đúng
      // số hiệu của lần hỏi mới nhất.
      return { id: a.id, hits: [hit(1, "phim.mp4"), hit(2, "anh.jpg", "image")], epoch: 7, relaxed: null, elapsedMs: 1.2, total: 2 };
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
    .on("reload_index", { loaded: true, fileCount: 2000, dirCount: 60, builtAtUnix: 1700000001, problem: null });
  return r;
}

async function mountApp() {
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  await settle(80);
  return { div, app, cleanup: () => { unmount(app); div.remove(); } };
}

const $ = (root, sel) => root.querySelector(sel);
const $$ = (root, sel) => [...root.querySelectorAll(sel)];
const chipByText = (root, text) =>
  $$(root, "button").find((b) => b.textContent.trim().startsWith(text));

function type(input, value) {
  input.value = value;
  input.dispatchEvent(new window.Event("input", { bubbles: true }));
}
function click(el) {
  // Không tìm thấy phần tử là một thất bại đáng báo cáo, không phải một vụ
  // sập làm mất hết các ca kiểm thử còn lại.
  if (!el) {
    check("click() gọi trên phần tử tồn tại", false, "selector không khớp nút nào");
    return;
  }
  el.dispatchEvent(new window.MouseEvent("click", { bubbles: true, cancelable: true }));
}

// ---------------------------------------------------------------- TC-2.1
// Gõ vào ô tìm kiếm phải gọi tới backend, và kết quả phải hiện ra.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  const input = $(div, "input.search");
  check("TC-2.1a ô tìm kiếm tồn tại", !!input);
  type(input, "phim");
  await settle(400); // coalesce có độ trễ
  check("TC-2.1b gõ thì gọi search", ipc.count("search") >= 1, `search gọi ${ipc.count("search")} lần`);
  const rows = $$(div, ".row");
  check("TC-2.1c kết quả hiện ra", rows.length === 2, `thấy ${rows.length} dòng`);
  const names = $$(div, ".name").map((n) => n.textContent.trim());
  check("TC-2.1d tên tệp đúng", names.includes("phim.mp4") && names.includes("anh.jpg"), names.join(","));
  cleanup();
}

// ---------------------------------------------------------------- TC-2.2
// Chip loại phải lọc và gọi lại search với activeKinds đúng.
// Đây là đường đi SearchBar -> App: chip nằm ở component con, còn activeKinds
// và runSearch ở component cha.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  type($(div, "input.search"), "abc");
  await settle(400);
  searchArgs = [];
  const videoChip = chipByText(div, "Video");
  check("TC-2.2a chip Video tồn tại", !!videoChip);
  click(videoChip);
  await settle(300);
  const last = searchArgs[searchArgs.length - 1];
  check(
    "TC-2.2b bấm chip Video thì search lại với kinds=[video]",
    last && JSON.stringify(last.kinds) === JSON.stringify(["video"]),
    `kinds=${JSON.stringify(last?.kinds)}`,
  );
  click(videoChip);
  await settle(300);
  const last2 = searchArgs[searchArgs.length - 1];
  check(
    "TC-2.2c bấm lại thì bỏ lọc",
    last2 && Array.isArray(last2.kinds) && last2.kinds.length === 0,
    `kinds=${JSON.stringify(last2?.kinds)}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.3
// Chip loại KHÔNG được gọi search khi ô tìm kiếm rỗng.
// Bản gốc gọi runSearch() vô điều kiện trong toggleKind; bản mới gọi rerun()
// có kiểm tra. Cả hai đều không nên bắn IPC khi không có truy vấn.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  const before = ipc.count("search");
  click(chipByText(div, "Video"));
  await settle(300);
  check(
    "TC-2.3 chip không bắn search khi ô tìm kiếm rỗng",
    ipc.count("search") === before,
    `search tăng thêm ${ipc.count("search") - before}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.4
// Nút Lọc mở FilterPanel, và chọn độ phân giải phải đi tới search.
// Đường đi: App -> FilterPanel -> $bindable filters -> App -> search.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  type($(div, "input.search"), "abc");
  await settle(400);
  check("TC-2.4a bảng lọc chưa hiện lúc đầu", !$(div, ".filters"));
  click(chipByText(div, "Lọc"));
  await settle(60);
  check("TC-2.4b bấm Lọc thì bảng lọc hiện ra", !!$(div, ".filters"));
  searchArgs = [];
  const p1080 = chipByText(div, "≥1080p");
  check("TC-2.4c có chip ≥1080p", !!p1080);
  click(p1080);
  await settle(300);
  const last = searchArgs[searchArgs.length - 1];
  check(
    "TC-2.4d chọn ≥1080p thì filters.minHeight=1080 đi tới search",
    last && last.filters && last.filters.minHeight === 1080,
    `filters=${JSON.stringify(last?.filters)}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.5
// Bộ lọc đang bật thì bảng lọc phải ở lại kể cả khi bấm tắt nút Lọc.
// (filtersActive giờ do FilterPanel ghi ngược lên App qua $bindable.)
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  click(chipByText(div, "Lọc"));
  await settle(60);
  click(chipByText(div, "4K"));
  await settle(200);
  click(chipByText(div, "Lọc")); // tắt showFilters
  await settle(80);
  check(
    "TC-2.5a bảng lọc ở lại khi còn bộ lọc đang bật",
    !!$(div, ".filters"),
    "bảng biến mất — người dùng mất đường tắt bộ lọc đang chặn kết quả",
  );
  const filterChip = chipByText(div, "Lọc");
  check(
    "TC-2.5b chip Lọc có dấu ● khi đang lọc",
    filterChip && filterChip.textContent.includes("●"),
    `nhãn chip = "${filterChip?.textContent.trim()}"`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.6
// Bỏ lọc phải xoá sạch và bảng lọc biến mất (vì showFilters đã tắt ở TC-2.5,
// nhưng ở đây showFilters vẫn bật nên bảng ở lại — kiểm giá trị filters).
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  type($(div, "input.search"), "abc");
  await settle(400);
  click(chipByText(div, "Lọc"));
  await settle(60);
  click(chipByText(div, "4K"));
  await settle(200);
  click(chipByText(div, "7 ngày"));
  await settle(200);
  searchArgs = [];
  const clearBtn = chipByText(div, "Bỏ lọc");
  check("TC-2.6a nút Bỏ lọc hiện ra khi có bộ lọc", !!clearBtn);
  click(clearBtn);
  await settle(300);
  const last = searchArgs[searchArgs.length - 1];
  check(
    "TC-2.6b Bỏ lọc trả filters về rỗng",
    last &&
      last.filters.minHeight === 0 &&
      last.filters.minDurationMs === 0 &&
      last.filters.maxDurationMs === 0 &&
      last.filters.withinDays === 0,
    `filters=${JSON.stringify(last?.filters)}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.7
// Nút sắp xếp phải đổi order và gọi lại search.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  type($(div, "input.search"), "abc");
  await settle(400);
  searchArgs = [];
  const sortChip = chipByText(div, "Liên quan");
  check("TC-2.7a chip sắp xếp mặc định là 'Liên quan'", !!sortChip);
  click(sortChip);
  await settle(300);
  const last = searchArgs[searchArgs.length - 1];
  check("TC-2.7b bấm thì order='newest'", last && last.order === "newest", `order=${last?.order}`);
  check(
    "TC-2.7c nhãn chip đổi thành 'Mới nhất'",
    !!chipByText(div, "Mới nhất"),
    "nhãn không đổi",
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.8
// Chế độ trùng lặp: bật thì DuplicateFinder xuất hiện, tắt thì biến mất và
// gửi cancel_duplicates.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  const dupeChip = chipByText(div, "Trùng lặp");
  check("TC-2.8a có chip Trùng lặp", !!dupeChip);
  click(dupeChip);
  await settle(120);
  check("TC-2.8b bật thì thanh dupebar hiện ra", !!$(div, ".dupebar"));
  check("TC-2.8c bật thì gọi find_duplicates", ipc.count("find_duplicates") === 1);
  click(chipByText(div, "Trùng lặp"));
  await settle(120);
  check("TC-2.8d tắt thì dupebar biến mất", !$(div, ".dupebar"));
  check(
    "TC-2.8e tắt thì gửi cancel_duplicates",
    ipc.count("cancel_duplicates") >= 1,
    `cancel_duplicates = ${ipc.count("cancel_duplicates")}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.9
// Sự kiện index-reloaded phải chạy lại tìm kiếm khi đang có truy vấn.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  type($(div, "input.search"), "phim");
  await settle(400);
  const before = ipc.count("search");
  await ipc.emit("index-reloaded", {});
  await settle(300);
  check(
    "TC-2.9 index-reloaded thì tìm lại",
    ipc.count("search") > before,
    `search trước ${before}, sau ${ipc.count("search")}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.10
// Sự kiện summon phải lấy con trỏ về ô tìm kiếm.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  const input = $(div, "input.search");
  input.blur();
  await ipc.emit("summon", {});
  await settle(80);
  check(
    "TC-2.10 summon lấy con trỏ về ô tìm kiếm",
    document.activeElement === input,
    `activeElement = ${document.activeElement?.tagName}.${document.activeElement?.className}`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.11
// Thanh trạng thái: số kết quả không được hiện khi đang ở chế độ trùng lặp.
// (Đây là chỗ tôi đã cố ý đổi hành vi so với bản gốc.)
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  type($(div, "input.search"), "phim");
  await settle(400);
  const timingBefore = $(div, ".timing");
  check("TC-2.11a có dòng '2 kết quả' khi đang tìm kiếm", !!timingBefore && timingBefore.textContent.includes("2"));
  click(chipByText(div, "Trùng lặp"));
  await settle(150);
  check(
    "TC-2.11b vào chế độ trùng lặp thì KHÔNG còn đếm kết quả tìm kiếm",
    !$(div, ".timing"),
    `vẫn thấy "${$(div, ".timing")?.textContent}" — đang đếm một danh sách không hiện trên màn hình`,
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.12
// Quét: bấm Quét lại thì gọi request_scan và thanh tiến trình hiện ra.
{
  baseHandlers(ipc);
  ipc.reset();
  // Chỉ "đang quét" sau khi lệnh quét thực sự được gửi — nếu không thì lúc
  // cửa sổ vừa mở nút đã mang nhãn "Đang quét…" và không tìm thấy "Quét lại".
  let started = false;
  ipc.on("request_scan", () => { started = true; return null; });
  ipc.on("scan_progress", () =>
    started
      ? { scanning: true, progress: { message: "Đang đọc ổ C:", phase: "local", volumesDone: 0, volumesTotal: 2, finished: false, error: null } }
      : { scanning: false, progress: null });
  const { div, cleanup } = await mountApp();
  click(chipByText(div, "Quét lại"));
  await settle(400);
  check("TC-2.12a bấm Quét lại thì gọi request_scan", ipc.count("request_scan") === 1);
  check("TC-2.12b thanh tiến trình quét hiện ra", !!$(div, ".scan"), "không thấy .scan");
  const msg = $(div, ".scan-head span");
  check(
    "TC-2.12c thanh hiện đúng thông điệp từ backend",
    msg && msg.textContent.includes("Đang đọc ổ C:"),
    `thấy "${msg?.textContent}"`,
  );
  check(
    "TC-2.12d nút Quét lại bị vô hiệu hoá khi đang quét",
    chipByText(div, "Đang quét")?.disabled === true,
    "nút vẫn bấm được — có thể khởi động hai lần quét chồng nhau",
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.13
// Nút Dừng chỉ hiện ở giai đoạn mạng.
{
  baseHandlers(ipc);
  ipc.reset();
  ipc.on("scan_progress", {
    scanning: true,
    progress: { message: "quét mạng", phase: "network", volumesDone: 1, volumesTotal: 2, finished: false, error: null },
  });
  ipc.on("network_drives", [{ letter: "Z", remote: "\\\\srv\\share" }]);
  const { div, cleanup } = await mountApp();
  await settle(80);
  const netBtn = chipByText(div, "+ ổ mạng");
  check("TC-2.13a có nút + ổ mạng khi máy có ổ mạng", !!netBtn);
  click(netBtn);
  await settle(400);
  check("TC-2.13b gọi request_scan_with_network", ipc.count("request_scan_with_network") === 1);
  const stopBtn = $(div, ".stop");
  check("TC-2.13c giai đoạn mạng thì có nút Dừng", !!stopBtn);
  if (stopBtn) {
    click(stopBtn);
    await settle(80);
    check("TC-2.13d bấm Dừng thì gọi cancel_scan", ipc.count("cancel_scan") === 1);
  } else {
    check("TC-2.13d bấm Dừng thì gọi cancel_scan", false, "không có nút để bấm");
  }
  cleanup();
}

// ---------------------------------------------------------------- TC-2.14
// Giai đoạn local thì KHÔNG được có nút Dừng (nút không làm gì được).
{
  baseHandlers(ipc);
  ipc.reset();
  let started14 = false;
  ipc.on("request_scan", () => { started14 = true; return null; });
  ipc.on("scan_progress", () =>
    started14
      ? { scanning: true, progress: { message: "quét local", phase: "local", volumesDone: 0, volumesTotal: 2, finished: false, error: null } }
      : { scanning: false, progress: null });
  const { div, cleanup } = await mountApp();
  click(chipByText(div, "Quét lại"));
  await settle(400);
  check(
    "TC-2.14 giai đoạn local thì không có nút Dừng",
    !$(div, ".stop"),
    "có nút Dừng mà bấm cũng không dừng được tiến trình nâng quyền",
  );
  cleanup();
}

// ---------------------------------------------------------------- TC-2.15
// Thông báo cập nhật: hộp thoại tự hiện, "Để sau" thu về mũi tên giữa footer.
// Chiều sâu của UX này nằm ở nhóm 9; ở đây chỉ giữ đường xương sống.
{
  baseHandlers(ipc);
  ipc.reset();
  const { div, cleanup } = await mountApp();
  check("TC-2.15a không có bản mới thì không hiện hộp thoại", !document.querySelector("[role=dialog]"));
  check("TC-2.15b không có bản mới thì không có mũi tên", !div.querySelector(".update-arrow"));
  cleanup();

  ipc.on("update_status", {
    checked: true,
    available: { version: "1.0.2", notes: "- Sửa lỗi cuộn" },
    current: "1.0.1",
  });
  const two = await mountApp();
  const dlg = document.querySelector("[role=dialog]");
  check("TC-2.15c có bản mới thì hộp thoại tự hiện", !!dlg);
  check("TC-2.15d hộp thoại nêu số hiệu bản mới", (dlg?.textContent ?? "").includes("1.0.2"));
  const later = [...document.querySelectorAll("[role=dialog] button")].find((b) =>
    b.textContent.trim().startsWith("Để sau"),
  );
  check("TC-2.15e có nút Để sau", !!later);
  later?.dispatchEvent(new window.MouseEvent("click", { bubbles: true, cancelable: true }));
  await settle(60);
  check(
    "TC-2.15f Để sau: hộp thoại đóng, mũi tên hiện giữa chân cửa sổ",
    !document.querySelector("[role=dialog]") && !!two.div.querySelector(".update-arrow"),
  );
  two.cleanup();
  baseHandlers(ipc);
}

finish();
});
