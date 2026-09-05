<script lang="ts">
  import MediaRow from "./MediaRow.svelte";
  import VirtualList from "./VirtualList.svelte";
  import {
    cancelDuplicates,
    dupeGroups,
    dupeProgress,
    findDuplicates,
    formatBytes,
    formatCount,
    type DupeGroup,
    type DupeProgress,
    type SearchHit,
  } from "./search";

  let {
    epoch,
    rowHeight,
    thumbSize,
    onerror,
    onopen,
    onreveal,
    oncontextmenu,
  }: {
    epoch: number;
    rowHeight: number;
    thumbSize: number;
    onerror: (message: string) => void;
    onopen: (hit: SearchHit) => void;
    onreveal: (hit: SearchHit) => void;
    oncontextmenu: (e: MouseEvent, hit: SearchHit) => void;
  } = $props();

  let dupes = $state<DupeGroup[]>([]);
  let stat = $state<DupeProgress | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;

  /// Các nhóm được trải phẳng thành từng dòng để cùng một bộ ảo hoá vẽ được:
  /// một dòng tiêu đề cho mỗi nhóm, rồi tới các tệp của nó.
  type DupeRow =
    | { head: true; group: DupeGroup; n: number }
    | { head: false; hit: SearchHit; n: number; epoch: number };

  const rows = $derived.by<DupeRow[]>(() => {
    const out: DupeRow[] = [];
    for (const g of dupes) {
      out.push({ head: true, group: g, n: g.files.length });
      // `g.epoch` chứ không phải `epoch` của App: vị trí trong `hit.index`
      // thuộc về chỉ mục lúc quét, nên ảnh thu nhỏ phải hỏi theo epoch đó.
      for (const f of g.files)
        out.push({ head: false, hit: f, n: g.files.length, epoch: g.epoch });
    }
    return out;
  });

  // ---- Con trỏ bàn phím ----
  //
  // Con trỏ đếm theo *tệp*, không theo dòng: dòng tiêu đề nhóm không phải là
  // thứ chọn được, và một con trỏ dừng lại trên "3 bản sao" thì Enter chẳng
  // có nghĩa gì. Danh sách vị trí các dòng tệp được dịch sẵn ở đây để mũi tên
  // chỉ việc cộng trừ.
  const fileRows = $derived(
    rows.map((r, i) => (r.head ? -1 : i)).filter((i) => i >= 0),
  );
  let cursor = $state(0);
  /// Vị trí dòng (trong `rows`) đang được chọn; -1 khi chưa có gì.
  const selRow = $derived(fileRows[cursor] ?? -1);

  let listRef = $state<ReturnType<typeof VirtualList> | undefined>();

  // Danh sách vừa nạp xong thì con trỏ về tệp đầu — giữ con trỏ cũ là trỏ vào
  // một vị trí của danh sách không còn tồn tại.
  $effect(() => {
    void rows;
    cursor = 0;
  });

  /// Bàn phím sống ở App (một chủ sở hữu duy nhất, cùng lý do với chốt chặn
  /// menu/preview bên đó); component này chỉ đưa ra các thao tác.
  export function move(delta: number) {
    if (!fileRows.length) return;
    cursor = Math.max(0, Math.min(fileRows.length - 1, cursor + delta));
    listRef?.scrollToIndex(fileRows[cursor]);
  }

  /// Enter trên tệp đang chọn: mở tệp, hoặc mở thư mục chứa khi `reveal`.
  export function activate(reveal: boolean) {
    const r = rows[selRow];
    if (r && !r.head) (reveal ? onreveal : onopen)(r.hit);
  }

  // Việc quét bắt đầu khi component gắn vào và dừng khi nó bị gỡ ra. Trước
  // đây đó là hai hàm rời `startDupes`/`exitDupes` mà người gọi phải nhớ ghép
  // đôi cho đúng; buộc vào vòng đời thì không quên được nữa.
  $effect(() => {
    start();
    return () => {
      clearInterval(timer);
      timer = undefined;
      // Rời khỏi màn này thì đĩa được nghỉ. Thiếu dòng này thì lần quét còn
      // đọc thêm vài phút nữa để ra một câu trả lời chẳng ai quay lại xem,
      // trong khi tranh ổ đĩa với việc mà người dùng vừa quay về làm.
      cancelDuplicates().catch(() => {});
    };
  });

  async function start() {
    // Một lần quét đã xong vẫn còn được backend giữ. Chạy lại chỉ vì người
    // dùng quay lại màn này là ném đi mười phút đọc đĩa để tới đúng cái kết
    // quả cũ.
    //
    // Hỏi `completed` chứ không phải `groups > 0`: một thư viện không có gì
    // trùng lặp là một lần quét đã xong mà câu trả lời tình cờ rỗng, và coi
    // đó là "chưa quét bao giờ" thì lần nào ghé qua cũng quét lại từ đầu.
    try {
      stat = await dupeProgress();
      if (stat.running) {
        // Ai đó rời đi giữa chừng rồi quay lại. Theo dõi lần đang chạy thay
        // vì từ chối để mở lần thứ hai.
        poll();
        return;
      }
      if (stat.completed) {
        dupes = await dupeGroups();
        return;
      }
    } catch {
      // rơi xuống dưới và quét
    }

    dupes = [];
    try {
      await findDuplicates();
    } catch (e) {
      onerror(String(e));
      return;
    }
    poll();
  }

  function poll() {
    clearInterval(timer);
    timer = setInterval(async () => {
      try {
        stat = await dupeProgress();
      } catch {
        return;
      }
      if (!stat.running) {
        clearInterval(timer);
        timer = undefined;
        // Một lần quét bị dừng để lại `completed` false và kết quả rỗng; hiện
        // cái đó thành "không tìm thấy tệp trùng lặp" là nói dối.
        if (stat.completed) dupes = await dupeGroups();
      }
    }, 400);
  }
</script>

<div class="dupebar">
  {#if stat?.running}
    <span>
      Đang đối chiếu {formatCount(stat.hashed)}/{formatCount(stat.candidates)} tệp
      cùng dung lượng…
    </span>
    <div class="scan-bar"><div class="scan-fill"></div></div>
  {:else if dupes.length}
    <!--
      Số nhóm và tổng dung lượng thu hồi được phải nói về cùng một tập. Bản
      đầu ghép số nhóm *lấy về* với phần lãng phí của *tất cả* các nhóm, đọc
      thành "500 nhóm đang tốn của bạn 520 GB" — lệch hơn mười lần.
    -->
    <span>
      <b>{formatCount(stat?.groups ?? dupes.length)}</b> nhóm trùng lặp ·
      có thể thu hồi <b>{formatBytes(stat?.wasted ?? 0)}</b>
      {#if (stat?.groups ?? 0) > dupes.length}
        <span class="dupenote">— đang hiện {formatCount(dupes.length)} nhóm lãng phí nhiều nhất</span>
      {/if}
    </span>
    <!--
      Nói thẳng ra vì tầng 2 đối chiếu hai đầu tệp, không phải toàn bộ. Cách
      đó đúng để tìm ứng viên và sai nếu lấy làm căn cứ để xoá mà không xem lại.
    -->
    <span class="dupenote">Đối chiếu theo dung lượng và hai đầu tệp — hãy xem lại trước khi xoá</span>
  {:else if stat?.completed}
    <span>Không tìm thấy tệp trùng lặp nào.</span>
  {:else}
    <!--
      Không giống với việc không tìm thấy gì: một lần quét bị dừng thì không
      có câu trả lời nào cả, và nói "không tìm thấy" là một khẳng định mà nó
      chưa bao giờ đưa ra.
    -->
    <span>Đã dừng — chưa đối chiếu xong.</span>
  {/if}
</div>

<div class="results">
  {#if rows.length}
    <VirtualList bind:this={listRef} items={rows} itemHeight={rowHeight} columns={1} overscan={4}>
      {#snippet row(r: DupeRow, i: number)}
        {#if r.head}
          <div class="ghead">
            <span class="gcount">{r.n} bản sao</span>
            <span class="gsize">{formatBytes(r.group.size)} mỗi tệp</span>
            <span class="gwaste">thừa {formatBytes(r.group.wasted)}</span>
          </div>
        {:else}
          <div
            class="row dupe"
            class:sel={i === selRow}
            role="option"
            aria-selected={i === selRow}
            tabindex="-1"
            onclick={() => (cursor = fileRows.indexOf(i))}
            ondblclick={() => onopen(r.hit)}
            oncontextmenu={(e) => oncontextmenu(e, r.hit)}
            onkeydown={() => {}}
          >
            <MediaRow hit={r.hit} epoch={r.epoch ?? epoch} {thumbSize} />
          </div>
        {/if}
      {/snippet}
    </VirtualList>
  {:else}
    <p class="empty">
      {#if stat?.running}
        Đang đối chiếu…
      {:else if stat?.completed}
        Không có tệp nào trùng lặp — không có gì để thu hồi.
      {:else}
        Chưa có kết quả
      {/if}
    </p>
  {/if}
</div>

<style>
  .dupebar {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 9px 14px 11px;
    font-size: 12.5px;
    color: #cfe0ff;
    background: #1e2836;
    border: 1px solid #2f4260;
    border-radius: 8px;
  }
  .dupebar b { color: #fff; }
  .dupenote { color: var(--text-dim); font-size: 11.5px; }

  /* Không xác định một cách có chủ ý: tổng số bản ghi của một ổ chỉ biết được
     khi quét tới cuối ổ đó, nên mọi con số phần trăm đều là bịa. */
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

  .results {
    display: flex;
    flex: 1;
    min-height: 0;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
  }

  .ghead {
    display: flex;
    gap: 10px;
    align-items: center;
    height: 100%;
    padding: 0 12px;
    font-size: 12px;
    color: var(--text-dim);
    background: #20242c;
    border-top: 1px solid var(--border);
  }
  .gcount { color: var(--text); font-weight: 600; }
  .gwaste { margin-left: auto; color: #ffc978; }

  .row {
    display: flex;
    gap: 11px;
    align-items: center;
    height: 100%;
    padding: 0 12px;
    cursor: default;
  }
  .row:hover { background: #262a33; }
  .row.dupe { padding-left: 24px; }
  /* Cùng màu với dòng chọn bên danh sách tìm kiếm — hai chế độ, một ngôn ngữ. */
  .row.sel { background: #2f3a4f; }

  .empty {
    flex: 1;
    margin: 0;
    padding: 40px;
    text-align: center;
    color: var(--text-dim);
  }
</style>
