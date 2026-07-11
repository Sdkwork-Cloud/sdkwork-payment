use sdkwork_payment_service::{
    CancelOwnerPaymentIntentCommand, CreateOwnerRefundCommand, PaymentIntentDetailQuery,
    RefundListQuery,
};

use crate::{
    test_sqlite_pool::payment_store_e2e_sqlite_memory_pool, SqliteCommercePaymentIntentStore,
    SqliteCommerceRefundStore,
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
