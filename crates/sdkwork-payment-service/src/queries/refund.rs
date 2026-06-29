use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundDetailQuery {
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub refund_id: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundListQuery {
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub status: Option<String>,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateOwnerRefundCommand {
    pub amount: Option<String>,
    pub idempotency_key: String,
    pub order_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub payment_attempt_id: Option<String>,
    pub reason_code: Option<String>,
    pub request_no: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundView {
    pub amount: CommerceMoney,
    pub currency_code: String,
    pub order_id: String,
    pub payment_attempt_id: String,
    pub reason_code: Option<String>,
    pub refund_id: String,
    pub refund_no: String,
    pub status: String,
}

impl RefundDetailQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        refund_id: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("refund_id", refund_id)?;

        Ok(Self {
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            refund_id: refund_id.trim().to_string(),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl RefundListQuery {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        status: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;

        Ok(Self {
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            status: optional_text(status),
            tenant_id: tenant_id.trim().to_string(),
        })
    }
}

impl CreateOwnerRefundCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
        payment_attempt_id: Option<&str>,
        amount: Option<&str>,
        reason_code: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            amount: optional_text(amount),
            idempotency_key: idempotency_key.trim().to_string(),
            order_id: order_id.trim().to_string(),
            organization_id: optional_text(organization_id),
            owner_user_id: owner_user_id.trim().to_string(),
            payment_attempt_id: optional_text(payment_attempt_id),
            reason_code: optional_text(reason_code),
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
