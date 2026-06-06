use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn create_pool(database_url: String) -> PgPool {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .connect(&database_url)
        .await
        .expect("[数据库管理]: 无法连接到目标数据库")
}

pub async fn init_db(pgpool: &PgPool)  -> bool {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            nickname TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pgpool)
    .await
    .expect("[数据库管理]: 无法创建用户表");
    true
}
