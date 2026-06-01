<script lang="ts">
  import { slide } from 'svelte/transition';
  import { Sheet, Input, Button } from 'reglass-material';
  import { store } from '$lib/stores/budget.svelte';
  import { categoryIcon, CATEGORY_ICON_KEYS } from '$lib/icons';
  import { sanitizeAmount, rublesStringToKopecks } from '$lib/utils';
  import type { Item } from '$lib/types';
  import { hapticLight } from '$lib/haptics';
  import DatePicker from '$lib/components/DatePicker.svelte';

  interface Props {
    show?: boolean;
    editData?: Item | null;
  }

  let { show = $bindable(false), editData = $bindable(null) }: Props = $props();

  $effect(() => {
    if (show) hapticLight();
  });

  let itemType = $state<'expense' | 'income'>('expense');
  let name = $state('');
  let amount = $state(0);
  let amountDisplay = $state('');
  let nameInput = $state<HTMLInputElement | null>(null);

  function onAmountInput() {
    const cleaned = sanitizeAmount(amountDisplay);
    if (cleaned !== amountDisplay) amountDisplay = cleaned;
    amount = rublesStringToKopecks(amountDisplay);
  }

  let saving = $state(false);

  // Advanced fields — hidden by default
  let showAdvanced = $state(false);
  let date = $state('');
  let category = $state('other');
  let description = $state('');
  let link = $state('');

  // Type toggle pill
  let toggleContainerEl = $state<HTMLDivElement>(undefined!);
  const typeBtnEls: HTMLButtonElement[] = [];
  let typePillLeft = $state(0);
  let typePillWidth = $state(0);

  $effect(() => {
    void itemType;
    void show;
    requestAnimationFrame(() => requestAnimationFrame(() => {
      const idx = itemType === 'expense' ? 0 : 1;
      const el = typeBtnEls[idx];
      if (el && toggleContainerEl) {
        typePillLeft = el.offsetLeft;
        typePillWidth = el.offsetWidth;
      }
    }));
  });

  const categories = $derived(store.data?.categories ?? []);

  // Inline category creation
  let creatingCategory = $state(false);
  let newCatName = $state('');
  let newCatIcon = $state('other');
  let newCatColor = $state('#00ACC1');

  const ADD_COLORS = [
    '#FF6D00', '#1A73E8', '#D93025', '#E91E63',
    '#7C4DFF', '#0D904F', '#00ACC1', '#F9AB00',
    '#795548', '#607D8B', '#FF5722', '#4CAF50',
    '#9C27B0', '#009688', '#3F51B5', '#FF9800',
  ];

  function startCreateCategory() {
    hapticLight();
    if (creatingCategory) { creatingCategory = false; return; }
    creatingCategory = true;
    newCatName = '';
    newCatIcon = 'other';
    newCatColor = '#00ACC1';
  }

  async function saveNewCategory() {
    if (!newCatName.trim()) return;
    hapticLight();
    const key = newCatName.trim().toLowerCase().replace(/[^a-zа-яё0-9]/gi, '_').substring(0, 32) || 'custom';
    let finalKey = key;
    let suffix = 1;
    while (categories.some(c => c.key === finalKey)) {
      finalKey = `${key}_${suffix++}`;
    }
    await store.addCategory(finalKey, newCatName.trim(), newCatIcon, newCatColor);
    category = finalKey;
    creatingCategory = false;
  }

  $effect(() => {
    if (show) {
      if (editData) {
        name = editData.name;
        amount = editData.amount; // Already in kopecks
        amountDisplay = (editData.amount / 100).toFixed(2).replace(/\.?0+$/, '');
        category = editData.category;
        date = editData.date;
        description = editData.description || '';
        link = editData.link;
        itemType = editData.item_type;
        const td = new Date();
        const todayStr = `${td.getFullYear()}-${String(td.getMonth() + 1).padStart(2, '0')}-${String(td.getDate()).padStart(2, '0')}`;
        showAdvanced = !!(editData.link || editData.description || editData.date !== todayStr);
      } else {
        name = '';
        amount = 0;
        amountDisplay = '';
        category = 'other';
        const d = new Date();
        date = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
        description = '';
        link = '';
        itemType = store.currentTab === 'income' ? 'income' : 'expense';
        showAdvanced = false;
      }
    }
  });

  $effect(() => {
    if (show) {
      requestAnimationFrame(() => nameInput?.focus());
    }
  });

  function handleFormKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      save();
    }
  }

  function close() {
    show = false;
    editData = null;
  }

  // Mirror the backend limits (src-tauri/src/validate.rs) so the user gets a
  // specific message instead of a generic "Ошибка" after a rejected invoke.
  const MAX_AMOUNT = 1_000_000_000_000; // kopecks (~10 млрд ₽)

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
    if (description.trim().length > 2000) {
      store.showSnackbar('Описание слишком длинное (макс. 2000 символов)');
      return;
    }
    if (link.trim().length > 2000) {
      store.showSnackbar('Ссылка слишком длинная (макс. 2000 символов)');
      return;
    }
    if (saving) return;
    saving = true;
    try {
      const payload = {
        name: name.trim(),
        amount, // Already in kopecks from rublesStringToKopecks
        category,
        date,
        description: description.trim(),
        link: link.trim(),
        item_type: itemType,
      };
      if (editData) {
        await store.updateItem(editData.id, payload);
      } else {
        await store.addItem(payload);
      }
      close();
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!editData) return;
    const itemId = editData.id;
    store.showConfirm('Удалить эту запись?', async () => {
      await store.deleteItem(itemId);
      close();
    });
  }

</script>

<Sheet show={show} onclose={close} scrollable>
      <div class="modal-header">
        <div class="modal-title">{editData ? 'Изменить' : 'Добавить'}</div>
        {#if editData}
          <button class="delete-btn tap-btn" onclick={remove} aria-label="Удалить">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="#D93025"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
          </button>
        {/if}
      </div>

      <!-- Expense / Income toggle -->
      <div class="type-toggle" bind:this={toggleContainerEl}>
        <div class="type-pill" style="left: {typePillLeft}px; width: {typePillWidth}px;"></div>
        <button bind:this={typeBtnEls[0]} class="type-btn tap-dim" class:active={itemType === 'expense'} class:active-expense={itemType === 'expense'} onclick={() => { hapticLight(); itemType = 'expense'; }}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M11 5v11.17l-4.88-4.88c-.39-.39-1.03-.39-1.42 0-.39.39-.39 1.02 0 1.41l6.59 6.59c.39.39 1.02.39 1.41 0l6.59-6.59c.39-.39.39-1.02 0-1.41a.996.996 0 00-1.41 0L13 16.17V5c0-.55-.45-1-1-1s-1 .45-1 1z"/></svg>
          Расход
        </button>
        <button bind:this={typeBtnEls[1]} class="type-btn tap-dim" class:active={itemType === 'income'} class:active-income={itemType === 'income'} onclick={() => { hapticLight(); itemType = 'income'; }}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M13 19V7.83l4.88 4.88c.39.39 1.03.39 1.42 0 .39-.39.39-1.02 0-1.41l-6.59-6.59a.996.996 0 00-1.41 0l-6.6 6.58c-.39.39-.39 1.02 0 1.41.39.39 1.02.39 1.41 0L11 7.83V19c0 .55.45 1 1 1s1-.45 1-1z"/></svg>
          Доход
        </button>
      </div>

      <!-- Always visible: name + amount -->
      <div class="form-group">
        <Input bind:element={nameInput} bind:value={name} placeholder="Название" onkeydown={handleFormKeydown} />
      </div>

      <div class="form-group">
        <Input
          type="text"
          inputmode="decimal"
          bind:value={amountDisplay}
          oninput={onAmountInput}
          onkeydown={handleFormKeydown}
          placeholder="0"
          style="font-size:1.75rem;font-weight:600;padding-right:2.5rem;letter-spacing:-0.03em;caret-color:var(--rg-primary)"
        />
        <span class="currency-hint">₽</span>
      </div>

      <!-- Advanced toggle -->
      <button class="advanced-toggle" onclick={() => showAdvanced = !showAdvanced}>
        <span>Дополнительно</span>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round" class:rotated={showAdvanced}><path d="M7.41 8.59L12 13.17l4.59-4.58L18 10l-6 6-6-6 1.41-1.41z"/></svg>
      </button>

      <!-- Advanced section -->
      {#if showAdvanced}
        <div class="advanced-section" transition:slide={{ duration: 250 }}>
          <div class="form-group">
            <span class="form-label">Дата</span>
            <DatePicker bind:value={date} />
          </div>

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
              <button
                class="cat-opt cat-add tap-btn"
                onclick={startCreateCategory}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
                <span>Добавить</span>
              </button>
            </div>

            {#if creatingCategory}
              <div class="cat-create-form" transition:slide={{ duration: 200 }}>
                <Input bind:value={newCatName} placeholder="Название категории" style="padding:0.625rem 0.75rem;font-size:0.875rem" />
                <div class="cat-create-icons">
                  {#each CATEGORY_ICON_KEYS as ic}
                    <button class="cat-create-icon-opt tap-btn" class:selected={newCatIcon === ic} onclick={() => { hapticLight(); newCatIcon = ic; }}>
                      {@html categoryIcon(ic)}
                    </button>
                  {/each}
                </div>
                <div class="cat-create-colors">
                  {#each ADD_COLORS as c}
                    <button
                      class="cat-create-color-opt tap-btn"
                      class:selected={newCatColor === c}
                      style="background: {c}"
                      onclick={() => { hapticLight(); newCatColor = c; }}
                    >
                      {#if newCatColor === c}
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="white"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/></svg>
                      {/if}
                    </button>
                  {/each}
                </div>
                <div class="cat-create-actions">
                  <button class="btn-text-sm tap-btn" onclick={() => creatingCategory = false}>Отмена</button>
                  <button class="btn-filled-sm tap-btn" onclick={saveNewCategory}>Создать</button>
                </div>
              </div>
            {/if}
          </div>

          <div class="form-group">
            <label class="form-label" for="add-description">Описание</label>
            <Input id="add-description" bind:value={description} placeholder="Заметка..." />
          </div>

          <div class="form-group">
            <label class="form-label" for="add-link">Ссылка</label>
            <Input id="add-link" bind:value={link} placeholder="https://..." />
          </div>
        </div>
      {/if}

      <div class="actions">
        <Button variant="text" onclick={close} disabled={saving}>Отмена</Button>
        <Button variant="filled" onclick={() => { hapticLight(); save(); }} disabled={saving}>{saving ? '...' : (editData ? 'Сохранить' : 'Добавить')}</Button>
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
    transition: background 0.15s ease;
  }

  .delete-btn:hover { background: var(--rg-danger-glass); filter: brightness(0.85); }

  .modal-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    letter-spacing: -0.02em;
  }

  .type-toggle {
    display: flex;
    gap: 0;
    margin-bottom: 1rem;
    background: var(--rg-surface);
    border-radius: 0.75rem;
    padding: 0.1875rem;
    position: relative;
    overflow: hidden;
    height: 2.375rem;
  }

  .type-pill {
    position: absolute;
    top: 0.1875rem;
    height: calc(100% - 0.375rem);
    background: var(--rg-pill-bg);
    border-radius: 0.5625rem;
    box-shadow: 0 1px 4px rgba(0,0,0,0.12);
    transition: left 0.12s var(--rg-spring-easing), width 0.12s var(--rg-spring-easing);
    will-change: left, width;
    pointer-events: none;
    z-index: 0;
  }

  .type-btn {
    flex: 1;
    padding: 0 1rem;
    border-radius: 0.5625rem;
    border: none;
    background: transparent;
    color: var(--rg-on-surface-variant);
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    transition: color 0.2s ease;
    position: relative;
    z-index: 1;
  }

  .type-btn.active-expense {
    color: var(--rg-danger);
    font-weight: 700;
  }

  .type-btn.active-income {
    color: var(--rg-price-pos);
    font-weight: 700;
  }

  .form-group {
    margin-bottom: 0.875rem;
    position: relative;
  }

  .form-label {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--rg-on-surface-variant);
    margin-bottom: 0.25rem;
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
    transition: color 0.15s ease;
    margin-bottom: 0.5rem;
  }

  .advanced-toggle:hover {
    color: var(--rg-primary);
  }

  .advanced-toggle svg {
    transition: transform 0.2s ease;
  }

  .advanced-toggle svg.rotated {
    transform: rotate(180deg);
  }

  .advanced-section {
    overflow: visible;
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

  .cat-add {
    background: color-mix(in srgb, var(--rg-primary) 10%, transparent);
    color: var(--rg-primary);
    font-weight: 600;
  }

  .cat-add:hover {
    background: color-mix(in srgb, var(--rg-primary) 16%, transparent);
  }

  .cat-create-form {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background: var(--rg-surface);
    border-radius: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .cat-create-icons {
    display: flex;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .cat-create-icon-opt {
    width: 2rem;
    height: 2rem;
    border-radius: 0.5rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    color: var(--rg-on-surface-variant);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .cat-create-icon-opt :global(svg) {
    width: 1rem;
    height: 1rem;
  }

  .cat-create-icon-opt.selected {
    background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
    color: var(--rg-primary);
  }

  .cat-create-icon-opt.selected :global(svg) {
    fill: var(--rg-primary);
  }

  .cat-create-colors {
    display: grid;
    grid-template-columns: repeat(auto-fill, 1.625rem);
    gap: 0.375rem;
    justify-content: center;
  }

  .cat-create-color-opt {
    width: 1.625rem;
    height: 1.625rem;
    border-radius: 0.375rem;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .cat-create-color-opt.selected {
    box-shadow: 0 0 0 2px var(--rg-sheet-bg), 0 0 0 4px var(--rg-on-surface);
  }

  .cat-create-actions {
    display: flex;
    gap: 0.375rem;
    justify-content: flex-end;
  }

  .btn-text-sm {
    padding: 0.5rem 1rem;
    border-radius: 999rem;
    border: none;
    background: transparent;
    color: var(--rg-primary);
    font-size: 0.8125rem;
    font-family: inherit;
    cursor: pointer;
    font-weight: 600;
  }

  .btn-filled-sm {
    padding: 0.5rem 1.25rem;
    border-radius: 999rem;
    border: none;
    background: var(--rg-primary);
    color: var(--rg-on-primary);
    font-size: 0.8125rem;
    font-family: inherit;
    cursor: pointer;
    font-weight: 600;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1.25rem;
  }
</style>
