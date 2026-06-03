<script lang="ts">
  import 'reglass-material/tokens.css';
  import '../app.css';
  import { theme } from 'reglass-material/theme';
  import { keyboard, insets } from 'reglass-material';
  import { onMount } from 'svelte';

  // Hydrate + apply all four theming attributes from the persisted 'plyet-*'
  // settings. The blocking script in app.html already applied them pre-paint;
  // this keeps the runtime controller state in sync.
  theme.configure({ storagePrefix: 'plyet' });

  let { children } = $props();

  // Keep the Android transient status bar's icon tint in sync with the theme so
  // it stays legible when revealed by a swipe. No-op off Android (the native
  // PlyetNative bridge is only injected by MainActivity).
  $effect(() => {
    const dark = theme.mode === 'dark';
    const bridge = (window as unknown as { PlyetNative?: { setStatusBarDark?: (d: boolean) => void } })
      .PlyetNative;
    bridge?.setStatusBarDark?.(dark);
  });

  onMount(() => {
    const onWheel = (e: WheelEvent) => { if (e.ctrlKey) e.preventDefault(); };
    const onKeydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && ['+', '-', '=', '0'].includes(e.key)) e.preventDefault();
    };
    const onGesture = (e: Event) => e.preventDefault();

    const teardownKeyboard = keyboard.init();
    const teardownInsets = insets.init();

    window.addEventListener('wheel', onWheel, { passive: false });
    window.addEventListener('keydown', onKeydown);
    window.addEventListener('gesturestart', onGesture);
    window.addEventListener('gesturechange', onGesture);
    window.addEventListener('gestureend', onGesture);

    return () => {
      teardownKeyboard();
      teardownInsets();
      window.removeEventListener('wheel', onWheel);
      window.removeEventListener('keydown', onKeydown);
      window.removeEventListener('gesturestart', onGesture);
      window.removeEventListener('gesturechange', onGesture);
      window.removeEventListener('gestureend', onGesture);
    };
  });
</script>

{@render children()}
