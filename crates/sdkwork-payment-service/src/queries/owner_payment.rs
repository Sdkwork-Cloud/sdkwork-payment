use std::collections::BTreeMap;

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

/// Scoped lookup for payable-order validation inside the payment executor.
///
/// Payment reads `commerce_order` rows only as a foreign reference — order lifecycle
/// remains owned by `sdkwork-order`. Callers must not depend on order crates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderPaymentReferenceQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderPaymentReferenceSnapshot {
    pub order_id: String,
    pub order_sn: String,
    pub order_subject: Option<String>,
    pub status: String,
    pub total_amount: CommerceMoney,
    pub pay_time: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayOwnerOrderCommand {
    pub order_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub payment_method: String,
    pub payment_scene: Option<String>,
    pub payment_attempt_callback_payload: Option<String>,
    pub tenant_id: String,
    pub idempotency_key: String,
    pub request_no: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayOwnerOrderOutcome {
    pub amount: CommerceMoney,
    pub order_id: String,
    pub out_trade_no: String,
    pub payment_id: String,
    pub payment_method: String,
    pub status: String,
    pub payment_params: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancelOrderPaymentsCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
}

impl OrderPaymentReferenceQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            order_id: order_id.trim().to_string(),
        })
    }
}

impl PayOwnerOrderCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        payment_method: &str,
        payment_scene: Option<String>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Self::with_payment_attempt_callback_payload(
            tenant_id,
            organization_id,
            owner_user_id,
            order_id,
            payment_method,
            payment_scene,
            None,
            request_no,
            idempotency_key,
        )
    }

    pub fn with_payment_attempt_callback_payload(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        payment_method: &str,
        payment_scene: Option<String>,
        payment_attempt_callback_payload: Option<String>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("payment_method", payment_method)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            order_id: order_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            payment_method: payment_method.trim().to_ascii_lowercase(),
            payment_scene: payment_scene
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| !value.is_empty()),
            payment_attempt_callback_payload,
            tenant_id: tenant_id.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
            request_no: request_no.trim().to_string(),
        })
    }
}

impl CancelOrderPaymentsCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;

        Ok(Self {
            tenant_id: tenant_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            order_id: order_id.trim().to_string(),
        })
    }
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
