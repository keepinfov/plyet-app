<script lang="ts">
  import { slide } from 'svelte/transition';
  import { Sheet, Input, Button, SegmentedToggle } from 'reglass-material';
  import { store } from '$lib/stores/budget.svelte';
  import { categoryIcon } from '$lib/icons';
  import { sanitizeAmount, rublesStringToKopecks } from '$lib/utils';
  import type { Freq } from '$lib/types';
  import { hapticLight } from '$lib/haptics';
  import DatePicker from '$lib/components/DatePicker.svelte';

  // The modal is driven entirely by the store so it can be opened from the FAB,
  // a recurring card, or a virtual feed card's "edit".
  const show = $derived(store.showRecurringModal);
  const editData = $derived(store.editingRecurring);

  let itemType = $state<'expense' | 'income'>('expense');
  let name = $state('');
  let amount = $state(0);
  let amountDisplay = $state('');
  let nameInput = $state<HTMLInputElement | null>(null);
  let saving = $state(false);

  let freq = $state<Freq>('monthly');
  let anchorDay = $state(1);
  let startDate = $state('');
  let endDate = $state('');
  let hasEnd = $state(false);
  let horizon = $state(3);

  let showAdvanced = $state(false);
  let category = $state('other');
  let description = $state('');
  let link = $state('');

  const categories = $derived(store.data?.categories ?? []);

  const freqOptions = [
    { key: 'weekly' as const, label: 'Неделя' },
    { key: 'monthly' as const, label: 'Месяц' },
    { key: 'yearly' as const, label: 'Год' },
  ];

  const WEEKDAYS = ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Вс'];

  function todayStr(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }

  function onAmountInput() {
    const cleaned = sanitizeAmount(amountDisplay);
    if (cleaned !== amountDisplay) amountDisplay = cleaned;
    amount = rublesStringToKopecks(amountDisplay);
  }

  $effect(() => {
    if (!show) return;
    hapticLight();
    if (editData) {
      itemType = editData.item_type;
      name = editData.name;
      amount = editData.amount;
      amountDisplay = (editData.amount / 100).toFixed(2).replace(/\.?0+$/, '');
      freq = editData.freq;
      anchorDay = editData.anchor_day;
      startDate = editData.start_date;
      endDate = editData.end_date ?? '';
      hasEnd = editData.end_date !== null;
      horizon = editData.horizon;
      category = editData.category;
      description = editData.description || '';
      link = editData.link || '';
      showAdvanced = !!(editData.description || editData.link);
    } else {
      const d = new Date();
      itemType = store.currentTab === 'income' ? 'income' : 'expense';
      name = '';
      amount = 0;
      amountDisplay = '';
      freq = 'monthly';
      anchorDay = d.getDate();
      startDate = todayStr();
      endDate = '';
      hasEnd = false;
      horizon = 3;
      category = 'other';
      description = '';
      link = '';
      showAdvanced = false;
    }
    requestAnimationFrame(() => nameInput?.focus());
  });

  // Switching to weekly remaps an out-of-range day-of-month anchor to a weekday.
  function setFreq(f: Freq) {
    hapticLight();
    if (f === 'weekly' && (anchorDay < 0 || anchorDay > 6)) {
      const d = startDate ? new Date(startDate + 'T00:00:00') : new Date();
      anchorDay = (d.getDay() + 6) % 7;
    } else if (f !== 'weekly' && (anchorDay < 1 || anchorDay > 31)) {
      const d = startDate ? new Date(startDate + 'T00:00:00') : new Date();
      anchorDay = d.getDate();
    }
    freq = f;
  }

  function close() {
    store.closeRecurringModal();
  }

  const MAX_AMOUNT = 1_000_000_000_000;

  async function save() {
    if (!name.trim() || amount <= 0) {
      store.showSnackbar('Заполните название и сумму');
      return;
    }
    if (name.trim().length > 200) {
      store.showSnackbar('Название слишком длинное (макс. 200 символов)');
      return;
    }
    if (amount > MAX_AMOUNT) {
      store.showSnackbar('Сумма слишком большая');
      return;
    }
    if (hasEnd && endDate && endDate < startDate) {
      store.showSnackbar('Дата окончания раньше начала');
      return;
    }
    if (saving) return;
    saving = true;
    try {
      const payload = {
        name: name.trim(),
        amount,
        category,
        item_type: itemType,
        freq,
        anchor_day: anchorDay,
        start_date: startDate,
        end_date: hasEnd && endDate ? endDate : null,
        horizon,
        description: description.trim(),
        link: link.trim(),
      };
      if (editData) {
        await store.updateRecurring(editData.id, payload);
      } else {
        await store.addRecurring(payload);
      }
      close();
    } finally {
      saving = false;
    }
  }

  function remove() {
    if (!editData) return;
    const id = editData.id;
    store.showConfirm('Удалить этот регулярный платёж? Уже записанные операции останутся.', async () => {
      await store.deleteRecurring(id);
      close();
    });
  }
</script>

<Sheet show={show} onclose={close} scrollable>
  <div class="modal-header">
    <div class="modal-title">{editData ? 'Регулярный платёж' : 'Новый платёж'}</div>
    {#if editData}
      <button class="delete-btn tap-btn" onclick={remove} aria-label="Удалить">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="#D93025"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
      </button>
    {/if}
  </div>

  <div class="type-seg">
    <SegmentedToggle
      items={[{ key: 'expense', label: 'Расход' }, { key: 'income', label: 'Доход' }]}
      value={itemType}
      onchange={(k) => { hapticLight(); itemType = k as 'expense' | 'income'; }}
    />
  </div>

  <div class="form-group">
    <Input bind:element={nameInput} bind:value={name} placeholder="Название" />
  </div>

  <div class="form-group">
    <Input
      type="text"
      inputmode="decimal"
      bind:value={amountDisplay}
      oninput={onAmountInput}
      placeholder="0"
      style="font-size:1.75rem;font-weight:600;padding-right:2.5rem;letter-spacing:-0.03em;caret-color:var(--rg-primary)"
    />
    <span class="currency-hint">₽</span>
  </div>

  <div class="form-group">
    <span class="form-label">Периодичность</span>
    <SegmentedToggle items={freqOptions} value={freq} onchange={(k) => setFreq(k as Freq)} />
  </div>

  <div class="form-group">
    {#if freq === 'weekly'}
      <span class="form-label">День недели</span>
      <div class="weekday-row">
        {#each WEEKDAYS as wd, i}
          <button
            class="weekday-chip tap-btn"
            class:selected={anchorDay === i}
            onclick={() => { hapticLight(); anchorDay = i; }}
          >{wd}</button>
        {/each}
      </div>
    {:else}
      <span class="form-label">{freq === 'yearly' ? 'День (месяц — из даты начала)' : 'День месяца'}</span>
      <div class="day-stepper">
        <button class="step-btn tap-btn" onclick={() => { hapticLight(); anchorDay = Math.max(1, anchorDay - 1); }} aria-label="Меньше">−</button>
        <span class="day-value">{anchorDay}</span>
        <button class="step-btn tap-btn" onclick={() => { hapticLight(); anchorDay = Math.min(31, anchorDay + 1); }} aria-label="Больше">+</button>
        <span class="day-hint">если в месяце меньше дней — последний день</span>
      </div>
    {/if}
  </div>

  <div class="form-group">
    <span class="form-label">Первый платёж</span>
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
        <label class="toggle-row">
          <span class="form-label" style="margin:0">Дата окончания</span>
          <input type="checkbox" bind:checked={hasEnd} />
        </label>
        {#if hasEnd}
          <div transition:slide={{ duration: 200 }}>
            <DatePicker bind:value={endDate} />
          </div>
        {/if}
      </div>

      <div class="form-group">
        <label class="form-label" for="recur-description">Описание</label>
        <Input id="recur-description" bind:value={description} placeholder="Заметка..." />
      </div>

      <div class="form-group">
        <label class="form-label" for="recur-link">Ссылка</label>
        <Input id="recur-link" bind:value={link} placeholder="https://..." />
      </div>
    </div>
  {/if}

  <div class="actions">
    <Button variant="text" onclick={close} disabled={saving}>Отмена</Button>
    <Button variant="filled" onclick={() => { hapticLight(); save(); }} disabled={saving}>{saving ? '...' : (editData ? 'Сохранить' : 'Создать')}</Button>
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
    right: 1rem;
    bottom: 0.75rem;
    font-size: 1.75rem;
    font-weight: 600;
    color: var(--rg-on-surface-variant);
    pointer-events: none;
    letter-spacing: -0.03em;
  }

  .weekday-row {
    display: flex;
    gap: 0.375rem;
  }

  .weekday-chip {
    flex: 1;
    padding: 0.5rem 0;
    border-radius: 0.625rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    color: var(--rg-on-surface-variant);
    font-size: 0.8125rem;
    font-family: inherit;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .weekday-chip.selected {
    background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
    color: var(--rg-primary);
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
    min-width: 2rem;
    text-align: center;
    font-size: 1.125rem;
    font-weight: 700;
    color: var(--rg-on-surface);
  }

  .day-hint {
    font-size: 0.75rem;
    color: var(--rg-on-surface-variant);
    opacity: 0.7;
    line-height: 1.2;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
  }

  .toggle-row input {
    width: 1.1rem;
    height: 1.1rem;
    accent-color: var(--rg-primary);
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
