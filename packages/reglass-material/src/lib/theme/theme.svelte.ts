// re:glass material — theme controller.
// A rune-based singleton that owns the four theming attributes applied to
// <html>: data-theme, data-accent, data-blur, data-transparency. State is
// persisted to localStorage under a configurable prefix and is SSR-safe.

export type ThemeMode = 'light' | 'dark';

export interface ThemeConfig {
  /** localStorage key prefix, e.g. 'plyet' → keys 'plyet-theme', 'plyet-accent', … */
  storagePrefix?: string;
  defaultMode?: ThemeMode;
  defaultAccent?: string;
  defaultBlur?: boolean;
  defaultTransparency?: boolean;
}

const hasWindow = () => typeof window !== 'undefined';
const hasDocument = () => typeof document !== 'undefined';

class ThemeController {
  mode = $state<ThemeMode>('light');
  accent = $state<string>('blue');
  blurEnabled = $state<boolean>(true);
  transparencyEnabled = $state<boolean>(true);

  #prefix = 'rg';

  #key(name: string): string {
    return `${this.#prefix}-${name}`;
  }

  #read(name: string): string | null {
    if (!hasWindow()) return null;
    try {
      return localStorage.getItem(this.#key(name));
    } catch {
      return null;
    }
  }

  #write(name: string, value: string): void {
    if (!hasWindow()) return;
    try {
      localStorage.setItem(this.#key(name), value);
    } catch {
      /* storage unavailable — ignore */
    }
  }

  /**
   * Hydrate state from storage (falling back to provided defaults) and apply
   * all attributes to <html>. Call once during app/layout init.
   */
  configure(config: ThemeConfig = {}): void {
    this.#prefix = config.storagePrefix ?? this.#prefix;

    this.mode = (this.#read('theme') as ThemeMode) || config.defaultMode || 'light';
    this.accent = this.#read('accent') || config.defaultAccent || 'blue';

    const blur = this.#read('blur');
    this.blurEnabled = blur !== null ? blur !== 'off' : config.defaultBlur ?? true;

    const transparency = this.#read('transparency');
    this.transparencyEnabled =
      transparency !== null ? transparency !== 'off' : config.defaultTransparency ?? true;

    this.applyAll();
  }

  /** Apply all four attributes to <html> at once. */
  applyAll(): void {
    if (!hasDocument()) return;
    const el = document.documentElement;
    el.setAttribute('data-theme', this.mode);
    el.setAttribute('data-accent', this.accent);
    el.setAttribute('data-blur', this.blurEnabled ? 'on' : 'off');
    el.setAttribute('data-transparency', this.transparencyEnabled ? 'on' : 'off');
  }

  setMode(mode: ThemeMode): void {
    this.mode = mode;
    this.#write('theme', mode);
    if (hasDocument()) document.documentElement.setAttribute('data-theme', mode);
  }

  toggleMode(): void {
    this.setMode(this.mode === 'light' ? 'dark' : 'light');
  }

  setAccent(accent: string): void {
    this.accent = accent;
    this.#write('accent', accent);
    if (hasDocument()) document.documentElement.setAttribute('data-accent', accent);
  }

  setBlur(enabled: boolean): void {
    this.blurEnabled = enabled;
    this.#write('blur', enabled ? 'on' : 'off');
    if (hasDocument()) document.documentElement.setAttribute('data-blur', enabled ? 'on' : 'off');
  }

  toggleBlur(): void {
    this.setBlur(!this.blurEnabled);
  }

  setTransparency(enabled: boolean): void {
    this.transparencyEnabled = enabled;
    this.#write('transparency', enabled ? 'on' : 'off');
    if (hasDocument())
      document.documentElement.setAttribute('data-transparency', enabled ? 'on' : 'off');
  }

  toggleTransparency(): void {
    this.setTransparency(!this.transparencyEnabled);
  }
}

export const theme = new ThemeController();
