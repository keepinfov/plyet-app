<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    active?: boolean;
    /** Custom active color (e.g. a category color). Falls back to primary. */
    tint?: string;
    children?: Snippet;
  }

  let {
    active = false,
    tint,
    type = 'button',
    class: className = '',
    style = '',
    children,
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class={`rg-chip tap-btn ${className}`}
  class:active
  style={`${tint != null ? `--rg-chip-tint:${tint};` : ''}${style}`}
  {...rest}
>
  {@render children?.()}
</button>

<style>
  .rg-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.3125rem 0.625rem;
    border-radius: 0.75rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    color: var(--rg-on-surface-variant);
    font-size: 0.6875rem;
    font-family: inherit;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;
    white-space: nowrap;
  }

  .rg-chip.active {
    background: color-mix(in srgb, var(--rg-chip-tint, var(--rg-primary)) 15%, transparent);
    color: var(--rg-chip-tint, var(--rg-primary));
    font-weight: 600;
  }

  .rg-chip :global(svg) {
    width: 0.875rem;
    height: 0.875rem;
    opacity: 0.7;
  }

  .rg-chip.active :global(svg) {
    opacity: 1;
  }
</style>
