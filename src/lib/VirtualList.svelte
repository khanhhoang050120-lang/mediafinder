<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  let {
    items,
    itemHeight,
    columns = 1,
    gap = 0,
    overscan = 3,
    row,
    onviewport,
  }: {
    items: T[];
    /** Fixed height of one row, in pixels. */
    itemHeight: number;
    /** 1 for a list; more for a grid. */
    columns?: number;
    gap?: number;
    /** Extra rows rendered above and below, so scrolling reveals nothing blank. */
    overscan?: number;
    row: Snippet<[T, number]>;
    /** Called with the visible index range, for prefetching. */
    onviewport?: (from: number, to: number) => void;
  } = $props();

  let viewport = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let height = $state(0);

  const stride = $derived(itemHeight + gap);
  const rowCount = $derived(Math.ceil(items.length / columns));
  const totalHeight = $derived(rowCount * stride);

  // The window of rows worth rendering. Everything else stays out of the DOM
  // entirely: with five thousand results, building them all would cost far
  // more than the search itself, and the browser would then lay out and paint
  // a document a hundred screens tall to show one.
  const firstRow = $derived(
    Math.max(0, Math.floor(scrollTop / stride) - overscan),
  );
  const lastRow = $derived(
    Math.min(rowCount, Math.ceil((scrollTop + height) / stride) + overscan),
  );

  const visible = $derived.by(() => {
    const out: { item: T; index: number }[] = [];
    for (let r = firstRow; r < lastRow; r++) {
      for (let c = 0; c < columns; c++) {
        const index = r * columns + c;
        if (index < items.length) out.push({ item: items[index], index });
      }
    }
    return out;
  });

  $effect(() => {
    onviewport?.(firstRow * columns, Math.min(items.length, lastRow * columns));
  });

  function onScroll() {
    if (viewport) scrollTop = viewport.scrollTop;
  }

  /** Bring an index into view without moving the list any further than needed. */
  export function scrollToIndex(index: number) {
    if (!viewport) return;
    const r = Math.floor(index / columns);
    const top = r * stride;
    const bottom = top + itemHeight;
    if (top < viewport.scrollTop) {
      viewport.scrollTop = top;
    } else if (bottom > viewport.scrollTop + height) {
      viewport.scrollTop = bottom - height;
    }
  }

  export function scrollToTop() {
    if (!viewport) return;
    // Gán thẳng thay vì `scrollTo({top:0})`: cùng kết quả, nhưng không phụ
    // thuộc vào một phương thức mà môi trường kiểm thử không có — một ngoại
    // lệ ném ra ở đây sẽ giết luôn phần việc đứng sau nó trong cùng effect.
    viewport.scrollTop = 0;
  }
</script>

<div
  class="viewport"
  bind:this={viewport}
  bind:clientHeight={height}
  onscroll={onScroll}
>
  <!--
    One tall spacer holds the scrollbar honest; the rendered rows are absolutely
    positioned inside it. Translating a small set of nodes is what keeps the DOM
    node count flat no matter how many results there are.
  -->
  <div class="spacer" style="height: {totalHeight}px">
    {#each visible as v (v.index)}
      <div
        class="cell"
        style="
          transform: translateY({Math.floor(v.index / columns) * stride}px);
          height: {itemHeight}px;
          left: {(v.index % columns) * (100 / columns)}%;
          width: {100 / columns}%;
        "
      >
        {@render row(v.item, v.index)}
      </div>
    {/each}
  </div>
</div>

<style>
  .viewport {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .spacer {
    position: relative;
    width: 100%;
  }

  .cell {
    position: absolute;
    top: 0;
    box-sizing: border-box;
  }
</style>
