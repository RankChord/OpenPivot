use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

use crate::{
    api::v1::extractors::require_user_id,
    app::AppState,
    error::AppError,
    models::user::UserSearchItem,
    repository::user,
};

use axum::{
    routing::get,
    Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/search", get(search_users))
}

#[derive(Debug, Deserialize)]
pub struct SearchUsersQuery {
    pub q: String,
}

pub async fn search_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<Vec<UserSearchItem>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let keyword = query.q.trim();

    if keyword.is_empty() {
        return Err(AppError::BadRequest);
    }

    let users = user::search_users(
        &state.db,
        current_user_id,
        keyword,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(users))
}