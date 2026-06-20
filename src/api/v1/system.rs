use axum::{http::StatusCode, response::IntoResponse};

use axum::{
    routing::get,
    Router,
};

use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
}

pub async fn health() -> impl IntoResponse{
    (StatusCode::OK, "openpivot status health")
}