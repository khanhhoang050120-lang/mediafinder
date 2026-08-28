<script lang="ts" module>
  import type { MediaKind } from "./search";

  export const KINDS: { key: MediaKind; label: string }[] = [
    { key: "video", label: "Video" },
    { key: "image", label: "Ảnh" },
    { key: "audio", label: "Nhạc" },
  ];
</script>

<script lang="ts">
  import type { NetworkDrive, Order } from "./search";

  let {
    query = $bindable(),
    activeKinds = $bindable(),
    grid = $bindable(),
    showFilters = $bindable(),
    order = $bindable(),
    dupeMode,
    filtersActive,
    scanning,
    lastScanHint,
    netDrives,
    inputEl = $bindable(),
    oninput,
    onchange,
    ontoggledupes,
    onscan,
  }: {
    query: string;
    activeKinds: MediaKind[];
    grid: boolean;
    showFilters: boolean;
    order: Order;
    dupeMode: boolean;
    filtersActive: boolean;
    scanning: boolean;
    /// Lời nhắc "lần quét gần nhất" cho tooltip nút Quét lại. App dựng câu
    /// này vì chỉ nó có `meta`; thanh này chỉ hiển thị.
    lastScanHint: string;
    netDrives: NetworkDrive[];
    /// Ô nhập được đưa ngược ra ngoài vì phím tắt toàn cục và phím Escape đều
    /// phải lấy được con trỏ về đây, mà cả hai đều sống ở cửa sổ chứ không
    /// phải ở thanh này.
    inputEl: HTMLInputElement | undefined;
    oninput: () => void;
    onchange: () => void;
    ontoggledupes: () => void;
    onscan: (withNetwork: boolean) => void;
  } = $props();

  function toggleKind(k: MediaKind) {
    activeKinds = activeKinds.includes(k)
      ? activeKinds.filter((x) => x !== k)
      : [...activeKinds, k];
    onchange();
  }
</script>

<div class="bar">
  <!--
    Hướng dẫn về khả năng tiếp cận nhắm tới những trang có vài ô nhập, nơi
    việc cướp con trỏ làm người ta mất phương hướng. Cửa sổ này sinh ra để
    được gõ vào: nó có đúng một ô, và làm khác đi thì mỗi lần tìm kiếm đều
    phải với tay lấy chuột trước.
  -->
  <!-- svelte-ignore a11y_autofocus -->
  <input
    bind:this={inputEl}
    bind:value={query}
    {oninput}
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
        onchange();
      }}
      title={order === "newest"
        ? "Đang xếp theo thời gian sửa đổi — bấm để quay về xếp theo mức độ khớp"
        : "Xếp mới nhất lên đầu"}
    >{order === "newest" ? "Mới nhất" : "Liên quan"}</button>
    <button
      class="chip"
      class:on={dupeMode}
      onclick={ontoggledupes}
      title="Tìm các tệp trùng lặp trong thư viện"
    >Trùng lặp</button>
    <button
      class="chip rescan"
      onclick={() => onscan(false)}
      disabled={scanning}
      title={`Quét lại các ổ gắn trong máy — vài giây${lastScanHint}`}
    >
      {scanning ? "Đang quét…" : "Quét lại"}
    </button>
    {#if netDrives.length > 0}
      <button
        class="chip rescan"
        onclick={() => onscan(true)}
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

<style>
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
</style>
