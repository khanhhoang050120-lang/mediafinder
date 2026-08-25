<script lang="ts">
  import { listen } from "@tauri-apps/api/event";

  import ContextMenu, { type MenuItem } from "./lib/ContextMenu.svelte";
  import Preview from "./lib/Preview.svelte";
  import VirtualList from "./lib/VirtualList.svelte";
  import {
    coalesce,
    formatCount,
    formatWhen,
    indexStatus,
    openFile,
    reloadIndex,
    cancelScan,
    networkDrives,
    requestScan,
    requestScanWithNetwork,
    revealInExplorer,
    scanProgress,
    dupeGroups,
    dupeProgress,
    enrichStatus,
    findDuplicates,
    formatBytes,
    formatDuration,
    formatResolution,
    hotkeyStatus,
    searchFiles,
    startFileDrag,
    thumbUrl,
    type IndexMeta,
    type NetworkDrive,
    type MediaKind,
    type DupeGroup,
    type DupeProgress,
    type EnrichStatus,
    type Filters,
    type Order,
    type HotkeyStatus,
    type RelaxedInfo,
    type ScanProgress,
    type SearchHit,
  } from "./lib/search";

  const KINDS: { key: MediaKind; label: string }[] = [
    { key: "video", label: "Video" },
    { key: "image", label: "Ảnh" },
    { key: "audio", label: "Nhạc" },
  ];

  /// The same words the filter chips use, so the badge and the chip that turns
  /// it on and off never disagree.
  const KIND_LABEL: Record<MediaKind, string> = Object.fromEntries(
    KINDS.map((k) => [k.key, k.label]),
  ) as Record<MediaKind, string>;

  // Named the way people describe what they are looking for, not the way the
  // numbers are stored.
  const RESOLUTIONS: { label: string; minHeight: number }[] = [
    { label: "≥720p", minHeight: 720 },
    { label: "≥1080p", minHeight: 1080 },
    { label: "4K", minHeight: 2160 },
  ];
  const DURATIONS: { label: string; min: number; max: number }[] = [
    { label: "< 1 phút", min: 0, max: 60_000 },
    { label: "1–10 phút", min: 60_000, max: 600_000 },
    { label: "> 10 phút", min: 600_000, max: 0 },
  ];

  // Rows are a fixed height so the virtualiser can find any index by
  // arithmetic instead of measuring; measuring would mean laying out every
  // result, which is the cost being avoided.
  const LIST_ROW = 46;
  const GRID_CELL = 168;
  const GRID_MIN_COL = 168;
  const THUMB_LIST = 64;
  const THUMB_GRID = 256;

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let epoch = $state(0);
  let selected = $state(0);
  let elapsedMs = $state(0);
  let searching = $state(false);
  let activeKinds = $state<MediaKind[]>([]);
  let meta = $state<IndexMeta | null>(null);
  let error = $state<string | null>(null);
  let relaxed = $state<RelaxedInfo | null>(null);
  let menu = $state<{ x: number; y: number; hit: SearchHit } | null>(null);
  let scan = $state<ScanProgress | null>(null);
  let scanning = $state(false);
  let grid = $state(false);
  let showFilters = $state(false);
  let minHeight = $state(0);
  let durationChoice = $state(-1);
  let enrich = $state<EnrichStatus | null>(null);
  let pollTimer: ReturnType<typeof setInterval> | undefined;
  let enrichTimer: ReturnType<typeof setInterval> | undefined;

  // Duplicate mode replaces the result list entirely: it answers a different
  // question from search, and mixing the two would make it unclear which one
  // a row belongs to.
  let dupeMode = $state(false);
  let dupes = $state<DupeGroup[]>([]);
  let dupeStat = $state<DupeProgress | null>(null);
  let dupeTimer: ReturnType<typeof setInterval> | undefined;

  /// Groups flattened into rows so the same virtualiser can draw them: a
  /// header row per group, then its files.
  type DupeRow =
    | { head: true; group: DupeGroup; n: number }
    | { head: false; hit: SearchHit; n: number };

  const dupeRows = $derived.by<DupeRow[]>(() => {
    const out: DupeRow[] = [];
    for (const g of dupes) {
      out.push({ head: true, group: g, n: g.files.length });
      for (const f of g.files) out.push({ head: false, hit: f, n: g.files.length });
    }
    return out;
  });

  const RECENCY: { label: string; days: number }[] = [
    { label: "7 ngày", days: 7 },
    { label: "30 ngày", days: 30 },
    { label: "1 năm", days: 365 },
  ];
  let recencyChoice = $state(-1);

  let order = $state<Order>("relevance");

  const filters = $derived<Filters>({
    minHeight,
    minDurationMs: durationChoice >= 0 ? DURATIONS[durationChoice].min : 0,
    maxDurationMs: durationChoice >= 0 ? DURATIONS[durationChoice].max : 0,
    withinDays: recencyChoice >= 0 ? RECENCY[recencyChoice].days : 0,
  });
  const filtersActive = $derived(
    minHeight > 0 || durationChoice >= 0 || recencyChoice >= 0,
  );

  /// Rows the user has picked out, by position in `hits`.
  ///
  /// Kept beside `selected` rather than replacing it: `selected` is where the
  /// keyboard is, and the set is what a command acts on. Every list in Windows
  /// works that way, and collapsing the two would make Shift-click impossible
  /// to express.
  let selection = $state<Set<number>>(new Set([0]));
  /// Where a Shift-click measures from.
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
      // Toggling off the last one would leave nothing selected and nothing to
      // drag, so the final row stays.
      if (next.has(i) && next.size > 1) next.delete(i);
      else next.add(i);
      selection = next;
      selected = i;
      anchor = i;
      return;
    }
    selectOnly(i);
  }

  /// The files a command should act on: the picked set if the row is part of
  /// it, otherwise just the row itself.
  ///
  /// Dragging a row that is *not* in the selection has to mean that row alone
  /// — otherwise a stray click somewhere else would silently drag files the
  /// user is not even looking at.
  function targetsFor(i: number): string[] {
    const set = selection.has(i) ? [...selection] : [i];
    return set
      .sort((a, b) => a - b)
      .map((n) => hits[n]?.path)
      .filter((p): p is string => !!p);
  }

  /// Whether the preview overlay is up.
  ///
  /// Not a copy of the hit: it follows `selected`, so stepping through results
  /// while the overlay is open cannot leave the two disagreeing about which
  /// file is on screen.
  let preview = $state(false);

  function openPreview(i: number) {
    if (!hits[i]) return;
    selectOnly(i);
    preview = true;
  }

  /// Move to the next or previous result without leaving the overlay.
  ///
  /// Scrolls the list underneath too, so closing the overlay leaves the user
  /// looking at the row they stopped on rather than where they started.
  function previewStep(delta: number) {
    if (!hits.length) return;
    const next = Math.max(0, Math.min(hits.length - 1, selected + delta));
    selectOnly(next);
    listRef?.scrollToIndex(next);
  }

  let inputEl: HTMLInputElement | undefined = $state();
  let listRef = $state<ReturnType<typeof VirtualList> | undefined>();
  let resultsWidth = $state(0);

  const columns = $derived(
    grid ? Math.max(1, Math.floor(resultsWidth / GRID_MIN_COL)) : 1,
  );
  const rowHeight = $derived(grid ? GRID_CELL : LIST_ROW);

  indexStatus().then((m) => (meta = m));

  // Asked once: registration happens at startup and the answer never changes.
  let hotkey = $state<HotkeyStatus | null>(null);
  hotkeyStatus().then((h) => (hotkey = h));

  // The index was replaced underneath us — a scheduled task refreshed the
  // cache while this window was open. Without acting on it the window would go
  // on showing yesterday's results, and the update nobody saw might as well
  // not have happened.
  $effect(() => {
    const stop = listen("index-reloaded", async () => {
      meta = await indexStatus();
      // Entry numbers are positions in the index, so every hit on screen now
      // points at a different file. Re-running is the only honest response;
      // keeping the old list would show right names against wrong paths.
      if (query.trim()) runSearch();
      else hits = [];
      refreshEnrich();
    });
    return () => {
      stop.then((off) => off());
    };
  });

  // The backend fires this whenever it brings the window forward on purpose —
  // the global hotkey, or a second launch reaching the copy already running.
  // Selecting rather than just focusing follows what every launcher does: the
  // next keystroke starts a new search instead of appending to the old one.
  $effect(() => {
    const stop = listen("summon", () => {
      inputEl?.focus();
      inputEl?.select();
    });
    return () => {
      stop.then((off) => off());
    };
  });

  // Enrichment runs for tens of minutes; poll slowly. It only ever moves in
  // one direction, so there is nothing to miss by looking less often.
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
  scanProgress().then((s) => {
    if (s.scanning) startPolling();
  });

  // Asked once at startup: drive mappings rarely change mid-session, and this
  // decides whether the network button exists at all.
  let netDrives = $state<NetworkDrive[]>([]);
  networkDrives().then((d) => (netDrives = d));

  async function startScan(withNetwork = false) {
    error = null;
    try {
      await (withNetwork ? requestScanWithNetwork() : requestScan());
      scanning = true;
      scanningNetwork = withNetwork;
      scan = null;
      startPolling();
    } catch (e) {
      // Declining UAC lands here. The backend already phrases it as an answer
      // rather than a failure, so show it verbatim.
      error = String(e);
    }
  }

  let scanningNetwork = $state(false);

  async function stopScan() {
    await cancelScan();
  }

  /// Hand a result over to whatever the user drops it on.
  ///
  /// `preventDefault()` first, and that is the whole trick: it cancels the
  /// drag the WebView was about to start on its own. Two drags at once leaves
  /// the cursor stuck and nothing dropped. What replaces it is a native drag
  /// carrying the shell's own file format, which is the only thing CapCut or
  /// an upload field will accept.
  function onDragStart(e: DragEvent, i: number) {
    e.preventDefault();
    const paths = targetsFor(i);
    if (!selection.has(i)) selectOnly(i);
    startFileDrag(paths).catch((err) => (error = String(err)));
  }

  function startPolling() {
    scanning = true;
    clearInterval(pollTimer);
    pollTimer = setInterval(async () => {
      let status;
      try {
        status = await scanProgress();
      } catch {
        return; // transient; the next tick picks it up
      }
      scan = status.progress;

      // `finished` is set by the indexer only after the cache is written, so
      // reloading here can never read a half-written file.
      if (status.progress?.finished) {
        stopPolling();
        if (status.progress.error) {
          error = status.progress.error;
        } else {
          try {
            meta = await reloadIndex();
            if (query.trim()) runSearch();
          } catch (e) {
            error = String(e);
          }
        }
        return;
      }

      // The child died without reporting anything — a crash, or Windows
      // refusing to start it. Without this the bar would spin forever.
      if (!status.scanning) {
        stopPolling();
        if (!status.progress?.finished) {
          error = "Tiến trình quét kết thúc bất thường. Dữ liệu cũ vẫn nguyên.";
        }
      }
    }, 250);
  }

  function stopPolling() {
    clearInterval(pollTimer);
    pollTimer = undefined;
    scanning = false;
    scan = null;
  }

  function setResolution(h: number) {
    minHeight = minHeight === h ? 0 : h;
    runSearch();
  }

  function setDuration(i: number) {
    durationChoice = durationChoice === i ? -1 : i;
    runSearch();
  }

  function clearFilters() {
    minHeight = 0;
    durationChoice = -1;
    recencyChoice = -1;
    runSearch();
  }

  async function startDupes() {
    error = null;
    dupeMode = true;

    // A finished scan is still held by the backend. Re-running it because the
    // user came back to this view would throw away ten minutes of disk reading
    // to arrive at the same answer.
    try {
      dupeStat = await dupeProgress();
      if (!dupeStat.running && dupeStat.groups > 0) {
        dupes = await dupeGroups();
        return;
      }
    } catch {
      // fall through and scan
    }

    dupes = [];
    try {
      await findDuplicates();
    } catch (e) {
      error = String(e);
      return;
    }
    clearInterval(dupeTimer);
    dupeTimer = setInterval(async () => {
      try {
        dupeStat = await dupeProgress();
      } catch {
        return;
      }
      if (!dupeStat.running) {
        clearInterval(dupeTimer);
        dupeTimer = undefined;
        dupes = await dupeGroups();
      }
    }, 400);
  }

  function exitDupes() {
    clearInterval(dupeTimer);
    dupeTimer = undefined;
    dupeMode = false;
    dupes = [];
    dupeStat = null;
  }

  function toggleKind(k: MediaKind) {
    activeKinds = activeKinds.includes(k)
      ? activeKinds.filter((x) => x !== k)
      : [...activeKinds, k];
    runSearch();
  }

  function runSearch() {
    const q = query.trim();
    if (!q) {
      hits = [];
      relaxed = null;
      elapsedMs = 0;
      return;
    }
    searching = true;
    searchFiles(q, activeKinds, filters, 5000, order)
      .then((res) => {
        // `null` means a newer keystroke already superseded this one.
        if (!res) return;
        hits = res.hits;
        epoch = res.epoch;
        relaxed = res.relaxed;
        elapsedMs = res.elapsedMs;
        selectOnly(0);
        listRef?.scrollToTop();
      })
      .catch((e) => (error = String(e)))
      .finally(() => (searching = false));
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
    return [
      {
        label: "Xem trước",
        icon: "eye",
        shortcut: "Shift+Enter",
        action: () => openPreview(hits.indexOf(hit)),
      },
      { label: "Mở tệp", icon: "open", shortcut: "Enter", action: () => open(hit) },
      {
        label: "Mở thư mục chứa tệp",
        icon: "folder",
        shortcut: "Ctrl+Enter",
        action: () => reveal(hit),
      },
      { label: "Sao chép đường dẫn", icon: "copy", action: () => copyPath(hit) },
    ];
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


  function onKeydown(e: KeyboardEvent) {
    // While the context menu is up it owns the keyboard. Both components listen
    // on `window`, and `stopPropagation` does not stop a sibling listener on
    // the same target — without this guard Escape would close the menu *and*
    // clear the search box in the same keystroke.
    if (menu || preview) return;

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
      case "Enter":
        e.preventDefault();
        // Enter still means "hand this to Windows". Shift+Enter is the
        // in-app look, so the fast, non-committal action needs a modifier and
        // the one that leaves the app does not change meaning.
        if (e.shiftKey) openPreview(selected);
        else (e.ctrlKey ? reveal : open)(hits[selected]);
        break;
      case "Escape":
        e.preventDefault();
        if (error) error = null;
        else if (query) {
          query = "";
          hits = [];
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
    !!meta && !scanning && (!meta.loaded || meta.fileCount === 0),
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
  <div class="bar">
    <!--
      The a11y guidance targets pages with several inputs, where stealing focus
      is disorienting. This window exists to be typed into: it has one field,
      and anything else would mean reaching for the mouse before every search.
    -->
    <!-- svelte-ignore a11y_autofocus -->
    <input
      bind:this={inputEl}
      bind:value={query}
      oninput={onInput}
      class="search"
      type="text"
      placeholder="Tìm video, ảnh, nhạc…"
      autocomplete="off"
      spellcheck="false"
      autofocus
    />
    <div class="chips">
      {#each KINDS as k (k.key)}
        <button
          class="chip"
          class:on={activeKinds.includes(k.key)}
          onclick={() => toggleKind(k.key)}
        >{k.label}</button>
      {/each}
      <button
        class="chip icon"
        class:on={grid}
        title={grid ? "Chuyển sang danh sách" : "Chuyển sang lưới ảnh"}
        aria-label={grid ? "Chuyển sang danh sách" : "Chuyển sang lưới ảnh"}
        onclick={() => (grid = !grid)}
      >
        {#if grid}
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
            <path d="M2 4h12M2 8h12M2 12h12" stroke-linecap="round" />
          </svg>
        {:else}
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
            <rect x="2" y="2" width="5" height="5" rx="1" />
            <rect x="9" y="2" width="5" height="5" rx="1" />
            <rect x="2" y="9" width="5" height="5" rx="1" />
            <rect x="9" y="9" width="5" height="5" rx="1" />
          </svg>
        {/if}
      </button>
      <button
        class="chip"
        class:on={showFilters || filtersActive}
        onclick={() => (showFilters = !showFilters)}
        title="Lọc theo độ phân giải và thời lượng"
      >Lọc{filtersActive ? " ●" : ""}</button>
      <!--
        Sắp xếp là một nút bật/tắt chứ không phải hộp chọn: chỉ có hai cách sắp
        và cách thứ hai trả lời đúng một câu hỏi — "tôi vừa tải gì về".
      -->
      <button
        class="chip"
        class:on={order === "newest"}
        onclick={() => {
          order = order === "newest" ? "relevance" : "newest";
          if (query.trim()) runSearch();
        }}
        title={order === "newest"
          ? "Đang xếp theo thời gian sửa đổi — bấm để quay về xếp theo mức độ khớp"
          : "Xếp mới nhất lên đầu"}
      >{order === "newest" ? "Mới nhất" : "Liên quan"}</button>
      <button
        class="chip"
        class:on={dupeMode}
        onclick={() => (dupeMode ? exitDupes() : startDupes())}
        title="Tìm các tệp trùng lặp trong thư viện"
      >Trùng lặp</button>
      <button
        class="chip rescan"
        onclick={() => startScan(false)}
        disabled={scanning}
        title="Quét lại các ổ gắn trong máy — vài giây"
      >
        {scanning ? "Đang quét…" : "Quét lại"}
      </button>
      {#if netDrives.length > 0}
        <button
          class="chip rescan"
          onclick={() => startScan(true)}
          disabled={scanning}
          title={`Quét ổ trong máy VÀ ${netDrives.length} ổ mạng (${netDrives
            .map((d) => d.letter + ":")
            .join(", ")}) — mất vài phút vì phải duyệt qua mạng`}
        >
          + ổ mạng
        </button>
      {/if}
    </div>
  </div>

  {#if showFilters || filtersActive}
    <div class="filters">
      <span class="flabel">Độ phân giải</span>
      {#each RESOLUTIONS as r (r.minHeight)}
        <button
          class="chip small"
          class:on={minHeight === r.minHeight}
          onclick={() => setResolution(r.minHeight)}
        >{r.label}</button>
      {/each}

      <span class="fsep"></span>
      <span class="flabel">Thời lượng</span>
      {#each DURATIONS as d, i (d.label)}
        <button
          class="chip small"
          class:on={durationChoice === i}
          onclick={() => setDuration(i)}
        >{d.label}</button>
      {/each}

      <span class="fsep"></span>
      <span class="flabel">Sửa đổi trong</span>
      {#each RECENCY as r, i (r.days)}
        <button
          class="chip small"
          class:on={recencyChoice === i}
          onclick={() => {
            recencyChoice = recencyChoice === i ? -1 : i;
            if (query.trim()) runSearch();
          }}
        >{r.label}</button>
      {/each}

      {#if filtersActive}
        <button class="chip small clear" onclick={clearFilters}>Bỏ lọc</button>
      {/if}

      {#if enrich && enrich.total > 0 && enrich.done < enrich.total}
        <!--
          Said plainly rather than hidden. A resolution filter can only match a
          file somebody has looked at, so a short result list here is a fact
          about progress, not about the library — and the user has no way to
          know that unless it is written down.
        -->
        <span class="fnote" class:working={enrich.running}>
          Đã đọc thuộc tính {formatCount(enrich.done)}/{formatCount(enrich.total)} tệp
          {enrich.running ? "· đang tiếp tục" : "· tạm dừng"}
        </span>
      {/if}
    </div>
  {/if}

  {#if error}
    <div class="error" role="alert">
      <span>{error}</span>
      <button class="dismiss" onclick={() => (error = null)}>Đóng</button>
    </div>
  {/if}

  {#if scanning}
    <div class="scan">
      <div class="scan-head">
        <span>{scan?.message ?? "Đang khởi động tiến trình quét…"}</span>
        {#if scan && scan.volumesTotal > 0}
          <span class="scan-count">ổ {scan.volumesDone + 1}/{scan.volumesTotal}</span>
        {/if}
        <!-- Offered only during the network phase, because only that phase can
             honour it: the local scan runs in a separate elevated process this
             one has no handle on. A stop button that does nothing is worse
             than no stop button. -->
        {#if scanningNetwork && scan?.phase === "network"}
          <button class="stop" onclick={stopScan}>Dừng</button>
        {/if}
      </div>
      <!-- Indeterminate on purpose: a volume's total record count is not known
           until the scan reaches the end of it, so any percentage would be
           invented. -->
      <div class="scan-bar"><div class="scan-fill"></div></div>
    </div>
  {/if}

  {#if relaxed}
    <div class="partial">
      Không có tệp nào khớp đủ <b>{relaxed.totalTokens}</b> từ.
      Đang hiện các tệp khớp nhiều nhất — <b>{relaxed.bestMatched}/{relaxed.totalTokens}</b> từ.
    </div>
  {/if}

  {#if dupeMode}
    <div class="dupebar">
      {#if dupeStat?.running}
        <span>
          Đang đối chiếu {formatCount(dupeStat.hashed)}/{formatCount(dupeStat.candidates)} tệp
          cùng dung lượng…
        </span>
        <div class="scan-bar"><div class="scan-fill"></div></div>
      {:else if dupes.length}
        <!--
          The group count and the reclaimable total must describe the same set.
          The first version paired the number of groups *fetched* with the
          waste across *all* of them, which read as "500 groups are costing you
          520 GB" — off by more than a factor of ten.
        -->
        <span>
          <b>{formatCount(dupeStat?.groups ?? dupes.length)}</b> nhóm trùng lặp ·
          có thể thu hồi <b>{formatBytes(dupeStat?.wasted ?? 0)}</b>
          {#if (dupeStat?.groups ?? 0) > dupes.length}
            <span class="dupenote">— đang hiện {formatCount(dupes.length)} nhóm lãng phí nhiều nhất</span>
          {/if}
        </span>
        <!--
          Said out loud because tier 2 compares the two ends of a file, not all
          of it. That is right for finding candidates and wrong as a basis for
          deleting something without looking.
        -->
        <span class="dupenote">Đối chiếu theo dung lượng và hai đầu tệp — hãy xem lại trước khi xoá</span>
      {:else}
        <span>Không tìm thấy tệp trùng lặp nào.</span>
      {/if}
    </div>
  {/if}

  <div class="results" class:grid={grid && !dupeMode} bind:clientWidth={resultsWidth}>
    {#if dupeMode}
      {#if dupeRows.length}
        <VirtualList items={dupeRows} itemHeight={LIST_ROW} columns={1} overscan={4}>
          {#snippet row(r: DupeRow)}
            {#if r.head}
              <div class="ghead">
                <span class="gcount">{r.n} bản sao</span>
                <span class="gsize">{formatBytes(r.group.size)} mỗi tệp</span>
                <span class="gwaste">thừa {formatBytes(r.group.wasted)}</span>
              </div>
            {:else}
              <div
                class="row dupe"
                role="option"
                aria-selected="false"
                tabindex="-1"
                ondblclick={() => open(r.hit)}
                oncontextmenu={(e) => {
                  e.preventDefault();
                  menu = { x: e.clientX, y: e.clientY, hit: r.hit };
                }}
                onkeydown={() => {}}
              >
                <img
                  class="thumb"
                  src={thumbUrl(epoch, r.hit.index, THUMB_LIST)}
                  alt=""
                  loading="lazy"
                  decoding="async"
                  onerror={(e) => (e.currentTarget as HTMLImageElement).classList.add("failed")}
                />
                <span class="text">
                  <span class="name">{r.hit.name}</span>
                  <span class="dir">{r.hit.dir}</span>
                </span>
              </div>
            {/if}
          {/snippet}
        </VirtualList>
      {:else}
        <p class="empty">
          {dupeStat?.running ? "Đang đối chiếu…" : "Chưa có kết quả"}
        </p>
      {/if}
    {:else if hits.length}
      <VirtualList
        bind:this={listRef}
        items={hits}
        itemHeight={rowHeight}
        {columns}
        overscan={grid ? 2 : 4}
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
            <!--
              `loading="lazy"` matters even here: the virtualiser keeps a few
              overscan rows in the DOM that are not on screen, and without it
              every one of them would trigger a decode.
            -->
            <img
              class="thumb"
              src={thumbUrl(epoch, hit.index, grid ? THUMB_GRID : THUMB_LIST)}
              alt=""
              loading="lazy"
              decoding="async"
              onerror={(e) => (e.currentTarget as HTMLImageElement).classList.add("failed")}
            />
            <!--
              An icon, not a letter. The first cut used the English initial —
              V/I/A — in a Vietnamese interface whose own filter chips say
              "Video / Ảnh / Nhạc". `I` for Ảnh and `A` for Nhạc meant nothing
              to the person reading them, and the first thing the user asked
              was what the letters stood for. A shape needs no translation.
            -->
            <span class="kind {hit.kind}" title={KIND_LABEL[hit.kind]} aria-label={KIND_LABEL[hit.kind]}>
              {#if hit.kind === "video"}
                <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                  <path d="M6.2 4.6v6.8a.5.5 0 0 0 .77.42l5.2-3.4a.5.5 0 0 0 0-.84l-5.2-3.4a.5.5 0 0 0-.77.42Z" />
                </svg>
              {:else if hit.kind === "image"}
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                  <rect x="2.6" y="3.4" width="10.8" height="9.2" rx="1.4" />
                  <circle cx="6" cy="6.6" r="1.05" fill="currentColor" stroke="none" />
                  <path d="M3.2 11.4 6.4 8.6l2.2 1.9 2.1-2 2.1 2.1" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              {:else}
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                  <path d="M6.1 11.4V4.3l6-1.3v7" stroke-linecap="round" stroke-linejoin="round" />
                  <circle cx="4.5" cy="11.6" r="1.7" fill="currentColor" stroke="none" />
                  <circle cx="10.5" cy="10.2" r="1.7" fill="currentColor" stroke="none" />
                </svg>
              {/if}
            </span>
            <span class="text">
              <span class="name">{hit.name}</span>
              <span class="dir">{hit.dir}</span>
            </span>
            <span class="facts">
              {#if formatResolution(hit.width, hit.height)}
                <span class="fact res">{formatResolution(hit.width, hit.height)}</span>
              {/if}
              {#if hit.durationMs}
                <span class="fact">{formatDuration(hit.durationMs)}</span>
              {/if}
              {#if hit.size}
                <span class="fact dim">{formatBytes(hit.size)}</span>
              {/if}
            </span>
            {#if relaxed}
              <span class="matched" title="Số từ khớp trên tổng số từ trong truy vấn">
                {hit.matched}/{relaxed.totalTokens}
              </span>
            {/if}
          </div>
        {/snippet}
      </VirtualList>
    {:else}
      {#if needsFirstScan}
        <div class="firstrun">
          <div class="big">Chưa có chỉ mục — cần quét ổ đĩa một lần</div>
          <p>
            MediaFinder đọc thẳng bảng tệp của NTFS nên tìm ra kết quả trong vài
            mili giây. Đổi lại, nó phải đọc bảng đó một lần trước đã.
          </p>
          <ul>
            <li>
              Windows sẽ hỏi quyền Administrator <strong>đúng một lần</strong> —
              đọc bảng tệp bắt buộc phải có quyền đó.
            </li>
            <li>
              Lần quét đầu mất khoảng <strong>nửa phút tới vài phút</strong>, tuỳ
              số tệp trong máy.
            </li>
            <li>
              Sau lần này ứng dụng tự cập nhật lúc đăng nhập và mỗi ngày một lần,
              và <strong>không hỏi quyền lần nào nữa</strong>.
            </li>
            <li>
              Ứng dụng chạy sẵn cùng Windows ở chế độ ẩn để phím tắt
              <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Space</kbd> luôn gọi được — tắt
              phần này lúc nào cũng được, hướng dẫn nằm trong tệp đọc thêm.
            </li>
          </ul>
          <div class="actions">
            <button class="primary" onclick={() => startScan(false)}>
              Quét lần đầu
            </button>
            {#if netDrives.length}
              <button onclick={() => startScan(true)}>
                Quét cả ổ mạng ({netDrives.map((d) => d.letter).join(", ")})
              </button>
            {/if}
          </div>
          {#if netDrives.length}
            <p class="quiet">
              Ổ mạng lâu hơn nhiều lần. Bỏ qua bây giờ cũng được — nút
              <strong>+ ổ mạng</strong> ở trên làm đúng việc đó bất cứ lúc nào.
            </p>
          {/if}
        </div>
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
    {/if}
  </div>

  <div class="status">
    <span>{statusLine}</span>
    {#if hits.length}
      <span class="timing">{formatCount(hits.length)} kết quả · {elapsedMs.toFixed(1)} ms</span>
    {/if}
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

  .bar {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .search {
    flex: 1;
    min-width: 0;
    padding: 12px 16px;
    font-size: 18px;
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    outline: none;
    user-select: text;
  }
  .search:focus { border-color: var(--accent); }
  .search::placeholder { color: var(--text-dim); }

  .chips { display: flex; gap: 6px; align-items: center; }
  .chip {
    padding: 7px 13px;
    font: inherit;
    font-size: 13px;
    color: var(--text-dim);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 999px;
    cursor: default;
  }
  .chip:hover { color: var(--text); }
  .chip.on {
    color: #fff;
    background: var(--accent);
    border-color: var(--accent);
  }
  .chip.icon {
    display: grid;
    place-items: center;
    padding: 7px 10px;
  }
  .chip.icon svg {
    width: 15px;
    height: 15px;
    stroke-linejoin: round;
  }
  .chip.rescan { margin-left: 4px; }
  .chip.rescan:disabled { opacity: 0.55; }

  .scan {
    padding: 9px 14px 11px;
    font-size: 12.5px;
    color: #cfe0ff;
    background: #1e2836;
    border: 1px solid #2f4260;
    border-radius: 8px;
  }
  .scan-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 8px;
  }
  .stop {
    font: inherit;
    font-size: 12px;
    color: inherit;
    background: none;
    border: 1px solid currentColor;
    border-radius: 5px;
    padding: 1px 8px;
    cursor: pointer;
    opacity: 0.75;
  }
  .stop:hover { opacity: 1; }

  .scan-count { color: var(--text-dim); }
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

  /* Amber rather than red: this is not an error, it is the search telling the
     user what it did. Red would suggest something went wrong. */
  .partial {
    padding: 9px 14px;
    font-size: 12.5px;
    color: #ffe0b0;
    background: #3d3020;
    border: 1px solid #5c4726;
    border-radius: 8px;
  }
  .partial b { color: #ffc978; }

  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding: 9px 12px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .flabel {
    font-size: 12px;
    color: var(--text-dim);
    margin-right: 2px;
  }
  .fsep {
    width: 1px;
    height: 18px;
    margin: 0 6px;
    background: var(--border);
  }
  .chip.small {
    padding: 4px 10px;
    font-size: 12px;
  }
  .chip.small.clear {
    margin-left: 6px;
    color: #ffc978;
    border-color: #5c4726;
  }
  .fnote {
    margin-left: auto;
    font-size: 11.5px;
    color: var(--text-dim);
  }
  .fnote.working { color: #cfe0ff; }

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
  .gwaste { margin-left: auto; color: #ffc978; }

  .row.dupe { padding-left: 24px; }

  .facts {
    flex: 0 0 auto;
    display: flex;
    gap: 5px;
    align-items: center;
  }
  .fact {
    padding: 2px 6px;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
    background: #22262e;
    border-radius: 4px;
  }
  .fact.res { color: #9fd3ff; }
  .fact.dim { opacity: 0.7; }

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

  /* ---- list mode ---- */
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

  .thumb {
    flex: 0 0 auto;
    width: 40px;
    height: 30px;
    object-fit: cover;
    background: #12151b;
    border-radius: 4px;
  }
  /* A file the shell cannot preview keeps its coloured kind badge instead of
     showing a broken-image glyph. */
  .thumb:global(.failed) { display: none; }

  .kind {
    flex: 0 0 22px;
    height: 22px;
    display: grid;
    place-items: center;
    color: #fff;
    border-radius: 5px;
  }
  .kind svg {
    width: 14px;
    height: 14px;
  }
  .kind.video { background: #5b6cff; }
  .kind.image { background: #23a06a; }
  .kind.audio { background: #c2683a; }

  /* Long paths are the norm here; a horizontal scrollbar on the whole list
     would make it unusable. */
  .text { min-width: 0; display: flex; flex-direction: column; }
  .name,
  .dir {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .name { font-size: 14px; }
  .dir { font-size: 11.5px; color: var(--text-dim); }

  .matched {
    flex: 0 0 auto;
    padding: 2px 7px;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
    background: #2c313b;
    border-radius: 999px;
  }

  /* ---- grid mode ---- */
  .results.grid .row {
    position: relative;
    flex-direction: column;
    gap: 6px;
    align-items: stretch;
    padding: 8px;
    border-radius: 8px;
  }
  .results.grid .thumb {
    flex: 1;
    width: 100%;
    height: auto;
    min-height: 0;
    object-fit: contain;
    border-radius: 6px;
  }
  .results.grid .kind {
    position: absolute;
    top: 14px;
    left: 14px;
    opacity: 0.9;
  }
  .results.grid .text { flex: 0 0 auto; }
  .results.grid .name { font-size: 12px; white-space: normal; line-height: 1.3; max-height: 2.6em; }
  .results.grid .dir { display: none; }
  .results.grid .matched { position: absolute; top: 14px; right: 14px; }
  /* In the grid the picture carries the meaning; the numbers would crowd it. */
  .results.grid .facts { display: none; }

  .hint {
    display: inline-block;
    margin-top: 14px;
    font-size: 12px;
    opacity: 0.75;
  }
  /* A second line under the hotkey hint, deliberately quieter: it answers a
     question the user has not asked yet, and should not compete with the one
     they have. */
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

  /* Amber rather than red: the app still works, only the shortcut is gone. */
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

  .firstrun {
    max-width: 560px;
    margin: 48px auto 0;
    padding: 0 24px;
    text-align: left;
    line-height: 1.6;
  }
  .firstrun .big {
    font-size: 17px;
    font-weight: 600;
    margin-bottom: 10px;
    text-align: center;
  }
  .firstrun p {
    color: var(--dim, #8b93a3);
    margin: 0 0 12px;
  }
  .firstrun ul {
    color: var(--dim, #8b93a3);
    margin: 0 0 20px;
    padding-left: 20px;
  }
  .firstrun li {
    margin-bottom: 6px;
  }
  .firstrun kbd {
    font: inherit;
    font-size: 11px;
    padding: 1px 5px;
    border: 1px solid var(--line, #2a2e37);
    border-radius: 4px;
  }
  .firstrun .actions {
    display: flex;
    gap: 10px;
    justify-content: center;
    flex-wrap: wrap;
  }
  .firstrun button {
    padding: 9px 18px;
    border-radius: 8px;
    border: 1px solid var(--line, #2a2e37);
    background: var(--hover, #232833);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .firstrun button.primary {
    background: #2d6cdf;
    border-color: #2d6cdf;
    color: #fff;
    font-weight: 600;
  }
  .firstrun button:hover {
    filter: brightness(1.12);
  }
  .firstrun .quiet {
    margin-top: 14px;
    font-size: 12px;
    text-align: center;
  }

  .empty {
    flex: 1;
    margin: 0;
    padding: 40px;
    text-align: center;
    color: var(--text-dim);
  }

  .status {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
    color: var(--text-dim);
    padding: 0 3px;
  }
  .timing { flex: 0 0 auto; }
</style>
