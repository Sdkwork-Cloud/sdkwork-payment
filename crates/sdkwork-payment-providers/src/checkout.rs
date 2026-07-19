use std::collections::BTreeMap;

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_payment_service::PayOwnerOrderOutcome;
use serde_json::{json, Value};

use crate::adapter::{
    normalize_provider_code, PaymentCreateIntentRequest, PaymentProviderOperationOutcome,
};
use crate::money::money_to_minor;
use crate::registry::PaymentProviderRegistry;

pub struct CheckoutContext {
    pub provider_code: String,
    pub currency_code: String,
    pub tenant_id: String,
    pub order_id: String,
    pub idempotency_key: String,
    pub notify_url: Option<String>,
    pub payment_scene: Option<String>,
    pub payment_metadata: Option<Value>,
}

pub async fn enrich_pay_owner_order_outcome(
    registry: &PaymentProviderRegistry,
    context: &CheckoutContext,
    mut outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let provider_code = normalize_provider_code(&context.provider_code);
    if provider_code == "sandbox" || provider_code.is_empty() {
        return Ok(outcome);
    }
    let adapter = registry.resolve(&provider_code).ok_or_else(|| {
        CommerceServiceError::provider_unavailable(format!(
            "payment provider {provider_code} is not configured"
        ))
    })?;

    let amount_minor = money_to_minor(&outcome.amount)?;
    let _notify_url = context
        .notify_url
        .clone()
        .or_else(|| registry.default_notify_url(&provider_code));
    let (provider_method_key, metadata) = provider_request_context(context, &outcome);

    let request = PaymentCreateIntentRequest {
        tenant_id: Some(context.tenant_id.clone()),
        merchant_order_no: Some(outcome.out_trade_no.clone()),
        amount_minor: Some(amount_minor),
        currency: Some(context.currency_code.clone()),
        payment_scene: Some(provider_method_key),
        metadata,
    };

    let provider_outcome = adapter.create_payment_intent(request).await?;
    let payment_params = payment_params_from_provider(&provider_code, &provider_outcome);
    outcome.payment_params.extend(payment_params);
    if let Some(url) = cashier_url_from_provider(&provider_code, &provider_outcome) {
        outcome.payment_params.insert("cashierUrl".to_owned(), url);
    }
    Ok(outcome)
}

fn provider_request_context(
    context: &CheckoutContext,
    outcome: &PayOwnerOrderOutcome,
) -> (String, Value) {
    let mut metadata = context
        .payment_metadata
        .clone()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    metadata["order_id"] = json!(context.order_id);
    metadata["idempotency_key"] = json!(context.idempotency_key);
    metadata["subject"] = json!(format!(
        "Order {}",
        outcome
            .payment_params
            .get("orderSn")
            .cloned()
            .unwrap_or_default()
    ));
    metadata["payment_scene"] = json!(context
        .payment_scene
        .clone()
        .unwrap_or_else(|| outcome.payment_method.clone()));
    (outcome.payment_method.clone(), metadata)
}

fn payment_params_from_provider(
    provider_code: &str,
    outcome: &PaymentProviderOperationOutcome,
) -> BTreeMap<String, String> {
    let mut params = BTreeMap::new();
    params.insert("providerCode".to_owned(), provider_code.to_owned());
    if let Some(native_id) = &outcome.native_id {
        params.insert("providerTransactionId".to_owned(), native_id.clone());
    }
    if let Some(status) = &outcome.raw_status {
        params.insert("providerStatus".to_owned(), status.clone());
    }
    match provider_code {
        "stripe" => {
            if let Some(secret) = outcome.payload.get("client_secret").and_then(Value::as_str) {
                params.insert("clientSecret".to_owned(), secret.to_owned());
            }
            params.insert("nextAction".to_owned(), "stripe_confirm".to_owned());
        }
        "alipay" => {
            if let Some(qr) = outcome.payload.get("qr_code").and_then(Value::as_str) {
                params.insert("qrCodeUrl".to_owned(), qr.to_owned());
                params.insert("nextAction".to_owned(), "qr_code".to_owned());
            }
        }
        "wechat_pay" => {
            if let Some(qr) = outcome.payload.get("code_url").and_then(Value::as_str) {
                params.insert("qrCodeUrl".to_owned(), qr.to_owned());
                params.insert("nextAction".to_owned(), "qr_code".to_owned());
            }
        }
        _ => {
            params.insert("nextAction".to_owned(), "cashier".to_owned());
        }
    }
    params
}

fn cashier_url_from_provider(
    provider_code: &str,
    outcome: &PaymentProviderOperationOutcome,
) -> Option<String> {
    match provider_code {
        "alipay" => outcome
            .payload
            .get("qr_code")
            .and_then(Value::as_str)
            .map(str::to_owned),
        "wechat_pay" => outcome
            .payload
            .get("code_url")
            .and_then(Value::as_str)
            .map(str::to_owned),
        "stripe" => outcome
            .native_id
            .as_ref()
            .map(|id| format!("stripe://payment_intent/{id}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{provider_request_context, CheckoutContext};
    use sdkwork_contract_service::CommerceMoney;
    use sdkwork_payment_service::PayOwnerOrderOutcome;

    #[test]
    fn provider_request_uses_method_key_and_preserves_payer_metadata() {
        let context = CheckoutContext {
            provider_code: "wechat_pay".to_owned(),
            currency_code: "CNY".to_owned(),
            tenant_id: "tenant-1".to_owned(),
            order_id: "order-1".to_owned(),
            idempotency_key: "idem-1".to_owned(),
            notify_url: Some("https://pay.example.test/webhook".to_owned()),
            payment_scene: Some("mini_program".to_owned()),
            payment_metadata: Some(serde_json::json!({"openid":"payer-openid"})),
        };
        let outcome = PayOwnerOrderOutcome {
            amount: CommerceMoney::new("100").expect("amount"),
            order_id: "order-1".to_owned(),
            out_trade_no: "trade-1".to_owned(),
            payment_id: "payment-1".to_owned(),
            payment_method: "wechat_jsapi".to_owned(),
            status: "pending".to_owned(),
            payment_params: Default::default(),
        };

        let (method_key, metadata) = provider_request_context(&context, &outcome);
        assert_eq!(method_key, "wechat_jsapi");
        assert_eq!(metadata["openid"], "payer-openid");
        assert_eq!(metadata["payment_scene"], "mini_program");
    }
}
