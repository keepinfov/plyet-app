<script lang="ts">
  import { store } from '$lib/stores/budget.svelte';
  import RecurringCard from './RecurringCard.svelte';
  import ProductCard from './ProductCard.svelte';
  import { flip } from 'svelte/animate';
  import { fade } from 'svelte/transition';

  // Sorted soonest-next-payment first feels most useful at a glance.
  const rules = $derived(
    [...store.recurringForBudget].sort((a, b) => a.start_date.localeCompare(b.start_date))
  );
  // Active products first, then closed; otherwise by start date.
  const products = $derived(
    [...store.productsForBudget].sort((a, b) => {
      if (a.status !== b.status) return a.status === 'active' ? -1 : 1;
      return a.start_date.localeCompare(b.start_date);
    })
  );
  const isEmpty = $derived(rules.length === 0 && products.length === 0);
</script>

<div class="regular-screen">
  {#if isEmpty}
    <div class="empty-state" transition:fade={{ duration: 200 }}>
      <svg width="4.5rem" height="4.5rem" viewBox="0 0 24 24" fill="var(--rg-outline)" opacity="0.4">
        <path d="M12 6v3l4-4-4-4v3c-4.42 0-8 3.58-8 8 0 1.57.46 3.03 1.24 4.26L6.7 14.8c-.45-.83-.7-1.79-.7-2.8 0-3.31 2.69-6 6-6zm6.76 1.74L17.3 9.2c.44.84.7 1.79.7 2.8 0 3.31-2.69 6-6 6v-3l-4 4 4 4v-3c4.42 0 8-3.58 8-8 0-1.57-.46-3.03-1.24-4.26z"/>
      </svg>
      <p>Нет регулярных платежей</p>
      <span class="empty-hint">Подписки, зарплата, вклады, кредиты — добавьте через +</span>
    </div>
  {:else}
    {#if rules.length > 0}
      <section class="section">
        <h2 class="section-title">Платежи</h2>
        <div class="section-list">
          {#each rules as rule (rule.id)}
            <div animate:flip={{ duration: 250 }}>
              <RecurringCard {rule} />
            </div>
          {/each}
        </div>
      </section>
    {/if}
    {#if products.length > 0}
      <section class="section">
        <h2 class="section-title">Вклады и кредиты</h2>
        <div class="section-list">
          {#each products as product (product.id)}
            <div animate:flip={{ duration: 250 }}>
              <ProductCard {product} />
            </div>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .regular-screen {
    padding: 0.75rem 1rem 1rem;
  }

  .section {
    margin-bottom: 1.25rem;
  }

  .section-title {
    font-size: 0.6875rem;
    font-weight: 700;
    color: var(--rg-on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin: 0 0 0.5rem 0.25rem;
  }

  .section-list {
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
