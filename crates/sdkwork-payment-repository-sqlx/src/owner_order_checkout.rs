//! Owner-order pay PSP enrichment after repository persistence.
//!
//! Shared by payment and order app-api routers so `orders.payments.create` and `payments.create`
//! return the same cashier parameters.

use sdkwork_contract_service::CommerceServiceError;
use sdkwork_payment_providers::{
    enrich_pay_owner_order_outcome, normalize_provider_code, provider_registry_for_account,
    CheckoutContext, PaymentProviderRegistry, ProviderAccountBinding, ProviderCredentialBundle,
};
use sdkwork_payment_service::{
    CreateOwnerPaymentAttemptOutcome, PayOwnerOrderOutcome, PaymentRecordItem,
};
use sqlx::{PgPool, Pool, Sqlite};

use crate::provider_account::{
    load_active_provider_account_postgres, load_active_provider_account_sqlite,
    PaymentProviderAccountRecord,
};

pub fn provider_account_binding(record: &PaymentProviderAccountRecord) -> ProviderAccountBinding {
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

use crate::owner_payment_params::owner_order_payment_params;
use crate::payment_attempt_context::{
    load_payment_attempt_provider_context_postgres, load_payment_attempt_provider_context_sqlite,
    persist_attempt_enrichment_postgres, persist_attempt_enrichment_sqlite,
};

pub fn payment_record_is_checkout_eligible(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "created" | "pending" | "processing"
    )
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
    let idempotency_key = ctx.idempotency_key.clone();
    let enriched = enrich_owner_order_payment_sqlite(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        &record.order_id,
        &idempotency_key,
        None,
        outcome,
    )
    .await?;
    persist_attempt_enrichment_sqlite(pool, tenant_id, &record.id, &enriched.payment_params)
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
    let idempotency_key = ctx.idempotency_key.clone();
    let enriched = enrich_owner_order_payment_postgres(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        &record.order_id,
        &idempotency_key,
        None,
        outcome,
    )
    .await?;
    persist_attempt_enrichment_postgres(pool, tenant_id, &record.id, &enriched.payment_params)
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
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    idempotency_key: &str,
    payment_scene: Option<&str>,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let provider_code = outcome
        .payment_params
        .get("providerCode")
        .cloned()
        .unwrap_or_else(|| "sandbox".to_owned());
    let account =
        load_active_provider_account_sqlite(pool, tenant_id, organization_id, &provider_code)
            .await?;
    let enriched = enrich_owner_order_payment_outcome(
        deployment_registry,
        credentials,
        account.as_ref().map(provider_account_binding),
        tenant_id,
        order_id,
        idempotency_key,
        payment_scene,
        &provider_code,
        outcome,
    )
    .await?;
    persist_attempt_enrichment_sqlite(
        pool,
        tenant_id,
        &enriched.payment_id,
        &enriched.payment_params,
    )
    .await?;
    Ok(enriched)
}

pub async fn enrich_owner_order_payment_postgres(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    idempotency_key: &str,
    payment_scene: Option<&str>,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let provider_code = outcome
        .payment_params
        .get("providerCode")
        .cloned()
        .unwrap_or_else(|| "sandbox".to_owned());
    let account =
        load_active_provider_account_postgres(pool, tenant_id, organization_id, &provider_code)
            .await?;
    let enriched = enrich_owner_order_payment_outcome(
        deployment_registry,
        credentials,
        account.as_ref().map(provider_account_binding),
        tenant_id,
        order_id,
        idempotency_key,
        payment_scene,
        &provider_code,
        outcome,
    )
    .await?;
    persist_attempt_enrichment_postgres(
        pool,
        tenant_id,
        &enriched.payment_id,
        &enriched.payment_params,
    )
    .await?;
    Ok(enriched)
}

pub async fn enrich_owner_payment_attempt_sqlite(
    pool: &Pool<Sqlite>,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    idempotency_key: &str,
    payment_scene: Option<&str>,
    outcome: CreateOwnerPaymentAttemptOutcome,
) -> Result<CreateOwnerPaymentAttemptOutcome, CommerceServiceError> {
    let pay_outcome = attempt_outcome_to_pay_outcome(&outcome);
    let enriched = enrich_owner_order_payment_sqlite(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        order_id,
        idempotency_key,
        payment_scene,
        pay_outcome,
    )
    .await?;
    Ok(merge_attempt_payment_params(
        outcome,
        enriched.payment_params,
    ))
}

pub async fn enrich_owner_payment_attempt_postgres(
    pool: &PgPool,
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    idempotency_key: &str,
    payment_scene: Option<&str>,
    outcome: CreateOwnerPaymentAttemptOutcome,
) -> Result<CreateOwnerPaymentAttemptOutcome, CommerceServiceError> {
    let pay_outcome = attempt_outcome_to_pay_outcome(&outcome);
    let enriched = enrich_owner_order_payment_postgres(
        pool,
        deployment_registry,
        credentials,
        tenant_id,
        organization_id,
        order_id,
        idempotency_key,
        payment_scene,
        pay_outcome,
    )
    .await?;
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

async fn enrich_owner_order_payment_outcome(
    deployment_registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    account: Option<ProviderAccountBinding>,
    tenant_id: &str,
    order_id: &str,
    idempotency_key: &str,
    payment_scene: Option<&str>,
    provider_code: &str,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let registry = match account {
        Some(binding) => provider_registry_for_account(credentials, Some(binding)),
        None => deployment_registry.clone(),
    };
    let notify_url = credentials.provider_notify_url(&normalize_provider_code(provider_code));
    let context = CheckoutContext {
        provider_code: provider_code.to_owned(),
        currency_code: "CNY".to_owned(),
        tenant_id: tenant_id.to_owned(),
        order_id: order_id.to_owned(),
        idempotency_key: idempotency_key.to_owned(),
        notify_url,
        payment_scene: payment_scene.map(str::to_owned),
    };
    enrich_pay_owner_order_outcome(&registry, &context, outcome).await
}
