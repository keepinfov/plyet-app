<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    title?: string;
    description?: string;
    /** nav = larger primary list row; setting = compact settings row. */
    variant?: 'nav' | 'setting';
    /** Show a trailing chevron (typical for navigation rows). */
    chevron?: boolean;
    leading?: Snippet;
    trailing?: Snippet;
    children?: Snippet;
  }

  let {
    title,
    description,
    variant = 'setting',
    chevron = false,
    type = 'button',
    class: className = '',
    leading,
    trailing,
    children,
    ...rest
  }: Props = $props();
</script>

<button {type} class={`rg-row ${className}`} data-variant={variant} {...rest}>
  {#if leading}
    <span class="rg-row-icon">{@render leading()}</span>
  {/if}
  <span class="rg-row-text">
    {#if children}
      {@render children()}
    {:else}
      {#if title}<span class="rg-row-title">{title}</span>{/if}
      {#if description}<span class="rg-row-desc">{description}</span>{/if}
    {/if}
  </span>
  {#if trailing}{@render trailing()}{/if}
  {#if chevron}
    <svg class="rg-row-chevron" width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M8.59 16.59L13.17 12 8.59 7.41 10 6l6 6-6 6-1.41-1.41z"/></svg>
  {/if}
</button>

<style>
  .rg-row {
    width: 100%;
    display: flex;
    align-items: center;
    border: none;
    background: var(--rg-surface);
    cursor: pointer;
    transition: background 0.15s ease, transform 0.2s var(--rg-spring-easing);
    text-align: left;
    font-family: inherit;
  }

  .rg-row:active {
    transform: scale(0.97);
    transition: background 0.15s ease, transform 0.1s ease-out;
  }

  @media (hover: hover) {
    .rg-row:hover {
      background: color-mix(in srgb, var(--rg-primary) 8%, var(--rg-surface));
    }
  }

  .rg-row:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .rg-row[data-variant='nav'] {
    gap: 0.875rem;
    padding: 0.875rem 1rem;
    border-radius: 1rem;
  }
  .rg-row[data-variant='setting'] {
    gap: 0.75rem;
    padding: 0.75rem;
    border-radius: 0.75rem;
  }

  .rg-row-icon {
    border-radius: 0.875rem;
    background: color-mix(in srgb, var(--rg-primary) 10%, transparent);
    color: var(--rg-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .rg-row[data-variant='nav'] .rg-row-icon { width: 2.5rem; height: 2.5rem; }
  .rg-row[data-variant='setting'] .rg-row-icon { width: 2.25rem; height: 2.25rem; border-radius: 0.75rem; }

  .rg-row-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    min-width: 0;
  }

  .rg-row-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--rg-on-surface);
  }

  .rg-row-desc {
    font-size: 0.75rem;
    color: var(--rg-on-surface-variant);
    opacity: 0.7;
  }

  .rg-row-chevron {
    color: var(--rg-on-surface-variant);
    opacity: 0.4;
    flex-shrink: 0;
  }
</style>
