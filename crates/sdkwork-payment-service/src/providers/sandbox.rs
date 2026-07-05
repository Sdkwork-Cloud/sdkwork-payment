use sdkwork_contract_service::CommerceServiceError;

use crate::{
    CreatePaymentIntentCommand, CreateRefundCommand, PaymentIntentDraft, PaymentProviderPort,
};

/// Local sandbox provider for development and contract tests.
///
/// Returns deterministic cashier drafts without calling external PSP HTTP APIs.
#[derive(Debug, Clone, Default)]
pub struct SandboxPaymentProvider;

impl PaymentProviderPort for SandboxPaymentProvider {
    fn create_payment_intent(
        &self,
        command: &CreatePaymentIntentCommand,
    ) -> Result<PaymentIntentDraft, CommerceServiceError> {
        PaymentIntentDraft::new(
            &command.tenant_id,
            &command.order_id,
            &command.payment_method,
            &command.provider_code,
            command.amount.clone(),
            &command.idempotency_key,
        )
    }

    fn refund(&self, _command: &CreateRefundCommand) -> Result<(), CommerceServiceError> {
        Ok(())
    }
}
