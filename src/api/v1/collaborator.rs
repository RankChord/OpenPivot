use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json,
    Router,
};
use uuid::Uuid;

use crate::{
    api::v1::extractors::require_actor_id,
    app::AppState,
    error::AppError,
    models::collaborator::{CreateCollaboratorRequest, CollaboratorRequestResponse, CollaboratorItem},
    repository::{collaborator, actor},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/requests", post(create_collaborator_request))                                      // 创建协作者请求
        .route("/requests", get(list_received_pending_requests))                                    // 获取协作者请求
        .route("/requests/{id}/accept", post(accept_collaborator_request))                          // 通过协作者请求
        .route("/requests/{id}/reject", post(reject_collaborator_request))                          // 拒绝协作者请求
        .route("/", get(list_collaborators))                                                        // 获取协作者列表
        
}

pub async fn create_collaborator_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateCollaboratorRequest>,
) -> Result<Json<CollaboratorRequestResponse>, AppError> {
    // 尝试获取请求中的用户鉴权信息
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    
    // 判断请求鉴权是否合法
    if current_actor_id == payload.actor_id {
        return Err(AppError::BadRequest("cannot send request to yourself"));
    }

    // 寻找对应用户ID
    let target_actor = actor::find_actor_by_id(&state.db, payload.actor_id)
        .await
        .map_err(|_| AppError::Internal("target actor lookup failed"))?;
    
    // 判断用户是否存在
    if target_actor.is_none() {
        return Err(AppError::BadRequest("target actor not found"));
    }
    
    // 查询协作关系 
    let already_collaborators = collaborator::collaboratorship_exists(
        &state.db,
        current_actor_id,
        payload.actor_id,
    )
    .await
    .map_err(|_| AppError::Internal("collaboratorship check failed"))?;
    
    // 判断是否已为协作关系
    if already_collaborators {
        return Err(AppError::Conflict("already collaborators"));
    }
    
    // 查询协作关系请求
    let pending_request = collaborator::find_pending_request_between_actors(
        &state.db,
        current_actor_id,
        payload.actor_id,
    )
    .await
    .map_err(|_| AppError::Internal("pending request lookup failed"))?;

    // 判断是否已有协作关系请求
    
    if pending_request.is_some() {
        return Err(AppError::Conflict("collaboration request already pending"));
    }
    // 构造协作关系请求
    let request = collaborator::create_collaborator_request(
        &state.db,
        current_actor_id,
        payload.actor_id,
        payload.message.as_deref(),
    )
    .await
    .map_err(|_| AppError::Internal("create collaborator request failed"))?;

    Ok(Json(request.into_response()))
}

// 通过协作请求
pub async fn accept_collaborator_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
) -> Result<Json<CollaboratorRequestResponse>, AppError> {
    // 获取用户鉴权信息
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    
    //  构造通过鉴权请i去
    let request = collaborator::accept_collaborator_request(
        &state.db,
        request_id,
        current_actor_id,
    )
    .await
    .map_err(|_| AppError::Internal("accept collaborator request failed"))?;
    
    // 判断请求是否合法
    match request {
        Some(request) => Ok(Json(request.into_response())),
        None => Err(AppError::BadRequest("request not found or not addressee")),
    }
}

// 拒绝协作请求
pub async fn reject_collaborator_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
) -> Result<Json<CollaboratorRequestResponse>, AppError> {
    // 获取用户鉴权信息
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    
    // 构造拒绝协作请求体
    let request = collaborator::reject_collaborator_request(
        &state.db,
        request_id,
        current_actor_id,
    )
    .await
    .map_err(|_| AppError::Internal("reject collaborator request failed"))?;
    
    // 判断请求是否合法
    match request {
        Some(request) => Ok(Json(request.into_response())),
        None => Err(AppError::BadRequest("request not found or not addressee")),
    }
}

// 获取请求列表
pub async fn list_received_pending_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CollaboratorRequestResponse>>, AppError> {
    // 获取用户鉴权信息
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    
    // 构造列表请求的请求体
    let requests = collaborator::list_received_pending_requests(
        &state.db,
        current_actor_id,
    )
    .await
    .map_err(|_| AppError::Internal("list pending requests failed"))?;
    
    // 构造返回体
    let response = requests
        .into_iter()
        .map(|request| request.into_response())
        .collect();

    Ok(Json(response))
}

// 获取协作列表
pub async fn list_collaborators(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CollaboratorItem>>, AppError> {
    // 获取用户鉴权信息
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    
    // 获取协作者列表
    let collaborators = collaborator::list_collaborators(&state.db, current_actor_id)
        .await
        .map_err(|_| AppError::Internal("list collaborators failed"))?;
    
    Ok(Json(collaborators))
}
