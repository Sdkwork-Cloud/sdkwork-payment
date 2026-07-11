use sdkwork_payment_service::{
    ConfirmOwnerOrderPaymentOutcome, OrderPaymentSettlementAttempt,
    OwnerOrderPaymentConfirmationFuture, OwnerOrderPaymentConfirmationPort,
};

use crate::{PostgresCommerceOwnerOrderPaymentStore, SqliteCommerceOwnerOrderPaymentStore};

impl OwnerOrderPaymentConfirmationPort for SqliteCommerceOwnerOrderPaymentStore {
    fn confirm_owner_order_payment<'a>(
        &'a self,
        attempt: &'a OrderPaymentSettlementAttempt,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ConfirmOwnerOrderPaymentOutcome> {
        Box::pin(async move {
            SqliteCommerceOwnerOrderPaymentStore::confirm_owner_order_payment(self, attempt).await
        })
    }
}

impl OwnerOrderPaymentConfirmationPort for PostgresCommerceOwnerOrderPaymentStore {
    fn confirm_owner_order_payment<'a>(
        &'a self,
        attempt: &'a OrderPaymentSettlementAttempt,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ConfirmOwnerOrderPaymentOutcome> {
        Box::pin(async move {
            PostgresCommerceOwnerOrderPaymentStore::confirm_owner_order_payment(self, attempt).await
        })
    }
}
