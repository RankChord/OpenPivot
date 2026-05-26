use crate::{CronJob, MissedRunPolicy, NewCronJob, PayloadKind, Schedule};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
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
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub async fn create_job(&self, job: NewCronJob) -> Result<CronJob, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        self.create_job_with_id(id, job).await
    }

    pub async fn create_job_with_id(
        &self,
        id: String,
        job: NewCronJob,
    ) -> Result<CronJob, sqlx::Error> {
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

    pub async fn import_legacy_jobs_file(&self, path: &Path) -> Result<usize, sqlx::Error> {
        if !path.exists() {
            return Ok(0);
        }

        let contents = std::fs::read_to_string(path).map_err(|error| sqlx::Error::Io(error))?;
        let value: Value = serde_json::from_str(&contents)
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
        let jobs = value.as_array().ok_or_else(|| {
            sqlx::Error::Protocol("legacy cron jobs file must be a JSON array".into())
        })?;

        let mut imported = 0;
        for legacy_job in jobs {
            let Some(id) = legacy_job.get("id").and_then(Value::as_str) else {
                continue;
            };
            if self.load_job(id).await?.is_some() {
                continue;
            }

            let Some(schedule) = legacy_job.get("schedule").and_then(parse_legacy_schedule) else {
                continue;
            };
            let Some(command) = legacy_job.get("command").and_then(Value::as_str) else {
                continue;
            };

            self.create_job_with_id(
                id.to_string(),
                NewCronJob {
                    name: legacy_job
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or(command)
                        .to_string(),
                    description: legacy_job
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    schedule,
                    payload_kind: PayloadKind::AgentPrompt,
                    payload_json: serde_json::json!({ "prompt": command }),
                    enabled: legacy_job
                        .get("enabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    missed_run_policy: MissedRunPolicy::RunOnce,
                },
            )
            .await?;
            imported += 1;
        }

        Ok(imported)
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
             ) VALUES (?, ?, ?, ?, ?, 'reserved')",
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
            status: "reserved".into(),
            error: None,
        }))
    }

    pub async fn load_trigger(
        &self,
        cron_job_id: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<Option<CronTriggerRecord>, sqlx::Error> {
        let row =
            sqlx::query("SELECT * FROM cron_triggers WHERE cron_job_id = ? AND scheduled_for = ?")
                .bind(cron_job_id)
                .bind(scheduled_for.timestamp())
                .fetch_optional(&*self.pool)
                .await?;

        row.map(row_to_trigger).transpose()
    }

    pub async fn mark_trigger_created(
        &self,
        cron_job_id: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE cron_triggers SET status = 'created', error = NULL WHERE cron_job_id = ? AND scheduled_for = ?",
        )
        .bind(cron_job_id)
        .bind(scheduled_for.timestamp())
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_triggers_for_job(
        &self,
        cron_job_id: &str,
    ) -> Result<Vec<CronTriggerRecord>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT * FROM cron_triggers WHERE cron_job_id = ? ORDER BY scheduled_for ASC",
        )
        .bind(cron_job_id)
        .fetch_all(&*self.pool)
        .await?;
        rows.into_iter().map(row_to_trigger).collect()
    }

    pub async fn delete_trigger(
        &self,
        cron_job_id: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM cron_triggers WHERE cron_job_id = ? AND scheduled_for = ?")
            .bind(cron_job_id)
            .bind(scheduled_for.timestamp())
            .execute(&*self.pool)
            .await?;
        Ok(())
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

    pub async fn advance_job_if_next_run(
        &self,
        job_id: &str,
        expected_next_run_at: DateTime<Utc>,
        last_run_at: DateTime<Utc>,
        next_run_at: Option<DateTime<Utc>>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE cron_jobs SET last_run_at = ?, next_run_at = ?, updated_at = ?
             WHERE id = ? AND next_run_at = ?",
        )
        .bind(last_run_at.timestamp())
        .bind(next_run_at.map(|dt| dt.timestamp()))
        .bind(Utc::now().timestamp())
        .bind(job_id)
        .bind(expected_next_run_at.timestamp())
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

pub fn default_legacy_jobs_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("cron")
        .join("jobs.json")
}

fn parse_legacy_schedule(value: &Value) -> Option<Schedule> {
    if let Some(schedule) = value
        .as_str()
        .and_then(|schedule| Schedule::parse(schedule).ok())
    {
        return Some(schedule);
    }

    serde_json::from_value(value.clone()).ok()
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

fn row_to_trigger(row: sqlx::sqlite::SqliteRow) -> Result<CronTriggerRecord, sqlx::Error> {
    Ok(CronTriggerRecord {
        id: row.get("id"),
        cron_job_id: row.get("cron_job_id"),
        scheduled_for: ts(row.get("scheduled_for")),
        triggered_at: ts(row.get("triggered_at")),
        task_id: row.get("task_id"),
        status: row.get("status"),
        error: row.get("error"),
    })
}

fn ts(value: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(value, 0).single().unwrap()
}

fn optional_ts(value: Option<i64>) -> Option<DateTime<Utc>> {
    value.map(ts)
}
