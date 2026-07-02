use crate::data::*;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
    data_dir: PathBuf,
}

const MONTHS_RU: [&str; 12] = [
    "Январь",
    "Февраль",
    "Март",
    "Апрель",
    "Май",
    "Июнь",
    "Июль",
    "Август",
    "Сентябрь",
    "Октябрь",
    "Ноябрь",
    "Декабрь",
];

fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Keep an imported uuid, or mint one for legacy rows that predate uuids.
/// (The unique uuid indexes already exist during import, so empty strings
/// must be replaced before insertion, not backfilled afterwards.)
fn uuid_or_new(uuid: &str) -> String {
    if uuid.is_empty() {
        new_uuid()
    } else {
        uuid.to_string()
    }
}

/// Current instant as ISO8601 UTC — metadata timestamps only (created_at /
/// updated_at / deleted_at). Business dates are device-local calendar strings
/// and are never derived on the backend.
fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// "YYYY-MM" month key of a validated "YYYY-MM-DD" business date.
fn period_of(date: &str) -> &str {
    &date[..7]
}

/// Russian display name for a month sub-budget, e.g. "2026-07" → "Июль 2026".
fn month_label(period: &str) -> String {
    let year = &period[..4];
    let m: usize = period[5..7].parse().unwrap_or(1);
    format!("{} {}", MONTHS_RU[m.saturating_sub(1).min(11)], year)
}

/// Whether a budget name matches the legacy seed pattern "Месяц YYYY"
/// (e.g. "Июль 2026") — used to rename flat month-named budgets during
/// hierarchy normalization so the tree doesn't read "Июль 2026 → Июль 2026".
fn is_month_name(name: &str) -> bool {
    match name.rsplit_once(' ') {
        Some((month, year)) => {
            MONTHS_RU.contains(&month)
                && year.len() == 4
                && year.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

impl Database {
    pub fn open(data_dir: &PathBuf) -> Result<Self, String> {
        fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let db_path = data_dir.join("plyet.db");
        let json_path = data_dir.join("budgets.json");
        let db_existed = db_path.exists();
        let needs_migration = !db_existed && json_path.exists();

        let mut conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| e.to_string())?;

        // One-off safety copy before the destructive v6 hierarchy migration.
        if db_existed {
            let version: i64 = conn
                .query_row(
                    "SELECT COALESCE((SELECT CAST(value AS INTEGER) FROM meta WHERE key='schema_version'), 0)",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if version > 0 && version < 6 {
                let backup_dir = data_dir.join("backups");
                let _ = fs::create_dir_all(&backup_dir);
                let _ = fs::copy(&db_path, backup_dir.join("plyet-pre-v6.db"));
            }
        }

        Self::create_tables(&mut conn)?;

        // Auto-backup: keep last 7 daily backups
        Self::auto_backup(data_dir, &db_path);

        let db = Database {
            conn: Mutex::new(conn),
            data_dir: data_dir.to_path_buf(),
        };

        if needs_migration {
            db.migrate_from_json(&json_path)?;
            // Rename old file
            let _ = fs::rename(&json_path, data_dir.join("budgets.json.bak"));
        }

        // Ensure default categories, a root budget and an owner member exist
        db.ensure_defaults()?;

        Ok(db)
    }

    fn create_tables(conn: &mut Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS budgets (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                budget_limit INTEGER NOT NULL DEFAULT 0,
                icon TEXT NOT NULL DEFAULT 'wallet'
            );

            CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY,
                budget_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                amount INTEGER NOT NULL,
                category TEXT NOT NULL DEFAULT 'other',
                date TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                link TEXT NOT NULL DEFAULT '',
                item_type TEXT NOT NULL DEFAULT 'expense',
                completed INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (budget_id) REFERENCES budgets(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS categories (
                key TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                icon TEXT NOT NULL,
                color TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| e.to_string())?;

        // Schema version tracking
        let version: i64 = conn
            .query_row(
                "SELECT COALESCE((SELECT CAST(value AS INTEGER) FROM meta WHERE key='schema_version'), 0)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if version < 1 {
            conn.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '1')",
                [],
            )
            .map_err(|e| e.to_string())?;
        }

        if version < 2 {
            // Migrate amounts from whole units to kopecks (x100)
            conn.execute("UPDATE items SET amount = amount * 100", [])
                .map_err(|e| e.to_string())?;
            conn.execute("UPDATE budgets SET budget_limit = budget_limit * 100", [])
                .map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '2')",
                [],
            )
            .map_err(|e| e.to_string())?;
        }

        if version < 3 {
            // Recurring rules: definitions stored here, future occurrences are
            // computed virtually on the frontend and materialized on payment.
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS recurring (
                    id INTEGER PRIMARY KEY,
                    budget_id INTEGER NOT NULL,
                    name TEXT NOT NULL,
                    amount INTEGER NOT NULL,
                    category TEXT NOT NULL DEFAULT 'other',
                    item_type TEXT NOT NULL DEFAULT 'expense',
                    freq TEXT NOT NULL DEFAULT 'monthly',
                    anchor_day INTEGER NOT NULL DEFAULT 1,
                    start_date TEXT NOT NULL,
                    end_date TEXT,
                    horizon INTEGER NOT NULL DEFAULT 1,
                    last_paid_date TEXT,
                    description TEXT NOT NULL DEFAULT '',
                    link TEXT NOT NULL DEFAULT '',
                    FOREIGN KEY (budget_id) REFERENCES budgets(id) ON DELETE CASCADE
                );
                ",
            )
            .map_err(|e| e.to_string())?;

            // Link a materialized item back to the rule/product that spawned it.
            if !Self::column_exists(conn, "items", "source_kind") {
                conn.execute(
                    "ALTER TABLE items ADD COLUMN source_kind TEXT NOT NULL DEFAULT ''",
                    [],
                )
                .map_err(|e| e.to_string())?;
            }
            if !Self::column_exists(conn, "items", "source_id") {
                conn.execute(
                    "ALTER TABLE items ADD COLUMN source_id INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| e.to_string())?;
            }

            conn.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '3')",
                [],
            )
            .map_err(|e| e.to_string())?;
        }

        if version < 4 {
            // Complex products: deposits and loans. Their payment schedules are
            // computed virtually on the frontend and materialized on payment,
            // mirroring the recurring-rule architecture.
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS products (
                    id INTEGER PRIMARY KEY,
                    budget_id INTEGER NOT NULL,
                    kind TEXT NOT NULL DEFAULT 'loan',
                    name TEXT NOT NULL,
                    principal INTEGER NOT NULL DEFAULT 0,
                    annual_rate_bps INTEGER NOT NULL DEFAULT 0,
                    term_months INTEGER NOT NULL DEFAULT 1,
                    start_date TEXT NOT NULL,
                    payment_model TEXT NOT NULL DEFAULT 'annuity',
                    early_rate_bps INTEGER,
                    horizon INTEGER NOT NULL DEFAULT 1,
                    category TEXT NOT NULL DEFAULT 'other',
                    status TEXT NOT NULL DEFAULT 'active',
                    principal_paid INTEGER NOT NULL DEFAULT 0,
                    payments_made INTEGER NOT NULL DEFAULT 0,
                    manual_payment INTEGER,
                    description TEXT NOT NULL DEFAULT '',
                    link TEXT NOT NULL DEFAULT '',
                    FOREIGN KEY (budget_id) REFERENCES budgets(id) ON DELETE CASCADE
                );
                ",
            )
            .map_err(|e| e.to_string())?;

            conn.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '4')",
                [],
            )
            .map_err(|e| e.to_string())?;
        }

        if version < 5 {
            // Undoable materialization: record how much a materialized item
            // advanced its source so un-completing can roll it back exactly.
            // `source_principal` = principal_paid delta (loan/mortgage), and
            // `source_payment` = 1 if it counted as a scheduled payment.
            if !Self::column_exists(conn, "items", "source_principal") {
                conn.execute(
                    "ALTER TABLE items ADD COLUMN source_principal INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| e.to_string())?;
            }
            if !Self::column_exists(conn, "items", "source_payment") {
                conn.execute(
                    "ALTER TABLE items ADD COLUMN source_payment INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| e.to_string())?;
            }
            // Mortgage down payment (первоначальный взнос); 0 for other products.
            if !Self::column_exists(conn, "products", "down_payment") {
                conn.execute(
                    "ALTER TABLE products ADD COLUMN down_payment INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| e.to_string())?;
            }

            conn.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '5')",
                [],
            )
            .map_err(|e| e.to_string())?;
        }

        if version < 6 {
            // Budget hierarchy + sync foundation, in one transaction:
            // - budgets become a two-level tree (root → month/custom children),
            //   existing flat budgets turn into roots and their items are
            //   redistributed into auto-created month sub-budgets by date;
            // - every entity gets a stable uuid, created_at/updated_at (UTC)
            //   and a soft-delete tombstone (deleted_at);
            // - items get author_id and a members table appears — the
            //   foundation for cloud sync and multi-user attribution.
            let tx = conn.transaction().map_err(|e| e.to_string())?;

            for (table, col, ddl) in [
                (
                    "budgets",
                    "parent_id",
                    "ALTER TABLE budgets ADD COLUMN parent_id INTEGER",
                ),
                (
                    "budgets",
                    "kind",
                    "ALTER TABLE budgets ADD COLUMN kind TEXT NOT NULL DEFAULT 'root'",
                ),
                (
                    "budgets",
                    "period",
                    "ALTER TABLE budgets ADD COLUMN period TEXT",
                ),
                (
                    "budgets",
                    "reflect_in_months",
                    "ALTER TABLE budgets ADD COLUMN reflect_in_months INTEGER NOT NULL DEFAULT 0",
                ),
                (
                    "items",
                    "author_id",
                    "ALTER TABLE items ADD COLUMN author_id TEXT",
                ),
            ] {
                if !Self::column_exists(&tx, table, col) {
                    tx.execute(ddl, []).map_err(|e| e.to_string())?;
                }
            }

            for table in ["budgets", "items", "recurring", "products", "categories"] {
                for (col, ddl_type) in [
                    ("uuid", "TEXT NOT NULL DEFAULT ''"),
                    ("created_at", "TEXT NOT NULL DEFAULT ''"),
                    ("updated_at", "TEXT NOT NULL DEFAULT ''"),
                    ("deleted_at", "TEXT"),
                ] {
                    if !Self::column_exists(&tx, table, col) {
                        tx.execute(
                            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, col, ddl_type),
                            [],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
            }

            tx.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS members (
                    id INTEGER PRIMARY KEY,
                    uuid TEXT NOT NULL DEFAULT '',
                    name TEXT NOT NULL,
                    role TEXT NOT NULL DEFAULT 'owner',
                    created_at TEXT NOT NULL DEFAULT '',
                    updated_at TEXT NOT NULL DEFAULT '',
                    deleted_at TEXT
                );
                ",
            )
            .map_err(|e| e.to_string())?;

            // Backfill uuids/timestamps, turn flat budgets into roots and
            // redistribute their items into month sub-budgets.
            Self::normalize_hierarchy(&tx, true)?;

            tx.execute_batch(
                "
                CREATE UNIQUE INDEX IF NOT EXISTS idx_budgets_uuid ON budgets(uuid);
                CREATE UNIQUE INDEX IF NOT EXISTS idx_items_uuid ON items(uuid);
                CREATE UNIQUE INDEX IF NOT EXISTS idx_recurring_uuid ON recurring(uuid);
                CREATE UNIQUE INDEX IF NOT EXISTS idx_products_uuid ON products(uuid);
                CREATE UNIQUE INDEX IF NOT EXISTS idx_members_uuid ON members(uuid);
                CREATE UNIQUE INDEX IF NOT EXISTS idx_budgets_parent_period
                    ON budgets(parent_id, period) WHERE period IS NOT NULL AND deleted_at IS NULL;
                CREATE INDEX IF NOT EXISTS idx_items_budget ON items(budget_id);
                ",
            )
            .map_err(|e| e.to_string())?;

            tx.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '6')",
                [],
            )
            .map_err(|e| e.to_string())?;

            tx.commit().map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Whether `table` has a column named `col`. Table/column names are fixed
    /// literals at the call sites, so interpolation here is not injectable.
    fn column_exists(conn: &Connection, table: &str, col: &str) -> bool {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1",
                table
            ),
            params![col],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false)
    }

    /// Bring pre-hierarchy rows up to the v6 shape. Shared by the v5→v6 DB
    /// migration and by JSON import (both legacy files and, harmlessly
    /// idempotently, v6 envelopes):
    /// 1. backfill missing uuids and metadata timestamps;
    /// 2. default budget `kind` where imports left it empty;
    /// 3. optionally rename flat roots that carry the old seeded month name;
    /// 4. move items owned directly by a root into that root's month
    ///    sub-budgets by business date (auto-creating the months).
    fn normalize_hierarchy(conn: &Connection, rename_seed_names: bool) -> Result<(), String> {
        let now = now_iso();

        // 1a. uuids for id-keyed tables (row by row: each needs a distinct value)
        for table in ["budgets", "items", "recurring", "products", "members"] {
            let ids: Vec<i64> = {
                let mut stmt = conn
                    .prepare(&format!(
                        "SELECT id FROM {} WHERE uuid IS NULL OR uuid = ''",
                        table
                    ))
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |r| r.get(0))
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<i64>, _>>()
                    .map_err(|e| e.to_string())?;
                rows
            };
            for id in ids {
                conn.execute(
                    &format!("UPDATE {} SET uuid = ?1 WHERE id = ?2", table),
                    params![new_uuid(), id],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        // 1b. uuids for categories (keyed by `key`)
        let keys: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT key FROM categories WHERE uuid IS NULL OR uuid = ''")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| r.get(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<String>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        };
        for key in keys {
            conn.execute(
                "UPDATE categories SET uuid = ?1 WHERE key = ?2",
                params![new_uuid(), key],
            )
            .map_err(|e| e.to_string())?;
        }
        // 1c. timestamps (one shared migration instant — real creation times are unknown)
        for table in [
            "budgets",
            "items",
            "recurring",
            "products",
            "categories",
            "members",
        ] {
            conn.execute(
                &format!(
                    "UPDATE {} SET created_at = ?1 WHERE created_at IS NULL OR created_at = ''",
                    table
                ),
                params![now],
            )
            .map_err(|e| e.to_string())?;
            conn.execute(
                &format!(
                    "UPDATE {} SET updated_at = ?1 WHERE updated_at IS NULL OR updated_at = ''",
                    table
                ),
                params![now],
            )
            .map_err(|e| e.to_string())?;
        }

        // 2. default `kind` for rows imported without one
        conn.execute(
            "UPDATE budgets SET kind = 'root' WHERE (kind IS NULL OR kind = '') AND parent_id IS NULL",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE budgets SET kind = 'custom' WHERE (kind IS NULL OR kind = '') AND parent_id IS NOT NULL",
            [],
        )
        .map_err(|e| e.to_string())?;

        // 3. rename legacy month-named flat budgets so the tree isn't "Июль 2026 → Июль 2026"
        if rename_seed_names {
            let named: Vec<(i64, String)> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, name FROM budgets
                         WHERE parent_id IS NULL AND kind = 'root' AND deleted_at IS NULL",
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<(i64, String)>, _>>()
                    .map_err(|e| e.to_string())?;
                rows
            };
            for (id, name) in named {
                if is_month_name(&name) {
                    conn.execute(
                        "UPDATE budgets SET name = 'Мой бюджет', updated_at = ?1 WHERE id = ?2",
                        params![now, id],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }

        // 4. redistribute items owned directly by roots into month sub-budgets
        let roots: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT id FROM budgets WHERE parent_id IS NULL AND deleted_at IS NULL")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| r.get(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<i64>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        };
        for root in roots {
            let periods: Vec<String> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT DISTINCT substr(date, 1, 7) FROM items
                         WHERE budget_id = ?1 AND deleted_at IS NULL",
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![root], |r| r.get(0))
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<String>, _>>()
                    .map_err(|e| e.to_string())?;
                rows
            };
            for period in periods {
                let month_id = Self::ensure_month_budget(conn, root, &period)?;
                conn.execute(
                    "UPDATE items SET budget_id = ?1, updated_at = ?2
                     WHERE budget_id = ?3 AND substr(date, 1, 7) = ?4 AND deleted_at IS NULL",
                    params![month_id, now, root, period],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    fn migrate_from_json(&self, json_path: &PathBuf) -> Result<(), String> {
        let json = fs::read_to_string(json_path).map_err(|e| e.to_string())?;
        let data: AppData = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;

        let tx = conn.transaction().map_err(|e| e.to_string())?;

        // Import categories
        for cat in &data.categories {
            tx.execute(
                "INSERT OR IGNORE INTO categories (key, name, icon, color, is_default) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![cat.key, cat.name, cat.icon, cat.color, cat.is_default as i32],
            ).map_err(|e| e.to_string())?;
        }

        // Import budgets and items. The legacy budgets.json predates the
        // kopeck migration (schema v2), so its amounts are whole rubles and
        // must be scaled by 100 to match the current kopeck-based schema.
        for budget in &data.budgets {
            tx.execute(
                "INSERT INTO budgets (id, name, budget_limit, icon) VALUES (?1, ?2, ?3, ?4)",
                params![
                    budget.id as i64,
                    budget.name,
                    budget.limit * 100,
                    budget.icon
                ],
            )
            .map_err(|e| e.to_string())?;

            for item in &budget.items {
                let item_type_str = match item.item_type {
                    ItemType::Income => "income",
                    ItemType::Expense => "expense",
                };
                tx.execute(
                    "INSERT INTO items (id, budget_id, name, amount, category, date, description, link, item_type, completed)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        item.id as i64,
                        budget.id as i64,
                        item.name,
                        item.amount * 100,
                        item.category,
                        item.date,
                        item.description,
                        item.link,
                        item_type_str,
                        item.completed as i32,
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        // budgets.json predates the hierarchy: normalize it the same way as
        // the v5→v6 DB migration (uuids, roots, month redistribution).
        Self::normalize_hierarchy(&tx, true)?;

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn ensure_defaults(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = now_iso();

        // On first run (no live categories), seed all defaults. ON CONFLICT
        // resurrects a tombstoned row instead of failing the unique key.
        let cat_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM categories WHERE deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if cat_count == 0 {
            let defaults = default_categories();
            for cat in &defaults {
                conn.execute(
                    "INSERT INTO categories (key, name, icon, color, is_default, uuid, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?6)
                     ON CONFLICT(key) DO UPDATE SET deleted_at = NULL, updated_at = ?6",
                    params![cat.key, cat.name, cat.icon, cat.color, new_uuid(), now],
                )
                .map_err(|e| e.to_string())?;
            }
        } else {
            // Always ensure "other" exists (mandatory fallback)
            conn.execute(
                "INSERT INTO categories (key, name, icon, color, is_default, uuid, created_at, updated_at)
                 VALUES ('other', 'Другое', 'other', '#00ACC1', 1, ?1, ?2, ?2)
                 ON CONFLICT(key) DO UPDATE SET deleted_at = NULL, updated_at = ?2",
                params![new_uuid(), now],
            )
            .map_err(|e| e.to_string())?;
        }

        // Seed a root budget if none exist. The current-month sub-budget is
        // created lazily by the frontend ("current month" is device-local).
        let root_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM budgets WHERE parent_id IS NULL AND deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if root_count == 0 {
            conn.execute(
                "INSERT INTO budgets (name, budget_limit, icon, kind, uuid, created_at, updated_at)
                 VALUES ('Мой бюджет', 0, 'wallet', 'root', ?1, ?2, ?2)",
                params![new_uuid(), now],
            )
            .map_err(|e| e.to_string())?;
        }

        // Seed the owner member (future multi-user attribution points here).
        let member_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM members WHERE deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if member_count == 0 {
            conn.execute(
                "INSERT INTO members (uuid, name, role, created_at, updated_at)
                 VALUES (?1, 'Я', 'owner', ?2, ?2)",
                params![new_uuid(), now],
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    // ── Hierarchy helpers ───────────────────────────────────────

    /// Root budget id of any budget (itself when it has no parent).
    fn resolve_root(conn: &Connection, budget_id: i64) -> Result<i64, String> {
        conn.query_row(
            "SELECT COALESCE(parent_id, id) FROM budgets WHERE id = ?1 AND deleted_at IS NULL",
            params![budget_id],
            |r| r.get(0),
        )
        .map_err(|_| "Budget not found".to_string())
    }

    /// Find-or-create the month sub-budget of `root_id` for `period`
    /// ("YYYY-MM"). New months inherit the root's icon and its limit (the
    /// root limit acts as the default monthly limit). Idempotent; the partial
    /// unique index on (parent_id, period) backstops races.
    fn ensure_month_budget(conn: &Connection, root_id: i64, period: &str) -> Result<i64, String> {
        if let Ok(id) = conn.query_row(
            "SELECT id FROM budgets WHERE parent_id = ?1 AND period = ?2 AND deleted_at IS NULL",
            params![root_id, period],
            |r| r.get::<_, i64>(0),
        ) {
            return Ok(id);
        }
        let (limit, icon): (i64, String) = conn
            .query_row(
                "SELECT budget_limit, icon FROM budgets
                 WHERE id = ?1 AND parent_id IS NULL AND deleted_at IS NULL",
                params![root_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| "Root budget not found".to_string())?;
        let now = now_iso();
        conn.execute(
            "INSERT INTO budgets (name, budget_limit, icon, parent_id, kind, period, uuid, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'month', ?5, ?6, ?7, ?7)",
            params![month_label(period), limit, icon, root_id, period, new_uuid(), now],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    /// Which budget an item dated `date` belongs to, given the budget the user
    /// was looking at: custom sub-budgets keep their items; everything else
    /// routes by business date into the root's month sub-budget (auto-created).
    fn route_item_budget(conn: &Connection, requested: i64, date: &str) -> Result<i64, String> {
        let (kind, parent_id): (String, Option<i64>) = conn
            .query_row(
                "SELECT kind, parent_id FROM budgets WHERE id = ?1 AND deleted_at IS NULL",
                params![requested],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| "Budget not found".to_string())?;
        if kind == "custom" {
            return Ok(requested);
        }
        let root = parent_id.unwrap_or(requested);
        Self::ensure_month_budget(conn, root, period_of(date))
    }

    // ── Queries ─────────────────────────────────────────────────

    pub fn load_all(&self) -> Result<AppData, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        Self::query_app_data(&conn, false)
    }

    /// Full snapshot for export: includes tombstoned rows so a future sync /
    /// restore can propagate deletions.
    fn load_all_full(&self) -> Result<AppData, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        Self::query_app_data(&conn, true)
    }

    fn query_app_data(conn: &Connection, include_deleted: bool) -> Result<AppData, String> {
        Ok(AppData {
            budgets: Self::query_budgets(conn, include_deleted)?,
            categories: Self::query_categories(conn, include_deleted)?,
            recurring: Self::query_recurring_full(conn, include_deleted)?,
            products: Self::query_products_full(conn, include_deleted)?,
            members: Self::query_members(conn, include_deleted)?,
            next_id: 0, // Not used with SQLite
        })
    }

    fn live_clause(include_deleted: bool) -> &'static str {
        if include_deleted {
            "1=1"
        } else {
            "deleted_at IS NULL"
        }
    }

    /// All budgets with their items, in two queries (no N+1).
    fn query_budgets(conn: &Connection, include_deleted: bool) -> Result<Vec<Budget>, String> {
        let live = Self::live_clause(include_deleted);

        let mut items_by_budget: HashMap<i64, Vec<Item>> = HashMap::new();
        {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT budget_id, id, name, amount, category, date, description, link,
                            item_type, completed, source_kind, source_id,
                            uuid, created_at, updated_at, deleted_at, author_id
                     FROM items WHERE {} ORDER BY budget_id, id",
                    live
                ))
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    let budget_id: i64 = row.get(0)?;
                    let item_type_str: String = row.get(8)?;
                    Ok((
                        budget_id,
                        Item {
                            id: row.get::<_, i64>(1)? as u64,
                            name: row.get(2)?,
                            amount: row.get(3)?,
                            category: row.get(4)?,
                            date: row.get(5)?,
                            description: row.get(6)?,
                            link: row.get(7)?,
                            item_type: if item_type_str == "income" {
                                ItemType::Income
                            } else {
                                ItemType::Expense
                            },
                            completed: row.get::<_, i32>(9)? != 0,
                            source_kind: row.get(10)?,
                            source_id: row.get::<_, i64>(11)? as u64,
                            uuid: row.get(12)?,
                            created_at: row.get(13)?,
                            updated_at: row.get(14)?,
                            deleted_at: row.get(15)?,
                            author_id: row.get(16)?,
                        },
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to read item: {}", e))?;
            for (budget_id, item) in rows {
                items_by_budget.entry(budget_id).or_default().push(item);
            }
        }

        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, name, budget_limit, icon, parent_id, kind, period,
                        reflect_in_months, uuid, created_at, updated_at, deleted_at
                 FROM budgets WHERE {} ORDER BY id",
                live
            ))
            .map_err(|e| e.to_string())?;
        let budgets = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                Ok(Budget {
                    id: id as u64,
                    name: row.get(1)?,
                    limit: row.get(2)?,
                    icon: row.get(3)?,
                    items: Vec::new(),
                    parent_id: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                    kind: row.get(5)?,
                    period: row.get(6)?,
                    reflect_in_months: row.get::<_, i32>(7)? != 0,
                    uuid: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    deleted_at: row.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read budget: {}", e))?;

        let budgets = budgets
            .into_iter()
            .map(|mut b| {
                b.items = items_by_budget.remove(&(b.id as i64)).unwrap_or_default();
                b
            })
            .collect();
        Ok(budgets)
    }

    /// The live budget list — the standard payload returned by every mutation
    /// (routing can touch budgets other than the one on screen).
    fn query_live_budgets(conn: &Connection) -> Result<Vec<Budget>, String> {
        Self::query_budgets(conn, false)
    }

    fn query_products_full(
        conn: &Connection,
        include_deleted: bool,
    ) -> Result<Vec<Product>, String> {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, budget_id, kind, name, principal, annual_rate_bps, term_months,
                        start_date, payment_model, early_rate_bps, horizon, category, status,
                        principal_paid, payments_made, manual_payment, description, link, down_payment,
                        uuid, created_at, updated_at, deleted_at
                 FROM products WHERE {} ORDER BY id",
                Self::live_clause(include_deleted)
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Product {
                    id: row.get::<_, i64>(0)? as u64,
                    budget_id: row.get::<_, i64>(1)? as u64,
                    kind: row.get(2)?,
                    name: row.get(3)?,
                    principal: row.get(4)?,
                    annual_rate_bps: row.get(5)?,
                    term_months: row.get(6)?,
                    start_date: row.get(7)?,
                    payment_model: row.get(8)?,
                    early_rate_bps: row.get(9)?,
                    horizon: row.get(10)?,
                    category: row.get(11)?,
                    status: row.get(12)?,
                    principal_paid: row.get(13)?,
                    payments_made: row.get(14)?,
                    manual_payment: row.get(15)?,
                    description: row.get(16)?,
                    link: row.get(17)?,
                    down_payment: row.get(18)?,
                    uuid: row.get(19)?,
                    created_at: row.get(20)?,
                    updated_at: row.get(21)?,
                    deleted_at: row.get(22)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read products: {}", e))?;
        Ok(rows)
    }

    fn query_products(conn: &Connection) -> Result<Vec<Product>, String> {
        Self::query_products_full(conn, false)
    }

    fn query_recurring_full(
        conn: &Connection,
        include_deleted: bool,
    ) -> Result<Vec<Recurring>, String> {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, budget_id, name, amount, category, item_type, freq, anchor_day,
                        start_date, end_date, horizon, last_paid_date, description, link,
                        uuid, created_at, updated_at, deleted_at
                 FROM recurring WHERE {} ORDER BY id",
                Self::live_clause(include_deleted)
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let item_type_str: String = row.get(5)?;
                Ok(Recurring {
                    id: row.get::<_, i64>(0)? as u64,
                    budget_id: row.get::<_, i64>(1)? as u64,
                    name: row.get(2)?,
                    amount: row.get(3)?,
                    category: row.get(4)?,
                    item_type: if item_type_str == "income" {
                        ItemType::Income
                    } else {
                        ItemType::Expense
                    },
                    freq: row.get(6)?,
                    anchor_day: row.get(7)?,
                    start_date: row.get(8)?,
                    end_date: row.get(9)?,
                    horizon: row.get(10)?,
                    last_paid_date: row.get(11)?,
                    description: row.get(12)?,
                    link: row.get(13)?,
                    uuid: row.get(14)?,
                    created_at: row.get(15)?,
                    updated_at: row.get(16)?,
                    deleted_at: row.get(17)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read recurring: {}", e))?;
        Ok(rows)
    }

    fn query_recurring(conn: &Connection) -> Result<Vec<Recurring>, String> {
        Self::query_recurring_full(conn, false)
    }

    fn query_categories(conn: &Connection, include_deleted: bool) -> Result<Vec<Category>, String> {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT key, name, icon, color, is_default, uuid, created_at, updated_at, deleted_at
                 FROM categories WHERE {} ORDER BY is_default DESC, key",
                Self::live_clause(include_deleted)
            ))
            .map_err(|e| e.to_string())?;
        let cats = stmt
            .query_map([], |row| {
                Ok(Category {
                    key: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    color: row.get(3)?,
                    is_default: row.get::<_, i32>(4)? != 0,
                    uuid: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    deleted_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read category: {}", e))?;
        Ok(cats)
    }

    fn query_live_categories(conn: &Connection) -> Result<Vec<Category>, String> {
        Self::query_categories(conn, false)
    }

    fn query_members(conn: &Connection, include_deleted: bool) -> Result<Vec<Member>, String> {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, uuid, name, role, created_at, updated_at, deleted_at
                 FROM members WHERE {} ORDER BY id",
                Self::live_clause(include_deleted)
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Member {
                    id: row.get::<_, i64>(0)? as u64,
                    uuid: row.get(1)?,
                    name: row.get(2)?,
                    role: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    deleted_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read member: {}", e))?;
        Ok(rows)
    }

    // ── Item mutations ──────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    fn insert_item(
        conn: &Connection,
        budget_id: i64,
        name: &str,
        amount: i64,
        category: &str,
        date: &str,
        description: &str,
        link: &str,
        item_type: &str,
        completed: bool,
        source_kind: &str,
        source_id: i64,
        source_principal: i64,
        source_payment: i64,
    ) -> Result<i64, String> {
        let now = now_iso();
        conn.execute(
            "INSERT INTO items
                (budget_id, name, amount, category, date, description, link, item_type, completed,
                 source_kind, source_id, source_principal, source_payment, uuid, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
            params![
                budget_id, name, amount, category, date, description, link, item_type,
                completed as i32, source_kind, source_id, source_principal, source_payment,
                new_uuid(), now
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_item(
        &self,
        budget_id: u64,
        name: &str,
        amount: i64,
        category: &str,
        date: &str,
        description: &str,
        link: &str,
        item_type: &str,
    ) -> Result<Vec<Budget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let target = Self::route_item_budget(&conn, budget_id as i64, date)?;
        Self::insert_item(
            &conn,
            target,
            name,
            amount,
            category,
            date,
            description,
            link,
            item_type,
            false,
            "",
            0,
            0,
            0,
        )?;
        Self::query_live_budgets(&conn)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_item(
        &self,
        item_id: u64,
        name: &str,
        amount: i64,
        category: &str,
        date: &str,
        description: &str,
        link: &str,
        item_type: &str,
    ) -> Result<Vec<Budget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let current_budget: i64 = conn
            .query_row(
                "SELECT budget_id FROM items WHERE id = ?1 AND deleted_at IS NULL",
                params![item_id as i64],
                |r| r.get(0),
            )
            .map_err(|_| "Item not found".to_string())?;

        // Items in a custom sub-budget stay put; month items follow their date.
        let target = Self::route_item_budget(&conn, current_budget, date)?;

        conn.execute(
            "UPDATE items SET name=?1, amount=?2, category=?3, date=?4, description=?5, link=?6,
                    item_type=?7, budget_id=?8, updated_at=?9
             WHERE id=?10",
            params![
                name,
                amount,
                category,
                date,
                description,
                link,
                item_type,
                target,
                now_iso(),
                item_id as i64
            ],
        )
        .map_err(|e| e.to_string())?;
        Self::query_live_budgets(&conn)
    }

    pub fn delete_item(&self, item_id: u64) -> Result<Vec<Budget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = now_iso();
        conn.execute(
            "UPDATE items SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, item_id as i64],
        )
        .map_err(|e| e.to_string())?;
        Self::query_live_budgets(&conn)
    }

    pub fn toggle_completed(&self, item_id: u64) -> Result<Vec<Budget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rows = conn
            .execute(
                "UPDATE items SET completed = 1 - completed, updated_at = ?1
                 WHERE id = ?2 AND deleted_at IS NULL",
                params![now_iso(), item_id as i64],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Item not found".into());
        }
        Self::query_live_budgets(&conn)
    }

    /// Undo a materialized occurrence: tombstone the real item and roll its
    /// source back so the virtual occurrence reappears. For recurring rules
    /// this recomputes `last_paid_date` from the remaining materialized items
    /// (searched across the whole root subtree — occurrences of one rule live
    /// in different month sub-budgets); for products it reverses the
    /// `payments_made`/`principal_paid` deltas and re-activates the product.
    pub fn unmaterialize_item(&self, item_id: u64) -> Result<UnmaterializeResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let (budget_id, source_kind, source_id, source_principal, source_payment): (
            i64,
            String,
            i64,
            i64,
            i64,
        ) = conn
            .query_row(
                "SELECT budget_id, source_kind, source_id, source_principal, source_payment
                 FROM items WHERE id = ?1 AND deleted_at IS NULL",
                params![item_id as i64],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|_| "Item not found".to_string())?;

        if source_kind.is_empty() {
            return Err("Item is not linked to a recurring rule or product".into());
        }

        let now = now_iso();
        conn.execute(
            "UPDATE items SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, item_id as i64],
        )
        .map_err(|e| e.to_string())?;

        if source_kind == "recurring" {
            // Roll the rule back to its most recent still-materialized
            // occurrence anywhere under the same root.
            let root = Self::resolve_root(&conn, budget_id)?;
            let last: Option<String> = conn
                .query_row(
                    "SELECT MAX(date) FROM items
                     WHERE source_kind = 'recurring' AND source_id = ?1 AND deleted_at IS NULL
                       AND budget_id IN (SELECT id FROM budgets WHERE id = ?2 OR parent_id = ?2)",
                    params![source_id, root],
                    |r| r.get(0),
                )
                .unwrap_or(None);
            conn.execute(
                "UPDATE recurring SET last_paid_date = ?1, updated_at = ?2 WHERE id = ?3",
                params![last, now, source_id],
            )
            .map_err(|e| e.to_string())?;
        } else {
            // deposit | loan | mortgage: reverse this item's contribution and
            // re-activate (removing any payment means it's no longer fully done).
            conn.execute(
                "UPDATE products
                 SET payments_made = MAX(0, payments_made - ?1),
                     principal_paid = MAX(0, principal_paid - ?2),
                     status = 'active',
                     updated_at = ?3
                 WHERE id = ?4",
                params![source_payment, source_principal, now, source_id],
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(UnmaterializeResult {
            budgets: Self::query_live_budgets(&conn)?,
            recurring: Self::query_recurring(&conn)?,
            products: Self::query_products(&conn)?,
        })
    }

    // ── Recurring CRUD ──────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn add_recurring(
        &self,
        budget_id: u64,
        name: &str,
        amount: i64,
        category: &str,
        item_type: &str,
        freq: &str,
        anchor_day: i64,
        start_date: &str,
        end_date: Option<&str>,
        horizon: i64,
        description: &str,
        link: &str,
    ) -> Result<Vec<Recurring>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        // Rules always attach to the root: their payments spread across months.
        let root = Self::resolve_root(&conn, budget_id as i64)?;
        let now = now_iso();

        conn.execute(
            "INSERT INTO recurring
                (budget_id, name, amount, category, item_type, freq, anchor_day,
                 start_date, end_date, horizon, last_paid_date, description, link,
                 uuid, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11, ?12, ?13, ?14, ?14)",
            params![
                root,
                name,
                amount,
                category,
                item_type,
                freq,
                anchor_day,
                start_date,
                end_date,
                horizon,
                description,
                link,
                new_uuid(),
                now
            ],
        )
        .map_err(|e| e.to_string())?;

        Self::query_recurring(&conn)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_recurring(
        &self,
        id: u64,
        name: &str,
        amount: i64,
        category: &str,
        item_type: &str,
        freq: &str,
        anchor_day: i64,
        start_date: &str,
        end_date: Option<&str>,
        horizon: i64,
        description: &str,
        link: &str,
    ) -> Result<Vec<Recurring>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        // last_paid_date is intentionally left untouched: changing the amount or
        // schedule only affects future (still-virtual) occurrences, never the
        // already-materialized history.
        let rows = conn
            .execute(
                "UPDATE recurring SET name=?1, amount=?2, category=?3, item_type=?4, freq=?5,
                    anchor_day=?6, start_date=?7, end_date=?8, horizon=?9, description=?10, link=?11,
                    updated_at=?12
                 WHERE id=?13 AND deleted_at IS NULL",
                params![
                    name, amount, category, item_type, freq, anchor_day,
                    start_date, end_date, horizon, description, link, now_iso(), id as i64
                ],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Recurring not found".into());
        }
        Self::query_recurring(&conn)
    }

    pub fn delete_recurring(&self, id: u64) -> Result<Vec<Recurring>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE recurring SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now_iso(), id as i64],
        )
        .map_err(|e| e.to_string())?;
        Self::query_recurring(&conn)
    }

    /// Mark a virtual recurring occurrence on `date` as paid: insert a real
    /// completed `Item` (built from the rule's current fields) into the month
    /// sub-budget matching the payment date, and advance `last_paid_date`.
    pub fn materialize_recurring(&self, id: u64, date: &str) -> Result<MaterializeResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let (budget_id, name, amount, category, item_type, description, link): (
            i64,
            String,
            i64,
            String,
            String,
            String,
            String,
        ) = conn
            .query_row(
                "SELECT budget_id, name, amount, category, item_type, description, link
                 FROM recurring WHERE id = ?1 AND deleted_at IS NULL",
                params![id as i64],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ))
                },
            )
            .map_err(|_| "Recurring not found".to_string())?;

        let root = Self::resolve_root(&conn, budget_id)?;
        let target = Self::ensure_month_budget(&conn, root, period_of(date))?;
        Self::insert_item(
            &conn,
            target,
            &name,
            amount,
            &category,
            date,
            &description,
            &link,
            &item_type,
            true,
            "recurring",
            id as i64,
            0,
            0,
        )?;

        conn.execute(
            "UPDATE recurring SET last_paid_date = ?1, updated_at = ?2 WHERE id = ?3",
            params![date, now_iso(), id as i64],
        )
        .map_err(|e| e.to_string())?;

        Ok(MaterializeResult {
            budgets: Self::query_live_budgets(&conn)?,
            recurring: Self::query_recurring(&conn)?,
        })
    }

    // ── Product CRUD ────────────────────────────────────────────

    /// Create a deposit, loan or mortgage on the root budget. Opening a deposit
    /// debits its principal now (a real completed expense in the start month),
    /// since the money leaves the spendable balance; a mortgage likewise debits
    /// its down payment. Loans don't move cash on creation — only their
    /// scheduled payments do.
    #[allow(clippy::too_many_arguments)]
    pub fn add_product(
        &self,
        budget_id: u64,
        kind: &str,
        name: &str,
        principal: i64,
        annual_rate_bps: i64,
        term_months: i64,
        start_date: &str,
        payment_model: &str,
        early_rate_bps: Option<i64>,
        horizon: i64,
        category: &str,
        down_payment: i64,
        description: &str,
        link: &str,
    ) -> Result<ProductResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let root = Self::resolve_root(&conn, budget_id as i64)?;
        let now = now_iso();

        conn.execute(
            "INSERT INTO products
                (budget_id, kind, name, principal, annual_rate_bps, term_months, start_date,
                 payment_model, early_rate_bps, horizon, category, status, principal_paid,
                 payments_made, manual_payment, down_payment, description, link,
                 uuid, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'active', 0, 0, NULL, ?12, ?13, ?14, ?15, ?16, ?16)",
            params![
                root, kind, name, principal, annual_rate_bps, term_months, start_date,
                payment_model, early_rate_bps, horizon, category, down_payment, description, link,
                new_uuid(), now
            ],
        )
        .map_err(|e| e.to_string())?;

        let product_id = conn.last_insert_rowid();

        // Deposit: principal leaves the balance now. Mortgage: down payment does.
        let (upfront, upfront_desc) = match kind {
            "deposit" => (principal, ""),
            "mortgage" => (down_payment, "Первоначальный взнос"),
            _ => (0, ""),
        };
        if upfront > 0 {
            let target = Self::ensure_month_budget(&conn, root, period_of(start_date))?;
            Self::insert_item(
                &conn,
                target,
                name,
                upfront,
                category,
                start_date,
                upfront_desc,
                "",
                "expense",
                true,
                kind,
                product_id,
                0,
                0,
            )?;
        }

        Ok(ProductResult {
            budgets: Self::query_live_budgets(&conn)?,
            products: Self::query_products(&conn)?,
        })
    }

    /// Update editable product fields. Schedule-defining fields (principal, rate,
    /// term) are only meaningful before any payment; the frontend guards against
    /// editing them on an active product with progress.
    #[allow(clippy::too_many_arguments)]
    pub fn update_product(
        &self,
        id: u64,
        name: &str,
        principal: i64,
        annual_rate_bps: i64,
        term_months: i64,
        start_date: &str,
        payment_model: &str,
        early_rate_bps: Option<i64>,
        horizon: i64,
        category: &str,
        down_payment: i64,
        description: &str,
        link: &str,
    ) -> Result<Vec<Product>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rows = conn
            .execute(
                "UPDATE products SET name=?1, principal=?2, annual_rate_bps=?3, term_months=?4,
                    start_date=?5, payment_model=?6, early_rate_bps=?7, horizon=?8, category=?9,
                    down_payment=?10, description=?11, link=?12, updated_at=?13
                 WHERE id=?14 AND deleted_at IS NULL",
                params![
                    name,
                    principal,
                    annual_rate_bps,
                    term_months,
                    start_date,
                    payment_model,
                    early_rate_bps,
                    horizon,
                    category,
                    down_payment,
                    description,
                    link,
                    now_iso(),
                    id as i64
                ],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Product not found".into());
        }
        Self::query_products(&conn)
    }

    /// Soft-delete a product definition. Already-materialized items (real
    /// payment history) are intentionally kept.
    pub fn delete_product(&self, id: u64) -> Result<Vec<Product>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE products SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now_iso(), id as i64],
        )
        .map_err(|e| e.to_string())?;
        Self::query_products(&conn)
    }

    /// Materialize one virtual product occurrence (a loan payment or a deposit
    /// payout) into the month sub-budget matching the payment date. The
    /// schedule math lives on the frontend, which passes the computed `amount`,
    /// `principal_portion` (loan only) and whether this occurrence `closes` the
    /// product.
    #[allow(clippy::too_many_arguments)]
    pub fn materialize_product(
        &self,
        id: u64,
        date: &str,
        amount: i64,
        principal_portion: i64,
        item_type: &str,
        closes: bool,
    ) -> Result<ProductResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let (budget_id, name, category, kind): (i64, String, String, String) = conn
            .query_row(
                "SELECT budget_id, name, category, kind FROM products WHERE id = ?1 AND deleted_at IS NULL",
                params![id as i64],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|_| "Product not found".to_string())?;

        let root = Self::resolve_root(&conn, budget_id)?;
        let target = Self::ensure_month_budget(&conn, root, period_of(date))?;
        Self::insert_item(
            &conn,
            target,
            &name,
            amount,
            &category,
            date,
            "",
            "",
            item_type,
            true,
            &kind,
            id as i64,
            principal_portion,
            1,
        )?;

        let new_status = if closes { "closed" } else { "active" };
        conn.execute(
            "UPDATE products
             SET payments_made = payments_made + 1,
                 principal_paid = principal_paid + ?1,
                 status = ?2,
                 updated_at = ?3
             WHERE id = ?4",
            params![principal_portion, new_status, now_iso(), id as i64],
        )
        .map_err(|e| e.to_string())?;

        Ok(ProductResult {
            budgets: Self::query_live_budgets(&conn)?,
            products: Self::query_products(&conn)?,
        })
    }

    /// Make an extra payment toward a loan's principal: a real completed expense
    /// plus an increase to `principal_paid` (which shortens the remaining term,
    /// since the fixed payment stays the same). Closes the loan if fully repaid.
    pub fn loan_extra_payment(
        &self,
        id: u64,
        amount: i64,
        date: &str,
    ) -> Result<ProductResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let (budget_id, name, category, principal, principal_paid, kind): (
            i64,
            String,
            String,
            i64,
            i64,
            String,
        ) = conn
            .query_row(
                "SELECT budget_id, name, category, principal, principal_paid, kind
                 FROM products WHERE id = ?1 AND kind IN ('loan', 'mortgage') AND deleted_at IS NULL",
                params![id as i64],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .map_err(|_| "Loan not found".to_string())?;

        let root = Self::resolve_root(&conn, budget_id)?;
        let target = Self::ensure_month_budget(&conn, root, period_of(date))?;
        Self::insert_item(
            &conn,
            target,
            &name,
            amount,
            &category,
            date,
            "Досрочный платёж",
            "",
            "expense",
            true,
            &kind,
            id as i64,
            amount,
            0,
        )?;

        let closes = principal_paid + amount >= principal;
        let new_status = if closes { "closed" } else { "active" };
        conn.execute(
            "UPDATE products SET principal_paid = principal_paid + ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
            params![amount, new_status, now_iso(), id as i64],
        )
        .map_err(|e| e.to_string())?;

        Ok(ProductResult {
            budgets: Self::query_live_budgets(&conn)?,
            products: Self::query_products(&conn)?,
        })
    }

    /// Close a deposit early: return the body now and pay out the interest
    /// earned so far recomputed at the reduced rate (the frontend computes the
    /// total `payout`). Inserts a single income item and marks the product closed.
    pub fn close_deposit(&self, id: u64, date: &str, payout: i64) -> Result<ProductResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let (budget_id, name, category): (i64, String, String) = conn
            .query_row(
                "SELECT budget_id, name, category FROM products
                 WHERE id = ?1 AND kind = 'deposit' AND deleted_at IS NULL",
                params![id as i64],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|_| "Deposit not found".to_string())?;

        let root = Self::resolve_root(&conn, budget_id)?;
        let target = Self::ensure_month_budget(&conn, root, period_of(date))?;
        Self::insert_item(
            &conn,
            target,
            &name,
            payout,
            &category,
            date,
            "Досрочное закрытие",
            "",
            "income",
            true,
            "deposit",
            id as i64,
            0,
            0,
        )?;

        conn.execute(
            "UPDATE products SET status = 'closed', updated_at = ?1 WHERE id = ?2",
            params![now_iso(), id as i64],
        )
        .map_err(|e| e.to_string())?;

        Ok(ProductResult {
            budgets: Self::query_live_budgets(&conn)?,
            products: Self::query_products(&conn)?,
        })
    }

    // ── Budget CRUD ─────────────────────────────────────────────

    /// Create a new root budget. Its `limit` doubles as the default limit for
    /// auto-created month sub-budgets.
    pub fn create_budget(&self, name: &str, limit: i64, icon: &str) -> Result<Vec<Budget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO budgets (name, budget_limit, icon, kind, uuid, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'root', ?4, ?5, ?5)",
            params![name, limit, icon, new_uuid(), now_iso()],
        )
        .map_err(|e| e.to_string())?;
        Self::query_live_budgets(&conn)
    }

    /// Create a custom (themed) sub-budget under a root. `reflect_in_months`
    /// controls whether its items also appear in the root's month views.
    pub fn create_sub_budget(
        &self,
        root_id: u64,
        name: &str,
        limit: i64,
        icon: &str,
        reflect_in_months: bool,
    ) -> Result<Vec<Budget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let is_root: bool = conn
            .query_row(
                "SELECT parent_id IS NULL FROM budgets WHERE id = ?1 AND deleted_at IS NULL",
                params![root_id as i64],
                |r| r.get(0),
            )
            .map_err(|_| "Root budget not found".to_string())?;
        if !is_root {
            return Err("Sub-budgets can only be created under a root budget".into());
        }
        conn.execute(
            "INSERT INTO budgets (name, budget_limit, icon, parent_id, kind, reflect_in_months, uuid, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'custom', ?5, ?6, ?7, ?7)",
            params![name, limit, icon, root_id as i64, reflect_in_months as i32, new_uuid(), now_iso()],
        )
        .map_err(|e| e.to_string())?;
        Self::query_live_budgets(&conn)
    }

    pub fn update_budget(
        &self,
        budget_id: u64,
        name: &str,
        limit: i64,
        icon: &str,
        reflect_in_months: bool,
    ) -> Result<Vec<Budget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rows = conn
            .execute(
                "UPDATE budgets SET name=?1, budget_limit=?2, icon=?3, reflect_in_months=?4, updated_at=?5
                 WHERE id=?6 AND deleted_at IS NULL",
                params![name, limit, icon, reflect_in_months as i32, now_iso(), budget_id as i64],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Budget not found".into());
        }
        Self::query_live_budgets(&conn)
    }

    /// Soft-delete a budget. Roots take their whole subtree (months, customs,
    /// items, recurring rules, products) with them; month sub-budgets cannot be
    /// deleted directly (they'd just be auto-recreated by routing).
    pub fn delete_budget(&self, budget_id: u64) -> Result<Vec<Budget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let bid = budget_id as i64;
        let now = now_iso();

        let kind: String = conn
            .query_row(
                "SELECT kind FROM budgets WHERE id = ?1 AND deleted_at IS NULL",
                params![bid],
                |r| r.get(0),
            )
            .map_err(|_| "Budget not found".to_string())?;

        match kind.as_str() {
            "month" => return Err("Месячный бюджет нельзя удалить".into()),
            "custom" => {
                conn.execute(
                    "UPDATE items SET deleted_at = ?1, updated_at = ?1 WHERE budget_id = ?2 AND deleted_at IS NULL",
                    params![now, bid],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE budgets SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
                    params![now, bid],
                )
                .map_err(|e| e.to_string())?;
            }
            _ => {
                // Prevent deleting the last root budget
                let roots: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM budgets WHERE parent_id IS NULL AND deleted_at IS NULL",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                if roots <= 1 {
                    return Err("Cannot delete the last budget".into());
                }
                // Tombstone the whole subtree explicitly (soft deletes don't cascade).
                conn.execute(
                    "UPDATE items SET deleted_at = ?1, updated_at = ?1
                     WHERE deleted_at IS NULL
                       AND budget_id IN (SELECT id FROM budgets WHERE id = ?2 OR parent_id = ?2)",
                    params![now, bid],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE recurring SET deleted_at = ?1, updated_at = ?1 WHERE budget_id = ?2 AND deleted_at IS NULL",
                    params![now, bid],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE products SET deleted_at = ?1, updated_at = ?1 WHERE budget_id = ?2 AND deleted_at IS NULL",
                    params![now, bid],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE budgets SET deleted_at = ?1, updated_at = ?1
                     WHERE (id = ?2 OR parent_id = ?2) AND deleted_at IS NULL",
                    params![now, bid],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        Self::query_live_budgets(&conn)
    }

    /// Idempotent find-or-create for a month sub-budget; the frontend calls it
    /// on load so "the current month" always exists as a row.
    pub fn ensure_month_budget_cmd(
        &self,
        root_id: u64,
        period: &str,
    ) -> Result<Vec<Budget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        Self::ensure_month_budget(&conn, root_id as i64, period)?;
        Self::query_live_budgets(&conn)
    }

    // ── Category CRUD ───────────────────────────────────────────

    pub fn add_category(
        &self,
        key: &str,
        name: &str,
        icon: &str,
        color: &str,
    ) -> Result<Vec<Category>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let live_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM categories WHERE key = ?1 AND deleted_at IS NULL",
                params![key],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if live_exists {
            return Err("Category key already exists".into());
        }
        // Resurrect a tombstoned category with the same key instead of failing
        // the primary key.
        conn.execute(
            "INSERT INTO categories (key, name, icon, color, is_default, uuid, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?6)
             ON CONFLICT(key) DO UPDATE SET
                name = excluded.name, icon = excluded.icon, color = excluded.color,
                deleted_at = NULL, updated_at = excluded.updated_at",
            params![key, name, icon, color, new_uuid(), now_iso()],
        )
        .map_err(|e| e.to_string())?;
        Self::query_live_categories(&conn)
    }

    pub fn update_category(
        &self,
        key: &str,
        name: &str,
        icon: &str,
        color: &str,
    ) -> Result<Vec<Category>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rows = conn
            .execute(
                "UPDATE categories SET name=?1, icon=?2, color=?3, updated_at=?4 WHERE key=?5 AND deleted_at IS NULL",
                params![name, icon, color, now_iso(), key],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Category not found".into());
        }
        Self::query_live_categories(&conn)
    }

    pub fn delete_category(&self, key: &str) -> Result<Vec<Category>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // "other" is mandatory and cannot be deleted
        if key == "other" {
            return Err("Cannot delete the 'Другое' category".into());
        }

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM categories WHERE key = ?1 AND deleted_at IS NULL",
                params![key],
                |r| r.get::<_, i32>(0).map(|v| v > 0),
            )
            .map_err(|e| e.to_string())?;

        if !exists {
            return Err("Category not found".into());
        }

        let now = now_iso();
        // Reassign items to "other"
        conn.execute(
            "UPDATE items SET category = 'other', updated_at = ?1 WHERE category = ?2 AND deleted_at IS NULL",
            params![now, key],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE categories SET deleted_at = ?1, updated_at = ?1 WHERE key = ?2",
            params![now, key],
        )
        .map_err(|e| e.to_string())?;

        Self::query_live_categories(&conn)
    }

    /// Escape a string for safe CSV embedding: double internal quotes
    /// and prefix formula-triggering characters to prevent CSV injection.
    fn csv_safe(s: &str) -> String {
        let escaped = s.replace('"', "\"\"");
        if escaped.starts_with('=')
            || escaped.starts_with('+')
            || escaped.starts_with('-')
            || escaped.starts_with('@')
            || escaped.starts_with('\t')
            || escaped.starts_with('\r')
        {
            format!("'{}", escaped)
        } else {
            escaped
        }
    }

    // ── Auto-backup ──────────────────────────────────────────────

    fn auto_backup(data_dir: &PathBuf, db_path: &PathBuf) {
        if !db_path.exists() {
            return;
        }
        let backup_dir = data_dir.join("backups");
        let _ = fs::create_dir_all(&backup_dir);

        // Date stamp (UTC epoch-day math is fine here: backup filenames are
        // not business data, so the business-dates-are-local rule doesn't apply)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let days = secs / 86400;
        // Convert to YYYY-MM-DD
        let mut y = 1970u64;
        let mut rem = days;
        loop {
            let diy = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                366
            } else {
                365
            };
            if rem < diy {
                break;
            }
            rem -= diy;
            y += 1;
        }
        let ml = [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let mut m = 0u64;
        for (i, &len) in ml.iter().enumerate() {
            let d = if i == 1 && leap { len + 1 } else { len };
            if rem < d {
                m = i as u64;
                break;
            }
            rem -= d;
            m = i as u64;
        }
        let date_str = format!("{}-{:02}-{:02}", y, m + 1, rem + 1);

        let backup_name = format!("plyet-{}.db", date_str);
        let backup_path = backup_dir.join(&backup_name);

        // Only backup once per day
        if !backup_path.exists() {
            let _ = fs::copy(db_path, &backup_path);
        }

        // Prune: keep last 7 backups
        if let Ok(entries) = fs::read_dir(&backup_dir) {
            let mut files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| {
                            n.starts_with("plyet-")
                                && n.ends_with(".db")
                                && !n.starts_with("plyet-pre-")
                        })
                        .unwrap_or(false)
                })
                .collect();
            files.sort();
            while files.len() > 7 {
                let _ = fs::remove_file(files.remove(0));
            }
        }
    }

    // ── Export / Import ─────────────────────────────────────────

    /// CSV of one budget's items. For a root budget this unions the items of
    /// all its live children (months and customs), ordered by date — the
    /// root's own row set is normally empty after the v6 migration.
    pub fn export_csv(&self, budget_id: u64) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let bid = budget_id as i64;

        let parent_id: Option<i64> = conn
            .query_row(
                "SELECT parent_id FROM budgets WHERE id = ?1 AND deleted_at IS NULL",
                params![bid],
                |r| r.get(0),
            )
            .map_err(|_| "Budget not found".to_string())?;

        let scope_sql = if parent_id.is_none() {
            "budget_id IN (SELECT id FROM budgets WHERE (id = ?1 OR parent_id = ?1) AND deleted_at IS NULL)"
        } else {
            "budget_id = ?1"
        };
        let mut stmt = conn
            .prepare(&format!(
                "SELECT date, name, amount, item_type, category, completed, description
                 FROM items WHERE {} AND deleted_at IS NULL ORDER BY date, id",
                scope_sql
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![bid], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i32>(5)? != 0,
                    r.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let categories = Self::query_live_categories(&conn)?;

        let mut csv = String::from("\u{FEFF}Дата,Название,Сумма,Тип,Категория,Куплено,Описание\n");
        for (date, name, amount, item_type, category, completed, description) in &rows {
            let cat_name = categories
                .iter()
                .find(|c| &c.key == category)
                .map(|c| c.name.as_str())
                .unwrap_or(category);
            let item_type_str = if item_type == "income" {
                "Доход"
            } else {
                "Расход"
            };
            // CSV escape: wrap fields in quotes, double internal quotes
            let name = Self::csv_safe(name);
            let desc = Self::csv_safe(description);
            let cat = Self::csv_safe(cat_name);
            // Amount is stored in kopecks, export as rubles with 2 decimal places
            let amount_rub = format!("{:.2}", *amount as f64 / 100.0);
            csv.push_str(&format!(
                "{},\"{}\",{},{},\"{}\",{},\"{}\"\n",
                date,
                name,
                amount_rub,
                item_type_str,
                cat,
                if *completed { "Да" } else { "Нет" },
                desc,
            ));
        }
        Ok(csv)
    }

    pub fn export_json(&self) -> Result<String, String> {
        let envelope = ExportEnvelope {
            schema: 6,
            exported_at: now_iso(),
            data: self.load_all_full()?,
        };
        serde_json::to_string_pretty(&envelope).map_err(|e| e.to_string())
    }

    pub fn import_json(&self, json: &str) -> Result<AppData, String> {
        // Sniff the shape: a v6 envelope carries a `schema` field; anything
        // else is treated as a legacy (pre-hierarchy) plain AppData export and
        // normalized through the same path as the v5→v6 DB migration.
        let (import, legacy) = match serde_json::from_str::<ExportEnvelope>(json) {
            Ok(env) => {
                if env.schema > 6 {
                    return Err(format!(
                        "Файл экспортирован более новой версией приложения (schema {})",
                        env.schema
                    ));
                }
                (env.data, false)
            }
            Err(_) => {
                let data: AppData =
                    serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {}", e))?;
                (data, true)
            }
        };

        // Validate size limits
        if import.budgets.len() > 1000 {
            return Err("Слишком много бюджетов (макс. 1000)".into());
        }
        if import.categories.len() > 500 {
            return Err("Слишком много категорий (макс. 500)".into());
        }
        if import.members.len() > 100 {
            return Err("Слишком много участников (макс. 100)".into());
        }
        let total_items: usize = import.budgets.iter().map(|b| b.items.len()).sum();
        if total_items > 100_000 {
            return Err("Слишком много записей (макс. 100 000)".into());
        }

        // Validate every field with the same rules as the IPC commands, so a
        // crafted/corrupt file cannot persist invalid data. Done before the
        // transaction: on the first failure nothing is written.
        use crate::validate::*;
        for cat in &import.categories {
            validate_category_key(&cat.key)?;
            validate_name(&cat.name)?;
            validate_icon(&cat.icon)?;
            validate_color(&cat.color)?;
            validate_uuid_field(&cat.uuid)?;
            validate_timestamp_field(&cat.created_at)?;
            validate_timestamp_field(&cat.updated_at)?;
        }
        let budget_ids: std::collections::HashSet<u64> =
            import.budgets.iter().map(|b| b.id).collect();
        for budget in &import.budgets {
            validate_name(&budget.name)?;
            validate_amount(budget.limit)?;
            validate_icon(&budget.icon)?;
            validate_budget_kind(&budget.kind)?;
            validate_uuid_field(&budget.uuid)?;
            validate_timestamp_field(&budget.created_at)?;
            validate_timestamp_field(&budget.updated_at)?;
            if let Some(p) = &budget.period {
                validate_period(p)?;
            }
            if let Some(parent) = budget.parent_id {
                if !budget_ids.contains(&parent) {
                    return Err("Budget parent_id references a missing budget".into());
                }
            }
            for item in &budget.items {
                validate_name(&item.name)?;
                validate_amount(item.amount)?;
                validate_date(&item.date)?;
                validate_description(&item.description)?;
                validate_link(&item.link)?;
                validate_uuid_field(&item.uuid)?;
                validate_timestamp_field(&item.created_at)?;
                validate_timestamp_field(&item.updated_at)?;
                if let Some(author) = &item.author_id {
                    validate_uuid_field(author)?;
                }
                let item_type_str = match item.item_type {
                    ItemType::Income => "income",
                    ItemType::Expense => "expense",
                };
                validate_item_type(item_type_str)?;
            }
        }
        for rec in &import.recurring {
            validate_name(&rec.name)?;
            validate_amount(rec.amount)?;
            validate_date(&rec.start_date)?;
            if let Some(ed) = &rec.end_date {
                validate_date(ed)?;
            }
            validate_description(&rec.description)?;
            validate_link(&rec.link)?;
            validate_freq(&rec.freq)?;
            validate_horizon(rec.horizon)?;
            validate_anchor_day(rec.anchor_day)?;
            validate_uuid_field(&rec.uuid)?;
            let rec_type_str = match rec.item_type {
                ItemType::Income => "income",
                ItemType::Expense => "expense",
            };
            validate_item_type(rec_type_str)?;
        }
        for prod in &import.products {
            validate_name(&prod.name)?;
            validate_amount(prod.principal)?;
            validate_amount(prod.down_payment)?;
            validate_date(&prod.start_date)?;
            validate_product_kind(&prod.kind)?;
            validate_rate(prod.annual_rate_bps)?;
            if let Some(er) = prod.early_rate_bps {
                validate_rate(er)?;
            }
            validate_term(prod.term_months)?;
            validate_payment_model(&prod.payment_model)?;
            validate_product_status(&prod.status)?;
            validate_horizon(prod.horizon)?;
            validate_description(&prod.description)?;
            validate_link(&prod.link)?;
            validate_uuid_field(&prod.uuid)?;
        }
        for member in &import.members {
            validate_name(&member.name)?;
            validate_member_role(&member.role)?;
            validate_uuid_field(&member.uuid)?;
        }

        // Known category keys after import ("other" is always re-ensured below).
        let mut known_keys: std::collections::HashSet<String> =
            import.categories.iter().map(|c| c.key.clone()).collect();
        known_keys.insert("other".to_string());

        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Backup current DB before import
        let db_path = self.data_dir.join("plyet.db");
        let backup_path = self.data_dir.join("backups").join("plyet-pre-import.db");
        let _ = fs::create_dir_all(self.data_dir.join("backups"));
        let _ = fs::copy(&db_path, &backup_path);

        let tx = conn.transaction().map_err(|e| e.to_string())?;

        // Clear existing data
        tx.execute_batch(
            "DELETE FROM products; DELETE FROM recurring; DELETE FROM items; DELETE FROM budgets; DELETE FROM categories; DELETE FROM members;",
        )
        .map_err(|e| e.to_string())?;

        // Import categories
        for cat in &import.categories {
            tx.execute(
                "INSERT OR REPLACE INTO categories (key, name, icon, color, is_default, uuid, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    cat.key, cat.name, cat.icon, cat.color, cat.is_default as i32,
                    uuid_or_new(&cat.uuid), cat.created_at, cat.updated_at, cat.deleted_at
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        // Import budgets and items
        for budget in &import.budgets {
            tx.execute(
                "INSERT INTO budgets (id, name, budget_limit, icon, parent_id, kind, period,
                        reflect_in_months, uuid, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    budget.id as i64,
                    budget.name,
                    budget.limit,
                    budget.icon,
                    budget.parent_id.map(|v| v as i64),
                    budget.kind,
                    budget.period,
                    budget.reflect_in_months as i32,
                    uuid_or_new(&budget.uuid),
                    budget.created_at,
                    budget.updated_at,
                    budget.deleted_at,
                ],
            )
            .map_err(|e| e.to_string())?;

            for item in &budget.items {
                let item_type_str = match item.item_type {
                    ItemType::Income => "income",
                    ItemType::Expense => "expense",
                };
                // Reassign items referencing an unknown category to "other".
                let category: &str = if known_keys.contains(&item.category) {
                    &item.category
                } else {
                    "other"
                };
                tx.execute(
                    "INSERT INTO items (id, budget_id, name, amount, category, date, description, link,
                            item_type, completed, source_kind, source_id, uuid, created_at, updated_at,
                            deleted_at, author_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                    params![
                        item.id as i64,
                        budget.id as i64,
                        item.name,
                        item.amount,
                        category,
                        item.date,
                        item.description,
                        item.link,
                        item_type_str,
                        item.completed as i32,
                        item.source_kind,
                        item.source_id as i64,
                        uuid_or_new(&item.uuid),
                        item.created_at,
                        item.updated_at,
                        item.deleted_at,
                        item.author_id,
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        // Import recurring rules
        for rec in &import.recurring {
            let rec_type_str = match rec.item_type {
                ItemType::Income => "income",
                ItemType::Expense => "expense",
            };
            let category: &str = if known_keys.contains(&rec.category) {
                &rec.category
            } else {
                "other"
            };
            tx.execute(
                "INSERT INTO recurring
                    (id, budget_id, name, amount, category, item_type, freq, anchor_day,
                     start_date, end_date, horizon, last_paid_date, description, link,
                     uuid, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                params![
                    rec.id as i64,
                    rec.budget_id as i64,
                    rec.name,
                    rec.amount,
                    category,
                    rec_type_str,
                    rec.freq,
                    rec.anchor_day,
                    rec.start_date,
                    rec.end_date,
                    rec.horizon,
                    rec.last_paid_date,
                    rec.description,
                    rec.link,
                    uuid_or_new(&rec.uuid),
                    rec.created_at,
                    rec.updated_at,
                    rec.deleted_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        // Import products
        for prod in &import.products {
            let category: &str = if known_keys.contains(&prod.category) {
                &prod.category
            } else {
                "other"
            };
            tx.execute(
                "INSERT INTO products
                    (id, budget_id, kind, name, principal, annual_rate_bps, term_months, start_date,
                     payment_model, early_rate_bps, horizon, category, status, principal_paid,
                     payments_made, manual_payment, down_payment, description, link,
                     uuid, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
                params![
                    prod.id as i64,
                    prod.budget_id as i64,
                    prod.kind,
                    prod.name,
                    prod.principal,
                    prod.annual_rate_bps,
                    prod.term_months,
                    prod.start_date,
                    prod.payment_model,
                    prod.early_rate_bps,
                    prod.horizon,
                    category,
                    prod.status,
                    prod.principal_paid,
                    prod.payments_made,
                    prod.manual_payment,
                    prod.down_payment,
                    prod.description,
                    prod.link,
                    uuid_or_new(&prod.uuid),
                    prod.created_at,
                    prod.updated_at,
                    prod.deleted_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        // Import members
        for member in &import.members {
            tx.execute(
                "INSERT INTO members (id, uuid, name, role, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    member.id as i64,
                    uuid_or_new(&member.uuid),
                    member.name,
                    member.role,
                    member.created_at,
                    member.updated_at,
                    member.deleted_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        // Legacy files predate the hierarchy: turn flat budgets into roots and
        // spread their items into month sub-budgets. Idempotent for v6 data.
        Self::normalize_hierarchy(&tx, legacy)?;

        tx.commit().map_err(|e| e.to_string())?;

        // Re-ensure defaults
        drop(conn);
        self.ensure_defaults()?;
        self.load_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh unique data dir per test (cargo test runs tests in parallel).
    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "plyet-test-{}-{}-{}",
            name,
            std::process::id(),
            new_uuid()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn schema_version(dir: &PathBuf) -> i64 {
        let conn = Connection::open(dir.join("plyet.db")).unwrap();
        conn.query_row(
            "SELECT CAST(value AS INTEGER) FROM meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    /// Hand-build a schema-v5 database (the last flat-budget schema) with a
    /// month-named budget, items in two months and a recurring rule.
    fn create_v5_db(dir: &PathBuf) {
        let conn = Connection::open(dir.join("plyet.db")).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE budgets (
                id INTEGER PRIMARY KEY, name TEXT NOT NULL,
                budget_limit INTEGER NOT NULL DEFAULT 0, icon TEXT NOT NULL DEFAULT 'wallet');
            CREATE TABLE items (
                id INTEGER PRIMARY KEY, budget_id INTEGER NOT NULL, name TEXT NOT NULL,
                amount INTEGER NOT NULL, category TEXT NOT NULL DEFAULT 'other', date TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '', link TEXT NOT NULL DEFAULT '',
                item_type TEXT NOT NULL DEFAULT 'expense', completed INTEGER NOT NULL DEFAULT 0,
                source_kind TEXT NOT NULL DEFAULT '', source_id INTEGER NOT NULL DEFAULT 0,
                source_principal INTEGER NOT NULL DEFAULT 0, source_payment INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (budget_id) REFERENCES budgets(id) ON DELETE CASCADE);
            CREATE TABLE categories (
                key TEXT PRIMARY KEY, name TEXT NOT NULL, icon TEXT NOT NULL,
                color TEXT NOT NULL, is_default INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE recurring (
                id INTEGER PRIMARY KEY, budget_id INTEGER NOT NULL, name TEXT NOT NULL,
                amount INTEGER NOT NULL, category TEXT NOT NULL DEFAULT 'other',
                item_type TEXT NOT NULL DEFAULT 'expense', freq TEXT NOT NULL DEFAULT 'monthly',
                anchor_day INTEGER NOT NULL DEFAULT 1, start_date TEXT NOT NULL, end_date TEXT,
                horizon INTEGER NOT NULL DEFAULT 1, last_paid_date TEXT,
                description TEXT NOT NULL DEFAULT '', link TEXT NOT NULL DEFAULT '',
                FOREIGN KEY (budget_id) REFERENCES budgets(id) ON DELETE CASCADE);
            CREATE TABLE products (
                id INTEGER PRIMARY KEY, budget_id INTEGER NOT NULL, kind TEXT NOT NULL DEFAULT 'loan',
                name TEXT NOT NULL, principal INTEGER NOT NULL DEFAULT 0,
                annual_rate_bps INTEGER NOT NULL DEFAULT 0, term_months INTEGER NOT NULL DEFAULT 1,
                start_date TEXT NOT NULL, payment_model TEXT NOT NULL DEFAULT 'annuity',
                early_rate_bps INTEGER, horizon INTEGER NOT NULL DEFAULT 1,
                category TEXT NOT NULL DEFAULT 'other', status TEXT NOT NULL DEFAULT 'active',
                principal_paid INTEGER NOT NULL DEFAULT 0, payments_made INTEGER NOT NULL DEFAULT 0,
                manual_payment INTEGER, description TEXT NOT NULL DEFAULT '',
                link TEXT NOT NULL DEFAULT '', down_payment INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (budget_id) REFERENCES budgets(id) ON DELETE CASCADE);
            INSERT INTO meta (key, value) VALUES ('schema_version', '5');
            INSERT INTO budgets (id, name, budget_limit, icon) VALUES (1, 'Июнь 2026', 5000000, 'wallet');
            INSERT INTO items (id, budget_id, name, amount, category, date, item_type, completed)
                VALUES (1, 1, 'Продукты', 120000, 'food', '2026-06-05', 'expense', 1),
                       (2, 1, 'Зарплата', 15000000, 'salary', '2026-06-25', 'income', 1),
                       (3, 1, 'Кино', 80000, 'entertainment', '2026-07-10', 'expense', 0);
            INSERT INTO recurring (id, budget_id, name, amount, start_date, anchor_day)
                VALUES (1, 1, 'Подписка', 29900, '2026-06-01', 1);
            ",
        )
        .unwrap();
    }

    fn find_root<'a>(data: &'a AppData) -> &'a Budget {
        data.budgets.iter().find(|b| b.parent_id.is_none()).unwrap()
    }

    fn find_month<'a>(data: &'a AppData, period: &str) -> Option<&'a Budget> {
        data.budgets
            .iter()
            .find(|b| b.kind == "month" && b.period.as_deref() == Some(period))
    }

    #[test]
    fn fresh_db_seeds_v6_root_and_member() {
        let dir = test_dir("fresh");
        let db = Database::open(&dir).unwrap();
        let data = db.load_all().unwrap();

        assert_eq!(data.budgets.len(), 1);
        let root = &data.budgets[0];
        assert_eq!(root.name, "Мой бюджет");
        assert_eq!(root.kind, "root");
        assert!(root.parent_id.is_none());
        assert!(!root.uuid.is_empty());
        assert!(!root.created_at.is_empty());

        assert_eq!(data.members.len(), 1);
        assert_eq!(data.members[0].role, "owner");
        assert!(!data.members[0].uuid.is_empty());

        assert!(!data.categories.is_empty());
        assert!(data.categories.iter().all(|c| !c.uuid.is_empty()));

        drop(db);
        assert_eq!(schema_version(&dir), 6);
    }

    #[test]
    fn migrates_v5_flat_budget_to_hierarchy() {
        let dir = test_dir("migrate-v5");
        create_v5_db(&dir);

        let db = Database::open(&dir).unwrap();
        let data = db.load_all().unwrap();

        // The month-named flat budget became a renamed root…
        let root = find_root(&data);
        assert_eq!(root.name, "Мой бюджет");
        assert_eq!(root.kind, "root");
        assert!(root.items.is_empty(), "root items must move into months");

        // …with month sub-budgets per distinct item month.
        let june = find_month(&data, "2026-06").expect("june sub-budget");
        let july = find_month(&data, "2026-07").expect("july sub-budget");
        assert_eq!(june.name, "Июнь 2026");
        assert_eq!(july.name, "Июль 2026");
        assert_eq!(june.parent_id, Some(root.id));
        assert_eq!(june.limit, root.limit, "months inherit the root limit");
        assert_eq!(june.items.len(), 2);
        assert_eq!(july.items.len(), 1);
        assert_eq!(july.items[0].name, "Кино");

        // Recurring rules stay attached to the root.
        assert_eq!(data.recurring.len(), 1);
        assert_eq!(data.recurring[0].budget_id, root.id);

        // Every row got a unique uuid and timestamps.
        let mut uuids: Vec<&str> = data.budgets.iter().map(|b| b.uuid.as_str()).collect();
        uuids.extend(
            data.budgets
                .iter()
                .flat_map(|b| &b.items)
                .map(|i| i.uuid.as_str()),
        );
        uuids.extend(data.recurring.iter().map(|r| r.uuid.as_str()));
        assert!(uuids.iter().all(|u| !u.is_empty()));
        let unique: std::collections::HashSet<&&str> = uuids.iter().collect();
        assert_eq!(unique.len(), uuids.len());

        // Pre-migration safety copy exists.
        assert!(dir.join("backups").join("plyet-pre-v6.db").exists());

        drop(db);
        assert_eq!(schema_version(&dir), 6);
    }

    #[test]
    fn add_item_routes_by_date_and_creates_month_once() {
        let dir = test_dir("route");
        let db = Database::open(&dir).unwrap();
        let root_id = db.load_all().unwrap().budgets[0].id;

        db.add_item(
            root_id,
            "Кофе",
            25000,
            "food",
            "2026-07-15",
            "",
            "",
            "expense",
        )
        .unwrap();
        let budgets = db
            .add_item(
                root_id,
                "Чай",
                15000,
                "food",
                "2026-07-20",
                "",
                "",
                "expense",
            )
            .unwrap();

        let months: Vec<&Budget> = budgets.iter().filter(|b| b.kind == "month").collect();
        assert_eq!(months.len(), 1, "same month must be reused, not duplicated");
        assert_eq!(months[0].period.as_deref(), Some("2026-07"));
        assert_eq!(months[0].items.len(), 2);
    }

    #[test]
    fn update_item_moves_between_months() {
        let dir = test_dir("move");
        let db = Database::open(&dir).unwrap();
        let root_id = db.load_all().unwrap().budgets[0].id;

        let budgets = db
            .add_item(
                root_id,
                "Билет",
                500000,
                "other",
                "2026-07-15",
                "",
                "",
                "expense",
            )
            .unwrap();
        let item_id = budgets
            .iter()
            .find(|b| b.period.as_deref() == Some("2026-07"))
            .unwrap()
            .items[0]
            .id;

        let budgets = db
            .update_item(
                item_id,
                "Билет",
                500000,
                "other",
                "2026-08-02",
                "",
                "",
                "expense",
            )
            .unwrap();
        let july = budgets
            .iter()
            .find(|b| b.period.as_deref() == Some("2026-07"))
            .unwrap();
        let august = budgets
            .iter()
            .find(|b| b.period.as_deref() == Some("2026-08"))
            .unwrap();
        assert!(july.items.is_empty());
        assert_eq!(august.items.len(), 1);
        assert_eq!(
            august.items[0].id, item_id,
            "id must be stable across the move"
        );
    }

    #[test]
    fn custom_sub_budget_keeps_items_regardless_of_date() {
        let dir = test_dir("custom");
        let db = Database::open(&dir).unwrap();
        let root_id = db.load_all().unwrap().budgets[0].id;

        let budgets = db
            .create_sub_budget(root_id, "Отпуск", 0, "wallet", true)
            .unwrap();
        let custom = budgets.iter().find(|b| b.kind == "custom").unwrap();
        assert!(custom.reflect_in_months);
        let custom_id = custom.id;

        let budgets = db
            .add_item(
                custom_id,
                "Отель",
                3000000,
                "other",
                "2026-09-10",
                "",
                "",
                "expense",
            )
            .unwrap();
        let custom = budgets.iter().find(|b| b.id == custom_id).unwrap();
        assert_eq!(custom.items.len(), 1, "custom budgets keep their items");
        assert!(
            budgets
                .iter()
                .all(|b| b.period.as_deref() != Some("2026-09")),
            "no month budget should be auto-created for custom items"
        );
    }

    #[test]
    fn materialize_recurring_into_month_and_unmaterialize() {
        let dir = test_dir("materialize");
        let db = Database::open(&dir).unwrap();
        let root_id = db.load_all().unwrap().budgets[0].id;

        let recurring = db
            .add_recurring(
                root_id,
                "Подписка",
                29900,
                "bills",
                "expense",
                "monthly",
                5,
                "2026-07-01",
                None,
                3,
                "",
                "",
            )
            .unwrap();
        let rule_id = recurring[0].id;

        let result = db.materialize_recurring(rule_id, "2026-08-05").unwrap();
        assert_eq!(
            result.recurring[0].last_paid_date.as_deref(),
            Some("2026-08-05")
        );
        let august = result
            .budgets
            .iter()
            .find(|b| b.period.as_deref() == Some("2026-08"))
            .expect("payment month auto-created");
        assert_eq!(august.items.len(), 1);
        let item = &august.items[0];
        assert!(item.completed);
        assert_eq!(item.source_kind, "recurring");

        let undo = db.unmaterialize_item(item.id).unwrap();
        assert_eq!(undo.recurring[0].last_paid_date, None, "rule rolled back");
        let august = undo
            .budgets
            .iter()
            .find(|b| b.period.as_deref() == Some("2026-08"))
            .unwrap();
        assert!(august.items.is_empty(), "materialized item tombstoned");
    }

    #[test]
    fn soft_deleted_rows_hidden_but_exported_and_roundtrip() {
        let dir = test_dir("tombstone");
        let db = Database::open(&dir).unwrap();
        let root_id = db.load_all().unwrap().budgets[0].id;

        let budgets = db
            .add_item(
                root_id,
                "Ошибка",
                100,
                "other",
                "2026-07-01",
                "",
                "",
                "expense",
            )
            .unwrap();
        let item_id = budgets.iter().flat_map(|b| &b.items).next().unwrap().id;

        let budgets = db.delete_item(item_id).unwrap();
        assert!(
            budgets.iter().all(|b| b.items.is_empty()),
            "hidden from live view"
        );

        let json = db.export_json().unwrap();
        let env: ExportEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(env.schema, 6);
        let exported_item = env
            .data
            .budgets
            .iter()
            .flat_map(|b| &b.items)
            .find(|i| i.id == item_id)
            .expect("tombstone present in export");
        assert!(exported_item.deleted_at.is_some());
        assert!(!env.data.members.is_empty(), "members included in export");

        // Round-trip: import the export into the same DB, tombstone survives, UI stays clean.
        let data = db.import_json(&json).unwrap();
        assert!(data.budgets.iter().all(|b| b.items.is_empty()));
        let json2 = db.export_json().unwrap();
        let env2: ExportEnvelope = serde_json::from_str(&json2).unwrap();
        let total_items: usize = env2.data.budgets.iter().map(|b| b.items.len()).sum();
        assert_eq!(total_items, 1, "tombstoned item survived the round-trip");
    }

    #[test]
    fn legacy_json_import_normalizes_to_hierarchy() {
        let dir = test_dir("legacy-import");
        let db = Database::open(&dir).unwrap();

        // A pre-v6 export: flat month-named budget, no uuids, extinct `status` field.
        let legacy = r#"{
            "budgets": [{
                "id": 1, "name": "Июнь 2026", "limit": 5000000, "icon": "wallet",
                "items": [
                    {"id": 1, "name": "Продукты", "amount": 120000, "category": "food",
                     "date": "2026-06-05", "description": "", "link": "",
                     "item_type": "expense", "completed": true, "status": "completed"},
                    {"id": 2, "name": "Кино", "amount": 80000, "category": "other",
                     "date": "2026-07-10", "description": "", "link": "",
                     "item_type": "expense", "completed": false, "status": "planned"}
                ]
            }],
            "categories": [],
            "recurring": [],
            "products": [],
            "next_id": 3
        }"#;

        let data = db.import_json(legacy).unwrap();
        let root = find_root(&data);
        assert_eq!(root.name, "Мой бюджет", "legacy month-named budget renamed");
        assert!(root.items.is_empty());
        assert_eq!(find_month(&data, "2026-06").unwrap().items.len(), 1);
        assert_eq!(find_month(&data, "2026-07").unwrap().items.len(), 1);
        assert!(data.budgets.iter().all(|b| !b.uuid.is_empty()));
    }

    #[test]
    fn delete_budget_rules() {
        let dir = test_dir("delete-budget");
        let db = Database::open(&dir).unwrap();
        let root_id = db.load_all().unwrap().budgets[0].id;

        // The only root cannot be deleted.
        assert!(db.delete_budget(root_id).is_err());

        // Month sub-budgets cannot be deleted directly.
        let budgets = db.ensure_month_budget_cmd(root_id, "2026-07").unwrap();
        let month_id = budgets.iter().find(|b| b.kind == "month").unwrap().id;
        assert!(db.delete_budget(month_id).is_err());

        // A second root can be deleted, taking its subtree with it.
        let budgets = db.create_budget("Семья", 0, "wallet").unwrap();
        let second_root = budgets
            .iter()
            .find(|b| b.parent_id.is_none() && b.id != root_id)
            .unwrap()
            .id;
        db.add_item(
            second_root,
            "Тест",
            100,
            "other",
            "2026-07-01",
            "",
            "",
            "expense",
        )
        .unwrap();
        let budgets = db.delete_budget(second_root).unwrap();
        assert!(budgets.iter().all(|b| b.id != second_root));
        assert!(budgets.iter().all(|b| b.parent_id != Some(second_root)));
    }

    #[test]
    fn csv_export_for_root_unions_children() {
        let dir = test_dir("csv");
        let db = Database::open(&dir).unwrap();
        let root_id = db.load_all().unwrap().budgets[0].id;

        db.add_item(
            root_id,
            "Июльская",
            100,
            "other",
            "2026-07-15",
            "",
            "",
            "expense",
        )
        .unwrap();
        db.add_item(
            root_id,
            "Августовская",
            200,
            "other",
            "2026-08-15",
            "",
            "",
            "expense",
        )
        .unwrap();

        let csv = db.export_csv(root_id).unwrap();
        assert!(csv.contains("Июльская"));
        assert!(csv.contains("Августовская"));
    }
}
