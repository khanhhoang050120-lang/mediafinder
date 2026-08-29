// Nhóm 10 — lọc theo ổ đĩa.
//
// Hai tầng, cố ý:
//  * Phần đầu kiểm `drives.ts` thuần — nơi mọi quyết định thật sự nằm.
//  * Phần sau dựng App thật và bấm chip, vì các hàm thuần chạy đúng không
//    chứng minh được rằng chúng đã được NỐI vào danh sách kết quả. Bản v1.0.6
//    từng có đúng lỗi kiểu này: một nhánh chạy đúng trong kiểm thử nhưng là mã
//    chết trên máy thật.
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { readFile } from "node:fs/promises";
import { IpcRecorder, settle } from "./helpers";
import App from "../src/App.svelte";
import {
  bucketsFor,
  driveKey,
  driveLabel,
  filterByDrive,
  isNetworkDrive,
  networkLetters,
} from "../src/lib/drives";
import type { SearchHit } from "../src/lib/search";

const SEP = String.fromCharCode(92);

function mkHit(i: number, path: string): SearchHit {
  const cut = path.lastIndexOf("\\");
  return {
    index: i,
    name: path.slice(cut + 1),
    dir: path.slice(0, cut),
    path,
    kind: "video",
    matched: 1,
    size: 4096,
    width: 1920,
    height: 1080,
    durationMs: 60_000,
  } as SearchHit;
}

describe("t10 — nhận diện ổ từ đường dẫn", () => {
  it("đọc được cả ổ chữ cái lẫn UNC, và không đoán bừa với đường dẫn lạ", () => {
    expect(driveKey("D:\\media\\a.mp4")).toBe("D");
    // Chữ thường phải gộp chung với chữ hoa, nếu không `c:\` và `C:\` thành
    // hai chip riêng cho cùng một ổ.
    expect(driveKey("c:\\media\\a.mp4")).toBe("C");
    expect(driveKey("\\\\NAS\\share\\a.mp4")).toBe("\\\\NAS");
    expect(driveKey("\\\\nas\\share\\a.mp4")).toBe("\\\\NAS");

    // Không nhận ra thì trả rỗng để người gọi bỏ qua — dựng một chip vô nghĩa
    // còn tệ hơn là không dựng gì.
    expect(driveKey("")).toBe("");
    expect(driveKey("media\\a.mp4")).toBe("");
    expect(driveKey("\\\\")).toBe("");
  });

  it("giữ hai gạch chéo trong khoá UNC để máy chủ tên NAS không lẫn với ổ N", () => {
    expect(driveKey("\\\\NAS\\s\\a.mp4")).not.toBe(driveKey("N:\\a.mp4"));
    expect(driveLabel("\\\\NAS")).toBe("NAS");
    expect(driveLabel("D")).toBe("D:");
  });
});

describe("t10 — ổ mạng ánh xạ", () => {
  it("nhận ra ổ mạng ánh xạ, thứ mà cách nhận dạng chỉ-UNC bỏ sót hoàn toàn", () => {
    // Đây là ca thật của studio: cả bốn ổ NAS đều là ổ ánh xạ, nên nếu chỉ
    // nhận UNC thì toàn bộ nhánh ổ mạng không bao giờ chạy trên máy nào.
    const net = networkLetters([
      { letter: "Y:", remote: "\\\\NAS\\p" },
      { letter: "Z", remote: "\\\\NAS\\q" },
    ]);
    expect(isNetworkDrive("Y", net)).toBe(true);
    // Backend đã thấy trả về cả "Z" lẫn "Z:" — một dấu hai chấm không được
    // phép quyết định tính năng có chạy hay không.
    expect(isNetworkDrive("Z", net)).toBe(true);
    expect(isNetworkDrive("D", net)).toBe(false);
    // UNC thì tự nó đã nói ra, không cần danh sách.
    expect(isNetworkDrive("\\\\NAS", net)).toBe(true);
    expect(isNetworkDrive("\\\\NAS")).toBe(true);
    // Chưa có danh sách (khoảnh khắc trước khi backend trả lời) thì lùi về
    // cách cũ chứ không nổ.
    expect(isNetworkDrive("Y")).toBe(false);
  });
});

describe("t10 — gom nhóm và xếp thứ tự", () => {
  const hits = [
    mkHit(0, "Y:\\p\\a.mp4"),
    mkHit(1, "D:\\m\\b.mp4"),
    mkHit(2, "Y:\\p\\c.mp4"),
    mkHit(3, "C:\\m\\d.mp4"),
    mkHit(4, "\\\\NAS\\s\\e.mp4"),
  ];
  const net = networkLetters([{ letter: "Y:", remote: "\\\\NAS\\p" }]);

  it("đếm đúng và đẩy ổ mạng xuống cuối hàng", () => {
    const bs = bucketsFor(hits, net);
    // Ổ trong máy trước (C, D theo chữ cái), rồi ổ mạng (NAS, Y).
    expect(bs.map((b) => b.label)).toEqual(["C:", "D:", "NAS", "Y:"]);
    expect(bs.map((b) => b.count)).toEqual([1, 1, 1, 2]);
    expect(bs.map((b) => b.network)).toEqual([false, false, true, true]);
  });

  it("không dựng nhóm cho đường dẫn không đọc được", () => {
    expect(bucketsFor([mkHit(0, "khong-co-o.mp4")], net)).toEqual([]);
  });

  it("thiếu danh sách ổ mạng thì vẫn đếm đúng — chỉ mất phần phân biệt", () => {
    // Quan trọng: hỏng phần nhận dạng KHÔNG được làm hỏng phần lọc. Đây chính
    // là lý do một lỗi kiểu ổ-ánh-xạ có thể sống sót mà không ai báo.
    const bs = bucketsFor(hits);
    expect(bs.find((b) => b.id === "Y")?.count).toBe(2);
    expect(bs.find((b) => b.id === "Y")?.network).toBe(false);
  });
});

describe("t10 — lọc", () => {
  const hits = [
    mkHit(0, "D:\\m\\a.mp4"),
    mkHit(1, "Y:\\p\\b.mp4"),
    mkHit(2, "D:\\m\\c.mp4"),
  ];

  it("null trả về đúng mảng cũ, không phải bản sao", () => {
    // Trả bản sao thì mỗi lần tính lại sinh một mảng mới, và danh sách ảo hoá
    // dựng lại toàn bộ dòng dù chẳng có gì đổi.
    expect(filterByDrive(hits, null)).toBe(hits);
  });

  it("lọc theo khoá ổ và giữ nguyên thứ tự liên quan", () => {
    expect(filterByDrive(hits, "D").map((h) => h.name)).toEqual([
      "a.mp4",
      "c.mp4",
    ]);
    expect(filterByDrive(hits, "Y").map((h) => h.name)).toEqual(["b.mp4"]);
    expect(filterByDrive(hits, "Q")).toEqual([]);
  });
});

// ---- Tầng hai: nối vào ứng dụng thật ----
//
// Các hàm trên chạy đúng vẫn chưa nói được gì về việc chúng có được NỐI vào
// danh sách kết quả hay không. Phần này dựng App thật rồi bấm chip.

const HITS = [
  mkHit(1, "D:" + SEP + "m" + SEP + "alpha.mp4"),
  mkHit(2, "Y:" + SEP + "p" + SEP + "beta.mp4"),
  mkHit(3, "D:" + SEP + "m" + SEP + "gamma.mp4"),
  mkHit(4, "Y:" + SEP + "p" + SEP + "delta.mp4"),
];

let ipc: IpcRecorder;

function dungIpc(hits: SearchHit[]): void {
  ipc = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = ipc;
  ipc
    .on("index_status", {
      loaded: true,
      fileCount: hits.length,
      dirCount: 2,
      builtAtUnix: 1_700_000_000,
      problem: null,
    })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    // Hai ổ mạng ánh xạ, giống hệt máy studio: Y: là ổ mạng, D: là đĩa trong máy.
    .on("network_drives", [
      { letter: "Y:", remote: SEP + SEP + "NAS" + SEP + "p" },
    ])
    .on("update_status", { checked: true, available: null, current: "1.0.4" })
    .on("search", (a: { id: number }) => ({
      id: a.id,
      hits,
      epoch: 3,
      relaxed: null,
      elapsedMs: 1,
      total: hits.length,
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
}

const dangMo: (() => void)[] = [];
afterEach(() => {
  while (dangMo.length) dangMo.pop()!();
  localStorage.clear();
});

/// Dựng App, gõ một truy vấn, chờ kết quả về.
async function goTimKiem(): Promise<HTMLElement> {
  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  dangMo.push(() => {
    unmount(app);
    div.remove();
  });
  await settle(90);
  const o = div.querySelector("input.search") as HTMLInputElement;
  o.value = "a";
  o.dispatchEvent(new window.Event("input", { bubbles: true }));
  await settle(300);
  return div;
}

/// Các chip ổ, nhận ra bằng chữ trên mặt chip.
function chips(host: HTMLElement): HTMLButtonElement[] {
  return [...host.querySelectorAll<HTMLButtonElement>(".drives .dchip")];
}

const bam = (el: Element) =>
  el.dispatchEvent(
    new MouseEvent("click", { bubbles: true, cancelable: true }),
  );

describe("t10 — chip ổ đĩa thực sự lọc danh sách", () => {
  beforeEach(() => {
    localStorage.clear();
    dungIpc(HITS);
  });

  it("hiện hàng chip với số đếm đúng, ổ mạng đứng cuối", async () => {
    const div = await goTimKiem();
    const chu = chips(div).map((c) =>
      c.textContent!.replace(/\s+/g, " ").trim(),
    );
    expect(chu.length, `chip thấy được: ${JSON.stringify(chu)}`).toBe(3);
    expect(chu[0]).toContain("Tất cả");
    expect(chu[0]).toContain("4");
    // D: (đĩa trong máy) đứng trước Y: (ổ mạng ánh xạ) — chứng minh nhánh ổ
    // ánh xạ sống trên đường chạy thật, chứ không chỉ trong kiểm thử đơn vị.
    expect(chu[1]).toContain("D:");
    expect(chu[2]).toContain("Y:");
  });

  it("bấm một chip thì danh sách chỉ còn tệp của ổ đó, và không tìm kiếm lại", async () => {
    const div = await goTimKiem();
    expect(div.textContent).toContain("alpha.mp4");
    expect(div.textContent).toContain("beta.mp4");

    ipc.reset();
    bam(chips(div)[1]); // D:
    await settle(90);

    const sau = div.textContent ?? "";
    expect(sau).toContain("alpha.mp4");
    expect(sau).toContain("gamma.mp4");
    expect(sau).not.toContain("beta.mp4");
    expect(sau).not.toContain("delta.mp4");

    // Điều kiện cốt lõi của bản thiết kế: lọc chạy trên danh sách đã có, KHÔNG
    // gọi tìm kiếm lần nữa. Mất điều này là mất toàn bộ lý do làm ở giao diện.
    expect(ipc.count("search"), "lọc theo ổ không được gọi tìm kiếm lại").toBe(
      0,
    );
  });

  it("bấm lại chip đang chọn thì bỏ lọc", async () => {
    const div = await goTimKiem();
    bam(chips(div)[1]);
    await settle(90);
    expect(div.textContent).not.toContain("beta.mp4");

    bam(chips(div)[1]);
    await settle(90);
    expect(div.textContent).toContain("beta.mp4");
    expect(div.textContent).toContain("alpha.mp4");
  });

  it("không hiện hàng chip khi kết quả chỉ nằm trên một ổ", async () => {
    dungIpc([
      mkHit(1, "D:" + SEP + "m" + SEP + "alpha.mp4"),
      mkHit(2, "D:" + SEP + "m" + SEP + "gamma.mp4"),
    ]);
    const div = await goTimKiem();
    // Một hàng chip có đúng một ổ không nói thêm gì mà lấy mất một dòng của
    // danh sách — thứ đang thực sự cần chỗ.
    expect(
      div.querySelectorAll(".drives").length,
      "hàng chip không được dựng",
    ).toBe(0);
    expect(chips(div).length).toBe(0);
    expect(div.textContent).toContain("alpha.mp4");
  });
});

describe("t10 — số hiệu phiên bản ở chân cửa sổ", () => {
  beforeEach(() => {
    localStorage.clear();
    dungIpc(HITS);
  });

  it("hiện số hiệu bản đang chạy ở góc phải", async () => {
    const div = await goTimKiem();
    const ver = div.querySelector(".status .ver");
    expect(ver, "không thấy số hiệu ở chân cửa sổ").toBeTruthy();
    expect(ver!.textContent!.trim()).toBe("v1.0.4");
  });

  it("hiện ngay cả khi chưa gõ gì — không phải chờ có kết quả", async () => {
    const div = document.createElement("div");
    document.body.appendChild(div);
    const app = mount(App, { target: div });
    dangMo.push(() => {
      unmount(app);
      div.remove();
    });
    await settle(90);
    // Câu hỏi "anh đang dùng bản nào?" thường được hỏi lúc app vừa mở, chưa
    // tìm gì cả. Bắt người dùng gõ một truy vấn mới thấy được số hiệu thì
    // tính năng này hỏng đúng lúc nó cần nhất.
    expect(div.querySelector(".status .ver")?.textContent?.trim()).toBe("v1.0.4");
  });

  it("không hiện gì khi chưa biết số hiệu, thay vì hiện số sai", async () => {
    dungIpc(HITS);
    // `update_status` chưa trả lời (mất mạng, hoặc khoảnh khắc đầu tiên).
    ipc.on("update_status", () => new Promise(() => {}));
    const div = document.createElement("div");
    document.body.appendChild(div);
    const app = mount(App, { target: div });
    dangMo.push(() => {
      unmount(app);
      div.remove();
    });
    await settle(90);
    expect(div.querySelector(".status .ver")).toBeNull();
  });
});

describe("t10 — số hiệu hiện ra phải là số hiệu THẬT của bản build", () => {
  it("ba tệp khai báo phiên bản nói cùng một số", async () => {
    // Ba bài trên dùng "1.0.4" cứng trong mock, nên chúng chỉ chứng minh phần
    // giao diện vẽ đúng cái backend đưa cho — KHÔNG chứng minh cái backend đưa
    // là đúng. Mà đó mới là điều người dùng quan tâm: số hiện ở chân cửa sổ
    // phải là số của bản họ đang chạy.
    //
    // Backend lấy từ `CARGO_PKG_VERSION`, tức `Cargo.toml`. Bộ cài và trình
    // cập nhật lại lấy từ `tauri.conf.json`. Hai tệp đó lệch nhau thì app nói
    // một đằng, bộ cài một nẻo — và không có gì bắt được, vì mỗi bên tự nó
    // đều đúng.
    const doc = async (p: string) => JSON.parse(await readFile(p, "utf8")).version;
    const cargo = (await readFile("src-tauri/Cargo.toml", "utf8"))
      .split(String.fromCharCode(10))
      .find((l) => l.startsWith("version"))!
      .split(String.fromCharCode(34))[1]
      .trim();

    expect(await doc("src-tauri/tauri.conf.json")).toBe(cargo);
    expect(await doc("package.json")).toBe(cargo);
  });
});

describe("t10 — đổi ổ phải dọn cả tập đã chọn", () => {
  beforeEach(() => {
    localStorage.clear();
    dungIpc(HITS);
  });

  it("Ctrl+A rồi đổi ổ thì không còn kéo theo tệp của ổ khác", async () => {
    const div = await goTimKiem();
    // Ctrl+A chỉ tác dụng khi con trỏ KHÔNG ở trong ô tìm kiếm — bấm vào một
    // dòng kết quả trước, đúng như người dùng thật làm.
    (div.querySelector(".row") as HTMLElement).dispatchEvent(
      new MouseEvent("click", { bubbles: true }),
    );
    (div.querySelector("input.search") as HTMLInputElement).blur();
    await settle(60);
    // Chọn hết 4 kết quả (2 trên D:, 2 trên Y:).
    window.dispatchEvent(
      new KeyboardEvent("keydown", { key: "a", ctrlKey: true, bubbles: true }),
    );
    await settle(60);
    expect(div.querySelectorAll(".row.sel").length, "Ctrl+A phải chọn cả 4").toBe(4);

    // Sang ổ D: — danh sách còn 2 dòng.
    bam(chips(div)[1]);
    await settle(90);
    expect(div.querySelectorAll(".row").length).toBe(2);

    // Đây là điều quan trọng: sau khi đổi ổ, số dòng ĐANG CHỌN không được
    // nhiều hơn số dòng đang thấy. Nếu tập chọn còn giữ chỉ số của danh sách
    // cũ thì một cú kéo sẽ mang theo những tệp người dùng không hề nhìn thấy
    // — đúng cái mà chú thích của `targetsFor` nói là phải tránh.
    expect(div.querySelectorAll(".row.sel").length, "đổi ổ phải dọn tập đã chọn").toBe(1);
  });
});

describe("t10 — không nói 'không tìm thấy' khi vẫn đang tìm", () => {
  beforeEach(() => localStorage.clear());

  it("lần tìm cũ về sau không được tắt trạng thái đang-tìm của lần mới", async () => {
    dungIpc(HITS);
    // Lần gọi A chậm, lần gọi B nhanh — A trả lời SAU B, đúng thứ tự gây lỗi.
    const chos: ((v: unknown) => void)[] = [];
    ipc.on("search", (a: { id: number }) => {
      const dap = { id: a.id, hits: HITS, epoch: 3, relaxed: null, elapsedMs: 1, total: 4 };
      // Lần đầu treo lại, các lần sau trả ngay.
      return new Promise((r) => chos.push(() => r(dap)));
    });

    const div = document.createElement("div");
    document.body.appendChild(div);
    const app = mount(App, { target: div });
    dangMo.push(() => {
      unmount(app);
      div.remove();
    });
    await settle(90);

    const o = div.querySelector("input.search") as HTMLInputElement;
    o.value = "a";
    o.dispatchEvent(new window.Event("input", { bubbles: true }));
    await settle(120); // lần A bay đi, bị treo

    o.value = "ab";
    o.dispatchEvent(new window.Event("input", { bubbles: true }));
    await settle(120); // lần B cũng đang bay, chưa về

    // Giờ lần A mới trả lời. Nó ĐÃ BỊ THAY THẾ nên dữ liệu bị bỏ đúng — nhưng
    // `.finally` vẫn chạy và tắt cờ đang-tìm. Nếu B chưa xong, màn hình rơi
    // vào nhánh "Không tìm thấy kết quả nào" trong khi thực tế là CHƯA có
    // kết quả — thông báo khẳng định một điều app chưa hề xác minh.
    expect(chos.length, "phải có hai lần gọi đang bay").toBe(2);

    // A trả lời trước, trong khi B VẪN ĐANG BAY.
    chos[0]!(null);
    await settle(120);

    // Đây là khoảnh khắc quyết định: chưa có kết quả nào, B chưa về. Màn hình
    // không được khẳng định "không tìm thấy" — nó chưa biết điều đó.
    expect(
      div.textContent,
      "nói 'không tìm thấy' trong khi lần tìm mới vẫn đang chạy",
    ).not.toContain("Không tìm thấy kết quả nào");

    // B về, kết quả hiện ra.
    chos[1]!(null);
    await settle(120);
    expect(div.textContent).toContain("alpha.mp4");
  });
});
