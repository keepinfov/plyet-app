<script lang="ts">
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import type { Recurring } from '$lib/types';
  import { store } from '$lib/stores/budget.svelte';
  import { formatMoney, formatRelativeDate, formatDate, safeOpenUrl } from '$lib/utils';
  import { nextOccurrences } from '$lib/finance';
  import { categoryIcon } from '$lib/icons';
  import { hapticLight } from '$lib/haptics';

  interface Props {
    rule: Recurring;
  }

  let { rule }: Props = $props();

  let expanded = $state(false);

  const category = $derived(store.data?.categories.find(c => c.key === rule.category) ?? null);
  const isIncome = $derived(rule.item_type === 'income');
  const bgColor = $derived(category?.color ?? '#5F6368');

  const WEEKDAYS = ['понедельник', 'вторник', 'среда', 'четверг', 'пятница', 'суббота', 'воскресенье'];

  const schedule = $derived.by(() => {
    if (rule.freq === 'weekly') return `Каждую неделю · ${WEEKDAYS[rule.anchor_day] ?? ''}`;
    if (rule.freq === 'yearly') return `Каждый год · ${rule.anchor_day} число`;
    return `Каждый месяц · ${rule.anchor_day} число`;
  });

  const next = $derived(nextOccurrences(rule, 1)[0] ?? null);

  function handleTap() {
    hapticLight();
    expanded = !expanded;
  }

  function handleEdit(e: MouseEvent) {
    e.stopPropagation();
    hapticLight();
    store.openRecurringModal(rule);
  }

  function handleDelete(e: MouseEvent) {
    e.stopPropagation();
    store.showConfirm('Удалить регулярный платёж?', () => store.deleteRecurring(rule.id));
  }
</script>

<div class="card-wrapper">
  <div
    class="card"
    style="--cat-color: {bgColor}"
    onclick={handleTap}
    onkeydown={(e) => e.key === 'Enter' && handleTap()}
    role="button"
    tabindex="0"
  >
    <div class="card-top">
      <div class="card-cat-icon" style="background: color-mix(in srgb, {bgColor} 15%, transparent); color: {bgColor}">
        {@html categoryIcon(category?.icon ?? 'other')}
      </div>
      <div class="card-info">
        <h3>{rule.name}</h3>
        <div class="card-meta">
          <span class="recur-badge" title="Регулярный платёж">
            <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M12 6v3l4-4-4-4v3c-4.42 0-8 3.58-8 8 0 1.57.46 3.03 1.24 4.26L6.7 14.8c-.45-.83-.7-1.79-.7-2.8 0-3.31 2.69-6 6-6zm6.76 1.74L17.3 9.2c.44.84.7 1.79.7 2.8 0 3.31-2.69 6-6 6v-3l-4 4 4 4v-3c4.42 0 8-3.58 8-8 0-1.57-.46-3.03-1.24-4.26z"/></svg>
          </span>
          <span class="card-date">{schedule}</span>
        </div>
      </div>
      <span class="price-badge" class:price-positive={isIncome} class:price-negative={!isIncome}>
        <span class="price-text" class:positive={isIncome} class:negative={!isIncome}>
          {formatMoney(rule.amount)}
        </span>
      </span>
    </div>

    {#if expanded}
      <div class="card-detail" transition:slide={{ duration: 200, easing: cubicOut }}>
        <div class="detail-surface">
          <div class="detail-row">
            <span class="detail-label">Сумма</span>
            <span class="detail-value" class:detail-positive={isIncome} class:detail-negative={!isIncome}>{isIncome ? '+ ' : '− '}{formatMoney(rule.amount)}</span>
          </div>
          {#if category}
            <div class="detail-row">
              <span class="detail-label">Категория</span>
              <span class="detail-value" style="color: {bgColor}">
                {@html categoryIcon(category.icon)}
                {category.name}
              </span>
            </div>
          {/if}
          <div class="detail-row">
            <span class="detail-label">Расписание</span>
            <span class="detail-value">{schedule}</span>
          </div>
          {#if next}
            <div class="detail-row">
              <span class="detail-label">Следующий</span>
              <span class="detail-value">{formatDate(next)} · {formatRelativeDate(next)}</span>
            </div>
          {/if}
          {#if rule.end_date}
            <div class="detail-row">
              <span class="detail-label">До</span>
              <span class="detail-value">{formatDate(rule.end_date)}</span>
            </div>
          {/if}
          {#if rule.description}
            <div class="detail-row">
              <span class="detail-label">Описание</span>
              <span class="detail-value">{rule.description}</span>
            </div>
          {/if}
          {#if rule.link}
            <button class="detail-link-btn" onclick={(e: MouseEvent) => { e.stopPropagation(); safeOpenUrl(rule.link); }}>
              Открыть ссылку
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 19H5V5h7V3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2v-7h-2v7zM14 3v2h3.59l-9.83 9.83 1.41 1.41L19 6.41V10h2V3h-7z"/></svg>
            </button>
          {/if}
        </div>
        <div class="detail-actions">
          <button class="act-btn act-edit" onclick={handleEdit}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
            Изменить
          </button>
          <button class="act-btn act-delete" onclick={handleDelete}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
            Удалить
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .card-wrapper {
    position: relative;
    border-radius: 1rem;
  }

  .card {
    position: relative;
    width: 100%;
    background: color-mix(in srgb, var(--cat-color) 22%, var(--rg-surface-2));
    border-radius: 1rem;
    padding: 0.875rem;
    display: flex;
    flex-direction: column;
    cursor: pointer;
    transition: transform 0.35s cubic-bezier(0.175, 0.885, 0.32, 1.275);
    -webkit-user-select: none;
    user-select: none;
    border: none;
    text-align: left;
    color: inherit;
    font-family: inherit;
    overflow: hidden;
  }

  .card:active:not(:has(.act-btn:active, .detail-link-btn:active)) {
    transform: scale(0.97);
    transition: transform 0.15s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .card-top {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .card-cat-icon {
    width: 2.75rem;
    height: 2.75rem;
    border-radius: 0.875rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .card-cat-icon :global(svg) {
    width: 1.25rem;
    height: 1.25rem;
  }

  .card-info {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .card-info h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: var(--rg-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    margin-top: 0.125rem;
    min-width: 0;
  }

  .card-date {
    font-size: 0.75rem;
    font-weight: 400;
    color: var(--rg-on-surface-variant);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .recur-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--rg-primary);
    opacity: 0.85;
    flex-shrink: 0;
  }

  .price-badge {
    padding: 0.625rem 0.75rem;
    border-radius: 0.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    flex-shrink: 0;
    max-width: 50%;
  }

  .price-badge.price-negative { background: var(--rg-neon-red); }
  .price-badge.price-positive { background: var(--rg-neon-green); }

  .price-text {
    font-size: clamp(0.75rem, 3.8vw, 0.9375rem);
    font-weight: 800;
    letter-spacing: -0.3px;
    color: white;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .price-text.positive::before { content: '+ '; }
  .price-text.negative::before { content: '− '; }

  .card-detail {
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
    margin-top: 0.75rem;
  }

  .detail-surface {
    background: var(--rg-surface);
    border-radius: 0.75rem;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  :global([data-theme="dark"]) .detail-surface {
    background: rgba(255,255,255,0.04);
  }

  .detail-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .detail-label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--rg-on-surface-variant);
    flex-shrink: 0;
  }

  .detail-value {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    text-align: right;
    word-break: break-word;
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
  }

  .detail-positive { color: var(--rg-price-pos); }
  .detail-negative { color: var(--rg-price-neg); }

  .detail-value :global(svg) {
    width: 1rem;
    height: 1rem;
  }

  .detail-link-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--rg-primary);
    background: none;
    border: none;
    cursor: pointer;
    font-family: inherit;
    padding: 0;
    align-self: flex-end;
  }

  .detail-link-btn:active { opacity: 0.6; }

  .detail-actions {
    display: flex;
    gap: 0.5rem;
  }

  .act-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 0.75rem;
    border: none;
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
    font-weight: 700;
    transition: transform 0.15s ease;
  }

  .act-edit {
    background: color-mix(in srgb, var(--rg-primary) 12%, transparent);
    color: var(--rg-primary);
  }

  .act-delete {
    background: color-mix(in srgb, var(--rg-danger) 10%, transparent);
    color: var(--rg-danger);
  }

  .act-btn:active {
    transform: scale(0.96);
    transition: transform 0.1s ease;
  }
</style>
