/**
 * Sync-foundation fields shared by every persisted entity (mirrors the Rust
 * structs in src-tauri/src/data.rs):
 * - `uuid` — stable global identity (local integer ids collide across devices)
 * - `created_at` / `updated_at` — ISO8601 UTC metadata; never business logic
 * - `deleted_at` — soft-delete tombstone; the UI only ever receives live rows,
 *   tombstones exist in exports and for a future cloud sync
 */
export interface SyncMeta {
  uuid: string;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface Category extends SyncMeta {
  key: string;
  name: string;
  icon: string;
  color: string;
  is_default: boolean;
}

export type ItemType = 'expense' | 'income';
export type Freq = 'weekly' | 'monthly' | 'yearly';

/** Where a feed item came from — only set on virtual (not-yet-materialized) occurrences. */
export interface ItemSource {
  kind: 'recurring' | 'deposit' | 'loan' | 'mortgage';
  id: number;
  date: string;
  /** Loan only: principal repaid by this payment (kopecks); rest is interest. */
  principalPortion?: number;
  /** True when materializing this occurrence closes the product (final loan payment / deposit maturity). */
  closes?: boolean;
}

export interface Item extends SyncMeta {
  id: number;
  name: string;
  amount: number;
  category: string;
  date: string;
  description: string;
  link: string;
  item_type: ItemType;
  completed: boolean;
  /** Set on real items materialized from a recurring rule or product ('' for manual items). */
  source_kind?: string;
  source_id?: number;
  /** Member uuid of whoever recorded this item; null until multi-user lands. */
  author_id: string | null;
  /** Runtime-only: true for virtual occurrences generated from a recurring rule (never persisted). */
  virtual?: boolean;
  /** Runtime-only: link back to the rule that generated this virtual occurrence. */
  source?: ItemSource;
  /** Runtime-only: name of the custom sub-budget this item was reflected from (month view badge). */
  reflectedFrom?: string;
}

export interface Recurring extends SyncMeta {
  id: number;
  budget_id: number;
  name: string;
  amount: number;
  category: string;
  item_type: ItemType;
  freq: Freq;
  /** Day of month (1-31) for monthly/yearly, or weekday (0-6, Mon-Sun) for weekly. */
  anchor_day: number;
  start_date: string;
  end_date: string | null;
  horizon: number;
  last_paid_date: string | null;
  description: string;
  link: string;
}

export type ProductKind = 'deposit' | 'loan' | 'mortgage';
export type PaymentModel = 'annuity' | 'simple' | 'manual' | 'capitalized';
export type ProductStatus = 'active' | 'closed';

export interface Product extends SyncMeta {
  id: number;
  budget_id: number;
  kind: ProductKind;
  name: string;
  /** Deposit body or loan amount, in kopecks. */
  principal: number;
  /** Annual rate in basis points (12% = 1200). */
  annual_rate_bps: number;
  term_months: number;
  start_date: string;
  payment_model: PaymentModel;
  early_rate_bps: number | null;
  horizon: number;
  category: string;
  status: ProductStatus;
  /** Loan: principal repaid so far (kopecks). Remaining debt = principal - principal_paid. */
  principal_paid: number;
  payments_made: number;
  manual_payment: number | null;
  /** Mortgage only: down payment (первоначальный взнос), debited upfront. */
  down_payment: number;
  description: string;
  link: string;
}

export type BudgetKind = 'root' | 'month' | 'custom';

/**
 * Budgets form a two-level tree:
 * - root (`parent_id === null`) — a top-level budget; its `limit` is the
 *   default inherited by auto-created month sub-budgets
 * - month (`period === 'YYYY-MM'`) — auto-created; items are routed here by date
 * - custom — manually created ("Отпуск"); `reflect_in_months` mirrors its
 *   items into the root's month views (with a source badge)
 */
export interface Budget extends SyncMeta {
  id: number;
  name: string;
  limit: number;
  icon: string;
  items: Item[];
  parent_id: number | null;
  kind: BudgetKind;
  period: string | null;
  reflect_in_months: boolean;
}

/** Future multi-user attribution: only the schema exists today (single 'owner'). */
export interface Member extends SyncMeta {
  id: number;
  name: string;
  role: 'owner' | 'editor' | 'viewer';
}

/** Item mutations return the full live budget list (routing can create months). */
export interface MaterializeResult {
  budgets: Budget[];
  recurring: Recurring[];
}

export interface ProductResult {
  budgets: Budget[];
  products: Product[];
}

export interface UnmaterializeResult {
  budgets: Budget[];
  recurring: Recurring[];
  products: Product[];
}

export interface AppData {
  budgets: Budget[];
  categories: Category[];
  recurring: Recurring[];
  products: Product[];
  members: Member[];
}
