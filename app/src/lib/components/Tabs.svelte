<script lang="ts">
  import { slide, fade } from 'svelte/transition';
  import { SegmentedToggle } from 'reglass-material';
  import { hapticLight } from '$lib/haptics';
  import { store } from '$lib/stores/budget.svelte';
  import { categoryIcon } from '$lib/icons';

  let showSearch = $state(false);
  let showFilters = $state(false);
  let searchInput = $state<HTMLInputElement>(undefined!);

  function toggleSearch() {
    hapticLight();
    showSearch = !showSearch;
    if (!showSearch) {
      store.searchQuery = '';
    } else {
      requestAnimationFrame(() => searchInput?.focus());
    }
  }

  function toggleFilters() {
    hapticLight();
    showFilters = !showFilters;
  }

  const categories = $derived(store.data?.categories ?? []);

  const dateRanges = [
    { key: 'all' as const, label: 'Все' },
    { key: 'this-month' as const, label: 'Этот месяц' },
    { key: 'last-month' as const, label: 'Прошлый' },
  ];

  const hasActiveFilters = $derived(
    store.searchQuery !== '' || store.categoryFilter !== null || store.dateRange !== 'all' || store.currentSort !== 'date-desc'
  );

  function clearFilters() {
    hapticLight();
    store.searchQuery = '';
    store.categoryFilter = null;
    store.dateRange = 'all';
    store.currentSort = 'date-desc';
    showSearch = false;
    showFilters = false;
  }

  const tabs = [
    { key: 'expenses' as const, label: 'Траты' },
    { key: 'income' as const, label: 'Доходы' },
    { key: 'all' as const, label: 'Все' },
  ];

  const sortGroups = [
    { keys: ['date-desc', 'date-asc'] as const, label: 'Дата' },
    { keys: ['price-desc', 'price-asc'] as const, label: 'Сумма' },
    { keys: ['name-asc', 'name-desc'] as const, label: 'Имя' },
  ];

  function cycleSortGroup(group: typeof sortGroups[number]) {
    hapticLight();
    const idx = (group.keys as readonly string[]).indexOf(store.currentSort);
    if (idx === -1) store.currentSort = group.keys[0];
    else store.currentSort = group.keys[(idx + 1) % 2];
  }
</script>

<div class="toolbar">
  <SegmentedToggle
    items={tabs}
    value={store.currentTab}
    onchange={(key) => { hapticLight(); store.currentTab = key; }}
  />
  <button class="search-toggle tap-btn" class:active={showSearch || store.searchQuery} onclick={toggleSearch} aria-label="Поиск">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M15.5 14h-.79l-.28-.27A6.471 6.471 0 0016 9.5 6.5 6.5 0 109.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
  </button>
  <button class="filter-toggle tap-btn" class:active={showFilters} onclick={toggleFilters} aria-label="Фильтры">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M10 18h4v-2h-4v2zM3 6v2h18V6H3zm3 7h12v-2H6v2z"/></svg>
    {#if hasActiveFilters}
      <span class="filter-dot"></span>
    {/if}
  </button>
</div>

{#if showSearch}
  <div class="search-bar" transition:slide={{ duration: 200 }}>
    <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M15.5 14h-.79l-.28-.27A6.471 6.471 0 0016 9.5 6.5 6.5 0 109.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
    <input
      bind:this={searchInput}
      class="search-input"
      type="text"
      placeholder="Поиск по названию..."
      bind:value={store.searchQuery}
    />
    {#if store.searchQuery}
      <button class="search-clear tap-btn" onclick={() => { store.searchQuery = ''; searchInput?.focus(); }} aria-label="Очистить">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg>
      </button>
    {/if}
  </div>
{/if}

{#if showFilters}
  <div class="filter-dropdown" transition:slide={{ duration: 200 }}>
    <div class="filter-section">
      <span class="filter-label">Сортировка</span>
      <div class="filter-chips">
        {#each sortGroups as group}
          {@const isActive = (group.keys as readonly string[]).includes(store.currentSort)}
          {@const isDesc = store.currentSort === group.keys[0]}
          <button
            class="sort-chip tap-btn"
            class:active={isActive}
            onclick={() => cycleSortGroup(group)}
          >
            <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round" class="sort-chip-arrow" class:arrow-up={isActive && !isDesc}>
              <path d="M7 10l5 5 5-5H7z"/>
            </svg>
            {group.label}
          </button>
        {/each}
      </div>
    </div>

    <div class="filter-section">
      <span class="filter-label">Категория</span>
      <div class="filter-chips filter-chips-scroll">
        {#each categories as cat}
          <button
            class="cat-chip tap-btn"
            class:selected={store.categoryFilter === cat.key}
            style="--chip-color: {cat.color}"
            onclick={() => { hapticLight(); store.categoryFilter = store.categoryFilter === cat.key ? null : cat.key; }}
          >
            {@html categoryIcon(cat.icon)}
            <span>{cat.name}</span>
          </button>
        {/each}
      </div>
    </div>

    {#if store.scopeType === 'all'}
      <div class="filter-section">
        <span class="filter-label">Период</span>
        <div class="filter-chips">
          {#each dateRanges as dr}
            <button
              class="date-chip tap-btn"
              class:active={store.dateRange === dr.key}
              onclick={() => { hapticLight(); store.dateRange = dr.key; }}
            >
              {dr.label}
            </button>
          {/each}
        </div>
      </div>
    {/if}

    {#if hasActiveFilters}
      <div transition:fade={{ duration: 150 }}>
        <button class="clear-btn tap-btn" onclick={clearFilters} aria-label="Сбросить фильтры">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg>
          Сбросить
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0.5rem 1rem 0;
  }

  .search-toggle, .filter-toggle {
    flex-shrink: 0;
    width: 2.75rem;
    height: 2.75rem;
    border-radius: 0.75rem;
    border: none;
    background: var(--rg-surface-2);
    color: var(--rg-on-surface-variant);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
    position: relative;
  }

  .search-toggle.active, .filter-toggle.active {
    background: color-mix(in srgb, var(--rg-primary) 12%, transparent);
    color: var(--rg-primary);
  }

  .filter-dot {
    position: absolute;
    top: 0.25rem;
    right: 0.25rem;
    width: 0.375rem;
    height: 0.375rem;
    border-radius: 50%;
    background: var(--rg-primary);
    pointer-events: none;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0.5rem 1rem 0;
    padding: 0.5rem 0.75rem;
    height: 2.375rem;
    border-radius: 0.75rem;
    background: var(--rg-surface-2);
    border: none;
  }

  .search-bar:focus-within {
    background: color-mix(in srgb, var(--rg-primary) 8%, var(--rg-surface-2));
  }

  .search-icon {
    flex-shrink: 0;
    opacity: 0.6;
  }

  .search-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--rg-on-surface);
    font-size: 0.875rem;
    font-family: inherit;
    outline: none;
    min-width: 0;
  }

  .search-input::placeholder {
    color: var(--rg-on-surface-variant);
    opacity: 0.5;
  }

  .search-clear {
    flex-shrink: 0;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--rg-on-surface-variant);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .filter-dropdown {
    margin: 0.5rem 1rem 0;
    padding: 0.75rem;
    border-radius: 0.75rem;
    background: var(--rg-surface-2);
    border: none;
  }

  .filter-section {
    margin-bottom: 0.625rem;
  }

  .filter-section:last-of-type {
    margin-bottom: 0;
  }

  .filter-label {
    font-size: 0.625rem;
    font-weight: 700;
    color: var(--rg-on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin-bottom: 0.375rem;
    display: block;
  }

  .filter-chips {
    display: flex;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .filter-chips-scroll {
    flex-wrap: nowrap;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
    padding-bottom: 0.125rem;
  }

  .filter-chips-scroll::-webkit-scrollbar { display: none; }

  .sort-chip {
    display: flex;
    align-items: center;
    gap: 0.1875rem;
    padding: 0.3125rem 0.625rem;
    border-radius: 0.75rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    color: var(--rg-on-surface-variant);
    font-size: 0.6875rem;
    font-family: inherit;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .sort-chip.active {
    background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
    color: var(--rg-primary);
    font-weight: 600;
  }

  .sort-chip-arrow {
    transition: transform 0.15s ease;
  }

  .sort-chip-arrow.arrow-up {
    transform: rotate(180deg);
  }

  .cat-chip {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.3125rem 0.625rem;
    border-radius: 0.75rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    color: var(--rg-on-surface-variant);
    font-size: 0.6875rem;
    font-family: inherit;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .cat-chip :global(svg) {
    width: 0.875rem;
    height: 0.875rem;
    opacity: 0.7;
  }

  .cat-chip.selected {
    background: color-mix(in srgb, var(--chip-color) 15%, transparent);
    color: var(--chip-color);
    font-weight: 600;
  }

  .cat-chip.selected :global(svg) {
    opacity: 1;
  }

  .date-chip {
    padding: 0.3125rem 0.625rem;
    border-radius: 0.75rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    color: var(--rg-on-surface-variant);
    font-size: 0.6875rem;
    font-family: inherit;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .date-chip.active {
    background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
    color: var(--rg-primary);
    font-weight: 600;
  }

  .clear-btn {
    margin-top: 0.5rem;
    margin-left: auto;
    padding: 0.25rem 0.5rem;
    border-radius: 999rem;
    border: none;
    background: color-mix(in srgb, var(--rg-danger) 10%, transparent);
    color: var(--rg-danger);
    font-size: 0.6875rem;
    font-family: inherit;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    transition: background 0.15s ease;
    white-space: nowrap;
  }
</style>
