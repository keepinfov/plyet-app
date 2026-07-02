// Calendar helpers for the month-based budget hierarchy.
//
// Timezone rule (applies app-wide):
// - Business dates (item.date, budget periods, "the current month") are
//   **device-local calendar dates**, handled as plain strings — the backend
//   never computes "today" for business logic.
// - created_at/updated_at/deleted_at are ISO8601 **UTC** metadata and never
//   drive month placement.
// - finance.ts uses Date.UTC internally, but only as pure calendar arithmetic
//   on date strings — that is consistent with this rule.

const MONTHS_RU = [
  'Январь', 'Февраль', 'Март', 'Апрель', 'Май', 'Июнь',
  'Июль', 'Август', 'Сентябрь', 'Октябрь', 'Ноябрь', 'Декабрь',
];

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

/** Today as a device-local YYYY-MM-DD business date. */
export function todayISO(): string {
  const d = new Date();
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Month key ("YYYY-MM") of a YYYY-MM-DD business date. */
export function periodOf(date: string): string {
  return date.slice(0, 7);
}

/** The device-local current month key. */
export function currentPeriod(): string {
  return periodOf(todayISO());
}

/** Russian display label for a month key, e.g. "2026-07" → "Июль 2026". */
export function periodLabel(period: string): string {
  const year = period.slice(0, 4);
  const m = Number(period.slice(5, 7));
  const name = MONTHS_RU[m - 1] ?? period;
  return `${name} ${year}`;
}

/** Short label for chips, e.g. "2026-07" → "Июль" (year added when not current). */
export function periodChipLabel(period: string): string {
  const year = period.slice(0, 4);
  const m = Number(period.slice(5, 7));
  const name = MONTHS_RU[m - 1] ?? period;
  return year === String(new Date().getFullYear()) ? name : `${name} ${year.slice(2)}`;
}
