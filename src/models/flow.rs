use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug)]
pub enum FlowRunStatus {
    Running,
    WaitingAction,
    Completed,
    Canceled,
    Failed,
}

impl FlowRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlowRunStatus::Running => "running",
            FlowRunStatus::WaitingAction => "waiting_action",
            FlowRunStatus::Completed => "completed",
            FlowRunStatus::Canceled => "canceled",
            FlowRunStatus::Failed => "failed",
        }
    }
}

impl TryFrom<&str> for FlowRunStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "running" => Ok(FlowRunStatus::Running),
            "waiting_action" => Ok(FlowRunStatus::WaitingAction),
            "completed" => Ok(FlowRunStatus::Completed),
            "canceled" => Ok(FlowRunStatus::Canceled),
            "failed" => Ok(FlowRunStatus::Failed),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum FlowTaskStatus {
    Pending,
    Completed,
    Canceled,
}

impl FlowTaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlowTaskStatus::Pending => "pending",
            FlowTaskStatus::Completed => "completed",
            FlowTaskStatus::Canceled => "canceled",
        }
    }
}

impl TryFrom<&str> for FlowTaskStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(FlowTaskStatus::Pending),
            "completed" => Ok(FlowTaskStatus::Completed),
            "canceled" => Ok(FlowTaskStatus::Canceled),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum FlowEventType {
    FlowStarted,
    TaskCreated,
    TaskCompleted,
    SpaceNotified,
    FlowCompleted,
    FlowCanceled,
    FlowFailed,
}

impl FlowEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlowEventType::FlowStarted => "flow_started",
            FlowEventType::TaskCreated => "task_created",
            FlowEventType::TaskCompleted => "task_completed",
            FlowEventType::SpaceNotified => "space_notified",
            FlowEventType::FlowCompleted => "flow_completed",
            FlowEventType::FlowCanceled => "flow_canceled",
            FlowEventType::FlowFailed => "flow_failed",
        }
    }
}

impl TryFrom<&str> for FlowEventType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "flow_started" => Ok(FlowEventType::FlowStarted),
            "task_created" => Ok(FlowEventType::TaskCreated),
            "task_completed" => Ok(FlowEventType::TaskCompleted),
            "space_notified" => Ok(FlowEventType::SpaceNotified),
            "flow_completed" => Ok(FlowEventType::FlowCompleted),
            "flow_canceled" => Ok(FlowEventType::FlowCanceled),
            "flow_failed" => Ok(FlowEventType::FlowFailed),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct Flow {
    pub id: i64,
    pub space_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_by: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct FlowRun {
    pub id: i64,
    pub flow_id: i64,
    pub space_id: i64,
    pub status: FlowRunStatus,
    pub started_by: i64,
    pub current_task_id: Option<i64>,
    pub started_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
}

#[derive(Debug)]
pub struct FlowTask {
    pub id: i64,
    pub flow_run_id: i64,
    pub space_id: i64,
    pub assignee_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: FlowTaskStatus,
    pub result: Option<String>,
    pub created_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub completed_by: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FlowEvent {
    pub id: i64,
    pub flow_run_id: i64,
    pub space_id: i64,
    pub event_type: String,
    pub actor_id: Option<i64>,
    pub payload: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateFlowRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FlowResponse {
    pub id: i64,
    pub space_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_by: i64,
}

#[derive(Debug, Deserialize)]
pub struct StartFlowRunRequest {
    pub assignee_id: i64,
    pub task_title: String,
    pub task_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteFlowTaskRequest {
    pub result: String,
}

impl Flow {
    pub fn into_response(self) -> FlowResponse {
        FlowResponse {
            id: self.id,
            space_id: self.space_id,
            name: self.name,
            description: self.description,
            created_by: self.created_by,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StartFlowRunResponse {
    pub run_id: i64,
    pub task_id: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CompleteFlowTaskResponse {
    pub task_id: i64,
    pub run_id: i64,
    pub status: String,
}