<script lang="ts">
  import { reasonFor, type FilterState } from "./emptyReason";
  import type { Freshness } from "./search";

  let {
    filters,
    fresh,
    onclear,
    onrescan,
    onscannetwork,
  }: {
    filters: FilterState;
    /// `null` khi chưa hỏi xong backend — lúc đó chỉ nói câu trung tính.
    fresh: Freshness | null;
    onclear: () => void;
    onrescan: () => void;
    onscannetwork: () => void;
  } = $props();

  // `Date.now()` đọc trong `$derived` nên nó được tính lại mỗi khi `fresh` hay
  // `filters` đổi — đủ tươi cho một màn hình chỉ xuất hiện khi tìm không ra.
  // Không đặt bộ đếm giờ: một dòng chữ tự đổi từ "5 tiếng" sang "6 tiếng"
  // trong lúc người ta đang đọc chỉ gây phân tâm.
  const ly_do = $derived(reasonFor(filters, fresh, Math.floor(Date.now() / 1000)));

  function bam() {
    const d = ly_do.action?.do;
    if (d === "clear-filters") onclear();
    else if (d === "rescan") onrescan();
    else if (d === "scan-network") onscannetwork();
  }
</script>

<div class="empty">
  <p class="title">{ly_do.title}</p>
  <p class="detail">{ly_do.detail}</p>
  {#if ly_do.action}
    <!--
      Chỉ ra vấn đề mà bắt người dùng tự đi tìm chỗ sửa là làm xong một nửa
      việc. Nút này làm nốt nửa còn lại.
    -->
    <button class="act" class:slow={ly_do.action.do !== "clear-filters"} onclick={bam}>
      {ly_do.action.label}
    </button>
  {/if}
</div>

<style>
  .empty {
    /* Khối .results bọc ngoài là flex hàng ngang, nên thiếu flex:1 thì khối
       này chỉ rộng bằng nội dung và nằm nép sang một bên — đúng lỗi đã thấy:
       dòng báo lý do dạt hẳn sang lề phải. */
    flex: 1;
    align-self: center;
    text-align: center;
    padding: 34px 16px;
    color: var(--text-dim);
  }
  .title {
    margin: 0 0 6px;
    font-size: 15px;
    color: var(--text);
  }
  .detail {
    margin: 0 auto;
    max-width: 52ch;
    font-size: 12.5px;
    line-height: 1.55;
  }
  .act {
    margin-top: 14px;
    padding: 6px 15px;
    font-family: inherit;
    font-size: 12.5px;
    font-weight: 500;
    color: #fff;
    background: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 6px;
    cursor: pointer;
  }
  /* Việc mất vài phút thì không được trông giống việc mất một giây. Nút mờ
     hơn để người dùng dừng lại đọc nhãn — nhãn đã ghi sẵn thời gian. */
  .slow {
    color: var(--text);
    background: transparent;
    border-color: var(--border);
  }
  .act:hover {
    filter: brightness(1.1);
  }
  .act:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
