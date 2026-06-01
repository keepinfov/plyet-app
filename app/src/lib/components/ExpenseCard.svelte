<script lang="ts">
  import { onDestroy } from 'svelte';
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import type { Item } from '$lib/types';
  import { formatMoney, formatRelativeDate, formatDate, safeOpenUrl } from '$lib/utils';
  import { store } from '$lib/stores/budget.svelte';
  import { hapticLight } from '$lib/haptics';
  import { categoryIcon } from '$lib/icons';

  const { data } = store;

  interface Props {
    item: Item;
    onEdit: (item: Item) => void;
  }

  let { item, onEdit }: Props = $props();

  let expanded = $state(false);

  const category = $derived(data?.categories.find(c => c.key === item.category) ?? null);
  const isIncome = $derived(item.item_type === 'income');
  const isCompleted = $derived(item.completed);
  const isVirtual = $derived(item.virtual === true);
  const bgColor = $derived(category?.color ?? '#5F6368');

  function pay() {
    if (isVirtual && item.source) {
      store.materializeOccurrence(item.source);
    } else {
      store.toggleCompleted(item.id);
    }
  }

  function editParentRule() {
    if (!item.source) return;
    if (item.source.kind === 'recurring') {
      const rule = store.recurringForBudget.find(r => r.id === item.source!.id);
      if (rule) store.openRecurringModal(rule);
    } else {
      const product = store.productsForBudget.find(p => p.id === item.source!.id);
      if (product) store.openProductModal(product);
    }
  }

  let longPressed = false;
  let pressTimer: ReturnType<typeof setTimeout> | null = null;

  let cardEl: HTMLDivElement;
  let fillX = $state(0);
  let fillY = $state(0);
  let fillRadius = $state(0);
  let filling = $state(false);
  let fillTimer: ReturnType<typeof setTimeout>;
  onDestroy(() => {
    clearTimeout(fillTimer);
    if (pressTimer) clearTimeout(pressTimer);
  });

  function triggerRipple(clientX: number, clientY: number) {
    const rect = cardEl.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    const maxDist = Math.max(
      Math.hypot(x, y),
      Math.hypot(rect.width - x, y),
      Math.hypot(x, rect.height - y),
      Math.hypot(rect.width - x, rect.height - y),
    );
    fillX = x;
    fillY = y;
    fillRadius = maxDist;
    filling = true;
    clearTimeout(fillTimer);
    fillTimer = setTimeout(() => filling = false, 500);
  }

  function onTouchStart(e: TouchEvent) {
    const target = e.target as HTMLElement;
    if (target.closest('.detail-actions') || target.closest('.detail-link-btn') || target.closest('.card-link')) return;
    longPressed = false;
    if (pressTimer) clearTimeout(pressTimer);
    pressTimer = setTimeout(() => {
      longPressed = true;
      hapticLight();
      triggerRipple(e.touches[0].clientX, e.touches[0].clientY);
      pay();
    }, 500);
  }

  function onTouchMove(e: TouchEvent) {
    if (pressTimer) { clearTimeout(pressTimer); pressTimer = null; }
  }

  function onTouchEnd() {
    if (pressTimer) { clearTimeout(pressTimer); pressTimer = null; }
  }

  function handleTap(e?: MouseEvent) {
    if (longPressed) { longPressed = false; return; }
    hapticLight();
    expanded = !expanded;
  }

  function handleEditClick(e: MouseEvent) {
    e.stopPropagation();
    hapticLight();
    if (isVirtual) editParentRule();
    else onEdit(item);
  }

  function handleDeleteClick(e: MouseEvent) {
    e.stopPropagation();
    store.showConfirm('Удалить эту запись?', () => store.deleteItem(item.id));
  }

  function handleToggleComplete(e: MouseEvent) {
    e.stopPropagation();
    hapticLight();
    triggerRipple(e.clientX, e.clientY);
    pay();
  }
</script>

<div class="card-wrapper">
  <div
    class="card"
    class:completed={isCompleted}
    style="--cat-color: {bgColor}"
    bind:this={cardEl}
    onclick={handleTap}
    onkeydown={(e) => e.key === 'Enter' && handleTap()}
    ontouchstart={onTouchStart}
    ontouchmove={onTouchMove}
    ontouchend={onTouchEnd}
    role="button"
    tabindex="0"
  >
    <!-- Top row: icon, name/date, price -->
    <div class="card-top">
      <div class="card-cat-icon" style="background: color-mix(in srgb, {bgColor} 15%, transparent); color: {bgColor}">
        {@html categoryIcon(category?.icon ?? 'other')}
      </div>
      <div class="card-info">
        <h3>{item.name}</h3>
        <div class="card-meta">
          {#if isVirtual}
            <span class="recur-badge" title="Регулярный платёж">
              <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M12 6v3l4-4-4-4v3c-4.42 0-8 3.58-8 8 0 1.57.46 3.03 1.24 4.26L6.7 14.8c-.45-.83-.7-1.79-.7-2.8 0-3.31 2.69-6 6-6zm6.76 1.74L17.3 9.2c.44.84.7 1.79.7 2.8 0 3.31-2.69 6-6 6v-3l-4 4 4 4v-3c4.42 0 8-3.58 8-8 0-1.57-.46-3.03-1.24-4.26z"/></svg>
            </span>
          {/if}
          <span class="card-date">{formatRelativeDate(item.date)}</span>
          {#if item.link}
            <button class="card-link" onclick={(e: MouseEvent) => { e.stopPropagation(); safeOpenUrl(item.link); }} aria-label="Открыть ссылку">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M3.9 12c0-1.71 1.39-3.1 3.1-3.1h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-1.9H7c-1.71 0-3.1-1.39-3.1-3.1zM8 13h8v-2H8v2zm9-6h-4v1.9h4c1.71 0 3.1 1.39 3.1 3.1s-1.39 3.1-3.1 3.1h-4V17h4c2.76 0 5-2.24 5-5s-2.24-5-5-5z"/></svg>
            </button>
          {/if}
        </div>
      </div>
      <span class="price-badge" class:price-positive={isIncome} class:price-negative={!isIncome}>
        <span class="price-text" class:positive={isIncome} class:negative={!isIncome}>
          {formatMoney(item.amount)}
        </span>
      </span>
    </div>

    <!-- Expanded detail -->
    {#if expanded}
      <div class="card-detail" transition:slide={{ duration: 200, easing: cubicOut }}>
        <div class="detail-surface">
          <div class="detail-row">
            <span class="detail-label">Название</span>
            <span class="detail-value">{item.name}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Сумма</span>
            <span class="detail-value" class:detail-positive={isIncome} class:detail-negative={!isIncome}>{isIncome ? '+ ' : '− '}{formatMoney(item.amount)}</span>
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
            <button class="detail-link-btn" onclick={(e: MouseEvent) => { e.stopPropagation(); safeOpenUrl(item.link); }}>
              Открыть ссылку
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 19H5V5h7V3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2v-7h-2v7zM14 3v2h3.59l-9.83 9.83 1.41 1.41L19 6.41V10h2V3h-7z"/></svg>
            </button>
          {/if}
        </div>
        <div class="detail-actions">
          <button class="act-btn act-complete" onclick={(e: MouseEvent) => handleToggleComplete(e)}>
            {#if isCompleted}
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46A7.93 7.93 0 0020 12c0-4.42-3.58-8-8-8zm0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8L5.24 7.74A7.93 7.93 0 004 12c0 4.42 3.58 8 8 8v3l4-4-4-4v3z"/></svg>
              Вернуть
            {:else}
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/></svg>
              {isIncome ? 'Получено' : 'Оплачено'}
            {/if}
          </button>
          <button class="act-btn act-edit" onclick={(e: MouseEvent) => { e.stopPropagation(); handleEditClick(e); }}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
            Изменить
          </button>
          {#if !isVirtual}
            <button class="act-btn act-delete" onclick={(e: MouseEvent) => { e.stopPropagation(); handleDeleteClick(e); }}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
              Удалить
            </button>
          {/if}
        </div>
      </div>
    {/if}
    {#if filling}
      <span class="fill-wave" style="left: {fillX}px; top: {fillY}px; --fill-r: {fillRadius}px;"></span>
    {/if}
  </div>
</div>

<style>
  .card-wrapper {
    position: relative;
    border-radius: 1rem;
    animation: fadeInUp 0.25s ease-out forwards;
    animation-delay: calc(var(--card-index, 0) * 0.04s);
    opacity: 0;
  }

  /* ── Card container ── */
  .card {
    position: relative;
    z-index: 1;
    width: 100%;
    background: color-mix(in srgb, var(--cat-color) 22%, var(--rg-surface-2));
    border-radius: 1rem;
    padding: 0.875rem;
    display: flex;
    flex-direction: column;
    gap: 0;
    cursor: pointer;
    transition: background-color 0.2s ease, transform 0.35s cubic-bezier(0.175, 0.885, 0.32, 1.275);
    touch-action: pan-y;
    -webkit-user-select: none;
    user-select: none;
    border: none;
    box-shadow: none;
    text-align: left;
    color: inherit;
    font-family: inherit;
    overflow: hidden;
  }

  .card:active:not(:has(.act-btn:active, .detail-link-btn:active, .card-link:active)) {
    transform: scale(0.97);
    transition: transform 0.15s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .card.completed {
    background: var(--rg-surface-2);
    opacity: 0.55;
  }

  .card.completed h3 {
    text-decoration: line-through;
  }

  .card.completed .price-text {
    text-decoration: line-through;
  }

  /* ── Top row ── */
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
    transition: opacity 0.3s ease;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    margin-top: 0.125rem;
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

  .card-link {
    font-size: 0.75rem;
    text-decoration: none;
    color: var(--rg-primary);
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    border: none;
    background: none;
    cursor: pointer;
    font-family: inherit;
    padding: 0;
  }

  /* ── Price badge ── */
  .price-badge {
    padding: 0.625rem 0.75rem;
    border-radius: 0.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    border: none;
    flex-shrink: 0;
    max-width: 50%;
    transition: background 0.2s ease-out;
  }

  .price-badge.price-negative {
    background: var(--rg-neon-red);
  }

  .price-badge.price-positive {
    background: var(--rg-neon-green);
  }

  .card.completed .price-badge {
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
  }

  .price-text {
    font-size: clamp(0.75rem, 3.8vw, 0.9375rem);
    font-weight: 800;
    letter-spacing: -0.3px;
    color: white;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card.completed .price-text {
    color: var(--rg-on-surface-variant);
  }

  .price-text.positive::before { content: '+ '; }
  .price-text.negative::before { content: '− '; }

  /* ── Expanded detail ── */
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

  .detail-link-btn:active {
    opacity: 0.6;
  }

  /* ── Action buttons — full width, blocky ── */
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
    transition: transform 0.15s ease, opacity 0.15s ease;
  }

  .act-complete {
    background: color-mix(in srgb, var(--rg-price-pos) 12%, transparent);
    color: var(--rg-price-pos);
    display: none;
  }

  /* Show complete button only on tablets and desktops */
  @media (min-width: 600px) {
    .act-complete {
      display: flex;
    }
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

  /* ── Fill wave ── */
  .fill-wave {
    position: absolute;
    width: 0;
    height: 0;
    border-radius: 50%;
    background: rgba(128,128,128,0.12);
    transform: translate(-50%, -50%);
    animation: fill-expand 0.5s cubic-bezier(0.2, 0.6, 0.3, 1) forwards;
    pointer-events: none;
    z-index: 0;
  }

  @keyframes fill-expand {
    0% { width: 0; height: 0; opacity: 1; }
    60% { opacity: 1; }
    100% { width: calc(var(--fill-r) * 2); height: calc(var(--fill-r) * 2); opacity: 0; }
  }

  @keyframes fadeInUp {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
</style>
