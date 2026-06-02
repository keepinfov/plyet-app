<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { store } from '$lib/stores/budget.svelte';
  import { categoryIcon, CATEGORY_ICON_KEYS } from '$lib/icons';
  import { hapticLight } from '$lib/haptics';
  import { keyboardAvoid } from 'reglass-material';

  interface Props {
    show?: boolean;
    onclose?: () => void;
  }

  let { show = $bindable(false), onclose }: Props = $props();

  $effect(() => {
    if (show) hapticLight();
  });

  const categories = $derived(store.data?.categories ?? []);

  let editing = $state<string | null>(null); // category key being edited, or 'new' for adding
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

  const isCustomColor = $derived(!COLORS.includes(editColor));
  let showColorPicker = $state(false);
  let hue = $state(180);
  let sat = $state(80);
  let light = $state(50);

  // Canvas-based hue strip
  let hueCanvas = $state<HTMLCanvasElement>(undefined!);
  let slCanvas = $state<HTMLCanvasElement>(undefined!);
  let draggingHue = $state(false);
  let draggingSL = $state(false);

  function hslToHex(h: number, s: number, l: number): string {
    const a = s / 100 * Math.min(l, 100 - l) / 100;
    const f = (n: number) => {
      const k = (n + h / 30) % 12;
      const color = l / 100 - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
      return Math.round(255 * color).toString(16).padStart(2, '0');
    };
    return `#${f(0)}${f(8)}${f(4)}`;
  }

  function hexToHsl(hex: string): [number, number, number] {
    const r = parseInt(hex.slice(1, 3), 16) / 255;
    const g = parseInt(hex.slice(3, 5), 16) / 255;
    const b = parseInt(hex.slice(5, 7), 16) / 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h = 0, s = 0;
    const l = (max + min) / 2;
    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) * 60;
      else if (max === g) h = ((b - r) / d + 2) * 60;
      else h = ((r - g) / d + 4) * 60;
    }
    return [Math.round(h), Math.round(s * 100), Math.round(l * 100)];
  }

  function openCustomPicker() {
    if (isCustomColor) {
      [hue, sat, light] = hexToHsl(editColor);
    } else {
      hue = 180; sat = 80; light = 50;
    }
    showColorPicker = true;
    requestAnimationFrame(() => {
      drawHueStrip();
      drawSLPad();
    });
  }

  function drawHueStrip() {
    if (!hueCanvas) return;
    const ctx = hueCanvas.getContext('2d')!;
    const w = hueCanvas.width, h = hueCanvas.height;
    const grad = ctx.createLinearGradient(0, 0, w, 0);
    for (let i = 0; i <= 360; i += 30) {
      grad.addColorStop(i / 360, `hsl(${i}, 100%, 50%)`);
    }
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, w, h);
  }

  function drawSLPad() {
    if (!slCanvas) return;
    const ctx = slCanvas.getContext('2d')!;
    const w = slCanvas.width, h = slCanvas.height;
    const img = ctx.createImageData(w, h);
    const data = img.data;
    const hVal = hue;
    for (let y = 0; y < h; y++) {
      const l = (1 - y / h);
      const row = y * w * 4;
      for (let x = 0; x < w; x++) {
        const s = x / w;
        const a = s * Math.min(l, 1 - l);
        const off = row + x * 4;
        for (let i = 0; i < 3; i++) {
          const k = ([0, 8, 4][i] + hVal / 30) % 12;
          data[off + i] = Math.round(255 * (l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1)));
        }
        data[off + 3] = 255;
      }
    }
    ctx.putImageData(img, 0, 0);
  }

  function handleHueInput(clientX: number) {
    if (!hueCanvas) return;
    const rect = hueCanvas.getBoundingClientRect();
    const x = Math.max(0, Math.min(clientX - rect.left, rect.width));
    hue = Math.round((x / rect.width) * 360);
    editColor = hslToHex(hue, sat, light);
    drawSLPad();
  }

  function handleSLInput(clientX: number, clientY: number) {
    if (!slCanvas) return;
    const rect = slCanvas.getBoundingClientRect();
    const x = Math.max(0, Math.min(clientX - rect.left, rect.width));
    const y = Math.max(0, Math.min(clientY - rect.top, rect.height));
    sat = Math.round((x / rect.width) * 100);
    light = Math.round(100 - (y / rect.height) * 100);
    editColor = hslToHex(hue, sat, light);
  }

  function close() { onclose?.(); show = false; editing = null; }

  function startAdd() {
    editing = 'new';
    editName = '';
    editIcon = 'other';
    editColor = '#00ACC1';
    editKey = '';
  }

  function startEdit(key: string) {
    const cat = categories.find(c => c.key === key);
    if (!cat) return;
    editing = key;
    editName = cat.name;
    editIcon = cat.icon;
    editColor = cat.color;
    editKey = key;
  }

  async function save() {
    if (!editName.trim()) return;
    hapticLight();
    if (editing === 'new') {
      const key = editName.trim().toLowerCase().replace(/[^a-zа-яё0-9]/gi, '_').substring(0, 32) || 'custom';
      // Ensure unique key
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

  function confirmDelete(key: string) {
    store.showConfirm('Удалить эту категорию? Записи будут перенесены в "Другое".', () => {
      store.deleteCategory(key);
    });
  }

  function onOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      if (editing) editing = null;
      else close();
    }
  }
</script>

<svelte:window
  onkeydown={(e) => { if (e.key === 'Escape' && show) { if (editing) editing = null; else close(); } }}
  onmousemove={(e) => {
    if (draggingHue) handleHueInput(e.clientX);
    if (draggingSL) handleSLInput(e.clientX, e.clientY);
  }}
  onmouseup={() => { draggingHue = false; draggingSL = false; }}
/>

{#if show}
  <div class="overlay" transition:fade={{ duration: 180 }} onclick={onOverlayClick} onkeydown={(e) => { if (e.key === 'Escape') close(); }} use:keyboardAvoid role="dialog" aria-modal="true" tabindex="-1">
    <div class="sheet" transition:fly={{ y: -60, duration: 250, easing: cubicOut }}>

      {#if editing}
        <div class="modal-title">{editing === 'new' ? 'Новая категория' : 'Редактировать'}</div>

        <div class="form-group">
          <label class="form-label" for="cat-name">Название</label>
          <input id="cat-name" class="form-input" bind:value={editName} placeholder="Например: Здоровье" />
        </div>

        <div class="form-group">
          <span class="form-label">Иконка</span>
          <div class="icon-picker">
            {#each CATEGORY_ICON_KEYS as ic}
              <button class="icon-opt tap-btn" class:selected={editIcon === ic} onclick={() => { hapticLight(); editIcon = ic; }}>
                {@html categoryIcon(ic)}
              </button>
            {/each}
          </div>
        </div>

        <div class="form-group">
          <span class="form-label">Цвет</span>
          <div class="color-picker">
            {#each COLORS as c}
              <button
                class="color-opt tap-btn"
                class:selected={editColor === c}
                style="background: {c}"
                onclick={() => { hapticLight(); editColor = c; showColorPicker = false; }}
                aria-label={c}
              >
                {#if editColor === c}
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="white"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/></svg>
                {/if}
              </button>
            {/each}
            <button
              class="color-opt color-custom tap-btn"
              class:selected={isCustomColor}
              style="background: {isCustomColor ? editColor : 'var(--rg-surface)'}"
              onclick={() => openCustomPicker()}
              aria-label="Свой цвет"
            >
              {#if isCustomColor}
                <svg width="14" height="14" viewBox="0 0 24 24" fill="white"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/></svg>
              {:else}
                <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M12 2C6.49 2 2 6.49 2 12s4.49 10 10 10a2.5 2.5 0 002.5-2.5c0-.61-.23-1.21-.64-1.67a.528.528 0 01.13-.83c.09-.04.19-.06.28-.06H16c3.31 0 6-2.69 6-6 0-4.96-4.49-9-10-9zm-5.5 9a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm3-4a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm5 0a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm3 4a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/></svg>
              {/if}
            </button>
          </div>

          {#if showColorPicker}
            <div class="custom-picker">
              <div class="picker-preview" style="background: {editColor}"></div>
              <div class="picker-controls">
                <div class="picker-sl-wrap">
                  <canvas
                    bind:this={slCanvas}
                    class="picker-sl"
                    width="200"
                    height="150"
                    ontouchstart={(e) => { e.preventDefault(); draggingSL = true; handleSLInput(e.touches[0].clientX, e.touches[0].clientY); }}
                    ontouchmove={(e) => { if (draggingSL) { e.preventDefault(); handleSLInput(e.touches[0].clientX, e.touches[0].clientY); } }}
                    ontouchend={() => draggingSL = false}
                    onmousedown={(e) => { draggingSL = true; handleSLInput(e.clientX, e.clientY); }}
                  ></canvas>
                  <div class="picker-sl-cursor" style="left: {sat}%; top: {100 - light}%;"></div>
                </div>
                <div class="picker-hue-wrap">
                  <canvas
                    bind:this={hueCanvas}
                    class="picker-hue"
                    width="200"
                    height="20"
                    ontouchstart={(e) => { e.preventDefault(); draggingHue = true; handleHueInput(e.touches[0].clientX); }}
                    ontouchmove={(e) => { if (draggingHue) { e.preventDefault(); handleHueInput(e.touches[0].clientX); } }}
                    ontouchend={() => draggingHue = false}
                    onmousedown={(e) => { draggingHue = true; handleHueInput(e.clientX); }}
                  ></canvas>
                  <div class="picker-hue-cursor" style="left: {(hue / 360) * 100}%;"></div>
                </div>
                <div class="picker-hex">
                  <span class="form-label" style="margin-bottom: 0">HEX</span>
                  <input
                    class="form-input picker-hex-input"
                    value={editColor}
                    oninput={(e) => {
                      const v = (e.target as HTMLInputElement).value;
                      if (/^#[0-9a-fA-F]{6}$/.test(v)) {
                        editColor = v;
                        [hue, sat, light] = hexToHsl(v);
                        drawSLPad();
                      }
                    }}
                    maxlength="7"
                  />
                </div>
              </div>
            </div>
          {/if}
        </div>

        <div class="actions">
          <button class="btn-text tap-btn" onclick={() => editing = null}>Отмена</button>
          <button class="btn-filled tap-btn" onclick={save}>{editing === 'new' ? 'Создать' : 'Сохранить'}</button>
        </div>

      {:else}
        <div class="modal-title">Категории</div>

        {#each categories as cat}
          <div class="cat-item">
            <div class="cat-icon" style="background: color-mix(in srgb, {cat.color} 15%, transparent)">
              <span style="color: {cat.color}">{@html categoryIcon(cat.icon)}</span>
            </div>
            <div class="cat-info">
              <div class="cat-name">{cat.name}</div>
              <div class="cat-key">{cat.key}</div>
            </div>
            <div class="cat-actions">
              <button class="edit-btn tap-btn" onclick={() => { hapticLight(); startEdit(cat.key); }} aria-label="Редактировать">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
              </button>
              {#if cat.key !== 'other'}
                <button class="delete-cat-btn tap-btn" onclick={() => { hapticLight(); confirmDelete(cat.key); }} aria-label="Удалить">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
                </button>
              {/if}
            </div>
          </div>
        {/each}

        <button class="new-btn tap-btn" onclick={startAdd}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
          Новая категория
        </button>

        <div class="actions">
          <button class="btn-text tap-btn" onclick={close}>Закрыть</button>
        </div>
      {/if}
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
    padding: env(safe-area-inset-top, 0px) 0
      calc(env(safe-area-inset-bottom, 0px) + var(--rg-keyboard-height, 0px));
  }

  .sheet {
    width: calc(100% - 2rem);
    max-width: 26rem;
    max-height: 85dvh;
    background: var(--rg-sheet-glass-bg);
    backdrop-filter: var(--rg-glass-blur);
    -webkit-backdrop-filter: var(--rg-glass-blur);
    border-radius: 1.25rem;
    padding: 1.5rem;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-shadow: 0 8px 40px rgba(0,0,0,0.15);
    contain: layout style paint;
    will-change: transform;
  }

  .modal-title {
    font-size: 1.375rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    margin-bottom: 1.375rem;
    text-align: center;
    letter-spacing: -0.02em;
  }

  .cat-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.625rem 0.5rem;
    border-radius: 0.75rem;
    transition: background 0.15s ease;
    margin-bottom: 0.125rem;
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

  .cat-key {
    font-size: 0.6875rem;
    color: var(--rg-on-surface-variant);
    opacity: 0.6;
  }

  .cat-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .edit-btn, .delete-cat-btn {
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    border: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .edit-btn:hover { background: var(--rg-surface); }
  .delete-cat-btn:hover { background: var(--rg-danger-glass); }
  .delete-cat-btn:hover svg { fill: var(--rg-danger); }

  .new-btn {
    width: 100%;
    padding: 0.75rem;
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
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .new-btn:hover {
    background: color-mix(in srgb, var(--rg-primary) 14%, transparent);
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

  .form-input {
    width: 100%;
    padding: 0.75rem 1rem;
    border-radius: 0.75rem;
    border: none;
    background: var(--rg-surface);
    color: var(--rg-on-surface);
    font-size: 1rem;
    font-family: inherit;
    outline: none;
    scroll-margin-bottom: 6rem;
    transition: box-shadow 0.2s ease, background 0.2s ease;
  }

  .form-input:focus {
    outline: 2px solid color-mix(in srgb, var(--rg-primary) 40%, transparent);
    outline-offset: -1px;
  }

  .form-input::placeholder {
    color: var(--rg-on-surface-variant);
    opacity: 0.45;
  }

  .icon-picker {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .icon-opt {
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 0.75rem;
    border: none;
    background: color-mix(in srgb, var(--rg-on-surface) 6%, transparent);
    color: var(--rg-on-surface-variant);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .icon-opt :global(svg) {
    width: 1.25rem;
    height: 1.25rem;
  }

  .icon-opt.selected {
    background: color-mix(in srgb, var(--rg-primary) 15%, transparent);
    color: var(--rg-primary);
  }

  .icon-opt.selected :global(svg) {
    fill: var(--rg-primary);
  }

  .color-picker {
    display: grid;
    grid-template-columns: repeat(auto-fill, 2rem);
    gap: 0.5rem;
    justify-content: center;
    padding: 0.25rem;
  }

  .color-opt {
    width: 2rem;
    height: 2rem;
    border-radius: 0.5rem;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .color-opt.selected {
    box-shadow: 0 0 0 2px var(--rg-sheet-bg), 0 0 0 3px var(--rg-on-surface);
  }

  .color-custom:not(.selected) {
    border: none;
    background: var(--rg-surface);
  }

  .custom-picker {
    margin-top: 0.75rem;
    background: var(--rg-surface);
    border-radius: 0.75rem;
    padding: 0.75rem;
    display: flex;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .picker-preview {
    width: 3rem;
    height: 3rem;
    border-radius: 0.75rem;
    flex-shrink: 0;
  }

  .picker-controls {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
    min-width: 0;
  }

  .picker-sl-wrap {
    position: relative;
    width: 100%;
    aspect-ratio: 4 / 3;
    border-radius: 0.5rem;
    overflow: hidden;
    cursor: crosshair;
    touch-action: none;
  }

  .picker-sl {
    width: 100%;
    height: 100%;
    display: block;
  }

  .picker-sl-cursor {
    position: absolute;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid white;
    box-shadow: 0 0 3px rgba(0,0,0,0.4);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .picker-hue-wrap {
    position: relative;
    width: 100%;
    height: 1.25rem;
    border-radius: 0.625rem;
    overflow: hidden;
    cursor: pointer;
    touch-action: none;
  }

  .picker-hue {
    width: 100%;
    height: 100%;
    display: block;
  }

  .picker-hue-cursor {
    position: absolute;
    top: 50%;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid white;
    box-shadow: 0 0 3px rgba(0,0,0,0.4);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .picker-hex {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .picker-hex-input {
    width: 6rem;
    padding: 0.375rem 0.625rem;
    font-size: 0.8125rem;
    font-family: monospace;
    text-transform: uppercase;
    background: var(--rg-surface);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1.5rem;
  }

  .btn-text {
    padding: 0.75rem 1.5rem;
    border-radius: 999rem;
    border: none;
    background: transparent;
    color: var(--rg-primary);
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
    font-weight: 600;
    transition: background 0.15s ease;
  }

  .btn-text:hover { background: color-mix(in srgb, var(--rg-primary) 8%, transparent); }

  .btn-filled {
    padding: 0.75rem 1.75rem;
    border-radius: 999rem;
    border: none;
    background: var(--rg-primary);
    color: var(--rg-on-primary);
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.15s ease;
    box-shadow: 0 2px 8px var(--rg-primary-shadow);
  }

  .btn-filled:hover {
    box-shadow: 0 4px 14px var(--rg-primary-shadow);
    transform: translateY(-1px);
  }
</style>
