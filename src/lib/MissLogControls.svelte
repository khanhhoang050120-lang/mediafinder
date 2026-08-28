<script lang="ts">
  import {
    missLogClear,
    missLogOpen,
    missLogSetEnabled,
    missLogStatus,
    type MissLogStatus,
  } from "./search";

  /// Khối điều khiển bộ ghi truy-vấn-0-kết-quả.
  ///
  /// Sống ngay trong màn "Không tìm thấy kết quả nào" — đúng khoảnh khắc mà
  /// việc đo trở nên có nghĩa, không chiếm một pixel nào của giao diện chính.
  /// Ba lời hứa nói thẳng với người dùng: mặc định tắt, dữ liệu nằm trên máy
  /// này, và có nút xoá.

  let status = $state<MissLogStatus | null>(null);
  let error = $state<string | null>(null);

  function refresh() {
    missLogStatus()
      .then((s) => (status = s))
      .catch(() => (status = null));
  }
  refresh();

  async function toggle() {
    if (!status) return;
    error = null;
    await missLogSetEnabled(!status.enabled).catch((e) => (error = String(e)));
    refresh();
  }

  async function view() {
    error = null;
    await missLogOpen().catch((e) => (error = String(e)));
  }

  async function wipe() {
    error = null;
    await missLogClear().catch((e) => (error = String(e)));
    refresh();
  }
</script>

{#if status}
  <span class="misslog">
    {#if status.enabled}
      Đang ghi các truy vấn không ra kết quả để cải thiện tìm kiếm
      — đã ghi <b>{status.count}</b>, chỉ lưu trên máy này.
      <button class="lnk" onclick={view} disabled={status.count === 0}>Xem</button>
      <button class="lnk" onclick={wipe} disabled={status.count === 0}>Xoá</button>
      <button class="lnk" onclick={toggle}>Tắt</button>
    {:else}
      Muốn tìm kiếm khôn hơn? <button class="lnk" onclick={toggle}>Bật ghi</button>
      các truy vấn không ra kết quả — chỉ lưu trên máy này, xoá lúc nào cũng được.
    {/if}
    {#if error}<span class="err">{error}</span>{/if}
  </span>
{/if}

<style>
  .misslog {
    display: inline-block;
    margin-top: 14px;
    font-size: 12px;
    color: var(--text-dim);
    max-width: 52ch;
    line-height: 1.6;
  }
  .misslog b {
    color: var(--text);
  }
  .lnk {
    font: inherit;
    font-size: 12px;
    padding: 0 2px;
    color: var(--accent, #7fb8ff);
    background: none;
    border: none;
    cursor: pointer;
    text-decoration: underline;
  }
  .lnk:disabled {
    color: var(--text-dim);
    text-decoration: none;
    cursor: default;
    opacity: 0.6;
  }
  .err {
    display: block;
    color: #ef8f8f;
  }
</style>
