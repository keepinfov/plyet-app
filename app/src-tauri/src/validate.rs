/// Input validation for IPC commands.
/// Prevents oversized strings, invalid formats, and bad data from reaching the database.

pub fn validate_name(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if name.len() > 200 {
        return Err("Name is too long (max 200 characters)".into());
    }
    Ok(())
}

pub fn validate_description(desc: &str) -> Result<(), String> {
    if desc.len() > 2000 {
        return Err("Description is too long (max 2000 characters)".into());
    }
    Ok(())
}

pub fn validate_link(link: &str) -> Result<(), String> {
    if link.len() > 2000 {
        return Err("Link is too long (max 2000 characters)".into());
    }
    Ok(())
}

pub fn validate_date(date: &str) -> Result<(), String> {
    if date.len() != 10 {
        return Err("Invalid date format (expected YYYY-MM-DD)".into());
    }
    let bytes = date.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err("Invalid date format (expected YYYY-MM-DD)".into());
    }
    let year: u32 = date[0..4].parse().map_err(|_| "Invalid year in date")?;
    let month: u32 = date[5..7].parse().map_err(|_| "Invalid month in date")?;
    let day: u32 = date[8..10].parse().map_err(|_| "Invalid day in date")?;
    if year < 1970 || year > 2100 {
        return Err("Year out of range (1970-2100)".into());
    }
    if month < 1 || month > 12 {
        return Err("Month out of range (1-12)".into());
    }
    if day < 1 {
        return Err("Day out of range".into());
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => unreachable!(),
    };
    if day > max_day {
        return Err("Day out of range for this month".into());
    }
    Ok(())
}

pub fn validate_item_type(item_type: &str) -> Result<(), String> {
    match item_type {
        "expense" | "income" => Ok(()),
        _ => Err("Invalid item type (must be 'expense' or 'income')".into()),
    }
}

pub fn validate_color(color: &str) -> Result<(), String> {
    if color.len() > 20 {
        return Err("Color value is too long".into());
    }
    // Accept #RGB, #RRGGBB, or #RRGGBBAA hex formats
    let valid = color.len() == 4 || color.len() == 7 || color.len() == 9;
    if !valid || !color.starts_with('#') || !color[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid color format (expected #RRGGBB)".into());
    }
    Ok(())
}

pub fn validate_icon(icon: &str) -> Result<(), String> {
    if icon.len() > 50 {
        return Err("Icon key is too long (max 50 characters)".into());
    }
    // Icon keys should be alphanumeric with underscores only
    if !icon.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Invalid icon key".into());
    }
    Ok(())
}

pub fn validate_category_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("Category key cannot be empty".into());
    }
    if key.len() > 50 {
        return Err("Category key is too long (max 50 characters)".into());
    }
    Ok(())
}

pub fn validate_amount(amount: i64) -> Result<(), String> {
    if amount < 0 {
        return Err("Amount cannot be negative".into());
    }
    // Max ~10 billion rubles (in kopecks)
    if amount > 1_000_000_000_000 {
        return Err("Amount is too large".into());
    }
    Ok(())
}

pub fn validate_freq(freq: &str) -> Result<(), String> {
    match freq {
        "weekly" | "monthly" | "yearly" => Ok(()),
        _ => Err("Invalid frequency (must be 'weekly', 'monthly' or 'yearly')".into()),
    }
}

pub fn validate_horizon(horizon: i64) -> Result<(), String> {
    if horizon < 1 || horizon > 24 {
        return Err("Horizon out of range (1-24)".into());
    }
    Ok(())
}

pub fn validate_anchor_day(day: i64) -> Result<(), String> {
    // Weekly uses 0-6 (Mon-Sun); monthly/yearly use 1-31. Accept the union.
    if day < 0 || day > 31 {
        return Err("Anchor day out of range".into());
    }
    Ok(())
}

pub fn validate_product_kind(kind: &str) -> Result<(), String> {
    match kind {
        "deposit" | "loan" | "mortgage" => Ok(()),
        _ => Err("Invalid product kind (must be 'deposit', 'loan' or 'mortgage')".into()),
    }
}

pub fn validate_payment_model(model: &str) -> Result<(), String> {
    match model {
        "annuity" | "simple" | "manual" | "capitalized" => Ok(()),
        _ => Err("Invalid payment model".into()),
    }
}

pub fn validate_product_status(status: &str) -> Result<(), String> {
    match status {
        "active" | "closed" => Ok(()),
        _ => Err("Invalid product status".into()),
    }
}

pub fn validate_rate(bps: i64) -> Result<(), String> {
    // 0%..=1000% in basis points.
    if bps < 0 || bps > 1_000_000 {
        return Err("Rate out of range (0-1000%)".into());
    }
    Ok(())
}

pub fn validate_term(months: i64) -> Result<(), String> {
    // 1 month .. 100 years.
    if months < 1 || months > 1200 {
        return Err("Term out of range (1-1200 months)".into());
    }
    Ok(())
}

pub fn validate_period(period: &str) -> Result<(), String> {
    // A month key: YYYY-MM, same year range as validate_date.
    if period.len() != 7 || period.as_bytes()[4] != b'-' {
        return Err("Invalid period format (expected YYYY-MM)".into());
    }
    let year: u32 = period[0..4].parse().map_err(|_| "Invalid year in period")?;
    let month: u32 = period[5..7]
        .parse()
        .map_err(|_| "Invalid month in period")?;
    if year < 1970 || year > 2100 {
        return Err("Year out of range (1970-2100)".into());
    }
    if month < 1 || month > 12 {
        return Err("Month out of range (1-12)".into());
    }
    Ok(())
}

pub fn validate_budget_kind(kind: &str) -> Result<(), String> {
    match kind {
        "root" | "month" | "custom" => Ok(()),
        _ => Err("Invalid budget kind (must be 'root', 'month' or 'custom')".into()),
    }
}

pub fn validate_uuid_field(uuid: &str) -> Result<(), String> {
    // Empty is allowed on import (backfilled); otherwise a sane opaque id.
    if uuid.len() > 64 {
        return Err("UUID is too long (max 64 characters)".into());
    }
    if !uuid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("Invalid UUID format".into());
    }
    Ok(())
}

pub fn validate_timestamp_field(ts: &str) -> Result<(), String> {
    // Empty is allowed on import (backfilled); otherwise an ISO8601-ish string.
    if ts.len() > 40 {
        return Err("Timestamp is too long".into());
    }
    Ok(())
}

pub fn validate_member_role(role: &str) -> Result<(), String> {
    match role {
        "owner" | "editor" | "viewer" => Ok(()),
        _ => Err("Invalid member role".into()),
    }
}
