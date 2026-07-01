use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_order_service::points_recharge_payment_success_idempotency_key;
use sdkwork_payment_repository_sqlx::{
    ConfirmOwnerOrderPaymentOutcome, PostgresCommerceOwnerOrderPaymentStore,
    SqliteCommerceOwnerOrderPaymentStore,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, SqlitePool};

use crate::api_response::{forbidden, map_service_error, success_item, unauthorized, validation};
use crate::order_fulfillment_client::{
    OrderPointsRechargeFulfillmentClient, OrderPointsRechargeFulfillmentRequest,
};
use crate::subject::backend_runtime_subject_from_extension;

mod permissions {
    pub const CONFIRM: &str = "commerce.payments.confirm";
}

#[derive(Clone)]
enum OwnerOrderPaymentStoreKind {
    Sqlite(Arc<SqliteCommerceOwnerOrderPaymentStore>),
    Postgres(Arc<PostgresCommerceOwnerOrderPaymentStore>),
}

#[derive(Clone)]
struct OwnerOrderConfirmationState {
    store: OwnerOrderPaymentStoreKind,
    order_fulfillment: Arc<OrderPointsRechargeFulfillmentClient>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmOwnerOrderPaymentRequest {
    request_no: String,
    #[serde(default)]
    owner_user_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmOwnerOrderPaymentResponse {
    payment_confirmed: bool,
    payment_replayed: bool,
    fulfillment_accepted: bool,
    fulfillment_replayed: bool,
    order_id: String,
    points_credited: i64,
    fulfillment_status: String,
}

pub fn owner_order_confirmation_router_with_sqlite_pool(pool: SqlitePool) -> Router {
    build_owner_order_confirmation_router(OwnerOrderConfirmationState {
        store: OwnerOrderPaymentStoreKind::Sqlite(Arc::new(
            SqliteCommerceOwnerOrderPaymentStore::new(pool),
        )),
        order_fulfillment: Arc::new(OrderPointsRechargeFulfillmentClient::from_env()),
    })
}

pub fn owner_order_confirmation_router_with_postgres_pool(pool: PgPool) -> Router {
    build_owner_order_confirmation_router(OwnerOrderConfirmationState {
        store: OwnerOrderPaymentStoreKind::Postgres(Arc::new(
            PostgresCommerceOwnerOrderPaymentStore::new(pool),
        )),
        order_fulfillment: Arc::new(OrderPointsRechargeFulfillmentClient::from_env()),
    })
}

fn build_owner_order_confirmation_router(state: OwnerOrderConfirmationState) -> Router {
    Router::new()
        .route(
            "/backend/v3/api/payments/owner-orders/{orderId}/confirmations",
            post(confirm_owner_order_payment),
        )
        .with_state(state)
}

async fn confirm_owner_order_payment(
    State(state): State<OwnerOrderConfirmationState>,
    Extension(runtime_context): Extension<IamAppContext>,
    request_context: Extension<WebRequestContext>,
    Path(order_id): Path<String>,
    Json(body): Json<ConfirmOwnerOrderPaymentRequest>,
) -> Response {
    let ctx = Some(&request_context.0);
    if !runtime_context.has_permission(permissions::CONFIRM) {
        return forbidden(
            ctx,
            format!("missing required permission: {}", permissions::CONFIRM),
        );
    }

    let subject = match backend_runtime_subject_from_extension(Some(Extension(
        runtime_context.clone(),
    ))) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };

    if body.request_no.trim().is_empty() {
        return validation(ctx, "request_no is required");
    }

    let owner_user_id = body
        .owner_user_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| subject.user_id.clone());

    let payment_outcome = match confirm_payment(
        &state.store,
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &owner_user_id,
        &order_id,
    )
    .await
    {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };

    let fulfillment_request = OrderPointsRechargeFulfillmentRequest {
        request_no: body.request_no.clone(),
        idempotency_key: points_recharge_payment_success_idempotency_key(&order_id),
        paid_at: payment_outcome.paid_at.clone(),
        owner_user_id: owner_user_id.clone(),
    };

    let fulfillment_outcome = match state
        .order_fulfillment
        .create_points_recharge_fulfillment(&order_id, &fulfillment_request)
        .await
    {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };

    success_item(
        ctx,
        ConfirmOwnerOrderPaymentResponse {
            payment_confirmed: true,
            payment_replayed: payment_outcome.replayed,
            fulfillment_accepted: fulfillment_outcome.accepted,
            fulfillment_replayed: fulfillment_outcome.replayed,
            order_id: fulfillment_outcome.order_id,
            points_credited: fulfillment_outcome.points_credited,
            fulfillment_status: fulfillment_outcome.fulfillment_status,
        },
    )
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
