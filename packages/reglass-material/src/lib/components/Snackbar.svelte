<script lang="ts">
  import { fly } from 'svelte/transition';

  interface Props {
    message?: string;
    show?: boolean;
  }

  let { message = '', show = false }: Props = $props();

  let snackbarEl = $state<HTMLDivElement | null>(null);
  let currentWidth = $state<number | null>(null);

  // Lock the pill width across message changes so it animates smoothly.
  $effect(() => {
    void message;
    if (snackbarEl) {
      snackbarEl.style.width = 'max-content';
      const w = snackbarEl.offsetWidth;
      snackbarEl.style.width = `${currentWidth ?? w}px`;
      requestAnimationFrame(() => {
        currentWidth = w;
        if (snackbarEl) snackbarEl.style.width = `${w}px`;
      });
    }
  });
</script>

{#if show}
  <div class="rg-snackbar" bind:this={snackbarEl} transition:fly={{ y: -12, duration: 200 }}>
    {message}
  </div>
{/if}

<style>
  .rg-snackbar {
    position: fixed;
    top: calc(env(safe-area-inset-top, 0px) + 5rem);
    left: 50%;
    transform: translateX(-50%);
    background: var(--rg-snackbar-glass-bg);
    color: var(--rg-snackbar-text);
    padding: 0.625rem 1.25rem;
    border-radius: 999rem;
    font-size: 0.875rem;
    font-weight: 600;
    backdrop-filter: var(--rg-glass-blur-light);
    -webkit-backdrop-filter: var(--rg-glass-blur-light);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    z-index: 400;
    max-width: calc(100% - 1rem);
    width: max-content;
    text-align: center;
    white-space: normal;
    overflow-wrap: anywhere;
    pointer-events: none;
    transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }
</style>
