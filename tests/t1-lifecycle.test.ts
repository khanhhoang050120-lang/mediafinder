// Nhóm 1 — vòng đời và IPC.
//
// Đây là chỗ việc tách file dễ làm hỏng nhất: mỗi component giờ tự khởi động
// và tự dọn dẹp, mà trước kia App làm hộ. Một nhịp hẹn giờ sống sót sau khi
// component biến mất, hay một lệnh huỷ không được gửi, đều không hiện ra
// trong svelte-check hay vite build.
// @ts-nocheck — kịch bản chuyển nguyên trạng từ harness cũ; các hàm
// trợ giúp cục bộ chưa gán kiểu. Bài mới viết thì phải có kiểu đầy đủ.
import { it, expect } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle, makeCollector } from "./helpers";
import DuplicateFinder from "../src/lib/DuplicateFinder.svelte";
import { ScanState } from "../src/lib/scanState.svelte";

const ipc = new IpcRecorder();
globalThis.__ipc = ipc;

const { check, finish } = makeCollector();

it("Nhóm 1 — vòng đời & IPC", async () => {

function baseHandlers(r) {
  r.on("index_status", { loaded: true, fileCount: 1000, dirCount: 50, builtAtUnix: 1700000000, problem: null })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 10, total: 10 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", { available: null, current: "1.0.1" })
    .on("search", { hits: [], epoch: 1, relaxed: null, elapsedMs: 0.5, total: 0 })
    .on("dupe_progress", { running: false, completed: false, groups: 0, wasted: 0, hashed: 0, candidates: 0 })
    .on("dupe_groups", [])
    .on("find_duplicates", null)
    .on("cancel_duplicates", null)
    .on("cancel_scan", null)
    .on("request_scan", null)
    .on("request_scan_with_network", null)
    .on("reload_index", { loaded: true, fileCount: 2000, dirCount: 60, builtAtUnix: 1700000001, problem: null });
  return r;
}

const host = document.body;

// ---------------------------------------------------------------- TC-1.1
// DuplicateFinder phải gửi cancel_duplicates khi bị gỡ khỏi cây.
// Ở bản gốc việc này do exitDupes() làm; giờ nó nằm trong cleanup của $effect.
// Nếu sai, ổ đĩa tiếp tục bị đọc sau khi người dùng đã rời màn hình.
{
  baseHandlers(ipc);
  ipc.reset();
  const div = document.createElement("div");
  host.appendChild(div);
  const app = mount(DuplicateFinder, {
    target: div,
    props: {
      epoch: 1,
      rowHeight: 46,
      thumbSize: 64,
      onerror: () => {},
      onopen: () => {},
      oncontextmenu: () => {},
    },
  });
  await settle(50);
  const startedScan = ipc.count("find_duplicates") === 1;
  unmount(app);
  await settle(50);
  check(
    "TC-1.1a DuplicateFinder bắt đầu quét khi gắn vào",
    startedScan,
    `find_duplicates gọi ${ipc.count("find_duplicates")} lần`,
  );
  check(
    "TC-1.1b DuplicateFinder gửi cancel_duplicates khi bị gỡ",
    ipc.count("cancel_duplicates") >= 1,
    `cancel_duplicates gọi ${ipc.count("cancel_duplicates")} lần — ổ đĩa sẽ đọc tiếp nếu bằng 0`,
  );
  div.remove();
}

// ---------------------------------------------------------------- TC-1.2
// Nhịp hẹn giờ của DuplicateFinder phải chết theo component.
// Kiểm bằng cách đếm dupe_progress một lúc sau khi đã gỡ.
{
  ipc.reset();
  ipc.on("dupe_progress", { running: true, completed: false, groups: 0, wasted: 0, hashed: 5, candidates: 100 });
  const div = document.createElement("div");
  host.appendChild(div);
  const app = mount(DuplicateFinder, {
    target: div,
    props: { epoch: 1, rowHeight: 46, thumbSize: 64, onerror: () => {}, onopen: () => {}, oncontextmenu: () => {} },
  });
  await settle(900); // vài nhịp 400ms
  const during = ipc.count("dupe_progress");
  unmount(app);
  await settle(1000);
  const after = ipc.count("dupe_progress");
  check(
    "TC-1.2 nhịp hẹn giờ dupe dừng sau khi gỡ component",
    after === during,
    `trong lúc chạy ${during}, sau khi gỡ ${after} — chênh ${after - during} nghĩa là timer còn sống`,
  );
  div.remove();
  baseHandlers(ipc);
}

// ---------------------------------------------------------------- TC-1.3
// Quét đang chạy sẵn lúc mở màn hình trùng lặp thì phải theo dõi, không được
// gọi find_duplicates lần nữa (sẽ ném đi công sức đang có).
{
  ipc.reset();
  ipc.on("dupe_progress", { running: true, completed: false, groups: 0, wasted: 0, hashed: 20, candidates: 100 });
  const div = document.createElement("div");
  host.appendChild(div);
  const app = mount(DuplicateFinder, {
    target: div,
    props: { epoch: 1, rowHeight: 46, thumbSize: 64, onerror: () => {}, onopen: () => {}, oncontextmenu: () => {} },
  });
  await settle(60);
  check(
    "TC-1.3 không quét lại khi đã có lần quét đang chạy",
    ipc.count("find_duplicates") === 0,
    `find_duplicates gọi ${ipc.count("find_duplicates")} lần — phải là 0`,
  );
  unmount(app);
  await settle(30);
  div.remove();
  baseHandlers(ipc);
}

// ---------------------------------------------------------------- TC-1.4
// Lần quét đã xong thì phải lấy kết quả cũ, không quét lại.
{
  ipc.reset();
  ipc.on("dupe_progress", { running: false, completed: true, groups: 2, wasted: 1024, hashed: 100, candidates: 100 });
  ipc.on("dupe_groups", [
    { size: 512, wasted: 512, files: [
      { index: 1, name: "a.mp4", dir: "D:\\x", path: "D:\\x\\a.mp4", kind: "video", matched: 0, size: 512, width: 0, height: 0, durationMs: 0 },
      { index: 2, name: "b.mp4", dir: "D:\\y", path: "D:\\y\\b.mp4", kind: "video", matched: 0, size: 512, width: 0, height: 0, durationMs: 0 },
    ] },
  ]);
  const div = document.createElement("div");
  host.appendChild(div);
  const app = mount(DuplicateFinder, {
    target: div,
    props: { epoch: 1, rowHeight: 46, thumbSize: 64, onerror: () => {}, onopen: () => {}, oncontextmenu: () => {} },
  });
  await settle(60);
  check(
    "TC-1.4a dùng lại kết quả đã hoàn tất, không quét lại",
    ipc.count("find_duplicates") === 0,
    `find_duplicates gọi ${ipc.count("find_duplicates")} lần`,
  );
  check("TC-1.4b có lấy dupe_groups", ipc.count("dupe_groups") === 1);
  unmount(app);
  await settle(30);
  div.remove();
  baseHandlers(ipc);
}

// ---------------------------------------------------------------- TC-1.5
// ScanState: nhịp hẹn giờ phải dừng khi quét xong, và onreload phải bắn.
{
  ipc.reset();
  let seq = 0;
  ipc.on("scan_progress", () => {
    seq++;
    if (seq <= 2)
      return { scanning: true, progress: { message: "đang quét", phase: "local", volumesDone: 0, volumesTotal: 2, finished: false, error: null } };
    return { scanning: true, progress: { message: "xong", phase: "local", volumesDone: 2, volumesTotal: 2, finished: true, error: null } };
  });
  let reloaded = null;
  let errored = null;
  const st = new ScanState({ onreload: (m) => (reloaded = m), onerror: (e) => (errored = e) });
  await st.start(false);
  await settle(1200);
  const countAtEnd = ipc.count("scan_progress");
  await settle(800);
  check(
    "TC-1.5a ScanState gọi onreload khi quét xong",
    reloaded !== null && reloaded.fileCount === 2000,
    `reloaded=${JSON.stringify(reloaded)}`,
  );
  check("TC-1.5b ScanState không báo lỗi khi mọi thứ suôn sẻ", errored === null, `errored=${errored}`);
  check(
    "TC-1.5c nhịp hẹn giờ dừng sau khi quét xong",
    ipc.count("scan_progress") === countAtEnd,
    `sau khi xong còn gọi thêm ${ipc.count("scan_progress") - countAtEnd} lần`,
  );
  check("TC-1.5d scanning trở về false", st.scanning === false, `scanning=${st.scanning}`);
  st.dispose();
  baseHandlers(ipc);
}

// ---------------------------------------------------------------- TC-1.6
// ScanState: tiến trình con chết giữa chừng phải báo lỗi, không quay mãi.
{
  ipc.reset();
  let seq = 0;
  ipc.on("scan_progress", () => {
    seq++;
    if (seq <= 1)
      return { scanning: true, progress: { message: "đang quét", phase: "local", volumesDone: 0, volumesTotal: 2, finished: false, error: null } };
    return { scanning: false, progress: { message: "", phase: "local", volumesDone: 0, volumesTotal: 2, finished: false, error: null } };
  });
  let errored = null;
  const st = new ScanState({ onreload: () => {}, onerror: (e) => (errored = e) });
  await st.start(false);
  await settle(900);
  check(
    "TC-1.6a tiến trình quét chết bất thường thì báo lỗi",
    typeof errored === "string" && errored.includes("bất thường"),
    `errored=${errored}`,
  );
  check("TC-1.6b scanning trở về false sau khi chết", st.scanning === false);
  st.dispose();
  baseHandlers(ipc);
}

// ---------------------------------------------------------------- TC-1.7
// ScanState: từ chối UAC (request_scan ném lỗi) phải báo qua onerror và
// KHÔNG được bật scanning — nếu bật, thanh tiến trình quay mãi mà không có gì chạy.
{
  ipc.reset();
  ipc.on("request_scan", () => {
    throw new Error("Bạn đã từ chối quyền Administrator.");
  });
  let errored = null;
  const st = new ScanState({ onreload: () => {}, onerror: (e) => (errored = e) });
  await st.start(false);
  await settle(50);
  check("TC-1.7a từ chối UAC thì báo lỗi", errored !== null && String(errored).includes("Administrator"), `errored=${errored}`);
  check(
    "TC-1.7b từ chối UAC thì KHÔNG bật trạng thái đang quét",
    st.scanning === false,
    `scanning=${st.scanning} — nếu true thì thanh tiến trình quay vĩnh viễn`,
  );
  st.dispose();
  baseHandlers(ipc);
}

// ---------------------------------------------------------------- TC-1.8
// ScanState.dispose() phải dừng nhịp hẹn giờ.
{
  ipc.reset();
  ipc.on("scan_progress", {
    scanning: true,
    progress: { message: "quét", phase: "local", volumesDone: 0, volumesTotal: 1, finished: false, error: null },
  });
  const st = new ScanState({ onreload: () => {}, onerror: () => {} });
  await st.start(false);
  await settle(700);
  const during = ipc.count("scan_progress");
  st.dispose();
  await settle(700);
  check(
    "TC-1.8 dispose() dừng nhịp hẹn giờ quét",
    ipc.count("scan_progress") === during,
    `sau dispose còn gọi thêm ${ipc.count("scan_progress") - during} lần`,
  );
  baseHandlers(ipc);
}

finish();
});
