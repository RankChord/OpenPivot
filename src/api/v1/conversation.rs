use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json,
    Router,
};
use uuid::Uuid;

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

// 创建私聊会话
pub async fn create_direct_conversation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateDirectConversationRequest>,
) -> Result<Json<ConversationResponse>, AppError> {
    // 获取用户鉴权身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    
    // 判断用户鉴权是否合法
    if current_user_id == payload.user_id {
        return Err(AppError::BadRequest);
    }
    
    // 获取私聊对象是否为好友 ToDo: Friend定义 -> collaborator 
    let is_collaborator = friend::friendship_exists(
        &state.db,
        current_user_id,
        payload.user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;
    
    // 判断是否为好友 TODO: 后续改进为非好友私聊标签
    if !is_collaborator {
        return Err(AppError::Forbidden);
    }
    
    // 创建/拉取 私聊对话
    let conversation = conversation::create_or_get_direct_conversation(
        &state.db,
        current_user_id,
        payload.user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(conversation.into_response()))
}

// 获取对话列表
pub async fn list_conversations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ConversationResponse>>, AppError> {
    // 获取用户鉴权身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    
    // 获取对话列表信息
    let conversations = conversation::list_conversations(
        &state.db,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;
    
    // 构造信息返回体
    let response = conversations
        .into_iter()
        .map(|conversation| conversation.into_response())
        .collect();

    Ok(Json(response))
}

// 发送信息数据
pub async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<Uuid>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<Message>, AppError> {
    // 获取用户鉴权身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    
    // 发送内容不能为空
    if payload.content.trim().is_empty() {
        return Err(AppError::BadRequest);
    }

    // 获取用户是否在当前对话里
    let in_conversation = conversation::user_in_conversation(
        &state.db,
        conversation_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;
    
    // 判断用户是否在当前会话里
    if !in_conversation {
        return Err(AppError::Forbidden);
    }
    
    // 创建信息
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

// 获取信息列表
pub async fn list_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<Uuid>,
) -> Result<Json<Vec<Message>>, AppError> {
    // 获取用户鉴权身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    
    // 获取用户是否在请求的会话中
    let in_conversation = conversation::user_in_conversation(
        &state.db,
        conversation_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;
    
    // 判断用户是否在当前的会话中
    if !in_conversation {
        return Err(AppError::Forbidden);
    }
    
    // 构造信息
    let messages = message::list_messages(&state.db, conversation_id)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(Json(messages))
}
