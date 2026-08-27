<script lang="ts">
  import { installUpdate, openReleasesPage, type UpdateStatus } from "./search";

  let {
    update,
    open = $bindable(),
  }: {
    update: UpdateStatus;
    /// Hộp thoại đang mở hay không — App giữ quyền mở/đóng vì chính App phải
    /// biết mà nhường bàn phím (cùng lý do với chốt chặn menu/preview bên đó),
    /// và mũi tên mở-lại nằm ở footer của App chứ không ở đây.
    open: boolean;
  } = $props();

  let updating = $state(false);
  let percent = $state(0);
  let error = $state<string | null>(null);

  /// Phần "có gì mới" dành cho người dùng.
  ///
  /// Ghi chú trên máy chủ gồm changelog, rồi một vạch `---`, rồi hướng dẫn
  /// cài đặt cho người tải tay từ trang Releases. Người đang đứng TRONG ứng
  /// dụng không cần ai dạy cách chạy bộ cài — cắt ở vạch, giữ phần đầu.
  /// (Hợp đồng này ghi ở release.yml, chỗ sinh ra nội dung ấy.)
  const notes = $derived.by(() => {
    const raw = update.available?.notes ?? "";
    return raw.split(/\n\s*-{3,}\s*\n/)[0].trim();
  });

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

  // ---- Dải mờ "còn nữa" ở mép dưới ô ghi chú ----
  //
  // Ô ghi chú có trần chiều cao và tự cuộn (khung với hai nút thì đứng yên —
  // ghi chú dài cỡ nào cũng không đẩy được nút Cập nhật ra khỏi màn hình).
  // Dải mờ chỉ hiện khi thật sự còn chữ bên dưới, và tắt khi đã cuộn tới đáy
  // — một dải mờ thường trực trên ô ngắn là lời hứa suông.
  let notesEl = $state<HTMLDivElement | null>(null);
  let notesOverflow = $state(false);
  let notesAtEnd = $state(false);

  function measureNotes() {
    if (!notesEl) {
      notesOverflow = false;
      return;
    }
    notesOverflow = notesEl.scrollHeight > notesEl.clientHeight + 1;
    notesAtEnd =
      notesEl.scrollTop + notesEl.clientHeight >= notesEl.scrollHeight - 2;
  }

  $effect(() => {
    void notes;
    void open;
    measureNotes();
  });

  function later() {
    if (updating) return; // đang tải dở thì không có "để sau"
    open = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      later();
    }
  }
</script>

<svelte:window on:keydown|capture={onKeydown} on:resize={measureNotes} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={later} role="presentation">
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Thông báo cập nhật"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      {#if updating}
        <div class="title">Đang tải bản {update.available?.version}…</div>
        <div class="note">
          {#if percent > 0}{percent}% · {/if}Ứng dụng sẽ tự khởi động lại khi xong.
        </div>
      {:else}
        <div class="title">
          Có bản <b>{update.available?.version}</b>
          <span class="dim">— bạn đang dùng {update.current}</span>
        </div>

        {#if notes}
          <div class="notes-wrap" class:fade={notesOverflow && !notesAtEnd}>
            <div class="notes" bind:this={notesEl} onscroll={measureNotes}>{notes}</div>
          </div>
        {:else}
          <!-- Máy chủ không gửi ghi chú thì nói thẳng, đừng để một khoảng
               trắng khiến người ta tưởng hộp thoại bị lỗi. -->
          <div class="notes dim">Bản này không kèm ghi chú thay đổi.</div>
        {/if}
        <button class="all" onclick={() => openReleasesPage().catch(() => {})}>
          Xem đầy đủ trên trang Releases ↗
        </button>

        {#if error}
          <div class="error">Không tải được bản mới: {error}</div>
        {/if}

        <div class="actions">
          <button class="go" onclick={runUpdate}>
            {error ? "Thử lại" : "Cập nhật"}
          </button>
          <button class="later" onclick={later}>Để sau</button>
        </div>
        <div class="hint">
          Chọn "Để sau" thì lời mời nằm ở mũi tên dưới chân cửa sổ — bấm vào đó
          khi nào muốn cập nhật.
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 45;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .dialog {
    width: min(100%, 520px);
    max-height: 80vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 20px;
    background: var(--panel, #16181d);
    border: 1px solid #2d4a6b;
    border-radius: 12px;
    color: #cfe6ff;
  }
  .title {
    font-size: 15px;
  }
  .title b {
    color: #fff;
  }
  .dim {
    color: var(--dim, #8b93a3);
  }
  .notes-wrap {
    position: relative;
    background: #10151d;
    border: 1px solid var(--line, #2a2e37);
    border-radius: 8px;
    overflow: hidden;
  }
  /* Dải mờ báo "còn nữa" — chỉ vẽ khi lớp bọc mang class fade. */
  .notes-wrap.fade::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 34px;
    background: linear-gradient(to bottom, rgba(16, 21, 29, 0), #10151d);
    pointer-events: none;
  }
  .notes {
    font-size: 13px;
    line-height: 1.55;
    /* Ghi chú là văn bản nhiều dòng từ máy chủ; giữ nguyên xuống dòng của nó
       thay vì đổ thành một khối chữ liền — và bẻ được cả chuỗi liền mạch dài
       (URL, đường dẫn), thứ pre-wrap một mình không bẻ nổi và từng là đường
       tràn ngang duy nhất còn sót. */
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    /* Trần chiều cao: ghi chú dài thì CUỘN TRONG Ô — tiêu đề và hai nút của
       hộp thoại đứng yên, hành động chính không bao giờ trốn khỏi màn hình. */
    max-height: min(40vh, 320px);
    overflow-y: auto;
    padding: 10px 12px;
    color: var(--text, #e7eaf0);
  }
  .all {
    align-self: flex-start;
    font: inherit;
    font-size: 12px;
    color: #7fb8ff;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .all:hover {
    color: #b9d9ff;
    text-decoration: underline;
  }
  .note {
    font-size: 13px;
  }
  .error {
    font-size: 13px;
    color: #ffd7d7;
    white-space: pre-line;
  }
  .actions {
    display: flex;
    gap: 10px;
  }
  .go {
    font: inherit;
    padding: 7px 18px;
    color: #0d1b2a;
    background: #7fb8ff;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
  }
  .go:hover {
    background: #9ac8ff;
  }
  .later {
    font: inherit;
    font-size: 13px;
    color: inherit;
    background: none;
    border: 1px solid currentColor;
    border-radius: 8px;
    padding: 7px 14px;
    cursor: default;
    opacity: 0.8;
  }
  .later:hover {
    opacity: 1;
  }
  .hint {
    font-size: 11.5px;
    color: var(--dim, #8b93a3);
  }
</style>
