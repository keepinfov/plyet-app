// re:glass material — system-bar inset controller.
// A rune-based singleton that receives framework-measured Android insets and
// reflects them on <html> so layout can reserve real space with pure CSS:
//   --rg-inset-top: <N>px      (cutout / hidden status-bar height)
//   --rg-inset-bottom: <N>px   (navigation-bar height)
//
// Under edge-to-edge the WebView's env(safe-area-inset-*) only reflects the
// display cutout — not the hidden status bar, and not the nav bar. Android
// pushes the real values (CSS px) via the global bridge
// window.__plyetSetInsets(top, bottom). On iOS / desktop the bridge never
// fires, so these vars stay unset and consumers fall back to env() via max().

const hasWindow = () => typeof window !== 'undefined';
const hasDocument = () => typeof document !== 'undefined';

class InsetsController {
  /** Top inset (cutout / status bar) in CSS px. */
  top = $state(0);
  /** Bottom inset (navigation bar) in CSS px. */
  bottom = $state(0);

  #cleanup: (() => void) | null = null;

  /** Start receiving native insets. Idempotent. Returns a teardown fn. */
  init(): () => void {
    if (this.#cleanup) return this.#cleanup;
    if (!hasWindow()) {
      this.#cleanup = () => {};
      return this.#cleanup;
    }

    const bridge = (top: number, bottom: number) => this.set(top, bottom);
    (window as unknown as { __plyetSetInsets?: (t: number, b: number) => void }).__plyetSetInsets =
      bridge;

    // The native listener emits insets once during initial layout — before this
    // bridge existed — so that first value was dropped. Now that the bridge is
    // registered, ask native to re-dispatch so insets apply immediately instead
    // of only after the next window change. No-op off Android.
    (window as unknown as { PlyetNative?: { requestInsets?: () => void } }).PlyetNative?.requestInsets?.();

    this.#cleanup = () => {
      const w = window as unknown as { __plyetSetInsets?: (t: number, b: number) => void };
      if (w.__plyetSetInsets === bridge) delete w.__plyetSetInsets;
      this.#cleanup = null;
    };
    return this.#cleanup;
  }

  /** Called from native (Android) with inset heights in CSS px. */
  set(top: number, bottom: number): void {
    this.top = Math.max(0, top || 0);
    this.bottom = Math.max(0, bottom || 0);
    this.#write();
  }

  #write(): void {
    if (!hasDocument()) return;
    const el = document.documentElement;
    el.style.setProperty('--rg-inset-top', `${this.top}px`);
    el.style.setProperty('--rg-inset-bottom', `${this.bottom}px`);
  }
}

export const insets = new InsetsController();
