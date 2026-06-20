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
    models::{
        conversation::{CreateDirectConversationRequest, ConversationResponse},
        message::{Message, SendMessageRequest},
    },
    repository::{conversation, friend, message},
};

pub fn routes() -> Router<AppState> {
    let message_routes = get(list_messages)
        .post(send_message);

    Router::new()
        .route("/", get(list_conversations))
        .route("/direct", post(create_direct_conversation))
        .route("/{id}/messages", message_routes)
}

pub async fn create_direct_conversation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateDirectConversationRequest>,
) -> Result<Json<ConversationResponse>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    if current_user_id == payload.user_id {
        return Err(AppError::BadRequest);
    }

    let is_friend = friend::friendship_exists(
        &state.db,
        current_user_id,
        payload.user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if !is_friend {
        return Err(AppError::Forbidden);
    }

    let conversation = conversation::create_or_get_direct_conversation(
        &state.db,
        current_user_id,
        payload.user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(conversation.into_response()))
}


pub async fn list_conversations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ConversationResponse>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let conversations = conversation::list_conversations(
        &state.db,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    let response = conversations
        .into_iter()
        .map(|conversation| conversation.into_response())
        .collect();

    Ok(Json(response))
}

pub async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<i64>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<Message>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    if payload.content.trim().is_empty() {
        return Err(AppError::BadRequest);
    }

    let in_conversation = conversation::user_in_conversation(
        &state.db,
        conversation_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if !in_conversation {
        return Err(AppError::Forbidden);
    }

    let message = message::create_message(
        &state.db,
        conversation_id,
        current_user_id,
        payload.content.trim(),
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(message))
}

pub async fn list_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<i64>,
) -> Result<Json<Vec<Message>>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let in_conversation = conversation::user_in_conversation(
        &state.db,
        conversation_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if !in_conversation {
        return Err(AppError::Forbidden);
    }

    let messages = message::list_messages(&state.db, conversation_id)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(Json(messages))
}