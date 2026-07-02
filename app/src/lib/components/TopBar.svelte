<script lang="ts">
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { IconButton } from 'reglass-material';
  import BudgetModal from './BudgetModal.svelte';
  import SettingsModal from './SettingsModal.svelte';
  import { store } from '$lib/stores/budget.svelte';
  import { formatMoney } from '$lib/utils';
  import { periodLabel } from '$lib/dates';
  import { hapticLight } from '$lib/haptics';

  const scopeLabel = $derived(
    store.scopeType === 'all' ? 'Весь бюджет'
      : store.scopeType === 'custom' ? (store.currentCustomBudget?.name ?? '')
      : periodLabel(store.scopePeriod ?? '')
  );
</script>

<div class="top-bar" in:fly={{ y: -40, duration: 300, easing: cubicOut }}>
  <div class="title-wrap">
    <span class="title">{store.currentRoot?.name ?? 'Бюджеты'}</span>
    <span class="subtitle">{scopeLabel} · Лимит: {formatMoney(store.effectiveLimit)}</span>
  </div>
  <div class="actions">
    <IconButton onclick={() => { hapticLight(); store.showSettingsModal = true; }} aria-label="Настройки">
      <svg width="1.25rem" height="1.25rem" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.07.62-.07.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/></svg>
    </IconButton>
    <IconButton onclick={() => { hapticLight(); store.showBudgetModal = true; }} aria-label="Сменить бюджет">
      <svg width="1.375rem" height="1.375rem" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M3 13h2v-2H3v2zm0 4h2v-2H3v2zm0-8h2V7H3v2zm4 4h14v-2H7v2zm0 4h14v-2H7v2zM7 7v2h14V7H7z"/></svg>
    </IconButton>
  </div>
</div>

<BudgetModal show={store.showBudgetModal} onclose={() => store.showBudgetModal = false} />
<SettingsModal bind:show={store.showSettingsModal} />

<style>
  .top-bar {
    position: fixed;
    top: calc(var(--rg-safe-top) + 0.5rem);
    left: 50%;
    transform: translateX(-50%);
    width: calc(100% - 2.5rem);
    border-radius: 1rem;
    padding: 0.6875rem 1.125rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--rg-topbar-bg);
    backdrop-filter: var(--rg-glass-blur-light);
    -webkit-backdrop-filter: var(--rg-glass-blur-light);
    box-shadow: var(--rg-topbar-shadow);
    z-index: 200;
    user-select: none;
    contain: layout style paint;
  }

  .title-wrap {
    display: flex;
    flex-direction: column;
    pointer-events: none;
    min-width: 0;
    flex: 1;
  }

  .title {
    font-size: 1.125rem;
    font-weight: 900;
    letter-spacing: -0.03125rem;
    color: var(--rg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .subtitle {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--rg-on-surface-variant);
    opacity: 0.7;
    margin-top: -0.125rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  /* Constrain top bar width on desktop */
  @media (min-width: 768px) {
    .top-bar {
      max-width: 640px;
    }
  }
</style>
