use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use std::collections::BTreeMap;

use sdkwork_payment_service::{
    CheckoutStatusQuery, CheckoutStatusSnapshot, CreatePointsRechargeOrderCommand,
    CreatePointsRechargeOrderOutcome, RechargeGrantPreview, RechargePackageItem,
    RechargePackageListQuery, RechargeSettingsQuery, RechargeSettingsSnapshot,
};
use sdkwork_payment_repository_sqlx::{
    PostgresCommerceRechargeStore, SqliteCommerceRechargeStore,
};
use sdkwork_iam_context_service::IamAppContext;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, SqlitePool};

use crate::command_headers::validate_app_write_payload;
use crate::problem_details::problem_error_response;
use crate::subject::{
    app_runtime_subject_from_extension, optional_app_runtime_subject_from_headers,
    AppRuntimeSubject,
};

const MAX_CHECKOUT_ORDER_NO_LEN: usize = 128;
const MAX_RECHARGE_CENTS: i64 = 1_000_000;
const PAYMENT_EXPIRE_SECONDS: i64 = 1_800;
const DEFAULT_RECHARGE_PAYMENT_METHOD: &str = "wechat_pay";

pub type CommerceRechargeCheckoutFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceRechargeCheckoutStore: Send + Sync {
    fn list_recharge_packages<'a>(
        &'a self,
        query: RechargePackageListQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Vec<RechargePackageItem>>;

    fn load_recharge_settings<'a>(
        &'a self,
        query: RechargeSettingsQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, RechargeSettingsSnapshot>;

    fn create_points_recharge_order<'a>(
        &'a self,
        command: CreatePointsRechargeOrderCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, CreatePointsRechargeOrderOutcome>;

    fn retrieve_checkout_status<'a>(
        &'a self,
        query: CheckoutStatusQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<CheckoutStatusSnapshot>>;
}

#[derive(Clone)]
struct AppRechargeCheckoutState {
    store: Arc<dyn CommerceRechargeCheckoutStore>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitRechargeRequest {
    amount: Option<serde_json::Value>,
    client_request_no: Option<String>,
    currency_code: Option<String>,
    package_id: Option<String>,
    source: Option<String>,
}

struct CreateRechargeCommandInput<'a> {
    subject: &'a AppRuntimeSubject,
    amount: CommerceMoney,
    currency_code: &'a str,
    method: &'a str,
    request_no: &'a str,
    idempotency_key: &'a str,
    package_id: Option<&'a str>,
    client_request_no: Option<&'a str>,
    source: Option<&'a str>,
}

impl SubmitRechargeRequest {
    fn amount_value(&self) -> Option<&serde_json::Value> {
        self.amount.as_ref()
    }

    fn currency_code(&self) -> Option<&str> {
        self.currency_code.as_deref()
    }

    fn package_id(&self) -> Option<&str> {
        self.package_id.as_deref()
    }

    fn client_request_no(&self) -> Option<&str> {
        self.client_request_no.as_deref()
    }

    fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppRechargeApiResult<T: Serialize> {
    code: String,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RechargePackageResponse {
    id: String,
    price_amount: String,
    currency_code: String,
    bonus_points: i64,
    grant_amount: i64,
    points: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RechargePackageListResponse {
    items: Vec<RechargePackageResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RechargeGrantPreviewResponse {
    grant_amount: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RechargeSettingsResponse {
    base_currency_code: String,
    base_points_per_cny: String,
    currency_to_cny_rates: BTreeMap<String, String>,
    preview_examples: BTreeMap<String, BTreeMap<String, RechargeGrantPreviewResponse>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitRechargeResponse {
    success: bool,
    order_no: String,
    out_trade_no: String,
    amount: String,
    currency_code: String,
    points: i64,
    provider_code: String,
    payment_method: String,
    payment_product: String,
    status: String,
    next_action: String,
    cashier_url: String,
    qr_code_payload: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_payment_payload: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutStatusResponse {
    order_no: String,
    out_trade_no: String,
    amount: String,
    currency_code: String,
    points: i64,
    provider_code: String,
    payment_method: String,
    payment_product: String,
    order_status: String,
    payment_status: String,
    recharge_status: String,
    status: String,
    created_at: String,
    expires_at: String,
    paid_at: String,
    next_action: String,
    cashier_url: String,
    qr_code_payload: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_payment_payload: Option<String>,
}

impl CommerceRechargeCheckoutStore for SqliteCommerceRechargeStore {
    fn list_recharge_packages<'a>(
        &'a self,
        query: RechargePackageListQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Vec<RechargePackageItem>> {
        Box::pin(async move { self.list_recharge_packages(query).await })
    }

    fn create_points_recharge_order<'a>(
        &'a self,
        command: CreatePointsRechargeOrderCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, CreatePointsRechargeOrderOutcome> {
        Box::pin(async move { self.create_points_recharge_order(command).await })
    }

    fn load_recharge_settings<'a>(
        &'a self,
        query: RechargeSettingsQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, RechargeSettingsSnapshot> {
        Box::pin(async move { self.load_recharge_settings(query).await })
    }

    fn retrieve_checkout_status<'a>(
        &'a self,
        query: CheckoutStatusQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<CheckoutStatusSnapshot>> {
        Box::pin(async move { self.load_checkout_status(query).await })
    }
}

impl CommerceRechargeCheckoutStore for PostgresCommerceRechargeStore {
    fn list_recharge_packages<'a>(
        &'a self,
        query: RechargePackageListQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Vec<RechargePackageItem>> {
        Box::pin(async move { self.list_recharge_packages(query).await })
    }

    fn create_points_recharge_order<'a>(
        &'a self,
        command: CreatePointsRechargeOrderCommand,
    ) -> CommerceRechargeCheckoutFuture<'a, CreatePointsRechargeOrderOutcome> {
        Box::pin(async move { self.create_points_recharge_order(command).await })
    }

    fn load_recharge_settings<'a>(
        &'a self,
        query: RechargeSettingsQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, RechargeSettingsSnapshot> {
        Box::pin(async move { self.load_recharge_settings(query).await })
    }

    fn retrieve_checkout_status<'a>(
        &'a self,
        query: CheckoutStatusQuery,
    ) -> CommerceRechargeCheckoutFuture<'a, Option<CheckoutStatusSnapshot>> {
        Box::pin(async move { self.load_checkout_status(query).await })
    }
}

impl<T: Serialize> AppRechargeApiResult<T> {
    fn success(data: T) -> Self {
        Self {
            code: "2000".to_string(),
            msg: "SUCCESS".to_string(),
            data: Some(data),
        }
    }
}

pub fn app_recharge_checkout_router_with_sqlite_pool(pool: SqlitePool) -> Router {
    build_app_recharge_checkout_router(Arc::new(SqliteCommerceRechargeStore::new(pool)))
}

pub fn app_recharge_checkout_router_with_postgres_pool(pool: PgPool) -> Router {
    build_app_recharge_checkout_router(Arc::new(PostgresCommerceRechargeStore::new(pool)))
}

pub fn build_app_recharge_checkout_router(
    store: Arc<dyn CommerceRechargeCheckoutStore>,
) -> Router {
    Router::new()
            .route(
                "/app/v3/api/recharges/packages",
                get(fetch_recharge_packages),
            )
            .route(
                "/app/v3/api/recharges/settings",
                get(fetch_recharge_settings),
            )
            .route("/app/v3/api/recharges/orders", post(submit_recharge))
            .route(
                "/app/v3/api/recharges/orders/{orderId}",
                get(fetch_checkout_status),
            )
            .with_state(AppRechargeCheckoutState { store })
}

async fn fetch_recharge_packages(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
) -> Response {
    let query = match optional_app_runtime_subject_from_headers(runtime_context, &headers).await {
        Some(subject) => match RechargePackageListQuery::new(
            &subject.tenant_id,
            subject.organization_id.as_deref(),
        ) {
            Ok(query) => query,
            Err(error) => return commerce_error_response(error),
        },
        None => RechargePackageListQuery::public(),
    };

    match state.store.list_recharge_packages(query).await {
        Ok(items) => Json(AppRechargeApiResult::success(RechargePackageListResponse {
            items: items
                .into_iter()
                .map(map_recharge_package)
                .collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(error) => commerce_error_response(error),
    }
}

async fn fetch_recharge_settings(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
) -> Response {
    let query = match optional_app_runtime_subject_from_headers(runtime_context, &headers).await {
        Some(subject) => {
            match RechargeSettingsQuery::new(&subject.tenant_id, subject.organization_id.as_deref())
            {
                Ok(query) => query,
                Err(error) => return commerce_error_response(error),
            }
        }
        None => RechargeSettingsQuery::public(),
    };

    match state.store.load_recharge_settings(query).await {
        Ok(settings) => Json(AppRechargeApiResult::success(map_recharge_settings(
            settings,
        )))
        .into_response(),
        Err(error) => commerce_error_response(error),
    }
}

async fn submit_recharge(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Json(request): Json<SubmitRechargeRequest>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let amount = match validate_recharge_amount(request.amount_value()) {
        Ok(amount) => amount,
        Err(message) => return validation_response(message),
    };
    let currency_code = match validate_currency_code(request.currency_code()) {
        Ok(value) => value,
        Err(message) => return validation_response(message),
    };
    let method = DEFAULT_RECHARGE_PAYMENT_METHOD.to_string();
    let write_headers =
        match validate_app_write_payload(&headers, "recharge.submit", &request, |idempotency_key| {
            fallback_request_no(&subject, amount.as_str(), &method, idempotency_key)
        }) {
            Ok(value) => value,
            Err(response) => return response,
        };
    let command = match build_create_recharge_command(CreateRechargeCommandInput {
        subject: &subject,
        amount,
        currency_code: &currency_code,
        method: &method,
        request_no: &write_headers.request_no,
        idempotency_key: &write_headers.idempotency_key,
        package_id: request.package_id(),
        client_request_no: request.client_request_no(),
        source: request.source(),
    }) {
        Ok(command) => command,
        Err(error) => return commerce_error_response(error),
    };

    match state.store.create_points_recharge_order(command).await {
        Ok(outcome) => {
            Json(AppRechargeApiResult::success(map_recharge_outcome(outcome))).into_response()
        }
        Err(error) => commerce_error_response(error),
    }
}

async fn fetch_checkout_status(
    State(state): State<AppRechargeCheckoutState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(order_no): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let order_no = match validate_checkout_order_no(order_no) {
        Ok(order_no) => order_no,
        Err(message) => return validation_response(message),
    };
    let query = match CheckoutStatusQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_no,
    ) {
        Ok(query) => query,
        Err(error) => return commerce_error_response(error),
    };

    match state.store.retrieve_checkout_status(query).await {
        Ok(Some(snapshot)) => {
            Json(AppRechargeApiResult::success(map_checkout_status(snapshot))).into_response()
        }
        Ok(None) => problem_error_response(
            StatusCode::CONFLICT,
            "4090",
            "checkout order was not found",
        ),
        Err(error) => commerce_error_response(error),
    }
}

fn validate_recharge_amount(value: Option<&serde_json::Value>) -> Result<CommerceMoney, String> {
    let Some(value) = value else {
        return Err("recharge amount must be greater than zero".to_string());
    };
    let raw = match value {
        serde_json::Value::String(value) => value.trim().to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        _ => return Err("recharge amount must be a decimal amount".to_string()),
    };
    let cents = money_cents(&raw).map_err(|_| "recharge amount must be a decimal amount")?;
    if cents <= 0 {
        return Err("recharge amount must be greater than zero".to_string());
    }
    if cents > MAX_RECHARGE_CENTS {
        return Err("recharge amount must not exceed 10000.00".to_string());
    }
    CommerceMoney::new(&format_money_minor(cents)).map_err(str::to_string)
}

fn validate_currency_code(value: Option<&str>) -> Result<String, String> {
    let currency_code = value.unwrap_or_default().trim().to_ascii_uppercase();
    if currency_code.len() != 3
        || !currency_code
            .chars()
            .all(|character| character.is_ascii_uppercase())
    {
        return Err("currency code must be a 3-letter uppercase code".to_string());
    }
    Ok(currency_code)
}

fn validate_checkout_order_no(order_no: String) -> Result<String, String> {
    let order_no = order_no.trim().to_string();
    if order_no.is_empty() {
        return Err("checkout order number must not be empty".to_string());
    }
    if order_no.chars().count() > MAX_CHECKOUT_ORDER_NO_LEN {
        return Err(format!(
            "checkout order number length must not exceed {MAX_CHECKOUT_ORDER_NO_LEN} characters"
        ));
    }
    if !order_no.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
        return Err("checkout order number must contain only visible ASCII characters".to_string());
    }
    Ok(order_no)
}

fn build_create_recharge_command(
    input: CreateRechargeCommandInput<'_>,
) -> Result<CreatePointsRechargeOrderCommand, CommerceServiceError> {
    let now = current_unix_timestamp();
    let requested_at = format_unix_timestamp(now);
    let expire_at = format_unix_timestamp(now + PAYMENT_EXPIRE_SECONDS);
    let seed = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        input.subject.tenant_id,
        input.subject.organization_id.as_deref().unwrap_or(""),
        input.subject.user_id,
        input.amount.as_str(),
        input.method,
        input.request_no,
        input.idempotency_key,
    );
    let token = stable_hex_token(&seed);
    let order_no = format!("RC{}", token);
    let out_trade_no = format!("RECHARGE{}", token);

    CreatePointsRechargeOrderCommand::new(
        &input.subject.tenant_id,
        input.subject.organization_id.as_deref(),
        &input.subject.user_id,
        input.amount,
        input.currency_code,
        input.method,
        &format!("order-{token}"),
        &format!("order-item-{token}"),
        &format!("payment-intent-{token}"),
        &format!("payment-attempt-{token}"),
        &order_no,
        &out_trade_no,
        &requested_at,
        &expire_at,
        input.idempotency_key,
        input.package_id,
        input.client_request_no,
        input.source,
    )
}

fn map_recharge_package(value: RechargePackageItem) -> RechargePackageResponse {
    RechargePackageResponse {
        id: value.id,
        price_amount: value.price_amount.as_str().to_string(),
        currency_code: value.currency_code,
        bonus_points: value.bonus_points,
        grant_amount: value.grant_amount,
        points: value.points,
    }
}

fn map_recharge_settings(value: RechargeSettingsSnapshot) -> RechargeSettingsResponse {
    RechargeSettingsResponse {
        base_currency_code: value.base_currency_code,
        base_points_per_cny: value.base_points_per_cny,
        currency_to_cny_rates: value.currency_to_cny_rates,
        preview_examples: value
            .preview_examples
            .into_iter()
            .map(|(currency_code, amount_map)| {
                (
                    currency_code,
                    amount_map
                        .into_iter()
                        .map(|(amount, preview)| (amount, map_recharge_preview(preview)))
                        .collect::<BTreeMap<_, _>>(),
                )
            })
            .collect(),
    }
}

fn map_recharge_preview(value: RechargeGrantPreview) -> RechargeGrantPreviewResponse {
    RechargeGrantPreviewResponse {
        grant_amount: value.grant_amount,
    }
}

fn map_recharge_outcome(value: CreatePointsRechargeOrderOutcome) -> SubmitRechargeResponse {
    SubmitRechargeResponse {
        success: value.success,
        order_no: value.order_no,
        out_trade_no: value.out_trade_no,
        amount: value.amount.as_str().to_string(),
        currency_code: value.currency_code,
        points: value.points,
        provider_code: value.provider_code,
        payment_method: value.payment_method,
        payment_product: value.payment_product,
        status: value.status,
        next_action: value.next_action,
        cashier_url: value.cashier_url,
        qr_code_payload: value.qr_code_payload,
        request_payment_payload: value.request_payment_payload,
    }
}

fn map_checkout_status(value: CheckoutStatusSnapshot) -> CheckoutStatusResponse {
    CheckoutStatusResponse {
        order_no: value.order_no,
        out_trade_no: value.out_trade_no,
        amount: value.amount.as_str().to_string(),
        currency_code: value.currency_code,
        points: value.points,
        provider_code: value.provider_code,
        payment_method: value.payment_method,
        payment_product: value.payment_product,
        order_status: value.order_status,
        payment_status: value.payment_status,
        recharge_status: value.recharge_status,
        status: value.status,
        created_at: value.created_at,
        expires_at: value.expires_at,
        paid_at: value.paid_at,
        next_action: value.next_action,
        cashier_url: value.cashier_url,
        qr_code_payload: value.qr_code_payload,
        request_payment_payload: value.request_payment_payload,
    }
}

fn commerce_error_response(error: CommerceServiceError) -> Response {
    match error.code() {
        "validation" => validation_response(error.message()),
        "unauthenticated" | "unauthorized" => unauthorized_response(error.message().to_string()),
        "not-found" => problem_error_response(StatusCode::NOT_FOUND, "4040", error.message()),
        "conflict" | "invalid-state" | "unsupported-capability" => {
            problem_error_response(StatusCode::CONFLICT, "4090", error.message())
        }
        _ => problem_error_response(StatusCode::INTERNAL_SERVER_ERROR, "5000", error.message()),
    }
}

fn unauthorized_response(message: String) -> Response {
    problem_error_response(StatusCode::UNAUTHORIZED, "4010", message)
}

fn validation_response(message: impl Into<String>) -> Response {
    problem_error_response(StatusCode::BAD_REQUEST, "4001", message)
}

fn fallback_request_no(
    subject: &AppRuntimeSubject,
    amount: &str,
    method: &str,
    idempotency_key: &str,
) -> String {
    stable_header_token(&format!(
        "points-recharge-{}-{}-{}-{}",
        subject.user_id, amount, method, idempotency_key
    ))
}

fn stable_header_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn stable_hex_token(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn money_cents(amount: &str) -> Result<i64, ()> {
    let value = amount.trim();
    let mut parts = value.split('.');
    let whole = parts
        .next()
        .unwrap_or_default()
        .parse::<i64>()
        .map_err(|_| ())?;
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || fraction.len() > 2 {
        return Err(());
    }
    let mut padded = fraction.to_string();
    while padded.len() < 2 {
        padded.push('0');
    }
    let cents = if padded.is_empty() {
        0
    } else {
        padded.parse::<i64>().map_err(|_| ())?
    };
    whole
        .checked_mul(100)
        .and_then(|amount| amount.checked_add(cents))
        .ok_or(())
}

fn format_money_minor(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn format_unix_timestamp(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}
