<script lang="ts">
  import { fly, slide } from 'svelte/transition';
  import { tick } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { Sheet, Input, Button, IconButton, ListRow, TogglePill, ColorPicker, IconPicker } from 'reglass-material';
  import { store } from '$lib/stores/budget.svelte';
  import { categoryIcon, CATEGORY_ICON_KEYS } from '$lib/icons';
  import { hapticLight } from '$lib/haptics';

  // Lazy-load plugins to avoid module-level failures on Android
  const getSave = async () => (await import('@tauri-apps/plugin-dialog')).save;
  const getOpen = async () => (await import('@tauri-apps/plugin-dialog')).open;
  const getWriteTextFile = async () => (await import('@tauri-apps/plugin-fs')).writeTextFile;
  const getReadTextFile = async () => (await import('@tauri-apps/plugin-fs')).readTextFile;

  interface Props {
    show?: boolean;
  }

  let { show = $bindable(false) }: Props = $props();
  let importing = $state(false);
  let view = $state<'main' | 'appearance' | 'data' | 'categories'>('main');

  // Categories sub-view state
  const categories = $derived(store.data?.categories ?? []);
  let editing = $state<string | null>(null);
  let editName = $state('');
  let editIcon = $state('other');
  let editColor = $state('#00ACC1');
  let editKey = $state('');

  const COLORS = [
    '#FF6D00', '#1A73E8', '#D93025', '#E91E63',
    '#7C4DFF', '#0D904F', '#00ACC1', '#F9AB00',
    '#795548', '#607D8B', '#FF5722', '#4CAF50',
    '#9C27B0', '#009688', '#3F51B5', '#FF9800',
    '#E040FB', '#00E5FF', '#76FF03', '#FFD740',
    '#FF1744', '#651FFF', '#00B8D4', '#2E7D32',
    '#C62828', '#AD1457', '#4527A0', '#00695C',
    '#EF6C00', '#37474F', '#455A64', '#78909C',
  ];

  // Height animation for view transitions
  let viewContainerEl = $state<HTMLElement>(undefined!);
  let heightMounted = false;
  let heightTimer: ReturnType<typeof setTimeout> | null = null;

  $effect.pre(() => {
    void view;
    void editing;

    if (!viewContainerEl || !heightMounted) {
      heightMounted = true;
      return;
    }

    // Cancel any pending cleanup from previous animation
    if (heightTimer) { clearTimeout(heightTimer); heightTimer = null; }
    viewContainerEl.style.height = '';

    const currentHeight = viewContainerEl.offsetHeight;
    viewContainerEl.style.height = `${currentHeight}px`;

    const currentView = view;
    tick().then(() => requestAnimationFrame(() => {
      if (!viewContainerEl) return;

      // Hide outgoing views to measure only the new one
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

      // Restore hidden views
      hidden.forEach(v => v.style.display = '');

      viewContainerEl.style.height = `${currentHeight}px`;
      viewContainerEl.offsetHeight; // force reflow
      viewContainerEl.style.height = `${newHeight}px`;

      // Clean up after transition duration (220ms + buffer)
      heightTimer = setTimeout(() => {
        if (viewContainerEl) viewContainerEl.style.height = '';
        heightTimer = null;
      }, 250);
    }));
  });

  function startCatAdd() {
    editing = 'new';
    editName = '';
    editIcon = 'other';
    editColor = '#00ACC1';
    editKey = '';
  }

  function startCatEdit(key: string) {
    const cat = categories.find(c => c.key === key);
    if (!cat) return;
    editing = key;
    editName = cat.name;
    editIcon = cat.icon;
    editColor = cat.color;
    editKey = key;
  }

  async function saveCat() {
    if (!editName.trim()) return;
    hapticLight();
    if (editing === 'new') {
      const key = editName.trim().toLowerCase().replace(/[^a-zа-яё0-9]/gi, '_').substring(0, 32) || 'custom';
      let finalKey = key;
      let suffix = 1;
      while (categories.some(c => c.key === finalKey)) {
        finalKey = `${key}_${suffix++}`;
      }
      await store.addCategory(finalKey, editName.trim(), editIcon, editColor);
    } else if (editing) {
      await store.updateCategory(editing, editName.trim(), editIcon, editColor);
    }
    editing = null;
  }

  function confirmDeleteCat(key: string) {
    store.showConfirm('Удалить эту категорию? Записи будут перенесены в "Другое".', () => {
      store.deleteCategory(key);
    });
  }

  function close() {
    show = false;
    if (heightTimer) { clearTimeout(heightTimer); heightTimer = null; }
    setTimeout(() => { view = 'main'; editing = null; heightMounted = false; }, 300);
  }

  function handleClose() {
    if (editing) editing = null;
    else if (view !== 'main') goBack();
    else close();
  }

  function goBack() {
    hapticLight();
    if (editing) { editing = null; return; }
    view = 'main';
  }

  function openAppearance() {
    hapticLight();
    view = 'appearance';
  }

  function openData() {
    hapticLight();
    view = 'data';
  }

  function openCategories() {
    hapticLight();
    view = 'categories';
    editing = null;
  }

  async function handleExportCsv() {
    hapticLight();
    const csv = await store.exportCsv();
    if (!csv) return;
    try {
      const save = await getSave();
      const budgetName = (store.currentBudget?.name ?? 'budget').replace(/[^a-zA-Zа-яА-Я0-9_-]/g, '_');
      const filePath = await save({
        defaultPath: `${budgetName}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }]
      });
      if (!filePath) return;
      const writeTextFile = await getWriteTextFile();
      await writeTextFile(filePath, csv);
      store.showSnackbar('CSV экспортирован');
    } catch (e) {
      store.addLog('err', `export csv failed: ${e}`);
      store.showSnackbar('Ошибка экспорта CSV');
    }
  }

  async function handleExportJson() {
    hapticLight();
    const json = await store.exportJson();
    if (!json) return;
    try {
      const save = await getSave();
      const filePath = await save({
        defaultPath: 'plyet-backup.json',
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });
      if (!filePath) return;
      const writeTextFile = await getWriteTextFile();
      await writeTextFile(filePath, json);
      store.showSnackbar('JSON экспортирован');
    } catch (e) {
      store.addLog('err', `export json failed: ${e}`);
      store.showSnackbar('Ошибка экспорта JSON');
    }
  }

  async function handleExportLogs() {
    hapticLight();
    const text = store.logs.map(l => `[${l.ts}] [${l.level.toUpperCase()}] ${l.msg}`).join('\n');
    if (!text) {
      store.showSnackbar('Логи пусты');
      return;
    }
    try {
      const save = await getSave();
      const filePath = await save({
        defaultPath: 'plyet-debug.log',
        filters: [{ name: 'Log', extensions: ['log', 'txt'] }]
      });
      if (!filePath) return;
      const writeTextFile = await getWriteTextFile();
      await writeTextFile(filePath, text);
      store.showSnackbar('Логи экспортированы');
    } catch (e) {
      store.addLog('err', `export logs failed: ${e}`);
      store.showSnackbar('Ошибка экспорта логов');
    }
  }

  async function handleImportJson() {
    hapticLight();
    const open = await getOpen();
    const filePath = await open({
      filters: [{ name: 'JSON', extensions: ['json'] }]
    });
    if (!filePath) return;
    let text: string;
    try {
      const readTextFile = await getReadTextFile();
      text = await readTextFile(filePath as string);
    } catch (e) {
      store.showSnackbar('Ошибка чтения файла: ' + String(e));
      return;
    }
    // The actual import happens after the user confirms, so the loading state
    // must live inside the callback to reflect the real operation.
    store.showConfirm('Импорт заменит все текущие данные. Продолжить?', async () => {
      importing = true;
      try {
        await store.importJson(text);
        close();
      } finally {
        importing = false;
      }
    }, 'Импортировать');
  }

</script>

{#snippet catIconSnippet(ic: string)}{@html categoryIcon(ic)}{/snippet}

<Sheet show={show} onclose={handleClose} scrollable>
      <div class="view-container" bind:this={viewContainerEl}>

      {#if view === 'main'}
        <div class="view" data-view="main" in:fly={{ x: -60, duration: 200, easing: cubicOut }} out:fly={{ x: -60, duration: 150 }}>
          <div class="modal-header">
            <div class="modal-title">Настройки</div>
          </div>

          <div class="nav-list">
            <ListRow variant="nav" chevron title="Оформление" description="Тема, акцент, размытие" onclick={openAppearance}>
              {#snippet leading()}
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M12 3c-4.97 0-9 4.03-9 9s4.03 9 9 9c.83 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1.01-.23-.26-.38-.61-.38-1 0-.83.67-1.5 1.5-1.5H16c2.76 0 5-2.24 5-5 0-4.42-4.03-8-9-8zm-5.5 9c-.83 0-1.5-.67-1.5-1.5S5.67 9 6.5 9 8 9.67 8 10.5 7.33 12 6.5 12zm3-4C8.67 8 8 7.33 8 6.5S8.67 5 9.5 5s1.5.67 1.5 1.5S10.33 8 9.5 8zm5 0c-.83 0-1.5-.67-1.5-1.5S13.67 5 14.5 5s1.5.67 1.5 1.5S15.33 8 14.5 8zm3 4c-.83 0-1.5-.67-1.5-1.5S16.67 9 17.5 9s1.5.67 1.5 1.5-.67 1.5-1.5 1.5z"/></svg>
              {/snippet}
            </ListRow>

            <ListRow variant="nav" chevron title="Категории" description="Управление категориями" onclick={openCategories}>
              {#snippet leading()}
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M12 2l-5.5 9h11L12 2zm0 3.84L13.93 9h-3.87L12 5.84zM17.5 13c-2.49 0-4.5 2.01-4.5 4.5s2.01 4.5 4.5 4.5 4.5-2.01 4.5-4.5-2.01-4.5-4.5-4.5zm0 7c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5zM3 21.5h8v-8H3v8zm2-6h4v4H5v-4z"/></svg>
              {/snippet}
            </ListRow>

            <ListRow variant="nav" chevron title="Данные" description="Экспорт и импорт" onclick={openData}>
              {#snippet leading()}
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 12v7H5v-7H3v7c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2v-7h-2zm-6 .67l2.59-2.58L17 11.5l-5 5-5-5 1.41-1.41L11 12.67V3h2v9.67z"/></svg>
              {/snippet}
            </ListRow>
          </div>

          <div class="about-section">
            <span class="about-name">Plyet</span>
            <span class="about-dot">·</span>
            <span class="about-version">v0.1.0 ({__BUILD_TIMESTAMP__})</span>
          </div>
        </div>
      {/if}

      {#if view === 'appearance'}
        <div class="view" data-view="appearance" in:fly={{ x: 60, duration: 200, easing: cubicOut }} out:fly={{ x: 60, duration: 150 }}>
          <div class="sub-header">
            <IconButton variant="surface" onclick={goBack} aria-label="Назад">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>
            </IconButton>
            <div class="sub-title">Оформление</div>
          </div>

          <div class="setting-group">
            <ListRow title="Тема" onclick={() => { hapticLight(); store.toggleTheme(); }}>
              {#snippet leading()}
                {#if store.theme === 'light'}
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M12 7c-2.76 0-5 2.24-5 5s2.24 5 5 5 5-2.24 5-5-2.24-5-5-5zM2 13h2c.55 0 1-.45 1-1s-.45-1-1-1H2c-.55 0-1 .45-1 1s.45 1 1 1zm18 0h2c.55 0 1-.45 1-1s-.45-1-1-1h-2c-.55 0-1 .45-1 1s.45 1 1 1zM11 2v2c0 .55.45 1 1 1s1-.45 1-1V2c0-.55-.45-1-1-1s-1 .45-1 1zm0 18v2c0 .55.45 1 1 1s1-.45 1-1v-2c0-.55-.45-1-1-1s-1 .45-1 1z"/></svg>
                {:else}
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M12 3c-4.97 0-9 4.03-9 9s4.03 9 9 9 9-4.03 9-9c0-.46-.04-.92-.1-1.36-.98 1.37-2.58 2.26-4.4 2.26-2.98 0-5.4-2.42-5.4-5.4 0-1.81.89-3.42 2.26-4.4-.44-.06-.9-.1-1.36-.1z"/></svg>
                {/if}
              {/snippet}
              {#snippet trailing()}
                <TogglePill active={store.theme === 'dark'} label={store.theme === 'light' ? 'Светлая' : 'Тёмная'} />
              {/snippet}
            </ListRow>

            <ListRow title="Прозрачность" onclick={() => { hapticLight(); store.toggleTransparency(); }}>
              {#snippet leading()}
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M17.66 8L12 2.35 6.34 8A8.02 8.02 0 004 13.64c0 2 .78 4.11 2.34 5.67a7.99 7.99 0 0011.32 0c1.56-1.56 2.34-3.67 2.34-5.67C20 11.22 19.22 9.56 17.66 8zM6 14c.01-2 .62-3.27 1.76-4.4L12 5.27l4.24 4.38C17.38 10.77 17.99 12 18 14H6z"/></svg>
              {/snippet}
              {#snippet trailing()}
                <TogglePill active={store.transparencyEnabled} label={store.transparencyEnabled ? 'Вкл' : 'Выкл'} />
              {/snippet}
            </ListRow>

            {#if store.transparencyEnabled}
              <div transition:slide={{ duration: 200, easing: cubicOut }}>
                <ListRow title="Размытие" onclick={() => { hapticLight(); store.toggleBlur(); }}>
                  {#snippet leading()}
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M6 14c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1zm0-4c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1zm0 8c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1zm12-8c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1zm0 4c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1zm0-8c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1zm-6 8c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0-4c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0-4c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z"/></svg>
                  {/snippet}
                  {#snippet trailing()}
                    <TogglePill active={store.blurEnabled} label={store.blurEnabled ? 'Вкл' : 'Выкл'} />
                  {/snippet}
                </ListRow>
              </div>
            {/if}
          </div>

          <div class="setting-group">
            <div class="group-title">Акцентный цвет</div>
            <div class="color-picker">
              {#each [
                { key: 'blue', color: '#0061A4' },
                { key: 'yellow', color: '#E5A800' },
                { key: 'green', color: '#0D904F' },
                { key: 'purple', color: '#7C4DFF' },
                { key: 'red', color: '#C62828' },
                { key: 'teal', color: '#00897B' },
                { key: 'pink', color: '#C2185B' },
              ] as { key, color }}
                <button
                  class="color-swatch tap-btn"
                  class:selected={store.accentColor === key}
                  style="--swatch-color: {color}"
                  onclick={() => { hapticLight(); store.setAccentColor(key); }}
                >
                  {#if store.accentColor === key}
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="white"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/></svg>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        </div>
      {/if}

      {#if view === 'data'}
        <div class="view" data-view="data" in:fly={{ x: 60, duration: 200, easing: cubicOut }} out:fly={{ x: 60, duration: 150 }}>
          <div class="sub-header">
            <IconButton variant="surface" onclick={goBack} aria-label="Назад">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>
            </IconButton>
            <div class="sub-title">Данные</div>
          </div>

          <div class="setting-group">
            <div class="group-title">Экспорт</div>
            <ListRow title="Экспорт CSV" description="Текущий бюджет" onclick={handleExportCsv}>
              {#snippet leading()}
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"/></svg>
              {/snippet}
            </ListRow>

            <ListRow title="Экспорт JSON" description="Все данные" onclick={handleExportJson}>
              {#snippet leading()}
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 12v7H5v-7H3v7c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2v-7h-2zm-6 .67l2.59-2.58L17 11.5l-5 5-5-5 1.41-1.41L11 12.67V3h2v9.67z"/></svg>
              {/snippet}
            </ListRow>
          </div>

          <div class="setting-group">
            <div class="group-title">Импорт</div>
            <ListRow title="Импорт JSON" description={importing ? 'Импорт…' : 'Загрузить из файла'} onclick={handleImportJson} disabled={importing}>
              {#snippet leading()}
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M9 16h6v-6h4l-7-7-7 7h4v6zm-4 2h14v2H5v-2z"/></svg>
              {/snippet}
            </ListRow>
          </div>

          <div class="setting-group">
            <div class="group-title">Отладка</div>
            <ListRow title="Экспорт логов" description={`${store.logs.length} записей`} onclick={handleExportLogs}>
              {#snippet leading()}
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H5.17L4 17.17V4h16v12zM6 7h12v2H6V7zm0 4h9v2H6v-2z"/></svg>
              {/snippet}
            </ListRow>
          </div>
        </div>
      {/if}

      {#if view === 'categories'}
        <div class="view" data-view="categories" in:fly={{ x: 60, duration: 200, easing: cubicOut }} out:fly={{ x: 60, duration: 150 }}>
          <div class="sub-header">
            <IconButton variant="surface" onclick={goBack} aria-label="Назад">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>
            </IconButton>
            <div class="sub-title">{editing ? (editing === 'new' ? 'Новая категория' : 'Редактировать') : 'Категории'}</div>
          </div>

          {#if editing}
            <div class="cat-edit-form">
              <div class="form-group">
                <label class="form-label" for="cat-name">Название</label>
                <Input id="cat-name" bind:value={editName} placeholder="Например: Здоровье" onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); saveCat(); } }} />
              </div>

              <div class="form-group">
                <span class="form-label">Иконка</span>
                <IconPicker icons={CATEGORY_ICON_KEYS} value={editIcon} renderIcon={catIconSnippet} onchange={(ic) => { hapticLight(); editIcon = ic; }} />
              </div>

              <div class="form-group">
                <span class="form-label">Цвет</span>
                <ColorPicker bind:value={editColor} colors={COLORS} />
              </div>

              <div class="cat-actions-row">
                <Button variant="text" onclick={() => editing = null}>Отмена</Button>
                <Button variant="filled" onclick={saveCat}>{editing === 'new' ? 'Создать' : 'Сохранить'}</Button>
              </div>
            </div>
          {:else}
            <div class="cat-list">
              {#each categories as cat}
                <div class="cat-item">
                  <div class="cat-icon" style="background: color-mix(in srgb, {cat.color} 15%, transparent)">
                    <span style="color: {cat.color}">{@html categoryIcon(cat.icon)}</span>
                  </div>
                  <div class="cat-info">
                    <div class="cat-name">{cat.name}</div>
                  </div>
                  <div class="cat-btns">
                    <button class="cat-edit-btn tap-btn" onclick={() => { hapticLight(); startCatEdit(cat.key); }} aria-label="Редактировать">
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
                    </button>
                    {#if cat.key !== 'other'}
                      <button class="cat-del-btn tap-btn" onclick={() => { hapticLight(); confirmDeleteCat(cat.key); }} aria-label="Удалить">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
                      </button>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>

            <button class="new-cat-btn tap-btn" onclick={startCatAdd}>
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
              Новая категория
            </button>
          {/if}
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

  /* Main view header */
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

  /* Navigation rows (main screen) */
  .nav-list {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  /* Setting groups in sub-views */
  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    margin-bottom: 1.25rem;
  }

  .group-title {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--rg-on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding-left: 0.25rem;
  }

  /* Accent color picker */
  .color-picker {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--rg-surface);
    border-radius: 0.75rem;
    justify-content: center;
  }

  .color-swatch {
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 0.625rem;
    background: var(--swatch-color);
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.2s var(--rg-spring-easing), box-shadow 0.2s ease;
  }

  .color-swatch.selected {
    box-shadow: 0 0 0 2px var(--rg-sheet-glass-bg), 0 0 0 4px var(--swatch-color);
    transform: scale(1.05);
  }

  .color-swatch:active {
    transform: scale(0.9);
  }

  /* Categories sub-view */
  .cat-list {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    max-height: 50vh;
    overflow-y: auto;
  }

  .cat-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.625rem 0.5rem;
    border-radius: 0.75rem;
  }

  .cat-icon {
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 0.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .cat-icon span {
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 0;
  }

  .cat-icon :global(svg) {
    width: 1.25rem;
    height: 1.25rem;
  }

  .cat-info { flex: 1; min-width: 0; }

  .cat-name {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--rg-on-surface);
  }

  .cat-btns {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .cat-edit-btn, .cat-del-btn {
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

  .cat-edit-btn:hover { background: var(--rg-surface); }
  .cat-del-btn:hover { background: var(--rg-danger-glass); }
  .cat-del-btn:hover :global(svg) { fill: var(--rg-danger); }

  .new-cat-btn {
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

  .new-cat-btn:hover {
    background: color-mix(in srgb, var(--rg-primary) 14%, transparent);
  }

  /* Category edit form */
  .cat-edit-form {
    display: flex;
    flex-direction: column;
  }

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

  .cat-actions-row {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1rem;
  }

  /* About section */
  .about-section {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 1.25rem 0.5rem 0.25rem;
    color: var(--rg-on-surface-variant);
    opacity: 0.6;
  }

  .about-name {
    font-size: 0.8125rem;
    font-weight: 600;
  }

  .about-dot {
    font-size: 0.75rem;
  }

  .about-version {
    font-size: 0.75rem;
  }
</style>
