use sdkwork_payment_service::{
    CancelOwnerPaymentIntentCommand, CreateOwnerRefundCommand, OrderPaymentSettlementAttempt,
    PaymentIntentDetailQuery, RefundListQuery,
};

use crate::{
    test_sqlite_pool::payment_store_e2e_sqlite_memory_pool, SqliteCommerceOwnerOrderPaymentStore,
    SqliteCommercePaymentIntentStore, SqliteCommerceRefundStore,
};

async fn seed_paid_order_with_attempt(pool: &sqlx::SqlitePool) {
    let now = "2026-07-05T10:00:00Z";
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, owner_user_id, order_no, status, subject, currency_code,
             payment_status, created_at, paid_at, updated_at)
        VALUES ('order-1', '100001', 'user-1', 'ORD-1', 'paid', 'test', 'CNY', 'paid', ?, ?, ?)
        "#,
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed order");

    sqlx::query(
        r#"
        INSERT INTO commerce_order_amount_breakdown
            (id, tenant_id, order_id, allocation_type, payable_amount, discount_amount, created_at)
        VALUES ('breakdown-1', '100001', 'order-1', 'order_total', '1000', '0', ?)
        "#,
    )
    .bind(now)
    .execute(pool)
    .await
    .expect("seed breakdown");

    sqlx::query(
        r#"
        INSERT INTO commerce_payment_intent
            (id, tenant_id, owner_user_id, order_id, payment_intent_no, amount, status,
             idempotency_key, created_at, updated_at)
        VALUES ('pi-1', '100001', 'user-1', 'order-1', 'PI-1', '1000', 'succeeded',
                'pi-idem-1', ?, ?)
        "#,
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed intent");

    sqlx::query(
        r#"
        INSERT INTO commerce_payment_attempt
            (id, tenant_id, owner_user_id, payment_intent_id, order_id, amount, status,
             paid_at, idempotency_key, created_at, updated_at)
        VALUES ('pa-1', '100001', 'user-1', 'pi-1', 'order-1', '1000', 'succeeded',
                ?, 'pa-idem-1', ?, ?)
        "#,
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed attempt");
}

async fn seed_owner_order_payment_confirmation(
    pool: &sqlx::SqlitePool,
    attempt_status: &str,
    paid_at: Option<&str>,
) {
    let now = "2026-07-12T00:00:00Z";
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, owner_user_id, order_no, status, subject, currency_code,
             payment_status, created_at, updated_at)
        VALUES ('order-confirm', 'tenant-confirm', 'user-confirm', 'ORD-CONFIRM',
                'pending_payment', 'test', 'CNY', 'pending', ?, ?)
        "#,
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed confirmation order");

    sqlx::query(
        r#"
        INSERT INTO commerce_payment_intent
            (id, tenant_id, owner_user_id, order_id, payment_intent_no, amount, status,
             idempotency_key, created_at, updated_at)
        VALUES ('pi-confirm', 'tenant-confirm', 'user-confirm', 'order-confirm', 'PI-CONFIRM',
                '8800', ?, 'pi-confirm-idem', ?, ?)
        "#,
    )
    .bind(if attempt_status == "succeeded" {
        "succeeded"
    } else {
        "pending"
    })
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed confirmation intent");

    sqlx::query(
        r#"
        INSERT INTO commerce_payment_attempt
            (id, tenant_id, owner_user_id, payment_intent_id, order_id, amount, status,
             paid_at, idempotency_key, created_at, updated_at)
        VALUES ('pa-confirm', 'tenant-confirm', 'user-confirm', 'pi-confirm', 'order-confirm',
                '8800', ?, ?, 'pa-confirm-idem', ?, ?)
        "#,
    )
    .bind(attempt_status)
    .bind(paid_at)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed confirmation attempt");
}

async fn seed_second_confirmation_attempt(pool: &sqlx::SqlitePool) {
    let now = "2026-07-12T00:00:01Z";
    sqlx::query(
        r#"
        INSERT INTO commerce_payment_intent
            (id, tenant_id, owner_user_id, order_id, payment_intent_no, amount, status,
             idempotency_key, created_at, updated_at)
        VALUES ('pi-confirm-2', 'tenant-confirm', 'user-confirm', 'order-confirm', 'PI-CONFIRM-2',
                '8800', 'pending', 'pi-confirm-idem-2', ?, ?)
        "#,
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed second confirmation intent");

    sqlx::query(
        r#"
        INSERT INTO commerce_payment_attempt
            (id, tenant_id, owner_user_id, payment_intent_id, order_id, amount, status,
             out_trade_no, idempotency_key, created_at, updated_at)
        VALUES ('pa-confirm-2', 'tenant-confirm', 'user-confirm', 'pi-confirm-2', 'order-confirm',
                '8800', 'pending', 'out-confirm-2', 'pa-confirm-idem-2', ?, ?)
        "#,
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed second confirmation attempt");
}

fn confirmation_attempt() -> OrderPaymentSettlementAttempt {
    OrderPaymentSettlementAttempt {
        tenant_id: "tenant-confirm".to_owned(),
        organization_id: None,
        owner_user_id: "user-confirm".to_owned(),
        order_id: "order-confirm".to_owned(),
        payment_attempt_id: Some("pa-confirm".to_owned()),
        out_trade_no: None,
    }
}

#[tokio::test]
async fn owner_order_confirmation_replay_returns_original_paid_at() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    let original_paid_at = "2026-07-11T23:59:58Z";
    seed_owner_order_payment_confirmation(&pool, "succeeded", Some(original_paid_at)).await;

    let outcome = SqliteCommerceOwnerOrderPaymentStore::new(pool)
        .confirm_owner_order_payment(&confirmation_attempt())
        .await
        .expect("replayed confirmation");

    assert!(outcome.replayed);
    assert_eq!(outcome.paid_at, original_paid_at);
}

#[tokio::test]
async fn concurrent_owner_order_confirmations_report_exactly_one_first_write() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    seed_owner_order_payment_confirmation(&pool, "pending", None).await;
    let store = SqliteCommerceOwnerOrderPaymentStore::new(pool.clone());
    let attempt = confirmation_attempt();

    let (first, second) = tokio::join!(
        store.confirm_owner_order_payment(&attempt),
        store.confirm_owner_order_payment(&attempt)
    );
    let first = first.expect("first concurrent confirmation");
    let second = second.expect("second concurrent confirmation");

    assert_eq!(
        usize::from(!first.replayed) + usize::from(!second.replayed),
        1
    );
    assert_eq!(first.paid_at, second.paid_at);

    let persisted: (String, Option<String>) = sqlx::query_as(
        "SELECT status, paid_at FROM commerce_payment_attempt WHERE id = 'pa-confirm'",
    )
    .fetch_one(&pool)
    .await
    .expect("persisted confirmation attempt");
    assert_eq!(persisted.0, "succeeded");
    assert_eq!(persisted.1.as_deref(), Some(first.paid_at.as_str()));
}

#[tokio::test]
async fn exact_owner_order_confirmation_does_not_confirm_another_attempt() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    seed_owner_order_payment_confirmation(&pool, "pending", None).await;
    seed_second_confirmation_attempt(&pool).await;

    let outcome = SqliteCommerceOwnerOrderPaymentStore::new(pool.clone())
        .confirm_owner_order_payment(&confirmation_attempt())
        .await
        .expect("exact confirmation");

    assert!(!outcome.replayed);
    let statuses: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, status FROM commerce_payment_attempt WHERE order_id = 'order-confirm' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .expect("confirmation attempt statuses");
    assert_eq!(
        statuses,
        vec![
            ("pa-confirm".to_owned(), "succeeded".to_owned()),
            ("pa-confirm-2".to_owned(), "pending".to_owned()),
        ]
    );
}

#[tokio::test]
async fn manual_owner_order_confirmation_rejects_ambiguous_attempts() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    seed_owner_order_payment_confirmation(&pool, "pending", None).await;
    seed_second_confirmation_attempt(&pool).await;
    let mut attempt = confirmation_attempt();
    attempt.payment_attempt_id = None;

    let error = SqliteCommerceOwnerOrderPaymentStore::new(pool)
        .confirm_owner_order_payment(&attempt)
        .await
        .expect_err("ambiguous confirmation must fail");

    assert_eq!(error.code(), "conflict");
}

#[tokio::test]
async fn owner_order_confirmation_rejects_terminal_intent_mismatch() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    seed_owner_order_payment_confirmation(&pool, "pending", None).await;
    sqlx::query("UPDATE commerce_payment_intent SET status = 'canceled' WHERE id = 'pi-confirm'")
        .execute(&pool)
        .await
        .expect("cancel confirmation intent");

    let error = SqliteCommerceOwnerOrderPaymentStore::new(pool.clone())
        .confirm_owner_order_payment(&confirmation_attempt())
        .await
        .expect_err("terminal intent mismatch must fail");

    assert_eq!(error.code(), "invalid-state");
    let status: String =
        sqlx::query_scalar("SELECT status FROM commerce_payment_attempt WHERE id = 'pa-confirm'")
            .fetch_one(&pool)
            .await
            .expect("attempt status after rejected confirmation");
    assert_eq!(status, "pending");
}

#[tokio::test]
async fn owner_order_confirmation_rejects_soft_deleted_payment_state() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    seed_owner_order_payment_confirmation(&pool, "pending", None).await;
    sqlx::query(
        "UPDATE commerce_payment_intent SET deleted_at = '2026-07-12T00:00:02Z' WHERE id = 'pi-confirm'",
    )
    .execute(&pool)
    .await
    .expect("soft delete confirmation intent");

    let error = SqliteCommerceOwnerOrderPaymentStore::new(pool)
        .confirm_owner_order_payment(&confirmation_attempt())
        .await
        .expect_err("deleted intent must fail closed");

    assert_eq!(error.code(), "storage");
}

#[tokio::test]
async fn refund_create_is_idempotent_by_idempotency_key() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    seed_paid_order_with_attempt(&pool).await;

    let store = SqliteCommerceRefundStore::new(pool);
    let command = CreateOwnerRefundCommand::new(
        "100001",
        None,
        "user-1",
        "order-1",
        Some("pa-1"),
        Some("500"),
        Some("buyer_request"),
        "req-refund-1",
        "idem-refund-1",
    )
    .expect("command");

    let first = store
        .create_owner_refund(command.clone())
        .await
        .expect("first refund");
    let second = store
        .create_owner_refund(command)
        .await
        .expect("replay refund");

    assert_eq!(first.refund_id, second.refund_id);
    assert_eq!(first.refund_no, second.refund_no);
}

#[tokio::test]
async fn refund_list_uses_sql_pagination() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    seed_paid_order_with_attempt(&pool).await;

    let store = SqliteCommerceRefundStore::new(pool);
    for index in 0..3 {
        let command = CreateOwnerRefundCommand::new(
            "100001",
            None,
            "user-1",
            "order-1",
            Some("pa-1"),
            Some("100"),
            None,
            &format!("req-refund-{index}"),
            &format!("idem-refund-{index}"),
        )
        .expect("command");
        store.create_owner_refund(command).await.expect("refund");
    }

    let page = store
        .list_owner_refunds(
            RefundListQuery::new("100001", None, "user-1", None)
                .expect("query")
                .with_paging(0, 2),
        )
        .await
        .expect("list");

    assert_eq!(2, page.items.len());
    assert_eq!(3, page.total_items);
}

#[tokio::test]
async fn cancel_payment_intent_rejects_invalid_transition() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    let now = "2026-07-05T10:00:00Z";
    sqlx::query(
        r#"
        INSERT INTO commerce_payment_intent
            (id, tenant_id, owner_user_id, order_id, payment_intent_no, amount, status,
             idempotency_key, created_at, updated_at)
        VALUES ('pi-succeeded', '100001', 'user-1', 'order-1', 'PI-S', '1000', 'succeeded',
                'pi-idem-s', ?, ?)
        "#,
    )
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed succeeded intent");

    let store = SqliteCommercePaymentIntentStore::new(pool);
    let command = CancelOwnerPaymentIntentCommand::new("100001", None, "user-1", "pi-succeeded")
        .expect("command");

    let error = store
        .cancel_owner_payment_intent(command)
        .await
        .expect_err("must reject cancel on succeeded intent");
    assert!(error.message().contains("not cancelable"));
}

#[tokio::test]
async fn cancel_pending_payment_intent_succeeds() {
    let pool = payment_store_e2e_sqlite_memory_pool().await;
    let now = "2026-07-05T10:00:00Z";
    sqlx::query(
        r#"
        INSERT INTO commerce_payment_intent
            (id, tenant_id, owner_user_id, order_id, payment_intent_no, amount, status,
             idempotency_key, created_at, updated_at)
        VALUES ('pi-pending', '100001', 'user-1', 'order-1', 'PI-P', '1000', 'pending',
                'pi-idem-p', ?, ?)
        "#,
    )
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed pending intent");

    let store = SqliteCommercePaymentIntentStore::new(pool.clone());
    let command = CancelOwnerPaymentIntentCommand::new("100001", None, "user-1", "pi-pending")
        .expect("command");

    let view = store
        .cancel_owner_payment_intent(command)
        .await
        .expect("cancel pending");
    assert_eq!("canceled", view.status);

    let detail = store
        .retrieve_owner_payment_intent(
            PaymentIntentDetailQuery::new("100001", None, "user-1", "pi-pending")
                .expect("detail query"),
        )
        .await
        .expect("detail")
        .expect("row");
    assert_eq!("canceled", detail.status);
}
