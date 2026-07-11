use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

pub fn money_to_minor(amount: &CommerceMoney) -> Result<i64, CommerceServiceError> {
    let value = amount.as_str().trim();
    if value.is_empty() || !value.chars().all(|character| character.is_ascii_digit()) {
        return Err(CommerceServiceError::validation(
            "money amount must be a non-negative integer smallest-unit amount",
        ));
    }
    value
        .parse()
        .map_err(|_| CommerceServiceError::validation("money amount overflow"))
}

pub fn minor_to_decimal_string(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let absolute = amount_minor.abs();
    format!("{sign}{}.{:02}", absolute / 100, absolute % 100)
}
