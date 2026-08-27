// Nhóm 8 — lớp xem trước: khoá chuột lúc mở phải TỰ MỞ RA sau 800ms.
//
// Bug gốc: cơ chế chống cú double-click-mở-overlay lọt xuống video (làm cửa
// sổ tự phóng to toàn màn hình) khoá con chuột bằng `pointer-events: none`,
// nhưng dòng code bật lại chưa bao giờ tồn tại — video vĩnh viễn không bấm
// dừng hay tua được.
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { settle } from "./helpers";
import Preview from "../src/lib/Preview.svelte";
import type { SearchHit } from "../src/lib/search";

function mkHit(i: number, name: string, kind: SearchHit["kind"]): SearchHit {
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

const pending: (() => void)[] = [];
afterEach(() => {
  while (pending.length) pending.pop()!();
});

let closed: number;
let stepped: number[];

function mountPreview(hit: SearchHit) {
  closed = 0;
  stepped = [];
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(Preview, {
    target: div,
    props: {
      hit,
      epoch: 9,
      position: 1,
      total: 5,
      onclose: () => closed++,
      onstep: (d: number) => stepped.push(d),
      onopen: () => {},
    },
  });
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

const stage = (div: Element) => div.querySelector(".stage")!;

beforeEach(() => localStorage.clear());

describe("khoá chuột lúc mở overlay", () => {
  it("mở ra thì sân khấu đang khoá — đuôi cử chỉ double-click không được lọt xuống video", () => {
    const { div } = mountPreview(mkHit(1, "clip.mp4", "video"));
    expect(stage(div).classList.contains("disarmed"), "sân khấu phải khoá lúc vừa mở").toBe(true);
  });

  it("sau ~800ms khoá TỰ MỞ — đây chính là dòng code từng bị thiếu", async () => {
    const { div } = mountPreview(mkHit(1, "clip.mp4", "video"));
    await settle(600);
    expect(stage(div).classList.contains("disarmed"), "mở khoá quá sớm").toBe(true);
    await settle(350); // tổng ~950ms > 800ms
    expect(
      stage(div).classList.contains("disarmed"),
      "khoá không bao giờ mở — video không bấm dừng hay tua được",
    ).toBe(false);
  });

  it("double-click trong lúc còn khoá bị nuốt; sau khi mở khoá thì được tha", async () => {
    mountPreview(mkHit(1, "clip.mp4", "video"));
    const early = new MouseEvent("dblclick", { bubbles: true, cancelable: true });
    window.dispatchEvent(early);
    expect(early.defaultPrevented, "đuôi cử chỉ mở overlay phải bị nuốt").toBe(true);
    await settle(950);
    const late = new MouseEvent("dblclick", { bubbles: true, cancelable: true });
    window.dispatchEvent(late);
    expect(late.defaultPrevented, "double-click hợp lệ sau khi mở khoá vẫn bị nuốt").toBe(false);
  });

  it("bước sang tệp khác KHÔNG khoá lại — không có cử chỉ mới nào để phải đề phòng", async () => {
    const { div } = mountPreview(mkHit(1, "a.mp4", "video"));
    await settle(950);
    expect(stage(div).classList.contains("disarmed")).toBe(false);
    // đổi hit bằng cách... props không đổi được từ ngoài với mount() thường;
    // điều tương đương ở App là chỉ `hit` đổi còn component sống nguyên —
    // ở đây kiểm bất biến gần nhất: khoá không quay lại theo thời gian.
    await settle(300);
    expect(stage(div).classList.contains("disarmed"), "khoá tự quay lại").toBe(false);
  });
});

describe("phím Space — tạm dừng video, đóng với ảnh/nhạc", () => {
  const space = () =>
    window.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true, cancelable: true }));

  /// jsdom không có máy phát media — thay play/pause/paused bằng gián điệp.
  function spyVideo(div: Element, paused: boolean) {
    const video = div.querySelector("video") as HTMLVideoElement;
    expect(video, "không thấy thẻ video").toBeTruthy();
    const calls = { play: 0, pause: 0 };
    video.play = () => {
      calls.play++;
      return Promise.resolve();
    };
    video.pause = () => {
      calls.pause++;
    };
    Object.defineProperty(video, "paused", { configurable: true, get: () => paused });
    return calls;
  }

  it("video đang phát: Space tạm dừng, KHÔNG đóng overlay", async () => {
    const { div } = mountPreview(mkHit(1, "clip.mp4", "video"));
    await settle(20); // bind:this gán sau một nhịp flush
    const calls = spyVideo(div, false);
    space();
    expect(calls.pause).toBe(1);
    expect(calls.play).toBe(0);
    expect(closed, "overlay bị đóng ngay giữa đoạn đang xem").toBe(0);
  });

  it("video đang dừng: Space phát tiếp", async () => {
    const { div } = mountPreview(mkHit(1, "clip.mp4", "video"));
    await settle(20);
    const calls = spyVideo(div, true);
    space();
    expect(calls.play).toBe(1);
    expect(calls.pause).toBe(0);
    expect(closed).toBe(0);
  });

  it("ảnh: Space giữ nghĩa cũ — đóng", () => {
    mountPreview(mkHit(1, "anh.jpg", "image"));
    space();
    expect(closed).toBe(1);
  });

  it("nhạc: Space giữ nghĩa cũ — đóng", () => {
    mountPreview(mkHit(1, "bai.mp3", "audio"));
    space();
    expect(closed).toBe(1);
  });

  it("video hỏng (đang hiện fallback): Space đóng — không còn gì để dừng", async () => {
    const { div } = mountPreview(mkHit(1, "hong.mkv", "video"));
    div.querySelector("video")!.dispatchEvent(new Event("error"));
    await settle(20);
    expect(div.querySelector(".fallback"), "fallback chưa hiện").toBeTruthy();
    space();
    expect(closed).toBe(1);
  });

  it("gợi ý phím dưới chân chỉ nhắc Space khi là video", async () => {
    const a = mountPreview(mkHit(1, "clip.mp4", "video"));
    expect(a.div.querySelector("footer")!.textContent).toContain("tạm dừng");
    a.cleanup();
    const b = mountPreview(mkHit(2, "anh.jpg", "image"));
    expect(b.div.querySelector("footer")!.textContent).not.toContain("tạm dừng");
    b.cleanup();
  });
});

describe("bàn phím của overlay", () => {
  it("mũi tên bước tệp, Escape đóng — và sự kiện không lọt xuống dưới", () => {
    mountPreview(mkHit(1, "clip.mp4", "video"));
    const down = new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true });
    window.dispatchEvent(down);
    expect(stepped).toEqual([1]);
    expect(down.defaultPrevented).toBe(true);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true, cancelable: true }));
    expect(stepped).toEqual([1, -1]);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    expect(closed).toBe(1);
  });
});
