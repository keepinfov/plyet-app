<script lang="ts">
  import '$lib/tokens.css';
  import { theme } from '$lib/theme';

  theme.configure({ storagePrefix: 'rg' });

  let { children } = $props();

  const accents = ['blue', 'yellow', 'green', 'purple', 'red', 'teal', 'pink'];
</script>

<div class="pg-shell">
  <header class="pg-controls">
    <strong>re:glass material</strong>
    <button onclick={() => theme.toggleMode()}>mode: {theme.mode}</button>
    <button onclick={() => theme.toggleBlur()}>blur: {theme.blurEnabled ? 'on' : 'off'}</button>
    <button onclick={() => theme.toggleTransparency()}>
      transparency: {theme.transparencyEnabled ? 'on' : 'off'}
    </button>
    <label>
      accent:
      <select value={theme.accent} onchange={(e) => theme.setAccent(e.currentTarget.value)}>
        {#each accents as a}
          <option value={a}>{a}</option>
        {/each}
      </select>
    </label>
  </header>
  <main class="pg-main">
    {@render children()}
  </main>
</div>

<style>
  .pg-shell {
    min-height: 100dvh;
    background: var(--rg-surface);
    color: var(--rg-on-surface);
    font-family: system-ui, sans-serif;
  }
  .pg-controls {
    position: sticky;
    top: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: center;
    padding: 0.75rem 1rem;
    background: var(--rg-topbar-bg);
    backdrop-filter: var(--rg-glass-blur-light);
    -webkit-backdrop-filter: var(--rg-glass-blur-light);
    box-shadow: var(--rg-topbar-shadow);
    z-index: 10;
  }
  .pg-controls button,
  .pg-controls select {
    font: inherit;
    padding: 0.35rem 0.6rem;
    border-radius: 0.5rem;
    border: 1px solid var(--rg-outline);
    background: var(--rg-surface-2);
    color: var(--rg-on-surface);
    cursor: pointer;
  }
  .pg-main {
    padding: 1.5rem 1rem 4rem;
    max-width: 720px;
    margin: 0 auto;
  }
</style>
