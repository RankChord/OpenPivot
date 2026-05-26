use agent_kanban::{
    ExecutionResult, KanbanStore, NewTask, PayloadKind, TaskExecutor, TaskRecord, TaskStatus,
    WorkerRuntime,
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, sleep};

#[test]
fn retry_classifier_rejects_non_retryable_errors() {
    assert!(!agent_kanban::is_retryable_error("missing shell command"));
    assert!(!agent_kanban::is_retryable_error("permission denied"));
    assert!(!agent_kanban::is_retryable_error("forbidden"));
    assert!(!agent_kanban::is_retryable_error("validation failed"));
    assert!(!agent_kanban::is_retryable_error("cancelled"));
    assert!(agent_kanban::is_retryable_error("temporary network error"));
}

#[derive(Default)]
struct RecordingExecutor {
    seen_task_ids: Mutex<Vec<String>>,
}

#[async_trait]
impl TaskExecutor for RecordingExecutor {
    async fn execute(&self, task: &TaskRecord) -> ExecutionResult {
        self.seen_task_ids.lock().unwrap().push(task.id.clone());
        ExecutionResult::succeeded(Some("recorded".into()))
    }
}

struct FailingExecutor;

#[async_trait]
impl TaskExecutor for FailingExecutor {
    async fn execute(&self, _task: &TaskRecord) -> ExecutionResult {
        ExecutionResult::failed("executor failed".into())
    }
}

struct ErrorExecutor {
    error: &'static str,
}

#[async_trait]
impl TaskExecutor for ErrorExecutor {
    async fn execute(&self, _task: &TaskRecord) -> ExecutionResult {
        ExecutionResult::failed(self.error.into())
    }
}

struct DelayedExecutor {
    delay: Duration,
}

#[async_trait]
impl TaskExecutor for DelayedExecutor {
    async fn execute(&self, _task: &TaskRecord) -> ExecutionResult {
        sleep(self.delay).await;
        ExecutionResult::succeeded(Some("delayed".into()))
    }
}

struct CancellingExecutor {
    store: KanbanStore,
}

#[async_trait]
impl TaskExecutor for CancellingExecutor {
    async fn execute(&self, task: &TaskRecord) -> ExecutionResult {
        assert!(self.store.cancel_task(&task.id).await.unwrap());
        ExecutionResult::succeeded(Some("cancelled before completion".into()))
    }
}

#[tokio::test]
async fn run_once_executes_one_queued_task_and_marks_it_succeeded() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Execute me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();
    let executor = Arc::new(RecordingExecutor::default());
    let runtime = WorkerRuntime::new("worker-a".into(), store.clone(), executor.clone());

    let ran_task = runtime.run_once().await.unwrap();

    assert!(ran_task);
    assert_eq!(
        executor.seen_task_ids.lock().unwrap().as_slice(),
        [task.id.clone()]
    );
    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Succeeded);
}

#[tokio::test]
async fn run_once_returns_false_when_no_task() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let executor = Arc::new(RecordingExecutor::default());
    let runtime = WorkerRuntime::new("worker-a".into(), store, executor.clone());

    let ran_task = runtime.run_once().await.unwrap();

    assert!(!ran_task);
    assert!(executor.seen_task_ids.lock().unwrap().is_empty());
}

#[tokio::test]
async fn worker_marks_failed_execution_failed() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = create_task(&store).await;
    let runtime = WorkerRuntime::new("worker-a".into(), store.clone(), Arc::new(FailingExecutor));

    let ran_task = runtime.run_once().await.unwrap();

    assert!(ran_task);
    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Failed);
    assert_eq!(loaded.last_error.as_deref(), Some("executor failed"));
}

#[tokio::test]
async fn worker_requeues_retryable_failure() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Retry me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 1,
        })
        .await
        .unwrap();
    let runtime = WorkerRuntime::new(
        "worker-a".into(),
        store.clone(),
        Arc::new(ErrorExecutor {
            error: "temporary network error",
        }),
    );

    let ran_task = runtime.run_once().await.unwrap();

    assert!(ran_task);
    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Queued);
    assert_eq!(loaded.retry_count, 1);
    assert_eq!(
        loaded.last_error.as_deref(),
        Some("temporary network error")
    );

    let runs = store.list_task_runs(&task.id).await.unwrap();
    assert_eq!(runs[0].status, TaskStatus::Failed);
    assert_eq!(runs[0].error.as_deref(), Some("temporary network error"));

    let events = store.list_task_events(&task.id).await.unwrap();
    assert_eq!(events.last().unwrap().event_type, "task_requeued");
}

#[tokio::test]
async fn worker_marks_non_retryable_failure_failed() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Do not retry me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 1,
        })
        .await
        .unwrap();
    let runtime = WorkerRuntime::new(
        "worker-a".into(),
        store.clone(),
        Arc::new(ErrorExecutor {
            error: "missing shell command",
        }),
    );

    let ran_task = runtime.run_once().await.unwrap();

    assert!(ran_task);
    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Failed);
    assert_eq!(loaded.retry_count, 0);
    assert_eq!(loaded.last_error.as_deref(), Some("missing shell command"));
}

#[tokio::test]
async fn worker_heartbeats_during_delayed_execution() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = create_task(&store).await;
    let runtime = WorkerRuntime::new(
        "worker-a".into(),
        store.clone(),
        Arc::new(DelayedExecutor {
            delay: Duration::from_secs(3),
        }),
    )
    .with_claim_ttl_secs(2)
    .with_heartbeat_interval(Duration::from_secs(1));

    let run = tokio::spawn(async move { runtime.run_once().await });
    sleep(Duration::from_millis(2200)).await;
    let recovered = store.recover_expired_claims().await.unwrap();
    let ran_task = run.await.unwrap().unwrap();

    assert_eq!(recovered, 0);
    assert!(ran_task);
    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Succeeded);
}

#[tokio::test]
async fn worker_does_not_succeed_task_cancelled_during_execution() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = create_task(&store).await;
    let runtime = WorkerRuntime::new(
        "worker-a".into(),
        store.clone(),
        Arc::new(DelayedExecutor {
            delay: Duration::from_secs(2),
        }),
    )
    .with_claim_ttl_secs(2)
    .with_heartbeat_interval(Duration::from_secs(1));

    let run = tokio::spawn(async move { runtime.run_once().await });
    sleep(Duration::from_millis(200)).await;
    assert!(store.cancel_task(&task.id).await.unwrap());
    let ran_task = run.await.unwrap().unwrap();

    assert!(ran_task);
    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Cancelled);
}

#[tokio::test]
async fn worker_treats_cancellation_before_completion_as_successful_run_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = create_task(&store).await;
    let runtime = WorkerRuntime::new(
        "worker-a".into(),
        store.clone(),
        Arc::new(CancellingExecutor {
            store: store.clone(),
        }),
    );

    let ran_task = runtime.run_once().await.unwrap();

    assert!(ran_task);
    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Cancelled);
}

async fn create_task(store: &KanbanStore) -> TaskRecord {
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Execute me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap()
}
