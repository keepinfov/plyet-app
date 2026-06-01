<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Variant = 'filled' | 'text' | 'tonal' | 'danger' | 'surface';

  interface Props extends HTMLButtonAttributes {
    variant?: Variant;
    /** Stretch to fill available width (flex: 1). */
    full?: boolean;
    children?: Snippet;
  }

  let {
    variant = 'filled',
    full = false,
    type = 'button',
    class: className = '',
    children,
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class={`rg-btn tap-btn ${className}`}
  data-variant={variant}
  class:full
  {...rest}
>
  {@render children?.()}
</button>

<style>
  .rg-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    border: none;
    cursor: pointer;
    font-family: inherit;
    font-weight: 600;
    font-size: 0.875rem;
    transition: background 0.15s ease, box-shadow 0.15s ease, transform 0.15s ease, opacity 0.15s ease;
  }

  .rg-btn.full { flex: 1; }
  .rg-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  /* filled — pill, primary */
  .rg-btn[data-variant='filled'] {
    padding: 0.75rem 1.75rem;
    border-radius: 999rem;
    background: var(--rg-primary);
    color: var(--rg-on-primary);
    box-shadow: 0 2px 8px var(--rg-primary-shadow);
  }
  .rg-btn[data-variant='filled']:hover:not(:disabled) {
    box-shadow: 0 4px 14px var(--rg-primary-shadow);
    transform: translateY(-1px);
  }

  /* text — pill, transparent */
  .rg-btn[data-variant='text'] {
    padding: 0.75rem 1.5rem;
    border-radius: 999rem;
    background: transparent;
    color: var(--rg-primary);
  }
  .rg-btn[data-variant='text']:hover:not(:disabled) {
    background: color-mix(in srgb, var(--rg-primary) 8%, transparent);
  }

  /* tonal — blocky, primary tint */
  .rg-btn[data-variant='tonal'] {
    padding: 0.75rem;
    border-radius: 0.75rem;
    font-weight: 700;
    background: color-mix(in srgb, var(--rg-primary) 12%, transparent);
    color: var(--rg-primary);
  }

  /* danger — blocky, danger tint */
  .rg-btn[data-variant='danger'] {
    padding: 0.75rem;
    border-radius: 0.75rem;
    font-weight: 700;
    background: var(--rg-danger-glass);
    color: var(--rg-danger);
  }

  /* surface — blocky, neutral solid */
  .rg-btn[data-variant='surface'] {
    padding: 0.75rem;
    border-radius: 0.75rem;
    font-weight: 700;
    background: var(--rg-surface);
    color: var(--rg-on-surface-variant);
  }
</style>
