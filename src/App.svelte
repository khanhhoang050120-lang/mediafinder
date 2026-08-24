<script lang="ts">
  // P0 scaffold: UI shell only. IPC wiring lands in P3, thumbnails in P5.
  let query = $state("");
</script>

<main>
  <input
    class="search"
    type="text"
    placeholder="Tìm video, ảnh, nhạc…"
    autocomplete="off"
    spellcheck="false"
    bind:value={query}
  />

  <div class="results">
    <p class="empty">
      {#if query.length === 0}
        Gõ để tìm kiếm
      {:else}
        Chưa nối backend — tìm kiếm hoạt động từ giai đoạn P3
      {/if}
    </p>
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
    gap: 12px;
  }

  .search {
    width: 100%;
    padding: 14px 18px;
    font-size: 20px;
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    outline: none;
    user-select: text;
  }
  .search:focus { border-color: var(--accent); }
  .search::placeholder { color: var(--text-dim); }

  .results {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
  }

  .empty {
    margin: 0;
    padding: 32px;
    text-align: center;
    color: var(--text-dim);
  }
</style>
