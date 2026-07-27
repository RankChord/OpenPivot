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
    models::collaborator::{CreateCollaboratorRequest, CollaboratorRequestResponse, CollaboratorItem},
    repository::{collaborator, user},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/requests", post(create_collaborator_request))
        .route("/requests", get(list_received_pending_requests))
        .route("/requests/{id}/accept", post(accept_collaborator_request))
        .route("/requests/{id}/reject", post(reject_collaborator_request))
        .route("/", get(list_collaborators))
        
}

pub async fn create_collaborator_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateCollaboratorRequest>,
) -> Result<Json<CollaboratorRequestResponse>, AppError> {
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

    let already_collaborators = collaborator::collaboratorship_exists(
        &state.db,
        current_user_id,
        payload.user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if already_collaborators {
        return Err(AppError::Conflict);
    }

    let pending_request = collaborator::find_pending_request_between_users(
        &state.db,
        current_user_id,
        payload.user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if pending_request.is_some() {
        return Err(AppError::Conflict);
    }

    let request = collaborator::create_collaborator_request(
        &state.db,
        current_user_id,
        payload.user_id,
        payload.message.as_deref(),
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(request.into_response()))
}

pub async fn accept_collaborator_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<i64>,
) -> Result<Json<CollaboratorRequestResponse>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let request = collaborator::accept_collaborator_request(
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

pub async fn reject_collaborator_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<i64>,
) -> Result<Json<CollaboratorRequestResponse>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let request = collaborator::reject_collaborator_request(
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
) -> Result<Json<Vec<CollaboratorRequestResponse>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let requests = collaborator::list_received_pending_requests(
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
pub async fn list_collaborators(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CollaboratorItem>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let collaborators = collaborator::list_collaborators(&state.db, current_user_id)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(Json(collaborators))
}