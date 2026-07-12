use sdkwork_contract_service::CommerceServiceError;
use sdkwork_utils_rust::sha256_hash;
use serde_json::{json, Value};

use crate::payment_attempt_context::PaymentWebhookAttemptIdentity;

pub(crate) const WEBHOOK_EVENT_STATUS_FAILED: &str = "failed";
pub(crate) const WEBHOOK_EVENT_STATUS_PROCESSED: &str = "processed";
pub(crate) const WEBHOOK_EVENT_STATUS_QUEUED: &str = "queued";
pub(crate) const WEBHOOK_MATCH_STATE_MATCHED: &str = "matched";
pub(crate) const WEBHOOK_MATCH_STATE_UNMATCHED: &str = "unmatched";

pub(crate) struct WebhookEventPayloadInput<'a> {
    pub provider_code: &'a str,
    pub provider_event_id: &'a str,
    pub provider_scoped_event_id: &'a str,
    pub event_type: Option<&'a str>,
    pub out_trade_no: Option<&'a str>,
    pub payment_status: Option<&'a str>,
    pub provider_payload: &'a Value,
    pub attempt_identity: Option<&'a PaymentWebhookAttemptIdentity>,
    pub unmatched_reason: Option<&'a str>,
}

pub(crate) struct WebhookEventInsert<'a> {
    pub internal_id: &'a str,
    pub tenant_id: &'a str,
    pub organization_id: Option<&'a str>,
    pub provider_scoped_event_id: &'a str,
    pub event_type: &'a str,
    pub provider_code: &'a str,
    pub payload_json: &'a str,
    pub status: &'a str,
    pub last_error: Option<&'a str>,
    pub now: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoredWebhookPayload {
    pub provider_code: String,
    pub provider_event_id: String,
    pub provider_scoped_event_id: String,
    pub out_trade_no: Option<String>,
    pub payment_status: Option<String>,
    pub match_state: String,
    pub attempt_identity: Option<PaymentWebhookAttemptIdentity>,
}

pub(crate) fn provider_scoped_webhook_event_id(
    provider_code: &str,
    provider_event_id: &str,
) -> String {
    stable_scoped_hash(
        "webhook-event-",
        &[
            &provider_code.trim().to_ascii_lowercase(),
            provider_event_id.trim(),
        ],
    )
}

pub(crate) fn webhook_event_storage_id(tenant_id: &str, provider_scoped_event_id: &str) -> String {
    stable_scoped_hash(
        "webhook-record-",
        &[tenant_id.trim(), provider_scoped_event_id],
    )
}

pub(crate) fn build_stored_webhook_payload(
    input: WebhookEventPayloadInput<'_>,
) -> Result<String, CommerceServiceError> {
    let match_state = if input.attempt_identity.is_some() {
        WEBHOOK_MATCH_STATE_MATCHED
    } else {
        WEBHOOK_MATCH_STATE_UNMATCHED
    };
    let attempt = input.attempt_identity.map(|identity| {
        json!({
            "paymentAttemptId": identity.payment_attempt_id,
            "paymentIntentId": identity.payment_intent_id,
            "providerCode": identity.provider_code,
            "outTradeNo": identity.out_trade_no,
            "attemptStatus": identity.attempt_status,
            "tenantId": identity.tenant_id,
            "organizationId": identity.organization_id,
            "ownerUserId": identity.owner_user_id,
            "orderId": identity.order_id,
        })
    });
    serde_json::to_string(&json!({
        "schemaVersion": 1,
        "normalized": {
            "providerCode": input.provider_code,
            "providerEventId": input.provider_event_id,
            "providerScopedEventId": input.provider_scoped_event_id,
            "eventType": input.event_type,
            "outTradeNo": input.out_trade_no,
            "paymentStatus": input.payment_status,
            "matchState": match_state,
            "unmatchedReason": input.unmatched_reason,
            "attempt": attempt,
        },
        "providerPayload": input.provider_payload,
    }))
    .map_err(|error| CommerceServiceError::storage(format!("webhook payload json: {error}")))
}

pub(crate) fn parse_stored_webhook_payload(
    payload: &str,
) -> Result<StoredWebhookPayload, CommerceServiceError> {
    let parsed: Value = serde_json::from_str(payload).map_err(|error| {
        CommerceServiceError::storage(format!("webhook payload json invalid: {error}"))
    })?;
    let normalized = parsed.get("normalized").ok_or_else(|| {
        CommerceServiceError::conflict("stored webhook payload has no normalized identity")
    })?;
    let provider_code = required_string(normalized, "providerCode")?;
    let provider_event_id = required_string(normalized, "providerEventId")?;
    let provider_scoped_event_id = required_string(normalized, "providerScopedEventId")?;
    let match_state = required_string(normalized, "matchState")?;
    if !matches!(
        match_state.as_str(),
        WEBHOOK_MATCH_STATE_MATCHED | WEBHOOK_MATCH_STATE_UNMATCHED
    ) {
        return Err(CommerceServiceError::conflict(
            "stored webhook payload has an invalid match state",
        ));
    }

    let attempt_identity = match normalized.get("attempt") {
        Some(Value::Object(attempt)) => Some(PaymentWebhookAttemptIdentity {
            payment_attempt_id: required_object_string(attempt, "paymentAttemptId")?,
            payment_intent_id: required_object_string(attempt, "paymentIntentId")?,
            provider_code: required_object_string(attempt, "providerCode")?,
            out_trade_no: required_object_string(attempt, "outTradeNo")?,
            attempt_status: required_object_string(attempt, "attemptStatus")?,
            tenant_id: required_object_string(attempt, "tenantId")?,
            organization_id: optional_object_string(attempt, "organizationId"),
            owner_user_id: required_object_string(attempt, "ownerUserId")?,
            order_id: required_object_string(attempt, "orderId")?,
        }),
        Some(Value::Null) | None => None,
        Some(_) => {
            return Err(CommerceServiceError::conflict(
                "stored webhook attempt identity is malformed",
            ));
        }
    };

    if match_state == WEBHOOK_MATCH_STATE_MATCHED && attempt_identity.is_none() {
        return Err(CommerceServiceError::conflict(
            "stored matched webhook has no exact payment attempt identity",
        ));
    }
    if match_state == WEBHOOK_MATCH_STATE_UNMATCHED && attempt_identity.is_some() {
        return Err(CommerceServiceError::conflict(
            "stored unmatched webhook unexpectedly contains an attempt identity",
        ));
    }

    Ok(StoredWebhookPayload {
        provider_code,
        provider_event_id,
        provider_scoped_event_id,
        out_trade_no: optional_string(normalized, "outTradeNo"),
        payment_status: optional_string(normalized, "paymentStatus"),
        match_state,
        attempt_identity,
    })
}

pub(crate) fn validate_stored_webhook_scope(
    payload: &StoredWebhookPayload,
    stored_event_id: &str,
    stored_provider_code: &str,
    tenant_id: &str,
    organization_id: Option<&str>,
) -> Result<(), CommerceServiceError> {
    let provider_code = stored_provider_code.trim().to_ascii_lowercase();
    if provider_code != payload.provider_code.trim().to_ascii_lowercase() {
        return Err(CommerceServiceError::conflict(
            "stored webhook provider identity does not match its row",
        ));
    }
    let expected_event_id =
        provider_scoped_webhook_event_id(&provider_code, &payload.provider_event_id);
    if expected_event_id != stored_event_id || expected_event_id != payload.provider_scoped_event_id
    {
        return Err(CommerceServiceError::conflict(
            "stored webhook provider-scoped event identity is invalid",
        ));
    }
    if let Some(identity) = payload.attempt_identity.as_ref() {
        if identity.provider_code.trim().to_ascii_lowercase() != provider_code
            || payload.out_trade_no.as_deref() != Some(identity.out_trade_no.as_str())
            || identity.tenant_id != tenant_id
            || identity.organization_id.as_deref() != organization_id
        {
            return Err(CommerceServiceError::conflict(
                "stored webhook attempt scope does not match its event row",
            ));
        }
    }
    Ok(())
}

fn stable_scoped_hash(prefix: &str, parts: &[&str]) -> String {
    let mut canonical = String::new();
    for part in parts {
        canonical.push_str(&part.len().to_string());
        canonical.push(':');
        canonical.push_str(part);
        canonical.push('|');
    }
    format!("{prefix}{}", sha256_hash(canonical.as_bytes()))
}

fn required_string(value: &Value, field: &str) -> Result<String, CommerceServiceError> {
    optional_string(value, field).ok_or_else(|| {
        CommerceServiceError::conflict(format!(
            "stored webhook payload is missing normalized.{field}"
        ))
    })
}

fn optional_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn required_object_string(
    value: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, CommerceServiceError> {
    optional_object_string(value, field).ok_or_else(|| {
        CommerceServiceError::conflict(format!(
            "stored webhook attempt identity is missing {field}"
        ))
    })
}

fn optional_object_string(value: &serde_json::Map<String, Value>, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        build_stored_webhook_payload, parse_stored_webhook_payload,
        provider_scoped_webhook_event_id, webhook_event_storage_id, WebhookEventPayloadInput,
        WEBHOOK_EVENT_STATUS_FAILED,
    };
    use crate::payment_attempt_context::PaymentWebhookAttemptIdentity;

    fn identity() -> PaymentWebhookAttemptIdentity {
        PaymentWebhookAttemptIdentity {
            payment_attempt_id: "attempt-1".to_owned(),
            payment_intent_id: "intent-1".to_owned(),
            provider_code: "stripe".to_owned(),
            out_trade_no: "trade-1".to_owned(),
            attempt_status: "pending".to_owned(),
            tenant_id: "tenant-1".to_owned(),
            organization_id: Some("org-1".to_owned()),
            owner_user_id: "user-1".to_owned(),
            order_id: "order-1".to_owned(),
        }
    }

    #[test]
    fn provider_and_tenant_scope_are_part_of_webhook_identity() {
        let stripe = provider_scoped_webhook_event_id("stripe", "evt-1");
        let wechat = provider_scoped_webhook_event_id("wechat_pay", "evt-1");
        assert_ne!(stripe, wechat);
        assert_eq!(stripe, provider_scoped_webhook_event_id("STRIPE", "evt-1"));
        assert_ne!(
            webhook_event_storage_id("tenant-1", &stripe),
            webhook_event_storage_id("tenant-2", &stripe)
        );
    }

    #[test]
    fn matched_payload_round_trips_exact_attempt_identity() {
        let identity = identity();
        let scoped_event_id = provider_scoped_webhook_event_id("stripe", "evt-1");
        let payload = build_stored_webhook_payload(WebhookEventPayloadInput {
            provider_code: "stripe",
            provider_event_id: "evt-1",
            provider_scoped_event_id: &scoped_event_id,
            event_type: Some("payment.succeeded"),
            out_trade_no: Some("trade-1"),
            payment_status: Some("succeeded"),
            provider_payload: &json!({"id": "evt-1"}),
            attempt_identity: Some(&identity),
            unmatched_reason: None,
        })
        .expect("build stored payload");

        let parsed = parse_stored_webhook_payload(&payload).expect("parse stored payload");
        assert_eq!(parsed.attempt_identity, Some(identity));
        assert_eq!(parsed.provider_scoped_event_id, scoped_event_id);
    }

    #[test]
    fn unmatched_status_is_legal_in_postgres_and_sqlite_baselines() {
        let postgres =
            include_str!("../../../database/ddl/baseline/postgres/0001_payment_baseline.sql");
        let sqlite =
            include_str!("../../../database/ddl/baseline/sqlite/0001_payment_baseline.sql");
        let legal_status_contract =
            "CHECK (status IN ('queued', 'processing', 'processed', 'failed', 'dead'))";

        assert!(postgres.contains(legal_status_contract));
        assert!(sqlite.contains(legal_status_contract));
        assert_eq!(WEBHOOK_EVENT_STATUS_FAILED, "failed");
        assert_ne!(WEBHOOK_EVENT_STATUS_FAILED, "unmatched");
    }
}
