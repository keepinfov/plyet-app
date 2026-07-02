// Pure financial/date math for recurring rules (and, later, products).
// No Svelte/Tauri imports — keep these functions easily testable.

import type { Recurring, Product, Item } from '$lib/types';

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

function toStr(y: number, m: number, d: number): string {
  return `${y}-${pad(m)}-${pad(d)}`;
}

function parse(s: string): { y: number; m: number; d: number } {
  const [y, m, d] = s.split('-').map(Number);
  return { y, m, d };
}

/** Number of days in month `m` (1-12) of year `y`. */
function daysInMonth(y: number, m: number): number {
  return new Date(Date.UTC(y, m, 0)).getUTCDate();
}

/** Scheduled occurrence dates for a rule, in ascending order, ignoring paid/end state. */
function* scheduledDates(rule: Recurring): Generator<string> {
  const { y: sy, m: sm, d: sd } = parse(rule.start_date);

  if (rule.freq === 'weekly') {
    // anchor_day: 0-6 = Mon-Sun. JS getUTCDay: 0=Sun..6=Sat.
    const targetJsDay = (rule.anchor_day + 1) % 7;
    let cur = new Date(Date.UTC(sy, sm - 1, sd));
    let guard = 0;
    while (cur.getUTCDay() !== targetJsDay && guard++ < 7) {
      cur = new Date(cur.getTime() + 86_400_000);
    }
    for (let i = 0; i < 1000; i++) {
      yield toStr(cur.getUTCFullYear(), cur.getUTCMonth() + 1, cur.getUTCDate());
      cur = new Date(cur.getTime() + 7 * 86_400_000);
    }
  } else if (rule.freq === 'yearly') {
    let y = sy;
    for (let i = 0; i < 200; i++) {
      const dim = daysInMonth(y, sm);
      yield toStr(y, sm, Math.min(rule.anchor_day || sd, dim));
      y++;
    }
  } else {
    // monthly
    let y = sy;
    let m = sm;
    for (let i = 0; i < 1200; i++) {
      const dim = daysInMonth(y, m);
      yield toStr(y, m, Math.min(rule.anchor_day || sd, dim));
      m++;
      if (m > 12) {
        m = 1;
        y++;
      }
    }
  }
}

/**
 * Up to `horizon` upcoming unpaid occurrence dates (YYYY-MM-DD) for a rule.
 * "Unpaid" = scheduled date strictly after `last_paid_date` (or any date if never
 * paid). Bounded by `start_date`/`end_date`. Materializing a payment advances
 * `last_paid_date`, so the next call naturally returns the following occurrence(s).
 */
export function nextOccurrences(rule: Recurring, horizon: number): string[] {
  const out: string[] = [];
  const limit = Math.max(1, Math.min(horizon, 24));
  for (const d of scheduledDates(rule)) {
    if (d < rule.start_date) continue;
    if (rule.end_date && d > rule.end_date) break;
    if (rule.last_paid_date && d <= rule.last_paid_date) continue;
    out.push(d);
    if (out.length >= limit) break;
  }
  return out;
}

/** Add `n` whole months to a YYYY-MM-DD date, clamping the day to the target month length. */
export function addMonths(date: string, n: number): string {
  const { y, m, d } = parse(date);
  const total = (y * 12 + (m - 1)) + n;
  const ny = Math.floor(total / 12);
  const nm = (total % 12) + 1;
  const dim = daysInMonth(ny, nm);
  return toStr(ny, nm, Math.min(d, dim));
}

/** Monthly interest rate as a fraction, from annual basis points (12% = 1200 → 0.01). */
function monthlyRate(annualRateBps: number): number {
  return annualRateBps / 120_000;
}

/**
 * Fixed monthly annuity payment (kopecks) for a loan of `principal` at `rateBps`
 * annual over `termMonths`. Falls back to straight principal/term when rate is 0.
 */
export function annuityPayment(principal: number, rateBps: number, termMonths: number): number {
  if (termMonths <= 0) return 0;
  const i = monthlyRate(rateBps);
  if (i === 0) return Math.round(principal / termMonths);
  const factor = (i * Math.pow(1 + i, termMonths)) / (Math.pow(1 + i, termMonths) - 1);
  return Math.round(principal * factor);
}

/** Simple per-period interest income (kopecks) for a deposit body at `rateBps`. */
export function depositInterestPerPeriod(principal: number, rateBps: number): number {
  return Math.round(principal * monthlyRate(rateBps));
}

/**
 * Virtual upcoming occurrences for a product, as Item-like objects (never persisted).
 * Loan: monthly annuity expenses, each carrying its principal portion; the final
 * payment clamps to the remaining balance and closes the loan. Deposit: monthly
 * interest income; the maturity occurrence also returns the principal and closes.
 */
export function productOccurrences(product: Product, horizon: number): Item[] {
  if (product.status === 'closed') return [];
  const limit = Math.max(1, Math.min(horizon, 24));
  const out: Item[] = [];
  const done = product.payments_made;
  const term = product.term_months;
  if (done >= term) return [];

  const mk = (
    idx: number,
    amount: number,
    item_type: 'expense' | 'income',
    principalPortion: number,
    closes: boolean,
  ): Item => ({
    // Offset into a distinct negative range so product ids never collide with
    // recurring synthetic ids (which use a smaller -(id*100+i)-1 scheme).
    id: -1_000_000_000 - (product.id * 1000 + idx),
    name: product.name,
    amount,
    category: product.category,
    date: addMonths(product.start_date, idx + 1),
    description: product.description,
    link: product.link,
    item_type,
    completed: false,
    virtual: true,
    source: { kind: product.kind, id: product.id, date: addMonths(product.start_date, idx + 1), principalPortion, closes },
    uuid: '',
    created_at: '',
    updated_at: '',
    deleted_at: null,
    author_id: null,
  });

  if (product.kind === 'loan' || product.kind === 'mortgage') {
    const payment = annuityPayment(product.principal, product.annual_rate_bps, term);
    const i = monthlyRate(product.annual_rate_bps);
    let balance = product.principal - product.principal_paid;
    for (let k = done; k < term && out.length < limit; k++) {
      const interest = Math.round(balance * i);
      let principalPortion = payment - interest;
      let amount = payment;
      let closes = false;
      if (k === term - 1 || principalPortion >= balance) {
        principalPortion = balance;
        amount = balance + interest;
        closes = true;
      }
      out.push(mk(k, amount, 'expense', principalPortion, closes));
      balance -= principalPortion;
      if (closes) break;
    }
  } else {
    const interest = depositInterestPerPeriod(product.principal, product.annual_rate_bps);
    for (let k = done; k < term && out.length < limit; k++) {
      const isMaturity = k === term - 1;
      const amount = isMaturity ? interest + product.principal : interest;
      out.push(mk(k, amount, 'income', 0, isMaturity));
    }
  }
  return out;
}
