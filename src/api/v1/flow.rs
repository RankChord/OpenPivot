use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json,
    Router,
};

use crate::{
    api::v1::extractors::require_user_id,
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

pub async fn create_flow(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(space_id): Path<i64>,
    Json(payload): Json<CreateFlowRequest>,
) -> Result<Json<FlowResponse>, AppError> {
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

    let flow = flow::create_flow(
        &state.db,
        space_id,
        current_user_id,
        &payload.name,
        payload.description.as_deref(),
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(flow.into_response()))
}

pub async fn list_flows(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(space_id): Path<i64>,
) -> Result<Json<Vec<FlowResponse>>, AppError> {
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

    let flows = flow::list_flows_by_space(
        &state.db,
        space_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    let responses = flows
        .into_iter()
        .map(|flow| flow.into_response())
        .collect();

    Ok(Json(responses))
}

pub async fn start_flow_run(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((space_id, flow_id)): Path<(i64, i64)>,
    Json(payload): Json<StartFlowRunRequest>,
) -> Result<Json<StartFlowRunResponse>, AppError> {
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

    let assignee_is_member = space::is_space_member(
        &state.db,
        space_id,
        payload.assignee_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    if !assignee_is_member {
        return Err(AppError::BadRequest);
    }

    let (run, task) = flow::start_manual_flow_run(
        &state.db,
        flow_id,
        space_id,
        current_user_id,
        payload.assignee_id,
        &payload.task_title,
        payload.task_description.as_deref(),
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(StartFlowRunResponse {
        run_id: run.id,
        task_id: task.id,
        status: "waiting_action".to_string(),
    }))
}

pub async fn complete_flow_task(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(task_id): Path<i64>,
    Json(payload): Json<CompleteFlowTaskRequest>,
) -> Result<Json<CompleteFlowTaskResponse>, AppError> {
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    let task = flow::complete_flow_task(
        &state.db,
        task_id,
        current_user_id,
        &payload.result,
    )
    .await
    .map_err(|_| AppError::BadRequest)?;

    flow::append_flow_event(
        &state.db,
        task.flow_run_id,
        task.space_id,
        crate::models::flow::FlowEventType::TaskCompleted,
        Some(current_user_id),
        "{}",
    )
    .await
    .map_err(|_| AppError::Internal)?;

    let message_content = format!(
        "流程任务已完成：{}",
        task.title,
    );

    space::create_space_message(
        &state.db,
        task.space_id,
        current_user_id,
        &message_content,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    flow::append_flow_event(
        &state.db,
        task.flow_run_id,
        task.space_id,
        crate::models::flow::FlowEventType::SpaceNotified,
        Some(current_user_id),
        "{}",
    )
    .await
    .map_err(|_| AppError::Internal)?;

    let run = flow::complete_flow_run(
        &state.db,
        task.flow_run_id,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    flow::append_flow_event(
        &state.db,
        task.flow_run_id,
        task.space_id,
        crate::models::flow::FlowEventType::FlowCompleted,
        Some(current_user_id),
        "{}",
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(CompleteFlowTaskResponse {
        task_id: task.id,
        run_id: run.id,
        status: "completed".to_string(),
    }))
}