//! Owner-order pay PSP enrichment after repository persistence.
//!
//! Shared by payment and order app-api routers so `orders.payments.create` and `payments.create`
//! return the same cashier parameters.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use chrono::{DateTime, Duration, Utc};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_payment_providers::{
    enrich_pay_owner_order_outcome, normalize_provider_code, provider_registry_for_account,
    CheckoutContext, PaymentProviderRegistry, ProviderAccountBinding, ProviderCredentialBundle,
};
use sdkwork_payment_service::{
    CancelOrderPaymentsCommand, CreateOwnerPaymentAttemptOutcome, PayOwnerOrderOutcome,
    PaymentRecordItem,
};
use sqlx::{PgPool, Pool, Postgres, Sqlite, SqlitePool, Transaction};
use tokio::time::{sleep, Instant};

use crate::provider_account::{
    ensure_provider_account_matches, load_active_provider_account_for_channel_postgres,
    load_active_provider_account_for_channel_sqlite, load_active_provider_account_postgres,
    load_active_provider_account_sqlite, load_provider_account_for_existing_payment_postgres,
    load_provider_account_for_existing_payment_sqlite, PaymentProviderAccountRecord,
};

pub fn provider_account_binding(record: &PaymentProviderAccountRecord) -> ProviderAccountBinding {
    ProviderAccountBinding {
        provider_code: record.provider_code.clone(),
        merchant_id: record.merchant_id.clone(),
        environment: record.environment.clone(),
        secret_ref: record.secret_ref.clone(),
        webhook_secret_ref: record.webhook_secret_ref.clone(),
        certificate_ref: record.certificate_ref.clone(),
        primary_secret: record.primary_secret.clone(),
        webhook_secret: record.webhook_secret.clone(),
        certificate: record.certificate.clone(),
        metadata: record.metadata.clone(),
    }
}

use crate::owner_payment_params::owner_order_payment_params;
use crate::payment_attempt_context::{
    load_payment_attempt_provider_context_postgres, load_payment_attempt_provider_context_sqlite,
    persist_attempt_enrichment_postgres, persist_attempt_enrichment_sqlite,
};

const PROVIDER_CHECKOUT_TTL_SECONDS: i64 = 900;
const POSTGRES_CHECKOUT_LOCK_RETRY_MILLIS: u64 = 25;
const POSTGRES_CHECKOUT_LOCK_TIMEOUT_SECONDS: u64 = 30;
type CheckoutMutex = tokio::sync::Mutex<()>;
static SQLITE_CHECKOUT_LOCKS: OnceLock<Mutex<HashMap<String, Weak<CheckoutMutex>>>> =
    OnceLock::new();

#[derive(Clone, Copy)]
pub struct OwnerOrderPaymentEnrichmentContext<'a> {
    pub deployment_registry: &'a PaymentProviderRegistry,
    pub credentials: &'a ProviderCredentialBundle,
    pub tenant_id: &'a str,
    pub organization_id: Option<&'a str>,
    pub order_id: &'a str,
    pub payment_scene: Option<&'a str>,
}

pub fn payment_record_is_checkout_eligible(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "created" | "pending" | "processing"
    )
}

pub async fn cancel_owner_order_payments_with_provider_sqlite(
    pool: &SqlitePool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    command: CancelOrderPaymentsCommand,
) -> Result<(), CommerceServiceError> {
    let lock = sqlite_checkout_lock(checkout_lock_key_from_parts(
        &command.tenant_id,
        command.organization_id.as_deref(),
        &command.order_id,
    ));
    let _guard = lock.lock().await;
    crate::owner_order_provider_close::cancel_owner_order_payments_with_provider_sqlite_unlocked(
        pool,
        deployment_registry,
        credentials,
        command,
    )
    .await
}

pub async fn cancel_owner_order_payments_with_provider_postgres(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    command: CancelOrderPaymentsCommand,
) -> Result<(), CommerceServiceError> {
    let lock_key = checkout_lock_key_from_parts(
        &command.tenant_id,
        command.organization_id.as_deref(),
        &command.order_id,
    );
    let lock_transaction = acquire_postgres_checkout_lock(pool, &lock_key).await?;
    let result = crate::owner_order_provider_close::cancel_owner_order_payments_with_provider_postgres_unlocked(
        pool,
        deployment_registry,
        credentials,
        command,
    )
    .await;
    let release_result = release_postgres_checkout_lock(lock_transaction).await;
    match (result, release_result) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(()), Ok(_)) => Ok(()),
    }
}

pub async fn enrich_payment_record_checkout_sqlite(
    pool: &Pool<Sqlite>,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    record: PaymentRecordItem,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let base = payment_record_to_pay_outcome(&record, None);
    if !payment_record_is_checkout_eligible(&record.status) {
        return Ok(base);
    }
    let Some(ctx) =
        load_payment_attempt_provider_context_sqlite(pool, tenant_id, owner_user_id, &record.id)
            .await?
    else {
        return Ok(base);
    };
    let outcome = payment_record_to_pay_outcome(&record, Some(&ctx));
    let enriched = enrich_owner_order_payment_sqlite(
        pool,
        OwnerOrderPaymentEnrichmentContext {
            deployment_registry,
            credentials,
            tenant_id,
            organization_id,
            order_id: &record.order_id,
            payment_scene: None,
        },
        outcome,
    )
    .await?;
    Ok(enriched)
}

pub async fn enrich_payment_record_checkout_postgres(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    record: PaymentRecordItem,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let base = payment_record_to_pay_outcome(&record, None);
    if !payment_record_is_checkout_eligible(&record.status) {
        return Ok(base);
    }
    let Some(ctx) =
        load_payment_attempt_provider_context_postgres(pool, tenant_id, owner_user_id, &record.id)
            .await?
    else {
        return Ok(base);
    };
    let outcome = payment_record_to_pay_outcome(&record, Some(&ctx));
    let enriched = enrich_owner_order_payment_postgres(
        pool,
        OwnerOrderPaymentEnrichmentContext {
            deployment_registry,
            credentials,
            tenant_id,
            organization_id,
            order_id: &record.order_id,
            payment_scene: None,
        },
        outcome,
    )
    .await?;
    Ok(enriched)
}

fn payment_record_to_pay_outcome(
    record: &PaymentRecordItem,
    provider_ctx: Option<&crate::payment_attempt_context::PaymentAttemptProviderContext>,
) -> PayOwnerOrderOutcome {
    let provider_code = provider_ctx
        .map(|ctx| ctx.provider_code.clone())
        .unwrap_or_else(|| record.method.clone());
    let out_trade_no = provider_ctx
        .map(|ctx| ctx.out_trade_no.clone())
        .unwrap_or_else(|| record.order_no.clone());
    let mut payment_params =
        owner_order_payment_params(&provider_code, &record.order_no, None, &out_trade_no);
    if let Some(ctx) = provider_ctx {
        if let Some(channel_id) = ctx.channel_id.as_deref() {
            payment_params.insert("channelId".to_owned(), channel_id.to_owned());
        }
        if let Some(native_id) = ctx.provider_transaction_id.as_deref() {
            payment_params.insert("providerTransactionId".to_owned(), native_id.to_owned());
        }
    }
    PayOwnerOrderOutcome {
        amount: record.amount.clone(),
        order_id: record.order_id.clone(),
        out_trade_no,
        payment_id: record.id.clone(),
        payment_method: record.method.clone(),
        status: record.status.clone(),
        payment_params,
    }
}

pub async fn enrich_owner_order_payment_sqlite(
    pool: &Pool<Sqlite>,
    context: OwnerOrderPaymentEnrichmentContext<'_>,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let lock = sqlite_checkout_lock(checkout_lock_key(&context));
    let _guard = lock.lock().await;
    enrich_owner_order_payment_sqlite_locked(pool, context, outcome).await
}

async fn enrich_owner_order_payment_sqlite_locked(
    pool: &Pool<Sqlite>,
    context: OwnerOrderPaymentEnrichmentContext<'_>,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let owner_user_id = load_active_attempt_owner_user_id_sqlite(
        pool,
        context.tenant_id,
        context.organization_id,
        context.order_id,
        &outcome.payment_id,
    )
    .await?;
    let attempt_context = load_payment_attempt_provider_context_sqlite(
        pool,
        context.tenant_id,
        &owner_user_id,
        &outcome.payment_id,
    )
    .await?
    .ok_or_else(|| {
        CommerceServiceError::conflict(
            "payment attempt was superseded while checkout was being prepared",
        )
    })?;
    ensure_provider_attempt_snapshot(&attempt_context, &outcome)?;
    crate::owner_order_provider_close::close_expired_owner_order_provider_attempts_for_order_sqlite(
        pool,
        context.deployment_registry,
        context.credentials,
        context.tenant_id,
        context.organization_id,
        &owner_user_id,
        context.order_id,
    )
    .await?;
    let expires_at = provider_checkout_expiration_sqlite(
        pool,
        context.tenant_id,
        &owner_user_id,
        &outcome.payment_id,
    )
    .await?;
    crate::owner_order_provider_close::close_owner_order_provider_attempts_sqlite(
        pool,
        context.deployment_registry,
        context.credentials,
        context.tenant_id,
        context.organization_id,
        &owner_user_id,
        context.order_id,
        Some(&outcome.payment_id),
    )
    .await?;
    let provider_code = attempt_context.provider_code.clone();
    let account =
        provider_account_for_attempt_sqlite(pool, &context, &attempt_context, &provider_code)
            .await?;
    let enriched = enrich_owner_order_payment_outcome(
        &context,
        account.as_ref().map(provider_account_binding),
        &provider_code,
        &attempt_context.idempotency_key,
        Some(&attempt_context.payment_metadata),
        outcome,
        expires_at.as_deref(),
    )
    .await?;
    persist_attempt_enrichment_sqlite(
        pool,
        context.tenant_id,
        &enriched.payment_id,
        &enriched.payment_params,
    )
    .await?;
    Ok(enriched)
}

pub async fn enrich_owner_order_payment_postgres(
    pool: &PgPool,
    context: OwnerOrderPaymentEnrichmentContext<'_>,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let lock_key = checkout_lock_key(&context);
    let lock_transaction = acquire_postgres_checkout_lock(pool, &lock_key).await?;

    let result = enrich_owner_order_payment_postgres_locked(pool, context, outcome).await;
    let release_result = release_postgres_checkout_lock(lock_transaction).await;
    match (result, release_result) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(outcome), Ok(_)) => Ok(outcome),
    }
}

async fn enrich_owner_order_payment_postgres_locked(
    pool: &PgPool,
    context: OwnerOrderPaymentEnrichmentContext<'_>,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let owner_user_id = load_active_attempt_owner_user_id_postgres(
        pool,
        context.tenant_id,
        context.organization_id,
        context.order_id,
        &outcome.payment_id,
    )
    .await?;
    let attempt_context = load_payment_attempt_provider_context_postgres(
        pool,
        context.tenant_id,
        &owner_user_id,
        &outcome.payment_id,
    )
    .await?
    .ok_or_else(|| {
        CommerceServiceError::conflict(
            "payment attempt was superseded while checkout was being prepared",
        )
    })?;
    ensure_provider_attempt_snapshot(&attempt_context, &outcome)?;
    crate::owner_order_provider_close::close_expired_owner_order_provider_attempts_for_order_postgres(
        pool,
        context.deployment_registry,
        context.credentials,
        context.tenant_id,
        context.organization_id,
        &owner_user_id,
        context.order_id,
    )
    .await?;
    let expires_at = provider_checkout_expiration_postgres(
        pool,
        context.tenant_id,
        &owner_user_id,
        &outcome.payment_id,
    )
    .await?;
    crate::owner_order_provider_close::close_owner_order_provider_attempts_postgres(
        pool,
        context.deployment_registry,
        context.credentials,
        context.tenant_id,
        context.organization_id,
        &owner_user_id,
        context.order_id,
        Some(&outcome.payment_id),
    )
    .await?;
    let provider_code = attempt_context.provider_code.clone();
    let account =
        provider_account_for_attempt_postgres(pool, &context, &attempt_context, &provider_code)
            .await?;
    let enriched = enrich_owner_order_payment_outcome(
        &context,
        account.as_ref().map(provider_account_binding),
        &provider_code,
        &attempt_context.idempotency_key,
        Some(&attempt_context.payment_metadata),
        outcome,
        expires_at.as_deref(),
    )
    .await?;
    persist_attempt_enrichment_postgres(
        pool,
        context.tenant_id,
        &enriched.payment_id,
        &enriched.payment_params,
    )
    .await?;
    Ok(enriched)
}

async fn provider_account_for_attempt_sqlite(
    pool: &Pool<Sqlite>,
    context: &OwnerOrderPaymentEnrichmentContext<'_>,
    attempt: &crate::payment_attempt_context::PaymentAttemptProviderContext,
    provider_code: &str,
) -> Result<Option<PaymentProviderAccountRecord>, CommerceServiceError> {
    let account = if let Some(provider_account_id) = attempt.provider_account_id.as_deref() {
        load_provider_account_for_existing_payment_sqlite(
            pool,
            context.tenant_id,
            context.organization_id,
            provider_account_id,
        )
        .await?
        .ok_or_else(|| {
            CommerceServiceError::conflict(
                "payment attempt provider account snapshot is unavailable",
            )
        })?
        .into()
    } else if let Some(channel_id) = attempt.channel_id.as_deref() {
        load_active_provider_account_for_channel_sqlite(
            pool,
            context.tenant_id,
            context.organization_id,
            channel_id,
            provider_code,
        )
        .await?
    } else {
        load_active_provider_account_sqlite(
            pool,
            context.tenant_id,
            context.organization_id,
            provider_code,
        )
        .await?
    };
    ensure_provider_account_matches(account.as_ref(), provider_code)?;
    Ok(account)
}

async fn provider_account_for_attempt_postgres(
    pool: &PgPool,
    context: &OwnerOrderPaymentEnrichmentContext<'_>,
    attempt: &crate::payment_attempt_context::PaymentAttemptProviderContext,
    provider_code: &str,
) -> Result<Option<PaymentProviderAccountRecord>, CommerceServiceError> {
    let account = if let Some(provider_account_id) = attempt.provider_account_id.as_deref() {
        load_provider_account_for_existing_payment_postgres(
            pool,
            context.tenant_id,
            context.organization_id,
            provider_account_id,
        )
        .await?
        .ok_or_else(|| {
            CommerceServiceError::conflict(
                "payment attempt provider account snapshot is unavailable",
            )
        })?
        .into()
    } else if let Some(channel_id) = attempt.channel_id.as_deref() {
        load_active_provider_account_for_channel_postgres(
            pool,
            context.tenant_id,
            context.organization_id,
            channel_id,
            provider_code,
        )
        .await?
    } else {
        load_active_provider_account_postgres(
            pool,
            context.tenant_id,
            context.organization_id,
            provider_code,
        )
        .await?
    };
    ensure_provider_account_matches(account.as_ref(), provider_code)?;
    Ok(account)
}

fn checkout_lock_key(context: &OwnerOrderPaymentEnrichmentContext<'_>) -> String {
    checkout_lock_key_from_parts(context.tenant_id, context.organization_id, context.order_id)
}

fn checkout_lock_key_from_parts(
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
) -> String {
    fn component(value: &str) -> String {
        format!("{}:{value}", value.len())
    }

    let organization = organization_id
        .map(component)
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "payment-checkout:v1|{}|{organization}|{}",
        component(tenant_id),
        component(order_id)
    )
}

async fn acquire_postgres_checkout_lock(
    pool: &PgPool,
    lock_key: &str,
) -> Result<Transaction<'static, Postgres>, CommerceServiceError> {
    if pool.options().get_max_connections() < 2 {
        return Err(CommerceServiceError::storage(
            "payment checkout advisory locking requires a PostgreSQL pool with at least two connections",
        ));
    }

    let deadline =
        Instant::now() + std::time::Duration::from_secs(POSTGRES_CHECKOUT_LOCK_TIMEOUT_SECONDS);
    loop {
        let mut transaction = pool.begin().await.map_err(|error| {
            crate::shared::store_error("failed to begin payment checkout lock transaction", error)
        })?;
        let acquired = sqlx::query_scalar::<_, bool>(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
        )
        .bind(lock_key)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| {
            crate::shared::store_error("failed to acquire payment checkout advisory lock", error)
        })?;
        if acquired {
            return Ok(transaction);
        }
        transaction.rollback().await.map_err(|error| {
            crate::shared::store_error("failed to roll back payment checkout lock attempt", error)
        })?;
        if Instant::now() >= deadline {
            return Err(CommerceServiceError::locked(
                "payment checkout is already being processed",
            ));
        }
        sleep(std::time::Duration::from_millis(
            POSTGRES_CHECKOUT_LOCK_RETRY_MILLIS,
        ))
        .await;
    }
}

async fn release_postgres_checkout_lock(
    transaction: Transaction<'static, Postgres>,
) -> Result<(), CommerceServiceError> {
    transaction.commit().await.map_err(|error| {
        crate::shared::store_error("failed to release payment checkout advisory lock", error)
    })
}

fn sqlite_checkout_lock(key: String) -> Arc<CheckoutMutex> {
    let locks = SQLITE_CHECKOUT_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = locks
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(&key).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(CheckoutMutex::new(()));
    locks.insert(key, Arc::downgrade(&lock));
    lock
}

pub async fn enrich_owner_payment_attempt_sqlite(
    pool: &Pool<Sqlite>,
    context: OwnerOrderPaymentEnrichmentContext<'_>,
    outcome: CreateOwnerPaymentAttemptOutcome,
) -> Result<CreateOwnerPaymentAttemptOutcome, CommerceServiceError> {
    let pay_outcome = attempt_outcome_to_pay_outcome(&outcome);
    let enriched = enrich_owner_order_payment_sqlite(pool, context, pay_outcome).await?;
    Ok(merge_attempt_payment_params(
        outcome,
        enriched.payment_params,
    ))
}

pub async fn enrich_owner_payment_attempt_postgres(
    pool: &PgPool,
    context: OwnerOrderPaymentEnrichmentContext<'_>,
    outcome: CreateOwnerPaymentAttemptOutcome,
) -> Result<CreateOwnerPaymentAttemptOutcome, CommerceServiceError> {
    let pay_outcome = attempt_outcome_to_pay_outcome(&outcome);
    let enriched = enrich_owner_order_payment_postgres(pool, context, pay_outcome).await?;
    Ok(merge_attempt_payment_params(
        outcome,
        enriched.payment_params,
    ))
}

fn attempt_outcome_to_pay_outcome(
    outcome: &CreateOwnerPaymentAttemptOutcome,
) -> PayOwnerOrderOutcome {
    let mut payment_params = outcome.payment_params.clone();
    payment_params
        .entry("providerCode".to_owned())
        .or_insert_with(|| outcome.provider_code.clone());
    PayOwnerOrderOutcome {
        amount: outcome.amount.clone(),
        order_id: outcome.order_id.clone(),
        out_trade_no: outcome.out_trade_no.clone(),
        payment_id: outcome.attempt_id.clone(),
        payment_method: outcome.payment_method.clone(),
        status: outcome.status.clone(),
        payment_params,
    }
}

fn merge_attempt_payment_params(
    mut outcome: CreateOwnerPaymentAttemptOutcome,
    payment_params: std::collections::BTreeMap<String, String>,
) -> CreateOwnerPaymentAttemptOutcome {
    outcome.payment_params = payment_params;
    outcome
}

async fn load_active_attempt_owner_user_id_sqlite(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    attempt_id: &str,
) -> Result<String, CommerceServiceError> {
    let row = sqlx::query(
        "SELECT owner_user_id FROM commerce_payment_attempt WHERE tenant_id = CAST(? AS TEXT) AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL)) AND order_id = CAST(? AS TEXT) AND id = CAST(? AS TEXT) AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing') AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(organization_id)
    .bind(order_id)
    .bind(attempt_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        crate::shared::store_error("failed to load active payment attempt owner", error)
    })?
    .ok_or_else(|| {
        CommerceServiceError::conflict(
            "payment attempt was superseded while checkout was being prepared",
        )
    })?;
    Ok(sqlx::Row::try_get::<String, _>(&row, "owner_user_id").unwrap_or_default())
}

async fn load_active_attempt_owner_user_id_postgres(
    pool: &PgPool,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    attempt_id: &str,
) -> Result<String, CommerceServiceError> {
    let row = sqlx::query(
        "SELECT owner_user_id FROM commerce_payment_attempt WHERE tenant_id = CAST($1 AS TEXT) AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2::text IS NULL)) AND order_id = CAST($3 AS TEXT) AND id = CAST($4 AS TEXT) AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing') AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(order_id)
    .bind(attempt_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        crate::shared::store_error("failed to load active payment attempt owner", error)
    })?
    .ok_or_else(|| {
        CommerceServiceError::conflict(
            "payment attempt was superseded while checkout was being prepared",
        )
    })?;
    Ok(sqlx::Row::try_get::<String, _>(&row, "owner_user_id").unwrap_or_default())
}

async fn enrich_owner_order_payment_outcome(
    context: &OwnerOrderPaymentEnrichmentContext<'_>,
    account: Option<ProviderAccountBinding>,
    provider_code: &str,
    idempotency_key: &str,
    payment_metadata: Option<&serde_json::Value>,
    outcome: PayOwnerOrderOutcome,
    expires_at: Option<&str>,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let registry = match account {
        Some(binding) => provider_registry_for_account(context.credentials, Some(binding)),
        None => context.deployment_registry.clone(),
    };
    let checkout_context = provider_checkout_context(
        context,
        provider_code,
        idempotency_key,
        payment_metadata,
        expires_at,
    );
    enrich_pay_owner_order_outcome(&registry, &checkout_context, outcome).await
}

fn provider_checkout_context(
    context: &OwnerOrderPaymentEnrichmentContext<'_>,
    provider_code: &str,
    idempotency_key: &str,
    payment_metadata: Option<&serde_json::Value>,
    expires_at: Option<&str>,
) -> CheckoutContext {
    let notify_url = context
        .credentials
        .provider_notify_url(&normalize_provider_code(provider_code));
    CheckoutContext {
        provider_code: provider_code.to_owned(),
        currency_code: "CNY".to_owned(),
        tenant_id: context.tenant_id.to_owned(),
        order_id: context.order_id.to_owned(),
        idempotency_key: idempotency_key.to_owned(),
        expires_at: expires_at.map(str::to_owned),
        notify_url,
        payment_scene: context.payment_scene.map(str::to_owned),
        payment_metadata: payment_metadata.cloned(),
    }
}

fn ensure_provider_attempt_snapshot(
    attempt: &crate::payment_attempt_context::PaymentAttemptProviderContext,
    outcome: &PayOwnerOrderOutcome,
) -> Result<(), CommerceServiceError> {
    if attempt.idempotency_key.trim().is_empty() {
        return Err(CommerceServiceError::storage(
            "payment attempt is missing its persisted idempotency key",
        ));
    }
    if attempt.out_trade_no != outcome.out_trade_no {
        return Err(CommerceServiceError::conflict(
            "payment attempt changed while checkout was being prepared",
        ));
    }
    Ok(())
}

fn provider_checkout_expiration(
    order_expires_at: Option<&str>,
) -> Result<String, CommerceServiceError> {
    let now = Utc::now();
    let provider_limit = now + Duration::seconds(PROVIDER_CHECKOUT_TTL_SECONDS);
    let expires_at = match order_expires_at
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => DateTime::parse_from_rfc3339(value)
            .map_err(|_| CommerceServiceError::conflict("payment attempt expiry is invalid"))?
            .with_timezone(&Utc)
            .min(provider_limit),
        None => provider_limit,
    };
    if expires_at <= now {
        return Err(CommerceServiceError::conflict(
            "payment attempt has expired",
        ));
    }
    Ok(expires_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

async fn provider_checkout_expiration_sqlite(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    owner_user_id: &str,
    attempt_id: &str,
) -> Result<Option<String>, CommerceServiceError> {
    let row = sqlx::query(
        "SELECT expires_at FROM commerce_payment_attempt WHERE tenant_id = CAST(? AS TEXT) AND owner_user_id = CAST(? AS TEXT) AND id = CAST(? AS TEXT) AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing') AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .bind(owner_user_id)
    .bind(attempt_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| crate::shared::store_error("failed to load payment attempt expiry", error))?
    .ok_or_else(|| {
        CommerceServiceError::conflict(
            "payment attempt changed while checkout expiry was being loaded",
        )
    })?;
    let order_expires_at =
        sqlx::Row::try_get::<Option<String>, _>(&row, "expires_at").unwrap_or_default();
    let expires_at = provider_checkout_expiration(order_expires_at.as_deref())?;
    let update = sqlx::query(
        "UPDATE commerce_payment_attempt SET expires_at = ?, updated_at = ? WHERE tenant_id = CAST(? AS TEXT) AND id = CAST(? AS TEXT) AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing') AND deleted_at IS NULL",
    )
    .bind(&expires_at)
    .bind(crate::shared::current_timestamp_string())
    .bind(tenant_id)
    .bind(attempt_id)
    .execute(pool)
    .await
    .map_err(|error| crate::shared::store_error("failed to persist payment attempt expiry", error))?;
    ensure_attempt_expiry_persisted(update.rows_affected())?;
    Ok(Some(expires_at))
}

async fn provider_checkout_expiration_postgres(
    pool: &PgPool,
    tenant_id: &str,
    owner_user_id: &str,
    attempt_id: &str,
) -> Result<Option<String>, CommerceServiceError> {
    let row = sqlx::query(
        "SELECT to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at FROM commerce_payment_attempt WHERE tenant_id = CAST($1 AS TEXT) AND owner_user_id = CAST($2 AS TEXT) AND id = CAST($3 AS TEXT) AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing') AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .bind(owner_user_id)
    .bind(attempt_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| crate::shared::store_error("failed to load payment attempt expiry", error))?
    .ok_or_else(|| {
        CommerceServiceError::conflict(
            "payment attempt changed while checkout expiry was being loaded",
        )
    })?;
    let order_expires_at =
        sqlx::Row::try_get::<Option<String>, _>(&row, "expires_at").unwrap_or_default();
    let expires_at = provider_checkout_expiration(order_expires_at.as_deref())?;
    let update = sqlx::query(
        "UPDATE commerce_payment_attempt SET expires_at = $1::timestamptz, updated_at = $2::timestamptz WHERE tenant_id = CAST($3 AS TEXT) AND id = CAST($4 AS TEXT) AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing') AND deleted_at IS NULL",
    )
    .bind(&expires_at)
    .bind(crate::shared::current_timestamp_string())
    .bind(tenant_id)
    .bind(attempt_id)
    .execute(pool)
    .await
    .map_err(|error| crate::shared::store_error("failed to persist payment attempt expiry", error))?;
    ensure_attempt_expiry_persisted(update.rows_affected())?;
    Ok(Some(expires_at))
}

fn ensure_attempt_expiry_persisted(rows_affected: u64) -> Result<(), CommerceServiceError> {
    if rows_affected == 1 {
        Ok(())
    } else {
        Err(CommerceServiceError::conflict(
            "payment attempt changed while checkout expiry was being persisted",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn checkout_lock_key_is_unambiguous_and_preserves_missing_organization() {
        assert_ne!(
            checkout_lock_key_from_parts("tenant:a", Some("org"), "order"),
            checkout_lock_key_from_parts("tenant", Some("a:org"), "order")
        );
        assert_ne!(
            checkout_lock_key_from_parts("tenant", None, "order"),
            checkout_lock_key_from_parts("tenant", Some("0"), "order")
        );
    }

    #[test]
    fn provider_checkout_expiration_rejects_invalid_and_expired_boundaries() {
        assert!(provider_checkout_expiration(Some("not-a-timestamp")).is_err());
        assert!(provider_checkout_expiration(Some("2000-01-01T00:00:00Z")).is_err());
    }

    #[test]
    fn provider_checkout_expiration_is_capped_at_fifteen_minutes() {
        let before = Utc::now();
        let expiration = provider_checkout_expiration(Some("2099-01-01T00:00:00Z"))
            .expect("far-future order should remain payable");
        let expiration = DateTime::parse_from_rfc3339(&expiration)
            .expect("provider expiration should be RFC3339")
            .with_timezone(&Utc);
        let after = Utc::now();

        assert!(expiration >= before + Duration::seconds(PROVIDER_CHECKOUT_TTL_SECONDS - 1));
        assert!(expiration <= after + Duration::seconds(PROVIDER_CHECKOUT_TTL_SECONDS));
    }

    #[test]
    fn provider_checkout_expiration_preserves_an_earlier_order_boundary() {
        let order_expiration = Utc::now() + Duration::minutes(5);
        let value = order_expiration.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let expiration = provider_checkout_expiration(Some(&value))
            .expect("unexpired order boundary should be accepted");

        assert_eq!(expiration, value);
    }

    #[test]
    fn provider_checkout_uses_the_persisted_attempt_snapshot() {
        let credentials = ProviderCredentialBundle {
            stripe: None,
            alipay: None,
            wechat_pay: None,
            webhook_base_url: None,
        };
        let registry = PaymentProviderRegistry::from_credentials(credentials.clone());
        let metadata = serde_json::json!({"openid": "persisted-payer"});
        let context = OwnerOrderPaymentEnrichmentContext {
            deployment_registry: &registry,
            credentials: &credentials,
            tenant_id: "tenant-1",
            organization_id: Some("org-1"),
            order_id: "order-1",
            payment_scene: Some("mini_program"),
        };

        let checkout = provider_checkout_context(
            &context,
            "wechat_pay",
            "persisted-attempt-idempotency",
            Some(&metadata),
            Some("2099-01-01T00:00:00Z"),
        );

        assert_eq!(checkout.idempotency_key, "persisted-attempt-idempotency");
        assert_eq!(checkout.payment_metadata, Some(metadata));
        assert_eq!(checkout.payment_scene.as_deref(), Some("mini_program"));
    }

    #[tokio::test]
    async fn superseded_attempt_cannot_create_a_provider_checkout() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("checkout race test pool");
        sqlx::query(
            "CREATE TABLE commerce_payment_attempt (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, owner_user_id TEXT NOT NULL, order_id TEXT NOT NULL, status TEXT NOT NULL, deleted_at TEXT)",
        )
        .execute(&pool)
        .await
        .expect("checkout race test schema");
        sqlx::query(
            "INSERT INTO commerce_payment_attempt (id, tenant_id, organization_id, owner_user_id, order_id, status) VALUES ('attempt-loser', 'tenant-1', 'org-1', 'user-1', 'order-1', 'canceled')",
        )
        .execute(&pool)
        .await
        .expect("superseded checkout attempt");
        let credentials = ProviderCredentialBundle {
            stripe: None,
            alipay: None,
            wechat_pay: None,
            webhook_base_url: None,
        };
        let registry = PaymentProviderRegistry::from_credentials(credentials.clone());
        let outcome = PayOwnerOrderOutcome {
            amount: sdkwork_contract_service::CommerceMoney::new("100")
                .expect("valid checkout amount"),
            order_id: "order-1".to_owned(),
            out_trade_no: "trade-loser".to_owned(),
            payment_id: "attempt-loser".to_owned(),
            payment_method: "sandbox".to_owned(),
            status: "pending".to_owned(),
            payment_params: std::collections::BTreeMap::new(),
        };

        let error = enrich_owner_order_payment_sqlite_locked(
            &pool,
            OwnerOrderPaymentEnrichmentContext {
                deployment_registry: &registry,
                credentials: &credentials,
                tenant_id: "tenant-1",
                organization_id: Some("org-1"),
                order_id: "order-1",
                payment_scene: None,
            },
            outcome,
        )
        .await
        .expect_err("superseded attempt must not reach provider enrichment");

        assert_eq!(error.code(), "conflict");
        assert_eq!(
            error.message(),
            "payment attempt was superseded while checkout was being prepared"
        );
    }

    #[tokio::test]
    async fn concurrent_sqlite_checkout_allows_only_one_attempt_to_reach_enrichment() {
        let pool = sqlite_checkout_race_pool().await;
        seed_checkout_attempt(&pool, "intent-a", "attempt-a", "trade-a").await;
        seed_checkout_attempt(&pool, "intent-b", "attempt-b", "trade-b").await;
        let credentials = ProviderCredentialBundle {
            stripe: None,
            alipay: None,
            wechat_pay: None,
            webhook_base_url: None,
        };
        let registry = PaymentProviderRegistry::from_credentials(credentials.clone());
        let first = checkout_race_outcome("attempt-a", "trade-a");
        let second = checkout_race_outcome("attempt-b", "trade-b");

        let (first_result, second_result) = tokio::join!(
            enrich_owner_order_payment_sqlite(
                &pool,
                checkout_race_context(&registry, &credentials),
                first,
            ),
            enrich_owner_order_payment_sqlite(
                &pool,
                checkout_race_context(&registry, &credentials),
                second,
            ),
        );

        assert_eq!(
            usize::from(first_result.is_ok()) + usize::from(second_result.is_ok()),
            1,
            "exactly one concurrent attempt may reach provider enrichment"
        );
        let loser = first_result.err().or_else(|| second_result.err()).expect(
            "one concurrent attempt should be superseded after the winner closes conflicts",
        );
        assert_eq!(loser.code(), "conflict");
        assert_eq!(
            loser.message(),
            "payment attempt was superseded while checkout was being prepared"
        );

        let statuses = sqlx::query_scalar::<_, String>(
            "SELECT status FROM commerce_payment_attempt ORDER BY status",
        )
        .fetch_all(&pool)
        .await
        .expect("checkout race attempt statuses");
        assert_eq!(statuses, vec!["canceled".to_owned(), "pending".to_owned()]);
        let enriched_attempts = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM commerce_payment_attempt WHERE expires_at IS NOT NULL",
        )
        .fetch_one(&pool)
        .await
        .expect("checkout race enriched attempt count");
        assert_eq!(enriched_attempts, 1);
    }

    fn checkout_race_context<'a>(
        registry: &'a PaymentProviderRegistry,
        credentials: &'a ProviderCredentialBundle,
    ) -> OwnerOrderPaymentEnrichmentContext<'a> {
        OwnerOrderPaymentEnrichmentContext {
            deployment_registry: registry,
            credentials,
            tenant_id: "tenant-1",
            organization_id: Some("org-1"),
            order_id: "order-1",
            payment_scene: None,
        }
    }

    fn checkout_race_outcome(attempt_id: &str, out_trade_no: &str) -> PayOwnerOrderOutcome {
        let mut payment_params = std::collections::BTreeMap::new();
        payment_params.insert("providerCode".to_owned(), "sandbox".to_owned());
        PayOwnerOrderOutcome {
            amount: sdkwork_contract_service::CommerceMoney::new("100")
                .expect("valid checkout amount"),
            order_id: "order-1".to_owned(),
            out_trade_no: out_trade_no.to_owned(),
            payment_id: attempt_id.to_owned(),
            payment_method: "sandbox".to_owned(),
            status: "pending".to_owned(),
            payment_params,
        }
    }

    async fn sqlite_checkout_race_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("checkout race sqlite pool");
        for statement in [
            "CREATE TABLE commerce_order (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, owner_user_id TEXT NOT NULL, status TEXT NOT NULL, expired_at TEXT)",
            "CREATE TABLE commerce_payment_channel (id TEXT PRIMARY KEY, provider_account_id TEXT)",
            "CREATE TABLE commerce_payment_provider_account (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, provider_code TEXT NOT NULL, merchant_id TEXT, environment TEXT NOT NULL DEFAULT 'sandbox', secret_ref TEXT NOT NULL DEFAULT '', webhook_secret_ref TEXT, certificate_ref TEXT, metadata TEXT NOT NULL DEFAULT '{}', status TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT)",
            "CREATE TABLE commerce_payment_intent (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, status TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT)",
            "CREATE TABLE commerce_payment_attempt (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, owner_user_id TEXT NOT NULL, payment_intent_id TEXT NOT NULL, order_id TEXT NOT NULL, provider_code TEXT NOT NULL, channel_id TEXT, out_trade_no TEXT NOT NULL, amount TEXT NOT NULL, callback_payload TEXT NOT NULL, idempotency_key TEXT NOT NULL, status TEXT NOT NULL, expires_at TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT)",
        ] {
            sqlx::query(statement)
                .execute(&pool)
                .await
                .expect("checkout race test schema");
        }
        sqlx::query(
            "INSERT INTO commerce_order (id, tenant_id, organization_id, owner_user_id, status, expired_at) VALUES ('order-1', 'tenant-1', 'org-1', 'user-1', 'pending_payment', '2099-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("checkout race order");
        pool
    }

    async fn seed_checkout_attempt(
        pool: &sqlx::SqlitePool,
        intent_id: &str,
        attempt_id: &str,
        out_trade_no: &str,
    ) {
        sqlx::query(
            "INSERT INTO commerce_payment_intent (id, tenant_id, status, updated_at) VALUES (?, 'tenant-1', 'pending', '2026-07-26T00:00:00Z')",
        )
        .bind(intent_id)
        .execute(pool)
        .await
        .expect("checkout race payment intent");
        sqlx::query(
            "INSERT INTO commerce_payment_attempt (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id, provider_code, out_trade_no, amount, callback_payload, idempotency_key, status, created_at, updated_at) VALUES (?, 'tenant-1', 'org-1', 'user-1', ?, 'order-1', 'sandbox', ?, '100', '{}', ?, 'pending', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
        )
        .bind(attempt_id)
        .bind(intent_id)
        .bind(out_trade_no)
        .bind(format!("idem-{attempt_id}"))
        .execute(pool)
        .await
        .expect("checkout race payment attempt");
    }
}
