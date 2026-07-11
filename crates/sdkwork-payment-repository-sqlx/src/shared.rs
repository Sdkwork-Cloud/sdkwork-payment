//! Shared utility functions for the commerce payment repository-sqlx crate.
//!
//! These helpers are used across both PostgreSQL and SQLite repository
//! implementations. Keeping them in a single module eliminates duplication
//! and ensures consistent behavior.

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_payment_service::{
    validate_payment_wire_transition, validate_refund_wire_transition, CreateOwnerRefundCommand,
};
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

pub(crate) fn payment_attempt_is_terminal_success(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "succeeded" | "success" | "paid"
    )
}

pub(crate) fn ensure_payment_status_transition(
    from: &str,
    to: &str,
) -> Result<(), CommerceServiceError> {
    validate_payment_wire_transition(from, to)
}

pub(crate) fn ensure_refund_status_transition(
    from: Option<&str>,
    to: &str,
) -> Result<(), CommerceServiceError> {
    validate_refund_wire_transition(from, to)
}

/// Wrap a storage-layer error with a descriptive context message.
///
/// Accepts any `Display` type so it works uniformly with `sqlx::Error`,
/// `std::io::Error`, and other error types.
pub(crate) fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    CommerceServiceError::storage(format!("{message}: {error}"))
}

/// Produce a deterministic, filesystem-safe storage identifier from path parts.
///
/// Each part is sanitized: non-alphanumeric characters (except `-`, `_`, `.`)
/// are replaced with `-`, and parts are joined with `-`.
pub(crate) fn stable_storage_id(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|part| {
            part.chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                        character
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("-")
}

/// Return the current Unix timestamp as a string.
///
/// Uses `SystemTime` to avoid pulling in a heavy datetime dependency.
/// Returns `"0"` if the system clock is before `UNIX_EPOCH`.
pub(crate) fn current_timestamp_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}

/// Parse a money string into integer smallest currency units.
///
/// `CommerceMoney` is stored and exchanged as a non-negative integer string in
/// the smallest currency unit. For CNY/USD this means cents; for provider APIs
/// this value can be passed directly as the minor-unit amount.
///
/// # Errors
///
/// Returns a `validation` error if the value is empty, non-numeric, negative,
/// or overflows `i64`.
pub(crate) fn money_to_minor_units(value: &str) -> Result<i64, CommerceServiceError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CommerceServiceError::validation(
            "money amount must not be empty",
        ));
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err(CommerceServiceError::validation(
            "money amount must be a non-negative integer smallest-unit amount",
        ));
    }
    trimmed
        .parse::<i64>()
        .map_err(|_| CommerceServiceError::validation("money amount overflows i64 minor units"))
}

/// Resolve the refund amount string from the command or default to the order total.
pub(crate) fn resolve_refund_amount(
    command: &CreateOwnerRefundCommand,
    total_amount: &CommerceMoney,
) -> Result<String, CommerceServiceError> {
    Ok(command
        .amount
        .clone()
        .unwrap_or_else(|| total_amount.as_str().to_owned()))
}

/// Validate that the refund amount is positive and does not exceed the original payment.
pub(crate) fn validate_refund_bounds(
    refund_minor: i64,
    total_minor: i64,
) -> Result<(), CommerceServiceError> {
    if refund_minor <= 0 {
        return Err(CommerceServiceError::validation(
            "refund amount must be greater than zero",
        ));
    }
    if refund_minor > total_minor {
        return Err(CommerceServiceError::conflict(
            "refund amount exceeds original payment amount",
        ));
    }
    Ok(())
}

pub(crate) fn string_cell<R: StringCellRow>(row: &R, column: &str) -> String {
    row.string_cell(column)
}

pub(crate) trait StringCellRow {
    fn string_cell(&self, column: &str) -> String;
}

impl StringCellRow for SqliteRow {
    fn string_cell(&self, column: &str) -> String {
        self.try_get::<String, _>(column)
            .or_else(|_| self.try_get::<&str, _>(column).map(str::to_owned))
            .unwrap_or_default()
    }
}

impl StringCellRow for PgRow {
    fn string_cell(&self, column: &str) -> String {
        self.try_get::<Option<String>, _>(column)
            .ok()
            .flatten()
            .unwrap_or_default()
    }
}
