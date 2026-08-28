<script lang="ts">
  import { formatCount, formatWhen, type NetScanMark } from "./search";

  let {
    mark,
    onconfirm,
    oncancel,
  }: {
    /// Lần quét ổ mạng gần nhất; `null` khi chưa từng quét xong lần nào.
    mark: NetScanMark | null;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  /// "khoảng 4 phút" dễ hình dung hơn "271.5 giây" — người dùng đang quyết
  /// định có bỏ ra chừng ấy thời gian hay không, nên đơn vị phải là thứ họ
  /// nghĩ bằng.
  function humanDuration(seconds: number): string {
    if (seconds < 90) return `khoảng ${Math.round(seconds)} giây`;
    return `khoảng ${Math.round(seconds / 60)} phút`;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      oncancel();
    }
  }
</script>

<svelte:window on:keydown|capture={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={oncancel} role="presentation">
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Quét lại ổ mạng"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    {#if mark}
      <div class="title">Quét lại ổ mạng?</div>
      <!--
        Ba con số, vì cả ba đều cần cho quyết định: quét lần trước lúc nào (có
        đáng quét lại chưa), ra bao nhiêu (lần trước có việc gì không), và mất
        bao lâu (mình có chờ nổi không).
      -->
      <div class="facts">
        Lần trước quét lúc <b>{formatWhen(mark.atUnix)}</b>,
        tìm được <b>{formatCount(mark.files)}</b> tệp
        trên {mark.drives === 1 ? "một ổ mạng" : `${mark.drives} ổ mạng`},
        mất <b>{humanDuration(mark.seconds)}</b>.
      </div>
      <div class="note">
        Dữ liệu ổ mạng đang có vẫn dùng được — quét lại chỉ cần khi bạn vừa
        thêm hoặc đổi tệp trên đó.
      </div>
    {:else}
      <div class="title">Quét cả ổ mạng?</div>
      <div class="note">
        Ổ mạng duyệt qua đường truyền nên lâu hơn ổ trong máy nhiều lần — thường
        mất vài phút. Ổ trong máy sẽ được quét lại trước, rồi tới ổ mạng.
      </div>
    {/if}

    <div class="actions">
      <button class="go" onclick={onconfirm}>
        {mark ? "Quét lại" : "Quét"}
      </button>
      <button class="no" onclick={oncancel}>Không</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 46;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .dialog {
    width: min(100%, 460px);
    display: flex;
    flex-direction: column;
    gap: 11px;
    padding: 18px 20px;
    background: var(--bg-raised, #1e2128);
    border: 1px solid var(--border, #2c3038);
    border-radius: 12px;
    color: var(--text, #e6e8ec);
  }
  .title {
    font-size: 15px;
    font-weight: 600;
  }
  .facts {
    font-size: 13px;
    line-height: 1.6;
    padding: 10px 12px;
    background: #10151d;
    border: 1px solid var(--border, #2c3038);
    border-radius: 8px;
  }
  .facts b {
    color: #9fd3ff;
    font-weight: 600;
  }
  .note {
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--text-dim, #8b919c);
  }
  .actions {
    display: flex;
    gap: 10px;
    margin-top: 2px;
  }
  .go {
    font: inherit;
    padding: 7px 18px;
    font-weight: 600;
    color: #0d1b2a;
    background: #7fb8ff;
    border: none;
    border-radius: 8px;
    cursor: pointer;
  }
  .go:hover {
    background: #9ac8ff;
  }
  .no {
    font: inherit;
    font-size: 13px;
    color: inherit;
    background: none;
    border: 1px solid currentColor;
    border-radius: 8px;
    padding: 7px 16px;
    cursor: default;
    opacity: 0.8;
  }
  .no:hover {
    opacity: 1;
  }
</style>
