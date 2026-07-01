use std::sync::OnceLock;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_utils_rust::SdkWorkProblemDetail;
use serde::{Deserialize, Serialize};

static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP.get_or_init(reqwest::Client::new)
}

#[derive(Clone, Debug)]
pub struct OrderPointsRechargeFulfillmentClient {
    origin: String,
    auth_token: Option<String>,
}

impl OrderPointsRechargeFulfillmentClient {
    pub fn from_env() -> Self {
        let origin = std::env::var("SDKWORK_ORDER_BACKEND_API_ORIGIN")
            .unwrap_or_else(|_| "http://127.0.0.1:18093".to_owned())
            .trim()
            .trim_end_matches('/')
            .to_owned();
        let auth_token = std::env::var("SDKWORK_PAYMENT_ORDER_SERVICE_AUTH_TOKEN")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        Self { origin, auth_token }
    }

    pub async fn create_points_recharge_fulfillment(
        &self,
        order_id: &str,
        request: &OrderPointsRechargeFulfillmentRequest,
    ) -> Result<OrderPointsRechargeFulfillmentOutcome, CommerceServiceError> {
        let url = format!(
            "{}/backend/v3/api/orders/{order_id}/points-recharge/fulfillments",
            self.origin
        );
        let mut builder = http_client()
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .json(request);

        if let Some(token) = &self.auth_token {
            builder = builder.header(AUTHORIZATION, format!("Bearer {token}"));
        }

        let response = builder.send().await.map_err(|error| {
            CommerceServiceError::storage(format!(
                "order points recharge fulfillment request failed: {error}"
            ))
        })?;

        let status = response.status();
        if status.is_success() {
            let envelope = response
                .json::<FulfillmentEnvelope>()
                .await
                .map_err(|error| {
                    CommerceServiceError::storage(format!(
                        "order points recharge fulfillment response is invalid: {error}"
                    ))
                })?;
            return Ok(envelope.data);
        }

        if let Ok(problem) = response.json::<SdkWorkProblemDetail>().await {
            return Err(map_problem_detail(problem));
        }

        Err(CommerceServiceError::storage(format!(
            "order points recharge fulfillment failed with HTTP {status}"
        )))
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderPointsRechargeFulfillmentRequest {
    pub request_no: String,
    pub idempotency_key: String,
    pub paid_at: String,
    pub owner_user_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderPointsRechargeFulfillmentOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub order_id: String,
    pub order_no: String,
    pub points_credited: i64,
    pub fulfillment_status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FulfillmentEnvelope {
    #[allow(dead_code)]
    code: i32,
    data: OrderPointsRechargeFulfillmentOutcome,
    #[allow(dead_code)]
    trace_id: String,
}

fn map_problem_detail(problem: SdkWorkProblemDetail) -> CommerceServiceError {
    let message = problem.detail.unwrap_or_else(|| problem.title.clone());
    match problem.code {
        40401 => CommerceServiceError::not_found(message),
        40901 => CommerceServiceError::conflict(message),
        40001 | 40002 | 40003 | 40004 => CommerceServiceError::validation(message),
        40101 | 40102 | 40103 | 40104 => CommerceServiceError::unauthorized(message),
        _ => CommerceServiceError::storage(message),
    }
}
