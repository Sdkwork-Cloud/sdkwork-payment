use std::future::Future;
use std::pin::Pin;

use sdkwork_contract_service::CommerceServiceError;

pub type OwnerOrderPaymentConfirmationFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmOwnerOrderPaymentOutcome {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub paid_at: String,
    pub replayed: bool,
}

/// Payment-attempt context for in-process settlement after webhook ingest or manual confirmation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderPaymentSettlementAttempt {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
}

pub trait OwnerOrderPaymentConfirmationPort: Send + Sync {
    fn confirm_owner_order_payment<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        owner_user_id: &'a str,
        order_id: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ConfirmOwnerOrderPaymentOutcome>;
}

pub const OWNER_ORDER_PAYMENT_CONFIRMATION_PORT: &str = "payment.owner_order_payment.confirmation";
