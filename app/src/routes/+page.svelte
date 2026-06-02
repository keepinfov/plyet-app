<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { onBackButtonPress } from '@tauri-apps/api/app';
  import { invoke } from '@tauri-apps/api/core';
  import TopBar from '$lib/components/TopBar.svelte';
  import ChartSection from '$lib/components/ChartSection.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import CardsSection from '$lib/components/CardsSection.svelte';
  import RegularScreen from '$lib/components/RegularScreen.svelte';
  import { Fab, SegmentedToggle, Sheet } from 'reglass-material';
  import { hapticLight } from '$lib/haptics';
  import AddModal from '$lib/components/AddModal.svelte';
  import RecurringModal from '$lib/components/RecurringModal.svelte';
  import ProductModal from '$lib/components/ProductModal.svelte';
  import Snackbar from '$lib/components/Snackbar.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import LogsPanel from '$lib/components/LogsPanel.svelte';
  import DebugPanel from '$lib/components/DebugPanel.svelte';
  import { store } from '$lib/stores/budget.svelte';
  import type { Item } from '$lib/types';

  // On-screen error diagnostics — only in dev/debug builds
  let errorMsg = $state('');
  if (typeof window !== 'undefined' && import.meta.env.DEV) {
    window.onerror = (msg) => { errorMsg = String(msg); };
    window.onunhandledrejection = (e) => { errorMsg = String(e.reason); };
  }

  let showAdd = $state(false);
  let editData = $state<Item | null>(null);
  let showChooser = $state(false);
  let backTimer: ReturnType<typeof setTimeout> | null = null;
  let backListener: Awaited<ReturnType<typeof onBackButtonPress>> | undefined;

  const screens = [
    { key: 'feed' as const, label: 'Лента' },
    { key: 'regular' as const, label: 'Регулярные' },
  ];

  function handleBack() {
    if (store.confirmShow) { store.closeConfirm(); return; }
    if (showChooser) { showChooser = false; return; }
    if (store.showProductModal) { store.closeProductModal(); return; }
    if (store.showRecurringModal) { store.closeRecurringModal(); return; }
    if (showAdd) { showAdd = false; editData = null; return; }
    if (store.showSettingsModal) { store.showSettingsModal = false; return; }
    if (store.showBudgetModal) { store.showBudgetModal = false; return; }
    if (store.showDebug) { store.showDebug = false; return; }
    if (store.showLogs) { store.showLogs = false; return; }
    if (store.currentScreen !== 'feed') { store.currentScreen = 'feed'; return; }
    if (backTimer) {
      clearTimeout(backTimer);
      backTimer = null;
      invoke('request_exit');
    } else {
      store.showSnackbar('Нажмите ещё раз для выхода');
      backTimer = setTimeout(() => backTimer = null, 2000);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { e.preventDefault(); handleBack(); }
  }

  onMount(async () => {
    store.load();
    backListener = await onBackButtonPress(() => handleBack());
  });

  onDestroy(() => {
    backListener?.unregister();
    if (backTimer) clearTimeout(backTimer);
  });

  function openAdd() {
    hapticLight();
    editData = null;
    showAdd = true;
  }

  function onFabClick() {
    if (store.currentScreen === 'regular') {
      hapticLight();
      showChooser = true;
    } else {
      openAdd();
    }
  }

  function chooseRecurring() {
    showChooser = false;
    store.openRecurringModal();
  }

  function chooseProduct(kind: 'deposit' | 'loan' | 'mortgage') {
    showChooser = false;
    store.openProductModal(kind);
  }

  function onEdit(item: Item) {
    editData = item;
    showAdd = true;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if errorMsg}
  <div class="error-overlay">{errorMsg}</div>
{/if}
<div class="app-container">
  {#if store.loading}
    <div class="loading-screen">
      <div class="loading-spinner"></div>
    </div>
  {:else}
    <TopBar />
    <div class="scroll-area">
      {#if store.currentScreen === 'regular'}
        <RegularScreen />
      {:else}
        <ChartSection />
        <Tabs />
        <CardsSection {onEdit} />
      {/if}
    </div>
    <div class="screen-switch">
      <SegmentedToggle
        items={screens}
        value={store.currentScreen}
        onchange={(key) => { hapticLight(); store.currentScreen = key; }}
      />
    </div>
    <Fab onclick={onFabClick} class="fab-bar" aria-label="Добавить">
      <svg width="1.75rem" height="1.75rem" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 11h-6V5a1 1 0 0 0-2 0v6H5a1 1 0 0 0 0 2h6v6a1 1 0 0 0 2 0v-6h6a1 1 0 0 0 0-2z"/></svg>
    </Fab>
    <AddModal bind:show={showAdd} bind:editData />
    <RecurringModal />
    <ProductModal />
    <Sheet show={showChooser} onclose={() => showChooser = false}>
      <div class="chooser-title">Добавить</div>
      <div class="chooser-list">
        <button class="chooser-row tap-btn" onclick={chooseRecurring}>
          <span class="chooser-ic" style="color: var(--rg-primary)">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M12 6v3l4-4-4-4v3c-4.42 0-8 3.58-8 8 0 1.57.46 3.03 1.24 4.26L6.7 14.8c-.45-.83-.7-1.79-.7-2.8 0-3.31 2.69-6 6-6zm6.76 1.74L17.3 9.2c.44.84.7 1.79.7 2.8 0 3.31-2.69 6-6 6v-3l-4 4 4 4v-3c4.42 0 8-3.58 8-8 0-1.57-.46-3.03-1.24-4.26z"/></svg>
          </span>
          <span class="chooser-text">
            <span class="chooser-name">Регулярный платёж</span>
            <span class="chooser-sub">Подписка, зарплата, аренда</span>
          </span>
        </button>
        <button class="chooser-row tap-btn" onclick={() => chooseProduct('deposit')}>
          <span class="chooser-ic" style="color: var(--rg-price-pos)">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M11.8 10.9c-2.27-.59-3-1.2-3-2.15 0-1.09 1.01-1.85 2.7-1.85 1.78 0 2.44.85 2.5 2.1h2.21c-.07-1.72-1.12-3.3-3.21-3.81V3h-3v2.16c-1.94.42-3.5 1.68-3.5 3.61 0 2.31 1.91 3.46 4.7 4.13 2.5.6 3 1.48 3 2.41 0 .69-.49 1.79-2.7 1.79-2.06 0-2.87-.92-2.98-2.1h-2.2c.12 2.19 1.76 3.42 3.68 3.83V21h3v-2.15c1.95-.37 3.5-1.5 3.5-3.55 0-2.84-2.43-3.81-4.7-4.4z"/></svg>
          </span>
          <span class="chooser-text">
            <span class="chooser-name">Вклад, кредит или ипотека</span>
            <span class="chooser-sub">Проценты, аннуитет, досрочное погашение</span>
          </span>
        </button>
      </div>
    </Sheet>
    <Snackbar />
    <ConfirmDialog />
    <LogsPanel />
    <DebugPanel />
  {/if}
</div>

<style>
  .error-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 9999;
    background: #D93025;
    color: white;
    padding: 1rem;
    font-size: 12px;
    word-break: break-all;
    pointer-events: none;
  }

  .app-container {
    height: 100dvh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    background: var(--rg-surface);

  }

  .loading-screen {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .loading-spinner {
    width: 2.5rem;
    height: 2.5rem;
    border: 3px solid var(--rg-outline);
    border-top-color: var(--rg-primary);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Floating bottom tab bar — above content like the FAB, docked left of it. */
  .screen-switch {
    position: fixed;
    left: 1rem;
    right: calc(5.75rem + env(safe-area-inset-right, 0px));
    bottom: calc(env(safe-area-inset-bottom, 0px) + 2.625rem);
    display: flex;
    z-index: 200;
    border-radius: 0.75rem;
    box-shadow: 0 0.5rem 1.5rem rgba(0, 0, 0, 0.22);
    transition: opacity 0.2s ease, transform 0.2s ease;
  }

  /* Keep the floating tab bar out of the way while the keyboard is up. */
  :global([data-keyboard-open]) .screen-switch {
    opacity: 0;
    transform: translateY(140%) translateZ(0);
    pointer-events: none;
  }

  .scroll-area {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: contain;
    -webkit-overflow-scrolling: touch;
    padding-top: calc(env(safe-area-inset-top, 0px) + 4.25rem);
    padding-bottom: calc(env(safe-area-inset-bottom, 0px) + 6.5rem);
  }

  .chooser-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    letter-spacing: -0.02em;
    margin-bottom: 1rem;
  }

  .chooser-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .chooser-row {
    display: flex;
    align-items: center;
    gap: 0.875rem;
    padding: 0.875rem;
    border-radius: 1rem;
    border: none;
    background: var(--rg-surface-2);
    cursor: pointer;
    font-family: inherit;
    text-align: left;
    transition: transform 0.15s ease;
  }

  .chooser-row:active {
    transform: scale(0.98);
  }

  .chooser-ic {
    width: 2.75rem;
    height: 2.75rem;
    border-radius: 0.875rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: color-mix(in srgb, currentColor 14%, transparent);
  }

  .chooser-text {
    display: flex;
    flex-direction: column;
  }

  .chooser-name {
    font-size: 0.9375rem;
    font-weight: 700;
    color: var(--rg-on-surface);
  }

  .chooser-sub {
    font-size: 0.75rem;
    color: var(--rg-on-surface-variant);
    margin-top: 0.125rem;
  }

  /* Mobile: FAB becomes a rectangular bar aligned with the bottom tab bar */
  @media (max-width: 767px) {
    :global(.rg-fab.fab-bar) {
      height: 2.75rem;
      width: 4rem;
      bottom: calc(env(safe-area-inset-bottom, 0px) + 2.625rem);
      right: calc(1rem + env(safe-area-inset-right, 0px));
      border-radius: 0.75rem;
    }
  }

  /* Center content on wide screens */
  @media (min-width: 768px) {
    .scroll-area {
      max-width: 640px;
      margin-left: auto;
      margin-right: auto;
    }

    .screen-switch {
      left: 50%;
      right: auto;
      transform: translateX(-50%);
      width: min(20rem, calc(100% - 2rem));
    }
  }
</style>
