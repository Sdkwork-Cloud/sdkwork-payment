use crate::{CreatePaymentIntentCommand, CreateRefundCommand, PaymentIntentDraft};
use sdkwork_commerce_contract_service::CommerceServiceError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaymentProviderCommand {
    CreatePaymentIntent,
    QueryPaymentStatus,
    ClosePayment,
    Refund,
    VerifyWebhook,
}

pub struct PaymentProviderPortRequirement;

pub trait PaymentProviderPort {
    fn create_payment_intent(
        &self,
        command: &CreatePaymentIntentCommand,
    ) -> Result<PaymentIntentDraft, CommerceServiceError>;

    fn refund(&self, command: &CreateRefundCommand) -> Result<(), CommerceServiceError>;
}

pub const PAYMENT_REPOSITORY_PORT: &str = "payment.repository";
pub const PAYMENT_PROVIDER_PORT: &str = "payment.provider";
pub const IDEMPOTENCY_REPOSITORY_PORT: &str = "idempotency.repository";

impl PaymentProviderPortRequirement {
    pub fn standard_commands() -> Vec<PaymentProviderCommand> {
        vec![
            PaymentProviderCommand::CreatePaymentIntent,
            PaymentProviderCommand::QueryPaymentStatus,
            PaymentProviderCommand::ClosePayment,
            PaymentProviderCommand::Refund,
            PaymentProviderCommand::VerifyWebhook,
        ]
    }
}
