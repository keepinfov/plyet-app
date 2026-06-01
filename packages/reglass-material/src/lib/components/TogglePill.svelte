<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    active?: boolean;
    label?: string;
    children?: Snippet;
  }

  let { active = false, label, children }: Props = $props();
</script>

<span class="rg-toggle-pill" class:active>
  <span class="rg-toggle-label">
    {#if children}{@render children()}{:else}{label}{/if}
  </span>
</span>

<style>
  .rg-toggle-pill {
    display: inline-flex;
    align-items: center;
    padding: 0.3125rem 0.75rem;
    border-radius: 999rem;
    background: color-mix(in srgb, var(--rg-on-surface) 8%, transparent);
    border: none;
    transition: background 0.2s ease;
  }

  .rg-toggle-pill.active {
    background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
  }

  .rg-toggle-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--rg-on-surface-variant);
  }

  .rg-toggle-pill.active .rg-toggle-label {
    color: var(--rg-primary);
  }
</style>
