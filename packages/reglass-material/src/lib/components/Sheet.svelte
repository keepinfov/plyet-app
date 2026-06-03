<script lang="ts">
  import type { Snippet } from 'svelte';
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { keyboardAvoid } from '../actions/keyboardAvoid.js';

  interface Props {
    show?: boolean;
    onclose?: () => void;
    maxWidth?: string;
    padding?: string;
    /** Allow the sheet body to scroll (caps height at 85dvh). */
    scrollable?: boolean;
    zIndex?: number;
    role?: 'dialog' | 'alertdialog';
    class?: string;
    children?: Snippet;
  }

  let {
    show = false,
    onclose,
    maxWidth = '26rem',
    padding = '1.5rem',
    scrollable = false,
    zIndex = 300,
    role = 'dialog',
    class: className = '',
    children,
  }: Props = $props();

  function close() {
    onclose?.();
  }

  function onOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape' && show) close(); }} />

{#if show}
  <div
    class="rg-overlay"
    style={`z-index:${zIndex}`}
    transition:fade={{ duration: 180 }}
    onclick={onOverlayClick}
    onkeydown={(e) => { if (e.key === 'Escape') close(); }}
    use:keyboardAvoid
    {role}
    aria-modal="true"
    tabindex="-1"
  >
    <div
      class={`rg-sheet ${className}`}
      class:scrollable
      style={`max-width:${maxWidth};padding:${padding}`}
      transition:fly={{ y: 40, duration: 250, easing: cubicOut }}
    >
      {@render children?.()}
    </div>
  </div>
{/if}

<style>
  .rg-overlay {
    position: fixed;
    inset: 0;
    background: var(--rg-overlay-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: env(safe-area-inset-top, 0px) 1rem
      calc(env(safe-area-inset-bottom, 0px) + var(--rg-keyboard-height, 0px));
  }

  .rg-sheet {
    width: 100%;
    background: var(--rg-sheet-glass-bg);
    backdrop-filter: var(--rg-glass-blur);
    -webkit-backdrop-filter: var(--rg-glass-blur);
    border-radius: 1.25rem;
    box-shadow: 0 8px 40px rgba(0, 0, 0, 0.15);
    contain: layout style paint;
    will-change: transform;
  }

  .rg-sheet.scrollable {
    max-height: 85dvh;
    overflow: hidden;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  @media (orientation: landscape) and (max-height: 500px) {
    .rg-sheet.scrollable {
      max-height: 95vh;
    }
  }
</style>
