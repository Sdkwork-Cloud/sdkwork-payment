//! Payment service provider adapters for Stripe, Alipay, and WeChat Pay.

mod adapter;
mod alipay;
mod checkout;
mod credentials;
mod error;
mod http;
mod money;
mod operations;
mod registry;
mod stripe;
mod webhook_peek;
mod wechat_pay;

pub use adapter::{normalize_provider_code, PaymentProviderAdapter};
pub use adapter::{
    PaymentNormalizeWebhookRequest, PaymentVerifyWebhookRequest,
};
pub use checkout::{enrich_pay_owner_order_outcome, CheckoutContext};
pub use credentials::{
    build_order_payment_webhook_url, EnvPaymentCredentialResolver, ProviderAccountBinding,
    ProviderCredentialBundle, resolve_secret_ref, ORDER_PAYMENT_WEBHOOK_PATH,
};
pub use operations::{cancel_provider_payment, create_provider_refund};
pub use registry::{provider_registry_for_account, PaymentProviderRegistry};
pub use webhook_peek::{peek_webhook_routing_fields, WebhookPeekOutcome};
