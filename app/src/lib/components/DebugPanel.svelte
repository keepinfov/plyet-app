<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { store } from '$lib/stores/budget.svelte';
  import { keyboard } from 'reglass-material';

  // Tick to refresh non-reactive readings (visualViewport, computed styles).
  // Driven by visualViewport events + a slow interval while the panel is open.
  let tick = $state(0);

  $effect(() => {
    if (!store.showDebug) return;
    const bump = () => (tick += 1);
    const vv = typeof window !== 'undefined' ? window.visualViewport : null;
    vv?.addEventListener('resize', bump);
    vv?.addEventListener('scroll', bump);
    const id = setInterval(bump, 500);
    return () => {
      vv?.removeEventListener('resize', bump);
      vv?.removeEventListener('scroll', bump);
      clearInterval(id);
    };
  });

  function chromiumVersion(ua: string): string {
    const m = ua.match(/Chrome\/(\d+)\.[\d.]+/);
    if (!m) return 'не найдено (не Chromium?)';
    const major = Number(m[1]);
    const iw = major >= 108 ? 'есть interactive-widget' : 'НЕТ interactive-widget';
    return `${m[1]} — ${iw}`;
  }

  function cssVar(name: string): string {
    if (typeof document === 'undefined') return '—';
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '(пусто)';
  }

  function safeArea(side: string): string {
    if (typeof document === 'undefined') return '—';
    const probe = document.createElement('div');
    probe.style.position = 'fixed';
    probe.style.padding = `env(safe-area-inset-${side})`;
    document.body.appendChild(probe);
    const v = getComputedStyle(probe).paddingTop;
    probe.remove();
    return v;
  }

  interface Row {
    label: string;
    value: string;
  }

  const rows = $derived.by<Row[]>(() => {
    void tick; // re-read on every tick
    const w = typeof window !== 'undefined' ? window : undefined;
    const vv = w?.visualViewport ?? null;
    const layoutShrink = Math.max(0, keyboard.baselineInnerHeight - keyboard.innerHeight);
    const nativeCoverage = Math.max(0, keyboard.nativeImeHeight - layoutShrink);
    return [
      { label: 'User-Agent', value: w?.navigator.userAgent ?? '—' },
      { label: 'Chromium', value: w ? chromiumVersion(w.navigator.userAgent) : '—' },
      { label: 'Platform', value: w?.navigator.platform ?? '—' },
      { label: 'devicePixelRatio', value: String(w?.devicePixelRatio ?? '—') },
      { label: 'screen', value: w ? `${w.screen.width}×${w.screen.height}` : '—' },
      { label: 'innerHeight', value: String(w?.innerHeight ?? '—') },
      { label: 'baselineInnerHeight', value: String(keyboard.baselineInnerHeight) },
      { label: 'layoutShrink', value: String(Math.round(layoutShrink)) },
      { label: 'vv.height', value: vv ? String(Math.round(vv.height)) : 'нет visualViewport' },
      { label: 'vv.offsetTop', value: vv ? String(Math.round(vv.offsetTop)) : '—' },
      { label: 'vv.scale', value: vv ? String(vv.scale) : '—' },
      { label: 'webHeight', value: String(Math.round(keyboard.webHeight)) },
      { label: 'nativeImeHeight', value: String(Math.round(keyboard.nativeImeHeight)) },
      { label: 'nativeCoverage', value: String(Math.round(nativeCoverage)) },
      { label: 'effective height', value: String(Math.round(keyboard.height)) },
      { label: 'open', value: String(keyboard.open) },
      { label: '--rg-keyboard-height', value: cssVar('--rg-keyboard-height') },
      {
        label: 'data-keyboard-open',
        value:
          typeof document !== 'undefined' &&
          document.documentElement.hasAttribute('data-keyboard-open')
            ? 'да'
            : 'нет',
      },
      { label: 'safe-area top', value: safeArea('top') },
      { label: 'safe-area bottom', value: safeArea('bottom') },
    ];
  });

  function copyAll() {
    const text = rows.map((r) => `${r.label}: ${r.value}`).join('\n');
    navigator.clipboard.writeText(text).catch(() => {});
    store.showSnackbar('Скопировано ✓');
  }

  function onOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) store.showDebug = false;
  }
</script>

{#if store.showDebug}
  <div class="overlay" transition:fade={{ duration: 180 }} onclick={onOverlayClick} role="dialog" aria-modal="true" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && (store.showDebug = false)}>
    <div class="panel" transition:fly={{ y: 300, duration: 250, easing: cubicOut }}>
      <div class="header">
        <span class="title">🩺 Диагностика</span>
        <div class="actions">
          <button class="act-btn" onclick={copyAll} aria-label="Копировать">📋</button>
          <button class="act-btn" onclick={() => store.showDebug = false} aria-label="Закрыть">✕</button>
        </div>
      </div>
      <div class="list">
        {#each rows as row}
          <div class="row">
            <span class="key">{row.label}</span>
            <span class="val">{row.value}</span>
          </div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--rg-overlay-bg);
    z-index: 200;
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }

  .panel {
    width: 100%;
    max-width: 31.25rem;
    max-height: 80vh;
    background: var(--rg-sheet-glass-bg);
    backdrop-filter: var(--rg-glass-blur);
    -webkit-backdrop-filter: var(--rg-glass-blur);
    border-radius: 1rem 1rem 0 0;
    padding-bottom: env(safe-area-inset-bottom, 0px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    contain: layout style paint;
    will-change: transform;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    background: var(--rg-surface-2);
    flex-shrink: 0;
  }

  .title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--rg-on-surface);
  }

  .actions { display: flex; gap: 0.5rem; }

  .act-btn {
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 50%;
    border: none;
    background: var(--rg-surface-2);
    font-size: 1rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s ease;
  }

  .act-btn:active { background: var(--rg-outline); }

  .list {
    flex: 1;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0.75rem 1.25rem;
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 0.8125rem;
    line-height: 1.5;
  }

  .row {
    display: flex;
    gap: 0.75rem;
    padding: 0.375rem 0;
    border-bottom: 1px solid color-mix(in srgb, var(--rg-outline) 40%, transparent);
    word-break: break-all;
  }

  .key {
    flex-shrink: 0;
    width: 11rem;
    color: var(--rg-on-surface-variant);
  }

  .val {
    min-width: 0;
    color: var(--rg-on-surface);
    font-weight: 600;
  }
</style>
