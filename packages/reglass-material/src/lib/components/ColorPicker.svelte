<script lang="ts">
  interface Props {
    /** Selected color as a #rrggbb hex string. */
    value: string;
    /** Preset palette swatches. */
    colors?: string[];
    onchange?: (hex: string) => void;
  }

  let { value = $bindable('#00ACC1'), colors = [], onchange }: Props = $props();

  const isCustomColor = $derived(!colors.includes(value));

  let showCustom = $state(false);
  let hue = $state(180);
  let sat = $state(80);
  let light = $state(50);

  let hueCanvas = $state<HTMLCanvasElement | null>(null);
  let slCanvas = $state<HTMLCanvasElement | null>(null);
  let draggingHue = $state(false);
  let draggingSL = $state(false);

  function set(hex: string) {
    value = hex;
    onchange?.(hex);
  }

  function hslToHex(h: number, s: number, l: number): string {
    const a = (s / 100) * Math.min(l, 100 - l) / 100;
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
    const max = Math.max(r, g, b),
      min = Math.min(r, g, b);
    let h = 0,
      s = 0;
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

  function openCustom() {
    if (isCustomColor) {
      [hue, sat, light] = hexToHsl(value);
    } else {
      hue = 180;
      sat = 80;
      light = 50;
    }
    showCustom = true;
    requestAnimationFrame(() => {
      drawHueStrip();
      drawSLPad();
    });
  }

  function drawHueStrip() {
    if (!hueCanvas) return;
    const ctx = hueCanvas.getContext('2d')!;
    const w = hueCanvas.width,
      h = hueCanvas.height;
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
    const w = slCanvas.width,
      h = slCanvas.height;
    const img = ctx.createImageData(w, h);
    const data = img.data;
    const hVal = hue;
    for (let y = 0; y < h; y++) {
      const l = 1 - y / h;
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
    set(hslToHex(hue, sat, light));
    drawSLPad();
  }

  function handleSLInput(clientX: number, clientY: number) {
    if (!slCanvas) return;
    const rect = slCanvas.getBoundingClientRect();
    const x = Math.max(0, Math.min(clientX - rect.left, rect.width));
    const y = Math.max(0, Math.min(clientY - rect.top, rect.height));
    sat = Math.round((x / rect.width) * 100);
    light = Math.round(100 - (y / rect.height) * 100);
    set(hslToHex(hue, sat, light));
  }
</script>

<div class="rg-color-grid">
  {#each colors as c}
    <button
      class="rg-color-opt tap-btn"
      class:selected={value === c}
      style="background: {c}"
      onclick={() => { set(c); showCustom = false; }}
      aria-label={c}
    >
      {#if value === c}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="white"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/></svg>
      {/if}
    </button>
  {/each}
  <button
    class="rg-color-opt rg-color-custom tap-btn"
    class:selected={isCustomColor}
    style="background: {isCustomColor ? value : 'var(--rg-surface)'}"
    onclick={openCustom}
    aria-label="Custom color"
  >
    {#if isCustomColor}
      <svg width="14" height="14" viewBox="0 0 24 24" fill="white"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/></svg>
    {:else}
      <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--rg-on-surface-variant)"><path d="M12 2C6.49 2 2 6.49 2 12s4.49 10 10 10a2.5 2.5 0 002.5-2.5c0-.61-.23-1.21-.64-1.67a.528.528 0 01.13-.83c.09-.04.19-.06.28-.06H16c3.31 0 6-2.69 6-6 0-4.96-4.49-9-10-9zm-5.5 9a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm3-4a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm5 0a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm3 4a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/></svg>
    {/if}
  </button>
</div>

{#if showCustom}
  <div class="rg-custom-picker">
    <div class="rg-picker-preview" style="background: {value}"></div>
    <div class="rg-picker-controls">
      <div class="rg-picker-sl-wrap">
        <canvas
          bind:this={slCanvas}
          class="rg-picker-sl"
          width="200"
          height="150"
          ontouchstart={(e) => { e.preventDefault(); draggingSL = true; handleSLInput(e.touches[0].clientX, e.touches[0].clientY); }}
          ontouchmove={(e) => { if (draggingSL) { e.preventDefault(); handleSLInput(e.touches[0].clientX, e.touches[0].clientY); } }}
          ontouchend={() => (draggingSL = false)}
          onmousedown={(e) => { draggingSL = true; handleSLInput(e.clientX, e.clientY); }}
        ></canvas>
        <div class="rg-picker-sl-cursor" style="left: {sat}%; top: {100 - light}%;"></div>
      </div>
      <div class="rg-picker-hue-wrap">
        <canvas
          bind:this={hueCanvas}
          class="rg-picker-hue"
          width="200"
          height="20"
          ontouchstart={(e) => { e.preventDefault(); draggingHue = true; handleHueInput(e.touches[0].clientX); }}
          ontouchmove={(e) => { if (draggingHue) { e.preventDefault(); handleHueInput(e.touches[0].clientX); } }}
          ontouchend={() => (draggingHue = false)}
          onmousedown={(e) => { draggingHue = true; handleHueInput(e.clientX); }}
        ></canvas>
        <div class="rg-picker-hue-cursor" style="left: {(hue / 360) * 100}%;"></div>
      </div>
      <div class="rg-picker-hex">
        <span class="rg-picker-hex-label">HEX</span>
        <input
          class="rg-picker-hex-input"
          value={value}
          oninput={(e) => {
            const v = (e.target as HTMLInputElement).value;
            if (/^#[0-9a-fA-F]{6}$/.test(v)) {
              set(v);
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

<style>
  .rg-color-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, 2rem);
    gap: 0.5rem;
    justify-content: center;
    padding: 0.25rem;
  }

  .rg-color-opt {
    width: 2rem;
    height: 2rem;
    border-radius: 0.5rem;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: box-shadow 0.15s ease, transform 0.15s ease;
  }

  .rg-color-opt.selected {
    box-shadow: 0 0 0 2px var(--rg-sheet-bg), 0 0 0 3px var(--rg-on-surface);
  }

  .rg-color-custom:not(.selected) {
    border: none;
    background: var(--rg-surface);
  }

  .rg-custom-picker {
    margin-top: 0.75rem;
    background: var(--rg-surface);
    border-radius: 0.75rem;
    padding: 0.75rem;
    display: flex;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .rg-picker-preview {
    width: 3rem;
    height: 3rem;
    border-radius: 0.75rem;
    flex-shrink: 0;
  }

  .rg-picker-controls {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
    min-width: 0;
  }

  .rg-picker-sl-wrap {
    position: relative;
    width: 100%;
    aspect-ratio: 4 / 3;
    border-radius: 0.5rem;
    overflow: hidden;
    cursor: crosshair;
    touch-action: none;
  }

  .rg-picker-sl {
    width: 100%;
    height: 100%;
    display: block;
  }

  .rg-picker-sl-cursor {
    position: absolute;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid white;
    box-shadow: 0 0 3px rgba(0, 0, 0, 0.4);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .rg-picker-hue-wrap {
    position: relative;
    width: 100%;
    height: 1.25rem;
    border-radius: 0.625rem;
    overflow: hidden;
    cursor: pointer;
    touch-action: none;
  }

  .rg-picker-hue {
    width: 100%;
    height: 100%;
    display: block;
  }

  .rg-picker-hue-cursor {
    position: absolute;
    top: 50%;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid white;
    box-shadow: 0 0 3px rgba(0, 0, 0, 0.4);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .rg-picker-hex {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .rg-picker-hex-label {
    font-size: 0.7rem;
    font-weight: 700;
    color: var(--rg-primary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .rg-picker-hex-input {
    width: 6rem;
    padding: 0.375rem 0.625rem;
    font-size: 0.8125rem;
    font-family: monospace;
    text-transform: uppercase;
    background: var(--rg-surface);
    color: var(--rg-on-surface);
    border: none;
    border-radius: 0.5rem;
    outline: none;
  }

  .rg-picker-hex-input:focus {
    outline: 2px solid color-mix(in srgb, var(--rg-primary) 40%, transparent);
    outline-offset: -1px;
  }
</style>
