<script lang="ts">
  import DupeScopeAsk from "./DupeScopeAsk.svelte";
  import MediaRow from "./MediaRow.svelte";
  import VirtualList from "./VirtualList.svelte";
  import {
    cancelDuplicates,
    dupeEstimate,
    dupeGroups,
    dupeIdleStatus,
    setDupeIdle,
    verifyDupeGroup,
    type VerifyMuc,
    dupeProgress,
    verifyProgress,
    cancelVerify,
    findDuplicates,
    formatBytes,
    formatCount,
    type DupeGroup,
    type DupeProgress,
    type DupeScope,
    type ScopeEstimate,
    type SearchHit,
  } from "./search";

  let {
    epoch,
    rowHeight,
    thumbSize,
    onclose,
    onerror,
    onopen,
    onreveal,
    oncontextmenu,
  }: {
    epoch: number;
    rowHeight: number;
    thumbSize: number;
    /// Đóng hẳn chế độ trùng lặp, về đúng trạng thái ban đầu.
    onclose: () => void;
    onerror: (message: string) => void;
    onopen: (hit: SearchHit) => void;
    onreveal: (hit: SearchHit) => void;
    oncontextmenu: (e: MouseEvent, hit: SearchHit) => void;
  } = $props();

  let dupes = $state<DupeGroup[]>([]);
  let stat = $state<DupeProgress | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;

  /// Các nhóm được trải phẳng thành từng dòng để cùng một bộ ảo hoá vẽ được:
  /// một dòng tiêu đề cho mỗi nhóm, rồi tới các tệp của nó.
  type DupeRow =
    | { head: true; group: DupeGroup; n: number }
    | { head: false; hit: SearchHit; n: number; epoch: number };

  const rows = $derived.by<DupeRow[]>(() => {
    const out: DupeRow[] = [];
    for (const g of dupes) {
      out.push({ head: true, group: g, n: g.files.length });
      // `g.epoch` chứ không phải `epoch` của App: vị trí trong `hit.index`
      // thuộc về chỉ mục lúc quét, nên ảnh thu nhỏ phải hỏi theo epoch đó.
      for (const f of g.files)
        out.push({ head: false, hit: f, n: g.files.length, epoch: g.epoch });
    }
    return out;
  });

  // ---- Con trỏ bàn phím ----
  //
  // Con trỏ đếm theo *tệp*, không theo dòng: dòng tiêu đề nhóm không phải là
  // thứ chọn được, và một con trỏ dừng lại trên "3 bản sao" thì Enter chẳng
  // có nghĩa gì. Danh sách vị trí các dòng tệp được dịch sẵn ở đây để mũi tên
  // chỉ việc cộng trừ.
  const fileRows = $derived(
    rows.map((r, i) => (r.head ? -1 : i)).filter((i) => i >= 0),
  );
  let cursor = $state(0);
  /// Vị trí dòng (trong `rows`) đang được chọn; -1 khi chưa có gì.
  const selRow = $derived(fileRows[cursor] ?? -1);

  let listRef = $state<ReturnType<typeof VirtualList> | undefined>();

  // Con trỏ về đầu khi bắt đầu một LƯỢT QUÉT MỚI, không phải mỗi lần danh
  // sách dài thêm.
  //
  // Trước đây effect này bám vào `rows`, và điều đó đúng khi kết quả chỉ xuất
  // hiện một lần lúc quét xong. Nay backend công bố dần sau mỗi đợt, nên
  // `rows` dài ra mỗi 400 ms — bám vào nó thì con trỏ nhảy về đầu liên tục
  // trong suốt lượt quét, và người đang đọc dở danh sách không giữ được chỗ.
  //
  // So với số nhóm đã thấy chứ không phải nội dung: danh sách chỉ dài THÊM
  // trong một lượt (nhóm đã công bố là chung cuộc), nên con trỏ vẫn trỏ đúng
  // tệp cũ. Chỉ khi danh sách NGẮN ĐI — lượt mới bắt đầu — mới cần đưa về đầu.
  let soNhomTruoc = 0;
  $effect(() => {
    const n = rows.length;
    if (n >= soNhomTruoc) {
      soNhomTruoc = n;
      return;
    }
    soNhomTruoc = n;
    cursor = 0;
  });

  /// Bàn phím sống ở App (một chủ sở hữu duy nhất, cùng lý do với chốt chặn
  /// menu/preview bên đó); component này chỉ đưa ra các thao tác.
  export function move(delta: number) {
    if (!fileRows.length) return;
    cursor = Math.max(0, Math.min(fileRows.length - 1, cursor + delta));
    listRef?.scrollToIndex(fileRows[cursor]);
  }

  /// Enter trên tệp đang chọn: mở tệp, hoặc mở thư mục chứa khi `reveal`.
  export function activate(reveal: boolean) {
    const r = rows[selRow];
    if (r && !r.head) (reveal ? onreveal : onopen)(r.hit);
  }

  // Việc quét bắt đầu khi component gắn vào và dừng khi nó bị gỡ ra. Trước
  // đây đó là hai hàm rời `startDupes`/`exitDupes` mà người gọi phải nhớ ghép
  // đôi cho đúng; buộc vào vòng đời thì không quên được nữa.
  $effect(() => {
    start();
    return () => {
      clearInterval(timer);
      timer = undefined;
      // Rời khỏi màn này thì đĩa được nghỉ. Thiếu dòng này thì lần quét còn
      // đọc thêm vài phút nữa để ra một câu trả lời chẳng ai quay lại xem,
      // trong khi tranh ổ đĩa với việc mà người dùng vừa quay về làm.
      cancelDuplicates().catch(() => {});
      // Cùng lý do, cho tầng 3: một lượt xác minh 45 GB qua NAS không được
      // chạy tiếp sau khi người dùng đã đi chỗ khác.
      dungHoiTienDo();
      hangDoi = [];
      cancelVerify().catch(() => {});
    };
  });

  /// Ước lượng để hỏi; `null` nghĩa là chưa hỏi hoặc không cần hỏi.
  let hoiPhamVi = $state<ScopeEstimate | null>(null);

  /// Quét nền ổ trong máy đang bật không. Đọc một lần lúc dựng component —
  /// giá trị sống trong tiến trình backend, không phải trong localStorage.
  let quetNenBat = $state(true);
  dupeIdleStatus()
    .then(([bat]) => (quetNenBat = bat))
    .catch(() => {});

  /// Kết quả xác minh từng nhóm: khoá nhóm → trạng thái.
  ///
  /// `Map` chứ không phải trường trên `DupeGroup`: nhóm đến từ backend và bị
  /// thay mới mỗi 400 ms trong lúc quét, nên gắn trạng thái vào chúng là mất
  /// ngay ở nhịp sau.
  let xacMinh = $state(
    new Map<string, "dang" | "cho" | "that" | "nhanh" | "khac" | "loi" | "dung">(),
  );

  /// Hàng đợi các nhóm chờ tới lượt.
  ///
  /// Bản đầu từ chối thẳng: bấm nhóm thứ hai khi nhóm thứ nhất đang chạy thì
  /// backend trả lỗi "Đang xác minh một nhóm khác rồi". Người dùng bấm mười
  /// nhóm nhận về chín thông báo lỗi và mất sạch ý định — trong khi điều họ
  /// muốn hoàn toàn hợp lý và chỉ cần xếp hàng.
  ///
  /// Chạy lần lượt chứ không song song vì đo được: ba tệp trên cùng một đĩa cơ
  /// đọc song song chỉ nhanh hơn 1,17× (95 → 111 MB/s), còn hai nhóm cùng chạy
  /// thì tranh đầu đọc và cả hai đều chậm đi.
  let hangDoi = $state<{ g: DupeGroup; muc: VerifyMuc }[]>([]);
  let dangChay = false;

  /// Tiến độ lượt xác minh đang chạy, và nhóm nào đang chạy.
  ///
  /// Một biến chứ không phải một `Map`: backend chỉ cho phép MỘT lượt tại một
  /// thời điểm (hai luồng cùng đọc một ổ chỉ đổi tuần tự lấy tiếng lạch cạch),
  /// nên giữ một `Map` ở đây là hứa một khả năng không tồn tại.
  let tienDo = $state<{ khoa: string; phanTram: number | null } | null>(null);
  let timerXacMinh: ReturnType<typeof setInterval> | undefined;

  /// Khoá ổn định của một nhóm: dung lượng + vị trí tệp đầu.
  ///
  /// Không dùng chỉ số trong danh sách — danh sách dài ra trong lúc quét.
  function khoaNhom(g: DupeGroup): string {
    return `${g.size}:${g.files[0]?.index ?? -1}`;
  }

  /// Dọn nhịp hỏi tiến độ. Gọi ở mọi lối ra, kể cả lối lỗi.
  function dungHoiTienDo() {
    clearInterval(timerXacMinh);
    timerXacMinh = undefined;
    tienDo = null;
  }

  /// Xếp một nhóm vào hàng đợi.
  ///
  /// Bấm lại nhóm đang chờ thì bỏ nó ra — nút vừa là "thêm" vừa là "bỏ", đúng
  /// như người dùng trông đợi khi bấm nhầm.
  function xepHang(g: DupeGroup, muc: VerifyMuc) {
    const k = khoaNhom(g);
    if (xacMinh.get(k) === "cho") {
      hangDoi = hangDoi.filter((x) => khoaNhom(x.g) !== k);
      xacMinh.delete(k);
      xacMinh = new Map(xacMinh);
      return;
    }
    hangDoi = [...hangDoi, { g, muc }];
    xacMinh.set(k, "cho");
    xacMinh = new Map(xacMinh);
    void chayHangDoi();
  }

  /// Rút từng nhóm khỏi hàng đợi và chạy, lần lượt.
  async function chayHangDoi() {
    if (dangChay) return;
    dangChay = true;
    try {
      while (hangDoi.length) {
        const [dau, ...con] = hangDoi;
        hangDoi = con;
        await chayMot(dau.g, dau.muc);
      }
    } finally {
      dangChay = false;
    }
  }

  async function chayMot(g: DupeGroup, muc: VerifyMuc) {
    const k = khoaNhom(g);
    xacMinh.set(k, "dang");
    xacMinh = new Map(xacMinh);
    tienDo = { khoa: k, phanTram: null };

    // Hỏi mỗi 300 ms — cùng bậc với nhịp poll của tầng 2. Đủ mượt để thấy con
    // số nhích, đủ thưa để không tốn gì.
    clearInterval(timerXacMinh);
    timerXacMinh = setInterval(async () => {
      try {
        const p = await verifyProgress();
        if (!p.running) return;
        tienDo = {
          khoa: k,
          // `totalBytes === 0` nghĩa là chưa đo xong tổng. Trả `null` để giao
          // diện nói "đang đọc…" thay vì vẽ 0% — một thanh 0% đứng im trông y
          // hệt một lượt đã treo, tức nói dối theo đúng hướng tệ nhất.
          phanTram:
            p.totalBytes > 0
              ? Math.min(100, Math.floor((p.doneBytes / p.totalBytes) * 100))
              : null,
        };
      } catch {
        // Nuốt: mất một nhịp hỏi tiến độ không đáng để phá cả lượt xác minh.
      }
    }, 300);

    try {
      const kq = await verifyDupeGroup(
        g.files.map((f) => f.path),
        muc,
      );
      // Người dùng bấm Dừng: `groups` mới chỉ là phần đọc kịp, KHÔNG phải câu
      // trả lời. Hiện nó như một kết luận là mời họ xoá tệp chưa ai đọc.
      //
      // Và phân biệt hai mức: mức Nhanh trùng ở 200 điểm kiểm KHÔNG phải là
      // "trùng từng byte". Gộp hai câu đó làm một là nói quá điều đã chứng
      // minh — đúng thứ mà cả tầng 3 sinh ra để chống.
      const trangThai = kq.cancelled
        ? "dung"
        : kq.unreadable.length > 0
          ? "loi"
          : kq.groups.length > 1
            ? "khac"
            : kq.muc === "toanBo"
              ? "that"
              : "nhanh";
      xacMinh.set(k, trangThai);
    } catch (e) {
      onerror(String(e));
      xacMinh.delete(k);
    }
    dungHoiTienDo();
    xacMinh = new Map(xacMinh);
  }

  /// Dừng lượt đang chạy.
  async function dungXacMinh() {
    try {
      await cancelVerify();
    } catch {
      // Không sao: cờ dừng chỉ là lời xin, và lượt sắp kết thúc dù thế nào.
    }
  }

  /// Lượt quét này có bỏ sót tệp nào không.
  function thieuTep(s: DupeProgress): boolean {
    // `?? 0` và `?? []` không phải thừa: một bản backend cũ hơn (hoặc một lượt
    // cập nhật dở dang, khi tệp .exe đã đổi mà cửa sổ chưa nạp lại) trả về
    // `DupeProgress` không có hai trường này. Đọc `.length` của `undefined`
    // làm cả màn hình trắng — và đúng lỗi đó vừa làm bảy bài kiểm thử cũ đỏ.
    return (s.unreadable ?? 0) > 0 || (s.droppedDrives ?? []).length > 0;
  }

  /// Nói rõ thiếu ở đâu, ưu tiên thứ biết chắc.
  ///
  /// Ổ đã rớt thì gọi được TÊN, và đó là câu hữu ích hơn hẳn một con số: người
  /// dùng biết ngay phải nối lại ổ nào.
  function moTaThieu(s: DupeProgress): string {
    const o = s.droppedDrives ?? [];
    if (o.length) {
      return `Thiếu ${o.join(", ")} — ổ không còn kết nối`;
    }
    return `Thiếu ${formatCount(s.unreadable ?? 0)} tệp không đọc được`;
  }

  /// Giờ trong ngày của một mốc Unix: "08:15".
  function gioTrongNgay(unix: number): string {
    const d = new Date(unix * 1000);
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  }

  /// Đọc số giây thành câu người đọc được.
  function docThoiGian(giay: number): string {
    if (giay < 60) return "dưới một phút";
    const phut = Math.round(giay / 60);
    if (phut < 60) return `khoảng ${phut} phút`;
    const gio = Math.floor(phut / 60);
    const du = phut % 60;
    return du ? `khoảng ${gio} giờ ${du} phút` : `khoảng ${gio} giờ`;
  }

  async function start() {
    // Một lần quét đã xong vẫn còn được backend giữ. Chạy lại chỉ vì người
    // dùng quay lại màn này là ném đi mười phút đọc đĩa để tới đúng cái kết
    // quả cũ.
    //
    // Hỏi `completed` chứ không phải `groups > 0`: một thư viện không có gì
    // trùng lặp là một lần quét đã xong mà câu trả lời tình cờ rỗng, và coi
    // đó là "chưa quét bao giờ" thì lần nào ghé qua cũng quét lại từ đầu.
    try {
      stat = await dupeProgress();
      // Đang dừng dở: luồng cũ còn chạy nên `find_duplicates` sẽ bị từ chối.
      // Theo dõi cho tới khi nó dừng hẳn thay vì gọi một lệnh chắc chắn hỏng
      // rồi để màn hình trông như đang quét.
      if (stat.stopping) {
        poll();
        return;
      }
      if (stat.running) {
        // Ai đó rời đi giữa chừng rồi quay lại. Theo dõi lần đang chạy thay
        // vì từ chối để mở lần thứ hai.
        poll();
        return;
      }
      if (stat.completed) {
        dupes = await dupeGroups();
        return;
      }
    } catch {
      // rơi xuống dưới và quét
    }

    // Có ổ mạng thì hỏi trước. Cái giá của việc quét NAS đổ lên chính NAS mà
    // cả studio đang dùng, nên người bấm phải biết mình chọn gì.
    //
    // Không có ổ mạng nào thì không hỏi: một hộp thoại chỉ có một câu trả lời
    // đúng là một hộp thoại thừa.
    try {
      const est = await dupeEstimate();
      if (est.networkFiles > 0) {
        hoiPhamVi = est;
        return;
      }
    } catch {
      // Không ước lượng được thì quét ổ trong máy — mặc định an toàn, không
      // tự ý đọc NAS khi chưa ai đồng ý.
    }
    await chayQuet("localOnly");
  }

  async function chayQuet(scope: DupeScope) {
    hoiPhamVi = null;
    dupes = [];
    try {
      await findDuplicates(scope);
    } catch (e) {
      onerror(String(e));
      return;
    }
    poll();
  }

  function poll() {
    clearInterval(timer);
    // Số nhóm đã lấy về lần gần nhất — chỉ hỏi lại khi backend có thêm.
    let nhomDaLay = -1;
    timer = setInterval(async () => {
      try {
        stat = await dupeProgress();
      } catch {
        return;
      }

      // Lấy kết quả NGAY TRONG LÚC QUÉT, mỗi khi có nhóm mới.
      //
      // Backend xử lý các lớp dung lượng theo tiềm năng thu hồi giảm dần và
      // công bố sau mỗi đợt, nên nhóm hiện ra trước là nhóm đáng giá nhất —
      // và thứ hạng của nó đã chung cuộc, không bị đảo khi quét tiếp.
      //
      // Đo trên thư viện thật: tệp ≥256 MB chỉ chiếm 1,7% số tệp nhưng mang
      // 68% tổng tiềm năng. Người dọn ổ vì thế thấy phần lớn giá trị sau vài
      // giây thay vì chờ hết lượt.
      if (stat.groups !== nhomDaLay) {
        nhomDaLay = stat.groups;
        try {
          dupes = await dupeGroups();
        } catch {
          // Lấy không được thì giữ nguyên cái đang hiện, thử lại nhịp sau.
        }
      }

      if (!stat.running) {
        clearInterval(timer);
        timer = undefined;
        // Lấy lần cuối để chốt — kể cả khi bị dừng giữa chừng. Backend nay
        // GIỮ phần đã chốt thay vì vứt sạch, nên "dừng" không còn đồng nghĩa
        // với "mất hết".
        try {
          dupes = await dupeGroups();
        } catch {
          // giữ nguyên
        }
      }
    }, 400);
  }
</script>

{#if hoiPhamVi}
  <DupeScopeAsk
    est={hoiPhamVi}
    idleOn={quetNenBat}
    ontoggleidle={(v) => {
      quetNenBat = v;
      setDupeIdle(v).catch(() => {});
    }} onchoose={chayQuet} oncancel={() => {
      // Huỷ là thoát hẳn, không phải "đóng hộp thoại rồi ngồi đó": màn hình
      // về đúng trạng thái trước khi bấm, và nút Trùng lặp tắt sáng.
      hoiPhamVi = null;
      onclose();
    }} />
{/if}

<div class="dupebar">
  {#if stat?.stopping}
    <!--
      Trạng thái riêng, không gộp vào "đang quét": người vừa bấm huỷ cần biết
      máy đã nhận lệnh. Trên NAS một lần mở tệp có thể treo hàng chục giây,
      nên khoảng này không phải tức thì.
    -->
    <span>Đang dừng lượt quét…</span>
    <div class="scan-bar"><div class="scan-fill"></div></div>
  {:else if stat?.running}
    <span>
      Đang đối chiếu {formatCount(stat.hashed)}/{formatCount(stat.candidates)} tệp
      cùng dung lượng…
      {#if stat.etaSeconds !== null}
        <!--
          Chỉ hiện khi backend đã đo đủ. Nó im lặng cho tới khi mở xong 200
          tệp, vì tốc độ của vài tệp đầu là nhiễu — cache còn lạnh, luồng còn
          khởi động — và một con số nhảy loạn tệ hơn không hiện gì.
        -->
        <b class="eta">còn {docThoiGian(stat.etaSeconds)}</b>
      {/if}
    </span>
    <div class="scan-bar"><div class="scan-fill"></div></div>
  {:else if dupes.length}
    <!--
      Số nhóm và tổng dung lượng thu hồi được phải nói về cùng một tập. Bản
      đầu ghép số nhóm *lấy về* với phần lãng phí của *tất cả* các nhóm, đọc
      thành "500 nhóm đang tốn của bạn 520 GB" — lệch hơn mười lần.
    -->
    <span>
      <b>{formatCount(stat?.groups ?? dupes.length)}</b> nhóm trùng lặp ·
      có thể thu hồi <b>{formatBytes(stat?.wasted ?? 0)}</b>
      {#if (stat?.groups ?? 0) > dupes.length}
        <span class="dupenote">— đang hiện {formatCount(dupes.length)} nhóm lãng phí nhiều nhất</span>
      {/if}
      <!--
        Mốc quét. Từ khi có quét nền, kết quả có thể nằm sẵn từ 8 giờ sáng
        trong khi người dùng mở màn hình lúc 3 giờ chiều — không nói ra là lặp
        lại lỗi 4.1: hiện một câu trả lời cũ mà không cho biết nó cũ.
      -->
      {#if stat?.startedUnix}
        <span class="dupenote">· kết quả từ {gioTrongNgay(stat.startedUnix)}</span>
      {/if}
      {#if stat && thieuTep(stat)}
        <!-- Có kết quả không có nghĩa là kết quả đầy đủ. -->
        <span class="canhbao">· {moTaThieu(stat)}</span>
      {/if}
    </span>
    <!--
      Nói thẳng ra vì tầng 2 đối chiếu hai đầu tệp, không phải toàn bộ. Cách
      đó đúng để tìm ứng viên và sai nếu lấy làm căn cứ để xoá mà không xem lại.
    -->
    <span class="dupenote">Đối chiếu theo dung lượng và hai đầu tệp — hãy xem lại trước khi xoá</span>
  {:else if stat?.completed}
    <!--
      "Không tìm thấy" chỉ đúng khi đã NHÌN THẤY hết. Với 82% ứng viên nằm
      trên ổ mạng, một lượt quét thiếu tệp không phải chuyện hiếm — và khẳng
      định "không có gì trùng lặp" trong khi chưa đọc được 80% thư viện là
      kiểu nói dối tệ nhất: nó nghe như một câu trả lời dứt khoát.
    -->
    {#if thieuTep(stat)}
      <span class="canhbao">
        {moTaThieu(stat)} — chưa thể nói chắc có tệp trùng lặp hay không.
      </span>
    {:else}
      <span>Không tìm thấy tệp trùng lặp nào.</span>
    {/if}
  {:else}
    <!--
      Không giống với việc không tìm thấy gì: một lần quét bị dừng thì không
      có câu trả lời nào cả, và nói "không tìm thấy" là một khẳng định mà nó
      chưa bao giờ đưa ra.
    -->
    <span>Đã dừng — chưa đối chiếu xong.</span>
  {/if}
</div>

<div class="results">
  {#if rows.length}
    <VirtualList bind:this={listRef} items={rows} itemHeight={rowHeight} columns={1} overscan={4}>
      {#snippet row(r: DupeRow, i: number)}
        {#if r.head}
          <div class="ghead">
            <span class="gcount">{r.n} bản sao</span>
            <span class="gsize">{formatBytes(r.group.size)} mỗi tệp</span>
            <span class="gwaste">thừa {formatBytes(r.group.wasted)}</span>
            <!--
              Nút xác minh cho TỪNG nhóm.

              Tầng 2 chỉ đối chiếu dung lượng và hai đầu tệp, nên hai video
              khác nhau ở giữa vẫn bị gom chung — đúng để tìm ứng viên, sai
              hoàn toàn nếu lấy làm căn cứ xoá. Nút này đọc trọn từng byte,
              nhưng chỉ cho đúng nhóm người dùng sắp hành động: vài giây cho
              một nhóm, thay vì hàng giờ cho cả thư viện.
            -->
            {#if xacMinh.get(khoaNhom(r.group)) === "dang"}
              <!--
                Đang chạy: hiện phần trăm chứ không phải mấy chữ đứng im.

                Đo trên nhóm thật 3 × 16,65 GB, ổ D: là HDD SATA 61 MB/s đọc
                nguội: mức Toàn bộ mất ~14 phút. Suốt quãng ấy một dòng chữ
                đứng im không phân biệt được "cứ chờ" với "treo rồi", và cách
                duy nhất để thử là bỏ đi bấm lại — tức vứt hết phần đã đọc.
              -->
              <span class="gverify dangchay">
                {#if tienDo?.khoa === khoaNhom(r.group) && tienDo.phanTram !== null}
                  <span class="thanh" aria-hidden="true">
                    <span class="day" style="width:{tienDo.phanTram}%"></span>
                  </span>
                  <span class="sopt">{tienDo.phanTram}%</span>
                {:else}
                  <!-- Chưa đo xong tổng. Nói "đang đọc" thay vì vẽ 0% — thanh
                       0% đứng im trông y hệt một lượt đã treo. -->
                  đang đọc…
                {/if}
                <button class="nutdung" onclick={dungXacMinh} title="Dừng lượt đối chiếu này">
                  Dừng
                </button>
              </span>
            {:else if xacMinh.get(khoaNhom(r.group)) === "cho"}
              <!--
                Xếp hàng thay vì báo lỗi.

                Bản đầu từ chối thẳng lượt thứ hai, nên bấm mười nhóm là nhận
                chín thông báo lỗi. Chạy lần lượt chứ không song song vì đo
                được: ba tệp trên cùng đĩa cơ đọc song song chỉ được 1,17×.
              -->
              <span class="gverify cho">
                đang chờ
                <button
                  class="nutdung"
                  onclick={() => xepHang(r.group, "nhanh")}
                  title="Bỏ nhóm này khỏi hàng đợi"
                >
                  Bỏ
                </button>
              </span>
            {:else if xacMinh.get(khoaNhom(r.group)) === "nhanh"}
              <!--
                Mức Nhanh nói ĐÚNG điều nó đã chứng minh, không hơn.

                Nó đọc ~1% rải đều khắp tệp cộng trọn hai đầu. Hai tệp khác
                nhau mà qua lọt cả 200 điểm ấy là chuyện không xảy ra với dữ
                liệu thật — nhưng đó vẫn là xác suất, không phải chứng minh.
                Gọi nó là "trùng từng byte" là nói quá, và nói quá ở đúng chỗ
                người dùng sắp xoá 33,7 GB.
              -->
              <span
                class="gverify ok"
                title="Đã đối chiếu khoảng 1% nội dung, rải đều khắp tệp, cộng trọn phần đầu và phần cuối — mọi điểm kiểm đều khớp. Với dữ liệu thật thì đây gần như chắc chắn là bản sao. Muốn chắc tuyệt đối thì bấm 'Toàn bộ'."
              >
                ✓ khớp mọi điểm kiểm
                <button
                  class="nutdung"
                  onclick={() => xepHang(r.group, "toanBo")}
                  title="Đọc trọn từng byte để chắc chắn tuyệt đối. Chậm hơn nhiều."
                >
                  Toàn bộ
                </button>
              </span>
            {:else if xacMinh.get(khoaNhom(r.group)) === "that"}
              <span class="gverify ok" title="Đã đọc trọn từng byte của mọi tệp trong nhóm và chúng giống hệt nhau. Giữ một bản, xoá phần còn lại là an toàn.">
                ✓ trùng từng byte — an toàn để xoá bớt
              </span>
            {:else if xacMinh.get(khoaNhom(r.group)) === "khac"}
              <span class="gverify canh" title="Các tệp này chỉ giống nhau ở dung lượng và hai đầu, còn nội dung bên trong thì khác. Đừng xoá — chúng là những tệp khác nhau.">
                ⚠ KHÔNG phải bản sao — đừng xoá
              </span>
            {:else if xacMinh.get(khoaNhom(r.group)) === "loi"}
              <span class="gverify canh" title="Có tệp không mở được (đã bị xoá, ổ mạng rớt, hoặc đang bị chương trình khác khoá). Không đọc được không có nghĩa là khác nội dung — chưa thể kết luận.">
                chưa kết luận được — có tệp không đọc nổi
              </span>
            {:else if xacMinh.get(khoaNhom(r.group)) === "dung"}
              <span class="gverify" title="Lượt đối chiếu bị dừng giữa chừng nên chưa có câu trả lời. Bấm để chạy lại.">
                đã dừng —
                <button class="gverify nut" onclick={() => xepHang(r.group, "nhanh")}>
                  đối chiếu lại
                </button>
              </span>
            {:else}
              <!--
                Nhãn nói ra VIỆC nó làm, không chỉ tên nó.

                "Xác minh" một mình không cho người dùng biết vì sao nên bấm,
                mà đây lại đúng là nút đứng giữa họ và việc xoá 33,7 GB.

                Không dùng cụm "trùng thật" trên nút: đó là PHÁN QUYẾT sau khi
                chạy xong, để nó xuất hiện trên cả nút chưa bấm nghĩa là người
                liếc qua thấy cùng một cụm từ ở hai trạng thái trái ngược.
              -->
              <button
                class="gverify nut"
                onclick={() => xepHang(r.group, "nhanh")}
                title="Nhóm này mới được gom theo dung lượng và 64 KB ở hai đầu tệp — đủ chắc để nghi ngờ, chưa đủ chắc để xoá. Bấm để đối chiếu khoảng 1% nội dung rải đều khắp tệp, thường xong trong vài giây. Muốn chắc tuyệt đối thì dùng nút Toàn bộ ở bên."
              >
                Đối chiếu
              </button>
              <button
                class="nutdung"
                onclick={() => xepHang(r.group, "toanBo")}
                title="Đọc trọn từng byte của mọi tệp — chắc chắn tuyệt đối, nhưng mất khoảng một phút cho mỗi 4 GB trên ổ trong máy."
              >
                Toàn bộ
              </button>
            {/if}
          </div>
        {:else}
          <div
            class="row dupe"
            class:sel={i === selRow}
            role="option"
            aria-selected={i === selRow}
            tabindex="-1"
            onclick={() => (cursor = fileRows.indexOf(i))}
            ondblclick={() => onopen(r.hit)}
            oncontextmenu={(e) => oncontextmenu(e, r.hit)}
            onkeydown={() => {}}
          >
            <MediaRow hit={r.hit} epoch={r.epoch ?? epoch} {thumbSize} />
          </div>
        {/if}
      {/snippet}
    </VirtualList>
  {:else}
    <p class="empty">
      {#if stat?.stopping}
        Đang dừng lượt quét…
      {:else if stat?.running}
        Đang đối chiếu…
      {:else if stat?.completed}
        Không có tệp nào trùng lặp — không có gì để thu hồi.
      {:else}
        Chưa có kết quả
      {/if}
    </p>
  {/if}
</div>

<style>
  .dupebar {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 9px 14px 11px;
    font-size: 12.5px;
    color: #cfe0ff;
    background: #1e2836;
    border: 1px solid #2f4260;
    border-radius: 8px;
  }
  .dupebar b { color: #fff; }
  .dupenote { color: var(--text-dim); font-size: 11.5px; }

  /* Không xác định một cách có chủ ý: tổng số bản ghi của một ổ chỉ biết được
     khi quét tới cuối ổ đó, nên mọi con số phần trăm đều là bịa. */
  .eta {
    margin-left: 6px;
    font-weight: 500;
    color: var(--text);
  }

  .scan-bar {
    height: 4px;
    overflow: hidden;
    background: #16202c;
    border-radius: 2px;
  }
  .scan-fill {
    width: 35%;
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    animation: slide 1.3s ease-in-out infinite;
  }
  @keyframes slide {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(340%); }
  }

  .results {
    display: flex;
    flex: 1;
    min-height: 0;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
  }

  .ghead {
    display: flex;
    gap: 10px;
    align-items: center;
    height: 100%;
    padding: 0 12px;
    font-size: 12px;
    color: var(--text-dim);
    background: #20242c;
    border-top: 1px solid var(--border);
  }
  .gcount { color: var(--text); font-weight: 600; }
  /* Trạng thái xác minh, nép bên phải tiêu đề nhóm. */
  .gverify {
    margin-left: auto;
    font-size: 11px;
    white-space: nowrap;
  }
  .gverify.nut {
    padding: 2px 9px;
    font-family: inherit;
    color: var(--text-dim);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    cursor: pointer;
  }
  .gverify.nut:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .gverify.nut:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .gverify.ok {
    color: #47b483;
  }
  /* Cảnh báo dùng màu riêng: "có tệp khác nội dung" là lý do để DỪNG tay, chứ
     không phải một mẩu thông tin ngang hàng với dung lượng. */
  /* Cảnh báo thiếu tệp: cùng màu với cảnh báo xác minh, vì cùng một loại
     thông điệp — "dừng lại, câu trả lời này chưa đầy đủ". */
  .canhbao {
    color: #d4a04a;
  }

  .gverify.canh {
    color: #d4a04a;
  }

  /* --- Lượt xác minh đang chạy --------------------------------------- */

  .gverify.cho {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    opacity: 0.7;
  }

  .gverify.dangchay {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--text-dim);
  }

  /* Thanh tiến độ cố tình nhỏ và không màu mè: nó nằm trên dòng tiêu đề
     nhóm, cạnh dung lượng và số byte thừa, nên nó phải đọc được mà không
     giành lấy sự chú ý khỏi những con số đó. */
  .thanh {
    display: inline-block;
    width: 64px;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .day {
    display: block;
    height: 100%;
    background: var(--accent);
    /* Nhịp hỏi là 300 ms; chuyển tiếp cùng bậc để thanh trôi mượt thay vì
       giật từng nấc. */
    transition: width 300ms linear;
  }
  .sopt {
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }

  .nutdung {
    padding: 1px 7px;
    font-family: inherit;
    font-size: 11px;
    color: var(--text-dim);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    cursor: pointer;
  }
  .nutdung:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .nutdung:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .gwaste { margin-left: auto; color: #ffc978; }

  .row {
    display: flex;
    gap: 11px;
    align-items: center;
    height: 100%;
    padding: 0 12px;
    cursor: default;
  }
  .row:hover { background: #262a33; }
  .row.dupe { padding-left: 24px; }
  /* Cùng màu với dòng chọn bên danh sách tìm kiếm — hai chế độ, một ngôn ngữ. */
  .row.sel { background: #2f3a4f; }

  .empty {
    flex: 1;
    margin: 0;
    padding: 40px;
    text-align: center;
    color: var(--text-dim);
  }
</style>
