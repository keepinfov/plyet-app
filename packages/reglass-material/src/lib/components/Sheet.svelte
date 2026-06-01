<script lang="ts">
  import type { Snippet } from 'svelte';
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

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

  let overlayEl = $state<HTMLElement | null>(null);

  function close() {
    onclose?.();
  }

  function onOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
  }

  /**
   * Keyboard avoidance: when an input/textarea inside the sheet gains focus and the
   * on-screen keyboard shrinks the visual viewport, pin the overlay to the visible
   * region and scroll the focused field into view. Cleans up on blur.
   */
  function onFocusIn(e: FocusEvent) {
    const target = e.target as HTMLElement;
    if (!(target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return;
    const overlay = overlayEl;
    if (!overlay) return;

    const vv = window.visualViewport;
    if (!vv) {
      setTimeout(() => target.scrollIntoView({ block: 'center', behavior: 'smooth' }), 400);
      return;
    }

    const onResize = () => {
      overlay.style.height = `${vv.height}px`;
      overlay.style.top = `${vv.offsetTop}px`;
      overlay.style.bottom = 'auto';
      requestAnimationFrame(() => target.scrollIntoView({ block: 'center', behavior: 'smooth' }));
    };

    vv.addEventListener('resize', onResize);
    vv.addEventListener('scroll', onResize);

    const onBlur = () => {
      target.removeEventListener('blur', onBlur);
      vv.removeEventListener('resize', onResize);
      vv.removeEventListener('scroll', onResize);
      overlay.style.height = '';
      overlay.style.top = '';
      overlay.style.bottom = '';
    };
    target.addEventListener('blur', onBlur);
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape' && show) close(); }} />

{#if show}
  <div
    bind:this={overlayEl}
    class="rg-overlay"
    style={`z-index:${zIndex}`}
    transition:fade={{ duration: 180 }}
    onclick={onOverlayClick}
    onkeydown={(e) => { if (e.key === 'Escape') close(); }}
    {role}
    aria-modal="true"
    tabindex="-1"
  >
    <div
      class={`rg-sheet ${className}`}
      class:scrollable
      style={`max-width:${maxWidth};padding:${padding}`}
      transition:fly={{ y: 40, duration: 250, easing: cubicOut }}
      onfocusin={onFocusIn}
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
    padding: env(safe-area-inset-top, 0px) 1rem env(safe-area-inset-bottom, 0px);
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
  }

  @media (orientation: landscape) and (max-height: 500px) {
    .rg-sheet.scrollable {
      max-height: 95vh;
    }
  }
</style>
