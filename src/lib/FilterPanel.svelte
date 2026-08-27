<script lang="ts" module>
  import type { Filters } from "./search";

  // Đặt tên theo cách người ta mô tả thứ mình đang tìm, không phải theo cách
  // các con số được lưu.
  const RESOLUTIONS: { label: string; minHeight: number }[] = [
    { label: "≥720p", minHeight: 720 },
    { label: "≥1080p", minHeight: 1080 },
    { label: "4K", minHeight: 2160 },
  ];
  const DURATIONS: { label: string; min: number; max: number }[] = [
    { label: "< 1 phút", min: 0, max: 60_000 },
    { label: "1–10 phút", min: 60_000, max: 600_000 },
    { label: "> 10 phút", min: 600_000, max: 0 },
  ];
  const RECENCY: { label: string; days: number }[] = [
    { label: "7 ngày", days: 7 },
    { label: "30 ngày", days: 30 },
    { label: "1 năm", days: 365 },
  ];

  /// Bộ lọc rỗng, dùng làm giá trị khởi tạo bên phía App.
  export const NO_FILTERS: Filters = {
    minHeight: 0,
    minDurationMs: 0,
    maxDurationMs: 0,
    withinDays: 0,
  };
</script>

<script lang="ts">
  import { formatCount, type EnrichStatus } from "./search";

  let {
    enrich,
    onchange,
  }: {
    enrich: EnrichStatus | null;
    /// Bộ lọc mới đi kèm ngay trong lời gọi, chứ không qua một prop buộc hai
    /// chiều.
    ///
    /// Bản trước tính `filters` trong một `$effect` rồi ghi ngược lên cha qua
    /// `$bindable`. `$effect` chạy *sau* khi trình xử lý bấm chuột kết thúc,
    /// nên `onchange()` bắn đi lúc cha còn đang giữ bộ lọc của lần bấm trước:
    /// chọn ≥1080p thì tìm với "không lọc", rồi bấm "Bỏ lọc" lại tìm với đúng
    /// bộ lọc vừa xoá. Đưa giá trị vào tham số thì không còn khoảng hở nào để
    /// hai bên lệch nhau.
    onchange: (filters: Filters, active: boolean) => void;
  } = $props();

  let minHeight = $state(0);
  let durationChoice = $state(-1);
  let recencyChoice = $state(-1);

  // Một chiều, có chủ ý: ba lựa chọn ở trên là nguồn sự thật, `filters` chỉ là
  // bản dịch của chúng.
  const filters = $derived<Filters>({
    minHeight,
    minDurationMs: durationChoice >= 0 ? DURATIONS[durationChoice].min : 0,
    maxDurationMs: durationChoice >= 0 ? DURATIONS[durationChoice].max : 0,
    withinDays: recencyChoice >= 0 ? RECENCY[recencyChoice].days : 0,
  });
  const active = $derived(
    minHeight > 0 || durationChoice >= 0 || recencyChoice >= 0,
  );

  function setResolution(h: number) {
    minHeight = minHeight === h ? 0 : h;
    onchange(filters, active);
  }

  function setDuration(i: number) {
    durationChoice = durationChoice === i ? -1 : i;
    onchange(filters, active);
  }

  function setRecency(i: number) {
    recencyChoice = recencyChoice === i ? -1 : i;
    onchange(filters, active);
  }

  function clearFilters() {
    minHeight = 0;
    durationChoice = -1;
    recencyChoice = -1;
    onchange(filters, active);
  }
</script>

<div class="filters">
  <span class="flabel">Độ phân giải</span>
  {#each RESOLUTIONS as r (r.minHeight)}
    <button
      class="chip small"
      class:on={minHeight === r.minHeight}
      onclick={() => setResolution(r.minHeight)}
    >{r.label}</button>
  {/each}

  <span class="fsep"></span>
  <span class="flabel">Thời lượng</span>
  {#each DURATIONS as d, i (d.label)}
    <button
      class="chip small"
      class:on={durationChoice === i}
      onclick={() => setDuration(i)}
    >{d.label}</button>
  {/each}

  <span class="fsep"></span>
  <span class="flabel">Sửa đổi trong</span>
  {#each RECENCY as r, i (r.days)}
    <button
      class="chip small"
      class:on={recencyChoice === i}
      onclick={() => setRecency(i)}
    >{r.label}</button>
  {/each}

  {#if active}
    <button class="chip small clear" onclick={clearFilters}>Bỏ lọc</button>
  {/if}

  {#if enrich && enrich.total > 0 && enrich.done < enrich.total}
    <!--
      Nói thẳng chứ không giấu. Một bộ lọc độ phân giải chỉ khớp được với tệp
      mà ai đó đã đọc qua, nên một danh sách kết quả ngắn ở đây là chuyện của
      tiến độ, không phải chuyện của thư viện — và người dùng không có cách
      nào biết điều đó trừ khi nó được viết ra.
    -->
    <span class="fnote" class:working={enrich.running}>
      Đã đọc thuộc tính {formatCount(enrich.done)}/{formatCount(enrich.total)} tệp
      {enrich.running ? "· đang tiếp tục" : "· tạm dừng"}
    </span>
  {/if}
</div>

<style>
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding: 9px 12px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .flabel {
    font-size: 12px;
    color: var(--text-dim);
    margin-right: 2px;
  }
  .fsep {
    width: 1px;
    height: 18px;
    margin: 0 6px;
    background: var(--border);
  }
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
  .chip.small {
    padding: 4px 10px;
    font-size: 12px;
  }
  .chip.small.clear {
    margin-left: 6px;
    color: #ffc978;
    border-color: #5c4726;
  }
  .fnote {
    margin-left: auto;
    font-size: 11.5px;
    color: var(--text-dim);
  }
  .fnote.working { color: #cfe0ff; }
</style>
