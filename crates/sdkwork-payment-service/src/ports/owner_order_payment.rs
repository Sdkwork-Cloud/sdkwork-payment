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
    /// Exact payment attempt selected by webhook identity resolution.
    /// Manual confirmation may leave this unset, in which case the
    /// repository must resolve a single unambiguous candidate.
    pub payment_attempt_id: Option<String>,
    pub out_trade_no: Option<String>,
}

pub trait OwnerOrderPaymentConfirmationPort: Send + Sync {
    fn confirm_owner_order_payment<'a>(
        &'a self,
        attempt: &'a OrderPaymentSettlementAttempt,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ConfirmOwnerOrderPaymentOutcome>;
}

pub const OWNER_ORDER_PAYMENT_CONFIRMATION_PORT: &str = "payment.owner_order_payment.confirmation";
