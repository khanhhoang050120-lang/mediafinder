<script lang="ts">
  import { formatCount } from "./search";
  import type { DriveBucket } from "./drives";

  let {
    buckets,
    selected = $bindable(),
    total,
  }: {
    buckets: DriveBucket[];
    /// Ổ đang xem; `null` là "Tất cả".
    selected: string | null;
    /// Tổng số kết quả, cho chip "Tất cả".
    total: number;
  } = $props();
</script>

<!--
  Chỉ dựng khi kết quả trải trên nhiều ổ. Một hàng chip có đúng một ổ không
  cho biết thêm điều gì mà đường dẫn chưa nói, và chiếm mất một dòng của danh
  sách — thứ đang thực sự cần chỗ.
-->
{#if buckets.length > 1}
  <div class="drives">
    <span class="lbl">Ổ đĩa</span>
    <button
      class="dchip"
      class:on={selected === null}
      aria-pressed={selected === null}
      onclick={() => (selected = null)}
    >
      Tất cả <span class="n">{formatCount(total)}</span>
    </button>
    {#each buckets as b (b.id)}
      <button
        class="dchip"
        class:on={selected === b.id}
        class:nas={b.network}
        aria-pressed={selected === b.id}
        title={b.network ? "Ổ mạng — chậm hơn ổ trong máy" : `Ổ ${b.label}`}
        onclick={() => (selected = selected === b.id ? null : b.id)}
      >
        {b.label} <span class="n">{formatCount(b.count)}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .drives {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding: 7px 10px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .lbl {
    font-size: 12px;
    color: var(--text-dim);
    margin-right: 2px;
  }
  .dchip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    font: inherit;
    font-size: 12px;
    color: var(--text-dim);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 999px;
    cursor: default;
  }
  .dchip:hover {
    color: var(--text);
  }
  .dchip.on {
    color: #fff;
    background: var(--accent);
    border-color: var(--accent);
  }
  /* Số kết quả nhạt hơn nhãn ổ: nhãn là thứ để nhắm tới, con số là thứ để
     liếc qua. Chữ số đều bề ngang cho các chip thẳng hàng nhau. */
  .n {
    font-size: 11px;
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
  }
  .dchip.on .n {
    opacity: 0.85;
  }
  /* Ổ mạng mang màu riêng — không phải trang trí: nó chậm hơn và thường là
     kho lưu trữ chứ không phải nơi đang làm việc. */
  .dchip.nas {
    color: #ffc978;
    border-color: #5c4726;
  }
  .dchip.nas.on {
    color: #fff;
    background: #8a5a1e;
    border-color: #8a5a1e;
  }
</style>
