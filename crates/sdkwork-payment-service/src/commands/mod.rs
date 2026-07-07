use sdkwork_contract_service::CommerceMoney;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatePaymentIntentCommand {
    pub amount: CommerceMoney,
    pub idempotency_key: String,
    pub order_id: String,
    pub payment_method: String,
    pub provider_code: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateRefundCommand {
    pub amount: CommerceMoney,
    pub idempotency_key: String,
    pub payment_id: String,
    pub request_no: String,
    pub tenant_id: String,
}
