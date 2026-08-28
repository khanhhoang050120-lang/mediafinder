<script lang="ts">
  import {
    formatBytes,
    formatDuration,
    formatResolution,
    thumbUrl,
    type MediaKind,
    type SearchHit,
  } from "./search";
  import { acquireThumbSlot } from "./thumbQueue";
  import { driveKey, driveLabel, isNetworkDrive } from "./drives";

  // Cùng chữ mà các chip lọc dùng, để nhãn và cái chip bật/tắt nó không bao
  // giờ nói khác nhau.
  const KIND_LABEL: Record<MediaKind, string> = {
    video: "Video",
    image: "Ảnh",
    audio: "Nhạc",
  };

  let {
    hit,
    epoch,
    thumbSize,
    totalTokens = 0,
    grid = false,
    showDrive = false,
    netLetters = undefined,
  }: {
    hit: SearchHit;
    epoch: number;
    thumbSize: number;
    /// Số từ trong truy vấn, khi kết quả bị nới lỏng. 0 nghĩa là không hiện —
    /// chế độ trùng lặp không có truy vấn nào để mà nới.
    totalTokens?: number;
    /// Chế độ lưới đổi kiểu dáng của chính các phần bên trong dòng, nên nó
    /// phải là prop chứ không thể là selector `.grid .thumb` từ file cha:
    /// CSS của Svelte bị giới hạn theo từng file, một quy tắc viết ở App
    /// không với tới được các class khai báo ở đây.
    grid?: boolean;
    /// Hiện nhãn ổ đĩa đầu dòng. Chỉ bật khi kết quả trải trên nhiều ổ —
    /// một nhãn `D:` trên mọi dòng của một danh sách toàn ổ D là mực in
    /// không nói gì.
    showDrive?: boolean;
    /// Chữ cái các ổ mạng đang gắn. Thiếu nó thì ổ mạng ánh xạ (`Y:`) bị
    /// tô như ổ trong máy — xem ghi chú ở `isNetworkDrive`.
    netLetters?: Set<string>;
  } = $props();

  const drive = $derived(showDrive ? driveKey(hit.path) : "");

  // ---- Tải thumbnail: qua cửa xoay, lỗi tạm thì thử lại ----
  //
  // Thẻ <img> không đọc được mã lỗi HTTP, nên "đĩa đang bận" (503) và "tệp
  // này không có thumbnail" (404) trông giống hệt nhau từ đây. Phân biệt bằng
  // thời gian: thử lại vài lần có giãn cách — lỗi tạm sẽ khỏi khi đĩa rảnh,
  // còn lỗi thật thì backend đã nhớ trong miss-cache nên các lần thử lại gần
  // như miễn phí, hết lượt thì buông.
  const MAX_RETRY = 3;
  const BACKOFF_MS = [300, 1000, 3000];

  /// Dòng phải đứng yên trên màn hình chừng này rồi mới được hỏi ảnh.
  ///
  /// Không có nhịp chờ này, một cú kéo thanh cuộn thật nhanh sẽ mount rồi
  /// unmount hàng chục dòng trong tích tắc — mỗi dòng unmount trả chỗ cửa
  /// xoay, nhưng yêu cầu HTTP của nó đã bay đi rồi, nên cú kéo vẫn bơm đầy
  /// hàng đợi backend bằng toàn ảnh của những dòng không ai còn nhìn. Các
  /// dòng thật sự dừng lại sau đó phải xếp sau đống rác ấy, hoặc ăn 503 tới
  /// hết lượt thử lại. Dòng bị cuộn lướt qua thì chết trước 120ms và không
  /// bắn ra gì cả — đó chính là "the row asks again once it stops moving" mà
  /// backend trông đợi từ đầu.
  const SETTLE_MS = 120;

  const baseUrl = $derived(thumbUrl(epoch, hit.index, thumbSize));

  let granted = $state(false);
  let ready = $state(false);
  let failed = $state(false);
  let attempts = $state(0);

  let release: (() => void) | null = null;
  /// Một timer cho cả hai việc — chờ đứng yên và chờ thử lại — vì hai việc
  /// không bao giờ cùng chạy: chưa hỏi lần đầu thì chưa có gì để thử lại.
  let timer: ReturnType<typeof setTimeout> | undefined;

  function request() {
    granted = false;
    release?.();
    release = acquireThumbSlot(() => (granted = true));
  }

  // Bộ ảo hoá tái dùng cùng một instance cho tệp khác khi cuộn — mọi trạng
  // thái tải phải quay về đầu khi URL gốc đổi, nếu không vết "failed" của tệp
  // trước sẽ ẩn oan ảnh của tệp sau.
  $effect(() => {
    void baseUrl;
    ready = false;
    failed = false;
    attempts = 0;
    granted = false;
    clearTimeout(timer);
    timer = setTimeout(request, SETTLE_MS);
    return () => {
      clearTimeout(timer);
      release?.();
      release = null;
    };
  });

  function onLoad() {
    ready = true;
    release?.();
  }

  function onError() {
    release?.();
    granted = false;
    if (attempts >= MAX_RETRY) {
      failed = true;
      return;
    }
    const delay = BACKOFF_MS[attempts];
    timer = setTimeout(() => {
      // Đổi URL (&r=) để lần thử này không nhận lại câu từ chối cũ từ một
      // tầng cache nào đó trên đường đi.
      attempts += 1;
      request();
    }, delay);
  }

  const src = $derived(
    granted && !failed ? (attempts > 0 ? `${baseUrl}&r=${attempts}` : baseUrl) : undefined,
  );
</script>

<!--
  `loading="lazy"` quan trọng ngay cả ở đây: bộ ảo hoá giữ vài dòng dự phòng
  trong DOM mà không nằm trên màn hình, thiếu nó thì mỗi dòng đó đều bị giải mã.
-->
{#if !failed}
  <img
    class="thumb"
    class:grid={grid}
    class:ready
    {src}
    alt=""
    loading="lazy"
    decoding="async"
    onload={onLoad}
    onerror={onError}
  />
{/if}
<!--
  Một hình vẽ, không phải một chữ cái. Bản đầu dùng chữ cái tiếng Anh — V/I/A —
  trong giao diện tiếng Việt mà chính các chip lọc của nó ghi "Video / Ảnh /
  Nhạc". `I` cho Ảnh và `A` cho Nhạc chẳng có nghĩa gì với người đọc, và câu
  đầu tiên người dùng hỏi là mấy chữ đó viết tắt của gì. Hình thì không cần dịch.
-->
<span class="kind {hit.kind}" class:grid={grid} title={KIND_LABEL[hit.kind]} aria-label={KIND_LABEL[hit.kind]}>
  {#if hit.kind === "video"}
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M6.2 4.6v6.8a.5.5 0 0 0 .77.42l5.2-3.4a.5.5 0 0 0 0-.84l-5.2-3.4a.5.5 0 0 0-.77.42Z" />
    </svg>
  {:else if hit.kind === "image"}
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
      <rect x="2.6" y="3.4" width="10.8" height="9.2" rx="1.4" />
      <circle cx="6" cy="6.6" r="1.05" fill="currentColor" stroke="none" />
      <path d="M3.2 11.4 6.4 8.6l2.2 1.9 2.1-2 2.1 2.1" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  {:else}
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
      <path d="M6.1 11.4V4.3l6-1.3v7" stroke-linecap="round" stroke-linejoin="round" />
      <circle cx="4.5" cy="11.6" r="1.7" fill="currentColor" stroke="none" />
      <circle cx="10.5" cy="10.2" r="1.7" fill="currentColor" stroke="none" />
    </svg>
  {/if}
</span>
{#if drive}
  <span class="drive" class:nas={isNetworkDrive(drive, netLetters)} class:grid={grid}>
    {driveLabel(drive)}
  </span>
{/if}
<span class="text" class:grid={grid}>
  <span class="name">{hit.name}</span>
  <span class="dir">{hit.dir}</span>
</span>
<span class="facts" class:grid={grid}>
  {#if formatResolution(hit.width, hit.height)}
    <span class="fact res">{formatResolution(hit.width, hit.height)}</span>
  {/if}
  {#if hit.durationMs}
    <span class="fact">{formatDuration(hit.durationMs)}</span>
  {/if}
  {#if hit.size}
    <span class="fact dim">{formatBytes(hit.size)}</span>
  {/if}
</span>
{#if totalTokens > 0}
  <span class="matched" class:grid={grid} title="Số từ khớp trên tổng số từ trong truy vấn">
    {hit.matched}/{totalTokens}
  </span>
{/if}

<style>
  .thumb {
    flex: 0 0 auto;
    width: 40px;
    height: 30px;
    object-fit: cover;
    background: #12151b;
    border-radius: 4px;
    /* Hiện dần khi ảnh về, thay vì bụp: trong lúc chờ, ô tối làm chỗ giữ.
       Tệp hết lượt thử lại thì thẻ img bị gỡ hẳn (khối {#if} bên trên) và
       nhãn màu theo loại đứng thay — không bao giờ hiện biểu tượng ảnh hỏng. */
    opacity: 0.35;
    transition: opacity 120ms ease-out;
  }
  .thumb.ready { opacity: 1; }

  .kind {
    flex: 0 0 22px;
    height: 22px;
    display: grid;
    place-items: center;
    color: #fff;
    border-radius: 5px;
  }
  .kind svg {
    width: 14px;
    height: 14px;
  }
  .kind.video { background: #5b6cff; }
  .kind.image { background: #23a06a; }
  .kind.audio { background: #c2683a; }

  /* Đường dẫn dài là chuyện thường ở đây; một thanh cuộn ngang cho cả danh
     sách sẽ khiến nó không dùng được. */
  .text { min-width: 0; display: flex; flex-direction: column; }
  .name,
  .dir {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .name { font-size: 14px; }
  .dir { font-size: 11.5px; color: var(--text-dim); }

  .facts {
    flex: 0 0 auto;
    display: flex;
    gap: 5px;
    align-items: center;
  }
  .fact {
    padding: 2px 6px;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
    background: #22262e;
    border-radius: 4px;
  }
  .fact.res { color: #9fd3ff; }
  .fact.dim { opacity: 0.7; }

  .drive {
    flex: 0 0 auto;
    padding: 1px 6px;
    font-size: 10.5px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: #9fd3ff;
    background: #22313f;
    border: 1px solid #2f4a63;
    border-radius: 4px;
  }
  /* Cùng ngôn ngữ màu với chip ổ mạng ở hàng lọc bên trên. */
  .drive.nas {
    color: #ffc978;
    background: #3a2c17;
    border-color: #5c4726;
  }
  /* Trong lưới, nhãn nằm đè góc trên-phải của ảnh — chỗ duy nhất còn trống
     (góc trái đã là nhãn loại, phải-dưới là số từ khớp). */
  .drive.grid {
    position: absolute;
    top: 14px;
    right: 14px;
  }

  .matched {
    flex: 0 0 auto;
    padding: 2px 7px;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
    background: #2c313b;
    border-radius: 999px;
  }

  /* ---- chế độ lưới ---- */
  .thumb.grid {
    flex: 1;
    width: 100%;
    height: auto;
    min-height: 0;
    object-fit: contain;
    border-radius: 6px;
  }
  .kind.grid {
    position: absolute;
    top: 14px;
    left: 14px;
    opacity: 0.9;
  }
  .text.grid { flex: 0 0 auto; }
  .text.grid .name { font-size: 12px; white-space: normal; line-height: 1.3; max-height: 2.6em; }
  .text.grid .dir { display: none; }
  /* Khi cả hai cùng hiện ở lưới, nhãn ổ giữ góc trên-phải và số-từ-khớp
     lùi xuống dưới nó — hai thứ chồng lên nhau thì không đọc được cái nào. */
  .matched.grid { position: absolute; top: 14px; right: 14px; }
  .drive.grid ~ .matched.grid { top: 38px; }
  /* Trong lưới thì bức ảnh mang ý nghĩa; mấy con số sẽ làm chật chỗ. */
  .facts.grid { display: none; }
</style>
