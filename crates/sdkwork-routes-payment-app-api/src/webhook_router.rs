use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_payment_providers::{
    normalize_provider_code, peek_webhook_routing_fields, provider_registry_for_account,
    PaymentNormalizeWebhookRequest, PaymentProviderRegistry, ProviderAccountBinding,
    ProviderCredentialBundle, PaymentVerifyWebhookRequest,
};
use sdkwork_payment_repository_sqlx::{
    ingest_provider_webhook_sqlite, IngestProviderWebhookCommand,
    load_active_provider_account_by_merchant_id_postgres,
    load_active_provider_account_by_merchant_id_sqlite, load_active_provider_account_postgres,
    load_active_provider_account_sqlite, load_webhook_attempt_context_by_out_trade_no_postgres,
    load_webhook_attempt_context_by_out_trade_no_sqlite, PaymentProviderAccountRecord,
    PostgresCommerceOwnerOrderPaymentStore, SqliteCommerceOwnerOrderPaymentStore,
};
use sdkwork_routes_payment_backend_api::{
    settle_owner_order_after_payment_success, OrderPointsRechargeFulfillmentClient,
    OwnerOrderPaymentStoreKind,
};
use sdkwork_utils_rust::SdkWorkCommandData;
use sdkwork_web_core::WebRequestContext;
use sqlx::{PgPool, SqlitePool};

use crate::api_response::{map_service_error, success_command, validation};

#[derive(Clone)]
pub enum WebhookDatabase {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

#[derive(Clone)]
enum WebhookState {
    Sqlite {
        registry: Arc<PaymentProviderRegistry>,
        credentials: ProviderCredentialBundle,
        pool: SqlitePool,
        payments: Arc<SqliteCommerceOwnerOrderPaymentStore>,
        fulfillment: Arc<OrderPointsRechargeFulfillmentClient>,
    },
    Postgres {
        registry: Arc<PaymentProviderRegistry>,
        credentials: ProviderCredentialBundle,
        pool: PgPool,
        payments: Arc<PostgresCommerceOwnerOrderPaymentStore>,
        fulfillment: Arc<OrderPointsRechargeFulfillmentClient>,
    },
}

pub fn payment_webhook_router(
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
    database: WebhookDatabase,
) -> Router {
    match database {
        WebhookDatabase::Sqlite(pool) => Router::new()
            .route(
                "/app/v3/api/payments/webhooks/{providerCode}",
                post(receive_provider_webhook),
            )
            .with_state(WebhookState::Sqlite {
                registry,
                credentials,
                pool: pool.clone(),
                payments: Arc::new(SqliteCommerceOwnerOrderPaymentStore::new(pool)),
                fulfillment: Arc::new(OrderPointsRechargeFulfillmentClient::from_env()),
            }),
        WebhookDatabase::Postgres(pool) => Router::new()
            .route(
                "/app/v3/api/payments/webhooks/{providerCode}",
                post(receive_provider_webhook),
            )
            .with_state(WebhookState::Postgres {
                registry,
                credentials,
                pool: pool.clone(),
                payments: Arc::new(PostgresCommerceOwnerOrderPaymentStore::new(pool)),
                fulfillment: Arc::new(OrderPointsRechargeFulfillmentClient::from_env()),
            }),
    }
}

async fn receive_provider_webhook(
    State(state): State<WebhookState>,
    request_context: Option<axum::extract::Extension<WebRequestContext>>,
    Path(provider_code): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    match state {
        WebhookState::Sqlite {
            registry,
            credentials,
            pool,
            payments,
            fulfillment,
        } => {
            receive_provider_webhook_inner(
                ctx,
                registry,
                credentials,
                &pool,
                OwnerOrderPaymentStoreKind::Sqlite(payments),
                fulfillment,
                provider_code,
                headers,
                body,
            )
            .await
        }
        WebhookState::Postgres {
            registry,
            credentials,
            pool,
            payments,
            fulfillment,
        } => {
            receive_provider_webhook_inner(
                ctx,
                registry,
                credentials,
                &pool,
                OwnerOrderPaymentStoreKind::Postgres(payments),
                fulfillment,
                provider_code,
                headers,
                body,
            )
            .await
        }
    }
}

async fn receive_provider_webhook_inner<Pool>(
    ctx: Option<&WebRequestContext>,
    deployment_registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
    pool: &Pool,
    payment_store: OwnerOrderPaymentStoreKind,
    fulfillment: Arc<OrderPointsRechargeFulfillmentClient>,
    provider_code: String,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response
where
    Pool: WebhookIngestPool + WebhookCredentialPool + Send + Sync,
{
    let provider_code = normalize_provider_code(&provider_code);
    let registry = match pool
        .resolve_webhook_registry(
            &deployment_registry,
            &credentials,
            &provider_code,
            &body,
        )
        .await
    {
        Ok(registry) => registry,
        Err(error) => return map_service_error(ctx, error),
    };

    let adapter = match registry.resolve(&provider_code) {
        Some(adapter) => adapter,
        None => {
            return validation(
                ctx,
                format!("payment provider {provider_code} is not configured"),
            );
        }
    };

    let header_pairs = headers
        .iter()
        .filter_map(|(name, value)| {
            Some((
                name.as_str().to_owned(),
                value.to_str().ok()?.to_owned(),
            ))
        })
        .collect::<Vec<_>>();

    let verify_request = PaymentVerifyWebhookRequest {
        headers: header_pairs.clone(),
        body: body.to_vec(),
        metadata: serde_json::json!({ "provider_code": provider_code }),
    };

    match adapter.verify_webhook(verify_request).await {
        Ok(outcome) if outcome.verified => {}
        Ok(_) => return validation(ctx, "webhook signature verification failed"),
        Err(error) => {
            return validation(ctx, format!("webhook provider error: {error:?}"));
        }
    }

    let normalize_request = PaymentNormalizeWebhookRequest {
        headers: header_pairs,
        body: body.to_vec(),
        metadata: serde_json::json!({ "provider_code": provider_code }),
    };

    let event = match adapter.normalize_webhook(normalize_request).await {
        Ok(event) => event,
        Err(error) => {
            return validation(ctx, format!("webhook provider error: {error:?}"));
        }
    };

    let provider_event_id = event
        .provider_event_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "{provider_code}:{}",
                event
                    .out_trade_no
                    .as_deref()
                    .unwrap_or("unknown-out-trade-no")
            )
        });

    let ingest = match pool
        .ingest_provider_webhook(IngestProviderWebhookCommand {
            provider_code: event.provider_code.clone(),
            provider_event_id,
            event_type: event.event_type.clone(),
            out_trade_no: event.out_trade_no.clone(),
            payment_status: event.payment_status.clone(),
            payload: event.payload.clone(),
        })
        .await
    {
        Ok(outcome) => outcome,
        Err(error) => return map_service_error(ctx, error),
    };

    if let Some(scope) = ingest.settlement_scope.as_ref() {
        let request_no = format!("webhook:{}", ingest.webhook_event_id);
        if let Err(error) = settle_owner_order_after_payment_success(
            &payment_store,
            &fulfillment,
            scope,
            &request_no,
        )
        .await
        {
            return map_service_error(ctx, error);
        }
    }

    success_command(
        ctx,
        SdkWorkCommandData {
            accepted: true,
            resource_id: ingest
                .payment_attempt_id
                .or(Some(ingest.webhook_event_id)),
            status: ingest.applied_status.or_else(|| {
                if ingest.replayed {
                    Some("replayed".to_owned())
                } else if ingest.settlement_scope.is_some() {
                    Some("settled".to_owned())
                } else {
                    Some("accepted".to_owned())
                }
            }),
        },
    )
}

trait WebhookCredentialPool {
    fn resolve_webhook_registry(
        &self,
        deployment_registry: &PaymentProviderRegistry,
        credentials: &ProviderCredentialBundle,
        provider_code: &str,
        body: &[u8],
    ) -> impl std::future::Future<Output = Result<PaymentProviderRegistry, CommerceServiceError>>
           + Send;
}

trait WebhookIngestPool {
    fn ingest_provider_webhook(
        &self,
        command: IngestProviderWebhookCommand,
    ) -> impl std::future::Future<
        Output = Result<
            sdkwork_payment_repository_sqlx::IngestProviderWebhookOutcome,
            CommerceServiceError,
        >,
    > + Send;
}

async fn resolve_webhook_provider_account_sqlite(
    pool: &SqlitePool,
    credentials: &ProviderCredentialBundle,
    deployment_registry: &PaymentProviderRegistry,
    provider_code: &str,
    body: &[u8],
) -> Result<PaymentProviderRegistry, CommerceServiceError> {
    let peek = peek_webhook_routing_fields(provider_code, body);
    let account = if let Some(out_trade_no) = peek.out_trade_no.as_deref() {
        if let Some(context) =
            load_webhook_attempt_context_by_out_trade_no_sqlite(pool, out_trade_no).await?
        {
            load_active_provider_account_sqlite(
                pool,
                &context.tenant_id,
                context.organization_id.as_deref(),
                &context.provider_code,
            )
            .await?
        } else {
            None
        }
    } else if let Some(merchant_id) = peek.merchant_id.as_deref() {
        load_active_provider_account_by_merchant_id_sqlite(pool, provider_code, merchant_id).await?
    } else {
        None
    };
    Ok(registry_for_webhook_account(
        deployment_registry,
        credentials,
        account,
    ))
}

async fn resolve_webhook_provider_account_postgres(
    pool: &PgPool,
    credentials: &ProviderCredentialBundle,
    deployment_registry: &PaymentProviderRegistry,
    provider_code: &str,
    body: &[u8],
) -> Result<PaymentProviderRegistry, CommerceServiceError> {
    let peek = peek_webhook_routing_fields(provider_code, body);
    let account = if let Some(out_trade_no) = peek.out_trade_no.as_deref() {
        if let Some(context) =
            load_webhook_attempt_context_by_out_trade_no_postgres(pool, out_trade_no).await?
        {
            load_active_provider_account_postgres(
                pool,
                &context.tenant_id,
                context.organization_id.as_deref(),
                &context.provider_code,
            )
            .await?
        } else {
            None
        }
    } else if let Some(merchant_id) = peek.merchant_id.as_deref() {
        load_active_provider_account_by_merchant_id_postgres(pool, provider_code, merchant_id).await?
    } else {
        None
    };
    Ok(registry_for_webhook_account(
        deployment_registry,
        credentials,
        account,
    ))
}

fn registry_for_webhook_account(
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    account: Option<PaymentProviderAccountRecord>,
) -> PaymentProviderRegistry {
    match account {
        Some(record) => provider_registry_for_account(
            credentials,
            Some(provider_account_binding(&record)),
        ),
        None => deployment_registry.clone(),
    }
}

fn provider_account_binding(record: &PaymentProviderAccountRecord) -> ProviderAccountBinding {
    ProviderAccountBinding {
        provider_code: record.provider_code.clone(),
        merchant_id: record.merchant_id.clone(),
        environment: record.environment.clone(),
        secret_ref: record.secret_ref.clone(),
        webhook_secret_ref: record.webhook_secret_ref.clone(),
        certificate_ref: record.certificate_ref.clone(),
        metadata: record.metadata.clone(),
    }
}

impl WebhookCredentialPool for SqlitePool {
    async fn resolve_webhook_registry(
        &self,
        deployment_registry: &PaymentProviderRegistry,
        credentials: &ProviderCredentialBundle,
        provider_code: &str,
        body: &[u8],
    ) -> Result<PaymentProviderRegistry, CommerceServiceError> {
        resolve_webhook_provider_account_sqlite(
            self,
            credentials,
            deployment_registry,
            provider_code,
            body,
        )
        .await
    }
}

impl WebhookCredentialPool for PgPool {
    async fn resolve_webhook_registry(
        &self,
        deployment_registry: &PaymentProviderRegistry,
        credentials: &ProviderCredentialBundle,
        provider_code: &str,
        body: &[u8],
    ) -> Result<PaymentProviderRegistry, CommerceServiceError> {
        resolve_webhook_provider_account_postgres(
            self,
            credentials,
            deployment_registry,
            provider_code,
            body,
        )
        .await
    }
}

impl WebhookIngestPool for SqlitePool {
    async fn ingest_provider_webhook(
        &self,
        command: IngestProviderWebhookCommand,
    ) -> Result<
        sdkwork_payment_repository_sqlx::IngestProviderWebhookOutcome,
        CommerceServiceError,
    > {
        ingest_provider_webhook_sqlite(self, command).await
    }
}

impl WebhookIngestPool for PgPool {
    async fn ingest_provider_webhook(
        &self,
        command: IngestProviderWebhookCommand,
    ) -> Result<
        sdkwork_payment_repository_sqlx::IngestProviderWebhookOutcome,
        CommerceServiceError,
    > {
        sdkwork_payment_repository_sqlx::ingest_provider_webhook_postgres(self, command).await
    }
}
