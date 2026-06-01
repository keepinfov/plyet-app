<script lang="ts">
  import { slide } from 'svelte/transition';
  import { hapticLight } from '$lib/haptics';

  let { value = $bindable('') }: { value: string } = $props();

  const MONTHS = ['Январь','Февраль','Март','Апрель','Май','Июнь',
                   'Июль','Август','Сентябрь','Октябрь','Ноябрь','Декабрь'];
  const DAYS = ['Пн','Вт','Ср','Чт','Пт','Сб','Вс'];

  let open = $state(false);
  let viewDate = $state(new Date());
  let showYears = $state(false);
  let yearPage = $state(0);

  function prevMonth() { hapticLight(); viewDate = new Date(viewDate.getFullYear(), viewDate.getMonth() - 1, 1); }
  function nextMonth() { hapticLight(); viewDate = new Date(viewDate.getFullYear(), viewDate.getMonth() + 1, 1); }

  const yearGrid = $derived.by(() => {
    const base = Math.floor(viewDate.getFullYear() / 12) * 12 + yearPage * 12;
    return Array.from({ length: 12 }, (_, i) => base + i);
  });

  function selectYear(y: number) {
    hapticLight();
    viewDate = new Date(y, viewDate.getMonth(), 1);
    showYears = false;
  }

  const weeks = $derived.by(() => {
    const y = viewDate.getFullYear();
    const m = viewDate.getMonth();
    const firstDay = new Date(y, m, 1);
    let start = new Date(firstDay);
    const dayOfWeek = (firstDay.getDay() + 6) % 7;
    start.setDate(start.getDate() - dayOfWeek);
    const result: { day: number; month: number; year: number; isToday: boolean; isSelected: boolean; isOutside: boolean }[][] = [];
    const today = new Date(); today.setHours(0,0,0,0);
    const selDate = value ? new Date(value + 'T00:00:00') : null;
    for (let w = 0; w < 6; w++) {
      const week: any[] = [];
      for (let d = 0; d < 7; d++) {
        const dt = new Date(start);
        week.push({
          day: dt.getDate(), month: dt.getMonth(), year: dt.getFullYear(),
          isToday: dt.getTime() === today.getTime(),
          isSelected: selDate ? dt.getTime() === selDate.getTime() : false,
          isOutside: dt.getMonth() !== m,
        });
        start.setDate(start.getDate() + 1);
      }
      result.push(week);
    }
    return result;
  });

  function selectDate(day: number, month: number, year: number) {
    hapticLight();
    value = `${year}-${String(month + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
    open = false;
  }

  function formatDisplay(dateStr: string): string {
    if (!dateStr) return 'Выберите дату';
    const d = new Date(dateStr + 'T00:00:00');
    return `${d.getDate()} ${MONTHS[d.getMonth()].toLowerCase()} ${d.getFullYear()}`;
  }

  function openPicker() {
    if (value) viewDate = new Date(value + 'T00:00:00');
    else viewDate = new Date();
    open = !open;
  }
</script>

<div class="dp-wrap">
  <button class="date-btn" onclick={openPicker} type="button">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M19 3h-1V1h-2v2H8V1H6v2H5c-1.11 0-1.99.9-1.99 2L3 19c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V8h14v11zM9 10H7v2h2v-2zm4 0h-2v2h2v-2zm4 0h-2v2h2v-2z"/></svg>
    <span class:placeholder={!value}>{formatDisplay(value)}</span>
  </button>

  {#if open}
    <div class="calendar" transition:slide={{ duration: 200 }}>
      <div class="cal-header">
        {#if showYears}
          <button class="cal-nav tap-btn" onclick={() => yearPage--} aria-label="Предыдущие годы">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12z"/></svg>
          </button>
          <span class="cal-month">{yearGrid[0]} – {yearGrid[11]}</span>
          <button class="cal-nav tap-btn" onclick={() => yearPage++} aria-label="Следующие годы">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z"/></svg>
          </button>
        {:else}
          <button class="cal-nav tap-btn" onclick={prevMonth} aria-label="Предыдущий месяц">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12z"/></svg>
          </button>
          <button class="cal-month-btn" onclick={() => { showYears = true; yearPage = 0; }}>
            {MONTHS[viewDate.getMonth()]} {viewDate.getFullYear()}
          </button>
          <button class="cal-nav tap-btn" onclick={nextMonth} aria-label="Следующий месяц">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" stroke-linecap="round"><path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z"/></svg>
          </button>
        {/if}
      </div>
      {#if showYears}
        <div class="cal-year-grid">
          {#each yearGrid as y}
            <button class="cal-year" class:selected={y === viewDate.getFullYear()} onclick={() => selectYear(y)}>{y}</button>
          {/each}
        </div>
      {:else}
        <div class="cal-days">
          {#each DAYS as d}<span class="cal-day-label">{d}</span>{/each}
        </div>
        {#each weeks as week}
          <div class="cal-week">
            {#each week as d}
              <button class="cal-day" class:outside={d.isOutside} class:today={d.isToday} class:selected={d.isSelected}
                onclick={() => selectDate(d.day, d.month, d.year)}>{d.day}</button>
            {/each}
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .dp-wrap { position: relative; }

  .date-btn {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-radius: 0.75rem;
    border: none;
    background: var(--rg-surface-2);
    color: var(--rg-on-surface);
    font-size: 1rem;
    font-family: inherit;
    cursor: pointer;
    text-align: left;
    transition: box-shadow 0.2s ease, background 0.2s ease;
  }

  .date-btn:focus {
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--rg-primary) 20%, transparent);
    background: var(--rg-sheet-bg);
    outline: none;
  }

  .placeholder { color: var(--rg-on-surface-variant); opacity: 0.4; }

  .calendar {
    position: absolute;
    top: calc(100% + 0.5rem);
    left: 0;
    right: 0;
    z-index: 10;
    background: var(--rg-sheet-bg, var(--rg-surface-2));
    border-radius: 1rem;
    padding: 1rem;
    box-shadow: 0 8px 32px rgba(0,0,0,0.15);
    border: none;
  }

  .cal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }

  .cal-month, .cal-month-btn {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--rg-on-surface);
    background: none;
    border: none;
    font-family: inherit;
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    border-radius: 0.75rem;
  }

  .cal-month-btn:hover { background: var(--rg-surface-2); }

  .cal-year-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.5rem;
    padding: 0.25rem 0;
  }

  .cal-year {
    padding: 0.625rem;
    border: none;
    background: transparent;
    color: var(--rg-on-surface);
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
    border-radius: 0.75rem;
  }

  .cal-year.selected {
    background: var(--rg-primary);
    color: var(--rg-on-primary);
    font-weight: 600;
  }

  .cal-nav {
    width: 2rem; height: 2rem;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--rg-on-surface-variant);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .cal-days, .cal-week {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    text-align: center;
  }

  .cal-day-label {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--rg-on-surface-variant);
    padding: 0.375rem 0;
    text-transform: uppercase;
  }

  .cal-day {
    aspect-ratio: 1;
    border: none;
    background: transparent;
    color: var(--rg-on-surface);
    font-size: 0.8125rem;
    font-family: inherit;
    cursor: pointer;
    border-radius: 50%;
    transition: background 0.1s ease;
  }

  .cal-day.outside { color: var(--rg-outline); opacity: 0.4; }

  .cal-day.today {
    box-shadow: inset 0 0 0 1.5px var(--rg-primary);
    font-weight: 600;
  }

  .cal-day.selected {
    background: var(--rg-primary);
    color: var(--rg-on-primary);
    font-weight: 600;
    box-shadow: none;
  }
</style>
