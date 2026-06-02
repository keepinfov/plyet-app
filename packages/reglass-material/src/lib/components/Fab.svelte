<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    children?: Snippet;
  }

  let { type = 'button', class: className = '', children, ...rest }: Props = $props();
</script>

<button {type} class={`rg-fab ${className}`} {...rest}>
  {@render children?.()}
</button>

<style>
  .rg-fab {
    position: fixed;
    bottom: calc(2rem + env(safe-area-inset-bottom, 0px));
    right: calc(2rem + env(safe-area-inset-right, 0px));
    width: 4rem;
    height: 4rem;
    background: var(--rg-fab-bg);
    backdrop-filter: var(--rg-glass-blur);
    -webkit-backdrop-filter: var(--rg-glass-blur);
    color: var(--rg-on-primary);
    border-radius: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: none;
    border: none;
    cursor: pointer;
    transition: transform 0.2s var(--rg-spring-easing), background 0.2s ease;
    z-index: 101;
  }

  .rg-fab:hover {
    background: var(--rg-fab-bg-hover);
    transform: scale(1.06);
  }

  .rg-fab:active {
    transform: scale(0.92);
    transition: transform 0.1s ease-out;
  }

  /* When the soft keyboard is up, drop the FAB out of the way so it can't
     cover a focused input. translateZ(0) keeps it stable on iOS WKWebView. */
  :global([data-keyboard-open]) .rg-fab {
    opacity: 0;
    transform: translateY(140%) translateZ(0);
    pointer-events: none;
  }
</style>
