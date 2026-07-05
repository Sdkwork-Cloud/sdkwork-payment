use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::points_recharge_payment_success_idempotency_key;
use sdkwork_payment_repository_sqlx::{
    ConfirmOwnerOrderPaymentOutcome, OwnerOrderSettlementScope,
    PostgresCommerceOwnerOrderPaymentStore, SqliteCommerceOwnerOrderPaymentStore,
};

use crate::order_fulfillment_client::{
    OrderPointsRechargeFulfillmentClient, OrderPointsRechargeFulfillmentRequest,
};

#[derive(Clone, Debug)]
pub enum OwnerOrderPaymentStoreKind {
    Sqlite(std::sync::Arc<SqliteCommerceOwnerOrderPaymentStore>),
    Postgres(std::sync::Arc<PostgresCommerceOwnerOrderPaymentStore>),
}

#[derive(Debug, Clone)]
pub struct OwnerOrderSettlementOutcome {
    pub payment_confirmed: bool,
    pub payment_replayed: bool,
    pub fulfillment_accepted: bool,
    pub fulfillment_replayed: bool,
    pub order_id: String,
    pub points_credited: i64,
    pub fulfillment_status: String,
}

pub async fn settle_owner_order_after_payment_success(
    store: &OwnerOrderPaymentStoreKind,
    fulfillment: &OrderPointsRechargeFulfillmentClient,
    scope: &OwnerOrderSettlementScope,
    request_no: &str,
) -> Result<OwnerOrderSettlementOutcome, CommerceServiceError> {
    let payment_outcome = confirm_payment(
        store,
        &scope.tenant_id,
        scope.organization_id.as_deref(),
        &scope.owner_user_id,
        &scope.order_id,
    )
    .await?;

    let mut fulfillment_accepted = false;
    let mut fulfillment_replayed = false;
    let mut points_credited = 0_i64;
    let mut fulfillment_status = String::new();

    if is_points_recharge_subject(scope.order_subject.as_deref()) {
        let fulfillment_request = OrderPointsRechargeFulfillmentRequest {
            request_no: request_no.to_owned(),
            idempotency_key: points_recharge_payment_success_idempotency_key(&scope.order_id),
            paid_at: payment_outcome.paid_at.clone(),
            owner_user_id: scope.owner_user_id.clone(),
        };
        let fulfillment_outcome = fulfillment
            .create_points_recharge_fulfillment(&scope.order_id, &fulfillment_request)
            .await?;
        fulfillment_accepted = fulfillment_outcome.accepted;
        fulfillment_replayed = fulfillment_outcome.replayed;
        points_credited = fulfillment_outcome.points_credited;
        fulfillment_status = fulfillment_outcome.fulfillment_status;
    }

    Ok(OwnerOrderSettlementOutcome {
        payment_confirmed: true,
        payment_replayed: payment_outcome.replayed,
        fulfillment_accepted,
        fulfillment_replayed,
        order_id: scope.order_id.clone(),
        points_credited,
        fulfillment_status,
    })
}

async fn confirm_payment(
    store: &OwnerOrderPaymentStoreKind,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: &str,
) -> Result<ConfirmOwnerOrderPaymentOutcome, CommerceServiceError> {
    match store {
        OwnerOrderPaymentStoreKind::Sqlite(store) => {
            store
                .confirm_owner_order_payment(tenant_id, organization_id, owner_user_id, order_id)
                .await
        }
        OwnerOrderPaymentStoreKind::Postgres(store) => {
            store
                .confirm_owner_order_payment(tenant_id, organization_id, owner_user_id, order_id)
                .await
        }
    }
}

fn is_points_recharge_subject(subject: Option<&str>) -> bool {
    subject
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|value| value.eq_ignore_ascii_case("points_recharge"))
}
