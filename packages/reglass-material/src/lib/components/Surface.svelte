<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type SurfaceLevel = 1 | 2 | 3;

  interface Props extends HTMLAttributes<HTMLElement> {
    /**
     * Layering tier:
     *  1 — frosted glass background panel (topbar, sheets); honors blur/transparency.
     *  2 — solid M3 tonal surface (rows/cards sitting on glass).
     *  3 — tinted translucency over an M3 surface (the transaction-card look).
     */
    level?: SurfaceLevel;
    /** L3 tint color (any CSS color). Sets --rg-tint. */
    tint?: string;
    /** L3 tint strength as a percentage (default 22). */
    tintAmount?: number;
    /** L1 only: drop the glass blur and use the opaque sheet background. */
    solid?: boolean;
    variant?: 'filled' | 'outlined';
    /** Rendered element tag. */
    as?: string;
    radius?: string;
    /** Bindable reference to the rendered DOM element. */
    element?: HTMLElement | null;
    children?: Snippet;
  }

  let {
    level = 2,
    tint,
    tintAmount = 22,
    solid = false,
    variant = 'filled',
    as = 'div',
    radius = '1rem',
    element = $bindable(null),
    class: className = '',
    style = '',
    children,
    ...rest
  }: Props = $props();

  const styleVars = $derived(
    [
      `--rg-radius:${radius}`,
      tint != null ? `--rg-tint:${tint}` : '',
      `--rg-tint-amount:${tintAmount}%`,
      style,
    ]
      .filter(Boolean)
      .join(';')
  );
</script>

<svelte:element
  this={as}
  bind:this={element}
  class={`rg-surface ${className}`}
  data-level={level}
  data-variant={variant}
  data-solid={solid ? '' : undefined}
  style={styleVars}
  {...rest}
>
  {@render children?.()}
</svelte:element>

<style>
  .rg-surface {
    border-radius: var(--rg-radius, 1rem);
    border: none;
    box-shadow: none;
  }

  /* L1 — frosted glass; honors global blur/transparency tokens. */
  .rg-surface[data-level='1'] {
    background: var(--rg-sheet-glass-bg);
    backdrop-filter: var(--rg-glass-blur);
    -webkit-backdrop-filter: var(--rg-glass-blur);
  }
  .rg-surface[data-level='1'][data-solid] {
    background: var(--rg-sheet-bg);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  /* L2 — solid M3 tonal surface. */
  .rg-surface[data-level='2'] {
    background: var(--rg-surface-2);
  }

  /* L3 — tinted translucency over an M3 surface (ExpenseCard gold standard). */
  .rg-surface[data-level='3'] {
    background: color-mix(
      in srgb,
      var(--rg-tint, var(--rg-surface-2)) var(--rg-tint-amount, 22%),
      var(--rg-surface-2)
    );
  }

  .rg-surface[data-variant='outlined'] {
    border: 1px solid var(--rg-outline);
  }
</style>
