use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub key: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemStatus {
    Planned,
    Conducted,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Expense,
    Income,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub name: String,
    pub amount: i64,
    pub category: String,
    pub date: String,
    #[serde(default)]
    pub description: String,
    pub link: String,
    pub status: ItemStatus,
    pub item_type: ItemType,
    pub completed: bool,
    /// Link back to the recurring rule / product that materialized this item
    /// (`''` for manually-added items). Lets the UI offer "undo" on un-complete.
    #[serde(default)]
    pub source_kind: String,
    #[serde(default)]
    pub source_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: u64,
    pub name: String,
    pub limit: i64,
    pub icon: String,
    pub items: Vec<Item>,
}

/// A recurring income/expense rule. Future occurrences are computed virtually
/// on the frontend up to `horizon`; paying one materializes a real `Item` and
/// advances `last_paid_date`. Editing `amount` only affects future occurrences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recurring {
    pub id: u64,
    pub budget_id: u64,
    pub name: String,
    pub amount: i64,
    pub category: String,
    pub item_type: ItemType,
    /// 'weekly' | 'monthly' | 'yearly'
    pub freq: String,
    /// Day of month (1-31) for monthly/yearly, or weekday (0-6, Mon-Sun) for weekly.
    pub anchor_day: i64,
    pub start_date: String,
    #[serde(default)]
    pub end_date: Option<String>,
    pub horizon: i64,
    #[serde(default)]
    pub last_paid_date: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub link: String,
}

/// A complex financial product: a deposit (вклад) or a loan/mortgage (кредит).
/// Like recurring rules, future payments are virtual (computed on the frontend
/// up to `horizon`); materializing one writes a real `Item` and advances the
/// product's progress (`payments_made`, `principal_paid`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: u64,
    pub budget_id: u64,
    /// 'deposit' | 'loan'
    pub kind: String,
    pub name: String,
    /// Deposit body or loan amount, in kopecks.
    pub principal: i64,
    /// Annual rate in basis points (12% = 1200).
    pub annual_rate_bps: i64,
    pub term_months: i64,
    pub start_date: String,
    /// loan/mortgage: 'annuity'; deposit: 'simple'. (Other models are a later phase.)
    pub payment_model: String,
    /// Mortgage only: down payment (первоначальный взнос), in kopecks. Debited as
    /// a completed expense on creation; `principal` is the financed loan body.
    #[serde(default)]
    pub down_payment: i64,
    /// Reduced rate (bps) applied to earned interest on early deposit closure.
    #[serde(default)]
    pub early_rate_bps: Option<i64>,
    pub horizon: i64,
    pub category: String,
    /// 'active' | 'closed'
    pub status: String,
    /// Loan: principal repaid so far (kopecks). Remaining debt = principal - principal_paid.
    #[serde(default)]
    pub principal_paid: i64,
    /// Number of scheduled payments/payouts materialized so far.
    #[serde(default)]
    pub payments_made: i64,
    #[serde(default)]
    pub manual_payment: Option<i64>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppData {
    pub budgets: Vec<Budget>,
    pub categories: Vec<Category>,
    #[serde(default)]
    pub recurring: Vec<Recurring>,
    #[serde(default)]
    pub products: Vec<Product>,
    #[serde(default)]
    pub next_id: u64,
}

/// Result of a product mutation that also affects the budget (materialize,
/// extra payment, deposit open/close): the affected budget plus the updated
/// product list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductResult {
    pub budget: Budget,
    pub products: Vec<Product>,
}

/// Result of materializing a virtual occurrence: the affected budget (with the
/// new real item) plus the updated recurring list (advanced `last_paid_date`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializeResult {
    pub budget: Budget,
    pub recurring: Vec<Recurring>,
}

/// Result of un-materializing a sourced item (un-completing): the affected
/// budget plus both updated lists, since the rolled-back source may be a
/// recurring rule or a product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmaterializeResult {
    pub budget: Budget,
    pub recurring: Vec<Recurring>,
    pub products: Vec<Product>,
}

pub fn default_categories() -> Vec<Category> {
    vec![
        Category { key: "food".into(), name: "Еда".into(), icon: "food".into(), color: "#FF6D00".into(), is_default: true },
        Category { key: "transport".into(), name: "Транспорт".into(), icon: "transport".into(), color: "#1A73E8".into(), is_default: true },
        Category { key: "entertainment".into(), name: "Развлечения".into(), icon: "entertainment".into(), color: "#D93025".into(), is_default: true },
        Category { key: "shopping".into(), name: "Покупки".into(), icon: "shopping".into(), color: "#E91E63".into(), is_default: true },
        Category { key: "bills".into(), name: "Коммунальные".into(), icon: "bills".into(), color: "#7C4DFF".into(), is_default: true },
        Category { key: "salary".into(), name: "Зарплата".into(), icon: "salary".into(), color: "#0D904F".into(), is_default: true },
        Category { key: "other".into(), name: "Другое".into(), icon: "other".into(), color: "#00ACC1".into(), is_default: true },
    ]
}

