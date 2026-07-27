use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get},
    Json,
    Router,
};

use crate::{
    api::v1::extractors::require_user_id,
    app::AppState,
    error::AppError,
    models::space::{
        CreateSpaceMessageRequest,
        SpaceMessage,
        AddSpaceMemberRequest,
        AddSpaceMemberResponse,
        CreateSpaceRequest,
        CreateSpaceResponse,
        SpaceMemberRole,
    },
    repository::space,
};

pub fn routes() -> Router<AppState> {
    let root_routes = get(list_my_spaces)
        .post(create_space);

    let member_routes = get(list_space_members)
        .post(add_space_member);

    let message_routes = get(list_space_messages)
        .post(create_space_message);

    Router::new()
        .route("/", root_routes)
        .route("/{id}/members", member_routes)
        .route("/{id}/messages", message_routes)
}

pub async fn create_space(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateSpaceRequest>,
) -> Result<Json<SpaceResponse>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let name = payload.name.trim();

    if name.is_empty() {
        return Err(AppError::BadRequest);
    }

    let created_space = space::create_space(
        &state.db,
        current_user_id,
        name,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    space::add_space_member(
        &state.db,
        created_space.id,
        current_user_id,
        SpaceMemberRole::Owner,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(created_space.into_response()))
}

pub async fn list_my_spaces(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<SpaceResponse>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let spaces = space::list_my_spaces(
        &state.db,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    let response = spaces
        .into_iter()
        .map(|space| space.into_response())
        .collect();

    Ok(Json(response))
}

pub async fn add_space_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space_id): Path<i64>,
    Json(payload): Json<AddSpaceMemberRequest>,
) -> Result<Json<SpaceMemberResponse>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if !is_member {
        return Err(AppError::Forbidden);
    }

    let member = space::add_space_member(
        &state.db,
        space_id,
        payload.user_id,
        SpaceMemberRole::Member,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(member.into_response()))
}

pub async fn list_space_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space_id): Path<i64>,
) -> Result<Json<Vec<SpaceMemberResponse>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if !is_member {
        return Err(AppError::Forbidden);
    }

    let members = space::list_space_members(&state.db, space_id)
        .await
        .map_err(|_| AppError::Internal)?;

    let response = members
        .into_iter()
        .map(|member| member.into_response())
        .collect();

    Ok(Json(response))
}

pub async fn create_space_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space_id): Path<i64>,
    Json(payload): Json<CreateSpaceMessageRequest>,
) -> Result<Json<SpaceMessage>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let content = payload.content.trim();

    if content.is_empty() {
        return Err(AppError::BadRequest);
    }

    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if !is_member {
        return Err(AppError::Forbidden);
    }

    let message = space::create_space_message(
        &state.db,
        space_id,
        current_user_id,
        content,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(message))
}

pub async fn list_space_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space_id): Path<i64>,
) -> Result<Json<Vec<SpaceMessage>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if !is_member {
        return Err(AppError::Forbidden);
    }

    let messages = space::list_space_messages(
        &state.db,
        space_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(messages))
}

