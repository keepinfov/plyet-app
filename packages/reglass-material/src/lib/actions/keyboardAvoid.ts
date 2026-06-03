// re:glass material — keyboardAvoid action.
// Attach to a modal/sheet overlay. When an input or textarea inside it gains
// focus, scroll the field into the center of the visible viewport once the
// keyboard has settled. Positioning of the overlay itself is handled by CSS via
// the --rg-keyboard-height variable (see keyboard.svelte.ts); this action only
// nudges the focused field into view.

export function keyboardAvoid(node: HTMLElement) {
  let timer: ReturnType<typeof setTimeout> | null = null;

  const onFocusIn = (e: FocusEvent) => {
    const target = e.target as HTMLElement | null;
    if (!target || (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA')) return;

    if (timer) clearTimeout(timer);
    // Wait for the keyboard animation / viewport resize to settle, then scroll
    // once instead of on every resize+scroll event.
    timer = setTimeout(() => {
      timer = null;
      target.scrollIntoView({ block: 'center', behavior: 'smooth' });
    }, 250);
  };

  node.addEventListener('focusin', onFocusIn);

  return {
    destroy() {
      if (timer) clearTimeout(timer);
      node.removeEventListener('focusin', onFocusIn);
    },
  };
}
