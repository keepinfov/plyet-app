use serde::{Deserialize, Serialize};

/// Sync-foundation fields shared by every entity (flattened into each struct
/// rather than nested, to keep the SQLite mapping and the TS mirror flat):
/// - `uuid`: stable global identity (local integer ids collide across devices)
/// - `created_at` / `updated_at`: ISO8601 UTC metadata timestamps; never used
///   for business logic (business dates are local calendar-date strings)
/// - `deleted_at`: soft-delete tombstone, kept so a future sync can propagate
///   deletions; `None` = live row. The UI only ever sees live rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub key: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<String>,
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
    pub item_type: ItemType,
    pub completed: bool,
    /// Link back to the recurring rule / product that materialized this item
    /// (`''` for manually-added items). Lets the UI offer "undo" on un-complete.
    #[serde(default)]
    pub source_kind: String,
    #[serde(default)]
    pub source_id: u64,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<String>,
    /// Member uuid of whoever recorded this item. `None` until multi-user
    /// arrives; the column exists so sync/attribution won't need a migration.
    #[serde(default)]
    pub author_id: Option<String>,
}

fn default_budget_kind() -> String {
    "root".into()
}

/// Budgets form a two-level tree:
/// - `kind = 'root'`, `parent_id = None` — a top-level budget. Its `limit` is
///   the default limit inherited by newly auto-created month sub-budgets.
/// - `kind = 'month'`, `period = Some("YYYY-MM")` — an auto-created month
///   sub-budget; items are routed here by their business date.
/// - `kind = 'custom'` — a manually-created themed sub-budget ("Отпуск");
///   `reflect_in_months` controls whether its items also show up (with a
///   source badge) in the month views of the same root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: u64,
    pub name: String,
    pub limit: i64,
    pub icon: String,
    pub items: Vec<Item>,
    #[serde(default)]
    pub parent_id: Option<u64>,
    #[serde(default = "default_budget_kind")]
    pub kind: String,
    #[serde(default)]
    pub period: Option<String>,
    #[serde(default)]
    pub reflect_in_months: bool,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<String>,
}

/// A recurring income/expense rule. Rules belong to a **root** budget; future
/// occurrences are computed virtually on the frontend up to `horizon`, and
/// paying one materializes a real `Item` into the month sub-budget matching
/// the payment date, advancing `last_paid_date`. Editing `amount` only affects
/// future occurrences.
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
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<String>,
}

/// A complex financial product: a deposit (вклад) or a loan/mortgage (кредит).
/// Products belong to a **root** budget; like recurring rules, future payments
/// are virtual (computed on the frontend up to `horizon`), and materializing
/// one writes a real `Item` into the month sub-budget matching the payment
/// date, advancing the product's progress (`payments_made`, `principal_paid`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: u64,
    pub budget_id: u64,
    /// 'deposit' | 'loan' | 'mortgage'
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
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<String>,
}

/// A participant of the (future) shared budget. Only the schema exists today:
/// a single 'owner' member is seeded so `items.author_id` has something to
/// point at once attribution UI lands. Roles: 'owner' | 'editor' | 'viewer'.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id: u64,
    pub uuid: String,
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<String>,
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
    pub members: Vec<Member>,
    #[serde(default)]
    pub next_id: u64,
}

/// Versioned JSON export envelope. Legacy exports (plain `AppData`) are still
/// importable: the importer sniffs for the `schema` field and normalizes
/// legacy data through the same path as the v5→v6 DB migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEnvelope {
    pub schema: u32,
    pub exported_at: String,
    pub data: AppData,
}

/// Result of a product mutation that also affects budgets (materialize,
/// extra payment, deposit open/close). Item routing can auto-create a month
/// sub-budget, so mutations return the full (live) budget list instead of a
/// single budget — the dataset is small and local, and this removes all
/// client-side merge bookkeeping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductResult {
    pub budgets: Vec<Budget>,
    pub products: Vec<Product>,
}

/// Result of materializing a virtual occurrence: all live budgets (the new
/// real item may have created its month sub-budget) plus the updated
/// recurring list (advanced `last_paid_date`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializeResult {
    pub budgets: Vec<Budget>,
    pub recurring: Vec<Recurring>,
}

/// Result of un-materializing a sourced item (un-completing): all live budgets
/// plus both updated lists, since the rolled-back source may be a recurring
/// rule or a product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmaterializeResult {
    pub budgets: Vec<Budget>,
    pub recurring: Vec<Recurring>,
    pub products: Vec<Product>,
}

pub fn default_categories() -> Vec<Category> {
    fn cat(key: &str, name: &str, icon: &str, color: &str) -> Category {
        Category {
            key: key.into(),
            name: name.into(),
            icon: icon.into(),
            color: color.into(),
            is_default: true,
            uuid: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }
    vec![
        cat("food", "Еда", "food", "#FF6D00"),
        cat("transport", "Транспорт", "transport", "#1A73E8"),
        cat("entertainment", "Развлечения", "entertainment", "#D93025"),
        cat("shopping", "Покупки", "shopping", "#E91E63"),
        cat("bills", "Коммунальные", "bills", "#7C4DFF"),
        cat("salary", "Зарплата", "salary", "#0D904F"),
        cat("other", "Другое", "other", "#00ACC1"),
    ]
}
