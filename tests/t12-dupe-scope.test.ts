// Nhóm 12 — hỏi phạm vi trước khi quét trùng lặp, và thời gian còn lại.
//
// Cái giá của việc quét ổ mạng không đổ lên máy người bấm nút — nó đổ lên
// chính NAS mà cả studio đang dùng để làm việc, và 20–40 máy cùng quét là
// 20–40 luồng đọc ngẫu nhiên trên cùng vài ổ đĩa.
//
// Hai điều dễ sai nhất, và cả hai đều hỏng trong im lặng:
//   1. Hỏi cả khi không có ổ mạng → một hộp thoại chỉ có một câu trả lời đúng.
//   2. Không hỏi mà cứ quét cả NAS → người dùng không biết mình vừa trả giá gì.
import { afterEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { IpcRecorder, settle } from "./helpers";
import DuplicateFinder from "../src/lib/DuplicateFinder.svelte";
import type { ScopeEstimate } from "../src/lib/search";

const dangMo: (() => void)[] = [];
afterEach(() => {
  while (dangMo.length) dangMo.pop()!();
});

const KHONG_CHAY = {
  running: false,
  completed: false,
  groups: 0,
  wasted: 0,
  hashed: 0,
  candidates: 0,
  stopping: false,
  startedUnix: 0,
  unreadable: 0,
  droppedDrives: [] as string[],
  etaSeconds: null,
};

/// Dựng màn Trùng lặp với một ước lượng cho trước.
let daDong = 0;
async function moMan(est: ScopeEstimate, tienDo = KHONG_CHAY) {
  daDong = 0;
  const ipc = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = ipc;
  ipc
    .on("dupe_estimate", est)
    .on("dupe_progress", tienDo)
    .on("dupe_groups", [])
    .on("find_duplicates", null)
    .on("cancel_duplicates", null)
    .on("dupe_idle_status", [true, false])
    .on("set_dupe_idle", null)
    .on("thumb_url", "");

  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(DuplicateFinder, {
    target: div,
    props: {
      epoch: 1,
      rowHeight: 46,
      thumbSize: 64,
      onclose: () => (daDong += 1),
      onerror: () => {},
      onopen: () => {},
      onreveal: () => {},
      oncontextmenu: () => {},
    },
  });
  dangMo.push(() => {
    unmount(app);
    div.remove();
  });
  await settle(200);
  return { div, ipc };
}

/// Tham số `scope` của lần gọi `find_duplicates` gần nhất.
function phamViDaGoi(ipc: IpcRecorder): string | undefined {
  const c = ipc.calls.filter((x) => x.cmd === "find_duplicates").pop();
  return (c?.args as { scope?: string } | undefined)?.scope;
}

describe("t12 — hỏi trước khi quét ổ mạng", () => {
  it("có ổ mạng thì hỏi, và CHƯA quét gì cả", async () => {
    const { div, ipc } = await moMan({
      localFiles: 1_200,
      networkFiles: 240_000,
      networkDrives: ["Y:", "Z:"],
    });

    const hop = div.querySelector("[role=dialog]");
    expect(hop, "phải hỏi khi có ổ mạng").toBeTruthy();
    // Điều quan trọng nhất: chưa được đọc một byte nào của NAS trước khi
    // người dùng đồng ý.
    expect(ipc.count("find_duplicates"), "không được tự quét trước khi hỏi").toBe(0);

    const chu = (hop!.textContent ?? "").replace(/\s+/g, " ");
    // Con số phải là con số ĐẾM ĐƯỢC, để người dùng cân nhắc được.
    expect(chu).toContain("240.000");
    expect(chu).toContain("Y:, Z:");
  });

  it("không có ổ mạng thì KHÔNG hỏi, quét thẳng ổ trong máy", async () => {
    // Một hộp thoại chỉ có một câu trả lời đúng là một hộp thoại thừa.
    const { div, ipc } = await moMan({
      localFiles: 900,
      networkFiles: 0,
      networkDrives: [],
    });

    expect(div.querySelector("[role=dialog]"), "không có NAS thì đừng hỏi").toBeNull();
    expect(ipc.count("find_duplicates")).toBe(1);
    expect(phamViDaGoi(ipc)).toBe("localOnly");
  });

  it("chọn 'chỉ ổ trong máy' thì gửi đúng phạm vi đó", async () => {
    const { div, ipc } = await moMan({
      localFiles: 1_200,
      networkFiles: 240_000,
      networkDrives: ["Y:"],
    });

    const nut = [...div.querySelectorAll("[role=dialog] button")].find((b) =>
      b.textContent?.includes("Chỉ ổ trong máy"),
    );
    expect(nut, "thiếu nút 'Chỉ ổ trong máy'").toBeTruthy();
    nut!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await settle(120);

    expect(phamViDaGoi(ipc)).toBe("localOnly");
    expect(div.querySelector("[role=dialog]"), "chọn xong phải đóng hộp thoại").toBeNull();
  });

  it("chọn 'cả ổ mạng' thì gửi đúng phạm vi đó", async () => {
    const { div, ipc } = await moMan({
      localFiles: 1_200,
      networkFiles: 240_000,
      networkDrives: ["Y:"],
    });

    const nut = [...div.querySelectorAll("[role=dialog] button")].find((b) =>
      b.textContent?.includes("Cả ổ mạng"),
    );
    expect(nut, "thiếu nút 'Cả ổ mạng'").toBeTruthy();
    nut!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await settle(120);

    expect(phamViDaGoi(ipc)).toBe("everything");
  });

  it("bấm 'Huỷ' thì không quét gì, VÀ đóng hẳn chế độ trùng lặp", async () => {
    const { div, ipc } = await moMan({
      localFiles: 1_200,
      networkFiles: 240_000,
      networkDrives: ["Y:"],
    });

    const nut = [...div.querySelectorAll("[role=dialog] button")].find(
      (b) => b.textContent?.trim() === "Huỷ",
    );
    expect(nut).toBeTruthy();
    nut!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await settle(120);

    expect(ipc.count("find_duplicates"), "'Huỷ' mà vẫn quét là phản bội lựa chọn").toBe(0);
    expect(div.querySelector("[role=dialog]")).toBeNull();
    // Huỷ là về đúng trạng thái ban đầu — kể cả nút Trùng lặp trên thanh công
    // cụ cũng phải tắt sáng, nên component phải báo ra ngoài.
    expect(daDong, "Huỷ phải đóng hẳn chế độ trùng lặp").toBe(1);
  });

  it("hỏi thất bại thì lùi về ổ trong máy, không tự ý đọc NAS", async () => {
    // Mặc định an toàn: không biết chắc thì đừng đụng vào NAS.
    const ipc = new IpcRecorder();
    (globalThis as { __ipc?: IpcRecorder }).__ipc = ipc;
    ipc
      .on("dupe_estimate", () => {
        throw new Error("không đọc được chỉ mục");
      })
      .on("dupe_progress", KHONG_CHAY)
      .on("dupe_groups", [])
      .on("find_duplicates", null)
      .on("cancel_duplicates", null)
      .on("thumb_url", "");

    const div = document.createElement("div");
    document.body.appendChild(div);
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
    dangMo.push(() => {
      unmount(app);
      div.remove();
    });
    await settle(200);

    expect(phamViDaGoi(ipc)).toBe("localOnly");
  });
});

describe("t12 — thời gian còn lại", () => {
  it("backend chưa đủ dữ liệu thì màn hình im lặng", async () => {
    // `etaSeconds: null` nghĩa là chưa mở đủ 200 tệp. Tốc độ của vài tệp đầu
    // là nhiễu, và một con số nhảy từ "2 phút" lên "40 phút" rồi xuống
    // "5 phút" tệ hơn hẳn là không hiện gì.
    const { div } = await moMan(
      { localFiles: 10, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, running: true, hashed: 12, candidates: 10_000, etaSeconds: null },
    );
    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu).toContain("Đang đối chiếu");
    expect(chu).not.toContain("còn");
  });

  it("có số thì hiện thành câu đọc được", async () => {
    const { div } = await moMan(
      { localFiles: 10, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, running: true, hashed: 500, candidates: 10_000, etaSeconds: 154 },
    );
    expect((div.textContent ?? "").replace(/\s+/g, " ")).toContain("còn khoảng 3 phút");
  });

  it("dưới một phút thì không nói '0 phút'", async () => {
    const { div } = await moMan(
      { localFiles: 10, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, running: true, hashed: 9_900, candidates: 10_000, etaSeconds: 8 },
    );
    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu).toContain("dưới một phút");
    expect(chu).not.toContain("0 phút");
  });

  it("hàng giờ thì đọc theo giờ", async () => {
    const { div } = await moMan(
      { localFiles: 10, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, running: true, hashed: 500, candidates: 900_000, etaSeconds: 3600 + 25 * 60 },
    );
    expect((div.textContent ?? "").replace(/\s+/g, " ")).toContain("1 giờ 25 phút");
  });
});

describe("t12 — bấm lại trong lúc lượt cũ đang dừng", () => {
  it("đang dừng dở thì KHÔNG mở lượt mới, và nói rõ đang dừng", async () => {
    // Lỗi người dùng gặp: bấm Trùng lặp → chọn phạm vi → đổi ý bấm lại để huỷ
    // → bấm lần nữa để quét. `cancel()` ở backend chỉ GIƯƠNG CỜ rồi trả về
    // ngay, luồng vẫn kẹt giữa một lần mở tệp (trên NAS có thể hàng chục
    // giây). Trong khoảng đó `find_duplicates` bị từ chối, nhưng màn hình cũ
    // dựng lại như sắp quét — nên trông như app hỏng.
    const { div, ipc } = await moMan(
      { localFiles: 900, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, running: true, stopping: true, hashed: 50, candidates: 900 },
    );

    expect(ipc.count("find_duplicates"), "đang dừng thì không được mở lượt mới").toBe(0);
    // Ca này từng KHÔNG canh được gì: khi `stopping` thì `running` cũng true,
    // nên nhánh `running` cũ cũng chặn được lượt mới — bỏ hẳn xử lý `stopping`
    // mà bài vẫn xanh. Phép thử bằng cách phá mã đã lộ ra điều đó.
    //
    // Thứ chỉ nhánh `stopping` làm được là NÓI RA rằng đang dừng. Đó mới là
    // phần người dùng thiếu: họ bấm huỷ, rồi bấm quét, và màn hình vẫn nói
    // "đang đối chiếu" như thể lượt cũ còn sống.
    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu, "phải nói rõ đang dừng, không im lặng").toContain("Đang dừng");
    // Và không được nói "đang đối chiếu" — lượt đó đã bị huỷ rồi.
    expect(chu).not.toContain("Đang đối chiếu");
  });

  it("đang chạy bình thường thì theo dõi, không mở lượt thứ hai", async () => {
    // Lỗi người dùng gặp: bấm Trùng lặp → chọn phạm vi → đổi ý bấm lại để
    // huỷ → bấm lần nữa để quét. `cancel()` ở backend chỉ GIƯƠNG CỜ rồi trả
    // về ngay, luồng quét vẫn đang chạy — nên `start()` gặp `running == true`
    // và từ chối, còn màn hình thì đã dựng lại như thể sắp quét.
    //
    // Giao diện phải nhận ra trạng thái "đang dừng" và nói ra, thay vì lặng
    // lẽ không làm gì.
    const { div, ipc } = await moMan(
      { localFiles: 900, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, running: true, hashed: 50, candidates: 900, etaSeconds: null },
    );

    // Lượt cũ đang chạy: mở màn này KHÔNG được bắt đầu lượt thứ hai.
    expect(ipc.count("find_duplicates"), "đang chạy thì không mở lượt mới").toBe(0);
    expect((div.textContent ?? "").replace(/\s+/g, " ")).toContain("Đang đối chiếu");
  });
});

describe("t12 — quét nền ổ trong máy", () => {
  it("hộp thoại có ô bật/tắt, và nói rõ KHÔNG tự đọc ổ mạng", async () => {
    // Người đọc dòng này vừa thấy con số "đọc qua mạng nên chậm hơn nhiều" ở
    // ngay trên, nên phải chặn ngay cách hiểu rằng máy đang âm thầm đọc NAS.
    const { div } = await moMan({
      localFiles: 1_200,
      networkFiles: 240_000,
      networkDrives: ["Y:"],
    });

    const o = div.querySelector("[role=dialog] input[type=checkbox]");
    expect(o, "thiếu ô bật/tắt quét nền").toBeTruthy();

    const chu = (div.querySelector("[role=dialog]")?.textContent ?? "").replace(/\s+/g, " ");
    expect(chu).toContain("ổ trong máy");
    expect(chu, "phải nói rõ không bao giờ tự đọc NAS").toContain("Không bao giờ tự đọc ổ mạng");
  });

  it("bỏ tích thì gửi lệnh tắt xuống backend", async () => {
    const { div, ipc } = await moMan({
      localFiles: 1_200,
      networkFiles: 240_000,
      networkDrives: ["Y:"],
    });

    const o = div.querySelector("[role=dialog] input[type=checkbox]") as HTMLInputElement;
    o.checked = false;
    o.dispatchEvent(new Event("change", { bubbles: true }));
    await settle(90);

    const goi = ipc.calls.filter((c) => c.cmd === "set_dupe_idle").pop();
    expect(goi, "phải gửi lệnh xuống backend").toBeTruthy();
    expect((goi!.args as { enabled: boolean }).enabled).toBe(false);
  });
});

describe("t12 — nói rõ kết quả có từ lúc nào", () => {
  it("hiện mốc quét khi đã có kết quả", async () => {
    // Từ khi có quét nền, kết quả có thể nằm sẵn từ 8 giờ sáng trong khi người
    // dùng mở màn hình lúc 3 giờ chiều. Không nói ra là lặp lại lỗi 4.1: hiện
    // một câu trả lời cũ mà không cho biết nó cũ.
    const ipc = new IpcRecorder();
    (globalThis as { __ipc?: IpcRecorder }).__ipc = ipc;
    // 08:15 hôm nay, theo giờ máy.
    const d = new Date();
    d.setHours(8, 15, 0, 0);
    const moc = Math.floor(d.getTime() / 1000);

    ipc
      .on("dupe_estimate", { localFiles: 10, networkFiles: 0, networkDrives: [] })
      .on("dupe_progress", {
        ...KHONG_CHAY,
        completed: true,
        groups: 3,
        wasted: 5_000_000_000,
        startedUnix: moc,
      })
      .on("dupe_groups", [
        {
          size: 1_000_000_000,
          wasted: 2_000_000_000,
          epoch: 1,
          files: [],
        },
      ])
      .on("find_duplicates", null)
      .on("cancel_duplicates", null)
      .on("dupe_idle_status", [true, true])
      .on("set_dupe_idle", null)
      .on("thumb_url", "");

    const div = document.createElement("div");
    document.body.appendChild(div);
    const app = mount(DuplicateFinder, {
      target: div,
      props: {
        epoch: 1,
        rowHeight: 46,
        thumbSize: 64,
        onclose: () => {},
        onerror: () => {},
        onopen: () => {},
        onreveal: () => {},
        oncontextmenu: () => {},
      },
    });
    dangMo.push(() => {
      unmount(app);
      div.remove();
    });
    await settle(200);

    expect((div.textContent ?? "").replace(/\s+/g, " ")).toContain("kết quả từ 08:15");
  });
});

// ---- Tầng 3: xác minh trước khi xoá ----
//
// Tầng 2 chỉ đối chiếu dung lượng và HAI ĐẦU tệp, nên hai video khác nhau ở
// giữa vẫn bị gom chung. Đúng để tìm ứng viên, sai hoàn toàn nếu lấy làm căn
// cứ xoá — mà xoá chính là việc người dùng làm tiếp theo trên màn hình này.

const NHOM_MAU = {
  size: 1_000_000,
  wasted: 1_000_000,
  epoch: 1,
  files: [
    { index: 1, name: "a.mp4", path: "D:\m\a.mp4", dir: "D:\m", kind: "video" },
    { index: 2, name: "b.mp4", path: "D:\m\b.mp4", dir: "D:\m", kind: "video" },
  ],
};

async function moManCoKetQua(verify: unknown) {
  const ipc = new IpcRecorder();
  (globalThis as { __ipc?: IpcRecorder }).__ipc = ipc;
  ipc
    .on("dupe_estimate", { localFiles: 10, networkFiles: 0, networkDrives: [] })
    .on("dupe_progress", { ...KHONG_CHAY, completed: true, groups: 1, wasted: 1_000_000 })
    .on("dupe_groups", [NHOM_MAU])
    .on("find_duplicates", null)
    .on("cancel_duplicates", null)
    .on("dupe_idle_status", [true, true])
    .on("set_dupe_idle", null)
    .on("verify_dupe_group", verify)
    .on("thumb_url", "");

  const div = document.createElement("div");
  document.body.appendChild(div);
  const app = mount(DuplicateFinder, {
    target: div,
    props: {
      epoch: 1,
      rowHeight: 46,
      thumbSize: 64,
      onclose: () => {},
      onerror: () => {},
      onopen: () => {},
      onreveal: () => {},
      oncontextmenu: () => {},
    },
  });
  dangMo.push(() => {
    unmount(app);
    div.remove();
  });
  await settle(250);
  return { div, ipc };
}

function nutXacMinh(div: HTMLElement): HTMLButtonElement | undefined {
  return [...div.querySelectorAll<HTMLButtonElement>("button")].find(
    (b) => b.textContent?.trim() === "Xác minh",
  );
}

describe("t12 — xác minh trọn nội dung trước khi xoá", () => {
  it("mỗi nhóm có nút Xác minh riêng", async () => {
    const { div } = await moManCoKetQua({ groups: [], unreadable: [] });
    expect(nutXacMinh(div), "thiếu nút Xác minh trên tiêu đề nhóm").toBeTruthy();
  });

  it("một cụm duy nhất = trùng thật", async () => {
    const { div } = await moManCoKetQua({
      groups: [["D:\m\a.mp4", "D:\m\b.mp4"]],
      unreadable: [],
    });
    nutXacMinh(div)!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await settle(150);
    expect((div.textContent ?? "").replace(/\s+/g, " ")).toContain("trùng thật");
  });

  it("hai cụm = tầng 2 đã gom nhầm, phải CẢNH BÁO", async () => {
    // Đây là ca quan trọng nhất của cả tính năng: hai tệp cùng dung lượng,
    // cùng hai đầu, khác nhau ở giữa. Tầng 2 gom chung; nếu màn hình không
    // nói ra thì người dùng xoá mất một tệp không phải bản sao.
    const { div } = await moManCoKetQua({
      groups: [["D:\m\a.mp4"], ["D:\m\b.mp4"]],
      unreadable: [],
    });
    nutXacMinh(div)!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await settle(150);

    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu, "phải cảnh báo có tệp khác nội dung").toContain("khác nội dung");
    expect(chu, "KHÔNG được nói là trùng thật").not.toContain("trùng thật");
  });

  it("không đọc được hết thì KHÔNG khẳng định gì", async () => {
    // Không đọc được không phải là "khác nội dung", cũng không phải "trùng
    // thật". Nói bừa một trong hai đều là khẳng định điều chưa xác minh.
    const { div } = await moManCoKetQua({
      groups: [["D:\m\a.mp4"]],
      unreadable: ["D:\m\b.mp4"],
    });
    nutXacMinh(div)!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await settle(150);

    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu).toContain("không đọc được hết");
    expect(chu).not.toContain("trùng thật");
  });

  it("gửi đúng danh sách đường dẫn của nhóm đó", async () => {
    const { div, ipc } = await moManCoKetQua({ groups: [], unreadable: [] });
    nutXacMinh(div)!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await settle(150);

    const goi = ipc.calls.filter((c) => c.cmd === "verify_dupe_group").pop();
    expect(goi, "phải gọi lệnh xác minh").toBeTruthy();
    expect((goi!.args as { paths: string[] }).paths).toEqual([
      "D:\m\a.mp4",
      "D:\m\b.mp4",
    ]);
  });
});

describe("t12 — ổ rớt thì không nói dối", () => {
  it("thiếu tệp mà không có nhóm nào: KHÔNG nói 'không tìm thấy'", async () => {
    // Đây là ca quan trọng nhất. Với 82% ứng viên nằm trên ổ mạng, một lượt
    // quét thiếu tệp không hiếm — và khẳng định "không có gì trùng lặp" trong
    // khi chưa đọc được 80% thư viện là kiểu nói dối tệ nhất: nó nghe như một
    // câu trả lời dứt khoát.
    const { div } = await moMan(
      { localFiles: 10, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, completed: true, droppedDrives: ["Y:", "Z:"] },
    );
    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu, "phải nói rõ thiếu ổ nào").toContain("Y:, Z:");
    expect(chu).toContain("chưa thể nói chắc");
    expect(chu, "KHÔNG được khẳng định không có tệp trùng lặp").not.toContain(
      "Không tìm thấy tệp trùng lặp nào",
    );
  });

  it("gọi được TÊN ổ đã rớt, không chỉ một con số", async () => {
    // "Thiếu Y: — ổ không còn kết nối" cho người dùng biết phải nối lại ổ nào;
    // "thiếu 160.982 tệp" thì không.
    const { div } = await moMan(
      { localFiles: 10, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, completed: true, unreadable: 1200, droppedDrives: ["Y:"] },
    );
    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu).toContain("Thiếu Y: — ổ không còn kết nối");
  });

  it("không biết ổ nào thì nói số tệp", async () => {
    const { div } = await moMan(
      { localFiles: 10, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, completed: true, unreadable: 42 },
    );
    expect((div.textContent ?? "").replace(/\s+/g, " ")).toContain("Thiếu 42 tệp");
  });

  it("quét đủ thì vẫn nói 'không tìm thấy' như cũ", async () => {
    // Bản sửa không được biến một câu trả lời đúng thành một lời cảnh báo
    // thừa: thư viện không có gì trùng lặp là một câu trả lời thật.
    const { div } = await moMan(
      { localFiles: 10, networkFiles: 0, networkDrives: [] },
      { ...KHONG_CHAY, completed: true },
    );
    const chu = (div.textContent ?? "").replace(/\s+/g, " ");
    expect(chu).toContain("Không tìm thấy tệp trùng lặp nào");
    expect(chu).not.toContain("chưa thể nói chắc");
  });
});
