use sqlx::SqlitePool;

pub fn payment_store_e2e_migration_sql() -> &'static str {
    include_str!("../test_migrations/0001_payment_store_e2e.sql")
}

pub async fn payment_store_e2e_sqlite_memory_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("payment store e2e sqlite memory pool");
    for statement in split_sql_statements(payment_store_e2e_migration_sql()) {
        sqlx::query(&statement)
            .execute(&pool)
            .await
            .unwrap_or_else(|error| {
                panic!("payment store e2e migration failed on `{statement}`: {error}")
            });
    }
    pool
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(|chunk| {
            chunk
                .lines()
                .filter(|line| {
                    let trimmed = line.trim_start();
                    !trimmed.is_empty() && !trimmed.starts_with("--")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map(|statement| statement.trim().to_string())
        .filter(|statement| !statement.is_empty())
        .collect()
}
