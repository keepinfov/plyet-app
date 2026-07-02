<script lang="ts">
  import { store } from '$lib/stores/budget.svelte';
  import { periodChipLabel } from '$lib/dates';
  import { hapticLight } from '$lib/haptics';
  import { budgetIcon } from '$lib/icons';

  function select(scope: string) {
    hapticLight();
    store.setScope(scope);
  }
</script>

<div class="month-selector">
  <button class="chip tap-btn" class:active={store.currentScope === 'all'} onclick={() => select('all')}>
    Весь бюджет
  </button>
  {#each store.monthBudgets as month (month.id)}
    <button class="chip tap-btn" class:active={store.currentScope === month.period} onclick={() => select(month.period!)}>
      {periodChipLabel(month.period!)}
    </button>
  {/each}
  {#each store.customBudgets as custom (custom.id)}
    <button class="chip chip-custom tap-btn" class:active={store.currentScope === `custom:${custom.id}`} onclick={() => select(`custom:${custom.id}`)}>
      {@html budgetIcon(custom.icon)}
      {custom.name}
    </button>
  {/each}
</div>

<style>
  .month-selector {
    display: flex;
    gap: 0.375rem;
    padding: 0.5rem 1rem 0;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
  }

  .month-selector::-webkit-scrollbar {
    display: none;
  }

  .chip {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.375rem 0.75rem;
    border-radius: 0.75rem;
    border: none;
    background: var(--rg-surface-2);
    color: var(--rg-on-surface-variant);
    font-size: 0.75rem;
    font-family: inherit;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .chip :global(svg) {
    width: 0.875rem;
    height: 0.875rem;
    opacity: 0.8;
  }

  .chip.active {
    background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
    color: var(--rg-primary);
  }

  .chip-custom {
    border: 1px dashed color-mix(in srgb, var(--rg-on-surface-variant) 30%, transparent);
  }

  .chip-custom.active {
    border-color: var(--rg-primary);
  }
</style>
