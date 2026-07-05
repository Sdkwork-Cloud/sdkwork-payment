use std::collections::BTreeMap;

use serde_json::{json, Value};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::PayOwnerOrderOutcome;

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
    let adapter = registry
        .resolve(&provider_code)
        .ok_or_else(|| CommerceServiceError::provider_unavailable(format!(
            "payment provider {provider_code} is not configured"
        )))?;

    let amount_minor = money_to_minor(&outcome.amount)?;
    let _notify_url = context
        .notify_url
        .clone()
        .or_else(|| registry.default_notify_url(&provider_code));
    let mut metadata = json!({
        "order_id": context.order_id,
        "idempotency_key": context.idempotency_key,
        "subject": format!("Order {}", outcome.payment_params.get("orderSn").cloned().unwrap_or_default()),
    });
    if let Some(scene) = context.payment_scene.as_deref() {
        metadata["payment_scene"] = json!(scene);
    }

    let request = PaymentCreateIntentRequest {
        tenant_id: Some(context.tenant_id.clone()),
        merchant_order_no: Some(outcome.out_trade_no.clone()),
        amount_minor: Some(amount_minor),
        currency: Some(context.currency_code.clone()),
        payment_scene: context.payment_scene.clone(),
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
