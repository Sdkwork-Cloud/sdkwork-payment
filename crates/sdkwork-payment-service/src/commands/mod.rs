use sdkwork_contract_service::CommerceMoney;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatePointsRechargeOrderCommand {
    pub amount: CommerceMoney,
    pub client_request_no: Option<String>,
    pub currency_code: String,
    pub expire_at: String,
    pub idempotency_key: String,
    pub method: String,
    pub order_id: String,
    pub order_item_id: String,
    pub order_no: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub package_id: Option<String>,
    pub payment_attempt_id: String,
    pub payment_intent_id: String,
    pub requested_at: String,
    pub source: Option<String>,
    pub tenant_id: String,
    pub out_trade_no: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatePaymentIntentCommand {
    pub amount: CommerceMoney,
    pub idempotency_key: String,
    pub order_id: String,
    pub payment_method: String,
    pub provider_code: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateRefundCommand {
    pub amount: CommerceMoney,
    pub idempotency_key: String,
    pub payment_id: String,
    pub request_no: String,
    pub tenant_id: String,
}

impl CreatePointsRechargeOrderCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        amount: CommerceMoney,
        currency_code: &str,
        method: &str,
        order_id: &str,
        order_item_id: &str,
        payment_intent_id: &str,
        payment_attempt_id: &str,
        order_no: &str,
        out_trade_no: &str,
        requested_at: &str,
        expire_at: &str,
        idempotency_key: &str,
        package_id: Option<&str>,
        client_request_no: Option<&str>,
        source: Option<&str>,
    ) -> Result<Self, sdkwork_contract_service::CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("currency_code", currency_code)?;
        crate::validation::require_non_empty("method", method)?;
        crate::validation::require_non_empty("order_id", order_id)?;
        crate::validation::require_non_empty("order_item_id", order_item_id)?;
        crate::validation::require_non_empty("payment_intent_id", payment_intent_id)?;
        crate::validation::require_non_empty("payment_attempt_id", payment_attempt_id)?;
        crate::validation::require_non_empty("order_no", order_no)?;
        crate::validation::require_non_empty("out_trade_no", out_trade_no)?;
        crate::validation::require_non_empty("requested_at", requested_at)?;
        crate::validation::require_non_empty("expire_at", expire_at)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            amount,
            client_request_no: optional_text(client_request_no),
            currency_code: currency_code.trim().to_ascii_uppercase(),
            expire_at: expire_at.trim().to_string(),
            idempotency_key: idempotency_key.trim().to_string(),
            method: method.trim().to_ascii_lowercase(),
            order_id: order_id.trim().to_string(),
            order_item_id: order_item_id.trim().to_string(),
            order_no: order_no.trim().to_string(),
            organization_id: organization_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            owner_user_id: owner_user_id.trim().to_string(),
            package_id: optional_text(package_id),
            payment_attempt_id: payment_attempt_id.trim().to_string(),
            payment_intent_id: payment_intent_id.trim().to_string(),
            requested_at: requested_at.trim().to_string(),
            source: optional_text(source),
            tenant_id: tenant_id.trim().to_string(),
            out_trade_no: out_trade_no.trim().to_string(),
        })
    }
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
