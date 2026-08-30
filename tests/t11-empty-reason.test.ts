// Nhóm 11 — màn hình trống nói đúng nguyên nhân.
//
// Trước tính năng này app chỉ có MỘT câu cho bốn tình huống khác hẳn nhau, và
// ngầm đổ lỗi cho người gõ ở cả bốn. Nó đã gây thiệt hại thật: một người tìm
// tệp `.avif` trong lúc chip lọc Video đang bật, không thấy gì, rồi kết luận
// công cụ tìm kiếm kém đi.
//
// Phần dễ sai nhất là THỨ TỰ ƯU TIÊN — đảo nó là để một phỏng đoán che mất
// một sự thật. Phần lớn các ca dưới đây canh đúng chỗ đó.
import { afterEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import App from "../src/App.svelte";
import { agoText, reasonFor, type FilterState } from "../src/lib/emptyReason";
import type { Freshness } from "../src/lib/search";

const GIO = 3600;
const NAY = 1_700_000_000;

function tuoi(gio: number): number {
  return NAY - gio * GIO;
}

function fresh(p: Partial<Freshness> = {}): Freshness {
  return {
    builtAtUnix: NAY,
    local: [{ letter: "D", network: false, fileCount: 100 }],
    network: [],
    unscannedNetwork: [],
    ...p,
  };
}


const dangMo: (() => void)[] = [];
afterEach(() => {
  while (dangMo.length) dangMo.pop()!();
  localStorage.clear();
});

/// Dựng App, gõ một truy vấn KHÔNG cho kết quả nào.
async function mountRong() {
  const ipc = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = ipc;
  ipc
    .on("index_status", {
      loaded: true,
      fileCount: 100,
      dirCount: 5,
      builtAtUnix: Math.floor(Date.now() / 1000),
      problem: null,
    })
    .on("hotkey_status", { combo: "Ctrl+Alt+Space", active: true })
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", { checked: true, available: null, current: "1.0.7" })
    .on("index_freshness", {
      builtAtUnix: Math.floor(Date.now() / 1000),
      local: [{ letter: "D", network: false, fileCount: 100 }],
      network: [],
      unscannedNetwork: [],
    })
    .on("search", (a: { id: number }) => ({
      id: a.id,
      hits: [],
      epoch: 1,
      relaxed: null,
      elapsedMs: 1,
      total: 0,
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

  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  const cleanup = () => {
    unmount(app);
    div.remove();
  };
  dangMo.push(cleanup);
  await settle(90);
  const o = div.querySelector("input.search") as HTMLInputElement;
  o.value = "khong-co-tep-nao-ten-nhu-the-nay";
  o.dispatchEvent(new window.Event("input", { bubbles: true }));
  await settle(300);
  return { div, cleanup };
}

const KHONG_LOC: FilterState = { active: [], countWithout: 0 };

describe("t11 — bộ lọc đang che (ưu tiên cao nhất)", () => {
  it("nói rõ bộ lọc nào và bỏ ra thì còn bao nhiêu", () => {
    const r = reasonFor({ active: ["Video"], countWithout: 3 }, fresh(), NAY);
    expect(r.kind).toBe("filter");
    // "chắc chắn" chứ không phải "có thể": app có sẵn danh sách TRƯỚC khi lọc
    // nên nó đếm được thật, không phải suy đoán.
    expect(r.certainty).toBe("chắc chắn");
    expect(r.title).toContain("3");
    expect(r.detail).toContain("Video");
    expect(r.action?.do).toBe("clear-filters");
  });

  it("gộp nhiều bộ lọc vào một câu", () => {
    const r = reasonFor({ active: ["Video", "ổ D:"], countWithout: 12 }, fresh(), NAY);
    expect(r.detail).toContain("Video");
    expect(r.detail).toContain("ổ D:");
  });

  it("KHÔNG đổ cho bộ lọc khi bỏ lọc ra vẫn chẳng có gì", () => {
    // Đây là ca dễ sai nhất của nhánh này. Chip Video đang bật nhưng bỏ nó ra
    // cũng không có kết quả nào — nói "bộ lọc đang ẩn 0 kết quả" là chỉ sai
    // hướng, và người dùng sẽ tắt chip rồi vẫn không thấy gì.
    const r = reasonFor({ active: ["Video"], countWithout: 0 }, fresh(), NAY);
    expect(r.kind).not.toBe("filter");
  });

  it("bộ lọc thắng cả chỉ mục cũ — vì nó chắc chắn còn kia chỉ là suy đoán", () => {
    const r = reasonFor(
      { active: ["Video"], countWithout: 5 },
      fresh({ builtAtUnix: tuoi(48), network: [{ letter: "Y", network: true, fileCount: 9 }] }),
      NAY,
    );
    expect(r.kind).toBe("filter");
  });
});

describe("t11 — ổ mạng chưa quét lần nào", () => {
  it("tách khỏi 'ổ mạng cũ', vì mọi tệp trên đó đều vô hình", () => {
    const r = reasonFor(KHONG_LOC, fresh({ unscannedNetwork: ["Y", "Z"] }), NAY);
    expect(r.kind).toBe("unscanned-network");
    expect(r.certainty).toBe("chắc chắn");
    expect(r.title).toContain("Y:");
    expect(r.title).toContain("Z:");
    expect(r.action?.do).toBe("scan-network");
  });

  it("thắng nhánh ổ mạng cũ khi cả hai cùng đúng", () => {
    // Chỉ mục cũ VÀ có ổ chưa quét: câu "chưa quét lần nào" nặng hơn hẳn, vì
    // quét lại không giúp gì cho một ổ chưa từng được đụng tới.
    const r = reasonFor(
      KHONG_LOC,
      fresh({
        builtAtUnix: tuoi(50),
        network: [{ letter: "Y", network: true, fileCount: 9 }],
        unscannedNetwork: ["Z"],
      }),
      NAY,
    );
    expect(r.kind).toBe("unscanned-network");
  });
});

describe("t11 — chỉ mục cũ", () => {
  const coMang = { network: [{ letter: "Y", network: true, fileCount: 500 }] };

  it("ổ mạng cũ quá 2 tiếng thì nói ra, kèm thời gian chờ", () => {
    const r = reasonFor(KHONG_LOC, fresh({ builtAtUnix: tuoi(6), ...coMang }), NAY);
    expect(r.kind).toBe("stale-network");
    // "có thể" chứ không phải "chắc chắn": ổ mạng không có nhật ký thay đổi
    // để hỏi, nên app chỉ biết chỉ mục của mình cũ tới đâu — KHÔNG biết trên
    // NAS có gì mới. Khẳng định "tệp của bạn vừa được tải lên" là nói một điều
    // app chưa hề xác minh.
    expect(r.certainty).toBe("có thể");
    // Nút phải ghi rõ giá phải trả, để người dùng tự quyết có đáng chờ không.
    expect(r.action?.label).toMatch(/phút/);
  });

  it("ổ mạng vừa quét xong thì im lặng", () => {
    const r = reasonFor(KHONG_LOC, fresh({ builtAtUnix: tuoi(1), ...coMang }), NAY);
    expect(r.kind).toBe("genuinely-empty");
  });

  it("ổ trong máy cần cũ hơn hẳn mới đáng nói", () => {
    // Ngưỡng của ổ trong máy cao hơn ổ mạng (6 tiếng so với 2), vì tác vụ nền
    // quét nó mỗi ngày và mỗi lần đăng nhập — mốc của nó bình thường luôn
    // mới. Hạ ngưỡng xuống thì câu này hiện suốt ngày và mất hết ý nghĩa.
    expect(reasonFor(KHONG_LOC, fresh({ builtAtUnix: tuoi(3) }), NAY).kind).toBe(
      "genuinely-empty",
    );
    expect(reasonFor(KHONG_LOC, fresh({ builtAtUnix: tuoi(8) }), NAY).kind).toBe("stale-local");
  });

  it("ổ mạng được ưu tiên hơn ổ trong máy khi cả hai cùng cũ", () => {
    // Trong studio, tệp đồng nghiệp vừa đưa lên NAS là trường hợp thường gặp
    // nhất, và ổ mạng có khoảng mù rộng hơn nhiều (12 tiếng một lượt).
    const r = reasonFor(KHONG_LOC, fresh({ builtAtUnix: tuoi(20), ...coMang }), NAY);
    expect(r.kind).toBe("stale-network");
  });
});

describe("t11 — thật sự không có", () => {
  it("nói rõ ĐÃ LOẠI TRỪ những gì, để người dùng tin được", () => {
    const r = reasonFor(KHONG_LOC, fresh(), NAY);
    expect(r.kind).toBe("genuinely-empty");
    // Câu này chỉ đáng tin khi nó cho thấy mình đã kiểm tra: nêu mốc quét và
    // nói rõ không có bộ lọc nào bật. Thiếu phần đó thì nó lại là câu cũ.
    expect(r.detail).toMatch(/\d\d:\d\d/);
    expect(r.detail).toContain("không có bộ lọc");
    expect(r.action).toBeUndefined();
  });

  it("chưa có dữ liệu chỉ mục thì không bịa ra gì", () => {
    // Backend chưa trả lời, hoặc lỗi. Im lặng về nguyên nhân còn hơn đoán bừa.
    expect(reasonFor(KHONG_LOC, null, NAY).kind).toBe("genuinely-empty");
    expect(reasonFor(KHONG_LOC, fresh({ builtAtUnix: 0 }), NAY).kind).toBe("genuinely-empty");
    // Và không được nhắc tới mốc quét mà nó không biết.
    expect(reasonFor(KHONG_LOC, null, NAY).detail).not.toMatch(/\d\d:\d\d/);
  });

  it("đồng hồ chạy lùi không làm nó nói 'quét từ tương lai'", () => {
    // Đổi giờ hệ thống, hoặc NTP kéo đồng hồ lùi: mốc quét thành ra ở tương
    // lai, tuổi thành số âm.
    //
    // Ca này từng là một bài xanh VÔ NGHĨA: nó chỉ kiểm `kind`, mà số âm vẫn
    // nhỏ hơn mọi ngưỡng nên có clamp hay không cũng ra cùng một nhánh. Phép
    // thử bằng cách phá mã đã lộ ra điều đó — bỏ `Math.max(0, …)` mà cả 14 ca
    // vẫn xanh.
    //
    // Đã đào tiếp: `Math.max(0, …)` trong `reasonFor` hôm nay KHÔNG với tới
    // được — tuổi âm luôn nhỏ hơn mọi ngưỡng nên không nhánh nào chạy. Không
    // bài kiểm thử nào bắt được nó, và ca này không giả vờ là bắt được.
    //
    // Cái ca này thật sự canh là `agoText`: nó là hàm công khai, có thể được
    // gọi từ chỗ khác, và số âm phải ra chữ đọc được chứ không phải
    // "-3 tiếng trước".
    expect(agoText(-10_000)).toBe("vừa xong");

    const r = reasonFor(KHONG_LOC, fresh({ builtAtUnix: NAY + 10_000 }), NAY);
    expect(r.kind).toBe("genuinely-empty");
    expect(r.title).not.toContain("-");
    expect(r.detail).not.toContain("-");
  });
});

describe("t11 — chữ đọc thời gian", () => {
  it("đọc được như người nói", () => {
    expect(agoText(30)).toBe("vừa xong");
    expect(agoText(10 * 60)).toBe("10 phút trước");
    expect(agoText(6 * GIO)).toBe("6 tiếng trước");
    expect(agoText(25 * GIO)).toBe("hôm qua");
    expect(agoText(3 * 24 * GIO)).toBe("3 ngày trước");
  });
});

// ---- Bố cục: dựng App thật và soi cây DOM ----
//
// Ba nhóm trên kiểm logic chọn câu, và chúng xanh hết trong khi màn hình thật
// hiển thị dòng chữ dạt hẳn sang lề phải. Một bài kiểm thử về NỘI DUNG không
// bao giờ thấy được lỗi về CHỖ ĐỨNG.

describe("t11 — chỗ đứng của khối báo lý do", () => {
  it("đứng một mình trong hàng, không có ô rỗng nào chiếm chỗ bên cạnh", async () => {
    // Gốc của lỗi lệch: `.results` là hàng flex, và khối `<p class="empty">`
    // mang `flex: 1`. Trước đây nó LUÔN được dựng, nên khi có truy vấn nó
    // thành một ô RỖNG vẫn chiếm hết chiều ngang, đẩy phần báo lý do sang lề
    // phải. Người dùng bắt được ngay trên ảnh chụp màn hình.
    //
    // jsdom không tính bố cục nên không đo được toạ độ pixel. Nhưng bất biến
    // thì kiểm được, và nó chính là bản sửa: trong hàng đó có ĐÚNG MỘT khối,
    // và nó không rỗng.
    const { div } = await mountRong();
    const hang = div.querySelector(".results");
    expect(hang, "không thấy hàng kết quả").toBeTruthy();

    const con = [...hang!.children];
    expect(
      con.length,
      `hàng có ${con.length} khối — ô thừa nào cũng đẩy dòng chữ lệch đi`,
    ).toBe(1);
    expect(con[0].textContent?.trim(), "khối duy nhất lại rỗng").toBeTruthy();
    // Và nó phải là khối báo lý do, không phải khối gợi ý "Gõ để tìm kiếm".
    expect(con[0].textContent).toContain("Không tìm thấy");
  });
});

// ---- Phím tắt: ba trạng thái ----
//
// Trước bản này giao diện chỉ biết hai trạng thái, "có phím" và "không có
// phím". Trạng thái thứ ba — đang chạy bằng phím DỰ PHÒNG — không tồn tại, vì
// backend cũng chỉ thử đúng một tổ hợp rồi bỏ cuộc.

/// Dựng App với một trạng thái phím tắt cho trước, không gõ gì cả.
async function mountVoiPhim(hotkey: {
  combo: string;
  active: boolean;
  fallback: boolean;
  preferred: string;
}): Promise<HTMLElement> {
  const ipc = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = ipc;
  ipc
    .on("index_status", {
      loaded: true,
      fileCount: 100,
      dirCount: 5,
      builtAtUnix: Math.floor(Date.now() / 1000),
      problem: null,
    })
    .on("hotkey_status", hotkey)
    .on("enrich_status", { running: false, done: 1, total: 1 })
    .on("scan_progress", { scanning: false, progress: null })
    .on("network_drives", [])
    .on("update_status", { checked: true, available: null, current: "1.0.7" })
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

  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(App, { target: div });
  dangMo.push(() => {
    unmount(app);
    div.remove();
  });
  await settle(90);
  return div;
}

describe("t11 — gợi ý phím tắt", () => {
  it("giành được tổ hợp ưu tiên: hiện nó, không nói gì về dự phòng", async () => {
    const div = await mountVoiPhim({
      combo: "Ctrl+Alt+Space",
      active: true,
      fallback: false,
      preferred: "Ctrl+Alt+Space",
    });
    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu).toContain("để gọi cửa sổ này");
    expect([...div.querySelectorAll("kbd")].map((k) => k.textContent)).toEqual([
      "Ctrl",
      "Alt",
      "Space",
    ]);
    expect(chu).not.toContain("bị ứng dụng khác chiếm");
  });

  it("đang dùng phím dự phòng: hiện tổ hợp THẬT và nói rõ cái nào bị chiếm", async () => {
    const div = await mountVoiPhim({
      combo: "Ctrl+Alt+F",
      active: true,
      fallback: true,
      preferred: "Ctrl+Alt+Space",
    });
    // Phím hiện ra phải là tổ hợp đang dùng thật. In tổ hợp mong muốn ở đây là
    // mời người dùng bấm một phím không có tác dụng gì.
    expect([...div.querySelectorAll("kbd")].map((k) => k.textContent)).toEqual([
      "Ctrl",
      "Alt",
      "F",
    ]);
    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    // Và phải nói cái đã mất, nếu không người quen phím cũ sẽ tưởng app hỏng.
    expect(chu).toContain("Ctrl+Alt+Space");
    expect(chu).toContain("bị ứng dụng khác chiếm");
  });

  it("không giành được tổ hợp nào: không vẽ phím rỗng, chỉ đường khay hệ thống", async () => {
    const div = await mountVoiPhim({
      combo: "",
      active: false,
      fallback: false,
      preferred: "Ctrl+Alt+Space",
    });
    // Lỗi đã tránh: `combo` rỗng mà vẫn `split("+")` thì vẽ ra một ô phím
    // trống, rồi bảo người dùng bấm nó.
    expect(div.querySelectorAll("kbd").length, "vẽ phím rỗng").toBe(0);

    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu).toContain("khay hệ thống");
    // KHÔNG được khuyên "đóng ứng dụng đang chiếm rồi mở lại" — thứ chiếm phím
    // thường là bộ gõ tiếng Việt hoặc công cụ họ cần chạy suốt ngày.
    expect(chu).not.toContain("mở lại MediaFinder");
  });
});
