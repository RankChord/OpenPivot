use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Claimed,
    Running,
    Succeeded,
    Failed,
    Blocked,
    Cancelled,
    TimedOut,
    Lost,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Claimed => "claimed",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::Lost => "lost",
        }
    }
}

impl TryFrom<&str> for TaskStatus {
    type Error = KanbanTypeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(Self::Queued),
            "claimed" => Ok(Self::Claimed),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "blocked" => Ok(Self::Blocked),
            "cancelled" => Ok(Self::Cancelled),
            "timed_out" => Ok(Self::TimedOut),
            "lost" => Ok(Self::Lost),
            other => Err(KanbanTypeError::InvalidStatus(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadKind {
    AgentPrompt,
    ShellCommand,
}

impl PayloadKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AgentPrompt => "agent_prompt",
            Self::ShellCommand => "shell_command",
        }
    }
}

impl TryFrom<&str> for PayloadKind {
    type Error = KanbanTypeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "agent_prompt" => Ok(Self::AgentPrompt),
            "shell_command" => Ok(Self::ShellCommand),
            other => Err(KanbanTypeError::InvalidPayloadKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewTask {
    pub source: String,
    pub source_id: Option<String>,
    pub title: String,
    pub body: String,
    pub payload_kind: PayloadKind,
    pub payload_json: serde_json::Value,
    pub priority: i64,
    pub max_retries: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub source: String,
    pub source_id: Option<String>,
    pub title: String,
    pub body: String,
    pub payload_kind: PayloadKind,
    pub payload_json: serde_json::Value,
    pub status: TaskStatus,
    pub priority: i64,
    pub claim_lock: Option<String>,
    pub claim_expires_at: Option<DateTime<Utc>>,
    pub assigned_worker: Option<String>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub retry_count: i64,
    pub max_retries: i64,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRunRecord {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskEventRecord {
    pub id: String,
    pub task_id: String,
    pub run_id: Option<String>,
    pub event_type: String,
    pub payload_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub sequence: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimedTask {
    pub task: TaskRecord,
    pub run: TaskRunRecord,
    pub claim_lock: String,
}

#[derive(Debug, thiserror::Error)]
pub enum KanbanTypeError {
    #[error("invalid task status: {0}")]
    InvalidStatus(String),
    #[error("invalid payload kind: {0}")]
    InvalidPayloadKind(String),
}
