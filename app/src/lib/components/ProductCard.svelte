<script lang="ts">
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import type { Product } from '$lib/types';
  import { store } from '$lib/stores/budget.svelte';
  import { formatMoney, formatDate, safeOpenUrl, sanitizeAmount, rublesStringToKopecks } from '$lib/utils';
  import { annuityPayment, depositInterestPerPeriod, addMonths } from '$lib/finance';
  import { categoryIcon } from '$lib/icons';
  import { hapticLight } from '$lib/haptics';
  import { Sheet, Input, Button } from 'reglass-material';

  interface Props {
    product: Product;
  }

  let { product }: Props = $props();

  let expanded = $state(false);

  const category = $derived(store.data?.categories.find(c => c.key === product.category) ?? null);
  const bgColor = $derived(category?.color ?? '#5F6368');
  const isDeposit = $derived(product.kind === 'deposit');
  const isActive = $derived(product.status === 'active');
  const kindLabel = $derived(product.kind === 'deposit' ? 'Вклад' : product.kind === 'mortgage' ? 'Ипотека' : 'Кредит');
  const ratePct = $derived((product.annual_rate_bps / 100).toLocaleString('ru-RU', { maximumFractionDigits: 2 }));
  const maturity = $derived(addMonths(product.start_date, product.term_months));

  const monthlyPayment = $derived(annuityPayment(product.principal, product.annual_rate_bps, product.term_months));
  const monthlyIncome = $derived(depositInterestPerPeriod(product.principal, product.annual_rate_bps));
  const remainingDebt = $derived(Math.max(0, product.principal - product.principal_paid));
  const progress = $derived(product.term_months > 0 ? product.payments_made / product.term_months : 0);

  const badgeAmount = $derived(isDeposit ? monthlyIncome : monthlyPayment);

  function handleTap() {
    hapticLight();
    expanded = !expanded;
  }

  function handleEdit(e: MouseEvent) {
    e.stopPropagation();
    hapticLight();
    store.openProductModal(product);
  }

  function handleDelete(e: MouseEvent) {
    e.stopPropagation();
    store.showConfirm('Удалить этот продукт? Уже записанные операции останутся.', () => store.deleteProduct(product.id));
  }

  // Inline action sheet (extra payment / early close).
  let showAction = $state(false);
  let actionAmount = $state(0);
  let actionDisplay = $state('');

  function todayStr(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }

  function onAmountInput() {
    const cleaned = sanitizeAmount(actionDisplay);
    if (cleaned !== actionDisplay) actionDisplay = cleaned;
    actionAmount = rublesStringToKopecks(actionDisplay);
  }

  function openAction(e: MouseEvent) {
    e.stopPropagation();
    hapticLight();
    if (isDeposit) {
      actionAmount = remainingDebt; // unused for deposit
    } else {
      actionAmount = 0;
      actionDisplay = '';
    }
    showAction = true;
  }

  async function confirmAction() {
    if (isDeposit) {
      await store.closeDeposit(product.id, todayStr(), product.principal);
      showAction = false;
    } else {
      if (actionAmount <= 0) {
        store.showSnackbar('Введите сумму');
        return;
      }
      if (actionAmount > remainingDebt) {
        store.showSnackbar('Больше остатка долга');
        return;
      }
      await store.loanExtraPayment(product.id, actionAmount, todayStr());
      showAction = false;
    }
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
        <h3>{product.name}</h3>
        <div class="card-meta">
          <span class="kind-tag">{kindLabel}</span>
          <span class="card-date">{ratePct}% · {product.term_months} мес.{isActive ? '' : ' · закрыт'}</span>
        </div>
      </div>
      <span class="price-badge" class:price-positive={isDeposit} class:price-negative={!isDeposit}>
        <span class="price-text" class:positive={isDeposit} class:negative={!isDeposit}>
          {formatMoney(badgeAmount)}
        </span>
      </span>
    </div>

    {#if !isDeposit}
      <div class="pbar"><div class="pbar-fill" style="width: {Math.round(progress * 100)}%"></div></div>
    {/if}

    {#if expanded}
      <div class="card-detail" transition:slide={{ duration: 200, easing: cubicOut }}>
        <div class="detail-surface">
          {#if isDeposit}
            <div class="detail-row">
              <span class="detail-label">Тело вклада</span>
              <span class="detail-value">{formatMoney(product.principal)}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">Доход / мес</span>
              <span class="detail-value detail-positive">+ {formatMoney(monthlyIncome)}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">Погашение</span>
              <span class="detail-value">{formatDate(maturity)}</span>
            </div>
          {:else}
            {#if product.kind === 'mortgage' && product.down_payment > 0}
              <div class="detail-row">
                <span class="detail-label">Первонач. взнос</span>
                <span class="detail-value">{formatMoney(product.down_payment)}</span>
              </div>
            {/if}
            <div class="detail-row">
              <span class="detail-label">Платёж / мес</span>
              <span class="detail-value">{formatMoney(monthlyPayment)}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">Остаток долга</span>
              <span class="detail-value">{formatMoney(remainingDebt)}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">Выплачено</span>
              <span class="detail-value">{product.payments_made} / {product.term_months}</span>
            </div>
          {/if}
          <div class="detail-row">
            <span class="detail-label">Ставка</span>
            <span class="detail-value">{ratePct}% годовых</span>
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
          {#if product.description}
            <div class="detail-row">
              <span class="detail-label">Описание</span>
              <span class="detail-value">{product.description}</span>
            </div>
          {/if}
          {#if product.link}
            <button class="detail-link-btn" onclick={(e: MouseEvent) => { e.stopPropagation(); safeOpenUrl(product.link); }}>
              Открыть ссылку
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 19H5V5h7V3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2v-7h-2v7zM14 3v2h3.59l-9.83 9.83 1.41 1.41L19 6.41V10h2V3h-7z"/></svg>
            </button>
          {/if}
        </div>

        {#if isActive}
          <button class="act-btn act-primary" onclick={openAction}>
            {isDeposit ? 'Закрыть досрочно' : 'Досрочный платёж'}
          </button>
        {/if}
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

<Sheet show={showAction} onclose={() => showAction = false}>
  <div class="action-title">{isDeposit ? 'Закрыть вклад' : 'Досрочный платёж'}</div>
  {#if isDeposit}
    <p class="action-hint">Тело вклада {formatMoney(product.principal)} вернётся на баланс. Будущие проценты не начислятся.</p>
  {:else}
    <p class="action-hint">Остаток долга: {formatMoney(remainingDebt)}. Платёж уменьшит долг и сократит срок.</p>
    <div class="action-field">
      <Input
        type="text"
        inputmode="decimal"
        bind:value={actionDisplay}
        oninput={onAmountInput}
        placeholder="Сумма, ₽"
      />
    </div>
  {/if}
  <div class="action-buttons">
    <Button variant="text" onclick={() => showAction = false}>Отмена</Button>
    <Button variant="filled" onclick={() => { hapticLight(); confirmAction(); }}>
      {isDeposit ? 'Закрыть' : 'Внести'}
    </Button>
  </div>
</Sheet>

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
    gap: 0.625rem;
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

  .kind-tag {
    font-size: 0.6875rem;
    font-weight: 700;
    color: var(--cat-color);
    background: color-mix(in srgb, var(--cat-color) 18%, transparent);
    padding: 0.0625rem 0.375rem;
    border-radius: 0.375rem;
    flex-shrink: 0;
  }

  .card-date {
    font-size: 0.75rem;
    font-weight: 400;
    color: var(--rg-on-surface-variant);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .pbar {
    height: 0.375rem;
    border-radius: 0.25rem;
    background: color-mix(in srgb, var(--rg-on-surface) 10%, transparent);
    overflow: hidden;
  }

  .pbar-fill {
    height: 100%;
    border-radius: 0.25rem;
    background: var(--cat-color);
    transition: width 0.3s ease;
  }

  .card-detail {
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
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

  .act-primary {
    width: 100%;
    background: color-mix(in srgb, var(--cat-color) 22%, transparent);
    color: var(--cat-color);
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

  .action-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    margin-bottom: 0.5rem;
  }

  .action-hint {
    font-size: 0.8125rem;
    color: var(--rg-on-surface-variant);
    line-height: 1.4;
    margin: 0 0 0.875rem;
  }

  .action-field {
    margin-bottom: 0.875rem;
  }

  .action-buttons {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }
</style>
