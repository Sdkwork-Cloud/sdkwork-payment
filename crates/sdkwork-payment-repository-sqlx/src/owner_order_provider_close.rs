use sdkwork_contract_service::{CommercePaymentStatus, CommerceServiceError};
use sdkwork_payment_providers::{
    cancel_provider_payment, provider_registry_for_account, PaymentProviderRegistry,
    ProviderCredentialBundle,
};
use sdkwork_payment_service::CancelOrderPaymentsCommand;
use sqlx::{PgPool, SqlitePool};

use crate::owner_order_checkout::provider_account_binding;
use crate::payment_attempt_context::{
    load_payment_attempt_provider_context_postgres, load_payment_attempt_provider_context_sqlite,
};
use crate::provider_account::{
    ensure_provider_account_matches, load_active_provider_account_postgres,
    load_active_provider_account_sqlite, load_provider_account_for_existing_payment_postgres,
    load_provider_account_for_existing_payment_sqlite,
};
use crate::shared::{current_timestamp_string, store_error, string_cell};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveAttemptIdentity {
    attempt_id: String,
    payment_intent_id: String,
}

pub(crate) async fn cancel_owner_order_payments_with_provider_sqlite_unlocked(
    pool: &SqlitePool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    command: CancelOrderPaymentsCommand,
) -> Result<(), CommerceServiceError> {
    close_owner_order_provider_attempts_sqlite(
        pool,
        deployment_registry,
        credentials,
        &command.tenant_id,
        command.organization_id.as_deref(),
        &command.owner_user_id,
        &command.order_id,
        None,
    )
    .await?;
    crate::SqliteCommerceOwnerOrderPaymentStore::new(pool.clone())
        .cancel_order_payments(command)
        .await
}

pub(crate) async fn cancel_owner_order_payments_with_provider_postgres_unlocked(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    command: CancelOrderPaymentsCommand,
) -> Result<(), CommerceServiceError> {
    close_owner_order_provider_attempts_postgres(
        pool,
        deployment_registry,
        credentials,
        &command.tenant_id,
        command.organization_id.as_deref(),
        &command.owner_user_id,
        &command.order_id,
        None,
    )
    .await?;
    crate::PostgresCommerceOwnerOrderPaymentStore::new(pool.clone())
        .cancel_order_payments(command)
        .await
}

pub async fn close_owner_order_provider_attempts_sqlite(
    pool: &SqlitePool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: &str,
    excluded_attempt_id: Option<&str>,
) -> Result<(), CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT id, payment_intent_id
        FROM commerce_payment_attempt
        WHERE tenant_id = CAST(? AS TEXT)
          AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
          AND owner_user_id = CAST(? AS TEXT)
          AND order_id = CAST(? AS TEXT)
          AND (? IS NULL OR id <> CAST(? AS TEXT))
          AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
          AND deleted_at IS NULL
        ORDER BY created_at, id
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(organization_id)
    .bind(owner_user_id)
    .bind(order_id)
    .bind(excluded_attempt_id)
    .bind(excluded_attempt_id)
    .fetch_all(pool)
    .await
    .map_err(|error| store_error("failed to load active order payment attempts", error))?;

    let attempts = rows
        .iter()
        .map(|row| ActiveAttemptIdentity {
            attempt_id: string_cell(row, "id"),
            payment_intent_id: string_cell(row, "payment_intent_id"),
        })
        .collect::<Vec<_>>();
    close_attempts_sqlite(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        owner_user_id,
        attempts,
    )
    .await
}

pub async fn close_owner_order_provider_attempts_postgres(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: &str,
    excluded_attempt_id: Option<&str>,
) -> Result<(), CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT id, payment_intent_id
        FROM commerce_payment_attempt
        WHERE tenant_id = CAST($1 AS TEXT)
          AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2::text IS NULL))
          AND owner_user_id = CAST($3 AS TEXT)
          AND order_id = CAST($4 AS TEXT)
          AND ($5::text IS NULL OR id <> CAST($5 AS TEXT))
          AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
          AND deleted_at IS NULL
        ORDER BY created_at, id
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(owner_user_id)
    .bind(order_id)
    .bind(excluded_attempt_id)
    .fetch_all(pool)
    .await
    .map_err(|error| store_error("failed to load active order payment attempts", error))?;

    let attempts = rows
        .iter()
        .map(|row| ActiveAttemptIdentity {
            attempt_id: string_cell(row, "id"),
            payment_intent_id: string_cell(row, "payment_intent_id"),
        })
        .collect::<Vec<_>>();
    close_attempts_postgres(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        owner_user_id,
        attempts,
    )
    .await
}

pub async fn close_expired_owner_order_provider_attempts_sqlite(
    pool: &SqlitePool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
) -> Result<(), CommerceServiceError> {
    close_expired_owner_order_provider_attempts_sqlite_scoped(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        owner_user_id,
        None,
    )
    .await
}

pub(crate) async fn close_expired_owner_order_provider_attempts_for_order_sqlite(
    pool: &SqlitePool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: &str,
) -> Result<(), CommerceServiceError> {
    close_expired_owner_order_provider_attempts_sqlite_scoped(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        owner_user_id,
        Some(order_id),
    )
    .await
}

async fn close_expired_owner_order_provider_attempts_sqlite_scoped(
    pool: &SqlitePool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: Option<&str>,
) -> Result<(), CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT pa.id, pa.payment_intent_id
        FROM commerce_payment_attempt pa
        INNER JOIN commerce_order o
          ON o.tenant_id = pa.tenant_id
         AND o.id = pa.order_id
         AND o.owner_user_id = pa.owner_user_id
        WHERE pa.tenant_id = CAST(? AS TEXT)
          AND ((pa.organization_id = CAST(? AS TEXT)) OR (pa.organization_id IS NULL AND ? IS NULL))
          AND pa.owner_user_id = CAST(? AS TEXT)
          AND (? IS NULL OR pa.order_id = CAST(? AS TEXT))
          AND LOWER(COALESCE(pa.status, '')) IN ('created', 'pending', 'processing')
          AND pa.deleted_at IS NULL
          AND (
            LOWER(COALESCE(o.status, '')) IN ('expired', 'closed', 'cancelled', 'canceled')
            OR (o.expired_at IS NOT NULL AND o.expired_at <> '' AND datetime(o.expired_at) <= datetime('now'))
          )
        ORDER BY pa.created_at, pa.id
        LIMIT 100
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(organization_id)
    .bind(owner_user_id)
    .bind(order_id)
    .bind(order_id)
    .fetch_all(pool)
    .await
    .map_err(|error| store_error("failed to load expired order payment attempts", error))?;
    let attempts = rows
        .iter()
        .map(|row| ActiveAttemptIdentity {
            attempt_id: string_cell(row, "id"),
            payment_intent_id: string_cell(row, "payment_intent_id"),
        })
        .collect::<Vec<_>>();
    close_attempts_sqlite(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        owner_user_id,
        attempts,
    )
    .await
}

pub async fn close_expired_owner_order_provider_attempts_postgres(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
) -> Result<(), CommerceServiceError> {
    close_expired_owner_order_provider_attempts_postgres_scoped(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        owner_user_id,
        None,
    )
    .await
}

pub(crate) async fn close_expired_owner_order_provider_attempts_for_order_postgres(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: &str,
) -> Result<(), CommerceServiceError> {
    close_expired_owner_order_provider_attempts_postgres_scoped(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        owner_user_id,
        Some(order_id),
    )
    .await
}

async fn close_expired_owner_order_provider_attempts_postgres_scoped(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    order_id: Option<&str>,
) -> Result<(), CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT pa.id, pa.payment_intent_id
        FROM commerce_payment_attempt pa
        INNER JOIN commerce_order o
          ON o.tenant_id = pa.tenant_id
         AND o.id = pa.order_id
         AND o.owner_user_id = pa.owner_user_id
        WHERE pa.tenant_id = CAST($1 AS TEXT)
          AND ((pa.organization_id = CAST($2 AS TEXT)) OR (pa.organization_id IS NULL AND $2::text IS NULL))
          AND pa.owner_user_id = CAST($3 AS TEXT)
          AND ($4::text IS NULL OR pa.order_id = CAST($4 AS TEXT))
          AND LOWER(COALESCE(pa.status, '')) IN ('created', 'pending', 'processing')
          AND pa.deleted_at IS NULL
          AND (
            LOWER(COALESCE(o.status, '')) IN ('expired', 'closed', 'cancelled', 'canceled')
            OR (o.expired_at IS NOT NULL AND o.expired_at <= CURRENT_TIMESTAMP)
          )
        ORDER BY pa.created_at, pa.id
        LIMIT 100
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(owner_user_id)
    .bind(order_id)
    .fetch_all(pool)
    .await
    .map_err(|error| store_error("failed to load expired order payment attempts", error))?;
    let attempts = rows
        .iter()
        .map(|row| ActiveAttemptIdentity {
            attempt_id: string_cell(row, "id"),
            payment_intent_id: string_cell(row, "payment_intent_id"),
        })
        .collect::<Vec<_>>();
    close_attempts_postgres(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        owner_user_id,
        attempts,
    )
    .await
}

async fn close_attempts_sqlite(
    pool: &SqlitePool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    attempts: Vec<ActiveAttemptIdentity>,
) -> Result<(), CommerceServiceError> {
    for attempt in attempts {
        let Some(context) = load_payment_attempt_provider_context_sqlite(
            pool,
            tenant_id,
            owner_user_id,
            &attempt.attempt_id,
        )
        .await?
        else {
            continue;
        };
        let registry = registry_for_existing_attempt_sqlite(
            pool,
            deployment_registry,
            credentials,
            tenant_id,
            organization_id,
            &context,
        )
        .await?;
        cancel_provider_payment(
            &registry,
            &context.provider_code,
            &context.out_trade_no,
            context.provider_transaction_id.as_deref(),
        )
        .await?;
        mark_attempt_canceled_sqlite(pool, tenant_id, &attempt).await?;
    }
    Ok(())
}

async fn close_attempts_postgres(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    owner_user_id: &str,
    attempts: Vec<ActiveAttemptIdentity>,
) -> Result<(), CommerceServiceError> {
    for attempt in attempts {
        let Some(context) = load_payment_attempt_provider_context_postgres(
            pool,
            tenant_id,
            owner_user_id,
            &attempt.attempt_id,
        )
        .await?
        else {
            continue;
        };
        let registry = registry_for_existing_attempt_postgres(
            pool,
            deployment_registry,
            credentials,
            tenant_id,
            organization_id,
            &context,
        )
        .await?;
        cancel_provider_payment(
            &registry,
            &context.provider_code,
            &context.out_trade_no,
            context.provider_transaction_id.as_deref(),
        )
        .await?;
        mark_attempt_canceled_postgres(pool, tenant_id, &attempt).await?;
    }
    Ok(())
}

async fn registry_for_existing_attempt_sqlite(
    pool: &SqlitePool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    context: &crate::PaymentAttemptProviderContext,
) -> Result<PaymentProviderRegistry, CommerceServiceError> {
    if context.provider_code.trim().eq_ignore_ascii_case("sandbox") {
        return Ok(deployment_registry.clone());
    }
    let account = match context.provider_account_id.as_deref() {
        Some(account_id) => Some(
            load_provider_account_for_existing_payment_sqlite(
                pool,
                tenant_id,
                organization_id,
                account_id,
            )
            .await?
            .ok_or_else(|| {
                CommerceServiceError::conflict(
                    "original payment provider account is unavailable for close",
                )
            })?,
        ),
        None if context.channel_id.is_some() => None,
        None => {
            load_active_provider_account_sqlite(
                pool,
                tenant_id,
                organization_id,
                &context.provider_code,
            )
            .await?
        }
    };
    ensure_provider_account_matches(account.as_ref(), &context.provider_code)?;
    Ok(match account {
        Some(account) => {
            provider_registry_for_account(credentials, Some(provider_account_binding(&account)))
        }
        None => deployment_registry.clone(),
    })
}

async fn registry_for_existing_attempt_postgres(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    context: &crate::PaymentAttemptProviderContext,
) -> Result<PaymentProviderRegistry, CommerceServiceError> {
    if context.provider_code.trim().eq_ignore_ascii_case("sandbox") {
        return Ok(deployment_registry.clone());
    }
    let account = match context.provider_account_id.as_deref() {
        Some(account_id) => Some(
            load_provider_account_for_existing_payment_postgres(
                pool,
                tenant_id,
                organization_id,
                account_id,
            )
            .await?
            .ok_or_else(|| {
                CommerceServiceError::conflict(
                    "original payment provider account is unavailable for close",
                )
            })?,
        ),
        None if context.channel_id.is_some() => None,
        None => {
            load_active_provider_account_postgres(
                pool,
                tenant_id,
                organization_id,
                &context.provider_code,
            )
            .await?
        }
    };
    ensure_provider_account_matches(account.as_ref(), &context.provider_code)?;
    Ok(match account {
        Some(account) => {
            provider_registry_for_account(credentials, Some(provider_account_binding(&account)))
        }
        None => deployment_registry.clone(),
    })
}

async fn mark_attempt_canceled_sqlite(
    pool: &SqlitePool,
    tenant_id: &str,
    attempt: &ActiveAttemptIdentity,
) -> Result<(), CommerceServiceError> {
    let now = current_timestamp_string();
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| store_error("failed to begin payment attempt close transaction", error))?;
    let attempt_update = sqlx::query(
        r#"
        UPDATE commerce_payment_attempt
        SET status = ?, updated_at = ?
        WHERE tenant_id = CAST(? AS TEXT)
          AND id = CAST(? AS TEXT)
          AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
          AND deleted_at IS NULL
        "#,
    )
    .bind(CommercePaymentStatus::Canceled.as_str())
    .bind(&now)
    .bind(tenant_id)
    .bind(&attempt.attempt_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| store_error("failed to close payment attempt", error))?;
    let persisted_status = if attempt_update.rows_affected() == 0 {
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM commerce_payment_attempt WHERE tenant_id = CAST(? AS TEXT) AND id = CAST(? AS TEXT) AND deleted_at IS NULL",
        )
        .bind(tenant_id)
        .bind(&attempt.attempt_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to verify closed payment attempt", error))?
    } else {
        None
    };
    ensure_attempt_close_persisted(attempt_update.rows_affected(), persisted_status.as_deref())?;
    sqlx::query(
        r#"
        UPDATE commerce_payment_intent
        SET status = ?, updated_at = ?
        WHERE tenant_id = CAST(? AS TEXT)
          AND id = CAST(? AS TEXT)
          AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
          AND deleted_at IS NULL
          AND NOT EXISTS (
            SELECT 1 FROM commerce_payment_attempt pa
            WHERE pa.tenant_id = commerce_payment_intent.tenant_id
              AND pa.payment_intent_id = commerce_payment_intent.id
              AND LOWER(COALESCE(pa.status, '')) IN ('created', 'pending', 'processing')
              AND pa.deleted_at IS NULL
          )
        "#,
    )
    .bind(CommercePaymentStatus::Canceled.as_str())
    .bind(&now)
    .bind(tenant_id)
    .bind(&attempt.payment_intent_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| store_error("failed to close payment intent", error))?;
    tx.commit()
        .await
        .map_err(|error| store_error("failed to commit payment attempt close", error))
}

async fn mark_attempt_canceled_postgres(
    pool: &PgPool,
    tenant_id: &str,
    attempt: &ActiveAttemptIdentity,
) -> Result<(), CommerceServiceError> {
    let now = current_timestamp_string();
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| store_error("failed to begin payment attempt close transaction", error))?;
    let attempt_update = sqlx::query(
        r#"
        UPDATE commerce_payment_attempt
        SET status = $1, updated_at = $2::timestamptz
        WHERE tenant_id = CAST($3 AS TEXT)
          AND id = CAST($4 AS TEXT)
          AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
          AND deleted_at IS NULL
        "#,
    )
    .bind(CommercePaymentStatus::Canceled.as_str())
    .bind(&now)
    .bind(tenant_id)
    .bind(&attempt.attempt_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| store_error("failed to close payment attempt", error))?;
    let persisted_status = if attempt_update.rows_affected() == 0 {
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM commerce_payment_attempt WHERE tenant_id = CAST($1 AS TEXT) AND id = CAST($2 AS TEXT) AND deleted_at IS NULL",
        )
        .bind(tenant_id)
        .bind(&attempt.attempt_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to verify closed payment attempt", error))?
    } else {
        None
    };
    ensure_attempt_close_persisted(attempt_update.rows_affected(), persisted_status.as_deref())?;
    sqlx::query(
        r#"
        UPDATE commerce_payment_intent pi
        SET status = $1, updated_at = $2::timestamptz
        WHERE pi.tenant_id = CAST($3 AS TEXT)
          AND pi.id = CAST($4 AS TEXT)
          AND LOWER(COALESCE(pi.status, '')) IN ('created', 'pending', 'processing')
          AND pi.deleted_at IS NULL
          AND NOT EXISTS (
            SELECT 1 FROM commerce_payment_attempt pa
            WHERE pa.tenant_id = pi.tenant_id
              AND pa.payment_intent_id = pi.id
              AND LOWER(COALESCE(pa.status, '')) IN ('created', 'pending', 'processing')
              AND pa.deleted_at IS NULL
          )
        "#,
    )
    .bind(CommercePaymentStatus::Canceled.as_str())
    .bind(&now)
    .bind(tenant_id)
    .bind(&attempt.payment_intent_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| store_error("failed to close payment intent", error))?;
    tx.commit()
        .await
        .map_err(|error| store_error("failed to commit payment attempt close", error))
}

fn ensure_attempt_close_persisted(
    rows_affected: u64,
    persisted_status: Option<&str>,
) -> Result<(), CommerceServiceError> {
    match rows_affected {
        1 => Ok(()),
        0 => match persisted_status
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("canceled" | "cancelled" | "closed") => Ok(()),
            Some("succeeded" | "success" | "paid") => Err(CommerceServiceError::conflict(
                "payment attempt completed while it was being closed",
            )),
            Some(status) => Err(CommerceServiceError::storage(format!(
                "payment attempt close was not persisted from status {status}"
            ))),
            None => Err(CommerceServiceError::storage(
                "payment attempt disappeared while it was being closed",
            )),
        },
        count => Err(CommerceServiceError::storage(format!(
            "payment attempt close updated {count} rows; expected at most one"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
    use sqlx::sqlite::SqlitePoolOptions;

    use super::{
        close_expired_owner_order_provider_attempts_for_order_sqlite,
        close_expired_owner_order_provider_attempts_sqlite,
        close_owner_order_provider_attempts_sqlite, mark_attempt_canceled_sqlite,
        ActiveAttemptIdentity,
    };

    #[tokio::test]
    async fn closes_only_the_previous_active_attempt_for_an_order() {
        let pool = payment_close_test_pool().await;
        seed_attempt(
            &pool,
            "intent-old",
            "attempt-old",
            "order-current",
            "sandbox",
        )
        .await;
        seed_attempt(
            &pool,
            "intent-current",
            "attempt-current",
            "order-current",
            "sandbox",
        )
        .await;
        let credentials = empty_credentials();
        let registry = PaymentProviderRegistry::from_credentials(credentials.clone());

        close_owner_order_provider_attempts_sqlite(
            &pool,
            &registry,
            &credentials,
            "tenant-1",
            Some("org-1"),
            "user-1",
            "order-current",
            Some("attempt-current"),
        )
        .await
        .expect("close old attempt");

        assert_eq!(attempt_status(&pool, "attempt-old").await, "canceled");
        assert_eq!(intent_status(&pool, "intent-old").await, "canceled");
        assert_eq!(attempt_status(&pool, "attempt-current").await, "pending");
        assert_eq!(intent_status(&pool, "intent-current").await, "pending");
    }

    #[tokio::test]
    async fn provider_failure_keeps_the_attempt_retryable() {
        let pool = payment_close_test_pool().await;
        seed_attempt(
            &pool,
            "intent-provider",
            "attempt-provider",
            "order-current",
            "wechat_pay",
        )
        .await;
        let credentials = empty_credentials();
        let registry = PaymentProviderRegistry::from_credentials(credentials.clone());

        let error = close_owner_order_provider_attempts_sqlite(
            &pool,
            &registry,
            &credentials,
            "tenant-1",
            Some("org-1"),
            "user-1",
            "order-current",
            None,
        )
        .await
        .expect_err("unconfigured provider must fail closed");

        assert_eq!(error.code(), "provider-unavailable");
        assert_eq!(attempt_status(&pool, "attempt-provider").await, "pending");
        assert_eq!(intent_status(&pool, "intent-provider").await, "pending");
    }

    #[tokio::test]
    async fn successful_payment_wins_a_concurrent_provider_close() {
        let pool = payment_close_test_pool().await;
        seed_attempt(
            &pool,
            "intent-settled",
            "attempt-settled",
            "order-current",
            "sandbox",
        )
        .await;
        sqlx::query(
            "UPDATE commerce_payment_attempt SET status = 'succeeded' WHERE id = 'attempt-settled'",
        )
        .execute(&pool)
        .await
        .expect("settled attempt");
        sqlx::query(
            "UPDATE commerce_payment_intent SET status = 'succeeded' WHERE id = 'intent-settled'",
        )
        .execute(&pool)
        .await
        .expect("settled intent");

        let error = mark_attempt_canceled_sqlite(
            &pool,
            "tenant-1",
            &ActiveAttemptIdentity {
                attempt_id: "attempt-settled".to_owned(),
                payment_intent_id: "intent-settled".to_owned(),
            },
        )
        .await
        .expect_err("payment success must stop close/create replacement");

        assert_eq!(error.code(), "conflict");
        assert_eq!(
            error.message(),
            "payment attempt completed while it was being closed"
        );
        assert_eq!(attempt_status(&pool, "attempt-settled").await, "succeeded");
        assert_eq!(intent_status(&pool, "intent-settled").await, "succeeded");
    }

    #[tokio::test]
    async fn closes_attempts_whose_order_expired() {
        let pool = payment_close_test_pool().await;
        sqlx::query(
            "INSERT INTO commerce_order (id, tenant_id, organization_id, owner_user_id, status, expired_at) VALUES ('order-expired', 'tenant-1', 'org-1', 'user-1', 'expired', '2026-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("expired order");
        seed_attempt(
            &pool,
            "intent-expired",
            "attempt-expired",
            "order-expired",
            "sandbox",
        )
        .await;
        let credentials = empty_credentials();
        let registry = PaymentProviderRegistry::from_credentials(credentials.clone());

        close_expired_owner_order_provider_attempts_sqlite(
            &pool,
            &registry,
            &credentials,
            "tenant-1",
            Some("org-1"),
            "user-1",
        )
        .await
        .expect("close expired order attempt");

        assert_eq!(attempt_status(&pool, "attempt-expired").await, "canceled");
        assert_eq!(intent_status(&pool, "intent-expired").await, "canceled");
    }

    #[tokio::test]
    async fn checkout_expiry_cleanup_is_scoped_to_its_locked_order() {
        let pool = payment_close_test_pool().await;
        for order_id in ["order-expired-a", "order-expired-b"] {
            sqlx::query(
                "INSERT INTO commerce_order (id, tenant_id, organization_id, owner_user_id, status, expired_at) VALUES (?, 'tenant-1', 'org-1', 'user-1', 'expired', '2026-01-01T00:00:00Z')",
            )
            .bind(order_id)
            .execute(&pool)
            .await
            .expect("expired order");
        }
        seed_attempt(
            &pool,
            "intent-expired-a",
            "attempt-expired-a",
            "order-expired-a",
            "sandbox",
        )
        .await;
        seed_attempt(
            &pool,
            "intent-expired-b",
            "attempt-expired-b",
            "order-expired-b",
            "sandbox",
        )
        .await;
        let credentials = empty_credentials();
        let registry = PaymentProviderRegistry::from_credentials(credentials.clone());

        close_expired_owner_order_provider_attempts_for_order_sqlite(
            &pool,
            &registry,
            &credentials,
            "tenant-1",
            Some("org-1"),
            "user-1",
            "order-expired-a",
        )
        .await
        .expect("close current order expired attempt");

        assert_eq!(attempt_status(&pool, "attempt-expired-a").await, "canceled");
        assert_eq!(intent_status(&pool, "intent-expired-a").await, "canceled");
        assert_eq!(attempt_status(&pool, "attempt-expired-b").await, "pending");
        assert_eq!(intent_status(&pool, "intent-expired-b").await, "pending");
    }

    fn empty_credentials() -> ProviderCredentialBundle {
        ProviderCredentialBundle {
            stripe: None,
            alipay: None,
            wechat_pay: None,
            webhook_base_url: None,
        }
    }

    async fn payment_close_test_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("payment close sqlite pool");
        for statement in [
            "CREATE TABLE commerce_order (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, owner_user_id TEXT NOT NULL, status TEXT NOT NULL, expired_at TEXT)",
            "CREATE TABLE commerce_payment_channel (id TEXT PRIMARY KEY, provider_account_id TEXT)",
            "CREATE TABLE commerce_payment_provider_account (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, provider_code TEXT NOT NULL, merchant_id TEXT, environment TEXT NOT NULL DEFAULT 'sandbox', secret_ref TEXT NOT NULL DEFAULT '', webhook_secret_ref TEXT, certificate_ref TEXT, metadata TEXT NOT NULL DEFAULT '{}', status TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT)",
            "CREATE TABLE commerce_payment_intent (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, status TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT)",
            "CREATE TABLE commerce_payment_attempt (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, owner_user_id TEXT NOT NULL, payment_intent_id TEXT NOT NULL, order_id TEXT NOT NULL, provider_code TEXT NOT NULL, channel_id TEXT, out_trade_no TEXT NOT NULL, amount TEXT NOT NULL, callback_payload TEXT NOT NULL, idempotency_key TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT)",
        ] {
            sqlx::query(statement)
                .execute(&pool)
                .await
                .expect("payment close test schema");
        }
        sqlx::query(
            "INSERT INTO commerce_order (id, tenant_id, organization_id, owner_user_id, status, expired_at) VALUES ('order-current', 'tenant-1', 'org-1', 'user-1', 'pending_payment', '2099-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("current order");
        pool
    }

    async fn seed_attempt(
        pool: &sqlx::SqlitePool,
        intent_id: &str,
        attempt_id: &str,
        order_id: &str,
        provider_code: &str,
    ) {
        sqlx::query(
            "INSERT INTO commerce_payment_intent (id, tenant_id, status, updated_at) VALUES (?, 'tenant-1', 'pending', '2026-01-01T00:00:00Z')",
        )
        .bind(intent_id)
        .execute(pool)
        .await
        .expect("payment intent");
        sqlx::query(
            "INSERT INTO commerce_payment_attempt (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id, provider_code, channel_id, out_trade_no, amount, callback_payload, idempotency_key, status, created_at, updated_at) VALUES (?, 'tenant-1', 'org-1', 'user-1', ?, ?, ?, NULL, ?, '1.00', '{}', ?, 'pending', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .bind(attempt_id)
        .bind(intent_id)
        .bind(order_id)
        .bind(provider_code)
        .bind(format!("trade-{attempt_id}"))
        .bind(format!("idem-{attempt_id}"))
        .execute(pool)
        .await
        .expect("payment attempt");
    }

    async fn attempt_status(pool: &sqlx::SqlitePool, attempt_id: &str) -> String {
        sqlx::query_scalar("SELECT status FROM commerce_payment_attempt WHERE id = ?")
            .bind(attempt_id)
            .fetch_one(pool)
            .await
            .expect("attempt status")
    }

    async fn intent_status(pool: &sqlx::SqlitePool, intent_id: &str) -> String {
        sqlx::query_scalar("SELECT status FROM commerce_payment_intent WHERE id = ?")
            .bind(intent_id)
            .fetch_one(pool)
            .await
            .expect("intent status")
    }
}
