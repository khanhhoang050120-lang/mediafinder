// Nhóm 15 — nói thật về tuổi chỉ mục.
//
// Bối cảnh (BUG-025, P28): người dùng gõ đúng tên một tệp có thật, không thấy
// nó, nhìn xuống chân cửa sổ thấy "quét lúc 16:15" và kết luận phần mềm hỏng.
// Thật ra tệp nằm trên ổ mạng, mà ổ mạng lần cuối quét lúc 11:23 — chân cửa sổ
// chỉ biết `builtAtUnix`, và mốc đó bị đóng dấu lại ở MỌI lần ghi cache, kể cả
// lượt vá gia tăng ổ cục bộ. Nhóm này canh ba việc: diễn đạt tuổi cho đúng,
// nói ra ngay tại điểm hỏng, và thôi in một mốc duy nhất ở chân cửa sổ.
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import {
  daCu,
  moTaTuoi,
  NGUONG_CUC_BO_GIAY,
  NGUONG_O_MANG_GIAY,
} from "../src/lib/freshness";
import FreshnessNote from "../src/lib/FreshnessNote.svelte";
import App from "../src/App.svelte";

/// Mốc cố định để mọi phép tính thời gian trong nhóm này lặp lại được.
const BAY_GIO = 1_787_913_600_000; // 2026-08-28 17:20:00 giờ máy này
const giayTruoc = (n: number) => Math.floor(BAY_GIO / 1000) - n;

// ================================================================ đơn vị

describe("moTaTuoi — diễn đạt tuổi chỉ mục", () => {
  it("không có mốc thì trả chuỗi rỗng, để chỗ gọi tự quyết nói gì", () => {
    expect(moTaTuoi(0, BAY_GIO)).toBe("");
    expect(moTaTuoi(-5, BAY_GIO)).toBe("");
  });

  it("dưới một phút gộp thành 'vừa xong'", () => {
    expect(moTaTuoi(giayTruoc(0), BAY_GIO)).toBe("vừa xong");
    expect(moTaTuoi(giayTruoc(59), BAY_GIO)).toBe("vừa xong");
  });

  it("ranh giới phút/giờ/ngày rơi đúng chỗ", () => {
    expect(moTaTuoi(giayTruoc(60), BAY_GIO)).toBe("1 phút trước");
    expect(moTaTuoi(giayTruoc(59 * 60), BAY_GIO)).toBe("59 phút trước");
    expect(moTaTuoi(giayTruoc(60 * 60), BAY_GIO)).toBe("1 giờ trước");
    expect(moTaTuoi(giayTruoc(23 * 3600), BAY_GIO)).toBe("23 giờ trước");
    expect(moTaTuoi(giayTruoc(24 * 3600), BAY_GIO)).toBe("hôm qua");
    expect(moTaTuoi(giayTruoc(48 * 3600), BAY_GIO)).toBe("2 ngày trước");
  });

  it("đúng ca người dùng gặp: 2 giờ 25 phút trước", () => {
    // Tệp đến ổ 13:48:49, lần quét ổ mạng gần nhất 11:23:05.
    expect(moTaTuoi(giayTruoc(2 * 3600 + 25 * 60), BAY_GIO)).toBe("2 giờ trước");
  });

  it("đồng hồ lệch về tương lai không được in ra số âm", () => {
    // Mốc tới từ tệp do máy khác ghi, hoặc đồng hồ máy vừa được đồng bộ lại.
    // "-3 phút trước" trông như lỗi phần mềm và làm người dùng mất tin.
    expect(moTaTuoi(giayTruoc(-600), BAY_GIO)).toBe("vừa xong");
  });
});

describe("daCu — ngưỡng khác nhau cho hai loại ổ", () => {
  it("không có mốc thì không kết luận là cũ", () => {
    expect(daCu(0, NGUONG_CUC_BO_GIAY, BAY_GIO)).toBe(false);
  });

  it("ổ trong máy: 15 phút là bình thường, quá 30 phút là đáng nói", () => {
    // Tác vụ định kỳ chạy mỗi 15 phút; quá 30 phút nghĩa là nó không chạy.
    expect(daCu(giayTruoc(15 * 60), NGUONG_CUC_BO_GIAY, BAY_GIO)).toBe(false);
    expect(daCu(giayTruoc(31 * 60), NGUONG_CUC_BO_GIAY, BAY_GIO)).toBe(true);
  });

  it("ổ mạng: một giờ là bình thường, quá hai giờ mới đáng nhắc", () => {
    // Ngưỡng rộng hơn có chủ đích: ổ mạng chỉ làm mới khi có người bấm nút,
    // nên cảnh báo quá sớm sẽ hiện suốt ngày và người ta thôi đọc nó.
    expect(daCu(giayTruoc(3600), NGUONG_O_MANG_GIAY, BAY_GIO)).toBe(false);
    expect(daCu(giayTruoc(3 * 3600), NGUONG_O_MANG_GIAY, BAY_GIO)).toBe(true);
  });

  it("hai ngưỡng phải thật sự khác nhau", () => {
    // Bằng nhau thì việc tách làm hai hằng số là vô nghĩa, và ai đó sẽ gộp lại.
    expect(NGUONG_O_MANG_GIAY).toBeGreaterThan(NGUONG_CUC_BO_GIAY);
  });
});

// ================================================================ component

describe("FreshnessNote — chỉ lên tiếng khi có gì đáng nói", () => {
  let host: HTMLDivElement;
  let comp: Record<string, unknown> | null = null;

  function dung(props: {
    builtAtUnix: number;
    netMark: { atUnix: number; files: number; drives: number; seconds: number } | null;
    health: { taskExists: boolean } | null;
    check?: { atUnix: number; changed: boolean } | null;
    hasNetDrives?: boolean;
  }) {
    host = document.createElement("div");
    document.body.appendChild(host);
    comp = mount(FreshnessNote, { target: host, props }) as Record<string, unknown>;
    return host.textContent ?? "";
  }

  afterEach(() => {
    if (comp) unmount(comp);
    comp = null;
    host?.remove();
  });

  const TUOI = () => Math.floor(Date.now() / 1000);

  it("mọi thứ đều tươi thì im lặng hoàn toàn", () => {
    const t = dung({
      builtAtUnix: TUOI() - 60,
      netMark: { atUnix: TUOI() - 300, files: 320_528, drives: 3, seconds: 174 },
      health: { taskExists: true },
    });
    expect(t.trim()).toBe("");
  });

  it("chưa rõ mốc ổ mạng thì nói ra và chỉ đúng nút cần bấm", () => {
    // Câu cũ ở đây là "chưa quét lần nào" và nó NÓI SAI trên mọi máy nâng cấp:
    // `netscan.json` là tệp mới của bản này, nên nó vắng mặt kể cả trên máy
    // đang có 320.505 mục ổ mạng trong chỉ mục. Chưa biết thì nói là chưa biết.
    const t = dung({
      builtAtUnix: TUOI() - 60,
      netMark: null,
      health: { taskExists: true },
    });
    expect(t, "khong duoc khang dinh mot dieu minh khong biet").not.toContain(
      "chưa quét lần nào",
    );
    expect(t).toContain("chưa rõ lần trước");
    expect(t).toContain("+ ổ mạng");
  });

  it("mất tác vụ định kỳ thì cảnh báo và chỉ đường tự cứu", () => {
    // Đây là nhóm máy dính BUG-024: móc gỡ cài đặt xoá luôn tác vụ, và trước
    // bản này không có gì trên màn hình nói cho họ biết. Không có câu này thì
    // họ không bao giờ biết mà bấm.
    const t = dung({
      builtAtUnix: TUOI() - 60,
      netMark: { atUnix: TUOI() - 300, files: 1, drives: 1, seconds: 1 },
      health: { taskExists: false },
    });
    expect(t).toContain("Không còn tác vụ làm mới định kỳ");
    expect(t).toContain("Quét lại");
  });

  it("chưa hỏi được sức khoẻ tác vụ thì không được doạ nhầm", () => {
    // `health === null` là "chưa biết", không phải "đã mất".
    const t = dung({
      builtAtUnix: TUOI() - 60,
      netMark: { atUnix: TUOI() - 300, files: 1, drives: 1, seconds: 1 },
      health: null,
    });
    expect(t).not.toContain("Không còn tác vụ");
  });

  it("ổ mạng cũ thì hiện CẢ HAI mốc, không chỉ một", () => {
    // Bất biến trung tâm của cả nhóm: một mốc duy nhất là thứ đã làm người
    // dùng kết luận sai.
    const t = dung({
      builtAtUnix: TUOI() - 60,
      netMark: { atUnix: TUOI() - 5 * 3600, files: 320_528, drives: 3, seconds: 174 },
      health: { taskExists: true },
    });
    expect(t).toContain("Ổ trong máy");
    expect(t).toContain("Ổ mạng");
    expect(t).toContain("5 giờ trước");
  });

  it("ổ trong máy cũ cũng đủ để lên tiếng, dù ổ mạng vừa quét", () => {
    const t = dung({
      builtAtUnix: TUOI() - 3600,
      netMark: { atUnix: TUOI() - 60, files: 1, drives: 1, seconds: 1 },
      health: { taskExists: true },
    });
    expect(t).toContain("Ổ trong máy");
    expect(t).toContain("1 giờ trước");
  });
});

// ================================================================ tích hợp

describe("App — nói tuổi ở CẢ HAI trạng thái hỏng", () => {
  let r: IpcRecorder;
  let app: Record<string, unknown> | null = null;
  let host: HTMLDivElement;
  let served: unknown[] = [];
  let relaxed: unknown = null;

  const CU = Math.floor(Date.now() / 1000) - 5 * 3600;

  beforeEach(() => {
    r = new IpcRecorder();
    (globalThis as { __ipc?: IpcRecorder }).__ipc = r;
    served = [];
    relaxed = null;
    r.on("index_status", {
      loaded: true,
      fileCount: 368_959,
      dirCount: 15_096,
      builtAtUnix: Math.floor(Date.now() / 1000) - 60,
      problem: null,
    })
      .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
      .on("enrich_status", { running: false, done: 1, total: 1 })
      .on("scan_progress", { scanning: false, progress: null })
      .on("network_drives", [{ letter: "Y", remote: "\\NAS\padoma" }])
      .on("update_status", { checked: true, available: null, current: "1.0.5" })
      .on("net_scan_mark", { atUnix: CU, files: 320_528, drives: 3, seconds: 174.1 })
      .on("task_health", { taskExists: true })
      .on("last_check", { atUnix: Math.floor(Date.now() / 1000) - 60, changed: false })
      .on("miss_log_status", { enabled: false, count: 0 })
      .on("search", (a: { id: number }) => ({
        id: a.id,
        hits: served,
        epoch: 3,
        relaxed,
        elapsedMs: 1,
        total: served.length,
      }))
      .on("dupe_progress", {
        running: false,
        completed: false,
        groups: 0,
        wasted: 0,
        hashed: 0,
        candidates: 0,
      })
      .on("dupe_groups", [])
      .on("cancel_duplicates", null);
    host = document.createElement("div");
    document.body.appendChild(host);
  });

  afterEach(() => {
    if (app) unmount(app);
    app = null;
    host?.remove();
  });

  async function moUngDung(query: string) {
    app = mount(App, { target: host }) as Record<string, unknown>;
    await settle(20);
    const box = host.querySelector("input") as HTMLInputElement;
    box.value = query;
    box.dispatchEvent(new Event("input", { bubbles: true }));
    await settle(300);
  }

  it("chân cửa sổ nói HAI mốc, không còn một mốc 'quét lúc' nói dối", async () => {
    app = mount(App, { target: host }) as Record<string, unknown>;
    await settle(20);
    const t = host.textContent ?? "";
    expect(t).toContain("ổ trong máy");
    expect(t).toContain("ổ mạng");
    // Câu cũ in đúng một mốc và gọi nó là "quét lúc" — đó chính là lời nói dối
    // khi nửa ổ mạng của chỉ mục già hơn nhiều giờ.
    expect(t).not.toContain("quét lúc");
  });

  it("trạng thái 'Không tìm thấy kết quả nào' không còn im lặng về tuổi", async () => {
    // Đây là ca P28-2: tệp vừa chép xong, chỉ mục chưa biết, và trước bản này
    // nhánh 0-kết-quả không nói một chữ nào về tuổi chỉ mục.
    served = [];
    await moUngDung("kiemthu-tep-vua-chep-xong");
    const t = host.textContent ?? "";
    expect(t).toContain("Không tìm thấy kết quả nào");
    expect(t).toContain("Ổ mạng");
    expect(t).toContain("5 giờ trước");
  });

  it("băng 'khớp nhiều nhất' cũng kèm tuổi chỉ mục", async () => {
    served = [
      {
        index: 1,
        name: "khac.mp4",
        dir: "Y:\\a",
        path: "Y:\\a\\khac.mp4",
        kind: "video",
        matched: 10,
        size: 1,
        width: 0,
        height: 0,
        durationMs: 0,
      },
    ];
    relaxed = { totalTokens: 16, bestMatched: 10 };
    await moUngDung("a-lady-enjoying-swimming-with-the-huge-whale-shark");
    const t = host.textContent ?? "";
    expect(t).toContain("khớp nhiều nhất");
    expect(t).toContain("Ổ mạng");
    expect(t).toContain("5 giờ trước");
  });

  it("hỏi sức khoẻ tác vụ ĐÚNG MỘT LẦN lúc mở, không phải mỗi lần gõ", async () => {
    // Mỗi lượt sinh một tiến trình schtasks.exe; gọi theo phím gõ là tự bắn
    // vào chân về hiệu năng.
    served = [];
    await moUngDung("abc");
    expect(r.count("task_health")).toBe(1);
    expect(r.count("net_scan_mark")).toBe(1);
  });
});

describe("FreshnessNote — không doạ nhầm và không nói sai", () => {
  let host: HTMLDivElement;
  let comp: Record<string, unknown> | null = null;
  const TUOI = () => Math.floor(Date.now() / 1000);

  function dung(props: Record<string, unknown>) {
    host = document.createElement("div");
    document.body.appendChild(host);
    comp = mount(FreshnessNote, { target: host, props: props as never }) as Record<string, unknown>;
    return host.textContent ?? "";
  }
  afterEach(() => {
    if (comp) unmount(comp);
    comp = null;
    host?.remove();
  });

  it("máy vừa nâng cấp: KHÔNG được nói 'chưa quét lần nào' về ổ mạng", () => {
    // `netscan.json` là tệp mới của bản này nên nó vắng mặt trên MỌI máy nâng
    // cấp — kể cả máy đang có 320.505 mục ổ mạng trong chỉ mục. Nói "chưa quét
    // lần nào" ở đó là nói sai, và nói sai đúng trên màn hình được dựng lên để
    // "thôi để họ kết luận sai".
    const t = dung({
      builtAtUnix: TUOI() - 60,
      netMark: null,
      health: { taskExists: true },
      check: { atUnix: TUOI() - 60, changed: false },
    });
    expect(t).not.toContain("chưa quét lần nào");
    expect(t).toContain("chưa rõ lần trước");
  });

  it("máy yên tĩnh buổi tối: tác vụ vừa chạy thì KHÔNG tô cảnh báo", () => {
    // Bản vá gia tăng cố ý không ghi lại cache khi không có gì đổi, nên
    // `builtAtUnix` đứng yên hàng giờ trên một máy hoàn toàn khoẻ. Lấy tuổi từ
    // lần KIỂM chứ không phải lần ĐỔI mới trả lời đúng câu hỏi.
    const t = dung({
      builtAtUnix: TUOI() - 5 * 3600, // chỉ mục đổi lần cuối 5 giờ trước
      netMark: { atUnix: TUOI() - 300, files: 1, drives: 1, seconds: 1 },
      health: { taskExists: true },
      check: { atUnix: TUOI() - 120, changed: false }, // nhưng vừa kiểm 2 phút trước
    });
    expect(t.trim(), "may khoe ma van bi to canh bao").toBe("");
  });

  it("không có mốc kiểm thì lùi về mốc ghi cache, không im lặng luôn", () => {
    const t = dung({
      builtAtUnix: TUOI() - 5 * 3600,
      netMark: { atUnix: TUOI() - 300, files: 1, drives: 1, seconds: 1 },
      health: { taskExists: true },
      check: null,
    });
    expect(t).toContain("5 giờ trước");
  });

  it("không gắn ổ mạng nào thì đừng nhắc tới ổ mạng", () => {
    const t = dung({
      builtAtUnix: TUOI() - 5 * 3600,
      netMark: null,
      health: { taskExists: true },
      check: { atUnix: TUOI() - 5 * 3600, changed: false },
      hasNetDrives: false,
    });
    expect(t).toContain("Ổ trong máy");
    expect(t, "may khong co NAS ma van bi hoi ve o mang").not.toContain("Ổ mạng");
  });
});
