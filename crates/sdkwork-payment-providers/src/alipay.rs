use std::fmt;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Local;
use rsa::pkcs1v15::{Signature, SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde_json::{json, Value};
use sha2::Sha256;

use crate::adapter::{
    metadata_string, normalized_optional, require_non_empty, require_positive_amount,
    PaymentAdapterFuture, PaymentAdapterOperation, PaymentCancelPaymentIntentRequest,
    PaymentCreateIntentRequest, PaymentCreateRefundRequest, PaymentNormalizeWebhookRequest,
    PaymentNormalizedWebhookEvent, PaymentProviderAdapter, PaymentProviderCapabilities,
    PaymentProviderOperationOutcome, PaymentQueryPaymentIntentRequest, PaymentQueryRefundRequest,
    PaymentVerifyWebhookRequest, PaymentWebhookVerificationOutcome,
};
use crate::error::{ProviderError, ProviderResult};
use crate::http::ReqwestHttpClient;
use crate::money::minor_to_decimal_string;

const ALIPAY_PROVIDER_CODE: &str = "alipay";
const ALIPAY_GATEWAY_URL: &str = "https://openapi.alipay.com/gateway.do";
const ALIPAY_SANDBOX_GATEWAY_URL: &str = "https://openapi-sandbox.dl.alipaydev.com/gateway.do";

static ALIPAY_CAPABILITIES: PaymentProviderCapabilities = PaymentProviderCapabilities {
    provider_code: ALIPAY_PROVIDER_CODE,
    operations: &[
        PaymentAdapterOperation::CreatePaymentIntent,
        PaymentAdapterOperation::QueryPaymentIntent,
        PaymentAdapterOperation::CancelPaymentIntent,
        PaymentAdapterOperation::CreateRefund,
        PaymentAdapterOperation::QueryRefund,
        PaymentAdapterOperation::VerifyWebhook,
        PaymentAdapterOperation::NormalizeWebhook,
    ],
};

#[derive(Clone, PartialEq, Eq)]
pub struct AlipayPaymentProviderConfig {
    pub app_id: String,
    pub notify_url: Option<String>,
    pub return_url: Option<String>,
    pub sandbox: bool,
}

impl fmt::Debug for AlipayPaymentProviderConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AlipayPaymentProviderConfig")
            .field("app_id", &self.app_id)
            .field("notify_url", &self.notify_url)
            .field("return_url", &self.return_url)
            .field("sandbox", &self.sandbox)
            .finish()
    }
}

pub struct RsaAlipaySigner {
    signing_key: SigningKey<Sha256>,
    verifying_key: VerifyingKey<Sha256>,
}

impl RsaAlipaySigner {
    pub fn from_pkcs8_pem(
        private_key_pem: &str,
        alipay_public_key_pem: &str,
    ) -> ProviderResult<Self> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem).map_err(|error| {
            ProviderError::invalid_request(
                PaymentAdapterOperation::CreatePaymentIntent,
                format!("invalid Alipay merchant private key: {error}"),
            )
        })?;
        let public_key =
            RsaPublicKey::from_public_key_pem(alipay_public_key_pem).map_err(|error| {
                ProviderError::invalid_request(
                    PaymentAdapterOperation::VerifyWebhook,
                    format!("invalid Alipay platform public key: {error}"),
                )
            })?;
        Ok(Self {
            signing_key: SigningKey::<Sha256>::new(private_key),
            verifying_key: VerifyingKey::<Sha256>::new(public_key),
        })
    }

    pub fn sign(&self, payload: &str) -> ProviderResult<String> {
        let signature: Signature = self.signing_key.sign(payload.as_bytes());
        Ok(BASE64.encode(signature.to_bytes()))
    }

    pub fn verify(&self, payload: &str, signature: &str) -> ProviderResult<bool> {
        let decoded = BASE64.decode(signature).map_err(|error| {
            ProviderError::invalid_request(
                PaymentAdapterOperation::VerifyWebhook,
                format!("invalid Alipay signature encoding: {error}"),
            )
        })?;
        let signature = Signature::try_from(decoded.as_slice()).map_err(|error| {
            ProviderError::invalid_request(
                PaymentAdapterOperation::VerifyWebhook,
                format!("invalid Alipay signature: {error}"),
            )
        })?;
        Ok(self
            .verifying_key
            .verify(payload.as_bytes(), &signature)
            .is_ok())
    }
}

pub struct AlipayOpenApiClient {
    app_id: String,
    http: ReqwestHttpClient,
    pub(crate) signer: Arc<RsaAlipaySigner>,
}

impl AlipayOpenApiClient {
    pub fn new(
        config: &AlipayPaymentProviderConfig,
        signer: Arc<RsaAlipaySigner>,
    ) -> ProviderResult<Self> {
        validate_config_secret("app_id", &config.app_id)?;
        let gateway_url = if config.sandbox {
            ALIPAY_SANDBOX_GATEWAY_URL
        } else {
            ALIPAY_GATEWAY_URL
        };
        Ok(Self {
            app_id: config.app_id.clone(),
            http: ReqwestHttpClient::new(gateway_url)?,
            signer,
        })
    }

    async fn execute(&self, method: &str, biz_content: Value) -> ProviderResult<Value> {
        let mut params = vec![
            ("app_id".to_owned(), self.app_id.clone()),
            ("method".to_owned(), method.to_owned()),
            ("format".to_owned(), "JSON".to_owned()),
            ("charset".to_owned(), "utf-8".to_owned()),
            ("sign_type".to_owned(), "RSA2".to_owned()),
            ("timestamp".to_owned(), current_alipay_timestamp()),
            ("version".to_owned(), "1.0".to_owned()),
            (
                "biz_content".to_owned(),
                serde_json::to_string(&biz_content).map_err(|error| {
                    ProviderError::invalid_request(
                        PaymentAdapterOperation::CreatePaymentIntent,
                        format!("Alipay biz_content could not be serialized: {error}"),
                    )
                })?,
            ),
        ];
        let canonical = canonical_gateway_payload(&params);
        let signature = self.signer.sign(&canonical)?;
        params.push(("sign".to_owned(), signature));
        let response = self
            .http
            .post_form(ALIPAY_PROVIDER_CODE, "", params, None)
            .await?;
        alipay_response_payload(method, response)
    }
}

pub struct AlipayPaymentProviderAdapter {
    config: AlipayPaymentProviderConfig,
    client: AlipayOpenApiClient,
}

impl AlipayPaymentProviderAdapter {
    pub fn new(
        config: AlipayPaymentProviderConfig,
        signer: Arc<RsaAlipaySigner>,
    ) -> ProviderResult<Self> {
        validate_config_secret("app_id", &config.app_id)?;
        let client = AlipayOpenApiClient::new(&config, signer)?;
        Ok(Self { config, client })
    }
}

impl PaymentProviderAdapter for AlipayPaymentProviderAdapter {
    fn capabilities(&self) -> &'static PaymentProviderCapabilities {
        &ALIPAY_CAPABILITIES
    }

    fn create_payment_intent<'a>(
        &'a self,
        request: PaymentCreateIntentRequest,
    ) -> PaymentAdapterFuture<'a, PaymentProviderOperationOutcome> {
        Box::pin(async move {
            let out_trade_no = require_non_empty(
                request.merchant_order_no.as_deref(),
                PaymentAdapterOperation::CreatePaymentIntent,
                "merchant_order_no",
            )?;
            let amount_minor = require_positive_amount(
                request.amount_minor,
                PaymentAdapterOperation::CreatePaymentIntent,
                "amount_minor",
            )?;
            require_cny(
                request.currency.as_deref(),
                PaymentAdapterOperation::CreatePaymentIntent,
            )?;
            let subject = metadata_string(&request.metadata, "subject")
                .map(str::to_owned)
                .unwrap_or_else(|| out_trade_no.clone());
            let method_key = request
                .payment_scene
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("alipay_qr");
            let biz_content = build_alipay_biz_content(
                &out_trade_no,
                amount_minor,
                &subject,
                request.tenant_id.as_deref(),
                normalized_optional(self.config.notify_url.clone()).as_deref(),
                method_key,
                &request.metadata,
            )?;
            let (api_method, return_field) = alipay_method_for_key(method_key);
            let mut response = self.client.execute(api_method, biz_content).await?;
            // For redirect-style methods (page.pay/wap.pay), the gateway returns
            // a form HTML or URL in `body` rather than JSON fields. We surface
            // it under a normalized key so the cashier can render accordingly.
            if let Some(field) = return_field {
                if let Some(redirect) = response
                    .get(field)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                {
                    response["redirect_url"] = json!(redirect);
                }
            }
            alipay_operation_outcome(PaymentAdapterOperation::CreatePaymentIntent, response)
        })
    }

    fn query_payment_intent<'a>(
        &'a self,
        request: PaymentQueryPaymentIntentRequest,
    ) -> PaymentAdapterFuture<'a, PaymentProviderOperationOutcome> {
        Box::pin(async move {
            let out_trade_no = require_non_empty(
                request.payment_intent_id.as_deref(),
                PaymentAdapterOperation::QueryPaymentIntent,
                "payment_intent_id",
            )?;
            let response = self
                .client
                .execute(
                    "alipay.trade.query",
                    json!({
                        "out_trade_no": out_trade_no,
                    }),
                )
                .await?;
            alipay_operation_outcome(PaymentAdapterOperation::QueryPaymentIntent, response)
        })
    }

    fn cancel_payment_intent<'a>(
        &'a self,
        request: PaymentCancelPaymentIntentRequest,
    ) -> PaymentAdapterFuture<'a, PaymentProviderOperationOutcome> {
        Box::pin(async move {
            let out_trade_no = require_non_empty(
                request.payment_intent_id.as_deref(),
                PaymentAdapterOperation::CancelPaymentIntent,
                "payment_intent_id",
            )?;
            let response = self
                .client
                .execute(
                    "alipay.trade.close",
                    json!({
                        "out_trade_no": out_trade_no,
                    }),
                )
                .await?;
            alipay_operation_outcome(PaymentAdapterOperation::CancelPaymentIntent, response)
        })
    }

    fn create_refund<'a>(
        &'a self,
        request: PaymentCreateRefundRequest,
    ) -> PaymentAdapterFuture<'a, PaymentProviderOperationOutcome> {
        Box::pin(async move {
            let out_trade_no = require_non_empty(
                request.payment_intent_id.as_deref(),
                PaymentAdapterOperation::CreateRefund,
                "payment_intent_id",
            )?;
            let amount_minor = require_positive_amount(
                request.amount_minor,
                PaymentAdapterOperation::CreateRefund,
                "amount_minor",
            )?;
            let out_request_no = require_non_empty(
                request.refund_no.as_deref(),
                PaymentAdapterOperation::CreateRefund,
                "refund_no",
            )?;
            let mut biz_content = json!({
                "out_trade_no": out_trade_no,
                "refund_amount": minor_to_decimal_string(amount_minor),
                "out_request_no": out_request_no,
            });
            if let Some(reason) = normalized_optional(request.reason) {
                biz_content["refund_reason"] = json!(reason);
            }
            let response = self
                .client
                .execute("alipay.trade.refund", biz_content)
                .await?;
            alipay_operation_outcome(PaymentAdapterOperation::CreateRefund, response)
        })
    }

    fn query_refund<'a>(
        &'a self,
        request: PaymentQueryRefundRequest,
    ) -> PaymentAdapterFuture<'a, PaymentProviderOperationOutcome> {
        Box::pin(async move {
            let out_trade_no = require_non_empty(
                metadata_string(&request.metadata, "out_trade_no"),
                PaymentAdapterOperation::QueryRefund,
                "metadata.out_trade_no",
            )?;
            let out_request_no = require_non_empty(
                request.refund_no.as_deref(),
                PaymentAdapterOperation::QueryRefund,
                "refund_no",
            )?;
            let response = self
                .client
                .execute(
                    "alipay.trade.fastpay.refund.query",
                    json!({
                        "out_trade_no": out_trade_no,
                        "out_request_no": out_request_no,
                    }),
                )
                .await?;
            alipay_operation_outcome(PaymentAdapterOperation::QueryRefund, response)
        })
    }

    fn verify_webhook<'a>(
        &'a self,
        request: PaymentVerifyWebhookRequest,
    ) -> PaymentAdapterFuture<'a, PaymentWebhookVerificationOutcome> {
        let signer = self.client.signer.clone();
        Box::pin(async move {
            let fields = parse_form_body(&request.body, PaymentAdapterOperation::VerifyWebhook)?;
            let signature = form_value(&fields, "sign").ok_or_else(|| {
                ProviderError::invalid_request(
                    PaymentAdapterOperation::VerifyWebhook,
                    "Alipay webhook sign is required",
                )
            })?;
            let canonical = canonical_form_payload(&fields);
            let verified = signer.verify(&canonical, &signature)?;
            Ok(PaymentWebhookVerificationOutcome {
                verified,
                provider_event_id: if verified {
                    form_value(&fields, "notify_id")
                } else {
                    None
                },
            })
        })
    }

    fn normalize_webhook<'a>(
        &'a self,
        request: PaymentNormalizeWebhookRequest,
    ) -> PaymentAdapterFuture<'a, PaymentNormalizedWebhookEvent> {
        Box::pin(async move {
            let fields = parse_form_body(&request.body, PaymentAdapterOperation::NormalizeWebhook)?;
            let payload = form_fields_to_json(&fields);
            Ok(PaymentNormalizedWebhookEvent {
                provider_code: ALIPAY_PROVIDER_CODE.to_owned(),
                event_type: payload
                    .get("trade_status")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                provider_event_id: payload
                    .get("notify_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                out_trade_no: payload
                    .get("out_trade_no")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                payment_status: payload
                    .get("trade_status")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                payload,
            })
        })
    }
}

fn alipay_operation_outcome(
    operation: PaymentAdapterOperation,
    response: Value,
) -> ProviderResult<PaymentProviderOperationOutcome> {
    let native_id = response
        .get("trade_no")
        .and_then(Value::as_str)
        .or_else(|| response.get("out_trade_no").and_then(Value::as_str))
        .or_else(|| response.get("notify_id").and_then(Value::as_str))
        .map(str::to_owned)
        .ok_or_else(|| {
            ProviderError::invalid_response(operation, "Alipay response is missing trade id")
        })?;
    Ok(PaymentProviderOperationOutcome {
        provider_code: ALIPAY_PROVIDER_CODE.to_owned(),
        native_id: Some(native_id),
        raw_status: response
            .get("trade_status")
            .and_then(Value::as_str)
            .or_else(|| response.get("msg").and_then(Value::as_str))
            .map(str::to_owned),
        payload: response,
    })
}

fn alipay_response_payload(method: &str, payload: Value) -> ProviderResult<Value> {
    let response_key = format!("{}_response", method.replace('.', "_"));
    let response = payload.get(&response_key).cloned().unwrap_or(payload);
    let code = response
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("10000");
    if code != "10000" {
        let message = response
            .get("sub_msg")
            .or_else(|| response.get("msg"))
            .and_then(Value::as_str)
            .unwrap_or("Alipay request failed");
        return Err(ProviderError::transport(
            ALIPAY_PROVIDER_CODE,
            format!("Alipay {method} failed ({code}): {message}"),
        ));
    }
    Ok(response)
}

fn parse_form_body(
    body: &[u8],
    operation: PaymentAdapterOperation,
) -> ProviderResult<Vec<(String, String)>> {
    let body = std::str::from_utf8(body).map_err(|error| {
        ProviderError::invalid_response(
            operation,
            format!("Alipay form body must be UTF-8: {error}"),
        )
    })?;
    body.split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            Ok((
                percent_decode(key, operation)?,
                percent_decode(value, operation)?,
            ))
        })
        .collect()
}

fn form_value(fields: &[(String, String)], key: &str) -> Option<String> {
    fields
        .iter()
        .find(|(field_key, _)| field_key == key)
        .map(|(_, value)| value.to_owned())
}

fn canonical_form_payload(fields: &[(String, String)]) -> String {
    let mut pairs = fields
        .iter()
        .filter(|(key, value)| key != "sign" && key != "sign_type" && !value.is_empty())
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<Vec<_>>();
    pairs.sort_unstable_by(|left, right| left.0.cmp(right.0));
    pairs
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn canonical_gateway_payload(params: &[(String, String)]) -> String {
    let mut pairs = params
        .iter()
        .filter(|(key, value)| key != "sign" && !value.is_empty())
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<Vec<_>>();
    pairs.sort_unstable_by(|left, right| left.0.cmp(right.0));
    pairs
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn form_fields_to_json(fields: &[(String, String)]) -> Value {
    let mut object = serde_json::Map::new();
    for (key, value) in fields {
        object.insert(key.clone(), Value::String(value.clone()));
    }
    Value::Object(object)
}

fn require_cny(currency: Option<&str>, operation: PaymentAdapterOperation) -> ProviderResult<()> {
    let currency = require_non_empty(currency, operation, "currency")?;
    if !currency.eq_ignore_ascii_case("CNY") {
        return Err(ProviderError::invalid_request(
            operation,
            "Alipay domestic baseline currently supports CNY only",
        ));
    }
    Ok(())
}

fn validate_config_secret(field: &str, value: &str) -> ProviderResult<()> {
    if value.trim().is_empty() {
        return Err(ProviderError::invalid_request(
            PaymentAdapterOperation::CreatePaymentIntent,
            format!("Alipay {field} is required"),
        ));
    }
    Ok(())
}

fn current_alipay_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Builds `biz_content` for a given alipay method_key.
///
/// Supported method_keys (mirrors `commerce_payment_method.method_key` DB rows):
/// - `alipay_qr`    → `alipay.trade.precreate` (当面付扫码, returns `qr_code`)
/// - `alipay_pc`    → `alipay.trade.page.pay`  (PC 网站支付, returns form HTML)
/// - `alipay_wap`   → `alipay.trade.wap.pay`    (手机网站支付, returns redirect URL)
/// - `alipay_app`   → `alipay.trade.app.pay`   (App 支付, returns signed pay string)
/// - `alipay_jsapi` → `alipay.trade.create`     (JSAPI, returns `trade_no` for JSAPI唤起)
///
/// `buyer_id` is required for `alipay_jsapi` (buyer's openid in alipay) and
/// read from `metadata.buyer_id`.
fn build_alipay_biz_content(
    out_trade_no: &str,
    amount_minor: i64,
    subject: &str,
    tenant_id: Option<&str>,
    notify_url: Option<&str>,
    method_key: &str,
    metadata: &Value,
) -> ProviderResult<Value> {
    let mut biz_content = json!({
        "out_trade_no": out_trade_no,
        "total_amount": minor_to_decimal_string(amount_minor),
        "subject": subject,
    });
    if let Some(tenant_id) = tenant_id {
        biz_content["passback_params"] = json!(format!("tenant_id={tenant_id}"));
    }
    if let Some(notify_url) = notify_url {
        biz_content["notify_url"] = json!(notify_url);
    }
    match method_key {
        "alipay_qr" => {
            // 当面付 precreate — returns qr_code url
        }
        "alipay_pc" => {
            // PC 网站支付 — page.pay returns form HTML
            if let Some(return_url) = metadata_string(metadata, "return_url") {
                biz_content["return_url"] = json!(return_url);
            }
        }
        "alipay_wap" => {
            // 手机网站支付 — wap.pay returns redirect URL
            if let Some(return_url) = metadata_string(metadata, "return_url") {
                biz_content["return_url"] = json!(return_url);
            }
        }
        "alipay_app" => {
            // App 支付 — app.pay returns signed pay string
        }
        "alipay_jsapi" => {
            // JSAPI — trade.create returns trade_no; requires buyer_openid
            let buyer_id = metadata_string(metadata, "buyer_id")
                .or_else(|| metadata_string(metadata, "buyer_open_id"))
                .ok_or_else(|| {
                    ProviderError::invalid_request(
                        PaymentAdapterOperation::CreatePaymentIntent,
                        "alipay_jsapi requires metadata.buyer_id (buyer's openid)",
                    )
                })?;
            biz_content["buyer_id"] = json!(buyer_id);
        }
        _ => {
            return Err(ProviderError::invalid_request(
                PaymentAdapterOperation::CreatePaymentIntent,
                format!("unsupported alipay method_key: {method_key}"),
            ));
        }
    }
    Ok(biz_content)
}

/// Maps a method_key to the Alipay OpenAPI method name and the response field
/// holding the redirect URL (if any).
fn alipay_method_for_key(method_key: &str) -> (&'static str, Option<&'static str>) {
    match method_key {
        "alipay_qr" => ("alipay.trade.precreate", Some("qr_code")),
        "alipay_pc" => ("alipay.trade.page.pay", None),
        "alipay_wap" => ("alipay.trade.wap.pay", None),
        "alipay_app" => ("alipay.trade.app.pay", None),
        "alipay_jsapi" => ("alipay.trade.create", None),
        _ => ("alipay.trade.precreate", Some("qr_code")),
    }
}

fn percent_decode(value: &str, operation: PaymentAdapterOperation) -> ProviderResult<String> {
    urlencoding::decode(value.replace('+', " ").as_str())
        .map(|decoded| decoded.into_owned())
        .map_err(|error| {
            ProviderError::invalid_response(
                operation,
                format!("Alipay percent escape is invalid: {error}"),
            )
        })
}
