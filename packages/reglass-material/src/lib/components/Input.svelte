<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends HTMLInputAttributes {
    value?: string;
    /** filled = solid M3 field (default); bare = borderless/transparent (for toolbars). */
    variant?: 'filled' | 'bare';
    element?: HTMLInputElement | null;
  }

  let {
    value = $bindable(''),
    variant = 'filled',
    class: className = '',
    element = $bindable(null),
    ...rest
  }: Props = $props();
</script>

<input
  bind:this={element}
  bind:value
  class={`rg-input ${className}`}
  data-variant={variant}
  {...rest}
/>

<style>
  .rg-input {
    width: 100%;
    color: var(--rg-on-surface);
    font-family: inherit;
    outline: none;
    border: none;
    min-width: 0;
    scroll-margin-bottom: 6rem;
  }

  .rg-input[data-variant='filled'] {
    padding: 0.75rem 1rem;
    border-radius: 0.75rem;
    background: var(--rg-surface);
    font-size: 1rem;
    transition: box-shadow 0.2s ease, background 0.2s ease;
  }

  .rg-input[data-variant='filled']:focus {
    outline: 2px solid color-mix(in srgb, var(--rg-primary) 40%, transparent);
    outline-offset: -1px;
  }

  .rg-input[data-variant='bare'] {
    background: transparent;
    font-size: 0.875rem;
  }

  .rg-input::placeholder {
    color: var(--rg-on-surface-variant);
    opacity: 0.45;
  }
</style>
