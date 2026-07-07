//! Provider-side payment operations (cancel, refund) invoked from app handlers.

use sdkwork_contract_service::CommerceServiceError;
use serde_json::json;

use crate::adapter::{
    normalize_provider_code, PaymentCancelPaymentIntentRequest, PaymentCreateRefundRequest,
};
use crate::money::money_to_minor;
use crate::registry::PaymentProviderRegistry;
use sdkwork_contract_service::CommerceMoney;

pub async fn cancel_provider_payment(
    registry: &PaymentProviderRegistry,
    provider_code: &str,
    out_trade_no: &str,
    provider_transaction_id: Option<&str>,
) -> Result<(), CommerceServiceError> {
    let provider_code = normalize_provider_code(provider_code);
    if provider_code == "sandbox" || provider_code.is_empty() {
        return Ok(());
    }
    let adapter = registry.resolve(&provider_code).ok_or_else(|| {
        CommerceServiceError::provider_unavailable(format!(
            "payment provider {provider_code} is not configured"
        ))
    })?;
    let cancel_reference = match provider_code.as_str() {
        "stripe" => provider_transaction_id
            .filter(|value| value.starts_with("pi_"))
            .unwrap_or(out_trade_no),
        _ => out_trade_no,
    };
    adapter
        .cancel_payment_intent(PaymentCancelPaymentIntentRequest {
            payment_intent_id: Some(cancel_reference.to_owned()),
            reason: None,
            metadata: json!({}),
        })
        .await?;
    Ok(())
}

pub async fn create_provider_refund(
    registry: &PaymentProviderRegistry,
    provider_code: &str,
    out_trade_no: &str,
    refund_no: &str,
    refund_amount: &CommerceMoney,
    total_amount: &CommerceMoney,
    reason: Option<String>,
) -> Result<(), CommerceServiceError> {
    let provider_code = normalize_provider_code(provider_code);
    if provider_code == "sandbox" || provider_code.is_empty() {
        return Ok(());
    }
    let adapter = registry.resolve(&provider_code).ok_or_else(|| {
        CommerceServiceError::provider_unavailable(format!(
            "payment provider {provider_code} is not configured"
        ))
    })?;
    let amount_minor = money_to_minor(refund_amount)?;
    let total_amount_minor = money_to_minor(total_amount)?;
    adapter
        .create_refund(PaymentCreateRefundRequest {
            payment_intent_id: Some(out_trade_no.to_owned()),
            refund_no: Some(refund_no.to_owned()),
            amount_minor: Some(amount_minor),
            reason,
            metadata: json!({ "total_amount_minor": total_amount_minor }),
        })
        .await?;
    Ok(())
}
