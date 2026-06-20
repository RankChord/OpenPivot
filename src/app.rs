use sqlx::PgPool;

use crate::init::config::ConfigFile;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: ConfigFile,
}

use crate::api::v1;
use axum::Router;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .nest("/v1", v1::routes())
        .with_state(state)
}