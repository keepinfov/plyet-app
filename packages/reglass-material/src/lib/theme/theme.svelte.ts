// re:glass material — theme controller.
// A rune-based singleton that owns the four theming attributes applied to
// <html>: data-theme, data-accent, data-blur, data-transparency. State is
// persisted to localStorage under a configurable prefix and is SSR-safe.

/** The resolved appearance actually applied to <html data-theme>. */
export type ThemeMode = 'light' | 'dark';
/** The user's stored preference: a fixed mode, or follow the OS setting. */
export type ThemePref = ThemeMode | 'system';

export interface ThemeConfig {
  /** localStorage key prefix, e.g. 'plyet' → keys 'plyet-theme', 'plyet-accent', … */
  storagePrefix?: string;
  defaultPref?: ThemePref;
  defaultAccent?: string;
  defaultBlur?: boolean;
  defaultTransparency?: boolean;
}

const hasWindow = () => typeof window !== 'undefined';
const hasDocument = () => typeof document !== 'undefined';

const DARK_QUERY = '(prefers-color-scheme: dark)';

class ThemeController {
  /** User preference: 'light' | 'dark' | 'system'. */
  pref = $state<ThemePref>('system');
  /** Resolved appearance applied to the DOM ('system' collapses to one of these). */
  mode = $state<ThemeMode>('light');
  accent = $state<string>('blue');
  blurEnabled = $state<boolean>(true);
  transparencyEnabled = $state<boolean>(true);

  #prefix = 'rg';
  #mql: MediaQueryList | null = null;
  #onSystemChange = () => {
    if (this.pref === 'system') this.#resolveAndApply();
  };

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

    this.pref = (this.#read('theme') as ThemePref) || config.defaultPref || 'system';
    this.accent = this.#read('accent') || config.defaultAccent || 'blue';

    const blur = this.#read('blur');
    this.blurEnabled = blur !== null ? blur !== 'off' : config.defaultBlur ?? true;

    const transparency = this.#read('transparency');
    this.transparencyEnabled =
      transparency !== null ? transparency !== 'off' : config.defaultTransparency ?? true;

    this.#watchSystem();
    this.#resolveMode();
    this.applyAll();
  }

  /** OS-reported appearance, falling back to 'light' when unknown (SSR). */
  #systemMode(): ThemeMode {
    if (hasWindow() && window.matchMedia) {
      return window.matchMedia(DARK_QUERY).matches ? 'dark' : 'light';
    }
    return 'light';
  }

  #watchSystem(): void {
    if (this.#mql || !hasWindow() || !window.matchMedia) return;
    this.#mql = window.matchMedia(DARK_QUERY);
    this.#mql.addEventListener('change', this.#onSystemChange);
  }

  #resolveMode(): void {
    this.mode = this.pref === 'system' ? this.#systemMode() : this.pref;
  }

  #resolveAndApply(): void {
    this.#resolveMode();
    if (hasDocument()) document.documentElement.setAttribute('data-theme', this.mode);
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

  setPref(pref: ThemePref): void {
    this.pref = pref;
    this.#write('theme', pref);
    this.#watchSystem();
    this.#resolveAndApply();
  }

  /** Cycle Light → Dark → System → Light, for a single-tap theme control. */
  cyclePref(): void {
    const order: ThemePref[] = ['light', 'dark', 'system'];
    const next = order[(order.indexOf(this.pref) + 1) % order.length];
    this.setPref(next);
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
