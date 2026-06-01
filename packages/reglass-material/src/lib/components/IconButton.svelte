<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Variant = 'ghost' | 'surface' | 'tonal';
  type Shape = 'square' | 'round';

  interface Props extends HTMLButtonAttributes {
    /** ghost = transparent (default), surface = solid M3, tonal = primary tint. */
    variant?: Variant;
    shape?: Shape;
    size?: string;
    children?: Snippet;
  }

  let {
    variant = 'ghost',
    shape = 'square',
    size = '2.75rem',
    type = 'button',
    class: className = '',
    style = '',
    children,
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class={`rg-icon-btn tap-btn ${className}`}
  data-variant={variant}
  data-shape={shape}
  style={`--rg-ib-size:${size};${style}`}
  {...rest}
>
  {@render children?.()}
</button>

<style>
  .rg-icon-btn {
    width: var(--rg-ib-size, 2.75rem);
    height: var(--rg-ib-size, 2.75rem);
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--rg-on-surface-variant);
    background: transparent;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .rg-icon-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .rg-icon-btn[data-shape='square'] { border-radius: 0.75rem; }
  .rg-icon-btn[data-shape='round'] { border-radius: 50%; }

  .rg-icon-btn[data-variant='surface'] {
    background: var(--rg-surface);
    color: var(--rg-on-surface);
  }
  .rg-icon-btn[data-variant='tonal'] {
    background: color-mix(in srgb, var(--rg-primary) 12%, transparent);
    color: var(--rg-primary);
  }

  @media (hover: hover) {
    .rg-icon-btn[data-variant='ghost']:hover:not(:disabled) {
      background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
      color: var(--rg-primary);
    }
  }
</style>
