mod data;
mod db;
mod validate;

use data::*;
use db::Database;
use tauri::{Manager, State};
use validate::*;

#[tauri::command]
fn load_data(state: State<Database>) -> Result<AppData, String> {
    state.load_all()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn add_item(
    state: State<Database>,
    budget_id: u64,
    name: String,
    amount: i64,
    category: String,
    date: String,
    description: String,
    link: String,
    item_type: String,
) -> Result<Vec<Budget>, String> {
    validate_name(&name)?;
    validate_amount(amount)?;
    validate_date(&date)?;
    validate_description(&description)?;
    validate_link(&link)?;
    validate_item_type(&item_type)?;
    state.add_item(
        budget_id,
        &name,
        amount,
        &category,
        &date,
        &description,
        &link,
        &item_type,
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn update_item(
    state: State<Database>,
    item_id: u64,
    name: String,
    amount: i64,
    category: String,
    date: String,
    description: String,
    link: String,
    item_type: String,
) -> Result<Vec<Budget>, String> {
    validate_name(&name)?;
    validate_amount(amount)?;
    validate_date(&date)?;
    validate_description(&description)?;
    validate_link(&link)?;
    validate_item_type(&item_type)?;
    state.update_item(
        item_id,
        &name,
        amount,
        &category,
        &date,
        &description,
        &link,
        &item_type,
    )
}

#[tauri::command]
fn delete_item(state: State<Database>, item_id: u64) -> Result<Vec<Budget>, String> {
    state.delete_item(item_id)
}

#[tauri::command]
fn toggle_completed(state: State<Database>, item_id: u64) -> Result<Vec<Budget>, String> {
    state.toggle_completed(item_id)
}

#[tauri::command]
fn unmaterialize_item(state: State<Database>, item_id: u64) -> Result<UnmaterializeResult, String> {
    state.unmaterialize_item(item_id)
}

#[tauri::command]
fn add_recurring(
    state: State<Database>,
    budget_id: u64,
    name: String,
    amount: i64,
    category: String,
    item_type: String,
    freq: String,
    anchor_day: i64,
    start_date: String,
    end_date: Option<String>,
    horizon: i64,
    description: String,
    link: String,
) -> Result<Vec<Recurring>, String> {
    validate_name(&name)?;
    validate_amount(amount)?;
    validate_item_type(&item_type)?;
    validate_freq(&freq)?;
    validate_anchor_day(anchor_day)?;
    validate_date(&start_date)?;
    if let Some(ed) = &end_date {
        validate_date(ed)?;
    }
    validate_horizon(horizon)?;
    validate_description(&description)?;
    validate_link(&link)?;
    state.add_recurring(
        budget_id,
        &name,
        amount,
        &category,
        &item_type,
        &freq,
        anchor_day,
        &start_date,
        end_date.as_deref(),
        horizon,
        &description,
        &link,
    )
}

#[tauri::command]
fn update_recurring(
    state: State<Database>,
    id: u64,
    name: String,
    amount: i64,
    category: String,
    item_type: String,
    freq: String,
    anchor_day: i64,
    start_date: String,
    end_date: Option<String>,
    horizon: i64,
    description: String,
    link: String,
) -> Result<Vec<Recurring>, String> {
    validate_name(&name)?;
    validate_amount(amount)?;
    validate_item_type(&item_type)?;
    validate_freq(&freq)?;
    validate_anchor_day(anchor_day)?;
    validate_date(&start_date)?;
    if let Some(ed) = &end_date {
        validate_date(ed)?;
    }
    validate_horizon(horizon)?;
    validate_description(&description)?;
    validate_link(&link)?;
    state.update_recurring(
        id,
        &name,
        amount,
        &category,
        &item_type,
        &freq,
        anchor_day,
        &start_date,
        end_date.as_deref(),
        horizon,
        &description,
        &link,
    )
}

#[tauri::command]
fn delete_recurring(state: State<Database>, id: u64) -> Result<Vec<Recurring>, String> {
    state.delete_recurring(id)
}

#[tauri::command]
fn materialize_recurring(
    state: State<Database>,
    id: u64,
    date: String,
) -> Result<MaterializeResult, String> {
    validate_date(&date)?;
    state.materialize_recurring(id, &date)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn add_product(
    state: State<Database>,
    budget_id: u64,
    kind: String,
    name: String,
    principal: i64,
    annual_rate_bps: i64,
    term_months: i64,
    start_date: String,
    payment_model: String,
    early_rate_bps: Option<i64>,
    horizon: i64,
    category: String,
    down_payment: i64,
    description: String,
    link: String,
) -> Result<ProductResult, String> {
    validate_product_kind(&kind)?;
    validate_name(&name)?;
    validate_amount(principal)?;
    validate_amount(down_payment)?;
    validate_rate(annual_rate_bps)?;
    if let Some(er) = early_rate_bps {
        validate_rate(er)?;
    }
    validate_term(term_months)?;
    validate_date(&start_date)?;
    validate_payment_model(&payment_model)?;
    validate_horizon(horizon)?;
    validate_description(&description)?;
    validate_link(&link)?;
    state.add_product(
        budget_id,
        &kind,
        &name,
        principal,
        annual_rate_bps,
        term_months,
        &start_date,
        &payment_model,
        early_rate_bps,
        horizon,
        &category,
        down_payment,
        &description,
        &link,
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn update_product(
    state: State<Database>,
    id: u64,
    name: String,
    principal: i64,
    annual_rate_bps: i64,
    term_months: i64,
    start_date: String,
    payment_model: String,
    early_rate_bps: Option<i64>,
    horizon: i64,
    category: String,
    down_payment: i64,
    description: String,
    link: String,
) -> Result<Vec<Product>, String> {
    validate_name(&name)?;
    validate_amount(principal)?;
    validate_amount(down_payment)?;
    validate_rate(annual_rate_bps)?;
    if let Some(er) = early_rate_bps {
        validate_rate(er)?;
    }
    validate_term(term_months)?;
    validate_date(&start_date)?;
    validate_payment_model(&payment_model)?;
    validate_horizon(horizon)?;
    validate_description(&description)?;
    validate_link(&link)?;
    state.update_product(
        id,
        &name,
        principal,
        annual_rate_bps,
        term_months,
        &start_date,
        &payment_model,
        early_rate_bps,
        horizon,
        &category,
        down_payment,
        &description,
        &link,
    )
}

#[tauri::command]
fn delete_product(state: State<Database>, id: u64) -> Result<Vec<Product>, String> {
    state.delete_product(id)
}

#[tauri::command]
fn materialize_product(
    state: State<Database>,
    id: u64,
    date: String,
    amount: i64,
    principal_portion: i64,
    item_type: String,
    closes: bool,
) -> Result<ProductResult, String> {
    validate_date(&date)?;
    validate_amount(amount)?;
    validate_amount(principal_portion.max(0))?;
    validate_item_type(&item_type)?;
    state.materialize_product(id, &date, amount, principal_portion, &item_type, closes)
}

#[tauri::command]
fn loan_extra_payment(
    state: State<Database>,
    id: u64,
    amount: i64,
    date: String,
) -> Result<ProductResult, String> {
    validate_amount(amount)?;
    validate_date(&date)?;
    state.loan_extra_payment(id, amount, &date)
}

#[tauri::command]
fn close_deposit(
    state: State<Database>,
    id: u64,
    date: String,
    payout: i64,
) -> Result<ProductResult, String> {
    validate_date(&date)?;
    validate_amount(payout)?;
    state.close_deposit(id, &date, payout)
}

#[tauri::command]
fn create_budget(
    state: State<Database>,
    name: String,
    limit: i64,
    icon: String,
) -> Result<Vec<Budget>, String> {
    validate_name(&name)?;
    validate_amount(limit)?;
    validate_icon(&icon)?;
    state.create_budget(&name, limit, &icon)
}

#[tauri::command]
fn create_sub_budget(
    state: State<Database>,
    root_id: u64,
    name: String,
    limit: i64,
    icon: String,
    reflect_in_months: bool,
) -> Result<Vec<Budget>, String> {
    validate_name(&name)?;
    validate_amount(limit)?;
    validate_icon(&icon)?;
    state.create_sub_budget(root_id, &name, limit, &icon, reflect_in_months)
}

#[tauri::command]
fn update_budget(
    state: State<Database>,
    budget_id: u64,
    name: String,
    limit: i64,
    icon: String,
    reflect_in_months: bool,
) -> Result<Vec<Budget>, String> {
    validate_name(&name)?;
    validate_amount(limit)?;
    validate_icon(&icon)?;
    state.update_budget(budget_id, &name, limit, &icon, reflect_in_months)
}

#[tauri::command]
fn delete_budget(state: State<Database>, budget_id: u64) -> Result<Vec<Budget>, String> {
    state.delete_budget(budget_id)
}

#[tauri::command]
fn ensure_month_budget(
    state: State<Database>,
    root_id: u64,
    period: String,
) -> Result<Vec<Budget>, String> {
    validate_period(&period)?;
    state.ensure_month_budget_cmd(root_id, &period)
}

#[tauri::command]
fn add_category(
    state: State<Database>,
    key: String,
    name: String,
    icon: String,
    color: String,
) -> Result<Vec<Category>, String> {
    validate_category_key(&key)?;
    validate_name(&name)?;
    validate_icon(&icon)?;
    validate_color(&color)?;
    state.add_category(&key, &name, &icon, &color)
}

#[tauri::command]
fn update_category(
    state: State<Database>,
    key: String,
    name: String,
    icon: String,
    color: String,
) -> Result<Vec<Category>, String> {
    validate_category_key(&key)?;
    validate_name(&name)?;
    validate_icon(&icon)?;
    validate_color(&color)?;
    state.update_category(&key, &name, &icon, &color)
}

#[tauri::command]
fn delete_category(state: State<Database>, key: String) -> Result<Vec<Category>, String> {
    state.delete_category(&key)
}

#[tauri::command]
fn export_csv(state: State<Database>, budget_id: u64) -> Result<String, String> {
    state.export_csv(budget_id)
}

#[tauri::command]
fn export_json(state: State<Database>) -> Result<String, String> {
    state.export_json()
}

#[tauri::command]
fn import_json(state: State<Database>, json: String) -> Result<AppData, String> {
    state.import_json(&json)
}

#[tauri::command]
fn request_exit(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());

    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_haptics::init());

    builder
        .setup(|app| {
            let data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            let database =
                Database::open(&data_dir).map_err(|e| Box::<dyn std::error::Error>::from(e))?;
            app.manage(database);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_data,
            add_item,
            update_item,
            delete_item,
            toggle_completed,
            unmaterialize_item,
            add_recurring,
            update_recurring,
            delete_recurring,
            materialize_recurring,
            add_product,
            update_product,
            delete_product,
            materialize_product,
            loan_extra_payment,
            close_deposit,
            create_budget,
            create_sub_budget,
            update_budget,
            delete_budget,
            ensure_month_budget,
            add_category,
            update_category,
            delete_category,
            export_csv,
            export_json,
            import_json,
            request_exit,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, _| {});
}
