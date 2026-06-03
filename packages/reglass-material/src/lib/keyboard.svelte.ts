// re:glass material — on-screen keyboard controller.
// A rune-based singleton that tracks the soft keyboard and reflects it on
// <html> so any component can react with pure CSS:
//   --rg-keyboard-height: <N>px   (always present once init() runs)
//   data-keyboard-open            (set when the keyboard is up)
//
// Two independent signals are merged:
//   1. Web — window.visualViewport. On modern Android the layout viewport also
//      shrinks (interactive-widget=resizes-content) so this height is ~0; on
//      iOS WKWebView it does not shrink, so the height equals the keyboard.
//   2. Native — Android pushes the real IME height (CSS px) via the global
//      bridge window.__plyetSetKeyboardInset(dp). This is the only signal that
//      works on old/no-GMS WebViews (e.g. Huawei) that lack interactive-widget,
//      where the web path is a no-op.
//
// To avoid double-avoidance on modern Android (layout already shrank AND native
// reports full IME height), we count only the part of the native keyboard that
// overlaps the *current* layout viewport:
//   layoutShrink   = baselineInnerHeight − window.innerHeight
//   nativeCoverage = max(0, nativeImeHeight − layoutShrink)
//   effective      = max(webHeight, nativeCoverage)

const OPEN_THRESHOLD = 80;

const hasWindow = () => typeof window !== 'undefined';
const hasDocument = () => typeof document !== 'undefined';

class KeyboardController {
  /** Effective keyboard height consumers should clear (CSS px). */
  height = $state(0);
  open = $state(false);

  // Raw pieces — exposed for the on-device debug panel.
  /** visualViewport-derived height (CSS px). */
  webHeight = $state(0);
  /** Native Android IME height (CSS px) pushed via the bridge. */
  nativeImeHeight = $state(0);
  /** Live window.innerHeight. */
  innerHeight = $state(0);
  /** innerHeight observed while the keyboard is closed (the layout baseline). */
  baselineInnerHeight = $state(0);

  #cleanup: (() => void) | null = null;

  /**
   * Start tracking the keyboard. Idempotent — calling init() while already
   * active returns the existing cleanup. Returns a teardown fn.
   */
  init(): () => void {
    if (this.#cleanup) return this.#cleanup;
    if (!hasWindow()) {
      this.#cleanup = () => {};
      return this.#cleanup;
    }

    this.innerHeight = window.innerHeight;
    this.baselineInnerHeight = window.innerHeight;

    // Native bridge — Kotlin calls this via evaluateJavascript with the IME
    // height in dp (already converted to CSS px on the native side via density).
    const bridge = (px: number) => this.setNativeInset(px);
    (window as unknown as { __plyetSetKeyboardInset?: (px: number) => void }).__plyetSetKeyboardInset =
      bridge;

    const vv = window.visualViewport;
    const onWeb = () => {
      this.innerHeight = window.innerHeight;
      if (vv) this.webHeight = Math.max(0, window.innerHeight - vv.height - vv.offsetTop);
      this.#recompute();
    };

    if (vv) {
      vv.addEventListener('resize', onWeb);
      vv.addEventListener('scroll', onWeb);
    }
    onWeb();

    this.#cleanup = () => {
      if (vv) {
        vv.removeEventListener('resize', onWeb);
        vv.removeEventListener('scroll', onWeb);
      }
      const w = window as unknown as { __plyetSetKeyboardInset?: (px: number) => void };
      if (w.__plyetSetKeyboardInset === bridge) delete w.__plyetSetKeyboardInset;
      this.webHeight = 0;
      this.nativeImeHeight = 0;
      this.#recompute();
      this.#cleanup = null;
    };
    return this.#cleanup;
  }

  /** Called from native (Android) with the IME height in CSS px. */
  setNativeInset(px: number): void {
    this.nativeImeHeight = Math.max(0, px || 0);
    if (hasWindow()) this.innerHeight = window.innerHeight;
    this.#recompute();
  }

  #recompute(): void {
    // While the keyboard is fully closed, track the current innerHeight as the
    // layout baseline so a later layout-shrink can be measured against it.
    const closed = this.webHeight <= OPEN_THRESHOLD && this.nativeImeHeight <= OPEN_THRESHOLD;
    if (closed && this.innerHeight > 0) this.baselineInnerHeight = this.innerHeight;

    const layoutShrink = Math.max(0, this.baselineInnerHeight - this.innerHeight);
    const nativeCoverage = Math.max(0, this.nativeImeHeight - layoutShrink);
    this.height = Math.max(this.webHeight, nativeCoverage);
    this.open = this.height > OPEN_THRESHOLD;
    this.#write();
  }

  #write(): void {
    if (!hasDocument()) return;
    const el = document.documentElement;
    el.style.setProperty('--rg-keyboard-height', `${this.height}px`);
    if (this.open) el.setAttribute('data-keyboard-open', '');
    else el.removeAttribute('data-keyboard-open');
  }
}

export const keyboard = new KeyboardController();
