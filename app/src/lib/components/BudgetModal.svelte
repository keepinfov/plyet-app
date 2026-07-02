<script lang="ts">
  import { fly } from 'svelte/transition';
  import { tick } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { Sheet, Input, Button, IconButton, IconPicker } from 'reglass-material';
  import { store } from '$lib/stores/budget.svelte';
  import { formatMoney, rublesStringToKopecks, sanitizeAmount } from '$lib/utils';
  import { budgetIcon, DEFAULT_ICONS } from '$lib/icons';
  import { hapticLight } from '$lib/haptics';
  import type { Budget } from '$lib/types';

  interface Props {
    show?: boolean;
    onclose?: () => void;
  }

  let { show = $bindable(false), onclose }: Props = $props();

  $effect(() => {
    if (show) hapticLight();
  });

  const roots = $derived(store.roots);

  /** Spent/received across a root's whole subtree (its own items are normally empty post-migration). */
  function rootTotals(root: Budget) {
    const children = store.data?.budgets.filter(b => b.parent_id === root.id) ?? [];
    const items = [...root.items, ...children.flatMap(b => b.items)];
    const spent = items.filter(i => i.item_type !== 'income').reduce((s, i) => s + i.amount, 0);
    const income = items.filter(i => i.item_type === 'income').reduce((s, i) => s + i.amount, 0);
    return { spent, income };
  }

  let view = $state<'list' | 'form' | 'subform'>('list');
  let editBudgetId = $state<number | null>(null);
  let editSubId = $state<number | null>(null);
  let formName = $state('');
  let formLimitDisplay = $state('');
  let formIcon = $state('wallet');
  let formReflect = $state(false);

  function onLimitInput() {
    const cleaned = sanitizeAmount(formLimitDisplay);
    if (cleaned !== formLimitDisplay) formLimitDisplay = cleaned;
  }

  const icons = DEFAULT_ICONS;

  // Height animation for view transitions
  let viewContainerEl = $state<HTMLElement>(undefined!);
  let heightMounted = false;
  let heightTimer: ReturnType<typeof setTimeout> | null = null;

  $effect.pre(() => {
    void view;

    if (!viewContainerEl || !heightMounted) {
      heightMounted = true;
      return;
    }

    if (heightTimer) { clearTimeout(heightTimer); heightTimer = null; }
    viewContainerEl.style.height = '';

    const currentHeight = viewContainerEl.offsetHeight;
    viewContainerEl.style.height = `${currentHeight}px`;

    const currentView = view;
    tick().then(() => requestAnimationFrame(() => {
      if (!viewContainerEl) return;

      const views = viewContainerEl.querySelectorAll<HTMLElement>(':scope > .view');
      const hidden: HTMLElement[] = [];
      views.forEach(v => {
        if (v.dataset.view !== currentView) {
          v.style.display = 'none';
          hidden.push(v);
        }
      });

      viewContainerEl.style.height = 'auto';
      const newHeight = viewContainerEl.offsetHeight;

      hidden.forEach(v => v.style.display = '');

      viewContainerEl.style.height = `${currentHeight}px`;
      viewContainerEl.offsetHeight; // force reflow
      viewContainerEl.style.height = `${newHeight}px`;

      heightTimer = setTimeout(() => {
        if (viewContainerEl) viewContainerEl.style.height = '';
        heightTimer = null;
      }, 250);
    }));
  });

  function handleFormKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (view === 'subform') saveSubForm();
      else saveForm();
    }
  }

  function close() {
    onclose?.();
    show = false;
    if (heightTimer) { clearTimeout(heightTimer); heightTimer = null; }
    setTimeout(() => { view = 'list'; editBudgetId = null; editSubId = null; heightMounted = false; }, 300);
  }

  function select(id: number) { hapticLight(); store.selectRoot(id); close(); }

  function openNew() {
    hapticLight();
    editBudgetId = null;
    formName = '';
    formLimitDisplay = '';
    formIcon = 'wallet';
    view = 'form';
  }

  function openEdit(id: number) {
    hapticLight();
    const b = roots.find(b => b.id === id);
    if (!b) return;
    editBudgetId = id;
    formName = b.name;
    formLimitDisplay = String(b.limit / 100);
    formIcon = b.icon;
    view = 'form';
  }

  function openNewSub() {
    hapticLight();
    editSubId = null;
    formName = '';
    formLimitDisplay = '';
    formIcon = 'wallet';
    formReflect = false;
    view = 'subform';
  }

  function openEditSub(id: number) {
    hapticLight();
    const b = store.customBudgets.find(b => b.id === id);
    if (!b) return;
    editSubId = id;
    formName = b.name;
    formLimitDisplay = String(b.limit / 100);
    formIcon = b.icon;
    formReflect = b.reflect_in_months;
    view = 'subform';
  }

  function goBack() {
    hapticLight();
    view = 'list';
    editBudgetId = null;
    editSubId = null;
  }

  async function saveForm() {
    if (!formName.trim()) return;
    hapticLight();
    const limitKopecks = rublesStringToKopecks(formLimitDisplay);
    if (editBudgetId != null) {
      await store.updateBudget(editBudgetId, formName.trim(), limitKopecks, formIcon);
    } else {
      await store.createBudget(formName.trim(), limitKopecks, formIcon);
    }
    view = 'list';
    editBudgetId = null;
    close();
  }

  async function saveSubForm() {
    if (!formName.trim()) return;
    hapticLight();
    const limitKopecks = rublesStringToKopecks(formLimitDisplay);
    if (editSubId != null) {
      await store.updateBudget(editSubId, formName.trim(), limitKopecks, formIcon, formReflect);
    } else {
      await store.createSubBudget(formName.trim(), limitKopecks, formIcon, formReflect);
    }
    view = 'list';
    editSubId = null;
    close();
  }

  function handleClose() {
    if (view !== 'list') goBack();
    else close();
  }
</script>

{#snippet budgetIconSnippet(ic: string)}{@html budgetIcon(ic)}{/snippet}

<Sheet show={show} onclose={handleClose} scrollable>
  <div class="view-container" bind:this={viewContainerEl}>

      {#if view === 'list'}
        <div class="view" data-view="list" in:fly={{ x: -60, duration: 200, easing: cubicOut }} out:fly={{ x: -60, duration: 150 }}>
          <div class="modal-header">
            <div class="modal-title">Бюджеты</div>
          </div>

          {#each roots as budget}
            {@const totals = rootTotals(budget)}
            <div class="budget-item tap-dim" class:selected={budget.id === store.currentRootId} onclick={() => select(budget.id)} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && select(budget.id)}>
              <div class="budget-icon" class:icon-selected={budget.id === store.currentRootId}>{@html budgetIcon(budget.icon)}</div>
              <div class="budget-info">
                <div class="budget-name">{budget.name}</div>
                <div class="budget-amount">{formatMoney(totals.spent)} / {formatMoney(budget.limit + totals.income)}</div>
              </div>
              <div class="budget-actions">
                <button class="edit-budget-btn tap-btn" onclick={(e) => { e.stopPropagation(); openEdit(budget.id); }} aria-label="Редактировать бюджет">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
                </button>
                {#if roots.length > 1}
                  <button class="delete-budget-btn tap-btn" onclick={(e) => { e.stopPropagation(); store.showConfirm('Удалить этот бюджет со всеми его месяцами и записями?', () => store.deleteBudget(budget.id)); }} aria-label="Удалить бюджет">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
                  </button>
                {/if}
              </div>
            </div>
          {/each}

          <button class="new-btn tap-btn" onclick={openNew}>
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
            Новый бюджет
          </button>

          {#if store.currentRoot}
            <div class="sub-section-header">Под-бюджеты «{store.currentRoot.name}»</div>
            {#each store.customBudgets as sub}
              <div class="budget-item sub-item tap-dim" onclick={() => { store.setScope(`custom:${sub.id}`); close(); }} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && (store.setScope(`custom:${sub.id}`), close())}>
                <div class="budget-icon">{@html budgetIcon(sub.icon)}</div>
                <div class="budget-info">
                  <div class="budget-name">{sub.name}</div>
                  <div class="budget-amount">{sub.reflect_in_months ? 'Отражается в месяцах' : 'Отдельно от месяцев'}</div>
                </div>
                <div class="budget-actions">
                  <button class="edit-budget-btn tap-btn" onclick={(e) => { e.stopPropagation(); openEditSub(sub.id); }} aria-label="Редактировать под-бюджет">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
                  </button>
                  <button class="delete-budget-btn tap-btn" onclick={(e) => { e.stopPropagation(); store.showConfirm('Удалить этот под-бюджет со всеми записями?', () => store.deleteBudget(sub.id)); }} aria-label="Удалить под-бюджет">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
                  </button>
                </div>
              </div>
            {/each}
            <button class="new-btn sub-new-btn tap-btn" onclick={openNewSub}>
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
              Новый под-бюджет
            </button>
          {/if}
        </div>
      {/if}

      {#if view === 'form'}
        <div class="view" data-view="form" in:fly={{ x: 60, duration: 200, easing: cubicOut }} out:fly={{ x: 60, duration: 150 }}>
          <div class="sub-header">
            <IconButton variant="surface" onclick={goBack} aria-label="Назад">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>
            </IconButton>
            <div class="sub-title">{editBudgetId != null ? 'Редактировать' : 'Новый бюджет'}</div>
          </div>

          <div class="form-group">
            <label class="form-label" for="budget-name">Название</label>
            <Input id="budget-name" bind:value={formName} placeholder="Например: Семья" onkeydown={handleFormKeydown} />
          </div>

          <div class="form-group">
            <label class="form-label" for="budget-limit">Лимит по умолчанию для месяцев (₽)</label>
            <Input id="budget-limit" type="text" inputmode="decimal" bind:value={formLimitDisplay} oninput={onLimitInput} onkeydown={handleFormKeydown} placeholder="50000" />
          </div>

          <div class="form-group">
            <span class="form-label">Иконка</span>
            <IconPicker icons={icons} value={formIcon} renderIcon={budgetIconSnippet} onchange={(ic) => { hapticLight(); formIcon = ic; }} />
          </div>

          <div class="actions">
            <Button variant="text" onclick={goBack}>Отмена</Button>
            <Button variant="filled" onclick={saveForm}>{editBudgetId != null ? 'Сохранить' : 'Создать'}</Button>
          </div>
        </div>
      {/if}

      {#if view === 'subform'}
        <div class="view" data-view="subform" in:fly={{ x: 60, duration: 200, easing: cubicOut }} out:fly={{ x: 60, duration: 150 }}>
          <div class="sub-header">
            <IconButton variant="surface" onclick={goBack} aria-label="Назад">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>
            </IconButton>
            <div class="sub-title">{editSubId != null ? 'Редактировать под-бюджет' : 'Новый под-бюджет'}</div>
          </div>

          <div class="form-group">
            <label class="form-label" for="sub-name">Название</label>
            <Input id="sub-name" bind:value={formName} placeholder="Например: Отпуск" onkeydown={handleFormKeydown} />
          </div>

          <div class="form-group">
            <label class="form-label" for="sub-limit">Лимит (₽, необязательно)</label>
            <Input id="sub-limit" type="text" inputmode="decimal" bind:value={formLimitDisplay} oninput={onLimitInput} onkeydown={handleFormKeydown} placeholder="0" />
          </div>

          <div class="form-group">
            <span class="form-label">Иконка</span>
            <IconPicker icons={icons} value={formIcon} renderIcon={budgetIconSnippet} onchange={(ic) => { hapticLight(); formIcon = ic; }} />
          </div>

          <label class="reflect-row tap-dim">
            <input type="checkbox" bind:checked={formReflect} />
            <span>
              <span class="reflect-title">Отражать записи в месяцах</span>
              <span class="reflect-hint">Траты этого под-бюджета будут также показаны в соответствующем месяце по их дате (с пометкой источника)</span>
            </span>
          </label>

          <div class="actions">
            <Button variant="text" onclick={goBack}>Отмена</Button>
            <Button variant="filled" onclick={saveSubForm}>{editSubId != null ? 'Сохранить' : 'Создать'}</Button>
          </div>
        </div>
      {/if}

  </div>
</Sheet>

<style>
  .view-container {
    display: grid;
    grid-template: 1fr / 1fr;
    overflow: hidden;
    transition: height 220ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  .view {
    grid-area: 1 / 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
  }

  /* Header */
  .modal-header {
    margin-bottom: 1.25rem;
  }

  .modal-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    letter-spacing: -0.02em;
  }

  /* Sub-view header */
  .sub-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1.25rem;
  }

  .sub-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    letter-spacing: -0.02em;
  }

  .sub-section-header {
    font-size: 0.7rem;
    font-weight: 700;
    color: var(--rg-on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin: 1.25rem 0 0.375rem 0.25rem;
  }

  /* Budget list items */
  .budget-item {
    display: flex;
    align-items: center;
    gap: 0.875rem;
    padding: 0.875rem 1rem;
    border-radius: 1rem;
    cursor: pointer;
    transition: background 0.15s ease, box-shadow 0.15s ease;
    margin-bottom: 0.25rem;
  }

  .budget-item:hover { background: color-mix(in srgb, var(--rg-primary) 6%, transparent); }

  .budget-item.selected {
    background: color-mix(in srgb, var(--rg-primary) 12%, transparent);
  }

  .sub-item {
    padding: 0.625rem 1rem;
  }

  .budget-icon {
    width: 2.75rem;
    height: 2.75rem;
    border-radius: 1rem;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.375rem;
    background: var(--rg-surface);
  }

  .budget-icon :global(svg) {
    width: 1.375rem;
    height: 1.375rem;
  }

  .budget-icon.icon-selected {
    background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
    color: var(--rg-primary);
  }

  .budget-icon.icon-selected :global(svg) {
    fill: var(--rg-primary);
  }

  .budget-info { flex: 1; min-width: 0; }

  .budget-name {
    font-size: 1rem;
    font-weight: 500;
    color: var(--rg-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .budget-amount {
    font-size: 0.75rem;
    color: var(--rg-on-surface-variant);
    margin-top: 0.125rem;
  }

  .budget-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .edit-budget-btn, .delete-budget-btn {
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 50%;
    border: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .edit-budget-btn:hover { background: var(--rg-surface); }
  .delete-budget-btn:hover { background: var(--rg-danger-glass); }
  .delete-budget-btn:hover :global(svg) { fill: var(--rg-danger); }

  .new-btn {
    width: 100%;
    padding: 0.875rem;
    border-radius: 1rem;
    border: none;
    background: color-mix(in srgb, var(--rg-primary) 8%, transparent);
    color: var(--rg-primary);
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
    font-weight: 600;
    margin-top: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    transition: background 0.15s ease;
  }

  .new-btn:hover {
    background: color-mix(in srgb, var(--rg-primary) 14%, transparent);
  }

  .sub-new-btn {
    margin-top: 0.25rem;
  }

  /* Form */
  .form-group { margin-bottom: 1rem; }

  .form-label {
    font-size: 0.7rem;
    font-weight: 700;
    color: var(--rg-primary);
    margin-bottom: 0.3125rem;
    display: block;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .reflect-row {
    display: flex;
    align-items: flex-start;
    gap: 0.625rem;
    padding: 0.75rem;
    border-radius: 0.75rem;
    background: var(--rg-surface-2);
    cursor: pointer;
    margin-bottom: 0.5rem;
  }

  .reflect-row input[type="checkbox"] {
    margin-top: 0.1875rem;
    flex-shrink: 0;
  }

  .reflect-title {
    display: block;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--rg-on-surface);
  }

  .reflect-hint {
    display: block;
    font-size: 0.75rem;
    color: var(--rg-on-surface-variant);
    margin-top: 0.125rem;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1rem;
  }
</style>
