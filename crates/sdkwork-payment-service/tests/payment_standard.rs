use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_payment_service::{
    validate_payment_wire_transition, validate_refund_wire_transition, wire_product_types_from_scene_codes,
    PaymentIntentDraft, PaymentProviderCommand, PaymentProviderPort, PaymentProviderPortRequirement,
    PaymentStatus, PaymentTransition, RefundStatus, RefundTransition, SandboxPaymentProvider,
};
use sdkwork_payment_service::CreatePaymentIntentCommand;

#[test]
fn creates_payment_intent_with_method_provider_and_order_reference() {
    let intent = PaymentIntentDraft::new(
        "100001",
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

#[test]
fn validates_payment_wire_transitions() {
    assert!(validate_payment_wire_transition("pending", "canceled").is_ok());
    assert!(validate_payment_wire_transition("succeeded", "pending").is_err());
}

#[test]
fn validates_refund_wire_transitions() {
    assert!(validate_refund_wire_transition(None, "submitted").is_ok());
    assert!(validate_refund_wire_transition(Some("succeeded"), "submitted").is_err());
}

#[test]
fn maps_scene_codes_to_product_types() {
    let types = wire_product_types_from_scene_codes(&[
        "web".to_string(),
        "app".to_string(),
    ]);
    let codes: Vec<_> = types.iter().map(|(code, _)| code.as_str()).collect();
    assert!(codes.contains(&"pc"));
    assert!(codes.contains(&"app"));
}

#[test]
fn sandbox_provider_creates_payment_intent_draft() {
    let provider = SandboxPaymentProvider;
    let command = CreatePaymentIntentCommand {
        tenant_id: "100001".to_string(),
        order_id: "order-1".to_string(),
        payment_method: "wechat_pay".to_string(),
        provider_code: "wechat_pay".to_string(),
        amount: CommerceMoney::new("9.90").unwrap(),
        idempotency_key: "idem-1".to_string(),
    };
    let draft = PaymentProviderPort::create_payment_intent(&provider, &command).unwrap();
    assert_eq!(draft.order_id, "order-1");
    assert_eq!(draft.provider_code, "wechat_pay");
}
