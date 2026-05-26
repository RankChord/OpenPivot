use crate::cmd::load_config;
use crate::runtime::build_agent;
use agent_config::AgentConfig;
use agent_core::{AgentEvent, AgentEventCollector, AgentEventSink, AgentInput, AgentOutput};
use agent_cron::{CronJob, CronScheduler, CronStore, NewCronJob, default_legacy_jobs_file};
use agent_kanban::{
    ExecutionResult, KanbanStore, NewTask, TaskEventRecord, TaskExecutor, TaskRecord,
    TaskRunRecord, TaskStatus, WorkerRuntime,
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{
        Html, Response,
        sse::{Event, Sse},
    },
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

// 简单的 Gateway State
pub struct GatewayState {
    pub port: u16,
    pub config: AgentConfig,
    pub http_token: Option<String>,
    pub cron_store: Option<CronStore>,
    pub kanban_store: Option<KanbanStore>,
}

pub type SharedState = Arc<GatewayState>;

#[derive(Deserialize)]
pub struct TelegramWebhook {
    pub message: Option<TelegramMessage>,
}

#[derive(Deserialize)]
pub struct TelegramMessage {
    pub chat: TelegramChat,
    pub text: Option<String>,
}

#[derive(Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub session_id: String,
    pub output: String,
    pub events: Vec<SerializableAgentEvent>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CronJobResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub schedule: serde_json::Value,
    pub payload_kind: String,
    pub payload_json: serde_json::Value,
    pub missed_run_policy: String,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<CronJob> for CronJobResponse {
    fn from(job: CronJob) -> Self {
        Self {
            id: job.id,
            name: job.name,
            description: job.description,
            enabled: job.enabled,
            schedule: serde_json::to_value(job.schedule).unwrap_or(serde_json::Value::Null),
            payload_kind: job.payload_kind.as_str().to_string(),
            payload_json: job.payload_json,
            missed_run_policy: job.missed_run_policy.as_str().to_string(),
            next_run_at: job.next_run_at,
            last_run_at: job.last_run_at,
            created_at: job
                .created_at
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            updated_at: job
                .updated_at
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TaskResponse {
    pub id: String,
    pub source: String,
    pub source_id: Option<String>,
    pub title: String,
    pub status: TaskStatus,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TaskRunResponse {
    pub id: String,
    pub task_id: String,
    pub worker_id: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub session_id: Option<String>,
    pub output_summary: Option<String>,
    pub error: Option<String>,
    pub log_path: Option<String>,
}

impl From<TaskRunRecord> for TaskRunResponse {
    fn from(run: TaskRunRecord) -> Self {
        Self {
            id: run.id,
            task_id: run.task_id,
            worker_id: run.worker_id,
            status: run.status,
            started_at: run.started_at,
            finished_at: run.finished_at,
            duration_ms: run.duration_ms,
            session_id: run.session_id,
            output_summary: run.output_summary,
            error: run.error,
            log_path: run.log_path,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TaskEventResponse {
    pub id: String,
    pub task_id: String,
    pub run_id: Option<String>,
    pub event_type: String,
    pub payload_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub sequence: i64,
}

impl From<TaskEventRecord> for TaskEventResponse {
    fn from(event: TaskEventRecord) -> Self {
        Self {
            id: event.id,
            task_id: event.task_id,
            run_id: event.run_id,
            event_type: event.event_type,
            payload_json: event.payload_json,
            created_at: event.created_at,
            sequence: event.sequence,
        }
    }
}

impl From<TaskRecord> for TaskResponse {
    fn from(task: TaskRecord) -> Self {
        Self {
            id: task.id,
            source: task.source,
            source_id: task.source_id,
            title: task.title,
            status: task.status,
            last_error: task.last_error,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TaskListQuery {
    pub status: Option<TaskStatus>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SerializableAgentEvent {
    UserMessage {
        content: String,
    },
    ToolCall {
        id: String,
        name: String,
    },
    ToolResult {
        id: String,
        ok: bool,
        content: String,
        error: Option<String>,
    },
    Final {
        content: String,
    },
    BudgetExhausted {
        reason: String,
    },
}

impl From<AgentEvent> for SerializableAgentEvent {
    fn from(event: AgentEvent) -> Self {
        match event {
            AgentEvent::UserMessage { content } => Self::UserMessage { content },
            AgentEvent::ToolCall { id, name } => Self::ToolCall { id, name },
            AgentEvent::ToolResult {
                id,
                ok,
                content,
                error,
            } => Self::ToolResult {
                id,
                ok,
                content,
                error,
            },
            AgentEvent::Final { content } => Self::Final { content },
            AgentEvent::BudgetExhausted { reason } => Self::BudgetExhausted { reason },
        }
    }
}

pub fn sse_event_line(event: &AgentEvent) -> String {
    let serializable = SerializableAgentEvent::from(event.clone());
    let json = serde_json::to_string(&serializable).unwrap_or_else(|_| "{}".into());
    format!("data: {}\n\n", json)
}

pub fn sse_event(event: AgentEvent) -> Event {
    let serializable = SerializableAgentEvent::from(event);
    Event::default().json_data(serializable).unwrap_or_default()
}

pub fn output_from_events(events: &[AgentEvent], error: Option<&str>) -> String {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            AgentEvent::Final { content } => Some(content.clone()),
            AgentEvent::BudgetExhausted { reason } => {
                Some(format!("[Budget exhausted] {}", reason))
            }
            _ => None,
        })
        .unwrap_or_else(|| match error {
            Some(error) => format!("[Error] {}", error),
            None => String::new(),
        })
}

fn default_automation_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("automation.db")
}

struct ServeTaskExecutor {
    config: AgentConfig,
    kanban_store: KanbanStore,
}

impl TaskExecutor for ServeTaskExecutor {
    fn execute<'life0, 'life1, 'async_trait>(
        &'life0 self,
        task: &'life1 TaskRecord,
    ) -> Pin<Box<dyn Future<Output = ExecutionResult> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            match task.payload_kind {
                agent_kanban::PayloadKind::ShellCommand => execute_shell_command(task).await,
                agent_kanban::PayloadKind::AgentPrompt => self.execute_agent_prompt(task).await,
            }
        })
    }
}

impl ServeTaskExecutor {
    async fn execute_agent_prompt(&self, task: &TaskRecord) -> ExecutionResult {
        let Some(prompt) = task
            .payload_json
            .get("prompt")
            .and_then(|value| value.as_str())
        else {
            return ExecutionResult::failed("agent task missing prompt".to_string());
        };

        let agent = build_agent(&self.config, None).await;
        if let Err(error) = self
            .kanban_store
            .append_task_event(
                &task.id,
                None,
                "agent_run_started",
                serde_json::json!({"session_id": format!("task-{}", task.id)}),
            )
            .await
        {
            return ExecutionResult::failed(error.to_string());
        }

        let mut input = AgentInput {
            user_message: prompt.to_string(),
            session_id: format!("task-{}", task.id),
            system_prompt: "You are a helpful assistant executing a scheduled background task."
                .into(),
            history: Vec::new(),
            workspace_dir: std::env::current_dir().unwrap_or_default(),
        };
        let mut sink = PersistingAgentEventSink {
            store: self.kanban_store.clone(),
            task_id: task.id.clone(),
        };
        let output = agent.run_with_events(&mut input, &mut sink).await;

        match output {
            Ok(AgentOutput::Final(content)) => ExecutionResult::succeeded(Some(content)),
            Ok(AgentOutput::Cancelled) => ExecutionResult::failed("Cancelled".to_string()),
            Ok(AgentOutput::BudgetExhausted(reason)) => ExecutionResult::failed(reason),
            Err(error) => ExecutionResult::failed(error.to_string()),
        }
    }
}

struct PersistingAgentEventSink {
    store: KanbanStore,
    task_id: String,
}

#[async_trait::async_trait]
impl AgentEventSink for PersistingAgentEventSink {
    async fn emit(&mut self, event: AgentEvent) -> Result<(), agent_core::AgentError> {
        let payload_json = serde_json::to_value(&event)
            .map_err(|error| agent_core::AgentError::IoError(error.to_string()))?;
        self.store
            .append_task_event(&self.task_id, None, "agent_event", payload_json)
            .await
            .map_err(|error| agent_core::AgentError::IoError(error.to_string()))?;
        Ok(())
    }
}

async fn execute_shell_command(task: &TaskRecord) -> ExecutionResult {
    let Some(command) = task
        .payload_json
        .get("command")
        .and_then(|value| value.as_str())
    else {
        return ExecutionResult::failed("missing shell command".to_string());
    };

    match Command::new("sh").arg("-c").arg(command).output().await {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            ExecutionResult::succeeded(Some(stdout))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                ExecutionResult::failed(format!(
                    "shell command failed with status {} and stdout: {}",
                    output.status, stdout
                ))
            } else {
                ExecutionResult::failed(stderr)
            }
        }
        Err(error) => ExecutionResult::failed(error.to_string()),
    }
}

#[cfg(test)]
mod automation_tests {
    use super::*;
    use agent_kanban::{PayloadKind, TaskRecord, TaskStatus};
    use axum::body::Body;
    use axum::http::{Method, Request};
    use chrono::Utc;
    use tower::ServiceExt;

    async fn executor() -> ServeTaskExecutor {
        let dir = tempfile::tempdir().unwrap().keep();
        let kanban_store = KanbanStore::new(&dir.join("kanban.db")).await.unwrap();
        ServeTaskExecutor {
            config: AgentConfig::default(),
            kanban_store,
        }
    }

    fn task(payload_kind: PayloadKind, payload_json: serde_json::Value) -> TaskRecord {
        let now = Utc::now();
        TaskRecord {
            id: "task-1".into(),
            source: "test".into(),
            source_id: None,
            title: "test task".into(),
            body: String::new(),
            payload_kind,
            payload_json,
            status: TaskStatus::Queued,
            priority: 0,
            claim_lock: None,
            claim_expires_at: None,
            assigned_worker: None,
            last_heartbeat_at: None,
            retry_count: 0,
            max_retries: 1,
            last_error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    fn router_with_http_token(token: Option<&str>) -> Router {
        build_router(Arc::new(
            GatewayState::new(3000, AgentConfig::default())
                .with_http_token(token.map(str::to_string)),
        ))
    }

    #[test]
    fn default_automation_db_path_uses_agent_home() {
        let expected = dirs::home_dir()
            .unwrap_or_default()
            .join(".agent")
            .join("automation.db");

        assert_eq!(default_automation_db_path(), expected);
    }

    #[tokio::test]
    async fn serve_task_executor_succeeds_shell_command_with_trimmed_stdout() {
        let executor = executor().await;
        let task = task(
            PayloadKind::ShellCommand,
            serde_json::json!({"command": "printf 'hello\\n'"}),
        );

        let result = executor.execute(&task).await;

        assert_eq!(result.status, TaskStatus::Succeeded);
        assert_eq!(result.output_summary.as_deref(), Some("hello"));
        assert_eq!(result.error, None);
    }

    #[tokio::test]
    async fn serve_task_executor_fails_shell_command_with_stderr() {
        let executor = executor().await;
        let task = task(
            PayloadKind::ShellCommand,
            serde_json::json!({"command": "printf 'boom' >&2; exit 7"}),
        );

        let result = executor.execute(&task).await;

        assert_eq!(result.status, TaskStatus::Failed);
        assert_eq!(result.error.as_deref(), Some("boom"));
    }

    #[tokio::test]
    async fn serve_task_executor_fails_shell_command_with_status_when_stderr_empty() {
        let executor = executor().await;
        let task = task(
            PayloadKind::ShellCommand,
            serde_json::json!({"command": "printf out; exit 7"}),
        );

        let result = executor.execute(&task).await;
        let error = result.error.unwrap_or_default();

        assert_eq!(result.status, TaskStatus::Failed);
        assert!(!error.is_empty());
        assert!(error.contains("exit status") || error.contains("status"));
        assert!(error.contains("7"));
        assert!(error.contains("out"));
    }

    #[tokio::test]
    async fn serve_task_executor_fails_shell_command_without_command() {
        let executor = executor().await;
        let task = task(PayloadKind::ShellCommand, serde_json::json!({}));

        let result = executor.execute(&task).await;

        assert_eq!(result.status, TaskStatus::Failed);
        assert_eq!(result.error.as_deref(), Some("missing shell command"));
    }

    #[tokio::test]
    async fn persisting_agent_event_sink_persists_events_at_emit_time() {
        let dir = tempfile::tempdir().unwrap();
        let store = KanbanStore::new(&dir.path().join("kanban.db"))
            .await
            .unwrap();
        let created = store
            .create_task(agent_kanban::NewTask {
                source: "test".into(),
                source_id: None,
                title: "queued event task".into(),
                body: String::new(),
                payload_kind: PayloadKind::AgentPrompt,
                payload_json: serde_json::json!({"prompt": "hello"}),
                priority: 0,
                max_retries: 0,
            })
            .await
            .unwrap();
        let mut sink = PersistingAgentEventSink {
            store: store.clone(),
            task_id: created.id.clone(),
        };

        sink.emit(AgentEvent::UserMessage {
            content: "hello".into(),
        })
        .await
        .unwrap();

        let events = store.list_task_events(&created.id).await.unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[1].payload_json["type"], "user_message");
        assert_eq!(events[1].sequence, 1);

        sink.emit(AgentEvent::Final {
            content: "done".into(),
        })
        .await
        .unwrap();

        let events = store.list_task_events(&created.id).await.unwrap();

        assert_eq!(events.len(), 3);
        assert_eq!(events[1].payload_json["type"], "user_message");
        assert_eq!(events[2].payload_json["type"], "final");
        assert_eq!(events[1].sequence, 1);
        assert_eq!(events[2].sequence, 2);
    }

    #[test]
    fn http_task_creation_rejects_shell_command_payload() {
        let payload = agent_kanban::NewTask {
            source: "http".into(),
            source_id: None,
            title: "run shell".into(),
            body: String::new(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: serde_json::json!({"command": "id"}),
            priority: 0,
            max_retries: 0,
        };

        let error = validate_http_task(&payload).unwrap_err();

        assert_eq!(error.0, StatusCode::FORBIDDEN);
        assert_eq!(
            error.1.error,
            "shell_command payloads cannot be created over HTTP"
        );
    }

    #[test]
    fn http_cron_creation_rejects_shell_command_payload() {
        let payload = NewCronJob {
            name: "run shell later".into(),
            description: String::new(),
            schedule: agent_cron::Schedule::parse("1h").unwrap(),
            payload_kind: agent_cron::PayloadKind::ShellCommand,
            payload_json: serde_json::json!({"command": "id"}),
            enabled: true,
            missed_run_policy: agent_cron::MissedRunPolicy::RunOnce,
        };

        let error = validate_http_cron_job(&payload).unwrap_err();

        assert_eq!(error.0, StatusCode::FORBIDDEN);
        assert_eq!(
            error.1.error,
            "shell_command payloads cannot be created over HTTP"
        );
    }

    #[test]
    fn loopback_host_allows_missing_http_token() {
        assert!(validate_http_token_for_host("127.0.0.1", None).is_ok());
        assert!(validate_http_token_for_host("localhost", None).is_ok());
    }

    #[test]
    fn non_loopback_host_requires_http_token() {
        let error = validate_http_token_for_host("0.0.0.0", None).unwrap_err();
        assert!(error.to_string().contains("AGENT_HTTP_TOKEN"));
    }

    #[test]
    fn non_loopback_host_treats_blank_http_token_as_missing() {
        let error = validate_http_token_for_host("0.0.0.0", Some("   ")).unwrap_err();
        assert!(error.to_string().contains("AGENT_HTTP_TOKEN"));
    }

    #[test]
    fn bearer_token_auth_accepts_valid_token() {
        assert!(authorize_bearer_header(Some("Bearer secret"), Some("secret")).is_ok());
    }

    #[test]
    fn bearer_token_auth_rejects_missing_or_invalid_token() {
        assert!(authorize_bearer_header(None, Some("secret")).is_err());
        assert!(authorize_bearer_header(Some("Bearer wrong"), Some("secret")).is_err());
    }

    #[test]
    fn bearer_token_auth_treats_blank_expected_token_as_unconfigured() {
        assert!(authorize_bearer_header(None, Some("   ")).is_ok());
    }

    #[tokio::test]
    async fn protected_get_route_rejects_missing_token_when_configured() {
        let response = router_with_http_token(Some("secret"))
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/cron/jobs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_rejects_wrong_token() {
        let response = router_with_http_token(Some("secret"))
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/cron/jobs")
                    .header(axum::http::header::AUTHORIZATION, "Bearer wrong")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_accepts_valid_token() {
        let response = router_with_http_token(Some("secret"))
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/cron/jobs")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_post_rejects_missing_token_before_malformed_json() {
        let response = router_with_http_token(Some("secret"))
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/cron/jobs")
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn retry_task_returns_conflict_when_max_retries_reached() {
        let dir = tempfile::tempdir().unwrap();
        let store = KanbanStore::new(&dir.path().join("kanban.db"))
            .await
            .unwrap();
        let created = store
            .create_task(NewTask {
                source: "test".into(),
                source_id: None,
                title: "exhausted".into(),
                body: String::new(),
                payload_kind: PayloadKind::AgentPrompt,
                payload_json: serde_json::json!({"prompt": "hello"}),
                priority: 0,
                max_retries: 0,
            })
            .await
            .unwrap();
        let claimed = store
            .claim_next_task("worker-a", 60)
            .await
            .unwrap()
            .unwrap();
        store
            .complete_run(
                &claimed.task.id,
                &claimed.run.id,
                &claimed.claim_lock,
                TaskStatus::Failed,
                None,
                Some("boom"),
            )
            .await
            .unwrap();
        let app = build_router(Arc::new(GatewayState {
            port: 3000,
            config: AgentConfig::default(),
            http_token: None,
            cron_store: None,
            kanban_store: Some(store),
        }));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/tasks/{}/retry", created.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn retry_task_returns_not_found_for_missing_task_when_authorized() {
        let dir = tempfile::tempdir().unwrap();
        let store = KanbanStore::new(&dir.path().join("kanban.db"))
            .await
            .unwrap();
        let app = build_router(Arc::new(GatewayState {
            port: 3000,
            config: AgentConfig::default(),
            http_token: Some("secret".into()),
            cron_store: None,
            kanban_store: Some(store),
        }));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/tasks/missing-task/retry")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

impl GatewayState {
    pub fn new(port: u16, config: AgentConfig) -> Self {
        Self {
            port,
            config,
            http_token: None,
            cron_store: None,
            kanban_store: None,
        }
    }

    pub fn with_stores(
        port: u16,
        config: AgentConfig,
        cron_store: CronStore,
        kanban_store: KanbanStore,
    ) -> Self {
        Self {
            port,
            config,
            http_token: None,
            cron_store: Some(cron_store),
            kanban_store: Some(kanban_store),
        }
    }

    pub fn with_http_token(mut self, http_token: Option<String>) -> Self {
        self.http_token = http_token;
        self
    }
}

pub fn cron_dashboard_html() -> String {
    r#"<!doctype html>
<html>
<head><title>Cron Jobs</title></head>
<body>
<h1>Cron Jobs</h1>
<pre id="jobs"></pre>
<script>
async function loadCronJobs() {
  const response = await fetch('/api/cron/jobs');
  document.getElementById('jobs').textContent = JSON.stringify(await response.json(), null, 2);
}
loadCronJobs();
setInterval(loadCronJobs, 5000);
</script>
</body>
</html>"#
        .to_string()
}

pub fn tasks_dashboard_html() -> String {
    r#"<!doctype html>
<html>
<head><title>Kanban Tasks</title></head>
<body>
<h1>Kanban Tasks</h1>
<nav>queued running failed</nav>
<pre id="tasks"></pre>
<script>
async function loadTasks() {
  const response = await fetch('/api/tasks');
  document.getElementById('tasks').textContent = JSON.stringify(await response.json(), null, 2);
}
loadTasks();
setInterval(loadTasks, 5000);
</script>
</body>
</html>"#
        .to_string()
}

pub fn build_router(state: SharedState) -> Router {
    let protected_routes = Router::new()
        .route("/api/cron/jobs", get(list_cron_jobs).post(create_cron_job))
        .route(
            "/api/cron/jobs/{id}",
            get(get_cron_job).delete(delete_cron_job),
        )
        .route("/api/cron/jobs/{id}/pause", post(pause_cron_job))
        .route("/api/cron/jobs/{id}/resume", post(resume_cron_job))
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/{id}", get(get_task))
        .route("/api/tasks/{id}/runs", get(list_task_runs))
        .route("/api/tasks/{id}/events", get(list_task_events))
        .route("/api/tasks/{id}/cancel", post(cancel_task))
        .route("/api/tasks/{id}/retry", post(retry_task))
        .route("/run", post(handle_run))
        .route("/run/stream", post(handle_run_stream))
        .route("/webhook/telegram", post(handle_telegram))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            authorize_request,
        ));

    Router::new()
        .route("/health", get(health_check))
        .route("/cron", get(cron_dashboard))
        .route("/tasks", get(tasks_dashboard))
        .merge(protected_routes)
        .with_state(state)
}

pub async fn handle_run_stream(
    State(state): State<SharedState>,
    Json(payload): Json<RunRequest>,
) -> Result<Sse<ReceiverStream<Result<Event, Infallible>>>, (StatusCode, Json<ErrorResponse>)> {
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        let session_id = payload.session_id.unwrap_or_else(|| "http-session".into());
        let agent = build_agent(&state.config, None).await;
        let mut input = AgentInput {
            user_message: payload.message,
            session_id,
            system_prompt: "You are a helpful assistant capable of using tools to solve problems in a terminal environment.".into(),
            history: Vec::new(),
            workspace_dir: std::env::current_dir().unwrap_or_default(),
        };
        let tx_events = tx.clone();
        let mut sink = agent_core::callback_event_sink(move |event| {
            let _ = tx_events.try_send(Ok(sse_event(event)));
        });

        if let Err(error) = agent.run_with_events(&mut input, &mut sink).await {
            let _ = tx
                .send(Ok(sse_event(AgentEvent::Final {
                    content: format!("[Error] {}", error),
                })))
                .await;
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx)))
}

pub async fn run(
    port: u16,
    host: &str,
    config_path: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    println!("🚀 Starting Agent Gateway on http://{}:{}", host, port);

    let http_token = std::env::var("AGENT_HTTP_TOKEN")
        .ok()
        .filter(|token| !token.trim().is_empty());
    validate_http_token_for_host(host, http_token.as_deref())?;

    let config = load_config(config_path);
    let automation_db_path = default_automation_db_path();
    let cron_store = CronStore::new(&automation_db_path).await?;
    cron_store
        .import_legacy_jobs_file(&default_legacy_jobs_file())
        .await?;
    let kanban_store = KanbanStore::new(&automation_db_path).await?;

    let scheduler = CronScheduler::new(cron_store.clone(), kanban_store.clone());
    tokio::spawn(async move {
        scheduler.tick_loop(10).await;
        tracing::warn!("cron scheduler tick loop returned unexpectedly");
    });

    let worker = WorkerRuntime::new(
        "serve-worker".into(),
        kanban_store.clone(),
        Arc::new(ServeTaskExecutor {
            config: config.clone(),
            kanban_store: kanban_store.clone(),
        }),
    );
    tokio::spawn(async move {
        worker.run_loop(5).await;
        tracing::warn!("serve worker run loop returned unexpectedly");
    });

    let state = Arc::new(
        GatewayState::with_stores(port, config, cron_store, kanban_store)
            .with_http_token(http_token),
    );
    let app = build_router(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("Listening at {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn handle_run(
    State(state): State<SharedState>,
    Json(payload): Json<RunRequest>,
) -> ApiResult<RunResponse> {
    let session_id = payload.session_id.unwrap_or_else(|| "http-session".into());
    let agent = build_agent(&state.config, None).await;
    let mut input = AgentInput {
        user_message: payload.message,
        session_id: session_id.clone(),
        system_prompt: "You are a helpful assistant capable of using tools to solve problems in a terminal environment.".into(),
        history: Vec::new(),
        workspace_dir: std::env::current_dir().unwrap_or_default(),
    };
    let mut events = AgentEventCollector::new();

    let run_error = match agent.run_with_events(&mut input, &mut events).await {
        Ok(AgentOutput::Cancelled) => Some("Cancelled".to_string()),
        Err(error) => Some(error.to_string()),
        Ok(_) => None,
    };
    let collected_events = events.into_events();
    let output = output_from_events(&collected_events, run_error.as_deref());

    Ok(Json(RunResponse {
        session_id,
        output,
        events: collected_events
            .into_iter()
            .map(SerializableAgentEvent::from)
            .collect(),
    }))
}

async fn health_check(State(state): State<SharedState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "ok".to_string(),
        port: state.port,
    })
}

async fn cron_dashboard() -> Html<String> {
    Html(cron_dashboard_html())
}

async fn tasks_dashboard() -> Html<String> {
    Html(tasks_dashboard_html())
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn validate_http_token_for_host(host: &str, token: Option<&str>) -> anyhow::Result<()> {
    if is_loopback_host(host) || token.is_some_and(|token| !token.trim().is_empty()) {
        return Ok(());
    }

    anyhow::bail!(
        "AGENT_HTTP_TOKEN is required when binding Agent Gateway to non-loopback host {}",
        host
    )
}

fn unauthorized() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "missing or invalid bearer token".to_string(),
        }),
    )
}

fn authorize_bearer_header(
    authorization: Option<&str>,
    expected_token: Option<&str>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let Some(expected_token) = expected_token.filter(|token| !token.trim().is_empty()) else {
        return Ok(());
    };

    let expected_header = format!("Bearer {}", expected_token);
    if authorization == Some(expected_header.as_str()) {
        return Ok(());
    }

    tracing::warn!("rejecting unauthorized HTTP request");
    Err(unauthorized())
}

async fn authorize_request(
    State(state): State<SharedState>,
    headers: HeaderMap,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    authorize_headers(&headers, &state)?;
    Ok(next.run(request).await)
}

fn authorize_headers(
    headers: &HeaderMap,
    state: &GatewayState,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let authorization = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    authorize_bearer_header(authorization, state.http_token.as_deref())
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
}

fn not_found(resource: &str, id: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("{resource} not found: {id}"),
        }),
    )
}

fn conflict(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

fn forbidden_shell_command() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "shell_command payloads cannot be created over HTTP".to_string(),
        }),
    )
}

fn validate_http_cron_job(payload: &NewCronJob) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    match payload.payload_kind {
        agent_cron::PayloadKind::ShellCommand => Err(forbidden_shell_command()),
        agent_cron::PayloadKind::AgentPrompt => Ok(()),
    }
}

fn validate_http_task(payload: &NewTask) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    match payload.payload_kind {
        agent_kanban::PayloadKind::ShellCommand => Err(forbidden_shell_command()),
        agent_kanban::PayloadKind::AgentPrompt => Ok(()),
    }
}

async fn list_cron_jobs(State(state): State<SharedState>) -> ApiResult<Vec<CronJobResponse>> {
    let Some(store) = &state.cron_store else {
        return Ok(Json(Vec::new()));
    };

    let jobs = store
        .list_jobs()
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(CronJobResponse::from)
        .collect();
    Ok(Json(jobs))
}

async fn create_cron_job(
    State(state): State<SharedState>,
    Json(payload): Json<NewCronJob>,
) -> ApiResult<Option<CronJobResponse>> {
    validate_http_cron_job(&payload)?;

    let Some(store) = &state.cron_store else {
        return Ok(Json(None));
    };

    let job = store
        .create_job(payload)
        .await
        .map_err(internal_error)
        .map(CronJobResponse::from)?;
    Ok(Json(Some(job)))
}

async fn get_cron_job(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<CronJobResponse> {
    let Some(store) = &state.cron_store else {
        return Err(not_found("cron job", &id));
    };

    let Some(job) = store.load_job(&id).await.map_err(internal_error)? else {
        return Err(not_found("cron job", &id));
    };

    Ok(Json(CronJobResponse::from(job)))
}

async fn pause_cron_job(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<CronJobResponse> {
    set_cron_enabled(state, id, false).await
}

async fn resume_cron_job(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<CronJobResponse> {
    set_cron_enabled(state, id, true).await
}

async fn set_cron_enabled(
    state: SharedState,
    id: String,
    enabled: bool,
) -> ApiResult<CronJobResponse> {
    let Some(store) = &state.cron_store else {
        return Err(not_found("cron job", &id));
    };
    if store.load_job(&id).await.map_err(internal_error)?.is_none() {
        return Err(not_found("cron job", &id));
    }

    store
        .set_enabled(&id, enabled)
        .await
        .map_err(internal_error)?;
    let job = store
        .load_job(&id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("cron job", &id))?;
    Ok(Json(CronJobResponse::from(job)))
}

async fn delete_cron_job(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<CronJobResponse> {
    let Some(store) = &state.cron_store else {
        return Err(not_found("cron job", &id));
    };
    let Some(job) = store.load_job(&id).await.map_err(internal_error)? else {
        return Err(not_found("cron job", &id));
    };

    if !store.delete_job(&id).await.map_err(internal_error)? {
        return Err(not_found("cron job", &id));
    }

    Ok(Json(CronJobResponse::from(job)))
}

async fn list_tasks(
    State(state): State<SharedState>,
    Query(query): Query<TaskListQuery>,
) -> ApiResult<Vec<TaskResponse>> {
    let Some(store) = &state.kanban_store else {
        return Ok(Json(Vec::new()));
    };

    let tasks = store
        .list_tasks(query.status)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(TaskResponse::from)
        .collect();
    Ok(Json(tasks))
}

async fn create_task(
    State(state): State<SharedState>,
    Json(payload): Json<NewTask>,
) -> ApiResult<Option<TaskResponse>> {
    validate_http_task(&payload)?;

    let Some(store) = &state.kanban_store else {
        return Ok(Json(None));
    };

    let task = store
        .create_task(payload)
        .await
        .map_err(internal_error)
        .map(TaskResponse::from)?;
    Ok(Json(Some(task)))
}

async fn get_task(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<TaskResponse> {
    let Some(store) = &state.kanban_store else {
        return Err(not_found("task", &id));
    };

    let Some(task) = store.load_task(&id).await.map_err(internal_error)? else {
        return Err(not_found("task", &id));
    };

    Ok(Json(TaskResponse::from(task)))
}

async fn list_task_runs(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Vec<TaskRunResponse>> {
    let Some(store) = &state.kanban_store else {
        return Err(not_found("task", &id));
    };
    if store
        .load_task(&id)
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(not_found("task", &id));
    }

    let runs = store
        .list_task_runs(&id)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(TaskRunResponse::from)
        .collect();
    Ok(Json(runs))
}

async fn list_task_events(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Vec<TaskEventResponse>> {
    let Some(store) = &state.kanban_store else {
        return Err(not_found("task", &id));
    };
    if store
        .load_task(&id)
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(not_found("task", &id));
    }

    let events = store
        .list_task_events(&id)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(TaskEventResponse::from)
        .collect();
    Ok(Json(events))
}

async fn cancel_task(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<TaskResponse> {
    let Some(store) = &state.kanban_store else {
        return Err(not_found("task", &id));
    };
    if !store.cancel_task(&id).await.map_err(internal_error)? {
        return Err(not_found("task", &id));
    }

    let task = store
        .load_task(&id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("task", &id))?;
    Ok(Json(TaskResponse::from(task)))
}

async fn retry_task(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<TaskResponse> {
    let Some(store) = &state.kanban_store else {
        return Err(not_found("task", &id));
    };
    if store
        .load_task(&id)
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(not_found("task", &id));
    }

    if !store.retry_task(&id).await.map_err(internal_error)? {
        return Err(conflict(format!("task cannot be retried: {id}")));
    }

    let task = store
        .load_task(&id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("task", &id))?;
    Ok(Json(TaskResponse::from(task)))
}

async fn handle_telegram(
    State(state): State<SharedState>,
    Json(payload): Json<TelegramWebhook>,
) -> ApiResult<StatusResponse> {
    if let Some(msg) = payload.message {
        println!(
            "Received Telegram message from chat ID {}: {:?}",
            msg.chat.id, msg.text
        );
        // TODO: Integrate Agent Loop here to process msg.text and send response back via Telegram API
    } else {
        println!("Received empty webhook payload.");
    }

    Ok(Json(StatusResponse {
        status: "received".to_string(),
        port: state.port,
    }))
}
