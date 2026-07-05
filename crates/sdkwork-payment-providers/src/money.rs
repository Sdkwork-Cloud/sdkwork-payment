use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

pub fn money_to_minor(amount: &CommerceMoney) -> Result<i64, CommerceServiceError> {
    let value = amount.as_str().trim();
    let negative = value.starts_with('-');
    let value = value.trim_start_matches('-');
    let (units, fraction) = value.split_once('.').unwrap_or((value, "0"));
    let units: i64 = units
        .parse()
        .map_err(|_| CommerceServiceError::validation("invalid money amount"))?;
    let mut frac = fraction.to_string();
    if frac.len() < 2 {
        frac.push_str(&"0".repeat(2 - frac.len()));
    }
    let cents: i64 = frac[..2]
        .parse()
        .map_err(|_| CommerceServiceError::validation("invalid money fraction"))?;
    let minor = units
        .checked_mul(100)
        .and_then(|base| base.checked_add(cents))
        .ok_or_else(|| CommerceServiceError::validation("money amount overflow"))?;
    Ok(if negative { -minor } else { minor })
}

pub fn minor_to_decimal_string(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let absolute = amount_minor.abs();
    format!("{sign}{}.{:02}", absolute / 100, absolute % 100)
}
