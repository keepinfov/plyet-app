<script lang="ts">
  import Sheet from './Sheet.svelte';
  import Button from './Button.svelte';

  interface Props {
    show?: boolean;
    message?: string;
    confirmLabel?: string;
    cancelLabel?: string;
    onconfirm?: () => void;
    oncancel?: () => void;
  }

  let {
    show = false,
    message = '',
    confirmLabel = 'Удалить',
    cancelLabel = 'Отмена',
    onconfirm,
    oncancel,
  }: Props = $props();
</script>

<Sheet {show} maxWidth="20rem" zIndex={350} role="alertdialog" onclose={() => oncancel?.()}>
  <div class="rg-confirm-msg">{message}</div>
  <div class="rg-confirm-actions">
    <Button variant="surface" full onclick={() => oncancel?.()}>
      <svg width="1.125rem" height="1.125rem" viewBox="0 0 24 24" fill="currentColor"><path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg>
      {cancelLabel}
    </Button>
    <Button variant="danger" full onclick={() => onconfirm?.()}>
      <svg width="1.125rem" height="1.125rem" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
      {confirmLabel}
    </Button>
  </div>
</Sheet>

<style>
  .rg-confirm-msg {
    font-size: 1rem;
    font-weight: 700;
    color: var(--rg-on-surface);
    text-align: center;
    margin-bottom: 1.25rem;
    line-height: 1.4;
    overflow-wrap: break-word;
  }

  .rg-confirm-actions {
    display: flex;
    gap: 0.5rem;
  }
</style>
