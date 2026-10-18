//! 管理端 HTTP 适配器与静态前端。
//!
//! 此层只做路由、JSON 解析、身份边界与错误映射；规则统一委托给 Store。
//! SQLite 是同步 API，使用 spawn_blocking，避免数据库锁阻塞异步 HTTP 线程。
use crate::{
    auth::{self, AuthState},
    config::{Config, MAX_BODY_BYTES},
    mcp::McpServer,
    store::{Store, WorkspaceError},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::{
        StatusCode, Uri,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    middleware,
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use rmcp::transport::{
    StreamableHttpServerConfig,
    streamable_http_server::{session::local::LocalSessionManager, tower::StreamableHttpService},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone)]
struct ApiState {
    store: Store,
}

/// 两个受保护的入口：/api 是管理员，/mcp 是 Agent。页面资源独立于登录。
pub fn router(store: Store, config: Config) -> Router {
    let config = Arc::new(config);
    let auth_state = AuthState {
        store: store.clone(),
        config: config.clone(),
    };
    let api = Router::new()
        .route("/session", get(session))
        .route("/snapshot", get(snapshot))
        .route("/agents", post(register_agent))
        .route("/agents/{id}", patch(update_agent))
        .route("/agents/{id}/token", post(rotate_token))
        .route("/call", post(call))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth::protect_admin,
        ))
        .with_state(ApiState {
            store: store.clone(),
        });

    let mut transport = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_max_request_body_bytes(MAX_BODY_BYTES);
    transport = transport.with_allowed_hosts(
        config
            .allowed_hosts
            .clone()
            .unwrap_or_else(|| vec!["localhost".into(), "127.0.0.1".into(), "[::1]".into()]),
    );
    transport =
        transport.with_allowed_origins(config.allowed_origins.clone().unwrap_or_else(|| {
            vec![
                "http://localhost:*".into(),
                "https://localhost:*".into(),
                "http://127.0.0.1:*".into(),
                "https://127.0.0.1:*".into(),
                "http://[::1]:*".into(),
                "https://[::1]:*".into(),
            ]
        }));
    let mcp = StreamableHttpService::new(
        move || Ok(McpServer::new(store.clone())),
        LocalSessionManager::default().into(),
        transport,
    );
    let mcp_routes = Router::new()
        .nest_service("/mcp", mcp)
        .layer(middleware::from_fn_with_state(
            auth_state,
            auth::protect_mcp,
        ));
    Router::new()
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .nest("/api", api)
        .merge(mcp_routes)
        .fallback(get(asset))
}

struct ApiError {
    status: StatusCode,
    message: String,
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({"error":self.message}))).into_response()
    }
}
impl From<WorkspaceError> for ApiError {
    fn from(error: WorkspaceError) -> Self {
        match error {
            WorkspaceError::Rule(_) | WorkspaceError::Json(_) => Self {
                status: StatusCode::BAD_REQUEST,
                message: error.to_string(),
            },
            _ => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "workspace storage operation failed".into(),
            },
        }
    }
}
fn payload<T>(value: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    value.map(|Json(value)| value).map_err(|error| ApiError {
        status: error.status(),
        message: error.body_text(),
    })
}
async fn database<F>(store: Store, operation: F) -> Result<Json<Value>, ApiError>
where
    F: FnOnce(Store) -> Result<Value, WorkspaceError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || operation(store))
        .await
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "workspace worker failed".into(),
        })?
        .map(Json)
        .map_err(Into::into)
}
async fn session() -> Json<Value> {
    Json(json!({"role":"admin","mcp_url":"/mcp"}))
}
async fn snapshot(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    database(state.store, |store| store.snapshot()).await
}
async fn register_agent(
    State(state): State<ApiState>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let args = payload(body)?;
    database(state.store, move |store| store.register_agent(args)).await
}
async fn update_agent(
    State(state): State<ApiState>,
    Path(agent_id): Path<String>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let args = payload(body)?;
    database(state.store, move |store| {
        store.update_agent(&agent_id, args)
    })
    .await
}
async fn rotate_token(
    State(state): State<ApiState>,
    Path(agent_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    database(state.store, move |store| {
        store.rotate_agent_token(&agent_id)
    })
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCall {
    tool: String,
    #[serde(default = "empty_args")]
    arguments: Value,
    actor: Option<String>,
}
fn empty_args() -> Value {
    json!({})
}
async fn call(
    State(state): State<ApiState>,
    body: Result<Json<ToolCall>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let args = payload(body)?;
    database(state.store, move |store| match args.actor {
        Some(actor) if actor != "__operator" => store.execute(&actor, &args.tool, args.arguments),
        _ => store.admin_execute(&args.tool, args.arguments),
    })
    .await
}

/// 显式嵌入模块，cargo run 即可打开页面；部署二进制无需额外 Node 服务。
/// 不把 URL 当作磁盘路径，未列出的静态资源一律返回 404。
async fn asset(uri: Uri) -> Response {
    let (content, mime) = match uri.path() {
        "/" | "/index.html" => (
            include_str!("../frontend/index.html"),
            "text/html; charset=utf-8",
        ),
        "/styles.css" => (
            include_str!("../frontend/styles.css"),
            "text/css; charset=utf-8",
        ),
        "/task-graph.css" => (
            include_str!("../frontend/task-graph.css"),
            "text/css; charset=utf-8",
        ),
        "/js/app.js" => (
            include_str!("../frontend/js/app.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/api.js" => (
            include_str!("../frontend/js/api.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/ui.js" => (
            include_str!("../frontend/js/ui.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/task-graph-model.js" => (
            include_str!("../frontend/js/task-graph-model.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/views/overview.js" => (
            include_str!("../frontend/js/views/overview.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/views/agents.js" => (
            include_str!("../frontend/js/views/agents.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/views/tasks.js" => (
            include_str!("../frontend/js/views/tasks.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/views/task-graph.js" => (
            include_str!("../frontend/js/views/task-graph.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/views/task-details.js" => (
            include_str!("../frontend/js/views/task-details.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/views/files.js" => (
            include_str!("../frontend/js/views/files.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/views/inbox.js" => (
            include_str!("../frontend/js/views/inbox.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/views/runner.js" => (
            include_str!("../frontend/js/views/runner.js"),
            "text/javascript; charset=utf-8",
        ),
        "/js/views/connection.js" => (
            include_str!("../frontend/js/views/connection.js"),
            "text/javascript; charset=utf-8",
        ),
        _ => return (StatusCode::NOT_FOUND, "not found").into_response(),
    };
    let mut response =
        ([(CONTENT_TYPE, mime), (CACHE_CONTROL, "no-store")], content).into_response();
    response
        .headers_mut()
        .insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    response
        .headers_mut()
        .insert("Referrer-Policy", "same-origin".parse().unwrap());
    response.headers_mut().insert("Content-Security-Policy","default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; frame-ancestors 'none'".parse().unwrap());
    response
}
