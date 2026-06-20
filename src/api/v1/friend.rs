use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json,
    Router,
};

use crate::{
    api::v1::extractors::require_user_id,
    app::AppState,
    error::AppError,
    models::friend::{CreateFriendRequest, FriendRequestResponse, FriendItem},
    repository::{friend, user},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/requests", post(create_friend_request))
        .route("/requests", get(list_received_pending_requests))
        .route("/requests/{id}/accept", post(accept_friend_request))
        .route("/requests/{id}/reject", post(reject_friend_request))
        .route("/", get(list_friends))
        
}

pub async fn create_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateFriendRequest>,
) -> Result<Json<FriendRequestResponse>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    if current_user_id == payload.user_id {
        return Err(AppError::BadRequest);
    }

    let target_user = user::find_user_by_id(&state.db, payload.user_id)
        .await
        .map_err(|_| AppError::Internal)?;

    if target_user.is_none() {
        return Err(AppError::BadRequest);
    }

    let already_friends = friend::friendship_exists(
        &state.db,
        current_user_id,
        payload.user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if already_friends {
        return Err(AppError::Conflict);
    }

    let pending_request = friend::find_pending_request_between_users(
        &state.db,
        current_user_id,
        payload.user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if pending_request.is_some() {
        return Err(AppError::Conflict);
    }

    let request = friend::create_friend_request(
        &state.db,
        current_user_id,
        payload.user_id,
        payload.message.as_deref(),
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(request.into_response()))
}

pub async fn accept_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<i64>,
) -> Result<Json<FriendRequestResponse>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let request = friend::accept_friend_request(
        &state.db,
        request_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    match request {
        Some(request) => Ok(Json(request.into_response())),
        None => Err(AppError::BadRequest),
    }
}

pub async fn reject_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<i64>,
) -> Result<Json<FriendRequestResponse>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let request = friend::reject_friend_request(
        &state.db,
        request_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    match request {
        Some(request) => Ok(Json(request.into_response())),
        None => Err(AppError::BadRequest),
    }
}

pub async fn list_received_pending_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<FriendRequestResponse>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let requests = friend::list_received_pending_requests(
        &state.db,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    let response = requests
        .into_iter()
        .map(|request| request.into_response())
        .collect();

    Ok(Json(response))
}
pub async fn list_friends(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<FriendItem>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let friends = friend::list_friends(&state.db, current_user_id)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(Json(friends))
}