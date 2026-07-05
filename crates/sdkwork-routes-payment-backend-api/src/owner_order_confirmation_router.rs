use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_payment_repository_sqlx::{
    load_owner_order_settlement_scope_by_order_id_postgres,
    load_owner_order_settlement_scope_by_order_id_sqlite,
    PostgresCommerceOwnerOrderPaymentStore, SqliteCommerceOwnerOrderPaymentStore,
};
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, SqlitePool};

use crate::api_response::{forbidden, map_service_error, not_found, success_item, unauthorized, validation};
use crate::order_fulfillment_client::OrderPointsRechargeFulfillmentClient;
use crate::owner_order_settlement::{
    settle_owner_order_after_payment_success, OwnerOrderPaymentStoreKind,
};
use crate::subject::backend_runtime_subject_from_extension;

mod permissions {
    pub const CONFIRM: &str = "commerce.payments.confirm";
}

#[derive(Clone)]
enum ConfirmationDatabase {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

#[derive(Clone)]
struct OwnerOrderConfirmationState {
    store: OwnerOrderPaymentStoreKind,
    database: ConfirmationDatabase,
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
            SqliteCommerceOwnerOrderPaymentStore::new(pool.clone()),
        )),
        database: ConfirmationDatabase::Sqlite(pool),
        order_fulfillment: Arc::new(OrderPointsRechargeFulfillmentClient::from_env()),
    })
}

pub fn owner_order_confirmation_router_with_postgres_pool(pool: PgPool) -> Router {
    build_owner_order_confirmation_router(OwnerOrderConfirmationState {
        store: OwnerOrderPaymentStoreKind::Postgres(Arc::new(
            PostgresCommerceOwnerOrderPaymentStore::new(pool.clone()),
        )),
        database: ConfirmationDatabase::Postgres(pool),
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

    let scope = match resolve_settlement_scope(
        &state.database,
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &owner_user_id,
        &order_id,
    )
    .await
    {
        Ok(Some(scope)) => scope,
        Ok(None) => return not_found(ctx, "owner order payment scope was not found"),
        Err(error) => return map_service_error(ctx, error),
    };

    let settlement_outcome = match settle_owner_order_after_payment_success(
        &state.store,
        &state.order_fulfillment,
        &scope,
        &body.request_no,
    )
    .await
    {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };

    success_item(
        ctx,
        ConfirmOwnerOrderPaymentResponse {
            payment_confirmed: settlement_outcome.payment_confirmed,
            payment_replayed: settlement_outcome.payment_replayed,
            fulfillment_accepted: settlement_outcome.fulfillment_accepted,
            fulfillment_replayed: settlement_outcome.fulfillment_replayed,
            order_id: settlement_outcome.order_id,
            points_credited: settlement_outcome.points_credited,
            fulfillment_status: settlement_outcome.fulfillment_status,
        },
    )
}

async fn resolve_settlement_scope(
    database: &ConfirmationDatabase,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: &str,
) -> Result<Option<sdkwork_payment_repository_sqlx::OwnerOrderSettlementScope>, CommerceServiceError>
{
    match database {
        ConfirmationDatabase::Sqlite(pool) => {
            load_owner_order_settlement_scope_by_order_id_sqlite(
                pool,
                tenant_id,
                organization_id,
                owner_user_id,
                order_id,
            )
            .await
        }
        ConfirmationDatabase::Postgres(pool) => {
            load_owner_order_settlement_scope_by_order_id_postgres(
                pool,
                tenant_id,
                organization_id,
                owner_user_id,
                order_id,
            )
            .await
        }
    }
}
