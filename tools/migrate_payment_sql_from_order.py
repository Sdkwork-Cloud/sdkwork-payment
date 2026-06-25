#!/usr/bin/env python3
"""Move payment_intent/refund SQL modules from order repo to payment repo."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1].parent
ORDER = ROOT / "sdkwork-order/crates/sdkwork-commerce-order-repository-sqlx/src"
PAY = Path(__file__).resolve().parents[1] / "crates/sdkwork-commerce-payment-repository-sqlx/src"

STORE_HEADER = """#[derive(Debug, Clone)]
pub struct {store} {{
    pool: {pool},
    order_store: Arc<{order_store}>,
}}

impl {store} {{
    pub fn new(pool: {pool}) -> Self {{
        Self {{
            pool: pool.clone(),
            order_store: Arc::new({order_store}::new(pool)),
        }}
    }}

    pub fn pool(&self) -> &{pool} {{
        &self.pool
    }}
}}

impl {store} {{"""


def migrate_file(
    src_name: str,
    dst_name: str,
    order_store: str,
    store_name: str,
    pool_type: str,
    sqlx_import: str,
    sqlx_import_new: str,
) -> None:
    text = (ORDER / src_name).read_text(encoding="utf-8")
    text = text.replace(f"use crate::{order_store}::{order_store_type(order_store)};", "")
    text = text.replace(
        "use sdkwork_commerce_order_service::OrderOwnerDetailQuery;",
        "use sdkwork_commerce_order_service::OrderOwnerDetailQuery;\n"
        "use sdkwork_commerce_order_repository_sqlx::"
        f"{order_store_type(order_store)};\n"
        "use std::sync::Arc;",
    )
    text = text.replace(sqlx_import, sqlx_import_new)
    old_impl = f"impl {order_store_type(order_store)} {{"
    text = text.replace(
        old_impl,
        STORE_HEADER.format(
            store=store_name,
            pool=pool_type,
            order_store=order_store_type(order_store),
        ),
    )
    text = text.replace("self.retrieve_owner_order", "self.order_store.retrieve_owner_order")
    (PAY / dst_name).write_text(text, encoding="utf-8")


def order_store_type(order_store: str) -> str:
    return "SqliteCommerceOrderStore" if "sqlite" in order_store else "PostgresCommerceOrderStore"


def main() -> None:
    migrate_file(
        "sqlite_payment_intent.rs",
        "sqlite_payment_intent.rs",
        "sqlite_order",
        "SqliteCommercePaymentIntentStore",
        "SqlitePool",
        "use sqlx::{Row, Sqlite, Transaction};",
        "use sqlx::{Row, Sqlite, SqlitePool, Transaction};",
    )
    migrate_file(
        "postgres_payment_intent.rs",
        "postgres_payment_intent.rs",
        "postgres_order",
        "PostgresCommercePaymentIntentStore",
        "PgPool",
        "use sqlx::{Postgres, Row, Transaction};",
        "use sqlx::{PgPool, Postgres, Row, Transaction};",
    )
    migrate_file(
        "sqlite_refund.rs",
        "sqlite_refund.rs",
        "sqlite_order",
        "SqliteCommerceRefundStore",
        "SqlitePool",
        "use sqlx::{Row, Sqlite, Transaction};",
        "use sqlx::{Row, Sqlite, SqlitePool, Transaction};",
    )
    migrate_file(
        "postgres_refund.rs",
        "postgres_refund.rs",
        "postgres_order",
        "PostgresCommerceRefundStore",
        "PgPool",
        "use sqlx::{Postgres, Row, Transaction};",
        "use sqlx::{PgPool, Postgres, Row, Transaction};",
    )
    print("migrated payment_intent/refund SQL modules to payment repository")


if __name__ == "__main__":
    main()
