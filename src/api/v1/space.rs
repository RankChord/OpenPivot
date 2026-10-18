use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get},
    Json,
    Router,
};
use uuid::Uuid;

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

// 空间相关的API路由
pub fn routes() -> Router<AppState> {
    // 根路由，对应空间相关的操作
    let root_routes = get(list_my_spaces)
        .post(create_space);

    // 成员相关的路由
    let member_routes = get(list_space_members)
        .post(add_space_member);

    // 消息相关的路由
    let message_routes = get(list_space_messages)
        .post(create_space_message);

    Router::new()
        .route("/", root_routes)
        .route("/{id}/members", member_routes)
        .route("/{id}/messages", message_routes)
}

// --------------------- 创建空间 --------------------
async fn create_space(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateSpaceRequest>,
) -> Result<Json<CreateSpaceResponse>, AppError> {
    // 验证用户身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 去掉空格内容
    let name = payload.name.trim();

    // 验证空间名称是否为空
    if name.is_empty() {
        return Err(AppError::BadRequest);
    }

    // 创建空间
    let created_space = space::create_space(
        &state.db,
        current_user_id,
        name,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 将创建者添加为空间成员，角色为Owner
    space::add_space_member(
        &state.db,
        created_space.id,
        current_user_id,
        SpaceMemberRole::Owner,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 返回创建的空间信息
    Ok(Json(created_space.into_response()))
}

// --------------------- 获取当前用户的所有空间 --------------------
async fn list_my_spaces(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CreateSpaceResponse>>, AppError> {
    // 验证用户身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 调用空间 Repository，获取当前用户的所有空间
    let spaces = space::list_my_spaces(
        &state.db,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 将空间列表转换为响应格式
    let response = spaces
        .into_iter()
        .map(|space| space.into_response())
        .collect();

    // 返回空间列表作为 JSON 响应
    Ok(Json(response))
}

// --------------------- 添加空间成员 --------------------
async fn add_space_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space_id): Path<Uuid>,
    Json(payload): Json<AddSpaceMemberRequest>,
) -> Result<Json<AddSpaceMemberResponse>, AppError> {
    // 验证用户身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 检查当前用户是否是该空间的成员
    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 如果当前用户不是该空间的成员，则返回403 Forbidden错误
    if !is_member {
        return Err(AppError::Forbidden);
    }

    // 调用空间 Repository，添加新的空间成员
    let member = space::add_space_member(
        &state.db,
        space_id,
        payload.user_id,
        SpaceMemberRole::Member,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 将添加的成员信息转换为响应格式
    Ok(Json(member.into_response()))
}

// --------------------- 获取空间成员列表 --------------------
async fn list_space_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space_id): Path<Uuid>,
) -> Result<Json<Vec<AddSpaceMemberResponse>>, AppError> {
    // 验证用户身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 检查当前用户是否是该空间的成员
    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 如果当前用户不是该空间的成员，则返回403 Forbidden错误
    if !is_member {
        return Err(AppError::Forbidden);
    }

    // 调用空间 Repository，获取空间成员列表
    let members = space::list_space_members(&state.db, space_id)
        .await
        .map_err(|_| AppError::Internal)?;

    // 将空间成员列表转换为响应格式
    let response = members
        .into_iter()
        .map(|member| member.into_response())
        .collect();
        
    // 返回空间成员列表作为 JSON 响应
    Ok(Json(response))
}

// --------------------- 创建空间消息 --------------------
async fn create_space_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space_id): Path<Uuid>,
    Json(payload): Json<CreateSpaceMessageRequest>,
) -> Result<Json<SpaceMessage>, AppError> {
    // 验证用户身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 去掉空格内容
    let content = payload.content.trim();

    // 验证消息内容是否为空
    if content.is_empty() {
        return Err(AppError::BadRequest);
    }

    // 检查当前用户是否是该空间的成员
    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 如果当前用户不是该空间的成员，则返回403 Forbidden错误
    if !is_member {
        return Err(AppError::Forbidden);
    }

    // 创建空间消息
    let message = space::create_space_message(
        &state.db,
        space_id,
        current_user_id,
        content,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 返回创建的空间消息信息
    Ok(Json(message))
}

// --------------------- 获取空间消息列表 --------------------
async fn list_space_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space_id): Path<Uuid>,
) -> Result<Json<Vec<SpaceMessage>>, AppError> {
    // 验证用户身份
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 检查当前用户是否是该空间的成员
    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_user_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 如果当前用户不是该空间的成员，则返回403 Forbidden错误
    if !is_member {
        return Err(AppError::Forbidden);
    }

    // 获取空间消息列表
    let messages = space::list_space_messages(
        &state.db,
        space_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 返回空间消息列表作为 JSON 响应
    Ok(Json(messages))
}

