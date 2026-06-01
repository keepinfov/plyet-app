<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import Surface from './Surface.svelte';

  interface Props extends HTMLAttributes<HTMLElement> {
    /** Tint color for the L3 translucency (e.g. a category color). */
    tint?: string;
    tintAmount?: number;
    radius?: string;
    element?: HTMLElement | null;
    children?: Snippet;
  }

  let {
    tint,
    tintAmount = 22,
    radius = '1rem',
    element = $bindable(null),
    children,
    ...rest
  }: Props = $props();
</script>

<Surface level={3} {tint} {tintAmount} {radius} bind:element {...rest}>
  {@render children?.()}
</Surface>
