<script lang="ts">
  import type { SearchHit } from "./search";
  import { mediaUrl, formatBytes } from "./search";

  let {
    hit,
    epoch,
    position,
    total,
    onclose,
    onstep,
    onopen,
  }: {
    hit: SearchHit;
    epoch: number;
    position: number;
    total: number;
    onclose: () => void;
    onstep: (delta: number) => void;
    onopen: () => void;
  } = $props();

  /// Reset per file, not per open.
  ///
  /// Stepping through results keeps the overlay mounted and only swaps `hit`,
  /// so a failure on one file would otherwise stick to every file after it.
  let failed = $state(false);
  let loading = $state(true);
  let src = $derived(mediaUrl(epoch, hit.index));

  /// Whether the stage may receive mouse input yet.
  ///
  /// The gesture that opens the overlay must not also land on what the overlay
  /// puts under the cursor. A double-click on a row mounts a `<video>` right
  /// where the pointer just was, and Chromium reads the tail of that gesture
  /// as its own: the app window went fullscreen the moment the preview opened.
  ///
  /// Suppressing `dblclick` on the video was not enough — it stopped the leak
  /// in some runs and not others, which means the event reaching the player is
  /// not always the one being suppressed. Refusing pointer input outright for
  /// a moment does not depend on guessing which event it is.
  ///
  /// 250 ms was not enough, and the measurement that showed it is worth
  /// keeping: on a query returning 5.000 rows the window still went fullscreen
  /// in **2 of 5 opens**. A wall-clock timer races the input queue, and under
  /// that load the queue was the slower of the two — the timer disarmed the
  /// guard before the leaked event was ever processed.
  ///
  /// So the guard is now two things at once: no pointer input on the stage,
  /// **and** a window-level capture listener that swallows `dblclick`
  /// outright. Capture runs before every other handler, including whatever
  /// Chromium wires inside the media controls — which is why the earlier
  /// attempt at an `ondblclick` on the `<video>` itself only worked sometimes.
  /// 800 ms because the cost of guessing high is nil and guessing low is a
  /// window that fullscreens itself.
  let armed = $state(false);

  /// Refuse the tail of the gesture that opened this overlay.
  ///
  /// Runs in the capture phase on `window`, so it sees the event before the
  /// element under the cursor does.
  function swallowStrayDoubleClick(e: MouseEvent) {
    if (armed) return;
    e.preventDefault();
    e.stopPropagation();
  }

  /// Undo a fullscreen nobody asked for.
  ///
  /// This guards the **outcome**, not the cause, and that is a deliberate
  /// retreat: two attempts at the cause both failed a measurement.
  ///
  /// - `ondblclick` on the `<video>` — the window still went fullscreen in
  ///   2 of 5 opens.
  /// - `pointer-events: none` on the stage *plus* a capture-phase `dblclick`
  ///   swallow on `window` — still 2 of 5.
  ///
  /// With timing instrumentation it turned out to be deterministic rather than
  /// flaky: the window is 1920x1080 within 300 ms of the double-click, every
  /// single time, and `Esc` restores it. So something asks for fullscreen and
  /// it is **not** an event either guard can see. Rather than keep guessing
  /// which one, this watches the only thing that matters — the document going
  /// fullscreen while the overlay is still too young for anyone to have asked
  /// — and reverses it.
  ///
  /// A deliberate fullscreen still works: press the control, and by then the
  /// overlay is armed.
  function undoUnaskedFullscreen() {
    if (armed || !document.fullscreenElement) return;
    document.exitFullscreen().catch(() => {
      // Nothing to do about it, and nothing worth breaking the preview over.
    });
  }

  $effect(() => {
    // Depend on the URL so this re-runs on every step.
    void src;
    failed = false;
    loading = true;
  });

  $effect(() => {
    const t = setTimeout(() => (armed = true), 800);
    return () => clearTimeout(t);
  });

  function onKeydown(e: KeyboardEvent) {
    // The overlay owns the keyboard while it is up. Without stopping
    // propagation the list underneath would move at the same time, and closing
    // would land the user somewhere they never navigated to.
    switch (e.key) {
      case "Escape":
      case " ":
        e.preventDefault();
        e.stopPropagation();
        onclose();
        break;
      case "ArrowDown":
      case "ArrowRight":
        e.preventDefault();
        e.stopPropagation();
        onstep(1);
        break;
      case "ArrowUp":
      case "ArrowLeft":
        e.preventDefault();
        e.stopPropagation();
        onstep(-1);
        break;
      case "Enter":
        e.preventDefault();
        e.stopPropagation();
        onopen();
        break;
    }
  }
</script>

<svelte:window
  on:keydown|capture={onKeydown}
  on:dblclick|capture={swallowStrayDoubleClick}
/>
<svelte:document on:fullscreenchange={undoUnaskedFullscreen} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose} role="presentation">
  <!-- Clicks inside must not close: dragging a video's scrubber ends in a
       click, and closing on it would make the scrubber unusable. -->
  <div class="sheet" onclick={(e) => e.stopPropagation()} role="presentation">
    <header>
      <div class="who">
        <div class="name" title={hit.name}>{hit.name}</div>
        <div class="dir" title={hit.path}>{hit.dir}</div>
      </div>
      <div class="counter">{position} / {total}</div>
      <button class="close" onclick={onclose} title="Đóng (Esc)" aria-label="Đóng">
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
          <path
            d="M6 6l12 12M18 6L6 18"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </header>

    <div class="stage" class:disarmed={!armed}>
      {#if failed}
        <div class="fallback">
          <div class="big">Không xem trước được định dạng này</div>
          <div class="small">
            Cửa sổ của ứng dụng không giải mã được <code>{hit.name.split(".").pop()}</code>.
            Bấm <kbd>Enter</kbd> để mở bằng ứng dụng mặc định.
          </div>
          <button class="open" onclick={onopen}>Mở bằng ứng dụng mặc định</button>
        </div>
      {:else if hit.kind === "image"}
        <img
          {src}
          alt={hit.name}
          onload={() => (loading = false)}
          onerror={() => {
            loading = false;
            failed = true;
          }}
        />
      {:else if hit.kind === "video"}
        <!-- svelte-ignore a11y_media_has_caption -->
        <!--
          `ondblclick` is swallowed on purpose. Opening the preview with a
          double-click puts the video under a cursor that has just been
          double-clicked, and Chromium reads that as its own gesture: the app
          window went fullscreen the instant the overlay appeared. Measured —
          opening by keyboard left the window at 880x620, opening by
          double-click at 1920x1080.

          The cost is losing double-click-to-fullscreen inside the preview.
          The controls still carry a fullscreen button, so nothing is
          unreachable; a window that fullscreens itself on open is worse.
        -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <video
          {src}
          controls
          autoplay
          ondblclick={(e) => e.preventDefault()}
          onloadeddata={() => (loading = false)}
          onerror={() => {
            loading = false;
            failed = true;
          }}
        ></video>
      {:else}
        <div class="audio">
          <div class="disc" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="64" height="64">
              <path
                d="M9 18V5l12-2v13"
                fill="none"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <circle cx="6" cy="18" r="3" fill="currentColor" />
              <circle cx="18" cy="16" r="3" fill="currentColor" />
            </svg>
          </div>
          <audio
            {src}
            controls
            autoplay
            onloadeddata={() => (loading = false)}
            onerror={() => {
              loading = false;
              failed = true;
            }}
          ></audio>
        </div>
      {/if}

      {#if loading && !failed}
        <div class="loading">Đang tải…</div>
      {/if}
    </div>

    <footer>
      <span>{formatBytes(hit.size)}</span>
      {#if hit.width > 0}<span>{hit.width}×{hit.height}</span>{/if}
      <span class="spacer"></span>
      <span class="hint"><kbd>↑</kbd><kbd>↓</kbd> đổi tệp · <kbd>Enter</kbd> mở · <kbd>Esc</kbd> đóng</span>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgba(0, 0, 0, 0.72);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .sheet {
    display: flex;
    flex-direction: column;
    width: min(100%, 1100px);
    height: 100%;
    background: var(--panel, #16181d);
    border: 1px solid var(--line, #2a2e37);
    border-radius: 12px;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--line, #2a2e37);
  }
  .who {
    min-width: 0;
    flex: 1;
  }
  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dir {
    font-size: 12px;
    color: var(--dim, #8b93a3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    /* No `direction: rtl` here, tempting as it is for showing the tail of a
       long path. The bidi algorithm moves trailing punctuation to the front,
       so `D:\` rendered as `:D` for anything sitting in a drive root. The row
       list already shows paths left to right; matching it is both correct and
       consistent. */
  }
  .counter {
    font-size: 12px;
    color: var(--dim, #8b93a3);
    font-variant-numeric: tabular-nums;
  }
  .close {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .close:hover {
    background: var(--hover, #232833);
  }

  .stage.disarmed {
    pointer-events: none;
  }
  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
    display: grid;
    /*
      `minmax(0, 1fr)` rather than the implicit `auto` track, and that is the
      whole fix for the video spilling over the footer.

      An `auto` track is sized by its content, so a percentage height inside it
      has nothing definite to resolve against — the browser drops
      `max-height: 100%` entirely and draws the video at its natural size. A
      1920x1080 clip then rendered 1080 pixels tall inside a stage a few
      hundred tall, and the overflow landed on top of the footer.

      A `1fr` track takes its size from the stage, which flex has already
      sized, so the percentage has a real number to resolve against. The `0`
      minimum is what lets it shrink: grid tracks refuse to go below their
      content's minimum otherwise, which is the same trap one level up.
    */
    grid-template-rows: minmax(0, 1fr);
    grid-template-columns: minmax(0, 1fr);
    place-items: center;
    /* Backstop. Nothing in here may ever paint over the footer again. */
    overflow: hidden;
    background: #000;
    padding: 8px;
  }
  .stage img,
  .stage video {
    width: 100%;
    height: 100%;
    /*
      `scale-down`, not `contain`. Both letterbox to fit, but `contain` also
      blows a small file up to fill the box — a 320x240 clip would become a
      blurry wall. `scale-down` never draws larger than the file actually is.
    */
    object-fit: scale-down;
  }
  .audio {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    color: var(--dim, #8b93a3);
  }
  .audio audio {
    width: min(520px, 70vw);
  }

  .loading {
    position: absolute;
    bottom: 12px;
    right: 14px;
    font-size: 12px;
    color: var(--dim, #8b93a3);
  }
  .fallback {
    text-align: center;
    max-width: 460px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: center;
  }
  .fallback .big {
    font-size: 15px;
    font-weight: 600;
  }
  .fallback .small {
    font-size: 13px;
    color: var(--dim, #8b93a3);
    line-height: 1.5;
  }
  .fallback .open {
    margin-top: 6px;
    padding: 7px 14px;
    border-radius: 8px;
    border: 1px solid var(--line, #2a2e37);
    background: var(--hover, #232833);
    color: inherit;
    cursor: pointer;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-top: 1px solid var(--line, #2a2e37);
    font-size: 12px;
    color: var(--dim, #8b93a3);
  }
  .spacer {
    flex: 1;
  }
  kbd {
    font: inherit;
    padding: 1px 5px;
    border: 1px solid var(--line, #2a2e37);
    border-radius: 4px;
  }
  code {
    font-size: 12px;
  }
</style>
