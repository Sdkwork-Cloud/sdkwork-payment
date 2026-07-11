use sdkwork_payment_service::{
    ConfirmOwnerOrderPaymentOutcome, OwnerOrderPaymentConfirmationFuture,
    OwnerOrderPaymentConfirmationPort,
};

use crate::{PostgresCommerceOwnerOrderPaymentStore, SqliteCommerceOwnerOrderPaymentStore};

impl OwnerOrderPaymentConfirmationPort for SqliteCommerceOwnerOrderPaymentStore {
    fn confirm_owner_order_payment<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        owner_user_id: &'a str,
        order_id: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ConfirmOwnerOrderPaymentOutcome> {
        Box::pin(async move {
            SqliteCommerceOwnerOrderPaymentStore::confirm_owner_order_payment(
                self,
                tenant_id,
                organization_id,
                owner_user_id,
                order_id,
            )
            .await
        })
    }
}

impl OwnerOrderPaymentConfirmationPort for PostgresCommerceOwnerOrderPaymentStore {
    fn confirm_owner_order_payment<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        owner_user_id: &'a str,
        order_id: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ConfirmOwnerOrderPaymentOutcome> {
        Box::pin(async move {
            PostgresCommerceOwnerOrderPaymentStore::confirm_owner_order_payment(
                self,
                tenant_id,
                organization_id,
                owner_user_id,
                order_id,
            )
            .await
        })
    }
}
