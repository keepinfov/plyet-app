use crate::data::*;
use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
    data_dir: PathBuf,
}

impl Database {
    pub fn open(data_dir: &PathBuf) -> Result<Self, String> {
        fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let db_path = data_dir.join("plyet.db");
        let json_path = data_dir.join("budgets.json");
        let needs_migration = !db_path.exists() && json_path.exists();

        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| e.to_string())?;

        Self::create_tables(&conn)?;

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

        // Ensure default categories exist
        db.ensure_defaults()?;

        Ok(db)
    }

    fn create_tables(conn: &Connection) -> Result<(), String> {
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

        Ok(())
    }

    /// Whether `table` has a column named `col`. Table/column names are fixed
    /// literals at the call sites, so interpolation here is not injectable.
    fn column_exists(conn: &Connection, table: &str, col: &str) -> bool {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1", table),
            params![col],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false)
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
                params![budget.id as i64, budget.name, budget.limit * 100, budget.icon],
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

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn ensure_defaults(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // On first run (no categories), seed all defaults
        let cat_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))
            .unwrap_or(0);

        if cat_count == 0 {
            let defaults = default_categories();
            for cat in &defaults {
                conn.execute(
                    "INSERT OR IGNORE INTO categories (key, name, icon, color, is_default) VALUES (?1, ?2, ?3, ?4, 1)",
                    params![cat.key, cat.name, cat.icon, cat.color],
                )
                .map_err(|e| e.to_string())?;
            }
        } else {
            // Always ensure "other" exists (mandatory fallback)
            conn.execute(
                "INSERT OR IGNORE INTO categories (key, name, icon, color, is_default) VALUES ('other', 'Другое', 'other', '#00ACC1', 1)",
                [],
            )
            .map_err(|e| e.to_string())?;
        }

        // Seed budget if none exist
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM budgets", [], |r| r.get(0))
            .unwrap_or(0);
        if count == 0 {
            let seed = seed_budget_name();
            conn.execute(
                "INSERT INTO budgets (name, budget_limit, icon) VALUES (?1, 0, 'wallet')",
                params![seed],
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    // ── Queries ─────────────────────────────────────────────────

    pub fn load_all(&self) -> Result<AppData, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let categories = Self::query_categories(&conn)?;
        let budgets = Self::query_all_budgets(&conn)?;
        let recurring = Self::query_recurring(&conn)?;
        let products = Self::query_products(&conn)?;

        Ok(AppData {
            budgets,
            categories,
            recurring,
            products,
            next_id: 0, // Not used with SQLite
        })
    }

    fn query_products(conn: &Connection) -> Result<Vec<Product>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, budget_id, kind, name, principal, annual_rate_bps, term_months,
                        start_date, payment_model, early_rate_bps, horizon, category, status,
                        principal_paid, payments_made, manual_payment, description, link, down_payment
                 FROM products ORDER BY id",
            )
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
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read products: {}", e))?;
        Ok(rows)
    }

    fn query_recurring(conn: &Connection) -> Result<Vec<Recurring>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, budget_id, name, amount, category, item_type, freq, anchor_day,
                        start_date, end_date, horizon, last_paid_date, description, link
                 FROM recurring ORDER BY id",
            )
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
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read recurring: {}", e))?;
        Ok(rows)
    }

    fn query_categories(conn: &Connection) -> Result<Vec<Category>, String> {
        let mut stmt = conn
            .prepare("SELECT key, name, icon, color, is_default FROM categories ORDER BY is_default DESC, key")
            .map_err(|e| e.to_string())?;
        let cats = stmt
            .query_map([], |row| {
                Ok(Category {
                    key: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    color: row.get(3)?,
                    is_default: row.get::<_, i32>(4)? != 0,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read category: {}", e))?;
        Ok(cats)
    }

    fn query_all_budgets(conn: &Connection) -> Result<Vec<Budget>, String> {
        let mut budget_stmt = conn
            .prepare("SELECT id, name, budget_limit, icon FROM budgets ORDER BY id")
            .map_err(|e| e.to_string())?;
        let budget_rows: Vec<(i64, String, i64, String)> = budget_stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read budget: {}", e))?;

        let mut budgets = Vec::new();
        for (id, name, limit, icon) in budget_rows {
            let items = Self::query_items(conn, id)?;
            budgets.push(Budget {
                id: id as u64,
                name,
                limit,
                icon,
                items,
            });
        }
        Ok(budgets)
    }

    fn query_budget(conn: &Connection, budget_id: i64) -> Result<Budget, String> {
        let (name, limit, icon): (String, i64, String) = conn
            .query_row(
                "SELECT name, budget_limit, icon FROM budgets WHERE id = ?1",
                params![budget_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| "Budget not found".to_string())?;

        let items = Self::query_items(conn, budget_id)?;
        Ok(Budget {
            id: budget_id as u64,
            name,
            limit,
            icon,
            items,
        })
    }

    fn query_items(conn: &Connection, budget_id: i64) -> Result<Vec<Item>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, amount, category, date, description, link, item_type, completed,
                        source_kind, source_id
                 FROM items WHERE budget_id = ?1 ORDER BY id",
            )
            .map_err(|e| e.to_string())?;
        let items = stmt
            .query_map(params![budget_id], |row| {
                let item_type_str: String = row.get(7)?;
                let completed: i32 = row.get(8)?;
                Ok(Item {
                    id: row.get::<_, i64>(0)? as u64,
                    name: row.get(1)?,
                    amount: row.get(2)?,
                    category: row.get(3)?,
                    date: row.get(4)?,
                    description: row.get(5)?,
                    link: row.get(6)?,
                    status: if completed != 0 {
                        ItemStatus::Completed
                    } else {
                        ItemStatus::Planned
                    },
                    item_type: if item_type_str == "income" {
                        ItemType::Income
                    } else {
                        ItemType::Expense
                    },
                    completed: completed != 0,
                    source_kind: row.get(9)?,
                    source_id: row.get::<_, i64>(10)? as u64,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read item: {}", e))?;
        Ok(items)
    }

    // ── Mutations ───────────────────────────────────────────────

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
    ) -> Result<Budget, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let bid = budget_id as i64;

        // Verify budget exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM budgets WHERE id = ?1",
                params![bid],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !exists {
            return Err("Budget not found".into());
        }

        conn.execute(
            "INSERT INTO items (budget_id, name, amount, category, date, description, link, item_type, completed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
            params![bid, name, amount, category, date, description, link, item_type],
        )
        .map_err(|e| e.to_string())?;

        Self::query_budget(&conn, bid)
    }

    pub fn update_item(
        &self,
        budget_id: u64,
        item_id: u64,
        name: &str,
        amount: i64,
        category: &str,
        date: &str,
        description: &str,
        link: &str,
        item_type: &str,
    ) -> Result<Budget, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rows = conn
            .execute(
                "UPDATE items SET name=?1, amount=?2, category=?3, date=?4, description=?5, link=?6, item_type=?7
                 WHERE id=?8 AND budget_id=?9",
                params![name, amount, category, date, description, link, item_type, item_id as i64, budget_id as i64],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Item not found".into());
        }
        Self::query_budget(&conn, budget_id as i64)
    }

    pub fn delete_item(&self, budget_id: u64, item_id: u64) -> Result<Budget, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM items WHERE id = ?1 AND budget_id = ?2",
            params![item_id as i64, budget_id as i64],
        )
        .map_err(|e| e.to_string())?;
        Self::query_budget(&conn, budget_id as i64)
    }

    pub fn toggle_completed(&self, budget_id: u64, item_id: u64) -> Result<Budget, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rows = conn
            .execute(
                "UPDATE items SET completed = 1 - completed WHERE id = ?1 AND budget_id = ?2",
                params![item_id as i64, budget_id as i64],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Item not found".into());
        }
        Self::query_budget(&conn, budget_id as i64)
    }

    /// Undo a materialized occurrence: delete the real item and roll its source
    /// back so the virtual occurrence reappears. For recurring rules this
    /// recomputes `last_paid_date` from the remaining materialized items; for
    /// products it reverses the `payments_made`/`principal_paid` deltas this item
    /// contributed and re-activates the product. Returns the affected budget,
    /// the recurring list and the product list (the caller refreshes all three).
    pub fn unmaterialize_item(
        &self,
        budget_id: u64,
        item_id: u64,
    ) -> Result<UnmaterializeResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let bid = budget_id as i64;

        let (source_kind, source_id, source_principal, source_payment): (String, i64, i64, i64) =
            conn.query_row(
                "SELECT source_kind, source_id, source_principal, source_payment
                 FROM items WHERE id = ?1 AND budget_id = ?2",
                params![item_id as i64, bid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|_| "Item not found".to_string())?;

        if source_kind.is_empty() {
            return Err("Item is not linked to a recurring rule or product".into());
        }

        conn.execute(
            "DELETE FROM items WHERE id = ?1 AND budget_id = ?2",
            params![item_id as i64, bid],
        )
        .map_err(|e| e.to_string())?;

        if source_kind == "recurring" {
            // Roll the rule back to its most recent still-materialized occurrence.
            let last: Option<String> = conn
                .query_row(
                    "SELECT MAX(date) FROM items
                     WHERE source_kind = 'recurring' AND source_id = ?1 AND budget_id = ?2",
                    params![source_id, bid],
                    |r| r.get(0),
                )
                .unwrap_or(None);
            conn.execute(
                "UPDATE recurring SET last_paid_date = ?1 WHERE id = ?2",
                params![last, source_id],
            )
            .map_err(|e| e.to_string())?;
        } else {
            // deposit | loan | mortgage: reverse this item's contribution and
            // re-activate (removing any payment means it's no longer fully done).
            conn.execute(
                "UPDATE products
                 SET payments_made = MAX(0, payments_made - ?1),
                     principal_paid = MAX(0, principal_paid - ?2),
                     status = 'active'
                 WHERE id = ?3",
                params![source_payment, source_principal, source_id],
            )
            .map_err(|e| e.to_string())?;
        }

        let budget = Self::query_budget(&conn, bid)?;
        let recurring = Self::query_recurring(&conn)?;
        let products = Self::query_products(&conn)?;
        Ok(UnmaterializeResult {
            budget,
            recurring,
            products,
        })
    }

    // ── Recurring CRUD ──────────────────────────────────────────

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
        let bid = budget_id as i64;

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM budgets WHERE id = ?1",
                params![bid],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !exists {
            return Err("Budget not found".into());
        }

        conn.execute(
            "INSERT INTO recurring
                (budget_id, name, amount, category, item_type, freq, anchor_day,
                 start_date, end_date, horizon, last_paid_date, description, link)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11, ?12)",
            params![
                bid, name, amount, category, item_type, freq, anchor_day,
                start_date, end_date, horizon, description, link
            ],
        )
        .map_err(|e| e.to_string())?;

        Self::query_recurring(&conn)
    }

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
                    anchor_day=?6, start_date=?7, end_date=?8, horizon=?9, description=?10, link=?11
                 WHERE id=?12",
                params![
                    name, amount, category, item_type, freq, anchor_day,
                    start_date, end_date, horizon, description, link, id as i64
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
        conn.execute("DELETE FROM recurring WHERE id = ?1", params![id as i64])
            .map_err(|e| e.to_string())?;
        Self::query_recurring(&conn)
    }

    /// Mark a virtual recurring occurrence on `date` as paid: insert a real
    /// completed `Item` (built from the rule's current fields) linked via
    /// `source_*`, and advance the rule's `last_paid_date`.
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
                 FROM recurring WHERE id = ?1",
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

        conn.execute(
            "INSERT INTO items
                (budget_id, name, amount, category, date, description, link, item_type, completed, source_kind, source_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, 'recurring', ?9)",
            params![budget_id, name, amount, category, date, description, link, item_type, id as i64],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE recurring SET last_paid_date = ?1 WHERE id = ?2",
            params![date, id as i64],
        )
        .map_err(|e| e.to_string())?;

        let budget = Self::query_budget(&conn, budget_id)?;
        let recurring = Self::query_recurring(&conn)?;
        Ok(MaterializeResult { budget, recurring })
    }

    // ── Product CRUD ────────────────────────────────────────────

    /// Create a deposit, loan or mortgage. Opening a deposit debits its principal
    /// from the budget now (a real completed expense), since the money leaves the
    /// spendable balance; a mortgage likewise debits its down payment. Loans don't
    /// move cash on creation — only their scheduled payments do.
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
        let bid = budget_id as i64;

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM budgets WHERE id = ?1",
                params![bid],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !exists {
            return Err("Budget not found".into());
        }

        conn.execute(
            "INSERT INTO products
                (budget_id, kind, name, principal, annual_rate_bps, term_months, start_date,
                 payment_model, early_rate_bps, horizon, category, status, principal_paid,
                 payments_made, manual_payment, down_payment, description, link)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'active', 0, 0, NULL, ?12, ?13, ?14)",
            params![
                bid, kind, name, principal, annual_rate_bps, term_months, start_date,
                payment_model, early_rate_bps, horizon, category, down_payment, description, link
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
            conn.execute(
                "INSERT INTO items
                    (budget_id, name, amount, category, date, description, link, item_type, completed, source_kind, source_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', 'expense', 1, ?7, ?8)",
                params![bid, name, upfront, category, start_date, upfront_desc, kind, product_id],
            )
            .map_err(|e| e.to_string())?;
        }

        let budget = Self::query_budget(&conn, bid)?;
        let products = Self::query_products(&conn)?;
        Ok(ProductResult { budget, products })
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
                    down_payment=?10, description=?11, link=?12
                 WHERE id=?13",
                params![
                    name, principal, annual_rate_bps, term_months, start_date, payment_model,
                    early_rate_bps, horizon, category, down_payment, description, link, id as i64
                ],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Product not found".into());
        }
        Self::query_products(&conn)
    }

    /// Delete a product definition. Already-materialized items (real payment
    /// history) are intentionally kept.
    pub fn delete_product(&self, id: u64) -> Result<Vec<Product>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM products WHERE id = ?1", params![id as i64])
            .map_err(|e| e.to_string())?;
        Self::query_products(&conn)
    }

    /// Materialize one virtual product occurrence (a loan payment or a deposit
    /// payout). The schedule math lives on the frontend, which passes the
    /// computed `amount`, `principal_portion` (loan only) and whether this
    /// occurrence `closes` the product.
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
                "SELECT budget_id, name, category, kind FROM products WHERE id = ?1",
                params![id as i64],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|_| "Product not found".to_string())?;

        conn.execute(
            "INSERT INTO items
                (budget_id, name, amount, category, date, description, link, item_type, completed,
                 source_kind, source_id, source_principal, source_payment)
             VALUES (?1, ?2, ?3, ?4, ?5, '', '', ?6, 1, ?7, ?8, ?9, 1)",
            params![budget_id, name, amount, category, date, item_type, kind, id as i64, principal_portion],
        )
        .map_err(|e| e.to_string())?;

        let new_status = if closes { "closed" } else { "active" };
        conn.execute(
            "UPDATE products
             SET payments_made = payments_made + 1,
                 principal_paid = principal_paid + ?1,
                 status = ?2
             WHERE id = ?3",
            params![principal_portion, new_status, id as i64],
        )
        .map_err(|e| e.to_string())?;

        let budget = Self::query_budget(&conn, budget_id)?;
        let products = Self::query_products(&conn)?;
        Ok(ProductResult { budget, products })
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
                 FROM products WHERE id = ?1 AND kind IN ('loan', 'mortgage')",
                params![id as i64],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .map_err(|_| "Loan not found".to_string())?;

        conn.execute(
            "INSERT INTO items
                (budget_id, name, amount, category, date, description, link, item_type, completed,
                 source_kind, source_id, source_principal, source_payment)
             VALUES (?1, ?2, ?3, ?4, ?5, 'Досрочный платёж', '', 'expense', 1, ?6, ?7, ?3, 0)",
            params![budget_id, name, amount, category, date, kind, id as i64],
        )
        .map_err(|e| e.to_string())?;

        let closes = principal_paid + amount >= principal;
        let new_status = if closes { "closed" } else { "active" };
        conn.execute(
            "UPDATE products SET principal_paid = principal_paid + ?1, status = ?2 WHERE id = ?3",
            params![amount, new_status, id as i64],
        )
        .map_err(|e| e.to_string())?;

        let budget = Self::query_budget(&conn, budget_id)?;
        let products = Self::query_products(&conn)?;
        Ok(ProductResult { budget, products })
    }

    /// Close a deposit early: return the body now and pay out the interest
    /// earned so far recomputed at the reduced rate (the frontend computes the
    /// total `payout`). Inserts a single income item and marks the product closed.
    pub fn close_deposit(
        &self,
        id: u64,
        date: &str,
        payout: i64,
    ) -> Result<ProductResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let (budget_id, name, category): (i64, String, String) = conn
            .query_row(
                "SELECT budget_id, name, category FROM products WHERE id = ?1 AND kind = 'deposit'",
                params![id as i64],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|_| "Deposit not found".to_string())?;

        conn.execute(
            "INSERT INTO items
                (budget_id, name, amount, category, date, description, link, item_type, completed, source_kind, source_id)
             VALUES (?1, ?2, ?3, ?4, ?5, 'Досрочное закрытие', '', 'income', 1, 'deposit', ?6)",
            params![budget_id, name, payout, category, date, id as i64],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE products SET status = 'closed' WHERE id = ?1",
            params![id as i64],
        )
        .map_err(|e| e.to_string())?;

        let budget = Self::query_budget(&conn, budget_id)?;
        let products = Self::query_products(&conn)?;
        Ok(ProductResult { budget, products })
    }

    pub fn create_budget(&self, name: &str, limit: i64, icon: &str) -> Result<Budget, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO budgets (name, budget_limit, icon) VALUES (?1, ?2, ?3)",
            params![name, limit, icon],
        )
        .map_err(|e| e.to_string())?;
        let id = conn.last_insert_rowid();
        Self::query_budget(&conn, id)
    }

    pub fn update_budget(
        &self,
        budget_id: u64,
        name: &str,
        limit: i64,
        icon: &str,
    ) -> Result<Budget, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rows = conn
            .execute(
                "UPDATE budgets SET name=?1, budget_limit=?2, icon=?3 WHERE id=?4",
                params![name, limit, icon, budget_id as i64],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Budget not found".into());
        }
        Self::query_budget(&conn, budget_id as i64)
    }

    pub fn delete_budget(&self, budget_id: u64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Prevent deleting the last budget
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM budgets", [], |r| r.get(0))
            .unwrap_or(0);
        if count <= 1 {
            return Err("Cannot delete the last budget".into());
        }

        // CASCADE will delete items
        conn.execute(
            "DELETE FROM budgets WHERE id = ?1",
            params![budget_id as i64],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
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
        conn.execute(
            "INSERT INTO categories (key, name, icon, color, is_default) VALUES (?1, ?2, ?3, ?4, 0)",
            params![key, name, icon, color],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "Category key already exists".to_string()
            } else {
                e.to_string()
            }
        })?;
        Self::query_categories(&conn)
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
                "UPDATE categories SET name=?1, icon=?2, color=?3 WHERE key=?4",
                params![name, icon, color, key],
            )
            .map_err(|e| e.to_string())?;
        if rows == 0 {
            return Err("Category not found".into());
        }
        Self::query_categories(&conn)
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

        // Date stamp
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let days = secs / 86400;
        // Convert to YYYY-MM-DD
        let mut y = 1970u64;
        let mut rem = days;
        loop {
            let diy = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 366 } else { 365 };
            if rem < diy { break; }
            rem -= diy;
            y += 1;
        }
        let ml = [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let mut m = 0u64;
        for (i, &len) in ml.iter().enumerate() {
            let d = if i == 1 && leap { len + 1 } else { len };
            if rem < d { m = i as u64; break; }
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
                        .map(|n| n.starts_with("plyet-") && n.ends_with(".db"))
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

    pub fn export_csv(&self, budget_id: u64) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let budget = Self::query_budget(&conn, budget_id as i64)?;
        let categories = Self::query_categories(&conn)?;

        let mut csv = String::from("\u{FEFF}Дата,Название,Сумма,Тип,Категория,Куплено,Описание\n");
        for item in &budget.items {
            let cat_name = categories
                .iter()
                .find(|c| c.key == item.category)
                .map(|c| c.name.as_str())
                .unwrap_or(&item.category);
            let item_type_str = match item.item_type {
                ItemType::Income => "Доход",
                ItemType::Expense => "Расход",
            };
            // CSV escape: wrap fields in quotes, double internal quotes
            // Prefix formula-triggering characters to prevent CSV injection
            let name = Self::csv_safe(&item.name);
            let desc = Self::csv_safe(&item.description);
            let cat = Self::csv_safe(cat_name);
            // Amount is stored in kopecks, export as rubles with 2 decimal places
            let amount_rub = format!("{:.2}", item.amount as f64 / 100.0);
            csv.push_str(&format!(
                "{},\"{}\",{},{},\"{}\",{},\"{}\"\n",
                item.date,
                name,
                amount_rub,
                item_type_str,
                cat,
                if item.completed { "Да" } else { "Нет" },
                desc,
            ));
        }
        Ok(csv)
    }

    pub fn export_json(&self) -> Result<String, String> {
        let data = self.load_all()?;
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
    }

    pub fn import_json(&self, json: &str) -> Result<AppData, String> {
        let import: AppData = serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {}", e))?;

        // Validate size limits
        if import.budgets.len() > 1000 {
            return Err("Слишком много бюджетов (макс. 1000)".into());
        }
        if import.categories.len() > 500 {
            return Err("Слишком много категорий (макс. 500)".into());
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
        }
        for budget in &import.budgets {
            validate_name(&budget.name)?;
            validate_amount(budget.limit)?;
            validate_icon(&budget.icon)?;
            for item in &budget.items {
                validate_name(&item.name)?;
                validate_amount(item.amount)?;
                validate_date(&item.date)?;
                validate_description(&item.description)?;
                validate_link(&item.link)?;
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
            "DELETE FROM products; DELETE FROM recurring; DELETE FROM items; DELETE FROM budgets; DELETE FROM categories;",
        )
        .map_err(|e| e.to_string())?;

        // Import categories
        for cat in &import.categories {
            tx.execute(
                "INSERT OR REPLACE INTO categories (key, name, icon, color, is_default) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![cat.key, cat.name, cat.icon, cat.color, cat.is_default as i32],
            ).map_err(|e| e.to_string())?;
        }

        // Import budgets and items
        for budget in &import.budgets {
            tx.execute(
                "INSERT INTO budgets (id, name, budget_limit, icon) VALUES (?1, ?2, ?3, ?4)",
                params![budget.id as i64, budget.name, budget.limit, budget.icon],
            ).map_err(|e| e.to_string())?;

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
                    "INSERT INTO items (id, budget_id, name, amount, category, date, description, link, item_type, completed, source_kind, source_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
                    ],
                ).map_err(|e| e.to_string())?;
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
                     start_date, end_date, horizon, last_paid_date, description, link)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
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
                     payments_made, manual_payment, down_payment, description, link)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
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
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())?;

        // Re-ensure defaults
        drop(conn);
        self.ensure_defaults()?;
        self.load_all()
    }

    pub fn delete_category(&self, key: &str) -> Result<Vec<Category>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // "other" is mandatory and cannot be deleted
        if key == "other" {
            return Err("Cannot delete the 'Другое' category".into());
        }

        // Verify category exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM categories WHERE key = ?1",
                params![key],
                |r| r.get::<_, i32>(0).map(|v| v > 0),
            )
            .map_err(|e| e.to_string())?;

        if !exists {
            return Err("Category not found".into());
        }

        // Reassign items to "other"
        conn.execute(
            "UPDATE items SET category = 'other' WHERE category = ?1",
            params![key],
        )
        .map_err(|e| e.to_string())?;

        conn.execute("DELETE FROM categories WHERE key = ?1", params![key])
            .map_err(|e| e.to_string())?;

        Self::query_categories(&conn)
    }
}

fn seed_budget_name() -> String {
    let months = [
        "Январь", "Февраль", "Март", "Апрель", "Май", "Июнь",
        "Июль", "Август", "Сентябрь", "Октябрь", "Ноябрь", "Декабрь",
    ];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let days_since_epoch = (now.as_secs() / 86400) as i64;
    let mut y = 1970i64;
    let mut remaining = days_since_epoch;
    loop {
        let days_in_year = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
            366
        } else {
            365
        };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let is_leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let mut m = 0usize;
    for (i, &ml) in month_lengths.iter().enumerate() {
        let days = if i == 1 && is_leap { ml + 1 } else { ml };
        if remaining < days {
            m = i;
            break;
        }
        remaining -= days;
        m = i;
    }
    format!("{} {}", months[m], y)
}
