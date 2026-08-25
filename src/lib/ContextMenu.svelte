<script lang="ts">
  export interface MenuItem {
    label: string;
    /** One of the icon keys below. */
    icon: "open" | "folder" | "copy" | "eye";
    shortcut?: string;
    action: () => void;
  }

  let {
    x,
    y,
    items,
    onclose,
  }: { x: number; y: number; items: MenuItem[]; onclose: () => void } = $props();

  // Measured after render so the menu can be nudged back inside the window.
  let width = $state(0);
  let height = $state(0);

  const PAD = 6;
  // A menu opened near the right or bottom edge would otherwise be clipped, and
  // the item under the cursor is the one most likely to be cut off.
  const left = $derived(
    Math.max(PAD, Math.min(x, window.innerWidth - width - PAD)),
  );
  const top = $derived(
    Math.max(PAD, Math.min(y, window.innerHeight - height - PAD)),
  );

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<!--
  A full-screen catcher rather than a document-level click listener: it closes
  the menu on any click or right-click outside, and swallows that click so the
  result underneath is not also selected.
-->
<div
  class="catcher"
  role="presentation"
  onmousedown={(e) => {
    e.preventDefault();
    e.stopPropagation();
    onclose();
  }}
  oncontextmenu={(e) => {
    e.preventDefault();
    onclose();
  }}
></div>

<div
  class="menu"
  role="menu"
  tabindex="-1"
  bind:clientWidth={width}
  bind:clientHeight={height}
  style="left: {left}px; top: {top}px"
>
  {#each items as item (item.label)}
    <button
      class="item"
      role="menuitem"
      onclick={() => {
        item.action();
        onclose();
      }}
    >
      <span class="icon" aria-hidden="true">
        {#if item.icon === "folder"}
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
            <path d="M1.8 4.2h4l1.2 1.5h7.2v6.6a1 1 0 0 1-1 1H2.8a1 1 0 0 1-1-1V4.2Z" />
            <path d="M1.8 6.4h12.4" />
          </svg>
        {:else if item.icon === "eye"}
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
            <path d="M1.5 8s2.4-4 6.5-4 6.5 4 6.5 4-2.4 4-6.5 4-6.5-4-6.5-4Z" />
            <circle cx="8" cy="8" r="1.8" />
          </svg>
        {:else if item.icon === "open"}
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
            <path d="M9.5 2.5H13V6" />
            <path d="M13 2.5 7.5 8" />
            <path d="M12.5 9.5v3a1 1 0 0 1-1 1h-8a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1h3" />
          </svg>
        {:else}
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
            <rect x="5.5" y="5.5" width="8" height="8" rx="1.2" />
            <path d="M10.5 5.5v-2a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2" />
          </svg>
        {/if}
      </span>
      <span class="label">{item.label}</span>
      {#if item.shortcut}
        <span class="shortcut">{item.shortcut}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .catcher {
    position: fixed;
    inset: 0;
    z-index: 50;
  }

  /* Windows 11 shape — generous corner radius, hairline border, soft shadow —
     but in the app's own dark palette rather than the system light theme, so
     it does not look pasted in from another application. */
  .menu {
    position: fixed;
    z-index: 51;
    min-width: 232px;
    padding: 4px;
    background: #2b2f38;
    border: 1px solid #3a3f4a;
    border-radius: 8px;
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.45),
      0 1px 2px rgba(0, 0, 0, 0.3);
    outline: none;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 10px;
    font: inherit;
    font-size: 13px;
    color: var(--text);
    text-align: left;
    background: none;
    border: none;
    border-radius: 5px;
    cursor: default;
  }
  .item:hover,
  .item:focus-visible {
    background: #3a4150;
    outline: none;
  }

  .icon {
    display: flex;
    flex: 0 0 16px;
    color: var(--text-dim);
  }
  .icon svg {
    width: 16px;
    height: 16px;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .item:hover .icon {
    color: var(--text);
  }

  .label {
    flex: 1;
    white-space: nowrap;
  }

  .shortcut {
    flex: 0 0 auto;
    font-size: 11.5px;
    color: var(--text-dim);
  }
</style>
