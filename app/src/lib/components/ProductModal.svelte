<script lang="ts">
  import { slide } from 'svelte/transition';
  import { Sheet, Input, Button, SegmentedToggle } from 'reglass-material';
  import { store } from '$lib/stores/budget.svelte';
  import { categoryIcon } from '$lib/icons';
  import { sanitizeAmount, rublesStringToKopecks, formatMoney } from '$lib/utils';
  import { annuityPayment, depositInterestPerPeriod } from '$lib/finance';
  import { hapticLight } from '$lib/haptics';
  import DatePicker from '$lib/components/DatePicker.svelte';

  const show = $derived(store.showProductModal);
  const editData = $derived(store.editingProduct);

  let kind = $state<'deposit' | 'loan' | 'mortgage'>('deposit');
  let name = $state('');
  let principal = $state(0);
  let principalDisplay = $state('');
  let downPayment = $state(0);
  let downPaymentDisplay = $state('');
  let rateDisplay = $state('');
  let termMonths = $state(12);
  let startDate = $state('');
  let horizon = $state(3);
  let category = $state('other');
  let earlyRateDisplay = $state('');
  let description = $state('');
  let link = $state('');
  let showAdvanced = $state(false);
  let saving = $state(false);
  let nameInput = $state<HTMLInputElement | null>(null);

  const categories = $derived(store.data?.categories ?? []);

  const rateBps = $derived(Math.round((parseFloat(rateDisplay.replace(',', '.')) || 0) * 100));
  const earlyRateBps = $derived(earlyRateDisplay ? Math.round((parseFloat(earlyRateDisplay.replace(',', '.')) || 0) * 100) : null);

  // Mortgage: `principal` input is the property price; the financed loan body is
  // price − down payment. Deposit/loan use `principal` directly.
  const financed = $derived(kind === 'mortgage' ? Math.max(0, principal - downPayment) : principal);

  const previewPayment = $derived(
    kind === 'deposit'
      ? depositInterestPerPeriod(principal, rateBps)
      : annuityPayment(financed, rateBps, termMonths)
  );

  const kindTitle = $derived(
    editData
      ? (kind === 'deposit' ? 'Вклад' : kind === 'mortgage' ? 'Ипотека' : 'Кредит')
      : (kind === 'deposit' ? 'Новый вклад' : kind === 'mortgage' ? 'Новая ипотека' : 'Новый кредит')
  );

  function todayStr(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }

  function onPrincipalInput() {
    const cleaned = sanitizeAmount(principalDisplay);
    if (cleaned !== principalDisplay) principalDisplay = cleaned;
    principal = rublesStringToKopecks(principalDisplay);
  }

  function onDownPaymentInput() {
    const cleaned = sanitizeAmount(downPaymentDisplay);
    if (cleaned !== downPaymentDisplay) downPaymentDisplay = cleaned;
    downPayment = rublesStringToKopecks(downPaymentDisplay);
  }

  $effect(() => {
    if (!show) return;
    hapticLight();
    if (editData) {
      kind = editData.kind;
      name = editData.name;
      // Mortgage stores the financed body; the form edits property price (= body + down).
      const shownPrincipal = editData.kind === 'mortgage' ? editData.principal + editData.down_payment : editData.principal;
      principal = shownPrincipal;
      principalDisplay = (shownPrincipal / 100).toFixed(2).replace(/\.?0+$/, '');
      downPayment = editData.down_payment;
      downPaymentDisplay = editData.down_payment ? (editData.down_payment / 100).toFixed(2).replace(/\.?0+$/, '') : '';
      rateDisplay = (editData.annual_rate_bps / 100).toString();
      termMonths = editData.term_months;
      startDate = editData.start_date;
      horizon = editData.horizon;
      category = editData.category;
      earlyRateDisplay = editData.early_rate_bps != null ? (editData.early_rate_bps / 100).toString() : '';
      description = editData.description || '';
      link = editData.link || '';
      showAdvanced = !!(editData.description || editData.link || editData.early_rate_bps != null);
    } else {
      kind = store.newProductKind;
      name = '';
      principal = 0;
      principalDisplay = '';
      downPayment = 0;
      downPaymentDisplay = '';
      rateDisplay = '';
      termMonths = 12;
      startDate = todayStr();
      horizon = 3;
      category = 'other';
      earlyRateDisplay = '';
      description = '';
      link = '';
      showAdvanced = false;
    }
    requestAnimationFrame(() => nameInput?.focus());
  });

  function close() {
    store.closeProductModal();
  }

  const MAX_AMOUNT = 1_000_000_000_000;

  async function save() {
    if (!name.trim() || principal <= 0) {
      store.showSnackbar('Заполните название и сумму');
      return;
    }
    if (principal > MAX_AMOUNT) {
      store.showSnackbar('Сумма слишком большая');
      return;
    }
    if (rateBps < 0 || rateBps > 1_000_000) {
      store.showSnackbar('Некорректная ставка');
      return;
    }
    if (termMonths < 1 || termMonths > 1200) {
      store.showSnackbar('Некорректный срок');
      return;
    }
    if (kind === 'mortgage' && downPayment >= principal) {
      store.showSnackbar('Взнос должен быть меньше стоимости');
      return;
    }
    if (saving) return;
    saving = true;
    try {
      const paymentModel = kind === 'deposit' ? 'simple' : 'annuity';
      const downPaymentKop = kind === 'mortgage' ? downPayment : 0;
      if (editData) {
        await store.updateProduct(editData.id, {
          name: name.trim(),
          principal: financed,
          annual_rate_bps: rateBps,
          term_months: termMonths,
          start_date: startDate,
          payment_model: paymentModel,
          early_rate_bps: kind === 'deposit' ? earlyRateBps : null,
          horizon,
          category,
          down_payment: downPaymentKop,
          description: description.trim(),
          link: link.trim(),
        });
      } else {
        await store.addProduct({
          kind,
          name: name.trim(),
          principal: financed,
          annual_rate_bps: rateBps,
          term_months: termMonths,
          start_date: startDate,
          payment_model: paymentModel,
          early_rate_bps: kind === 'deposit' ? earlyRateBps : null,
          horizon,
          category,
          down_payment: downPaymentKop,
          description: description.trim(),
          link: link.trim(),
        });
      }
      close();
    } finally {
      saving = false;
    }
  }

  function remove() {
    if (!editData) return;
    const id = editData.id;
    store.showConfirm('Удалить этот продукт? Уже записанные операции останутся.', async () => {
      await store.deleteProduct(id);
      close();
    });
  }
</script>

<Sheet show={show} onclose={close} scrollable>
  <div class="modal-header">
    <div class="modal-title">{kindTitle}</div>
    {#if editData}
      <button class="delete-btn tap-btn" onclick={remove} aria-label="Удалить">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="#D93025"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
      </button>
    {/if}
  </div>

  {#if !editData}
    <div class="type-seg">
      <SegmentedToggle
        items={[{ key: 'deposit', label: 'Вклад' }, { key: 'loan', label: 'Кредит' }, { key: 'mortgage', label: 'Ипотека' }]}
        value={kind}
        onchange={(k) => { hapticLight(); kind = k as 'deposit' | 'loan' | 'mortgage'; }}
      />
    </div>
  {/if}

  <div class="form-group">
    <Input bind:element={nameInput} bind:value={name} placeholder="Название" />
  </div>

  <div class="form-group">
    <span class="form-label">{kind === 'deposit' ? 'Тело вклада' : kind === 'mortgage' ? 'Стоимость жилья' : 'Сумма кредита'}</span>
    <Input
      type="text"
      inputmode="decimal"
      bind:value={principalDisplay}
      oninput={onPrincipalInput}
      placeholder="0"
      style="padding-right:2rem"
    />
    <span class="currency-hint">₽</span>
  </div>

  {#if kind === 'mortgage'}
    <div class="form-group">
      <span class="form-label">Первоначальный взнос</span>
      <Input
        type="text"
        inputmode="decimal"
        bind:value={downPaymentDisplay}
        oninput={onDownPaymentInput}
        placeholder="0"
        style="padding-right:2rem"
      />
      <span class="currency-hint">₽</span>
    </div>
    {#if financed > 0}
      <div class="preview">
        <span>Сумма кредита</span>
        <b>{formatMoney(financed)}</b>
      </div>
    {/if}
  {/if}

  <div class="form-row">
    <div class="form-group half">
      <span class="form-label">Ставка, % годовых</span>
      <Input type="text" inputmode="decimal" bind:value={rateDisplay} placeholder="0" />
    </div>
    <div class="form-group half">
      <span class="form-label">Срок, мес.</span>
      <div class="day-stepper">
        <button class="step-btn tap-btn" onclick={() => { hapticLight(); termMonths = Math.max(1, termMonths - 1); }} aria-label="Меньше">−</button>
        <span class="day-value">{termMonths}</span>
        <button class="step-btn tap-btn" onclick={() => { hapticLight(); termMonths = Math.min(1200, termMonths + 1); }} aria-label="Больше">+</button>
      </div>
    </div>
  </div>

  {#if (kind === 'deposit' ? principal : financed) > 0 && termMonths > 0}
    <div class="preview">
      <span>{kind === 'deposit' ? 'Доход в месяц' : 'Платёж в месяц'}</span>
      <b class:pos={kind === 'deposit'}>{kind === 'deposit' ? '+ ' : ''}{formatMoney(previewPayment)}</b>
    </div>
  {/if}

  <div class="form-group">
    <span class="form-label">Дата начала</span>
    <DatePicker bind:value={startDate} />
  </div>

  <div class="form-group">
    <span class="form-label">Показывать вперёд</span>
    <div class="day-stepper">
      <button class="step-btn tap-btn" onclick={() => { hapticLight(); horizon = Math.max(1, horizon - 1); }} aria-label="Меньше">−</button>
      <span class="day-value">{horizon}</span>
      <button class="step-btn tap-btn" onclick={() => { hapticLight(); horizon = Math.min(24, horizon + 1); }} aria-label="Больше">+</button>
      <span class="day-hint">{horizon === 1 ? 'следующий платёж' : `ближайшие ${horizon}`}</span>
    </div>
  </div>

  <button class="advanced-toggle" onclick={() => showAdvanced = !showAdvanced}>
    <span>Дополнительно</span>
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round" class:rotated={showAdvanced}><path d="M7.41 8.59L12 13.17l4.59-4.58L18 10l-6 6-6-6 1.41-1.41z"/></svg>
  </button>

  {#if showAdvanced}
    <div class="advanced-section" transition:slide={{ duration: 250 }}>
      {#if kind === 'deposit'}
        <div class="form-group">
          <span class="form-label">Ставка при досрочном закрытии, %</span>
          <Input type="text" inputmode="decimal" bind:value={earlyRateDisplay} placeholder="0" />
        </div>
      {/if}

      <div class="form-group">
        <span class="form-label">Категория</span>
        <div class="cat-picker">
          {#each categories as cat}
            <button
              class="cat-opt tap-btn"
              class:selected={category === cat.key}
              style="--cat-color: {cat.color}"
              onclick={() => { hapticLight(); category = cat.key; }}
            >
              {@html categoryIcon(cat.icon)}
              <span>{cat.name}</span>
            </button>
          {/each}
        </div>
      </div>

      <div class="form-group">
        <label class="form-label" for="prod-description">Описание</label>
        <Input id="prod-description" bind:value={description} placeholder="Заметка..." />
      </div>

      <div class="form-group">
        <label class="form-label" for="prod-link">Ссылка</label>
        <Input id="prod-link" bind:value={link} placeholder="https://..." />
      </div>
    </div>
  {/if}

  <div class="actions">
    <Button variant="text" onclick={close} disabled={saving}>Отмена</Button>
    <Button variant="filled" onclick={() => { hapticLight(); save(); }} disabled={saving}>{saving ? '...' : (editData ? 'Сохранить' : (kind === 'deposit' ? 'Открыть' : 'Создать'))}</Button>
  </div>
</Sheet>

<style>
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
  }

  .delete-btn {
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 50%;
    border: none;
    background: var(--rg-danger-glass);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .modal-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    letter-spacing: -0.02em;
  }

  .type-seg {
    display: flex;
    margin-bottom: 1rem;
  }

  .form-group {
    margin-bottom: 0.875rem;
    position: relative;
  }

  .form-row {
    display: flex;
    gap: 0.75rem;
  }

  .form-group.half {
    flex: 1;
  }

  .form-label {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--rg-on-surface-variant);
    margin-bottom: 0.375rem;
    display: block;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .currency-hint {
    position: absolute;
    right: 0.875rem;
    bottom: 0.75rem;
    font-size: 1rem;
    font-weight: 600;
    color: var(--rg-on-surface-variant);
    pointer-events: none;
  }

  .preview {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 0.875rem;
    border-radius: 0.875rem;
    background: color-mix(in srgb, var(--rg-primary) 10%, transparent);
    margin-bottom: 0.875rem;
    font-size: 0.875rem;
    color: var(--rg-on-surface-variant);
  }

  .preview b {
    font-size: 1.0625rem;
    font-weight: 800;
    color: var(--rg-on-surface);
    letter-spacing: -0.02em;
  }

  .preview b.pos {
    color: var(--rg-price-pos);
  }

  .day-stepper {
    display: flex;
    align-items: center;
    gap: 0.625rem;
  }

  .step-btn {
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 0.625rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    color: var(--rg-on-surface);
    font-size: 1.25rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .day-value {
    min-width: 2.5rem;
    text-align: center;
    font-size: 1.125rem;
    font-weight: 700;
    color: var(--rg-on-surface);
  }

  .day-hint {
    font-size: 0.75rem;
    color: var(--rg-on-surface-variant);
    opacity: 0.7;
  }

  .advanced-toggle {
    width: 100%;
    padding: 0.75rem;
    border: none;
    border-radius: 0.75rem;
    background: transparent;
    color: var(--rg-on-surface-variant);
    font-size: 0.8125rem;
    font-family: inherit;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    margin-bottom: 0.5rem;
  }

  .advanced-toggle svg {
    transition: transform 0.2s ease;
  }

  .advanced-toggle svg.rotated {
    transform: rotate(180deg);
  }

  .cat-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .cat-opt {
    padding: 0.375rem 0.625rem;
    border-radius: 0.75rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    cursor: pointer;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8125rem;
    font-family: inherit;
    color: var(--rg-on-surface-variant);
  }

  .cat-opt :global(svg) {
    width: 1rem;
    height: 1rem;
    opacity: 0.7;
  }

  .cat-opt.selected {
    background: color-mix(in srgb, var(--cat-color) 15%, transparent);
    color: var(--cat-color);
    font-weight: 600;
  }

  .cat-opt.selected :global(svg) {
    opacity: 1;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1.25rem;
  }
</style>
