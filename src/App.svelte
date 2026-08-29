<script lang="ts">
  import { listen } from "@tauri-apps/api/event";

  import ContextMenu, { type MenuItem } from "./lib/ContextMenu.svelte";
  import DuplicateFinder from "./lib/DuplicateFinder.svelte";
  import FilterPanel, { NO_FILTERS } from "./lib/FilterPanel.svelte";
  import FirstRun from "./lib/FirstRun.svelte";
  import MediaRow from "./lib/MediaRow.svelte";
  import Preview from "./lib/Preview.svelte";
  import ScanStatusBar from "./lib/ScanStatusBar.svelte";
  import DriveChips from "./lib/DriveChips.svelte";
  import SearchBar from "./lib/SearchBar.svelte";
  import UpdateNotice from "./lib/UpdateNotice.svelte";
  import VirtualList from "./lib/VirtualList.svelte";
  import { bucketsFor, driveKey, filterByDrive, networkLetters } from "./lib/drives";
  import { loadPrefs, savePrefs } from "./lib/prefs";
  import { prefetchThumb } from "./lib/thumbQueue";
  import { ScanState } from "./lib/scanState.svelte";
  import {
    coalesce,
    formatCount,
    formatWhen,
    indexStatus,
    openFile,
    enrichStatus,
    hotkeyStatus,
    networkDrives,
    revealInExplorer,
    searchFiles,
    startFileDrag,
    thumbUrl,
    updateStatus,
    type EnrichStatus,
    type Filters,
    type HotkeyStatus,
    type IndexMeta,
    type MediaKind,
    type NetworkDrive,
    type Order,
    type RelaxedInfo,
    type SearchHit,
    type UpdateStatus,
  } from "./lib/search";

  // Các dòng có chiều cao cố định để bộ ảo hoá tìm ra chỉ số bất kỳ bằng phép
  // tính thay vì bằng cách đo; đo nghĩa là phải dàn trang mọi kết quả, mà đó
  // chính là cái giá đang cần tránh.
  const LIST_ROW = 46;
  const GRID_CELL = 168;
  const GRID_MIN_COL = 168;
  const THUMB_LIST = 64;
  const THUMB_GRID = 256;

  // ---- Trạng thái tìm kiếm ----
  //
  // Cả nhóm này ở lại đây, không đẩy ra tệp riêng: `epoch` là số hiệu của bản
  // chỉ mục còn `hit.index` là vị trí *trong* bản đó, nên `thumbUrl(epoch,
  // hit.index)` chỉ đúng khi hai giá trị đến từ cùng một lần tìm. Tách chúng
  // ra là biến một bất biến mà trình biên dịch giữ được thành một quy ước mà
  // người viết phải nhớ.
  // Ba tuỳ chọn bên dưới (activeKinds, order, grid) khởi tạo từ lần dùng
  // trước. Đọc một lần lúc dựng component — localStorage là đồng bộ nên không
  // có khung hình nào hiện giá trị mặc định rồi mới nhảy sang giá trị đã lưu.
  const prefs = loadPrefs();

  let query = $state("");
  /// Kết quả thô từ backend, trước khi lọc theo ổ.
  let allHits = $state<SearchHit[]>([]);
  /// Ổ đang xem; `null` là "Tất cả".
  ///
  /// KHÔNG lưu qua các phiên, khác với lưới/sắp xếp/loại tệp: một bộ lọc vô
  /// hình đang chặn kết quả là màn hình khó hiểu nhất, và mở app ra thấy
  /// "không tìm thấy gì" chỉ vì phiên trước lỡ lọc ổ Z là cái bẫy không đáng
  /// đặt.
  let drive = $state<string | null>(null);

  /// Cái mà toàn bộ giao diện làm việc trên đó — bàn phím, kéo-thả, xem
  /// trước, tải trước ảnh đều nhìn cùng một danh sách này. Lọc ở một chỗ duy
  /// nhất thay vì rắc `filterByDrive` khắp nơi: chỉ cần một chỗ quên là con
  /// trỏ bàn phím trỏ vào một tệp không có trên màn hình.
  const hits = $derived(filterByDrive(allHits, drive));


  // Đổi ổ là đổi danh sách bên dưới con trỏ. Không đưa nó về đầu thì
  // `selected` trỏ vào một vị trí của danh sách cũ — Enter mở nhầm tệp, hoặc
  // mở một tệp không có trên màn hình.
  //
  // So với giá trị trước đó chứ không chạy mỗi lần effect kích: lần chạy đầu
  // xảy ra lúc dựng component, khi chưa có danh sách nào để mà cuộn — và khi
  // `listRef` còn chưa tồn tại.
  let lastDrive: string | null = null;
  $effect(() => {
    const now = drive;
    if (now === lastDrive) return;
    lastDrive = now;
    selected = 0;
    listRef?.scrollToTop();
  });
  let epoch = $state(0);
  let selected = $state(0);
  let elapsedMs = $state(0);
  let searching = $state(false);
  let activeKinds = $state<MediaKind[]>(prefs.activeKinds);
  let relaxed = $state<RelaxedInfo | null>(null);
  let order = $state<Order>(prefs.order);
  let filters = $state<Filters>(NO_FILTERS);
  let filtersActive = $state(false);

  let meta = $state<IndexMeta | null>(null);
  let error = $state<string | null>(null);
  let menu = $state<{ x: number; y: number; hit: SearchHit } | null>(null);
  let grid = $state(prefs.grid);
  let showFilters = $state(false);
  let dupeMode = $state(false);
  let enrich = $state<EnrichStatus | null>(null);
  let enrichTimer: ReturnType<typeof setInterval> | undefined;

  const scan = new ScanState({
    onreload: (m) => {
      meta = m;
      if (query.trim()) runSearch();
    },
    onerror: (m) => (error = m),
  });

  /// Các dòng người dùng đã chọn ra, theo vị trí trong `hits`.
  ///
  /// Giữ bên cạnh `selected` chứ không thay thế nó: `selected` là chỗ bàn phím
  /// đang đứng, còn tập hợp là thứ mà một lệnh sẽ tác động lên. Mọi danh sách
  /// trong Windows đều làm vậy, và gộp hai thứ lại thì Shift+click không còn
  /// cách nào diễn đạt.
  let selection = $state<Set<number>>(new Set([0]));
  /// Chỗ mà một cú Shift+click đo từ đó.
  let anchor = $state(0);

  function selectOnly(i: number) {
    selected = i;
    anchor = i;
    selection = new Set([i]);
  }

  function onRowClick(e: MouseEvent, i: number) {
    if (e.shiftKey) {
      const [lo, hi] = anchor <= i ? [anchor, i] : [i, anchor];
      const next = new Set<number>();
      for (let n = lo; n <= hi; n++) next.add(n);
      selection = next;
      selected = i;
      return;
    }
    if (e.ctrlKey || e.metaKey) {
      const next = new Set(selection);
      // Bỏ chọn cái cuối cùng sẽ để lại không còn gì được chọn và không còn gì
      // để kéo, nên dòng cuối cùng ở lại.
      if (next.has(i) && next.size > 1) next.delete(i);
      else next.add(i);
      selection = next;
      selected = i;
      anchor = i;
      return;
    }
    selectOnly(i);
  }

  /// Những tệp mà một lệnh nên tác động lên: tập đã chọn nếu dòng đó nằm trong
  /// tập, còn không thì chỉ mình dòng đó.
  ///
  /// Kéo một dòng *không* nằm trong tập chọn bắt buộc phải có nghĩa là chỉ
  /// dòng đó — nếu không thì một cú bấm nhầm ở đâu đó sẽ lặng lẽ kéo đi những
  /// tệp mà người dùng còn không nhìn thấy.
  function targetsFor(i: number): string[] {
    const set = selection.has(i) ? [...selection] : [i];
    return set
      .sort((a, b) => a - b)
      .map((n) => hits[n]?.path)
      .filter((p): p is string => !!p);
  }

  /// Lớp xem trước có đang mở hay không.
  ///
  /// Không phải một bản sao của kết quả: nó bám theo `selected`, nên đi qua
  /// các kết quả trong lúc lớp phủ đang mở không thể để hai bên nói khác nhau
  /// về việc tệp nào đang hiện trên màn hình.
  let preview = $state(false);

  function openPreview(i: number) {
    if (!hits[i]) return;
    selectOnly(i);
    preview = true;
  }

  /// Sang kết quả kế tiếp hoặc trước đó mà không rời lớp phủ.
  ///
  /// Cuộn cả danh sách bên dưới nữa, để đóng lớp phủ ra là người dùng nhìn
  /// đúng dòng họ đã dừng lại, chứ không phải chỗ họ bắt đầu.
  function previewStep(delta: number) {
    if (!hits.length) return;
    const next = Math.max(0, Math.min(hits.length - 1, selected + delta));
    selectOnly(next);
    listRef?.scrollToIndex(next);
  }

  let inputEl: HTMLInputElement | undefined = $state();
  let listRef = $state<ReturnType<typeof VirtualList> | undefined>();
  let dupeRef = $state<ReturnType<typeof DuplicateFinder> | undefined>();
  let resultsWidth = $state(0);

  const columns = $derived(
    grid ? Math.max(1, Math.floor(resultsWidth / GRID_MIN_COL)) : 1,
  );
  const rowHeight = $derived(grid ? GRID_CELL : LIST_ROW);

  // ---- Tải trước thumbnail theo hướng cuộn ----
  //
  // VirtualList báo dải chỉ số đang hiện; đợi nó đứng yên một nhịp ngắn rồi
  // xếp hàng tải trước nửa màn kế tiếp theo hướng người dùng đang đi — với ưu
  // tiên thấp, nên không bao giờ tranh chỗ của ô đang nhìn. Cuộn tới nơi thì
  // ảnh đã nằm sẵn trong cache HTTP.
  let viewportTimer: ReturnType<typeof setTimeout> | undefined;
  let lastViewportFrom = 0;
  let prefetchCancels: (() => void)[] = [];

  function onViewport(from: number, to: number) {
    clearTimeout(viewportTimer);
    viewportTimer = setTimeout(() => {
      const down = from >= lastViewportFrom;
      lastViewportFrom = from;
      // Du doan cu cua huong cu khong con gia tri — rut het khoi hang.
      for (const c of prefetchCancels) c();
      prefetchCancels = [];
      const count = Math.max(columns, Math.ceil((to - from) / 2));
      const size = grid ? THUMB_GRID : THUMB_LIST;
      const [a, b] = down
        ? [to, Math.min(hits.length, to + count)]
        : [Math.max(0, from - count), from];
      for (let i = a; i < b; i++) {
        const h = hits[i];
        if (!h) continue;
        prefetchCancels.push(prefetchThumb(thumbUrl(epoch, h.index, size)));
      }
    }, 150);
  }

  indexStatus().then((m) => (meta = m));

  // Hỏi một lần: việc đăng ký xảy ra lúc khởi động và câu trả lời không đổi.
  let hotkey = $state<HotkeyStatus | null>(null);
  hotkeyStatus().then((h) => (hotkey = h));

  // Chỉ mục vừa bị thay ngay dưới chân — một tác vụ theo lịch đã làm mới cache
  // trong lúc cửa sổ này đang mở. Không xử lý thì cửa sổ cứ hiện kết quả của
  // hôm qua, và bản cập nhật không ai thấy thì cũng như không có.
  $effect(() => {
    const stop = listen("index-reloaded", async () => {
      meta = await indexStatus();
      // Số hiệu tệp là vị trí trong chỉ mục, nên mọi kết quả đang hiện giờ đều
      // trỏ vào một tệp khác. Chạy lại là câu trả lời trung thực duy nhất; giữ
      // danh sách cũ là hiện tên đúng bên cạnh đường dẫn sai.
      if (query.trim()) runSearch();
      else allHits = [];
      refreshEnrich();
    });
    return () => {
      stop.then((off) => off());
    };
  });

  // Backend phát sự kiện này mỗi khi nó cố ý đưa cửa sổ ra trước — phím tắt
  // toàn cục, hoặc một lần chạy thứ hai gặp bản đang chạy sẵn. Chọn hết chữ
  // chứ không chỉ lấy con trỏ là theo đúng cách mọi thanh khởi chạy làm: phím
  // tiếp theo bắt đầu một lần tìm mới thay vì nối vào lần cũ.
  $effect(() => {
    const stop = listen("summon", () => {
      inputEl?.focus();
      inputEl?.select();
    });
    return () => {
      stop.then((off) => off());
    };
  });

  // Việc đọc thuộc tính chạy hàng chục phút; hỏi thưa thôi. Nó chỉ đi một
  // chiều, nên nhìn ít hơn cũng không bỏ lỡ gì.
  const refreshEnrich = () =>
    enrichStatus()
      .then((e) => {
        enrich = e;
        if (!e.running && enrichTimer) {
          clearInterval(enrichTimer);
          enrichTimer = undefined;
        }
      })
      .catch(() => {});
  refreshEnrich();
  enrichTimer = setInterval(refreshEnrich, 3000);

  scan.resume().catch(() => {});

  // Cả hai nhịp hẹn giờ đều chết theo cửa sổ. Nhịp đọc thuộc tính tự dừng khi
  // backend báo đã xong, nhưng việc đó có thể mất hàng chục phút — và cho tới
  // lúc ấy nó vẫn hỏi tiếp dù giao diện đã biến mất.
  $effect(() => () => {
    scan.dispose();
    clearInterval(enrichTimer);
    enrichTimer = undefined;
    clearTimeout(viewportTimer);
    for (const c of prefetchCancels) c();
  });

  // Lưu ngay mỗi khi đổi, thay vì lưu lúc đóng: app này thường bị tắt bằng
  // Thoát ở khay hệ thống hoặc theo Windows, và một trình xử lý "trước khi
  // đóng" trong webview không được hứa hẹn sẽ kịp chạy.
  $effect(() => {
    savePrefs({ grid, order, activeKinds });
  });

  // Hỏi một lần lúc khởi động: ánh xạ ổ đĩa hiếm khi đổi giữa chừng, và đây là
  // thứ quyết định nút ổ mạng có tồn tại hay không.
  let netDrives = $state<NetworkDrive[]>([]);
  networkDrives().then((d) => (netDrives = d));

  /// Chữ cái của các ổ mạng đang gắn, suy từ `netDrives` — danh sách mà app
  /// vốn đã hỏi sẵn để dựng nút "+ Ổ mạng". Cần nó để phân biệt ổ mạng ÁNH XẠ
  /// (`Y:`) với đĩa trong máy; thiếu thì nhánh ổ mạng là mã chết.
  const netLetters = $derived(networkLetters(netDrives));
  const driveBuckets = $derived(bucketsFor(allHits, netLetters));
  /// Chỉ đeo nhãn ổ lên từng dòng khi kết quả trải trên nhiều ổ. Một danh
  /// sách toàn `D:` thì cái nhãn không nói thêm điều gì, chỉ lấy mất chỗ của
  /// tên tệp — thứ người dùng thực sự đang đọc.
  const showDrive = $derived(driveBuckets.length > 1);

  // Bản mới, nếu backend đã tìm thấy một bản lúc khởi động. Chỉ đọc kết quả
  // có sẵn — việc hỏi máy chủ xảy ra một lần ở Rust, không phải mỗi lần mở
  // cửa sổ.
  //
  // Ứng dụng thường khởi động ẩn lúc đăng nhập, nên tới lúc cửa sổ mở ra thì
  // câu trả lời gần như luôn có sẵn rồi.
  let update = $state<UpdateStatus | null>(null);
  updateStatus()
    .then((u) => (update = u))
    .catch(() => {});

  /// Hộp thoại cập nhật đang mở. App giữ nó (thay vì component tự giữ) vì bàn
  /// phím toàn cục phải biết mà nhường, và mũi tên mở-lại sống ở footer.
  let updateNoticeOpen = $state(false);
  /// Bản đã tự bật hộp thoại một lần — nhịp hỏi lại mỗi ngày của backend bắn
  /// sự kiện cho CÙNG một bản thì không dựng người ta dậy lần nữa; bản mới
  /// hơn nữa thì có.
  let noticeShownFor: string | null = null;

  $effect(() => {
    const v = update?.available?.version;
    if (v && v !== noticeShownFor) {
      noticeShownFor = v;
      updateNoticeOpen = true;
    }
  });

  // Tin cập nhật có thể về SAU khi cửa sổ đã mở: app khởi động cùng Windows
  // trước khi mạng kịp kết nối, backend giờ thử lại tới khi hỏi được và bắn
  // sự kiện này khi có bản mới — không nghe thì cửa sổ đang mở cứ im lặng
  // tới lần mở kế tiếp.
  $effect(() => {
    const stop = listen("update-available", async () => {
      update = await updateStatus().catch(() => null);
    });
    return () => {
      stop.then((off) => off());
    };
  });

  function startScan(withNetwork: boolean) {
    error = null;
    scan.start(withNetwork);
  }

  /// Giao một kết quả cho bất cứ thứ gì người dùng thả nó vào.
  ///
  /// `preventDefault()` trước tiên, và đó là toàn bộ mẹo: nó huỷ cú kéo mà
  /// WebView sắp tự mình bắt đầu. Hai cú kéo cùng lúc thì con trỏ kẹt lại và
  /// không thả được gì. Thay vào đó là một cú kéo gốc mang định dạng tệp của
  /// chính shell, thứ duy nhất mà CapCut hay một ô tải lên chấp nhận.
  function onDragStart(e: DragEvent, i: number) {
    e.preventDefault();
    const paths = targetsFor(i);
    if (!selection.has(i)) selectOnly(i);
    startFileDrag(paths).catch((err) => (error = String(err)));
  }

  function runSearch() {
    const q = query.trim();
    if (!q) {
      allHits = [];
      relaxed = null;
      elapsedMs = 0;
      return;
    }
    searching = true;
    searchFiles(q, activeKinds, filters, 5000, order)
      .then((res) => {
        // `null` nghĩa là một phím gõ mới hơn đã thay thế lần này rồi.
        if (!res) return;
        // Giữ ổ đang chọn qua lần tìm mới — người dùng thường gõ thêm vài ký
        // tự để thu hẹp, và giật bộ lọc khỏi tay họ giữa chừng là khó chịu.
        // Nhưng nếu ổ đó không còn kết quả nào thì buông: màn hình "không tìm
        // thấy gì" trong khi tệp vẫn nằm ngay đó, chỉ ở ổ khác, là cái bẫy tệ
        // nhất mà một bộ lọc có thể bày ra.
        if (drive && !res.hits.some((h) => driveKey(h.path) === drive)) drive = null;
        allHits = res.hits;
        epoch = res.epoch;
        relaxed = res.relaxed;
        elapsedMs = res.elapsedMs;
        selectOnly(0);
        listRef?.scrollToTop();
      })
      .catch((e) => (error = String(e)))
      .finally(() => (searching = false));
  }

  /// Chạy lại chỉ khi có gì đó để chạy. Các chip gọi vào đây, và ô tìm kiếm
  /// rỗng thì không có câu hỏi nào để hỏi lại.
  function rerun() {
    if (query.trim()) runSearch();
  }

  /// Nhận bộ lọc mới cùng lúc với lệnh chạy lại.
  ///
  /// Hai việc này phải xảy ra trong cùng một lượt: nếu bảng lọc chỉ ghi giá
  /// trị rồi để lần chạy lại tự đọc sau, lần tìm sẽ dùng bộ lọc của lần bấm
  /// trước đó.
  function applyFilters(next: Filters, active: boolean) {
    filters = next;
    filtersActive = active;
    rerun();
  }

  function onInput() {
    coalesce(runSearch);
  }

  async function act(fn: (p: string) => Promise<void>, hit: SearchHit | undefined) {
    if (!hit) return;
    try {
      await fn(hit.path);
    } catch (e) {
      error = String(e);
    }
  }

  const open = (h?: SearchHit) => act(openFile, h);
  const reveal = (h?: SearchHit) => act(revealInExplorer, h);

  function copyPath(hit: SearchHit) {
    navigator.clipboard.writeText(hit.path).catch((e) => (error = String(e)));
  }

  function menuItems(hit: SearchHit): MenuItem[] {
    const items: MenuItem[] = [];
    // Lớp xem trước bám theo `selected` trong `hits`, nên chỉ xem trước được
    // tệp đang nằm trong danh sách tìm kiếm. Kết quả ở chế độ trùng lặp đến
    // từ một danh sách khác — với chúng, `indexOf` trả -1 và mục này từng là
    // một cái nút bấm vào không có gì xảy ra. Một mục không hiện thì người
    // dùng còn hiểu được; một mục bấm mà im lặng thì trông như app hỏng.
    const i = hits.indexOf(hit);
    if (i >= 0) {
      items.push({
        label: "Xem trước",
        icon: "eye",
        shortcut: "Shift+Enter",
        action: () => openPreview(i),
      });
    }
    items.push(
      { label: "Mở tệp", icon: "open", shortcut: "Enter", action: () => open(hit) },
      {
        label: "Mở thư mục chứa tệp",
        icon: "folder",
        shortcut: "Ctrl+Enter",
        action: () => reveal(hit),
      },
      { label: "Sao chép đường dẫn", icon: "copy", action: () => copyPath(hit) },
    );
    return items;
  }

  function onContextMenu(e: MouseEvent, i: number) {
    e.preventDefault();
    selected = i;
    menu = { x: e.clientX, y: e.clientY, hit: hits[i] };
  }

  function move(step: number, extend = false) {
    if (!hits.length) return;
    const next = Math.max(0, Math.min(hits.length - 1, selected + step));
    selected = next;
    if (extend) {
      // Shift + mũi tên mở rộng dải tính từ chỗ neo, giống mọi danh sách khác
      // của Windows — không phải là thêm từng dòng một vào tập đang có.
      const [lo, hi] = anchor <= next ? [anchor, next] : [next, anchor];
      const set = new Set<number>();
      for (let n = lo; n <= hi; n++) set.add(n);
      selection = set;
    } else {
      anchor = next;
      selection = new Set([next]);
    }
    listRef?.scrollToIndex(next);
  }

  /// Home/End (và Ctrl+A) chỉ được cướp khi chúng không có việc gì khác để
  /// làm: con trỏ đang đứng trong ô tìm kiếm thì Home là "về đầu dòng chữ" và
  /// Ctrl+A là "chọn hết chữ" — cướp mất là phá việc sửa truy vấn. Giữ Ctrl
  /// thì luôn là lệnh của danh sách, giống cách Everything phân xử y hệt
  /// xung đột này.
  const wantsListJump = (e: KeyboardEvent) =>
    e.ctrlKey || document.activeElement !== inputEl;

  function onKeydown(e: KeyboardEvent) {
    // Trong lúc trình đơn ngữ cảnh đang mở thì nó sở hữu bàn phím. Cả hai
    // component đều nghe trên `window`, và `stopPropagation` không chặn được
    // một trình nghe anh em trên cùng một đích — thiếu chốt chặn này thì
    // Escape sẽ vừa đóng trình đơn *vừa* xoá ô tìm kiếm trong cùng một phím.
    if (menu || preview || updateNoticeOpen) return;

    // Chế độ trùng lặp có danh sách riêng, nhưng bàn phím vẫn xử lý ở đây —
    // một chủ sở hữu duy nhất, vì hai trình nghe anh em trên `window` không
    // chặn được nhau (đúng bài học của chốt chặn ngay trên). App định tuyến,
    // DuplicateFinder thao tác. Trước đây các phím này rơi thẳng xuống danh
    // sách tìm kiếm đang ẨN: Enter mở một tệp không hề có trên màn hình.
    if (dupeMode) {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          dupeRef?.move(1);
          break;
        case "ArrowUp":
          e.preventDefault();
          dupeRef?.move(-1);
          break;
        case "PageDown":
          e.preventDefault();
          dupeRef?.move(10);
          break;
        case "PageUp":
          e.preventDefault();
          dupeRef?.move(-10);
          break;
        // Không có phân xử wantsListJump như bên tìm kiếm: ở chế độ này danh
        // sách kết quả đang ẩn, nên sửa chữ trong ô tìm kiếm không đổi gì
        // trên màn hình — nhường Home/End cho con trỏ chữ là nhường cho một
        // việc vô hình, trong khi danh sách trùng lặp là thứ duy nhất đang
        // nhìn thấy.
        case "Home":
          e.preventDefault();
          dupeRef?.move(-1_000_000); // move() tự kẹp về đầu
          break;
        case "End":
          e.preventDefault();
          dupeRef?.move(1_000_000);
          break;
        case "Enter":
          e.preventDefault();
          dupeRef?.activate(e.ctrlKey);
          break;
        case "Escape":
          e.preventDefault();
          if (error) error = null;
          else dupeMode = false;
          inputEl?.focus();
          break;
      }
      return;
    }

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        move(columns, e.shiftKey);
        break;
      case "ArrowUp":
        e.preventDefault();
        move(-columns, e.shiftKey);
        break;
      case "ArrowRight":
        if (grid) {
          e.preventDefault();
          move(1);
        }
        break;
      case "ArrowLeft":
        if (grid) {
          e.preventDefault();
          move(-1);
        }
        break;
      case "PageDown":
        e.preventDefault();
        move(columns * 5);
        break;
      case "PageUp":
        e.preventDefault();
        move(-columns * 5);
        break;
      case "Home":
        if (wantsListJump(e)) {
          e.preventDefault();
          move(-selected, e.shiftKey);
        }
        break;
      case "End":
        if (wantsListJump(e)) {
          e.preventDefault();
          move(hits.length - 1 - selected, e.shiftKey);
        }
        break;
      case "a":
      case "A":
        // Chọn hết *kết quả* — nhưng chỉ khi con trỏ không đứng trong ô tìm
        // kiếm (xem wantsListJump). Neo và dòng đang đứng giữ nguyên, để một
        // cú Escape hay mũi tên sau đó vẫn biết mình đang ở đâu.
        if ((e.ctrlKey || e.metaKey) && document.activeElement !== inputEl && hits.length) {
          e.preventDefault();
          selection = new Set(hits.map((_, n) => n));
        }
        break;
      case "Enter":
        e.preventDefault();
        // Enter vẫn có nghĩa là "giao cái này cho Windows". Shift+Enter là cái
        // nhìn ngay trong ứng dụng, nên hành động nhanh và không ràng buộc thì
        // cần một phím bổ trợ, còn hành động rời khỏi ứng dụng thì không đổi
        // nghĩa.
        if (e.shiftKey) openPreview(selected);
        else (e.ctrlKey ? reveal : open)(hits[selected]);
        break;
      case "Escape":
        e.preventDefault();
        if (error) error = null;
        else if (query) {
          query = "";
          allHits = [];
          relaxed = null;
        }
        inputEl?.focus();
        break;
    }
  }

  /// Máy này chưa từng quét — không phải "tìm không ra", mà là "chưa có gì để tìm".
  ///
  /// Hai trạng thái đó trông giống hệt nhau nếu không nói rõ, và một người dùng
  /// mới sẽ gõ, không thấy gì, rồi kết luận phần mềm hỏng. Đây là màn hình duy
  /// nhất họ gặp trước khi phần mềm có ích, nên nó phải nói đủ: cần gì, mất bao
  /// lâu, và sau đó thì sao.
  const needsFirstScan = $derived(
    !!meta && !scan.scanning && (!meta.loaded || meta.fileCount === 0),
  );

  const statusLine = $derived(
    !meta
      ? "Đang tải…"
      : meta.problem
        ? meta.problem
        : `${formatCount(meta.fileCount)} tệp · ${formatCount(meta.dirCount)} thư mục · quét lúc ${formatWhen(meta.builtAtUnix)}`,
  );
</script>

<svelte:window on:keydown={onKeydown} />

<main>
  <SearchBar
    bind:query
    bind:activeKinds
    bind:grid
    bind:showFilters
    bind:order
    bind:inputEl
    {dupeMode}
    {filtersActive}
    {netDrives}
    scanning={scan.scanning}
    oninput={onInput}
    onchange={rerun}
    ontoggledupes={() => (dupeMode = !dupeMode)}
    onscan={startScan}
  />

  {#if showFilters || filtersActive}
    <FilterPanel {enrich} onchange={applyFilters} />
  {/if}

  {#if error}
    <div class="error" role="alert">
      <span>{error}</span>
      <button class="dismiss" onclick={() => (error = null)}>Đóng</button>
    </div>
  {/if}

  {#if scan.scanning}
    <ScanStatusBar scan={scan.progress} scanningNetwork={scan.network} />
  {/if}

  {#if dupeMode}
    <!--
      Chế độ trùng lặp thay hẳn danh sách kết quả: nó trả lời một câu hỏi khác
      với tìm kiếm, và trộn hai thứ lại thì không rõ một dòng thuộc về bên nào.
    -->
    <DuplicateFinder
      bind:this={dupeRef}
      {epoch}
      rowHeight={LIST_ROW}
      thumbSize={THUMB_LIST}
      onerror={(m) => (error = m)}
      onopen={open}
      onreveal={reveal}
      oncontextmenu={(e, hit) => {
        e.preventDefault();
        menu = { x: e.clientX, y: e.clientY, hit };
      }}
    />
  {:else}
    {#if relaxed}
      <div class="partial">
        Không có tệp nào khớp đủ <b>{relaxed.totalTokens}</b> từ.
        Đang hiện các tệp khớp nhiều nhất — <b>{relaxed.bestMatched}/{relaxed.totalTokens}</b> từ.
      </div>
    {/if}

    <DriveChips buckets={driveBuckets} bind:selected={drive} total={allHits.length} />

    <div class="results" class:grid bind:clientWidth={resultsWidth}>
      {#if hits.length}
        <VirtualList
          bind:this={listRef}
          items={hits}
          itemHeight={rowHeight}
          {columns}
          overscan={grid ? 2 : 4}
          onviewport={onViewport}
        >
          {#snippet row(hit: SearchHit, i: number)}
            <div
              class="row"
              class:sel={selection.has(i)}
              class:focused={i === selected}
              role="option"
              aria-selected={selection.has(i)}
              tabindex="-1"
              onclick={(e) => onRowClick(e, i)}
              ondblclick={() => openPreview(i)}
              oncontextmenu={(e) => onContextMenu(e, i)}
              onkeydown={() => {}}
              draggable="true"
              ondragstart={(e) => onDragStart(e, i)}
            >
              <MediaRow
                {hit}
                {epoch}
                {grid}
                thumbSize={grid ? THUMB_GRID : THUMB_LIST}
                totalTokens={relaxed?.totalTokens ?? 0}
                {showDrive}
                {netLetters}
              />
            </div>
          {/snippet}
        </VirtualList>
      {:else if needsFirstScan}
        <FirstRun {netDrives} onscan={startScan} />
      {:else}
        <p class="empty">
          {#if !query.trim()}
            Gõ để tìm kiếm · chuột phải vào kết quả để mở thư mục chứa tệp
            {#if hotkey}
              <br />
              <span class="hint" class:taken={!hotkey.active}>
                {#each hotkey.combo.split("+") as key, i}
                  {#if i > 0}+{/if}<kbd>{key}</kbd>
                {/each}
                {#if hotkey.active}
                  để gọi cửa sổ này từ bất kỳ đâu
                  <br />
                  <span class="quiet">
                    Đóng cửa sổ chỉ ẩn đi để phím tắt còn dùng được — chuột phải
                    biểu tượng ở khay hệ thống rồi chọn <em>Thoát</em> để tắt hẳn
                  </span>
                {:else}
                  đang bị ứng dụng khác chiếm — đóng ứng dụng đó rồi mở lại
                  MediaFinder để dùng được phím tắt
                {/if}
              </span>
            {/if}
          {:else if searching}
            Đang tìm…
          {:else}
            Không tìm thấy kết quả nào
          {/if}
        </p>
      {/if}
    </div>
  {/if}

  <div class="status">
    <span>{statusLine}</span>
    {#if update?.available && !updateNoticeOpen}
      <!-- Lời mời đã bị "Để sau" thu về đây: đứng giữa chân cửa sổ, đủ thấy
           mà không chắn việc, và bấm vào là hộp thoại quay lại. -->
      <button
        class="update-arrow"
        title={`Có bản ${update.available.version} — bấm để cập nhật`}
        aria-label="Mở thông báo cập nhật"
        onclick={() => (updateNoticeOpen = true)}
      >
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path
            d="M8 12.5v-9M4 7l4-3.5L12 7"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    {/if}
    <span class="right">
      {#if hits.length && !dupeMode}
        <span class="timing">{formatCount(hits.length)} kết quả · {elapsedMs.toFixed(1)} ms</span>
      {/if}
      <!--
        Số hiệu bản đang chạy. Nằm ở góc phải cùng, mờ hơn mọi thứ khác: nó
        không phải thứ ai cần đọc khi đang làm việc, nhưng là câu hỏi ĐẦU TIÊN
        của mọi lần báo lỗi — "anh đang dùng bản nào?". Không có nó thì câu trả
        lời phải đi vòng qua Control Panel.

        `update.current` đến từ `CARGO_PKG_VERSION` lúc biên dịch, và lệnh
        `update_status` không gọi mạng — nên số này luôn đúng với bản đang
        chạy, kể cả khi máy mất mạng. Chưa về thì không hiện gì, hơn là hiện
        một số sai.
      -->
      {#if update?.current}
        <span class="ver" title="Phiên bản MediaFinder đang chạy">v{update.current}</span>
      {/if}
    </span>
  </div>
</main>

{#if preview && hits[selected]}
  <Preview
    hit={hits[selected]}
    {epoch}
    position={selected + 1}
    total={hits.length}
    onclose={() => (preview = false)}
    onstep={previewStep}
    onopen={() => open(hits[selected])}
  />
{/if}

{#if update?.available}
  <UpdateNotice {update} bind:open={updateNoticeOpen} />
{/if}

{#if menu}
  <ContextMenu
    x={menu.x}
    y={menu.y}
    items={menuItems(menu.hit)}
    onclose={() => (menu = null)}
  />
{/if}

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 14px;
    gap: 10px;
  }

  /* Màu hổ phách chứ không phải đỏ: đây không phải lỗi, mà là việc tìm kiếm
     nói cho người dùng biết nó đã làm gì. Đỏ sẽ ám chỉ có gì đó hỏng. */
  .partial {
    padding: 9px 14px;
    font-size: 12.5px;
    color: #ffe0b0;
    background: #3d3020;
    border: 1px solid #5c4726;
    border-radius: 8px;
  }
  .partial b { color: #ffc978; }

  .error {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 10px 14px;
    font-size: 13px;
    white-space: pre-line;
    color: #ffd7d7;
    background: #4a2226;
    border: 1px solid #6b3238;
    border-radius: 8px;
  }
  .error span { flex: 1; }
  .dismiss {
    font: inherit;
    font-size: 12px;
    color: inherit;
    background: none;
    border: 1px solid currentColor;
    border-radius: 5px;
    padding: 3px 9px;
    cursor: default;
  }

  .results {
    display: flex;
    flex: 1;
    min-height: 0;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
  }

  /* ---- chế độ danh sách ---- */
  .row {
    display: flex;
    gap: 11px;
    align-items: center;
    height: 100%;
    padding: 0 12px;
    cursor: default;
  }
  .row:hover { background: #262a33; }
  .row.sel { background: #2f3a4f; }
  /* Trong một dải nhiều dòng, "đang chọn" và "con trỏ bàn phím đang ở đâu" là
     hai thứ khác nhau: nền cho cái thứ nhất, viền cho cái thứ hai. Dùng chung
     một dấu hiệu thì Shift+mũi tên trở nên không đọc được. */
  .row.focused {
    outline: 1px solid #4c8dff;
    outline-offset: -1px;
  }

  /* ---- chế độ lưới ---- */
  /* Chỉ còn phần khung của ô lưới ở lại đây; kiểu dáng của những thứ *bên
     trong* một dòng đi theo MediaRow, vì CSS của Svelte bị giới hạn theo từng
     tệp và một quy tắc viết ở đây không với tới các class khai báo ở đó. */
  .results.grid .row {
    position: relative;
    flex-direction: column;
    gap: 6px;
    align-items: stretch;
    padding: 8px;
    border-radius: 8px;
  }

  .hint {
    display: inline-block;
    margin-top: 14px;
    font-size: 12px;
    opacity: 0.75;
  }
  /* Một dòng thứ hai dưới phần gợi ý phím tắt, cố ý nhỏ tiếng hơn: nó trả lời
     một câu hỏi mà người dùng chưa hỏi, và không nên tranh chỗ với câu họ đang
     hỏi. */
  .quiet {
    display: inline-block;
    margin-top: 6px;
    font-size: 12px;
    opacity: 0.65;
  }
  .quiet em {
    font-style: normal;
    opacity: 0.9;
  }

  /* Màu hổ phách chứ không phải đỏ: ứng dụng vẫn chạy, chỉ mất phím tắt. */
  .hint.taken {
    color: #d8a657;
  }

  .hint kbd {
    padding: 2px 6px;
    font: inherit;
    font-size: 11px;
    color: var(--text);
    background: #2c313b;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
  }

  .empty {
    flex: 1;
    margin: 0;
    padding: 40px;
    text-align: center;
    color: var(--text-dim);
  }

  .status {
    position: relative; /* mỏ neo cho mũi tên cập nhật đứng chính giữa */
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
    color: var(--text-dim);
    padding: 0 3px;
  }
  .update-arrow {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: grid;
    place-items: center;
    width: 24px;
    height: 20px;
    padding: 0;
    color: #7fb8ff;
    background: #1e3350;
    border: 1px solid #2d4a6b;
    border-radius: 6px;
    cursor: pointer;
  }
  .update-arrow:hover {
    background: #27436a;
  }
  .timing { flex: 0 0 auto; }
  /* Gom hai mẩu bên phải lại để chúng đứng cạnh nhau ở mép phải, thay vì bị
     `space-between` đẩy ra hai đầu. */
  .right {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex: 0 0 auto;
  }
  /* Mờ hơn phần còn lại: có mặt khi cần tìm, không giành sự chú ý khi không. */
  .ver {
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 11px;
    opacity: 0.5;
  }
</style>
