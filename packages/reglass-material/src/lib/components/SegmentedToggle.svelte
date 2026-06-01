<script lang="ts" generics="T extends string">
  interface Item {
    key: T;
    label: string;
  }

  interface Props {
    items: Item[];
    value: T;
    onchange?: (key: T) => void;
  }

  let { items, value = $bindable(), onchange }: Props = $props();

  const activeIdx = $derived(items.findIndex((t) => t.key === value));

  let containerEl: HTMLDivElement | null = $state(null);
  const tabEls: HTMLButtonElement[] = [];
  let pillLeft = $state(0);
  let pillWidth = $state(0);
  let resizeTick = $state(0);

  $effect(() => {
    const el = containerEl;
    if (!el) return;
    const ro = new ResizeObserver(() => resizeTick++);
    ro.observe(el);
    return () => ro.disconnect();
  });

  $effect(() => {
    void value;
    void resizeTick;
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        const el = tabEls[activeIdx];
        if (el && containerEl) {
          pillLeft = el.offsetLeft;
          pillWidth = el.offsetWidth;
        }
      })
    );
  });

  function select(key: T) {
    value = key;
    onchange?.(key);
  }
</script>

<div class="rg-seg" bind:this={containerEl}>
  <div class="rg-seg-pill" style="left: {pillLeft}px; width: {pillWidth}px;"></div>
  {#each items as item, i}
    <button
      bind:this={tabEls[i]}
      class="rg-seg-tab tap-dim"
      class:active={value === item.key}
      data-label={item.label}
      onclick={() => select(item.key)}
    >
      {item.label}
    </button>
  {/each}
</div>

<style>
  .rg-seg {
    flex: 1;
    display: flex;
    gap: 0;
    background: var(--rg-surface-2);
    border-radius: 0.75rem;
    padding: 0.1875rem;
    position: relative;
    overflow: hidden;
    height: 2.75rem;
  }

  .rg-seg-pill {
    position: absolute;
    top: 0.1875rem;
    height: calc(100% - 0.375rem);
    background: var(--rg-pill-bg);
    border-radius: 0.5625rem;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.12);
    transition: left 0.12s var(--rg-spring-easing), width 0.12s var(--rg-spring-easing);
    will-change: left, width;
    pointer-events: none;
    z-index: 0;
  }

  .rg-seg-tab {
    flex: 1;
    padding: 0 1rem;
    text-align: center;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--rg-on-surface-variant);
    cursor: pointer;
    border: none;
    background: transparent;
    font-family: inherit;
    transition: color 0.2s ease, font-weight 0.2s ease;
    border-radius: 0.5625rem;
    position: relative;
    z-index: 1;
  }

  /* Reserve width for the bold (active) label so the pill doesn't reflow. */
  .rg-seg-tab::after {
    content: attr(data-label);
    font-weight: 700;
    display: block;
    height: 0;
    overflow: hidden;
    visibility: hidden;
  }

  .rg-seg-tab.active {
    color: var(--rg-primary);
    font-weight: 700;
  }
</style>
