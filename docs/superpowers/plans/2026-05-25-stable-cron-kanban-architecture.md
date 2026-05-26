# Stable Cron Kanban Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a durable cron-to-kanban automation layer where schedules persist, due cron jobs create recoverable tasks, workers claim and execute tasks with heartbeat, and HTTP/dashboard routes expose status.

**Architecture:** `agent-cron` owns cron schedules and trigger creation. A new `agent-kanban` crate owns durable task storage, claim locks, runs, events, heartbeat, and recovery. `agent-cli::cmd::serve` composes stores, starts an embedded scheduler/worker, and exposes API plus simple HTML dashboard.

**Tech Stack:** Rust 2024, Tokio, SQLx SQLite, Axum, Serde, Chrono, UUID, existing `agent-core` event stream and `agent-cli` runtime builder.

---

## File Structure

- Create `crates/agent-kanban/Cargo.toml`: dependencies for the durable task crate.
- Create `crates/agent-kanban/src/lib.rs`: public module exports.
- Create `crates/agent-kanban/src/types.rs`: task, run, event, status, payload, and claim types.
- Create `crates/agent-kanban/src/store.rs`: SQLite schema, CRUD, atomic claim, heartbeat, recovery, run/event persistence.
- Create `crates/agent-kanban/src/worker.rs`: embedded worker loop and execution traits.
- Create `crates/agent-kanban/tests/store_test.rs`: store and state transition tests.
- Create `crates/agent-kanban/tests/worker_test.rs`: worker claim/heartbeat/execution tests.
- Modify `Cargo.toml`: add `crates/agent-kanban` workspace member and workspace SQLx dependency.
- Modify `crates/agent-cron/Cargo.toml`: add `sqlx`, `uuid`, and `agent-kanban` where needed.
- Modify `crates/agent-cron/src/job.rs`: add stable payload and missed-run policy fields, improve schedule next-time behavior for supported intervals.
- Create `crates/agent-cron/src/store.rs`: durable cron job and trigger store backed by SQLite.
- Modify `crates/agent-cron/src/scheduler.rs`: make scheduler create Kanban tasks through stores.
- Modify `crates/agent-cron/src/lib.rs`: export `store`.
- Create `crates/agent-cron/tests/store_test.rs`: cron store persistence tests.
- Create `crates/agent-cron/tests/scheduler_test.rs`: due-job idempotency tests.
- Modify `crates/agent-cli/Cargo.toml`: depend on `agent-cron` and `agent-kanban` if not already present.
- Modify `crates/agent-cli/src/cmd/serve.rs`: add shared stores to `GatewayState`, add cron/task API routes, start scheduler/worker background tasks, serve dashboards.
- Create `crates/agent-cli/tests/cron_api_test.rs`: API serialization and route tests.
- Create `crates/agent-cli/tests/dashboard_test.rs`: HTML route tests.
- Modify `crates/agent-cli/src/cmd/cron.rs`: replace JSON file operations with `CronStore`.
- Modify `tools/cron-job/src/lib.rs`: create cron jobs through `CronStore` instead of writing JSON directly.
- Modify `crates/agent-cron-runner/src/main.rs`: use `CronStore`/`CronScheduler`, or reduce to a compatibility wrapper around the new scheduler.
- Modify `README.md`: document durable cron/kanban, API, dashboard, and recovery behavior.

## Task 1: Add `agent-kanban` Crate Skeleton And Workspace Dependency

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/agent-kanban/Cargo.toml`
- Create: `crates/agent-kanban/src/lib.rs`
- Create: `crates/agent-kanban/src/types.rs`

- [ ] **Step 1: Write the failing compile target**

Run: `cargo check -p agent-kanban`

Expected: FAIL with `package ID specification 'agent-kanban' did not match any packages`.

- [ ] **Step 2: Add the workspace member and shared SQLx dependency**

Modify root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/agent-core",
    "crates/agent-tools",
    "crates/agent-config",
    "crates/agent-llm",
    "crates/agent-cli",
    "crates/agent-plugin-sdk",
    "crates/agent-gateway",
    "crates/agent-worker",
    "crates/agent-memory",
    "crates/agent-skills",
    "crates/agent-permission",
    "crates/agent-cron",
    "crates/agent-kanban",
    "crates/agent-delegate",
    "crates/agent-sessions",
    "crates/agent-platforms",
    "crates/agent-cron-runner",
    "tools/read-file",
    "tools/write-file",
    "tools/shell",
    "tools/todo-tool",
    "tools/web-search",
    "tools/cron-job",
    "tools/browser",
    "tools/delegate",
    "crates/agent-tui",
]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
anyhow = "1"
async-trait = "0.1"
reqwest = { version = "0.12", features = ["json", "stream"] }
futures = "0.3"
tokio-stream = "0.1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
clap = { version = "4", features = ["derive"] }
toml = "0.8"
dirs = "5"
walkdir = "2"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
```

- [ ] **Step 3: Create `crates/agent-kanban/Cargo.toml`**

```toml
[package]
name = "agent-kanban"
version.workspace = true
edition.workspace = true

[dependencies]
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
chrono.workspace = true
uuid.workspace = true
sqlx.workspace = true
async-trait.workspace = true

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4: Create `crates/agent-kanban/src/lib.rs`**

```rust
pub mod store;
pub mod types;
pub mod worker;

pub use store::*;
pub use types::*;
pub use worker::*;
```

- [ ] **Step 5: Create `crates/agent-kanban/src/types.rs`**

```rust
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEventRecord {
    pub id: String,
    pub task_id: String,
    pub run_id: Option<String>,
    pub event_type: String,
    pub payload_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
```

- [ ] **Step 6: Run compile check**

Run: `cargo check -p agent-kanban`

Expected: FAIL because `store` and `worker` modules are declared but not created. This confirms the workspace sees the new crate.

## Task 2: Implement `KanbanStore` Schema And Basic Task Persistence

**Files:**
- Create: `crates/agent-kanban/src/store.rs`
- Create: `crates/agent-kanban/src/worker.rs`
- Create: `crates/agent-kanban/tests/store_test.rs`

- [ ] **Step 1: Write failing tests for task create/list/load**

Create `crates/agent-kanban/tests/store_test.rs`:

```rust
use agent_kanban::{KanbanStore, NewTask, PayloadKind, TaskStatus};
use serde_json::json;

#[tokio::test]
async fn creates_lists_and_loads_task() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("kanban.db");
    let store = KanbanStore::new(&db_path).await.unwrap();

    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Demo".into(),
            body: "Run demo".into(),
            payload_kind: PayloadKind::AgentPrompt,
            payload_json: json!({"prompt": "say hello"}),
            priority: 5,
            max_retries: 1,
        })
        .await
        .unwrap();

    assert_eq!(task.source, "manual");
    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(task.payload_kind, PayloadKind::AgentPrompt);
    assert_eq!(task.payload_json["prompt"], "say hello");

    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.id, task.id);

    let tasks = store.list_tasks(None).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Demo");
}

#[tokio::test]
async fn filters_tasks_by_status() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("kanban.db");
    let store = KanbanStore::new(&db_path).await.unwrap();

    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Queued".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    let queued = store.list_tasks(Some(TaskStatus::Queued)).await.unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].status, TaskStatus::Queued);

    let running = store.list_tasks(Some(TaskStatus::Running)).await.unwrap();
    assert!(running.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-kanban --test store_test creates_lists_and_loads_task -- --nocapture`

Expected: FAIL with unresolved `KanbanStore`.

- [ ] **Step 3: Implement `crates/agent-kanban/src/worker.rs` as a minimal module**

```rust
pub struct WorkerRuntime;
```

- [ ] **Step 4: Implement `crates/agent-kanban/src/store.rs`**

```rust
use crate::{NewTask, PayloadKind, TaskEventRecord, TaskRecord, TaskRunRecord, TaskStatus};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct KanbanStore {
    pool: Arc<SqlitePool>,
}

impl KanbanStore {
    pub async fn new(db_path: &Path) -> Result<Self, sqlx::Error> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let db_url = format!("sqlite://{}", db_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(options).await?;
        init_schema(&pool).await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub async fn create_task(&self, task: NewTask) -> Result<TaskRecord, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let payload_json = serde_json::to_string(&task.payload_json)
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

        sqlx::query(
            "INSERT INTO tasks (
                id, source, source_id, title, body, payload_kind, payload_json, status,
                priority, retry_count, max_retries, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&task.source)
        .bind(&task.source_id)
        .bind(&task.title)
        .bind(&task.body)
        .bind(task.payload_kind.as_str())
        .bind(payload_json)
        .bind(TaskStatus::Queued.as_str())
        .bind(task.priority)
        .bind(task.max_retries)
        .bind(now.timestamp())
        .bind(now.timestamp())
        .execute(&*self.pool)
        .await?;

        self.append_task_event(&id, None, "task_created", serde_json::json!({"title": task.title}))
            .await?;

        self.load_task(&id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn load_task(&self, id: &str) -> Result<Option<TaskRecord>, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await?;

        row.map(row_to_task).transpose()
    }

    pub async fn list_tasks(
        &self,
        status: Option<TaskStatus>,
    ) -> Result<Vec<TaskRecord>, sqlx::Error> {
        let rows = if let Some(status) = status {
            sqlx::query("SELECT * FROM tasks WHERE status = ? ORDER BY priority DESC, created_at ASC")
                .bind(status.as_str())
                .fetch_all(&*self.pool)
                .await?
        } else {
            sqlx::query("SELECT * FROM tasks ORDER BY priority DESC, created_at ASC")
                .fetch_all(&*self.pool)
                .await?
        };

        rows.into_iter().map(row_to_task).collect()
    }

    pub async fn append_task_event(
        &self,
        task_id: &str,
        run_id: Option<&str>,
        event_type: &str,
        payload_json: serde_json::Value,
    ) -> Result<TaskEventRecord, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let payload = serde_json::to_string(&payload_json)
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

        sqlx::query(
            "INSERT INTO task_events (id, task_id, run_id, event_type, payload_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(task_id)
        .bind(run_id)
        .bind(event_type)
        .bind(payload)
        .bind(now.timestamp())
        .execute(&*self.pool)
        .await?;

        Ok(TaskEventRecord {
            id,
            task_id: task_id.to_string(),
            run_id: run_id.map(str::to_string),
            event_type: event_type.to_string(),
            payload_json,
            created_at: now,
        })
    }
}

async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            source TEXT NOT NULL,
            source_id TEXT,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            payload_kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            status TEXT NOT NULL,
            priority INTEGER NOT NULL,
            claim_lock TEXT,
            claim_expires_at INTEGER,
            assigned_worker TEXT,
            last_heartbeat_at INTEGER,
            retry_count INTEGER NOT NULL,
            max_retries INTEGER NOT NULL,
            last_error TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            started_at INTEGER,
            completed_at INTEGER
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS task_runs (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            worker_id TEXT NOT NULL,
            status TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            finished_at INTEGER,
            duration_ms INTEGER,
            session_id TEXT,
            output_summary TEXT,
            error TEXT,
            log_path TEXT,
            FOREIGN KEY(task_id) REFERENCES tasks(id)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS task_events (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            run_id TEXT,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(task_id) REFERENCES tasks(id)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status, priority, created_at)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_task_events_task ON task_events(task_id, created_at)")
        .execute(pool)
        .await?;

    Ok(())
}

fn row_to_task(row: sqlx::sqlite::SqliteRow) -> Result<TaskRecord, sqlx::Error> {
    let payload_kind = PayloadKind::try_from(row.get::<String, _>("payload_kind").as_str())
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let status = TaskStatus::try_from(row.get::<String, _>("status").as_str())
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let payload_json = serde_json::from_str(&row.get::<String, _>("payload_json"))
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

    Ok(TaskRecord {
        id: row.get("id"),
        source: row.get("source"),
        source_id: row.get("source_id"),
        title: row.get("title"),
        body: row.get("body"),
        payload_kind,
        payload_json,
        status,
        priority: row.get("priority"),
        claim_lock: row.get("claim_lock"),
        claim_expires_at: optional_ts(row.get("claim_expires_at")),
        assigned_worker: row.get("assigned_worker"),
        last_heartbeat_at: optional_ts(row.get("last_heartbeat_at")),
        retry_count: row.get("retry_count"),
        max_retries: row.get("max_retries"),
        last_error: row.get("last_error"),
        created_at: ts(row.get("created_at")),
        updated_at: ts(row.get("updated_at")),
        started_at: optional_ts(row.get("started_at")),
        completed_at: optional_ts(row.get("completed_at")),
    })
}

fn ts(value: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(value, 0).single().unwrap()
}

fn optional_ts(value: Option<i64>) -> Option<DateTime<Utc>> {
    value.map(ts)
}
```

- [ ] **Step 5: Run tests to verify persistence passes**

Run: `cargo test -p agent-kanban --test store_test -- --nocapture`

Expected: PASS for both tests.

## Task 3: Add Atomic Claim, Heartbeat, Completion, And Recovery

**Files:**
- Modify: `crates/agent-kanban/src/store.rs`
- Modify: `crates/agent-kanban/tests/store_test.rs`

- [ ] **Step 1: Add failing tests for claim and heartbeat**

Append to `crates/agent-kanban/tests/store_test.rs`:

```rust
#[tokio::test]
async fn claim_is_atomic_and_creates_run() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db")).await.unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Claim me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    let first = store.claim_next_task("worker-a", 60).await.unwrap().unwrap();
    assert_eq!(first.task.id, task.id);
    assert_eq!(first.task.status, TaskStatus::Claimed);
    assert_eq!(first.run.task_id, task.id);

    let second = store.claim_next_task("worker-b", 60).await.unwrap();
    assert!(second.is_none());
}

#[tokio::test]
async fn heartbeat_and_completion_update_task_and_run() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db")).await.unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Finish me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    let claimed = store.claim_next_task("worker-a", 60).await.unwrap().unwrap();
    store.mark_running(&claimed.task.id, &claimed.run.id, &claimed.claim_lock).await.unwrap();
    store.heartbeat(&claimed.task.id, &claimed.claim_lock, 60).await.unwrap();
    store
        .complete_run(
            &claimed.task.id,
            &claimed.run.id,
            &claimed.claim_lock,
            TaskStatus::Succeeded,
            Some("ok"),
            None,
        )
        .await
        .unwrap();

    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Succeeded);
    assert!(loaded.completed_at.is_some());
    assert_eq!(loaded.claim_lock, None);
}

#[tokio::test]
async fn expired_claim_can_be_recovered_to_lost() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db")).await.unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Lose me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    let claimed = store.claim_next_task("worker-a", -1).await.unwrap().unwrap();
    let recovered = store.recover_expired_claims().await.unwrap();
    assert_eq!(recovered, 1);

    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Lost);
    assert!(loaded.last_error.unwrap().contains("expired claim"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-kanban --test store_test claim_is_atomic_and_creates_run -- --nocapture`

Expected: FAIL with missing `claim_next_task`.

- [ ] **Step 3: Add methods to `KanbanStore`**

Append these methods inside `impl KanbanStore` in `crates/agent-kanban/src/store.rs`:

```rust
    pub async fn claim_next_task(
        &self,
        worker_id: &str,
        claim_ttl_secs: i64,
    ) -> Result<Option<crate::ClaimedTask>, sqlx::Error> {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(claim_ttl_secs);
        let candidate = sqlx::query(
            "SELECT id FROM tasks
             WHERE status = 'queued'
             ORDER BY priority DESC, created_at ASC
             LIMIT 1",
        )
        .fetch_optional(&*self.pool)
        .await?;

        let Some(candidate) = candidate else {
            return Ok(None);
        };

        let task_id: String = candidate.get("id");
        let claim_lock = Uuid::new_v4().to_string();
        let updated = sqlx::query(
            "UPDATE tasks
             SET status = 'claimed', claim_lock = ?, claim_expires_at = ?, assigned_worker = ?,
                 updated_at = ?, started_at = COALESCE(started_at, ?)
             WHERE id = ? AND status = 'queued'",
        )
        .bind(&claim_lock)
        .bind(expires_at.timestamp())
        .bind(worker_id)
        .bind(now.timestamp())
        .bind(now.timestamp())
        .bind(&task_id)
        .execute(&*self.pool)
        .await?;

        if updated.rows_affected() == 0 {
            return Ok(None);
        }

        let run_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO task_runs (id, task_id, worker_id, status, started_at)
             VALUES (?, ?, ?, 'claimed', ?)",
        )
        .bind(&run_id)
        .bind(&task_id)
        .bind(worker_id)
        .bind(now.timestamp())
        .execute(&*self.pool)
        .await?;

        self.append_task_event(
            &task_id,
            Some(&run_id),
            "task_claimed",
            serde_json::json!({"worker_id": worker_id}),
        )
        .await?;

        let task = self.load_task(&task_id).await?.ok_or(sqlx::Error::RowNotFound)?;
        let run = TaskRunRecord {
            id: run_id,
            task_id: task_id.clone(),
            worker_id: worker_id.to_string(),
            status: TaskStatus::Claimed,
            started_at: now,
            finished_at: None,
            duration_ms: None,
            session_id: None,
            output_summary: None,
            error: None,
            log_path: None,
        };

        Ok(Some(crate::ClaimedTask {
            task,
            run,
            claim_lock,
        }))
    }

    pub async fn mark_running(
        &self,
        task_id: &str,
        run_id: &str,
        claim_lock: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE tasks SET status = 'running', last_heartbeat_at = ?, updated_at = ?
             WHERE id = ? AND claim_lock = ?",
        )
        .bind(now.timestamp())
        .bind(now.timestamp())
        .bind(task_id)
        .bind(claim_lock)
        .execute(&*self.pool)
        .await?;

        sqlx::query("UPDATE task_runs SET status = 'running' WHERE id = ?")
            .bind(run_id)
            .execute(&*self.pool)
            .await?;

        self.append_task_event(task_id, Some(run_id), "task_running", serde_json::json!({}))
            .await?;
        Ok(())
    }

    pub async fn heartbeat(
        &self,
        task_id: &str,
        claim_lock: &str,
        claim_ttl_secs: i64,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(claim_ttl_secs);
        sqlx::query(
            "UPDATE tasks SET last_heartbeat_at = ?, claim_expires_at = ?, updated_at = ?
             WHERE id = ? AND claim_lock = ? AND status IN ('claimed', 'running')",
        )
        .bind(now.timestamp())
        .bind(expires_at.timestamp())
        .bind(now.timestamp())
        .bind(task_id)
        .bind(claim_lock)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn complete_run(
        &self,
        task_id: &str,
        run_id: &str,
        claim_lock: &str,
        status: TaskStatus,
        output_summary: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE tasks
             SET status = ?, claim_lock = NULL, claim_expires_at = NULL, assigned_worker = NULL,
                 completed_at = ?, updated_at = ?, last_error = ?
             WHERE id = ? AND claim_lock = ?",
        )
        .bind(status.as_str())
        .bind(now.timestamp())
        .bind(now.timestamp())
        .bind(error)
        .bind(task_id)
        .bind(claim_lock)
        .execute(&*self.pool)
        .await?;

        sqlx::query(
            "UPDATE task_runs
             SET status = ?, finished_at = ?, output_summary = ?, error = ?
             WHERE id = ?",
        )
        .bind(status.as_str())
        .bind(now.timestamp())
        .bind(output_summary)
        .bind(error)
        .bind(run_id)
        .execute(&*self.pool)
        .await?;

        self.append_task_event(
            task_id,
            Some(run_id),
            "task_completed",
            serde_json::json!({"status": status.as_str(), "error": error}),
        )
        .await?;
        Ok(())
    }

    pub async fn recover_expired_claims(&self) -> Result<u64, sqlx::Error> {
        let now = Utc::now();
        let rows = sqlx::query(
            "SELECT id FROM tasks
             WHERE status IN ('claimed', 'running')
               AND claim_expires_at IS NOT NULL
               AND claim_expires_at < ?",
        )
        .bind(now.timestamp())
        .fetch_all(&*self.pool)
        .await?;

        let mut recovered = 0;
        for row in rows {
            let task_id: String = row.get("id");
            let error = "expired claim without fresh heartbeat";
            let result = sqlx::query(
                "UPDATE tasks
                 SET status = 'lost', claim_lock = NULL, claim_expires_at = NULL,
                     assigned_worker = NULL, last_error = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(error)
            .bind(now.timestamp())
            .bind(&task_id)
            .execute(&*self.pool)
            .await?;
            recovered += result.rows_affected();
            self.append_task_event(
                &task_id,
                None,
                "task_lost",
                serde_json::json!({"error": error}),
            )
            .await?;
        }

        Ok(recovered)
    }
```

- [ ] **Step 4: Run targeted tests**

Run: `cargo test -p agent-kanban --test store_test -- --nocapture`

Expected: PASS.

## Task 4: Add Durable `CronStore`

**Files:**
- Modify: `crates/agent-cron/Cargo.toml`
- Modify: `crates/agent-cron/src/job.rs`
- Create: `crates/agent-cron/src/store.rs`
- Modify: `crates/agent-cron/src/lib.rs`
- Create: `crates/agent-cron/tests/store_test.rs`

- [ ] **Step 1: Write failing cron store tests**

Create `crates/agent-cron/tests/store_test.rs`:

```rust
use agent_cron::{CronJob, CronStore, MissedRunPolicy, NewCronJob, PayloadKind, Schedule};
use serde_json::json;

#[tokio::test]
async fn creates_lists_loads_and_pauses_cron_job() {
    let dir = tempfile::tempdir().unwrap();
    let store = CronStore::new(&dir.path().join("cron.db")).await.unwrap();

    let job = store
        .create_job(NewCronJob {
            name: "Every five".into(),
            description: "demo".into(),
            schedule: Schedule::parse("5m").unwrap(),
            payload_kind: PayloadKind::AgentPrompt,
            payload_json: json!({"prompt": "say hello"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::RunOnce,
        })
        .await
        .unwrap();

    assert_eq!(job.name, "Every five");
    assert!(job.enabled);
    assert!(job.next_run_at.is_some());

    let loaded = store.load_job(&job.id).await.unwrap().unwrap();
    assert_eq!(loaded.id, job.id);

    let jobs = store.list_jobs().await.unwrap();
    assert_eq!(jobs.len(), 1);

    store.set_enabled(&job.id, false).await.unwrap();
    let paused = store.load_job(&job.id).await.unwrap().unwrap();
    assert!(!paused.enabled);
}

#[tokio::test]
async fn trigger_is_idempotent_for_same_scheduled_time() {
    let dir = tempfile::tempdir().unwrap();
    let store = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let job = store
        .create_job(NewCronJob {
            name: "Every five".into(),
            description: "demo".into(),
            schedule: Schedule::parse("5m").unwrap(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::RunOnce,
        })
        .await
        .unwrap();
    let scheduled_for = job.next_run_at.unwrap();

    let first = store
        .record_trigger(&job.id, scheduled_for, "task-1")
        .await
        .unwrap();
    let second = store
        .record_trigger(&job.id, scheduled_for, "task-2")
        .await
        .unwrap();

    assert!(first.is_some());
    assert!(second.is_none());
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p agent-cron --test store_test -- --nocapture`

Expected: FAIL with missing `CronStore` and type exports.

- [ ] **Step 3: Add dependencies to `crates/agent-cron/Cargo.toml`**

```toml
[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
chrono.workspace = true
uuid.workspace = true
sqlx.workspace = true
agent-kanban = { path = "../agent-kanban" }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4: Extend `crates/agent-cron/src/job.rs`**

Replace the file with:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schedule: Schedule,
    pub payload_kind: PayloadKind,
    pub payload_json: serde_json::Value,
    pub enabled: bool,
    pub missed_run_policy: MissedRunPolicy,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCronJob {
    pub name: String,
    pub description: String,
    pub schedule: Schedule,
    pub payload_kind: PayloadKind,
    pub payload_json: serde_json::Value,
    pub enabled: bool,
    pub missed_run_policy: MissedRunPolicy,
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
    type Error = ScheduleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "agent_prompt" => Ok(Self::AgentPrompt),
            "shell_command" => Ok(Self::ShellCommand),
            other => Err(ScheduleError::InvalidPayloadKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicy {
    Skip,
    RunOnce,
    CatchUp,
}

impl MissedRunPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Skip => "skip",
            Self::RunOnce => "run_once",
            Self::CatchUp => "catch_up",
        }
    }
}

impl TryFrom<&str> for MissedRunPolicy {
    type Error = ScheduleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "skip" => Ok(Self::Skip),
            "run_once" => Ok(Self::RunOnce),
            "catch_up" => Ok(Self::CatchUp),
            other => Err(ScheduleError::InvalidMissedRunPolicy(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Schedule {
    Duration(std::time::Duration),
    Every(String),
    Cron(String),
    Once(DateTime<Utc>),
}

impl Schedule {
    pub fn parse(input: &str) -> Result<Self, ScheduleError> {
        if let Ok(d) = parse_duration(input) {
            return Ok(Schedule::Duration(d));
        }
        if input.starts_with("every ") {
            return Ok(Schedule::Every(input.into()));
        }
        if is_cron_expression(input) {
            return Ok(Schedule::Cron(input.into()));
        }
        if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
            return Ok(Schedule::Once(dt.with_timezone(&Utc)));
        }
        Err(ScheduleError::InvalidFormat(input.into()))
    }

    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Schedule::Duration(d) => Some(after + chrono::Duration::from_std(*d).ok()?),
            Schedule::Once(dt) => {
                if *dt > after {
                    Some(*dt)
                } else {
                    None
                }
            }
            Schedule::Every(value) => parse_every_interval(value)
                .and_then(|duration| chrono::Duration::from_std(duration).ok())
                .map(|duration| after + duration)
                .or_else(|| Some(after + chrono::Duration::hours(1))),
            Schedule::Cron(_) => Some(after + chrono::Duration::hours(1)),
        }
    }
}

fn parse_duration(input: &str) -> Result<std::time::Duration, ()> {
    let input = input.trim();
    if input.is_empty() {
        return Err(());
    }
    let (num_str, suffix) = input.split_at(input.len() - 1);
    let num: u64 = num_str.parse().map_err(|_| ())?;
    match suffix {
        "s" => Ok(std::time::Duration::from_secs(num)),
        "m" => Ok(std::time::Duration::from_secs(num * 60)),
        "h" => Ok(std::time::Duration::from_secs(num * 3600)),
        "d" => Ok(std::time::Duration::from_secs(num * 86400)),
        _ => Err(()),
    }
}

fn parse_every_interval(input: &str) -> Option<std::time::Duration> {
    input.strip_prefix("every ").and_then(|rest| parse_duration(rest).ok())
}

fn is_cron_expression(input: &str) -> bool {
    let parts: Vec<&str> = input.split_whitespace().collect();
    parts.len() == 5
        && parts.iter().all(|p| {
            p.chars()
                .all(|c| c.is_ascii_digit() || c == '*' || c == ',' || c == '/' || c == '-')
        })
}

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("invalid schedule format: {0}")]
    InvalidFormat(String),
    #[error("invalid payload kind: {0}")]
    InvalidPayloadKind(String),
    #[error("invalid missed-run policy: {0}")]
    InvalidMissedRunPolicy(String),
}
```

- [ ] **Step 5: Create `crates/agent-cron/src/store.rs`**

```rust
use crate::{CronJob, MissedRunPolicy, NewCronJob, PayloadKind, Schedule};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CronStore {
    pool: Arc<SqlitePool>,
}

#[derive(Debug, Clone)]
pub struct CronTriggerRecord {
    pub id: String,
    pub cron_job_id: String,
    pub scheduled_for: DateTime<Utc>,
    pub triggered_at: DateTime<Utc>,
    pub task_id: String,
    pub status: String,
    pub error: Option<String>,
}

impl CronStore {
    pub async fn new(db_path: &Path) -> Result<Self, sqlx::Error> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let db_url = format!("sqlite://{}", db_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(options).await?;
        init_schema(&pool).await?;
        Ok(Self { pool: Arc::new(pool) })
    }

    pub async fn create_job(&self, job: NewCronJob) -> Result<CronJob, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let next_run_at = job.schedule.next_after(now);
        let schedule_json = serde_json::to_string(&job.schedule)
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
        let payload_json = serde_json::to_string(&job.payload_json)
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

        sqlx::query(
            "INSERT INTO cron_jobs (
                id, name, description, schedule_json, payload_kind, payload_json, enabled,
                missed_run_policy, next_run_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&job.name)
        .bind(&job.description)
        .bind(schedule_json)
        .bind(job.payload_kind.as_str())
        .bind(payload_json)
        .bind(job.enabled)
        .bind(job.missed_run_policy.as_str())
        .bind(next_run_at.map(|dt| dt.timestamp()))
        .bind(now.timestamp())
        .bind(now.timestamp())
        .execute(&*self.pool)
        .await?;

        self.load_job(&id).await?.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn load_job(&self, id: &str) -> Result<Option<CronJob>, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM cron_jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await?;
        row.map(row_to_job).transpose()
    }

    pub async fn list_jobs(&self) -> Result<Vec<CronJob>, sqlx::Error> {
        let rows = sqlx::query("SELECT * FROM cron_jobs ORDER BY created_at ASC")
            .fetch_all(&*self.pool)
            .await?;
        rows.into_iter().map(row_to_job).collect()
    }

    pub async fn list_due_jobs(&self, now: DateTime<Utc>) -> Result<Vec<CronJob>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT * FROM cron_jobs
             WHERE enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ?
             ORDER BY next_run_at ASC",
        )
        .bind(now.timestamp())
        .fetch_all(&*self.pool)
        .await?;
        rows.into_iter().map(row_to_job).collect()
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE cron_jobs SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled)
            .bind(Utc::now().timestamp())
            .bind(id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_job(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM cron_jobs WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn record_trigger(
        &self,
        cron_job_id: &str,
        scheduled_for: DateTime<Utc>,
        task_id: &str,
    ) -> Result<Option<CronTriggerRecord>, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let result = sqlx::query(
            "INSERT OR IGNORE INTO cron_triggers (
                id, cron_job_id, scheduled_for, triggered_at, task_id, status
             ) VALUES (?, ?, ?, ?, ?, 'created')",
        )
        .bind(&id)
        .bind(cron_job_id)
        .bind(scheduled_for.timestamp())
        .bind(now.timestamp())
        .bind(task_id)
        .execute(&*self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        Ok(Some(CronTriggerRecord {
            id,
            cron_job_id: cron_job_id.to_string(),
            scheduled_for,
            triggered_at: now,
            task_id: task_id.to_string(),
            status: "created".into(),
            error: None,
        }))
    }

    pub async fn advance_job(
        &self,
        job_id: &str,
        last_run_at: DateTime<Utc>,
        next_run_at: Option<DateTime<Utc>>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE cron_jobs SET last_run_at = ?, next_run_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(last_run_at.timestamp())
        .bind(next_run_at.map(|dt| dt.timestamp()))
        .bind(Utc::now().timestamp())
        .bind(job_id)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }
}

async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS cron_jobs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            schedule_json TEXT NOT NULL,
            payload_kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            enabled INTEGER NOT NULL,
            missed_run_policy TEXT NOT NULL,
            last_run_at INTEGER,
            next_run_at INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS cron_triggers (
            id TEXT PRIMARY KEY,
            cron_job_id TEXT NOT NULL,
            scheduled_for INTEGER NOT NULL,
            triggered_at INTEGER NOT NULL,
            task_id TEXT NOT NULL,
            status TEXT NOT NULL,
            error TEXT,
            UNIQUE(cron_job_id, scheduled_for)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_cron_jobs_due ON cron_jobs(enabled, next_run_at)")
        .execute(pool)
        .await?;
    Ok(())
}

fn row_to_job(row: sqlx::sqlite::SqliteRow) -> Result<CronJob, sqlx::Error> {
    let schedule = serde_json::from_str(&row.get::<String, _>("schedule_json"))
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let payload_json = serde_json::from_str(&row.get::<String, _>("payload_json"))
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let payload_kind = PayloadKind::try_from(row.get::<String, _>("payload_kind").as_str())
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let missed_run_policy =
        MissedRunPolicy::try_from(row.get::<String, _>("missed_run_policy").as_str())
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

    Ok(CronJob {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        schedule,
        payload_kind,
        payload_json,
        enabled: row.get("enabled"),
        missed_run_policy,
        last_run_at: optional_ts(row.get("last_run_at")),
        next_run_at: optional_ts(row.get("next_run_at")),
        created_at: ts(row.get("created_at")),
        updated_at: ts(row.get("updated_at")),
    })
}

fn ts(value: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(value, 0).single().unwrap()
}

fn optional_ts(value: Option<i64>) -> Option<DateTime<Utc>> {
    value.map(ts)
}
```

- [ ] **Step 6: Export store in `crates/agent-cron/src/lib.rs`**

```rust
pub mod job;
pub mod scheduler;
pub mod store;

#[cfg(test)]
pub mod parser;

pub use job::*;
pub use scheduler::*;
pub use store::*;
```

- [ ] **Step 7: Run cron store tests**

Run: `cargo test -p agent-cron --test store_test -- --nocapture`

Expected: PASS.

## Task 5: Make Cron Scheduler Create Kanban Tasks Idempotently

**Files:**
- Modify: `crates/agent-cron/src/scheduler.rs`
- Create: `crates/agent-cron/tests/scheduler_test.rs`

- [ ] **Step 1: Write failing scheduler integration test**

Create `crates/agent-cron/tests/scheduler_test.rs`:

```rust
use agent_cron::{CronScheduler, CronStore, MissedRunPolicy, NewCronJob, PayloadKind, Schedule};
use agent_kanban::{KanbanStore, TaskStatus};
use chrono::Utc;
use serde_json::json;

#[tokio::test]
async fn due_cron_job_creates_one_kanban_task_per_occurrence() {
    let dir = tempfile::tempdir().unwrap();
    let cron = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let kanban = KanbanStore::new(&dir.path().join("kanban.db")).await.unwrap();

    let job = cron
        .create_job(NewCronJob {
            name: "Every second".into(),
            description: "demo".into(),
            schedule: Schedule::parse("1s").unwrap(),
            payload_kind: PayloadKind::AgentPrompt,
            payload_json: json!({"prompt": "say hello"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::RunOnce,
        })
        .await
        .unwrap();

    cron.advance_job(&job.id, Utc::now(), Some(Utc::now() - chrono::Duration::seconds(1)))
        .await
        .unwrap();

    let scheduler = CronScheduler::new(cron.clone(), kanban.clone());
    let first = scheduler.tick_once(Utc::now()).await.unwrap();
    let second = scheduler.tick_once(Utc::now()).await.unwrap();

    assert_eq!(first, 1);
    assert_eq!(second, 0);

    let tasks = kanban.list_tasks(Some(TaskStatus::Queued)).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].source, "cron");
    assert_eq!(tasks[0].source_id.as_deref(), Some(job.id.as_str()));
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p agent-cron --test scheduler_test -- --nocapture`

Expected: FAIL because `CronScheduler::new` still takes tick seconds and has no `tick_once`.

- [ ] **Step 3: Replace `crates/agent-cron/src/scheduler.rs`**

```rust
use crate::{CronStore, PayloadKind};
use agent_kanban::{KanbanStore, NewTask, PayloadKind as KanbanPayloadKind};
use chrono::{DateTime, Utc};
use tokio::time::{Duration, interval};

#[derive(Clone)]
pub struct CronScheduler {
    cron_store: CronStore,
    kanban_store: KanbanStore,
}

impl CronScheduler {
    pub fn new(cron_store: CronStore, kanban_store: KanbanStore) -> Self {
        Self {
            cron_store,
            kanban_store,
        }
    }

    pub async fn tick_loop(&self, tick_secs: u64) {
        let mut ticker = interval(Duration::from_secs(tick_secs));
        loop {
            ticker.tick().await;
            if let Err(error) = self.tick_once(Utc::now()).await {
                tracing::error!(%error, "cron scheduler tick failed");
            }
        }
    }

    pub async fn tick_once(&self, now: DateTime<Utc>) -> Result<u64, sqlx::Error> {
        let due_jobs = self.cron_store.list_due_jobs(now).await?;
        let mut created = 0;

        for job in due_jobs {
            let Some(scheduled_for) = job.next_run_at else {
                continue;
            };

            let task = self
                .kanban_store
                .create_task(NewTask {
                    source: "cron".into(),
                    source_id: Some(job.id.clone()),
                    title: job.name.clone(),
                    body: job.description.clone(),
                    payload_kind: match job.payload_kind {
                        PayloadKind::AgentPrompt => KanbanPayloadKind::AgentPrompt,
                        PayloadKind::ShellCommand => KanbanPayloadKind::ShellCommand,
                    },
                    payload_json: job.payload_json.clone(),
                    priority: 0,
                    max_retries: 1,
                })
                .await?;

            if self
                .cron_store
                .record_trigger(&job.id, scheduled_for, &task.id)
                .await?
                .is_some()
            {
                created += 1;
            }

            self.cron_store
                .advance_job(&job.id, now, job.schedule.next_after(now))
                .await?;
        }

        Ok(created)
    }
}
```

- [ ] **Step 4: Run scheduler test**

Run: `cargo test -p agent-cron --test scheduler_test -- --nocapture`

Expected: PASS. If it creates two tasks, move `record_trigger` before task creation or add a cleanup path; the final passing behavior must be one queued task for the occurrence.

## Task 6: Implement Embedded Worker Runtime With Test Executor

**Files:**
- Modify: `crates/agent-kanban/src/worker.rs`
- Create: `crates/agent-kanban/tests/worker_test.rs`

- [ ] **Step 1: Write failing worker test**

Create `crates/agent-kanban/tests/worker_test.rs`:

```rust
use agent_kanban::{ExecutionResult, KanbanStore, NewTask, PayloadKind, TaskExecutor, TaskStatus, WorkerRuntime};
use async_trait::async_trait;
use serde_json::json;
use std::sync::{Arc, Mutex};

struct RecordingExecutor {
    seen: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl TaskExecutor for RecordingExecutor {
    async fn execute(&self, task: &agent_kanban::TaskRecord) -> ExecutionResult {
        self.seen.lock().unwrap().push(task.id.clone());
        ExecutionResult::succeeded("executed")
    }
}

#[tokio::test]
async fn worker_claims_executes_and_completes_one_task() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db")).await.unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Run".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    let seen = Arc::new(Mutex::new(Vec::new()));
    let worker = WorkerRuntime::new(
        "worker-a".into(),
        store.clone(),
        Arc::new(RecordingExecutor { seen: seen.clone() }),
    );

    let executed = worker.run_once().await.unwrap();
    assert!(executed);
    assert_eq!(seen.lock().unwrap().as_slice(), &[task.id.clone()]);

    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Succeeded);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p agent-kanban --test worker_test -- --nocapture`

Expected: FAIL with missing `TaskExecutor` and `WorkerRuntime::new`.

- [ ] **Step 3: Replace `crates/agent-kanban/src/worker.rs`**

```rust
use crate::{KanbanStore, TaskRecord, TaskStatus};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::{Duration, interval};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: TaskStatus,
    pub output_summary: Option<String>,
    pub error: Option<String>,
}

impl ExecutionResult {
    pub fn succeeded(summary: impl Into<String>) -> Self {
        Self {
            status: TaskStatus::Succeeded,
            output_summary: Some(summary.into()),
            error: None,
        }
    }

    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            status: TaskStatus::Failed,
            output_summary: None,
            error: Some(error.into()),
        }
    }
}

#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute(&self, task: &TaskRecord) -> ExecutionResult;
}

pub struct WorkerRuntime {
    worker_id: String,
    store: KanbanStore,
    executor: Arc<dyn TaskExecutor>,
    claim_ttl_secs: i64,
}

impl WorkerRuntime {
    pub fn new(worker_id: String, store: KanbanStore, executor: Arc<dyn TaskExecutor>) -> Self {
        Self {
            worker_id,
            store,
            executor,
            claim_ttl_secs: 60,
        }
    }

    pub async fn run_loop(&self, tick_secs: u64) {
        let mut ticker = interval(Duration::from_secs(tick_secs));
        loop {
            ticker.tick().await;
            if let Err(error) = self.store.recover_expired_claims().await {
                tracing::error!(%error, "failed to recover expired kanban claims");
            }
            if let Err(error) = self.run_once().await {
                tracing::error!(%error, "worker tick failed");
            }
        }
    }

    pub async fn run_once(&self) -> Result<bool, sqlx::Error> {
        let Some(claimed) = self
            .store
            .claim_next_task(&self.worker_id, self.claim_ttl_secs)
            .await?
        else {
            return Ok(false);
        };

        self.store
            .mark_running(&claimed.task.id, &claimed.run.id, &claimed.claim_lock)
            .await?;
        self.store
            .heartbeat(&claimed.task.id, &claimed.claim_lock, self.claim_ttl_secs)
            .await?;

        let result = self.executor.execute(&claimed.task).await;
        self.store
            .complete_run(
                &claimed.task.id,
                &claimed.run.id,
                &claimed.claim_lock,
                result.status,
                result.output_summary.as_deref(),
                result.error.as_deref(),
            )
            .await?;

        Ok(true)
    }
}
```

- [ ] **Step 4: Run worker tests**

Run: `cargo test -p agent-kanban --test worker_test -- --nocapture`

Expected: PASS.

## Task 7: Add Serve State, API Routes, And HTML Dashboards

**Files:**
- Modify: `crates/agent-cli/Cargo.toml`
- Modify: `crates/agent-cli/src/cmd/serve.rs`
- Create: `crates/agent-cli/tests/cron_api_test.rs`
- Create: `crates/agent-cli/tests/dashboard_test.rs`

- [ ] **Step 1: Write failing API serialization test**

Create `crates/agent-cli/tests/cron_api_test.rs`:

```rust
use agent_cli::cmd::serve::{CronJobResponse, TaskResponse};
use agent_cron::{CronJob, MissedRunPolicy, PayloadKind, Schedule};
use agent_kanban::{TaskRecord, TaskStatus};
use chrono::Utc;
use serde_json::json;

#[test]
fn serializes_cron_job_response() {
    let now = Utc::now();
    let job = CronJob {
        id: "job-1".into(),
        name: "Demo".into(),
        description: "demo".into(),
        schedule: Schedule::parse("5m").unwrap(),
        payload_kind: PayloadKind::AgentPrompt,
        payload_json: json!({"prompt": "hello"}),
        enabled: true,
        missed_run_policy: MissedRunPolicy::RunOnce,
        last_run_at: None,
        next_run_at: Some(now),
        created_at: now,
        updated_at: now,
    };

    let response = CronJobResponse::from(job);
    assert_eq!(response.id, "job-1");
    assert_eq!(response.name, "Demo");
    assert!(response.enabled);
}

#[test]
fn serializes_task_response() {
    let now = Utc::now();
    let task = TaskRecord {
        id: "task-1".into(),
        source: "cron".into(),
        source_id: Some("job-1".into()),
        title: "Demo".into(),
        body: "demo".into(),
        payload_kind: agent_kanban::PayloadKind::AgentPrompt,
        payload_json: json!({"prompt": "hello"}),
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
    };

    let response = TaskResponse::from(task);
    assert_eq!(response.id, "task-1");
    assert_eq!(response.status, "queued");
}
```

- [ ] **Step 2: Write failing dashboard HTML test**

Create `crates/agent-cli/tests/dashboard_test.rs`:

```rust
use agent_cli::cmd::serve::{cron_dashboard_html, tasks_dashboard_html};

#[test]
fn cron_dashboard_contains_polling_target() {
    let html = cron_dashboard_html();
    assert!(html.contains("Cron Jobs"));
    assert!(html.contains("/api/cron/jobs"));
}

#[test]
fn tasks_dashboard_contains_kanban_columns() {
    let html = tasks_dashboard_html();
    assert!(html.contains("Kanban Tasks"));
    assert!(html.contains("queued"));
    assert!(html.contains("running"));
    assert!(html.contains("failed"));
}
```

- [ ] **Step 3: Run tests to verify failure**

Run: `cargo test -p agent-cli --test cron_api_test --test dashboard_test -- --nocapture`

Expected: FAIL with missing response types and HTML helpers.

- [ ] **Step 4: Add dependencies to `crates/agent-cli/Cargo.toml`**

Ensure dependencies include:

```toml
agent-cron = { path = "../agent-cron" }
agent-kanban = { path = "../agent-kanban" }
```

- [ ] **Step 5: Add response types and HTML helpers to `serve.rs`**

Add imports:

```rust
use agent_cron::{CronJob, CronStore, NewCronJob};
use agent_kanban::{KanbanStore, NewTask, TaskRecord, TaskStatus};
use axum::response::Html;
```

Add response structs below `RunResponse`:

```rust
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CronJobResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub next_run_at: Option<String>,
    pub last_run_at: Option<String>,
}

impl From<CronJob> for CronJobResponse {
    fn from(job: CronJob) -> Self {
        Self {
            id: job.id,
            name: job.name,
            description: job.description,
            enabled: job.enabled,
            next_run_at: job.next_run_at.map(|dt| dt.to_rfc3339()),
            last_run_at: job.last_run_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TaskResponse {
    pub id: String,
    pub source: String,
    pub source_id: Option<String>,
    pub title: String,
    pub status: String,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<TaskRecord> for TaskResponse {
    fn from(task: TaskRecord) -> Self {
        Self {
            id: task.id,
            source: task.source,
            source_id: task.source_id,
            title: task.title,
            status: task.status.as_str().into(),
            last_error: task.last_error,
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
        }
    }
}

pub fn cron_dashboard_html() -> String {
    r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Cron Jobs</title></head>
<body>
<h1>Cron Jobs</h1>
<table id="jobs"><thead><tr><th>Name</th><th>Enabled</th><th>Next Run</th><th>Last Run</th></tr></thead><tbody></tbody></table>
<script>
async function refresh() {
  const jobs = await fetch('/api/cron/jobs').then(r => r.json());
  document.querySelector('#jobs tbody').innerHTML = jobs.map(j => `<tr><td>${j.name}</td><td>${j.enabled}</td><td>${j.next_run_at || ''}</td><td>${j.last_run_at || ''}</td></tr>`).join('');
}
refresh(); setInterval(refresh, 5000);
</script>
</body></html>"#.to_string()
}

pub fn tasks_dashboard_html() -> String {
    r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Kanban Tasks</title></head>
<body>
<h1>Kanban Tasks</h1>
<div id="columns">queued running blocked failed succeeded timed_out lost</div>
<pre id="tasks"></pre>
<script>
async function refresh() {
  const tasks = await fetch('/api/tasks').then(r => r.json());
  document.querySelector('#tasks').textContent = JSON.stringify(tasks, null, 2);
}
refresh(); setInterval(refresh, 5000);
</script>
</body></html>"#.to_string()
}
```

- [ ] **Step 6: Extend `GatewayState`**

Change `GatewayState`:

```rust
pub struct GatewayState {
    pub port: u16,
    pub config: AgentConfig,
    pub cron_store: Option<CronStore>,
    pub kanban_store: Option<KanbanStore>,
}
```

Change constructor:

```rust
impl GatewayState {
    pub fn new(port: u16, config: AgentConfig) -> Self {
        Self {
            port,
            config,
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
            cron_store: Some(cron_store),
            kanban_store: Some(kanban_store),
        }
    }
}
```

- [ ] **Step 7: Add API/dashboard routes**

Modify `build_router` routes:

```rust
pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/run", post(handle_run))
        .route("/run/stream", post(handle_run_stream))
        .route("/cron", get(handle_cron_dashboard))
        .route("/tasks", get(handle_tasks_dashboard))
        .route("/api/cron/jobs", get(handle_list_cron_jobs).post(handle_create_cron_job))
        .route("/api/tasks", get(handle_list_tasks).post(handle_create_task))
        .route("/webhook/telegram", post(handle_telegram))
        .with_state(state)
}
```

Add handlers:

```rust
async fn handle_cron_dashboard() -> Html<String> {
    Html(cron_dashboard_html())
}

async fn handle_tasks_dashboard() -> Html<String> {
    Html(tasks_dashboard_html())
}

async fn handle_list_cron_jobs(State(state): State<SharedState>) -> Json<Vec<CronJobResponse>> {
    let Some(store) = &state.cron_store else {
        return Json(Vec::new());
    };
    let jobs = store.list_jobs().await.unwrap_or_default();
    Json(jobs.into_iter().map(CronJobResponse::from).collect())
}

async fn handle_create_cron_job(
    State(state): State<SharedState>,
    Json(payload): Json<NewCronJob>,
) -> Json<Option<CronJobResponse>> {
    let Some(store) = &state.cron_store else {
        return Json(None);
    };
    Json(store.create_job(payload).await.ok().map(CronJobResponse::from))
}

async fn handle_list_tasks(State(state): State<SharedState>) -> Json<Vec<TaskResponse>> {
    let Some(store) = &state.kanban_store else {
        return Json(Vec::new());
    };
    let tasks = store.list_tasks(None).await.unwrap_or_default();
    Json(tasks.into_iter().map(TaskResponse::from).collect())
}

async fn handle_create_task(
    State(state): State<SharedState>,
    Json(payload): Json<NewTask>,
) -> Json<Option<TaskResponse>> {
    let Some(store) = &state.kanban_store else {
        return Json(None);
    };
    Json(store.create_task(payload).await.ok().map(TaskResponse::from))
}
```

- [ ] **Step 8: Run tests**

Run: `cargo test -p agent-cli --test cron_api_test --test dashboard_test -- --nocapture`

Expected: PASS.

## Task 8: Start Embedded Scheduler And Worker In `serve`

**Files:**
- Modify: `crates/agent-cli/src/cmd/serve.rs`
- Modify: `crates/agent-cli/src/runtime.rs` if a shared data path helper is needed.

- [ ] **Step 1: Add local data path helpers**

Add to `serve.rs`:

```rust
fn default_automation_db_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("automation.db")
}
```

- [ ] **Step 2: Add a simple executor for shell-command MVP**

Add to `serve.rs`:

```rust
struct ServeTaskExecutor;

#[async_trait::async_trait]
impl agent_kanban::TaskExecutor for ServeTaskExecutor {
    async fn execute(&self, task: &TaskRecord) -> agent_kanban::ExecutionResult {
        match task.payload_kind {
            agent_kanban::PayloadKind::ShellCommand => {
                let Some(command) = task.payload_json.get("command").and_then(|v| v.as_str()) else {
                    return agent_kanban::ExecutionResult::failed("shell task missing command");
                };
                match tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .output()
                    .await
                {
                    Ok(output) if output.status.success() => {
                        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        agent_kanban::ExecutionResult::succeeded(stdout)
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        agent_kanban::ExecutionResult::failed(stderr)
                    }
                    Err(error) => agent_kanban::ExecutionResult::failed(error.to_string()),
                }
            }
            agent_kanban::PayloadKind::AgentPrompt => {
                agent_kanban::ExecutionResult::failed("agent prompt execution is wired in the next task")
            }
        }
    }
}
```

- [ ] **Step 3: Initialize stores and background loops in `run`**

Replace the state creation block in `run`:

```rust
    let config = load_config(config_path);
    let automation_db = default_automation_db_path();
    let cron_store = CronStore::new(&automation_db).await?;
    let kanban_store = KanbanStore::new(&automation_db).await?;

    let scheduler = agent_cron::CronScheduler::new(cron_store.clone(), kanban_store.clone());
    tokio::spawn(async move {
        scheduler.tick_loop(10).await;
    });

    let worker = agent_kanban::WorkerRuntime::new(
        "serve-worker".into(),
        kanban_store.clone(),
        std::sync::Arc::new(ServeTaskExecutor),
    );
    tokio::spawn(async move {
        worker.run_loop(5).await;
    });

    let state = Arc::new(GatewayState::with_stores(port, config, cron_store, kanban_store));
    let app = build_router(state);
```

- [ ] **Step 4: Run build check**

Run: `cargo check -p agent-cli`

Expected: PASS.

## Task 9: Wire Agent Prompt Execution Into Worker

**Files:**
- Modify: `crates/agent-cli/src/cmd/serve.rs`
- Modify: `crates/agent-kanban/src/store.rs`
- Modify: `crates/agent-kanban/tests/store_test.rs`

- [ ] **Step 1: Add task event listing test**

Append to `crates/agent-kanban/tests/store_test.rs`:

```rust
#[tokio::test]
async fn lists_task_events_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db")).await.unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Events".into(),
            body: "body".into(),
            payload_kind: PayloadKind::AgentPrompt,
            payload_json: json!({"prompt": "hello"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    store
        .append_task_event(&task.id, None, "agent_event", json!({"type": "final"}))
        .await
        .unwrap();

    let events = store.list_task_events(&task.id).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "task_created");
    assert_eq!(events[1].event_type, "agent_event");
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p agent-kanban --test store_test lists_task_events_in_order -- --nocapture`

Expected: FAIL with missing `list_task_events`.

- [ ] **Step 3: Add `list_task_events` to `KanbanStore`**

Add inside `impl KanbanStore`:

```rust
    pub async fn list_task_events(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskEventRecord>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, task_id, run_id, event_type, payload_json, created_at
             FROM task_events WHERE task_id = ? ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(&*self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let payload_json = serde_json::from_str(&row.get::<String, _>("payload_json"))
                    .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
                Ok(TaskEventRecord {
                    id: row.get("id"),
                    task_id: row.get("task_id"),
                    run_id: row.get("run_id"),
                    event_type: row.get("event_type"),
                    payload_json,
                    created_at: ts(row.get("created_at")),
                })
            })
            .collect()
    }
```

- [ ] **Step 4: Replace `ServeTaskExecutor` with config-aware executor**

Change struct in `serve.rs`:

```rust
struct ServeTaskExecutor {
    config: AgentConfig,
    kanban_store: KanbanStore,
}
```

Replace the `AgentPrompt` branch:

```rust
            agent_kanban::PayloadKind::AgentPrompt => {
                let Some(prompt) = task.payload_json.get("prompt").and_then(|v| v.as_str()) else {
                    return agent_kanban::ExecutionResult::failed("agent task missing prompt");
                };

                let agent = build_agent(&self.config, None).await;
                let mut input = AgentInput {
                    user_message: prompt.to_string(),
                    session_id: format!("task-{}", task.id),
                    system_prompt: "You are a helpful assistant executing a scheduled background task.".into(),
                    history: Vec::new(),
                    workspace_dir: std::env::current_dir().unwrap_or_default(),
                };
                let task_id = task.id.clone();
                let store = self.kanban_store.clone();
                let mut sink = agent_core::callback_event_sink(move |event| {
                    let task_id = task_id.clone();
                    let store = store.clone();
                    tokio::spawn(async move {
                        let payload = serde_json::to_value(event).unwrap_or_else(|_| serde_json::json!({"error": "event serialization failed"}));
                        let _ = store.append_task_event(&task_id, None, "agent_event", payload).await;
                    });
                });

                match agent.run_with_events(&mut input, &mut sink).await {
                    Ok(AgentOutput::Final(content)) => agent_kanban::ExecutionResult::succeeded(content),
                    Ok(AgentOutput::Cancelled) => agent_kanban::ExecutionResult::failed("agent task cancelled"),
                    Err(error) => agent_kanban::ExecutionResult::failed(error.to_string()),
                }
            }
```

Change worker construction:

```rust
    let worker = agent_kanban::WorkerRuntime::new(
        "serve-worker".into(),
        kanban_store.clone(),
        std::sync::Arc::new(ServeTaskExecutor {
            config: config.clone(),
            kanban_store: kanban_store.clone(),
        }),
    );
```

- [ ] **Step 5: Run targeted tests and build**

Run: `cargo test -p agent-kanban --test store_test lists_task_events_in_order -- --nocapture`

Expected: PASS.

Run: `cargo check -p agent-cli`

Expected: PASS.

## Task 10: Replace CLI Cron JSON Management With `CronStore`

**Files:**
- Modify: `crates/agent-cli/src/cmd/cron.rs`

- [ ] **Step 1: Replace JSON helpers with DB path helper**

Replace file-level helper code in `crates/agent-cli/src/cmd/cron.rs`:

```rust
use agent_cron::CronStore;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum CronCommand {
    /// 查看所有定时任务
    List,
    /// 通过 ID 删除任务
    Delete { id: String },
    /// 暂停任务
    Pause { id: String },
    /// 恢复任务
    Resume { id: String },
}

fn automation_db_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("automation.db")
}
```

- [ ] **Step 2: Replace `run` implementation**

```rust
pub async fn run(cmd: CronCommand) -> anyhow::Result<()> {
    let store = CronStore::new(&automation_db_file()).await?;

    match cmd {
        CronCommand::List => {
            let jobs = store.list_jobs().await?;
            if jobs.is_empty() {
                println!("No scheduled tasks found.");
                return Ok(());
            }

            println!(
                "{:<38} | {:<20} | {:<8} | {:<25} | {}",
                "ID", "Name", "Enabled", "Next Run", "Description"
            );
            println!("{}", "-".repeat(120));

            for job in jobs {
                println!(
                    "{:<38} | {:<20} | {:<8} | {:<25} | {}",
                    job.id,
                    job.name.chars().take(20).collect::<String>(),
                    job.enabled,
                    job.next_run_at
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| "-".into()),
                    job.description.chars().take(30).collect::<String>()
                );
            }
        }
        CronCommand::Delete { id } => {
            if store.delete_job(&id).await? {
                println!("Deleted job: {}", id);
            } else {
                println!("Job '{}' not found.", id);
            }
        }
        CronCommand::Pause { id } => {
            store.set_enabled(&id, false).await?;
            println!("Paused job: {}", id);
        }
        CronCommand::Resume { id } => {
            store.set_enabled(&id, true).await?;
            println!("Resumed job: {}", id);
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Run CLI package check**

Run: `cargo check -p agent-cli`

Expected: PASS.

## Task 11: Update Cron Tool To Create Durable Cron Jobs

**Files:**
- Modify: `tools/cron-job/Cargo.toml`
- Modify: `tools/cron-job/src/lib.rs`
- Modify or create: `tools/cron-job/tests/cron_tool_test.rs`

- [ ] **Step 1: Add dependency**

Ensure `tools/cron-job/Cargo.toml` includes:

```toml
agent-cron = { path = "../../crates/agent-cron" }
```

- [ ] **Step 2: Replace direct JSON writes with `CronStore`**

In the tool handler that creates jobs, construct:

```rust
let store = agent_cron::CronStore::new(&dirs::home_dir().unwrap_or_default().join(".agent").join("automation.db")).await?;
let job = store
    .create_job(agent_cron::NewCronJob {
        name,
        description,
        schedule: agent_cron::Schedule::parse(&schedule)?,
        payload_kind: agent_cron::PayloadKind::AgentPrompt,
        payload_json: serde_json::json!({"prompt": command}),
        enabled: true,
        missed_run_policy: agent_cron::MissedRunPolicy::RunOnce,
    })
    .await?;
```

Return a tool result containing `job.id`, `job.name`, and `job.next_run_at`.

- [ ] **Step 3: Run tool tests**

Run: `cargo test -p cron-job -- --nocapture`

Expected: PASS.

## Task 12: Full Verification And Documentation

**Files:**
- Modify: `README.md`
- Modify: `crates/agent-cli/src/cmd/doctor.rs` if it reports cron state.

- [ ] **Step 1: Update README cron section**

Add a section:

```markdown
## Durable Cron And Kanban

The service stores automation state in `~/.agent/automation.db`.

- Cron jobs define when work should start.
- Due cron jobs create durable Kanban tasks.
- The embedded worker claims queued tasks, writes heartbeat, records runs, and appends task events.
- `/cron` shows schedule status.
- `/tasks` shows queued/running/finished task state.

Example API payload:

```json
{
  "name": "Every five minutes",
  "description": "Demo scheduled agent task",
  "schedule": { "Duration": { "secs": 300, "nanos": 0 } },
  "payload_kind": "agent_prompt",
  "payload_json": { "prompt": "Say hello from cron" },
  "enabled": true,
  "missed_run_policy": "run_once"
}
```
```

- [ ] **Step 2: Run formatting**

Run: `cargo fmt --all`

Expected: PASS with no output or formatted files.

- [ ] **Step 3: Run full test suite**

Run: `cargo test --workspace`

Expected: PASS.

- [ ] **Step 4: Run full build**

Run: `cargo build --workspace`

Expected: PASS.

- [ ] **Step 5: Manual smoke test**

Run service:

```bash
cargo run -p agent-cli -- serve --host 127.0.0.1 --port 8080
```

Create a cron job in another shell:

```bash
curl -s http://127.0.0.1:8080/api/cron/jobs \
  -H 'content-type: application/json' \
  -d '{"name":"Every five","description":"demo","schedule":{"Duration":{"secs":300,"nanos":0}},"payload_kind":"agent_prompt","payload_json":{"prompt":"Say hello from cron"},"enabled":true,"missed_run_policy":"run_once"}'
```

Check dashboards:

```bash
curl -s http://127.0.0.1:8080/cron
curl -s http://127.0.0.1:8080/tasks
curl -s http://127.0.0.1:8080/api/cron/jobs
curl -s http://127.0.0.1:8080/api/tasks
```

Expected: `/cron` contains `Cron Jobs`, `/tasks` contains `Kanban Tasks`, API endpoints return JSON arrays, and the job persists in `~/.agent/automation.db` after service restart.

## Self-Review Notes

- Spec coverage: The plan covers durable cron storage, Kanban task storage, idempotent trigger creation, worker claim/heartbeat/completion/recovery, API/dashboard surfaces, CLI/tool migration, Agent event persistence, and verification.
- Scope: The plan intentionally delivers local SQLite and embedded worker first. Multi-process worker hardening remains enabled by atomic claims but does not require a separate process in this batch.
- Type consistency: Payload/status enums are defined in `agent-kanban` and mirrored in `agent-cron` for API payloads; conversion happens in `CronScheduler`.
- Commit policy: Do not create git commits unless the user explicitly asks for commits. The implementation tasks are still separated so commits can be made later if requested.
