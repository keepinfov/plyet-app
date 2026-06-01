<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { store } from '$lib/stores/budget.svelte';

  function copyAll() { store.copyLogs(); }
  function clearAll() { store.clearLogs(); }

  function onOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) store.showLogs = false;
  }

  const levelLabels: Record<string, string> = {
    ok: '✅', err: '❌', warn: '⚠️', info: 'ℹ️',
  };

  const levelColors: Record<string, string> = {
    ok: 'var(--rg-price-pos)', err: 'var(--rg-price-neg)', warn: '#F9AB00', info: 'var(--rg-on-surface-variant)',
  };
</script>

{#if store.showLogs}
  <div class="overlay" transition:fade={{ duration: 180 }} onclick={onOverlayClick} role="dialog" aria-modal="true" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && (store.showLogs = false)}>
    <div class="panel" transition:fly={{ y: 300, duration: 250, easing: cubicOut }}>
      <div class="header">
        <span class="title">🐛 Логи ({store.logs.length})</span>
        <div class="actions">
          <button class="act-btn" onclick={copyAll} aria-label="Копировать все">📋</button>
          <button class="act-btn" onclick={clearAll} aria-label="Очистить">🗑️</button>
          <button class="act-btn" onclick={() => store.showLogs = false} aria-label="Закрыть">✕</button>
        </div>
      </div>
      <div class="list">
        {#if store.logs.length === 0}
          <div class="empty">Пока нет логов</div>
        {:else}
          {#each store.logs as log}
            <div class="row" style="color:{levelColors[log.level] ?? 'var(--rg-on-surface-variant)'}">
              <span class="level">{levelLabels[log.level] ?? '·'}</span>
              <span class="time">{log.ts.slice(11, 19)}</span>
              <span class="msg">{log.msg}</span>
            </div>
          {/each}
        {/if}
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
    max-height: 70vh;
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
    padding: 0.75rem 1.25rem;
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 0.8125rem;
    line-height: 1.6;
  }

  .empty {
    text-align: center;
    color: var(--rg-on-surface-variant);
    padding: 2rem;
  }

  .row {
    display: flex;
    gap: 0.5rem;
    padding: 0.25rem 0;
    word-break: break-all;
  }

  .level { flex-shrink: 0; }

  .time {
    flex-shrink: 0;
    color: var(--rg-on-surface-variant);
    font-size: 0.6875rem;
    padding-top: 0.125rem;
  }

  .msg { min-width: 0; }
</style>
