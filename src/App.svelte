<script lang="ts">
  import ContextMenu, { type MenuItem } from "./lib/ContextMenu.svelte";
  import {
    coalesce,
    formatCount,
    formatWhen,
    indexStatus,
    openFile,
    revealInExplorer,
    searchFiles,
    type IndexMeta,
    type MediaKind,
    type RelaxedInfo,
    type SearchHit,
  } from "./lib/search";

  const KINDS: { key: MediaKind; label: string }[] = [
    { key: "video", label: "Video" },
    { key: "image", label: "Ảnh" },
    { key: "audio", label: "Nhạc" },
  ];

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let selected = $state(0);
  let elapsedMs = $state(0);
  let searching = $state(false);
  let activeKinds = $state<MediaKind[]>([]);
  let meta = $state<IndexMeta | null>(null);
  let error = $state<string | null>(null);
  let relaxed = $state<RelaxedInfo | null>(null);
  let menu = $state<{ x: number; y: number; hit: SearchHit } | null>(null);

  let inputEl: HTMLInputElement | undefined = $state();
  let listEl: HTMLDivElement | undefined = $state();

  indexStatus().then((m) => (meta = m));

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
    searchFiles(q, activeKinds)
      .then((res) => {
        // `null` means a newer keystroke already superseded this one.
        if (!res) return;
        hits = res.hits;
        relaxed = res.relaxed;
        elapsedMs = res.elapsedMs;
        selected = 0;
        listEl?.scrollTo({ top: 0 });
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

  function move(delta: number) {
    if (!hits.length) return;
    selected = Math.max(0, Math.min(hits.length - 1, selected + delta));
    // Keep the selection in view without yanking the whole list around.
    listEl
      ?.querySelector<HTMLElement>(`[data-row="${selected}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }

  function onKeydown(e: KeyboardEvent) {
    // While the context menu is up it owns the keyboard. Both components
    // listen on `window`, and `stopPropagation` does not stop a sibling
    // listener on the same target — without this guard Escape would close the
    // menu *and* clear the search box in the same keystroke.
    if (menu) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        move(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        move(-1);
        break;
      case "PageDown":
        e.preventDefault();
        move(10);
        break;
      case "PageUp":
        e.preventDefault();
        move(-10);
        break;
      case "Enter":
        e.preventDefault();
        (e.ctrlKey ? reveal : open)(hits[selected]);
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
    </div>
  </div>

  {#if error}
    <div class="error" role="alert">
      <span>{error}</span>
      <button class="dismiss" onclick={() => (error = null)}>Đóng</button>
    </div>
  {/if}

  {#if relaxed}
    <div class="partial">
      Không có tệp nào khớp đủ <b>{relaxed.totalTokens}</b> từ.
      Đang hiện các tệp khớp nhiều nhất — <b>{relaxed.bestMatched}/{relaxed.totalTokens}</b> từ.
    </div>
  {/if}

  <div class="results" bind:this={listEl} role="listbox" tabindex="-1" aria-label="Kết quả tìm kiếm">
    {#if hits.length}
      {#each hits as hit, i}
        <div
          class="row"
          class:sel={i === selected}
          data-row={i}
          role="option"
          aria-selected={i === selected}
          tabindex="-1"
          onclick={() => (selected = i)}
          ondblclick={() => open(hit)}
          oncontextmenu={(e) => onContextMenu(e, i)}
          onkeydown={() => {}}
        >
          <span class="kind {hit.kind}">{hit.kind[0].toUpperCase()}</span>
          <span class="text">
            <span class="name">{hit.name}</span>
            <span class="dir">{hit.dir}</span>
          </span>
          {#if relaxed}
            <span class="matched" title="Số từ khớp trên tổng số từ trong truy vấn">
              {hit.matched}/{relaxed.totalTokens}
            </span>
          {/if}
        </div>
      {/each}
    {:else}
      <p class="empty">
        {#if !query.trim()}
          Gõ để tìm kiếm · chuột phải vào kết quả để mở thư mục chứa tệp
        {:else if searching}
          Đang tìm…
        {:else}
          Không tìm thấy kết quả nào
        {/if}
      </p>
    {/if}
  </div>

  <div class="status">
    <span>{statusLine}</span>
    {#if hits.length}
      <span class="timing">{formatCount(hits.length)} kết quả · {elapsedMs.toFixed(1)} ms</span>
    {/if}
  </div>
</main>

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

  .chips { display: flex; gap: 6px; }
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

  .matched {
    flex: 0 0 auto;
    padding: 2px 7px;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
    background: #2c313b;
    border-radius: 999px;
  }

  .results {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
  }

  .row {
    display: flex;
    gap: 11px;
    align-items: center;
    padding: 7px 12px;
    cursor: default;
  }
  .row:hover { background: #262a33; }
  .row.sel { background: #2f3a4f; }

  .kind {
    flex: 0 0 22px;
    height: 22px;
    display: grid;
    place-items: center;
    font-size: 11px;
    font-weight: 600;
    color: #fff;
    border-radius: 5px;
  }
  .kind.video { background: #5b6cff; }
  .kind.image { background: #23a06a; }
  .kind.audio { background: #c2683a; }

  /* The two lines must never widen the row: long paths are the norm here, and
     a horizontal scrollbar on the whole list would make it unusable. */
  .text { min-width: 0; display: flex; flex-direction: column; }
  .name,
  .dir {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .name { font-size: 14px; }
  .dir { font-size: 11.5px; color: var(--text-dim); }

  .empty {
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
