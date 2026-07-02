<script lang="ts">
  import ExpenseCard from './ExpenseCard.svelte';
  import { store } from '$lib/stores/budget.svelte';
  import { flip } from 'svelte/animate';
  import { slide, fade } from 'svelte/transition';
  import type { Item } from '$lib/types';

  interface Props {
    onEdit: (item: Item) => void;
  }

  const { onEdit }: Props = $props();

  // Suppress entry/exit animation on tab/filter changes (only animate on add/delete)
  let suppressOutro = $state(false);
  let prevTab = $state(store.currentTab);
  let prevCatFilter = $state(store.categoryFilter);
  let prevDateRange = $state(store.dateRange);

  $effect(() => {
    const tabChanged = store.currentTab !== prevTab;
    const catChanged = store.categoryFilter !== prevCatFilter;
    const dateChanged = store.dateRange !== prevDateRange;
    prevTab = store.currentTab;
    prevCatFilter = store.categoryFilter;
    prevDateRange = store.dateRange;
    if (tabChanged || catChanged || dateChanged) {
      suppressOutro = true;
      setTimeout(() => { suppressOutro = false; }, 50);
    }
  });

  function introFn(node: Element) {
    if (suppressOutro) return { duration: 0 };
    return slide(node, { duration: 250 });
  }

  function outroFn(_node: Element) {
    return { duration: 0 };
  }

  // Distinguish a truly empty budget from one where filters/search hid everything.
  const budgetHasItems = $derived(store.scopeItems.length > 0);
</script>

<div class="cards-section">
  <div class="cards-list">
    {#if store.filteredItems.length === 0}
      <div class="empty-state" transition:fade={{ duration: 200 }}>
        <svg width="4.5rem" height="4.5rem" viewBox="0 0 24 24" fill="var(--rg-outline)" opacity="0.4">
          <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-7 3c1.93 0 3.5 1.57 3.5 3.5S13.93 13 12 13s-3.5-1.57-3.5-3.5S10.07 6 12 6zm7 13H5v-.23c0-.62.28-1.2.76-1.58C7.47 15.82 9.64 15 12 15s4.53.82 6.24 2.19c.48.38.76.97.76 1.58V19z"/>
        </svg>
        {#if budgetHasItems}
          <p>Ничего не найдено</p>
          <span class="empty-hint">Попробуйте изменить поиск или фильтры</span>
        {:else}
          <p>Нет записей</p>
          <span class="empty-hint">Нажмите +, чтобы добавить первую запись</span>
        {/if}
      </div>
    {:else}
      {#each store.filteredItems as item, idx (item.id)}
        <div animate:flip={{ duration: 250 }} in:introFn out:outroFn style="--card-index: {idx}">
          <ExpenseCard {item} {onEdit} />
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .cards-section {
    padding: 0.75rem 1rem 1rem;
    contain: layout style;
  }

  .cards-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .empty-state {
    text-align: center;
    padding: 3rem 1.25rem;
    color: var(--rg-on-surface-variant);
  }

  .empty-state p {
    font-size: 0.9375rem;
    font-weight: 500;
    margin-top: 0.75rem;
  }

  .empty-hint {
    display: block;
    font-size: 0.8125rem;
    font-weight: 500;
    opacity: 0.65;
    margin-top: 0.25rem;
  }
</style>
