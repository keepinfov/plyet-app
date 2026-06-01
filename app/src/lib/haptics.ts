// Haptics module with safe fallback - no blocking imports
let hapticModule: any = null;
let isAvailable = false;

// Dynamically import haptics only in Tauri environment
if (typeof window !== 'undefined' && '__TAURI__' in window) {
  import('@tauri-apps/plugin-haptics')
    .then(module => {
      hapticModule = module;
      isAvailable = true;
    })
    .catch(() => {
      isAvailable = false;
    });
}

/**
 * Trigger light haptic feedback (for selections, toggles)
 */
export function hapticLight() {
  if (!isAvailable || !hapticModule) return;
  try {
    hapticModule.selectionFeedback();
  } catch {
    // Silently fail
  }
}

/**
 * Trigger medium impact haptic feedback (for button presses)
 */
export function hapticMedium() {
  if (!isAvailable || !hapticModule) return;
  try {
    hapticModule.impactFeedback('medium');
  } catch {
    // Silently fail
  }
}

/**
 * Trigger heavy impact haptic feedback (for important actions)
 */
export function hapticHeavy() {
  if (!isAvailable || !hapticModule) return;
  try {
    hapticModule.impactFeedback('heavy');
  } catch {
    // Silently fail
  }
}

/**
 * Trigger success notification haptic
 */
export function hapticSuccess() {
  if (!isAvailable || !hapticModule) return;
  try {
    hapticModule.notificationFeedback('success');
  } catch {
    // Silently fail
  }
}

/**
 * Trigger warning notification haptic
 */
export function hapticWarning() {
  if (!isAvailable || !hapticModule) return;
  try {
    hapticModule.notificationFeedback('warning');
  } catch {
    // Silently fail
  }
}

/**
 * Trigger error notification haptic
 */
export function hapticError() {
  if (!isAvailable || !hapticModule) return;
  try {
    hapticModule.notificationFeedback('error');
  } catch {
    // Silently fail
  }
}
