<script lang="ts">
  import { installUpdate, type UpdateStatus } from "./search";

  let { update }: { update: UpdateStatus } = $props();

  // Toàn bộ trạng thái của việc cập nhật sống ở đây, vì không có gì ngoài dải
  // băng này đọc tới nó. Trước đây năm biến state nằm chung với state tìm kiếm
  // trong App.svelte mà chẳng liên quan gì tới nhau.
  let updating = $state(false);
  let percent = $state(0);
  let error = $state<string | null>(null);
  let dismissed = $state(false);

  async function runUpdate() {
    updating = true;
    error = null;
    percent = 0;
    try {
      // Không có gì chạy sau lệnh này khi mọi thứ suôn sẻ: bộ cài chạy xong
      // thì ứng dụng tự khởi động lại.
      await installUpdate((p) => (percent = p));
    } catch (e) {
      updating = false;
      error = String(e);
    }
  }
</script>

{#if !dismissed}
  <div class="update" role="status">
    {#if updating}
      <span>
        Đang tải bản {update.available}…
        {#if percent > 0}{percent}%{/if}
      </span>
      <span class="update-note">Ứng dụng sẽ tự khởi động lại khi xong.</span>
    {:else if error}
      <span>Không tải được bản mới: {error}</span>
      <button class="update-go" onclick={runUpdate}>Thử lại</button>
      <button class="dismiss" onclick={() => (dismissed = true)}>Để sau</button>
    {:else}
      <span>
        Có bản <strong>{update.available}</strong> — bạn đang dùng
        {update.current}.
      </span>
      <button class="update-go" onclick={runUpdate}>Cập nhật</button>
      <button class="dismiss" onclick={() => (dismissed = true)}>Để sau</button>
    {/if}
  </div>
{/if}

<style>
  /* Cùng hình dạng với thanh lỗi nhưng màu xanh: đây là tin tốt, không phải
     sự cố, và không nên đọc thoáng qua mà tưởng là hỏng hóc. */
  .update {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 10px 14px;
    font-size: 13px;
    color: #cfe6ff;
    background: #1e3350;
    border: 1px solid #2d4a6b;
    border-radius: 8px;
  }
  .update > span:first-child { flex: 1; }
  .update-note {
    font-size: 12px;
    opacity: 0.75;
  }
  .update-go {
    font: inherit;
    padding: 4px 12px;
    color: #0d1b2a;
    background: #7fb8ff;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }
  .update-go:hover { background: #9ac8ff; }
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
</style>
