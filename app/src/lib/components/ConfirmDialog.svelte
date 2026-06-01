<script lang="ts">
  import { ConfirmDialog } from 'reglass-material';
  import { store } from '$lib/stores/budget.svelte';
  import { hapticLight } from '$lib/haptics';

  function confirm() {
    hapticLight();
    const cb = store.confirmCallback;
    store.closeConfirm();
    cb?.();
  }

  function cancel() {
    hapticLight();
    store.closeConfirm();
  }
</script>

<ConfirmDialog
  show={store.confirmShow}
  message={store.confirmMsg}
  confirmLabel={store.confirmLabel}
  onconfirm={confirm}
  oncancel={cancel}
/>
