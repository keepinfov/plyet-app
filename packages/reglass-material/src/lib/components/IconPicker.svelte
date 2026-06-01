<script lang="ts" generics="T extends string">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Icon identifiers to render as options. */
    icons: T[];
    value: T;
    /** Render function for an icon given its key. The kit ships no icons itself. */
    renderIcon: Snippet<[T]>;
    onchange?: (key: T) => void;
  }

  let { icons, value = $bindable(), renderIcon, onchange }: Props = $props();

  function select(key: T) {
    value = key;
    onchange?.(key);
  }
</script>

<div class="rg-icon-picker">
  {#each icons as ic}
    <button class="rg-icon-opt tap-btn" class:selected={value === ic} onclick={() => select(ic)}>
      {@render renderIcon(ic)}
    </button>
  {/each}
</div>

<style>
  .rg-icon-picker {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .rg-icon-opt {
    width: 2.75rem;
    height: 2.75rem;
    border-radius: 0.75rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    color: var(--rg-on-surface-variant);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .rg-icon-opt :global(svg) {
    width: 1.25rem;
    height: 1.25rem;
  }

  .rg-icon-opt.selected {
    background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
    color: var(--rg-primary);
  }

  .rg-icon-opt.selected :global(svg) {
    fill: var(--rg-primary);
  }
</style>
