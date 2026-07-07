use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentIntentDetailQuery {
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub payment_intent_id: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateOwnerPaymentIntentCommand {
    pub idempotency_key: String,
    pub order_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub payment_method: String,
    pub request_no: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancelOwnerPaymentIntentCommand {
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub payment_intent_id: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateOwnerPaymentAttemptCommand {
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub payment_intent_id: String,
    pub request_no: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentIntentView {
    pub amount: CommerceMoney,
    pub currency_code: String,
    pub order_id: String,
    pub payment_intent_id: String,
    pub payment_intent_no: String,
    pub payment_method: String,
    pub provider_code: String,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateOwnerPaymentAttemptOutcome {
    pub amount: CommerceMoney,
    pub attempt_id: String,
    pub order_id: String,
    pub out_trade_no: String,
    pub payment_intent_id: String,
    pub payment_method: String,
    pub provider_code: String,
    pub status: String,
    pub payment_params: std::collections::BTreeMap<String, String>,
}

impl PaymentIntentDetailQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        payment_intent_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("payment_intent_id", payment_intent_id)?;

        Ok(Self {
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            payment_intent_id: payment_intent_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl CreateOwnerPaymentIntentCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        payment_method: &str,
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
            idempotency_key: idempotency_key.trim().to_string(),
            order_id: order_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            payment_method: payment_method.trim().to_ascii_lowercase(),
            request_no: request_no.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl CancelOwnerPaymentIntentCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        payment_intent_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("payment_intent_id", payment_intent_id)?;

        Ok(Self {
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            payment_intent_id: payment_intent_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl CreateOwnerPaymentAttemptCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        payment_intent_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("payment_intent_id", payment_intent_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            idempotency_key: idempotency_key.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            payment_intent_id: payment_intent_id.trim().to_string(),
            request_no: request_no.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
