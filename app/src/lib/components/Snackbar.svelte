<script lang="ts">
  import { Snackbar } from 'reglass-material';
  import { store } from '$lib/stores/budget.svelte';
</script>

<Snackbar message={store.snackbarMsg} show={store.snackbarShow} />
