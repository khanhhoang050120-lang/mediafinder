<script lang="ts">
  import { cancelScan, type ScanProgress } from "./search";

  let {
    scan,
    scanningNetwork,
  }: {
    scan: ScanProgress | null;
    scanningNetwork: boolean;
  } = $props();
</script>

<div class="scan">
  <div class="scan-head">
    <span>{scan?.message ?? "Đang khởi động tiến trình quét…"}</span>
    {#if scan && scan.volumesTotal > 0}
      <span class="scan-count">ổ {scan.volumesDone + 1}/{scan.volumesTotal}</span>
    {/if}
    <!-- Chỉ mời khi đang ở giai đoạn mạng, vì chỉ giai đoạn đó làm theo được:
         lần quét ổ trong máy chạy ở một tiến trình nâng quyền riêng mà tiến
         trình này không cầm handle. Một nút Dừng không làm gì còn tệ hơn là
         không có nút Dừng. -->
    {#if scanningNetwork && scan?.phase === "network"}
      <button class="stop" onclick={() => cancelScan()}>Dừng</button>
    {/if}
  </div>
  <!-- Không xác định một cách có chủ ý: tổng số bản ghi của một ổ chỉ biết
       được khi quét tới cuối ổ đó, nên mọi con số phần trăm đều là bịa. -->
  <div class="scan-bar"><div class="scan-fill"></div></div>
</div>

<style>
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
</style>
