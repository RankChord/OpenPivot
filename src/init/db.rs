use sqlx::{postgres::PgPoolOptions, PgPool};

use super::config::DatabaseConfig;

pub async fn create_pool(config: &DatabaseConfig) -> PgPool {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(1)
        .connect(&config.connection_url())
        .await
        .expect("[数据库管理]: 无法连接到目标数据库")
}

pub async fn run_migrations(pgpool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pgpool)
        .await
        .expect("[数据库管理]: 数据库迁移失败");
}
