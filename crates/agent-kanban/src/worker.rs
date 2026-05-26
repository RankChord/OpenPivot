use crate::{KanbanStore, TaskRecord, TaskStatus};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

pub fn is_retryable_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    ![
        "missing",
        "permission",
        "forbidden",
        "validation",
        "cancelled",
    ]
    .iter()
    .any(|marker| error.contains(marker))
}

pub struct ExecutionResult {
    pub status: TaskStatus,
    pub output_summary: Option<String>,
    pub error: Option<String>,
}

impl ExecutionResult {
    pub fn succeeded(output_summary: Option<String>) -> Self {
        Self {
            status: TaskStatus::Succeeded,
            output_summary,
            error: None,
        }
    }

    pub fn failed(error: String) -> Self {
        Self {
            status: TaskStatus::Failed,
            output_summary: None,
            error: Some(error),
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
    heartbeat_interval: Duration,
}

impl WorkerRuntime {
    pub fn new(worker_id: String, store: KanbanStore, executor: Arc<dyn TaskExecutor>) -> Self {
        Self {
            worker_id,
            store,
            executor,
            claim_ttl_secs: 60,
            heartbeat_interval: Duration::from_secs(30),
        }
    }

    pub fn with_claim_ttl_secs(mut self, claim_ttl_secs: i64) -> Self {
        self.claim_ttl_secs = claim_ttl_secs;
        self
    }

    pub fn with_heartbeat_interval(mut self, heartbeat_interval: Duration) -> Self {
        self.heartbeat_interval = heartbeat_interval;
        self
    }

    pub async fn run_loop(&self, tick_secs: u64) {
        let tick = Duration::from_secs(tick_secs);

        loop {
            if let Err(error) = self.store.recover_expired_claims().await {
                tracing::error!(?error, "failed to recover expired claims");
            }

            if let Err(error) = self.run_once().await {
                tracing::error!(?error, "worker run failed");
            }

            sleep(tick).await;
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
        if self
            .heartbeat_or_cancelled(&claimed.task.id, &claimed.claim_lock)
            .await?
        {
            return Ok(true);
        }

        let execute = self.executor.execute(&claimed.task);
        tokio::pin!(execute);
        let mut heartbeat = tokio::time::interval(self.effective_heartbeat_interval());
        heartbeat.tick().await;

        let result = loop {
            tokio::select! {
                result = &mut execute => break result,
                _ = heartbeat.tick() => {
                    if self
                        .heartbeat_or_cancelled(&claimed.task.id, &claimed.claim_lock)
                        .await?
                    {
                        return Ok(true);
                    }
                }
            }
        };

        if self.store.is_task_cancelled(&claimed.task.id).await? {
            return Ok(true);
        }

        if self
            .complete_or_cancelled(
                &claimed.task.id,
                &claimed.run.id,
                &claimed.claim_lock,
                result,
            )
            .await?
        {
            return Ok(true);
        }

        Ok(true)
    }

    async fn heartbeat_or_cancelled(
        &self,
        task_id: &str,
        claim_lock: &str,
    ) -> Result<bool, sqlx::Error> {
        if self.store.is_task_cancelled(task_id).await? {
            return Ok(true);
        }

        if let Err(error) = self
            .store
            .heartbeat(task_id, claim_lock, self.claim_ttl_secs)
            .await
        {
            if matches!(error, sqlx::Error::RowNotFound)
                && self.store.is_task_cancelled(task_id).await?
            {
                return Ok(true);
            }
            return Err(error);
        }

        Ok(false)
    }

    async fn complete_or_cancelled(
        &self,
        task_id: &str,
        run_id: &str,
        claim_lock: &str,
        result: ExecutionResult,
    ) -> Result<bool, sqlx::Error> {
        if self.store.is_task_cancelled(task_id).await? {
            return Ok(true);
        }

        if result.status == TaskStatus::Failed {
            if let Some(error) = result.error.as_deref() {
                if is_retryable_error(error) {
                    if self
                        .store
                        .requeue_for_retry(task_id, run_id, claim_lock, error)
                        .await?
                    {
                        return Ok(false);
                    }
                    return Ok(false);
                }
            }
        }

        if let Err(error) = self
            .store
            .complete_run(
                task_id,
                run_id,
                claim_lock,
                result.status,
                result.output_summary.as_deref(),
                result.error.as_deref(),
            )
            .await
        {
            if matches!(error, sqlx::Error::RowNotFound)
                && self.store.is_task_cancelled(task_id).await?
            {
                return Ok(true);
            }
            return Err(error);
        }

        Ok(false)
    }

    fn effective_heartbeat_interval(&self) -> Duration {
        let ttl_half_secs = (self.claim_ttl_secs / 2).max(1) as u64;
        self.heartbeat_interval
            .min(Duration::from_secs(ttl_half_secs))
            .max(Duration::from_secs(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewTask, PayloadKind};
    use serde_json::json;

    struct NoopExecutor;

    #[async_trait]
    impl TaskExecutor for NoopExecutor {
        async fn execute(&self, _task: &TaskRecord) -> ExecutionResult {
            ExecutionResult::succeeded(None)
        }
    }

    #[tokio::test]
    async fn heartbeat_or_cancelled_treats_row_not_found_from_cancelled_task_as_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        let store = KanbanStore::new(&dir.path().join("kanban.db"))
            .await
            .unwrap();
        let runtime = WorkerRuntime::new("worker-a".into(), store.clone(), Arc::new(NoopExecutor));
        let claimed = claim_running_task(&store).await;
        assert!(store.cancel_task(&claimed.task.id).await.unwrap());

        let cancelled = runtime
            .heartbeat_or_cancelled(&claimed.task.id, &claimed.claim_lock)
            .await
            .unwrap();

        assert!(cancelled);
    }

    #[tokio::test]
    async fn complete_or_cancelled_treats_row_not_found_from_cancelled_task_as_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        let store = KanbanStore::new(&dir.path().join("kanban.db"))
            .await
            .unwrap();
        let runtime = WorkerRuntime::new("worker-a".into(), store.clone(), Arc::new(NoopExecutor));
        let claimed = claim_running_task(&store).await;
        assert!(store.cancel_task(&claimed.task.id).await.unwrap());

        let cancelled = runtime
            .complete_or_cancelled(
                &claimed.task.id,
                &claimed.run.id,
                &claimed.claim_lock,
                ExecutionResult::succeeded(Some("late success".into())),
            )
            .await
            .unwrap();

        assert!(cancelled);
    }

    async fn claim_running_task(store: &KanbanStore) -> crate::ClaimedTask {
        store
            .create_task(NewTask {
                source: "manual".into(),
                source_id: None,
                title: "Race me".into(),
                body: "body".into(),
                payload_kind: PayloadKind::ShellCommand,
                payload_json: json!({"command": "pwd"}),
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
            .mark_running(&claimed.task.id, &claimed.run.id, &claimed.claim_lock)
            .await
            .unwrap();
        claimed
    }
}
