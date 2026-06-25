use sdkwork_commerce_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_commerce_payment_service::{
    PaymentIntentDraft, PaymentProviderCommand, PaymentProviderPortRequirement, PaymentStatus,
    PaymentTransition, RefundStatus, RefundTransition,
};

#[test]
fn creates_payment_intent_with_method_provider_and_order_reference() {
    let intent = PaymentIntentDraft::new(
        "tenant-1",
        "order-1",
        "wechat_pay",
        "wechat_pay",
        CommerceMoney::new("19.90").unwrap(),
        "idem-1",
    )
    .unwrap();

    assert_eq!(intent.order_id, "order-1");
    assert_eq!(intent.payment_method, "wechat_pay");
    assert_eq!(intent.provider_code, "wechat_pay");
    assert_eq!(intent.amount.as_str(), "19.90");
}

#[test]
fn payment_domain_contract_uses_explicit_method_and_provider_code_fields() {
    let domain_source = include_str!("../src/domain/mod.rs");
    let command_source = include_str!("../src/commands/mod.rs");

    assert!(domain_source.contains("pub payment_method: String"));
    assert!(domain_source.contains("pub provider_code: String"));
    assert!(
        !domain_source.contains("pub provider: String"),
        "PaymentIntentDraft must not collapse payment_method and provider_code into provider",
    );
    assert!(command_source.contains("pub payment_method: String"));
    assert!(command_source.contains("pub provider_code: String"));
    assert!(
        !command_source.contains("pub provider: String"),
        "CreatePaymentIntentCommand must not collapse payment_method and provider_code into provider",
    );
}

#[test]
fn validates_payment_status_lifecycle() {
    assert_eq!(
        PaymentTransition::new(PaymentStatus::Created, PaymentStatus::Pending).validate(),
        Ok(())
    );
    assert_eq!(
        PaymentTransition::new(PaymentStatus::Succeeded, PaymentStatus::Pending).validate(),
        Err(CommerceServiceError::invalid_state(
            "invalid payment status transition"
        ))
    );
}

#[test]
fn validates_refund_status_lifecycle() {
    assert_eq!(
        RefundTransition::new(RefundStatus::Requested, RefundStatus::Processing).validate(),
        Ok(())
    );
    assert!(
        RefundTransition::new(RefundStatus::Succeeded, RefundStatus::Processing)
            .validate()
            .is_err()
    );
}

#[test]
fn payment_provider_contract_exposes_required_commands() {
    assert_eq!(
        PaymentProviderPortRequirement::standard_commands(),
        vec![
            PaymentProviderCommand::CreatePaymentIntent,
            PaymentProviderCommand::QueryPaymentStatus,
            PaymentProviderCommand::ClosePayment,
            PaymentProviderCommand::Refund,
            PaymentProviderCommand::VerifyWebhook,
        ],
    );
}
