<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import type { Item } from '$lib/types';
  import { formatMoney, formatDate, safeOpenUrl } from '$lib/utils';
  import { store } from '$lib/stores/budget.svelte';
  import { categoryIcon } from '$lib/icons';
  import { hapticLight } from '$lib/haptics';

  interface Props {
    item: Item | null;
    onEdit: (item: Item) => void;
    onclose: () => void;
  }

  let { item, onEdit, onclose }: Props = $props();

  const show = $derived(item !== null);
  const category = $derived(item ? store.data?.categories.find(c => c.key === item.category) ?? null : null);
  const isIncome = $derived(item?.item_type === 'income');

  function handleEdit() {
    if (!item) return;
    hapticLight();
    onEdit(item);
    onclose();
  }

  function handleDelete() {
    if (!item) return;
    hapticLight();
    store.showConfirm('Удалить эту запись?', () => {
      store.deleteItem(item!.id);
      onclose();
    });
  }

  function onOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onclose();
  }
</script>

{#if show && item}
  <div class="overlay" transition:fade={{ duration: 180 }} onclick={onOverlayClick} role="dialog" aria-modal="true" tabindex="-1" onkeydown={(e) => { if (e.key === 'Escape') onclose(); }}>
    <div class="sheet" transition:fly={{ y: 40, duration: 250, easing: cubicOut }}>
      <div class="header">
        <h2 class="title">{item.name}</h2>
        <button class="close-btn tap-btn" onclick={onclose} aria-label="Закрыть">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg>
        </button>
      </div>

      <div class="amount" class:positive={isIncome} class:negative={!isIncome}>
        {isIncome ? '+ ' : '− '}{formatMoney(item.amount)}
      </div>
      <span class="type-label">{isIncome ? 'Пополнение' : 'Расход'}</span>

      <div class="details">
        {#if category}
          <div class="detail-row">
            <span class="detail-label">Категория</span>
            <span class="detail-value cat-value" style="--cat-color: {category.color}">
              {@html categoryIcon(category.icon)}
              {category.name}
            </span>
          </div>
        {/if}
        <div class="detail-row">
          <span class="detail-label">Дата</span>
          <span class="detail-value">{formatDate(item.date)}</span>
        </div>
        {#if item.description}
          <div class="detail-row">
            <span class="detail-label">Описание</span>
            <span class="detail-value">{item.description}</span>
          </div>
        {/if}
        {#if item.link}
          <div class="detail-row">
            <span class="detail-label">Ссылка</span>
            <button class="detail-link" onclick={() => safeOpenUrl(item.link)}>
              Открыть
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 19H5V5h7V3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2v-7h-2v7zM14 3v2h3.59l-9.83 9.83 1.41 1.41L19 6.41V10h2V3h-7z"/></svg>
            </button>
          </div>
        {/if}
        <div class="detail-row">
          <span class="detail-label">Статус</span>
          <span class="detail-value status" class:completed={item.completed}>
            {#if item.completed}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/></svg>
              {isIncome ? 'Получено' : 'Оплачено'}
            {:else}
              В плане
            {/if}
          </span>
        </div>
      </div>

      <div class="actions">
        <button class="btn-danger tap-btn" onclick={handleDelete}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
          Удалить
        </button>
        <button class="btn-filled tap-btn" onclick={handleEdit}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
          Изменить
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--rg-overlay-bg);
    z-index: 300;
    display: flex;
    align-items: center;
    justify-content: center;
    padding-top: env(safe-area-inset-top, 0.5rem);
    padding-bottom: env(safe-area-inset-bottom, 0px);
  }

  .sheet {
    width: calc(100% - 2rem);
    max-width: 22rem;
    background: var(--rg-sheet-glass-bg);
    backdrop-filter: var(--rg-glass-blur);
    -webkit-backdrop-filter: var(--rg-glass-blur);
    border-radius: 1.25rem;
    padding: 1.25rem 1.5rem;
    box-shadow: 0 8px 40px rgba(0,0,0,0.15);
    contain: layout style paint;
    will-change: transform;
    -webkit-user-select: text;
    user-select: text;
  }

  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .title {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    letter-spacing: -0.02em;
    word-break: break-word;
  }

  .close-btn {
    flex-shrink: 0;
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--rg-on-surface-variant);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .amount {
    font-size: 1.75rem;
    font-weight: 800;
    letter-spacing: -0.5px;
    margin-top: 0.5rem;
    word-break: break-word;
  }

  .amount.positive { color: var(--rg-price-pos); }
  .amount.negative { color: var(--rg-price-neg); }

  .type-label {
    font-size: 0.75rem;
    color: var(--rg-on-surface-variant);
    font-weight: 500;
  }

  .details {
    margin-top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .detail-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .detail-label {
    font-size: 0.8125rem;
    color: var(--rg-on-surface-variant);
    flex-shrink: 0;
  }

  .detail-value {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--rg-on-surface);
    text-align: right;
    word-break: break-word;
  }

  .cat-value {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    color: var(--cat-color);
  }

  .cat-value :global(svg) {
    width: 1rem;
    height: 1rem;
  }

  .detail-link {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--rg-primary);
    background: none;
    border: none;
    cursor: pointer;
    font-family: inherit;
    padding: 0;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }

  .status.completed {
    color: var(--rg-price-pos);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1.25rem;
  }

  .btn-danger {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.625rem 1rem;
    border-radius: 999rem;
    border: none;
    background: color-mix(in srgb, var(--rg-danger) 10%, transparent);
    color: var(--rg-danger);
    font-size: 0.8125rem;
    font-family: inherit;
    cursor: pointer;
    font-weight: 600;
    transition: background 0.15s ease;
  }

  .btn-filled {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.625rem 1.25rem;
    border-radius: 999rem;
    border: none;
    background: var(--rg-primary);
    color: var(--rg-on-primary);
    font-size: 0.8125rem;
    font-family: inherit;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.15s ease;
    box-shadow: 0 2px 8px var(--rg-primary-shadow);
  }
</style>
