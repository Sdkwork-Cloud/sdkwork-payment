use sdkwork_order_service::{
    ConfirmOwnerOrderPaymentOutcome, OwnerOrderPaymentConfirmationFuture,
    OwnerOrderPaymentConfirmationPort,
};

use crate::{
    PostgresCommerceOwnerOrderPaymentStore, SqliteCommerceOwnerOrderPaymentStore,
};

impl OwnerOrderPaymentConfirmationPort for SqliteCommerceOwnerOrderPaymentStore {
    fn confirm_owner_order_payment<'a>(
        &'a self,
        tenant_id: &'a str,
        organization_id: Option<&'a str>,
        owner_user_id: &'a str,
        order_id: &'a str,
    ) -> OwnerOrderPaymentConfirmationFuture<'a, ConfirmOwnerOrderPaymentOutcome> {
        Box::pin(async move {
            let outcome = SqliteCommerceOwnerOrderPaymentStore::confirm_owner_order_payment(
                self,
                tenant_id,
                organization_id,
                owner_user_id,
                order_id,
            )
            .await?;
            Ok(ConfirmOwnerOrderPaymentOutcome {
                tenant_id: outcome.tenant_id,
                organization_id: outcome.organization_id,
                owner_user_id: outcome.owner_user_id,
                order_id: outcome.order_id,
                paid_at: outcome.paid_at,
                replayed: outcome.replayed,
            })
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
            let outcome = PostgresCommerceOwnerOrderPaymentStore::confirm_owner_order_payment(
                self,
                tenant_id,
                organization_id,
                owner_user_id,
                order_id,
            )
            .await?;
            Ok(ConfirmOwnerOrderPaymentOutcome {
                tenant_id: outcome.tenant_id,
                organization_id: outcome.organization_id,
                owner_user_id: outcome.owner_user_id,
                order_id: outcome.order_id,
                paid_at: outcome.paid_at,
                replayed: outcome.replayed,
            })
        })
    }
}
