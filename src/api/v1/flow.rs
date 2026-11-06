use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json,
    Router,
};
use uuid::Uuid;

use crate::{
    api::v1::extractors::require_actor_id,
    app::AppState,
    error::AppError,
    models::flow::{
        CompleteFlowTaskRequest,
        CompleteFlowTaskResponse,
        CreateFlowRequest,
        FlowResponse,
        StartFlowRunRequest,
        StartFlowRunResponse,
    },
    repository::{flow, space},
};

// 创建路由节点
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/spaces/{space_id}/flows",
            get(list_flows).post(create_flow),
        )
        .route(
            "/spaces/{space_id}/flows/{flow_id}/runs",
            post(start_flow_run),
        )
        .route(
            "/flow-tasks/{task_id}/complete",
            post(complete_flow_task),
        )
}

// 创建工作流
pub async fn create_flow(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(space_id): Path<Uuid>,
    Json(payload): Json<CreateFlowRequest>,
) -> Result<Json<FlowResponse>, AppError> {
    // 检查对象用户信息
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 获取当前对象与空间关系
    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_actor_id,
    )
    .await
    .map_err(|_| AppError::Internal("check space membership failed"))?;

    // 判断当前用户是否属于空间
    if !is_member {
        return Err(AppError::Forbidden("not a member of this space"));
    }

    // 创建工作流结构体
    let flow = flow::create_flow(
        &state.db,
        space_id,
        current_actor_id,
        &payload.name,
        payload.description.as_deref(),
    )
    .await
    .map_err(|_| AppError::Internal("create flow failed"))?;

    Ok(Json(flow.into_response()))
}

// 获取工作流列表
pub async fn list_flows(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(space_id): Path<Uuid>,
) -> Result<Json<Vec<FlowResponse>>, AppError> {
    // 获取对象账户信息
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 获取对象与当前空间关系
    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_actor_id,
    )
    .await
    .map_err(|_| AppError::Internal("check space membership failed"))?;

    // 判断当前用户是否属于空间
    if !is_member {
        return Err(AppError::Forbidden("not a member of this space"));
    }

    // 获取工作流列表
    let flows = flow::list_flows_by_space(
        &state.db,
        space_id,
    )
    .await
    .map_err(|_| AppError::Internal("list flows failed" ))?;

    // 将工作流列表转换为响应格式
    let responses = flows
        .into_iter()
        .map(|flow| flow.into_response())
        .collect();

    Ok(Json(responses))
}

// 启动工作流函数
pub async fn start_flow_run(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((space_id, flow_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<StartFlowRunRequest>,
) -> Result<Json<StartFlowRunResponse>, AppError> {
    // 获取对象账户信息
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 获取当前对象与空间关系
    let is_member = space::is_space_member(
        &state.db,
        space_id,
        current_actor_id,
    )
    .await
    .map_err(|_| AppError::Internal("check space membership failed"))?;

    // 判断当前用户是否属于空间
    if !is_member {
        return Err(AppError::Forbidden("not a member of this space"));
    }

    // 获取当前任务执行人与空间关系
    let assignee_is_member = space::is_space_member(
        &state.db,
        space_id,
        payload.assignee_id,
    )
    .await
    .map_err(|_| AppError::Internal("check space membership failed"))?;

    // 判断当前任务执行人是否属于空间
    if !assignee_is_member {
        return Err(AppError::BadRequest("current assignee is not a member of this space"));
    }

    // 启动工作流运行
    let (run, task) = flow::start_manual_flow_run(
        &state.db,
        flow_id,
        space_id,
        current_actor_id,
        payload.assignee_id,
        &payload.task_title,
        payload.task_description.as_deref(),
    )
    .await
    .map_err(|_| AppError::Internal("start flow run failed"))?;

    Ok(Json(StartFlowRunResponse {
        run_id: run.id,
        task_id: task.id,
        status: "waiting_action".to_string(),
    }))
}

// 完成工作流任务
pub async fn complete_flow_task(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(task_id): Path<Uuid>,
    Json(payload): Json<CompleteFlowTaskRequest>,
) -> Result<Json<CompleteFlowTaskResponse>, AppError> {
    // 获取对象账户信息
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 开启事务
    let mut tx = state.db.begin().await
        .map_err(|_| AppError::Internal("transaction begin failed"))?;

    // 获取当前任务信息
    let task = flow::complete_flow_task(
        &mut *tx,
        task_id,
        current_actor_id,
        &payload.result,
    )
    .await
    .map_err(|_| AppError::BadRequest("complete flow task failed"))?;

    // 记录任务完成事件
    flow::append_flow_event(
        &mut *tx,
        task.flow_run_id,
        task.space_id,
        crate::models::flow::FlowEventType::TaskCompleted,
        Some(current_actor_id),
        "{}",
    )
    .await
    .map_err(|_| AppError::Internal("append flow event failed"))?;

    // 创建空间消息
    let message_content = format!(
        "流程任务已完成：{}",
        task.title,
    );

    // 创建空间消息
    space::create_space_message(
        &mut *tx,
        task.space_id,
        current_actor_id,
        &message_content,
    )
    .await
    .map_err(|_| AppError::Internal("create space message failed"))?;

    // 记录空间通知事件
    flow::append_flow_event(
        &mut *tx,
        task.flow_run_id,
        task.space_id,
        crate::models::flow::FlowEventType::SpaceNotified,
        Some(current_actor_id),
        "{}",
    )
    .await
    .map_err(|_| AppError::Internal("append flow event failed"))?;

    // 完成工作流运行
    let run = flow::complete_flow_run(
        &mut *tx,
        task.flow_run_id,
    )
    .await
    .map_err(|_| AppError::Internal("complete flow run failed"))?;

    // 记录工作流完成事件
    flow::append_flow_event(
        &mut *tx,
        task.flow_run_id,
        task.space_id,
        crate::models::flow::FlowEventType::FlowCompleted,
        Some(current_actor_id),
        "{}",
    )
    .await
    .map_err(|_| AppError::Internal("append flow event failed"))?;

    // 提交事务
    tx.commit().await
        .map_err(|_| AppError::Internal("transaction commit failed"))?;

    Ok(Json(CompleteFlowTaskResponse {
        task_id: task.id,
        run_id: run.id,
        status: "completed".to_string(),
    }))
}