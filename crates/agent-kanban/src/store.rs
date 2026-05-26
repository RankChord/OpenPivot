use crate::{
    ClaimedTask, NewTask, PayloadKind, TaskEventRecord, TaskRecord, TaskRunRecord, TaskStatus,
};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone)]
pub struct KanbanStore {
    pool: Arc<SqlitePool>,
}

impl KanbanStore {
    pub async fn new(db_path: &Path) -> Result<Self, sqlx::Error> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| sqlx::Error::Io(error.into()))?;
        }

        let db_url = format!("sqlite://{}", db_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)?
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new().connect_with(options).await?;
        init_schema(&pool).await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub async fn create_task(&self, task: NewTask) -> Result<TaskRecord, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        self.create_task_with_id(id, task).await
    }

    pub async fn create_task_with_id(
        &self,
        id: String,
        task: NewTask,
    ) -> Result<TaskRecord, sqlx::Error> {
        let now = Utc::now();
        let payload_json = serde_json::to_string(&task.payload_json)
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
        let mut tx = self.pool.begin().await?;

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
        .execute(&mut *tx)
        .await?;

        append_task_event_in_tx(
            &mut tx,
            &id,
            None,
            "task_created",
            serde_json::json!({ "title": task.title }),
        )
        .await?;
        tx.commit().await?;

        self.load_task(&id).await?.ok_or(sqlx::Error::RowNotFound)
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
            sqlx::query(
                "SELECT * FROM tasks WHERE status = ? ORDER BY priority DESC, created_at ASC",
            )
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

    pub async fn list_task_runs(&self, task_id: &str) -> Result<Vec<TaskRunRecord>, sqlx::Error> {
        let rows = sqlx::query("SELECT * FROM task_runs WHERE task_id = ? ORDER BY started_at ASC")
            .bind(task_id)
            .fetch_all(&*self.pool)
            .await?;

        rows.into_iter().map(row_to_task_run).collect()
    }

    pub async fn list_task_events(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskEventRecord>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT * FROM task_events WHERE task_id = ? ORDER BY sequence ASC, created_at ASC, id ASC",
        )
        .bind(task_id)
        .fetch_all(&*self.pool)
        .await?;

        rows.into_iter().map(row_to_task_event).collect()
    }

    pub async fn append_task_event(
        &self,
        task_id: &str,
        run_id: Option<&str>,
        event_type: &str,
        payload_json: serde_json::Value,
    ) -> Result<TaskEventRecord, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let locked = sqlx::query("UPDATE tasks SET updated_at = updated_at WHERE id = ?")
            .bind(task_id)
            .execute(&mut *tx)
            .await?;
        if locked.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(sqlx::Error::RowNotFound);
        }

        let event =
            append_task_event_in_tx(&mut tx, task_id, run_id, event_type, payload_json).await?;
        tx.commit().await?;
        Ok(event)
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<bool, sqlx::Error> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE tasks
             SET status = 'cancelled', claim_lock = NULL, claim_expires_at = NULL,
                 assigned_worker = NULL, completed_at = ?, updated_at = ?
             WHERE id = ? AND status IN ('queued', 'claimed', 'running')",
        )
        .bind(now.timestamp())
        .bind(now.timestamp())
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query(
            "UPDATE task_runs
             SET status = 'cancelled', finished_at = ?
             WHERE task_id = ? AND status IN ('claimed', 'running')",
        )
        .bind(now.timestamp())
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

        append_task_event_in_tx(
            &mut tx,
            task_id,
            None,
            "task_cancelled",
            serde_json::json!({}),
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn is_task_cancelled(&self, task_id: &str) -> Result<bool, sqlx::Error> {
        let row = sqlx::query("SELECT status FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(&*self.pool)
            .await?;

        let Some(row) = row else {
            return Err(sqlx::Error::RowNotFound);
        };
        let status: String = row.get("status");
        Ok(status == TaskStatus::Cancelled.as_str())
    }

    pub async fn retry_task(&self, task_id: &str) -> Result<bool, sqlx::Error> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE tasks
             SET status = 'queued', claim_lock = NULL, claim_expires_at = NULL,
                 assigned_worker = NULL, last_heartbeat_at = NULL, completed_at = NULL,
                 last_error = NULL, retry_count = retry_count + 1, updated_at = ?
             WHERE id = ? AND status IN ('failed', 'timed_out', 'lost')
               AND retry_count < max_retries",
        )
        .bind(now.timestamp())
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        append_task_event_in_tx(
            &mut tx,
            task_id,
            None,
            "task_retried",
            serde_json::json!({}),
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn requeue_for_retry(
        &self,
        task_id: &str,
        run_id: &str,
        claim_lock: &str,
        previous_error: &str,
    ) -> Result<bool, sqlx::Error> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE tasks
             SET status = 'queued', claim_lock = NULL, claim_expires_at = NULL,
                 assigned_worker = NULL, last_heartbeat_at = NULL, completed_at = NULL,
                 last_error = ?, retry_count = retry_count + 1, updated_at = ?
             WHERE id = ? AND claim_lock = ? AND status IN ('claimed', 'running')
               AND retry_count < max_retries",
        )
        .bind(previous_error)
        .bind(now.timestamp())
        .bind(task_id)
        .bind(claim_lock)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            let row = sqlx::query(
                "SELECT status, claim_lock, retry_count, max_retries FROM tasks WHERE id = ?",
            )
            .bind(task_id)
            .fetch_optional(&mut *tx)
            .await?;

            let Some(row) = row else {
                tx.rollback().await?;
                return Err(sqlx::Error::RowNotFound);
            };

            let status: String = row.get("status");
            let current_claim_lock: Option<String> = row.get("claim_lock");
            let retry_count: i64 = row.get("retry_count");
            let max_retries: i64 = row.get("max_retries");

            if !matches!(status.as_str(), "claimed" | "running")
                || current_claim_lock.as_deref() != Some(claim_lock)
            {
                tx.rollback().await?;
                return Err(sqlx::Error::RowNotFound);
            }

            if retry_count < max_retries {
                tx.rollback().await?;
                return Err(sqlx::Error::RowNotFound);
            }

            sqlx::query(
                "UPDATE tasks
                 SET status = 'failed', claim_lock = NULL, claim_expires_at = NULL,
                     assigned_worker = NULL, completed_at = ?, updated_at = ?, last_error = ?
                 WHERE id = ? AND claim_lock = ? AND status IN ('claimed', 'running')",
            )
            .bind(now.timestamp())
            .bind(now.timestamp())
            .bind(previous_error)
            .bind(task_id)
            .bind(claim_lock)
            .execute(&mut *tx)
            .await?;

            let run_updated = sqlx::query(
                "UPDATE task_runs
                 SET status = 'failed', finished_at = ?, error = ?
                 WHERE id = ? AND task_id = ? AND status IN ('claimed', 'running')",
            )
            .bind(now.timestamp())
            .bind(previous_error)
            .bind(run_id)
            .bind(task_id)
            .execute(&mut *tx)
            .await?;

            if run_updated.rows_affected() == 0 {
                tx.rollback().await?;
                return Err(sqlx::Error::RowNotFound);
            }

            append_task_event_in_tx(
                &mut tx,
                task_id,
                None,
                "task_retry_exhausted",
                serde_json::json!({ "error": previous_error, "retry_count": retry_count }),
            )
            .await?;
            tx.commit().await?;
            return Ok(false);
        }

        let retry_count: i64 = sqlx::query_scalar("SELECT retry_count FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&mut *tx)
            .await?;

        let run_updated = sqlx::query(
            "UPDATE task_runs
             SET status = 'failed', finished_at = ?, error = ?
             WHERE id = ? AND task_id = ? AND status IN ('claimed', 'running')",
        )
        .bind(now.timestamp())
        .bind(previous_error)
        .bind(run_id)
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

        if run_updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(sqlx::Error::RowNotFound);
        }

        append_task_event_in_tx(
            &mut tx,
            task_id,
            None,
            "task_requeued",
            serde_json::json!({ "error": previous_error, "retry_count": retry_count }),
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn claim_next_task(
        &self,
        worker_id: &str,
        claim_ttl_secs: i64,
    ) -> Result<Option<ClaimedTask>, sqlx::Error> {
        let candidates = sqlx::query(
            "SELECT id FROM tasks
             WHERE status = 'queued'
             ORDER BY priority DESC, created_at ASC
             LIMIT 10",
        )
        .fetch_all(&*self.pool)
        .await?;

        for candidate in candidates {
            let now = Utc::now();
            let expires_at = now + chrono::Duration::seconds(claim_ttl_secs);
            let mut tx = self.pool.begin().await?;
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
            .execute(&mut *tx)
            .await?;

            if updated.rows_affected() == 0 {
                tx.rollback().await?;
                continue;
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
            .execute(&mut *tx)
            .await?;

            append_task_event_in_tx(
                &mut tx,
                &task_id,
                Some(&run_id),
                "task_claimed",
                serde_json::json!({"worker_id": worker_id}),
            )
            .await?;

            let task = sqlx::query("SELECT * FROM tasks WHERE id = ?")
                .bind(&task_id)
                .fetch_optional(&mut *tx)
                .await?
                .map(row_to_task)
                .transpose()?
                .ok_or(sqlx::Error::RowNotFound)?;
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

            tx.commit().await?;
            return Ok(Some(ClaimedTask {
                task,
                run,
                claim_lock,
            }));
        }

        Ok(None)
    }

    pub async fn mark_running(
        &self,
        task_id: &str,
        run_id: &str,
        claim_lock: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE tasks
             SET status = 'running', last_heartbeat_at = ?, updated_at = ?
             WHERE id = ? AND claim_lock = ? AND status = 'claimed'",
        )
        .bind(now.timestamp())
        .bind(now.timestamp())
        .bind(task_id)
        .bind(claim_lock)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        let run_updated = sqlx::query(
            "UPDATE task_runs
             SET status = 'running'
             WHERE id = ? AND task_id = ? AND status = 'claimed'",
        )
        .bind(run_id)
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

        if run_updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(sqlx::Error::RowNotFound);
        }

        append_task_event_in_tx(
            &mut tx,
            task_id,
            Some(run_id),
            "task_running",
            serde_json::json!({}),
        )
        .await?;
        tx.commit().await?;
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
        let updated = sqlx::query(
            "UPDATE tasks
             SET last_heartbeat_at = ?, claim_expires_at = ?, updated_at = ?
             WHERE id = ? AND claim_lock = ? AND status IN ('claimed', 'running')",
        )
        .bind(now.timestamp())
        .bind(expires_at.timestamp())
        .bind(now.timestamp())
        .bind(task_id)
        .bind(claim_lock)
        .execute(&*self.pool)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

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
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE tasks
             SET status = ?, claim_lock = NULL, claim_expires_at = NULL, assigned_worker = NULL,
                 completed_at = ?, updated_at = ?, last_error = ?
             WHERE id = ? AND claim_lock = ? AND status IN ('claimed', 'running')",
        )
        .bind(status.as_str())
        .bind(now.timestamp())
        .bind(now.timestamp())
        .bind(error)
        .bind(task_id)
        .bind(claim_lock)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(sqlx::Error::RowNotFound);
        }

        let run_updated = sqlx::query(
            "UPDATE task_runs
             SET status = ?, finished_at = ?, output_summary = ?, error = ?
             WHERE id = ? AND task_id = ? AND status IN ('claimed', 'running')",
        )
        .bind(status.as_str())
        .bind(now.timestamp())
        .bind(output_summary)
        .bind(error)
        .bind(run_id)
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

        if run_updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(sqlx::Error::RowNotFound);
        }

        append_task_event_in_tx(
            &mut tx,
            task_id,
            Some(run_id),
            "task_completed",
            serde_json::json!({"status": status.as_str(), "error": error}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn recover_expired_claims(&self) -> Result<u64, sqlx::Error> {
        let now = Utc::now();
        let cutoff = now.timestamp();
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query(
            "SELECT id FROM tasks
             WHERE status IN ('claimed', 'running')
               AND claim_expires_at IS NOT NULL
               AND claim_expires_at < ?",
        )
        .bind(cutoff)
        .fetch_all(&mut *tx)
        .await?;

        let mut recovered = 0;
        for row in rows {
            let task_id: String = row.get("id");
            let error = "expired claim without fresh heartbeat";
            let result = sqlx::query(
                "UPDATE tasks
                 SET status = 'lost', claim_lock = NULL, claim_expires_at = NULL,
                     assigned_worker = NULL, last_error = ?, updated_at = ?
                 WHERE id = ? AND status IN ('claimed', 'running')
                   AND claim_expires_at IS NOT NULL
                   AND claim_expires_at < ?",
            )
            .bind(error)
            .bind(cutoff)
            .bind(&task_id)
            .bind(cutoff)
            .execute(&mut *tx)
            .await?;

            if result.rows_affected() > 0 {
                recovered += result.rows_affected();
                sqlx::query(
                    "UPDATE task_runs
                     SET status = 'lost', finished_at = ?, error = ?
                     WHERE task_id = ? AND status IN ('claimed', 'running')",
                )
                .bind(cutoff)
                .bind(error)
                .bind(&task_id)
                .execute(&mut *tx)
                .await?;

                append_task_event_in_tx(
                    &mut tx,
                    &task_id,
                    None,
                    "task_lost",
                    serde_json::json!({"error": error}),
                )
                .await?;
            }
        }

        tx.commit().await?;
        Ok(recovered)
    }
}

async fn append_task_event_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    task_id: &str,
    run_id: Option<&str>,
    event_type: &str,
    payload_json: serde_json::Value,
) -> Result<TaskEventRecord, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let payload = serde_json::to_string(&payload_json)
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let sequence: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sequence) + 1, 0) FROM task_events WHERE task_id = ?",
    )
    .bind(task_id)
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO task_events (id, task_id, run_id, event_type, payload_json, created_at, sequence)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(task_id)
    .bind(run_id)
    .bind(event_type)
    .bind(payload)
    .bind(now.timestamp())
    .bind(sequence)
    .execute(&mut **tx)
    .await?;

    Ok(TaskEventRecord {
        id,
        task_id: task_id.to_string(),
        run_id: run_id.map(str::to_string),
        event_type: event_type.to_string(),
        payload_json,
        created_at: now,
        sequence,
    })
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
            sequence INTEGER NOT NULL,
            FOREIGN KEY(task_id) REFERENCES tasks(id)
        )",
    )
    .execute(pool)
    .await?;

    let has_sequence: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM pragma_table_info('task_events') WHERE name = 'sequence'",
    )
    .fetch_optional(pool)
    .await?;
    if has_sequence.is_none() {
        sqlx::query("ALTER TABLE task_events ADD COLUMN sequence INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await?;
    }

    sqlx::query(
        "UPDATE task_events
         SET sequence = (
             SELECT COUNT(*) - 1
             FROM task_events AS earlier
             WHERE earlier.task_id = task_events.task_id
               AND (
                   earlier.created_at < task_events.created_at
                   OR (earlier.created_at = task_events.created_at AND earlier.id <= task_events.id)
               )
         )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status, priority, created_at)",
    )
    .execute(pool)
    .await?;
    sqlx::query("DROP INDEX IF EXISTS idx_task_events_task")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_task_events_task ON task_events(task_id, sequence, created_at, id)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_task_events_task_sequence ON task_events(task_id, sequence)",
    )
    .execute(pool)
    .await?;

    Ok(())
}

fn row_to_task(row: SqliteRow) -> Result<TaskRecord, sqlx::Error> {
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
        claim_expires_at: optional_ts(row.get("claim_expires_at")).transpose()?,
        assigned_worker: row.get("assigned_worker"),
        last_heartbeat_at: optional_ts(row.get("last_heartbeat_at")).transpose()?,
        retry_count: row.get("retry_count"),
        max_retries: row.get("max_retries"),
        last_error: row.get("last_error"),
        created_at: ts(row.get("created_at"))?,
        updated_at: ts(row.get("updated_at"))?,
        started_at: optional_ts(row.get("started_at")).transpose()?,
        completed_at: optional_ts(row.get("completed_at")).transpose()?,
    })
}

fn row_to_task_run(row: SqliteRow) -> Result<TaskRunRecord, sqlx::Error> {
    let status = TaskStatus::try_from(row.get::<String, _>("status").as_str())
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

    Ok(TaskRunRecord {
        id: row.get("id"),
        task_id: row.get("task_id"),
        worker_id: row.get("worker_id"),
        status,
        started_at: ts(row.get("started_at"))?,
        finished_at: optional_ts(row.get("finished_at")).transpose()?,
        duration_ms: row.get("duration_ms"),
        session_id: row.get("session_id"),
        output_summary: row.get("output_summary"),
        error: row.get("error"),
        log_path: row.get("log_path"),
    })
}

fn row_to_task_event(row: SqliteRow) -> Result<TaskEventRecord, sqlx::Error> {
    let payload_json = serde_json::from_str(&row.get::<String, _>("payload_json"))
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

    Ok(TaskEventRecord {
        id: row.get("id"),
        task_id: row.get("task_id"),
        run_id: row.get("run_id"),
        event_type: row.get("event_type"),
        payload_json,
        created_at: ts(row.get("created_at"))?,
        sequence: row.get("sequence"),
    })
}

fn ts(value: i64) -> Result<DateTime<Utc>, sqlx::Error> {
    Utc.timestamp_opt(value, 0)
        .single()
        .ok_or_else(|| sqlx::Error::Protocol(format!("invalid timestamp: {value}")))
}

fn optional_ts(value: Option<i64>) -> Option<Result<DateTime<Utc>, sqlx::Error>> {
    value.map(ts)
}
