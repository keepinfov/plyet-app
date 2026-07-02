<script lang="ts">
  import { store } from '$lib/stores/budget.svelte';
  import { formatMoney } from '$lib/utils';
  import RetypeNumber from './RetypeNumber.svelte';
  import { fly, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { onMount, onDestroy } from 'svelte';

  const categories = $derived(store.data?.categories ?? []);

  // Planned totals include virtual (not-yet-materialized) recurring occurrences;
  // completed totals below read only real items, so the real balance stays real.
  const allExpenseItems = $derived(
    store.feedItems.filter(i => i.item_type !== 'income')
  );

  // Only completed expenses for chart display
  const completedExpenseItems = $derived(
    allExpenseItems.filter(i => i.completed)
  );

  // Only completed income counts toward budget
  const totalIncome = $derived(
    store.scopeItems.filter(i => i.item_type === 'income' && i.completed).reduce((s, i) => s + i.amount, 0)
  );

  // Chart shows actual (completed) spending
  const totalSpent = $derived(
    completedExpenseItems.reduce((s, i) => s + i.amount, 0)
  );

  // Remaining accounts for ALL expenses (completed + planned)
  const totalPlanned = $derived(
    allExpenseItems.reduce((s, i) => s + i.amount, 0)
  );

  const allIncomeItems = $derived(
    store.feedItems.filter(i => i.item_type === 'income')
  );
  const totalAllIncome = $derived(
    allIncomeItems.reduce((s, i) => s + i.amount, 0)
  );

  const effectiveLimit = $derived(store.scopeLimit + totalIncome);
  const remaining = $derived(effectiveLimit - totalPlanned);

  const plannedExpenses = $derived(totalPlanned - totalSpent);
  const plannedIncome = $derived(totalAllIncome - totalIncome);
  const realBalance = $derived(effectiveLimit - totalSpent);
  const effectiveLimitAll = $derived(store.scopeLimit + totalAllIncome);
  const plannedBalance = $derived(effectiveLimitAll - totalPlanned);

  let centerView = $state<'detail' | 'balance'>('balance');

  const centerScale = $derived.by(() => {
    const values = centerView === 'detail'
      ? [totalSpent, plannedExpenses, totalIncome, plannedIncome]
      : [realBalance, plannedBalance];
    const maxLen = Math.max(...values.map(v => formatMoney(Math.abs(v)).length));
    if (maxLen <= 12) return 1;
    return Math.max(0.7, 12 / maxLen);
  });

  const catTotals = $derived.by(() => {
    const map: Record<string, number> = {};
    for (const item of completedExpenseItems) {
      map[item.category] = (map[item.category] ?? 0) + item.amount;
    }
    return map;
  });

  const catEntries = $derived(
    Object.entries(catTotals)
      .map(([key, amount]) => ({ key, amount, cat: categories.find(c => c.key === key) }))
      .filter((e): e is { key: string; amount: number; cat: NonNullable<typeof e.cat> } => e.amount > 0 && !!e.cat)
      .sort((a, b) => b.amount - a.amount)
  );

  // Sum only entries with existing categories (for chart proportions)
  const chartTotal = $derived(catEntries.reduce((s, e) => s + e.amount, 0));

  // Keep legend items in DOM during collapse animation
  let displayEntries = $state<typeof catEntries>([]);
  let hideTimeout: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (catEntries.length > 0) {
      if (hideTimeout) { clearTimeout(hideTimeout); hideTimeout = null; }
      displayEntries = catEntries;
    } else if (displayEntries.length > 0) {
      hideTimeout = setTimeout(() => { displayEntries = []; }, 350);
    }
  });

  onDestroy(() => { if (hideTimeout) clearTimeout(hideTimeout); });


  // Rounded rectangle geometry — wide landscape shape
  const w = 360;
  const h = 180;
  const m = 22;      // margin
  const r = 32;      // corner radius
  const sw = 36;     // stroke width

  // Build rounded rect path
  const d = [
    `M ${m + r},${m}`,
    `L ${w - m - r},${m}`,
    `A ${r},${r} 0 0 1 ${w - m},${m + r}`,
    `L ${w - m},${h - m - r}`,
    `A ${r},${r} 0 0 1 ${w - m - r},${h - m}`,
    `L ${m + r},${h - m}`,
    `A ${r},${r} 0 0 1 ${m},${h - m - r}`,
    `L ${m},${m + r}`,
    `A ${r},${r} 0 0 1 ${m + r},${m}`,
    `Z`,
  ].join(' ');

  // Total perimeter: 2 horizontal + 2 vertical edges + 4 quarter circles
  const straightLen = 2 * (w - 2 * m - 2 * r) + 2 * (h - 2 * m - 2 * r);
  const cornerLen = 2 * Math.PI * r;
  const totalLen = straightLen + cornerLen;

  // Animate chart segments from 0 on first render and when entries first appear
  let chartProgress = $state(0);
  let prevEntryCount = 0;

  onMount(() => {
    requestAnimationFrame(() => { chartProgress = 1; });
  });

  $effect(() => {
    const count = catEntries.length;
    if (count > 0 && prevEntryCount === 0 && chartProgress === 1) {
      chartProgress = 0;
      requestAnimationFrame(() => { chartProgress = 1; });
    }
    prevEntryCount = count;
  });
</script>

{#if store.currentRoot}
  <div class="chart-section" transition:fly={{ y: 12, duration: 300, easing: cubicOut }}>
    <div class="chart-card">
    <div class="chart-wrap">
      <svg class="chart-svg" viewBox="0 0 {w} {h}">
        <!-- Track: rounded rectangle -->
        <path
          d={d}
          fill="none"
          stroke="var(--rg-outline)"
          stroke-width={sw}
          stroke-linecap="round"
          stroke-linejoin="round"
          opacity="0.5"
        />

        {#each catEntries as entry, idx (entry.key)}
          {@const segLen = chartTotal > 0 ? (entry.amount / chartTotal) * totalLen * chartProgress : 0}
          {@const offset = chartTotal > 0 ? catEntries.slice(0, idx).reduce((acc, e) => acc + (e.amount / chartTotal) * totalLen * chartProgress, 0) : 0}
          <path
            d={d}
            fill="none"
            stroke={entry.cat.color}
            stroke-width={sw}
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-dasharray="{Math.max(0.1, segLen)} {totalLen}"
            stroke-dashoffset={-offset}
            style="transition: stroke-dasharray 0.3s ease-out, stroke-dashoffset 0.3s ease-out;"
          />
        {/each}
      </svg>

      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="chart-center" style="--center-scale: {centerScale}" onclick={() => centerView = centerView === 'detail' ? 'balance' : 'detail'}>
        {#key centerView}
          <div class="center-content" in:fly={{ y: 8, duration: 200, easing: cubicOut }} out:fly={{ y: -8, duration: 150 }}>
            {#if centerView === 'detail'}
              <div class="detail-line">
                <span class="line-neg">-<RetypeNumber value={totalSpent} /></span><span class="line-planned" class:hidden-value={plannedExpenses <= 0}>-<RetypeNumber value={plannedExpenses} /></span>
              </div>
              <div class="detail-line">
                <span class="line-pos">+<RetypeNumber value={totalIncome} /></span><span class="line-planned" class:hidden-value={plannedIncome <= 0}>+<RetypeNumber value={plannedIncome} /></span>
              </div>
            {:else}
              <span class="balance-value" class:negative={realBalance < 0}>
                {realBalance < 0 ? '-' : ''}<RetypeNumber value={Math.abs(realBalance)} />
              </span>
              <span class="balance-planned" class:balance-hidden={plannedBalance === realBalance}>
                {plannedBalance < 0 ? '-' : ''}<RetypeNumber value={Math.abs(plannedBalance)} />
              </span>
            {/if}
          </div>
        {/key}
      </div>
    </div>

    <div class="legend-wrap" class:has-items={catEntries.length > 0}>
      <div class="chart-legend">
        {#each displayEntries as entry (entry.key)}
          <div class="legend-item tap-btn" style="--legend-bg: {entry.cat.color}14; --legend-color: {entry.cat.color}" transition:scale={{ duration: 200, start: 0.8, easing: cubicOut }}>
            <span class="legend-dot" style="background:{entry.cat.color}"></span>
            {entry.cat.name} · {chartTotal > 0 ? Math.round((entry.amount / chartTotal) * 100) : 0}%
          </div>
        {/each}
      </div>
    </div>
    </div>
  </div>
{/if}

<style>
  .chart-section {
    padding: 0.75rem 1rem 0;
    contain: style;
  }

  .chart-card {
    background: var(--rg-surface-2);
    border-radius: 1rem;
    padding: 1.25rem 1rem 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .chart-wrap {
    position: relative;
    width: 100%;
    aspect-ratio: 2 / 1;
    display: flex;
    align-items: center;
    justify-content: center;
    container-type: inline-size;
  }

  .chart-svg {
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .chart-center {
    position: absolute;
    inset: 0;
    text-align: center;
    pointer-events: auto;
    cursor: pointer;
    display: grid;
    place-items: center;
    padding: 2rem 3.5rem;
  }

  .center-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    grid-area: 1 / 1;
  }

  .detail-line {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: center;
    line-height: 1.4;
    max-width: 100%;
  }

  .line-neg {
    font-size: calc(clamp(1.125rem, 6.5cqw, 2rem) * var(--center-scale, 1));
    font-weight: 700;
    color: var(--rg-price-neg);
  }

  .line-pos {
    font-size: calc(clamp(1.125rem, 6.5cqw, 2rem) * var(--center-scale, 1));
    font-weight: 700;
    color: var(--rg-price-pos);
  }

  .line-planned {
    font-size: calc(clamp(0.875rem, 4.5cqw, 1.375rem) * var(--center-scale, 1));
    font-weight: 600;
    color: var(--rg-on-surface-variant);
    opacity: 0.5;
    transition: opacity 0.3s ease, max-width 0.3s ease;
    max-width: 10rem;
    margin-left: 0.35rem;
    white-space: nowrap;
  }

  .line-planned.hidden-value {
    opacity: 0;
    max-width: 0;
    margin-left: 0;
    overflow: hidden;
  }

  .balance-value {
    font-size: calc(clamp(1.75rem, 8.5cqw, 2.75rem) * var(--center-scale, 1));
    font-weight: 700;
    color: var(--rg-price-pos);
    line-height: 1.2;
  }

  .balance-value.negative {
    color: var(--rg-price-neg);
  }

  .balance-planned {
    font-size: calc(clamp(0.875rem, 4.5cqw, 1.375rem) * var(--center-scale, 1));
    font-weight: 600;
    color: var(--rg-on-surface-variant);
    opacity: 0.5;
    max-height: 1.5rem;
    overflow: hidden;
    transition: opacity 0.3s ease, max-height 0.3s ease;
  }

  .balance-planned.balance-hidden {
    opacity: 0;
    max-height: 0;
  }

  .legend-wrap {
    max-height: 0;
    opacity: 0;
    margin-top: 0;
    transition: max-height 0.28s cubic-bezier(0.4, 0, 0.2, 1),
                opacity 0.25s ease,
                margin-top 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    width: 100%;
  }

  .legend-wrap.has-items {
    opacity: 1;
    margin-top: 1rem;
    max-height: 8rem;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }

  .legend-wrap.has-items::-webkit-scrollbar {
    display: none;
  }

  .chart-legend {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.375rem;
    padding: 0.125rem 0.5rem;
  }

  .chart-legend::-webkit-scrollbar {
    display: none;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--legend-color, var(--rg-on-surface-variant));
    background: var(--legend-bg, rgba(95,99,104,0.08));
    border: none;
    padding: 0.3125rem 0.6875rem;
    border-radius: 0.75rem;
    transition: transform 0.12s ease;
    flex-shrink: 0;
    white-space: nowrap;
  }

  .legend-item:hover { transform: translateY(-1px); }

  .legend-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 0.125rem;
    flex-shrink: 0;
  }
</style>
