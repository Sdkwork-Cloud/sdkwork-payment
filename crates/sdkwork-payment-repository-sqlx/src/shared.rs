//! Shared utility functions for the commerce payment repository-sqlx crate.
//!
//! These helpers are used across both PostgreSQL and SQLite repository
//! implementations. Keeping them in a single module eliminates duplication
//! and ensures consistent behavior.

use chrono::{DateTime, SecondsFormat, Utc};
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

pub(crate) fn required_persisted_paid_at(paid_at: &str) -> Result<String, CommerceServiceError> {
    let paid_at = paid_at.trim();
    if paid_at.is_empty() {
        return Err(CommerceServiceError::storage(
            "succeeded payment attempt is missing persisted paid_at",
        ));
    }
    DateTime::parse_from_rfc3339(paid_at)
        .map(|_| paid_at.to_owned())
        .map_err(|_| {
            CommerceServiceError::storage(
                "succeeded payment attempt has a non-RFC3339 persisted paid_at",
            )
        })
}

pub(crate) fn ensure_confirmation_intent_update(
    rows_affected: u64,
    persisted_status: Option<&str>,
) -> Result<(), CommerceServiceError> {
    match rows_affected {
        1 => Ok(()),
        0 => {
            let Some(status) = persisted_status else {
                return Err(CommerceServiceError::storage(
                    "owner payment intent disappeared during confirmation",
                ));
            };
            if payment_attempt_is_terminal_success(status) {
                return Ok(());
            }
            ensure_payment_status_transition(status, "succeeded")?;
            Err(CommerceServiceError::storage(
                "owner payment intent was not updated despite a transitionable status",
            ))
        }
        count => Err(CommerceServiceError::storage(format!(
            "owner payment intent confirmation updated {count} rows; expected at most one"
        ))),
    }
}

pub(crate) fn resolve_confirmation_attempt_replayed(
    rows_affected: u64,
    persisted_status: Option<&str>,
) -> Result<bool, CommerceServiceError> {
    let Some(status) = persisted_status else {
        return Err(CommerceServiceError::storage(
            "owner payment attempt disappeared during confirmation",
        ));
    };

    match rows_affected {
        1 if payment_attempt_is_terminal_success(status) => Ok(false),
        0 if payment_attempt_is_terminal_success(status) => Ok(true),
        0 => {
            ensure_payment_status_transition(status, "succeeded")?;
            Err(CommerceServiceError::storage(
                "owner payment attempt was not updated despite a transitionable status",
            ))
        }
        1 => Err(CommerceServiceError::storage(
            "owner payment attempt update did not persist succeeded status",
        )),
        count => Err(CommerceServiceError::storage(format!(
            "owner payment attempt confirmation updated {count} rows; expected at most one"
        ))),
    }
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

/// Return the current UTC timestamp in the wire/storage RFC3339 format.
pub(crate) fn current_timestamp_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
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

#[cfg(test)]
mod tests {
    use super::{
        ensure_confirmation_intent_update, required_persisted_paid_at,
        resolve_confirmation_attempt_replayed,
    };

    #[test]
    fn confirmation_replay_requires_and_preserves_persisted_paid_at() {
        assert_eq!(
            required_persisted_paid_at("2026-07-12T01:02:03Z").expect("persisted paid_at"),
            "2026-07-12T01:02:03Z"
        );
        assert!(required_persisted_paid_at(" ").is_err());
    }

    #[test]
    fn confirmation_update_counts_distinguish_first_write_from_replay() {
        assert!(!resolve_confirmation_attempt_replayed(1, Some("succeeded"))
            .expect("first confirmation"));
        assert!(
            resolve_confirmation_attempt_replayed(0, Some("succeeded")).expect("concurrent replay")
        );
        assert!(resolve_confirmation_attempt_replayed(0, Some("pending")).is_err());
        assert!(resolve_confirmation_attempt_replayed(2, Some("succeeded")).is_err());

        ensure_confirmation_intent_update(1, None).expect("updated intent");
        ensure_confirmation_intent_update(0, Some("succeeded")).expect("intent replay");
        assert!(ensure_confirmation_intent_update(0, Some("pending")).is_err());
        assert!(ensure_confirmation_intent_update(0, None).is_err());
    }
}
