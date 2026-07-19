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
    load_active_provider_account_for_channel_postgres,
    load_active_provider_account_for_channel_sqlite, load_active_provider_account_postgres,
    load_active_provider_account_sqlite, PaymentProviderAccountRecord,
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

#[derive(Clone, Copy)]
pub struct OwnerOrderPaymentEnrichmentContext<'a> {
    pub deployment_registry: &'a PaymentProviderRegistry,
    pub credentials: &'a ProviderCredentialBundle,
    pub tenant_id: &'a str,
    pub organization_id: Option<&'a str>,
    pub order_id: &'a str,
    pub idempotency_key: &'a str,
    pub payment_scene: Option<&'a str>,
    pub payment_metadata: Option<&'a serde_json::Value>,
}

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
        OwnerOrderPaymentEnrichmentContext {
            deployment_registry,
            credentials,
            tenant_id,
            organization_id,
            order_id: &record.order_id,
            idempotency_key: &idempotency_key,
            payment_scene: None,
            payment_metadata: Some(&ctx.payment_metadata),
        },
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
        OwnerOrderPaymentEnrichmentContext {
            deployment_registry,
            credentials,
            tenant_id,
            organization_id,
            order_id: &record.order_id,
            idempotency_key: &idempotency_key,
            payment_scene: None,
            payment_metadata: Some(&ctx.payment_metadata),
        },
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
    let provider_code = outcome
        .payment_params
        .get("providerCode")
        .cloned()
        .unwrap_or_else(|| "sandbox".to_owned());
    let account = match outcome.payment_params.get("channelId") {
        Some(channel_id) => {
            load_active_provider_account_for_channel_sqlite(
                pool,
                context.tenant_id,
                context.organization_id,
                channel_id,
                &provider_code,
            )
            .await?
        }
        None => {
            load_active_provider_account_sqlite(
                pool,
                context.tenant_id,
                context.organization_id,
                &provider_code,
            )
            .await?
        }
    };
    let enriched = enrich_owner_order_payment_outcome(
        &context,
        account.as_ref().map(provider_account_binding),
        &provider_code,
        outcome,
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
    let provider_code = outcome
        .payment_params
        .get("providerCode")
        .cloned()
        .unwrap_or_else(|| "sandbox".to_owned());
    let account = match outcome.payment_params.get("channelId") {
        Some(channel_id) => {
            load_active_provider_account_for_channel_postgres(
                pool,
                context.tenant_id,
                context.organization_id,
                channel_id,
                &provider_code,
            )
            .await?
        }
        None => {
            load_active_provider_account_postgres(
                pool,
                context.tenant_id,
                context.organization_id,
                &provider_code,
            )
            .await?
        }
    };
    let enriched = enrich_owner_order_payment_outcome(
        &context,
        account.as_ref().map(provider_account_binding),
        &provider_code,
        outcome,
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

async fn enrich_owner_order_payment_outcome(
    context: &OwnerOrderPaymentEnrichmentContext<'_>,
    account: Option<ProviderAccountBinding>,
    provider_code: &str,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let registry = match account {
        Some(binding) => provider_registry_for_account(context.credentials, Some(binding)),
        None => context.deployment_registry.clone(),
    };
    let notify_url = context
        .credentials
        .provider_notify_url(&normalize_provider_code(provider_code));
    let context = CheckoutContext {
        provider_code: provider_code.to_owned(),
        currency_code: "CNY".to_owned(),
        tenant_id: context.tenant_id.to_owned(),
        order_id: context.order_id.to_owned(),
        idempotency_key: context.idempotency_key.to_owned(),
        notify_url,
        payment_scene: context.payment_scene.map(str::to_owned),
        payment_metadata: context.payment_metadata.cloned(),
    };
    enrich_pay_owner_order_outcome(&registry, &context, outcome).await
}
