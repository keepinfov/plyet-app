export interface Category {
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

export interface Item {
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
  /** Runtime-only: true for virtual occurrences generated from a recurring rule (never persisted). */
  virtual?: boolean;
  /** Runtime-only: link back to the rule that generated this virtual occurrence. */
  source?: ItemSource;
}

export interface Recurring {
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

export interface Product {
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

export interface MaterializeResult {
  budget: Budget;
  recurring: Recurring[];
}

export interface ProductResult {
  budget: Budget;
  products: Product[];
}

export interface UnmaterializeResult {
  budget: Budget;
  recurring: Recurring[];
  products: Product[];
}

export interface Budget {
  id: number;
  name: string;
  limit: number;
  icon: string;
  items: Item[];
}

export interface AppData {
  budgets: Budget[];
  categories: Category[];
  recurring: Recurring[];
  products: Product[];
}
