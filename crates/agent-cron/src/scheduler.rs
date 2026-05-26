use crate::{CronJob, CronStore, MissedRunPolicy, PayloadKind, Schedule};
use agent_kanban::{KanbanStore, NewTask};
use chrono::{DateTime, Utc};
use tokio::time::{Duration, interval};
use uuid::Uuid;

const CATCH_UP_LIMIT: usize = 10;

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

            let policy_tick = missed_run_policy_tick(&job, scheduled_for, now);

            for occurrence in &policy_tick.occurrences {
                if self.process_occurrence(&job, *occurrence).await? {
                    created += 1;
                }
            }

            self.cron_store
                .advance_job_if_next_run(
                    &job.id,
                    scheduled_for,
                    policy_tick.last_run_at,
                    policy_tick.next_run_at,
                )
                .await?;
        }

        Ok(created)
    }

    async fn process_occurrence(
        &self,
        job: &CronJob,
        scheduled_for: DateTime<Utc>,
    ) -> Result<bool, sqlx::Error> {
        let reserved = self
            .cron_store
            .record_trigger(&job.id, scheduled_for, &Uuid::new_v4().to_string())
            .await?;

        if let Some(trigger) = reserved {
            self.create_task_for_trigger(job, scheduled_for, trigger.task_id)
                .await?;
            return Ok(true);
        }

        if let Some(trigger) = self.cron_store.load_trigger(&job.id, scheduled_for).await? {
            if trigger.status == "reserved" {
                self.create_task_for_trigger(job, scheduled_for, trigger.task_id)
                    .await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn create_task_for_trigger(
        &self,
        job: &CronJob,
        scheduled_for: DateTime<Utc>,
        task_id: String,
    ) -> Result<(), sqlx::Error> {
        if self.kanban_store.load_task(&task_id).await?.is_none() {
            if let Err(error) = self
                .kanban_store
                .create_task_with_id(task_id, new_task(job))
                .await
            {
                self.cron_store
                    .delete_trigger(&job.id, scheduled_for)
                    .await?;
                return Err(error);
            }
        }

        self.cron_store
            .mark_trigger_created(&job.id, scheduled_for)
            .await
    }
}

struct PolicyTick {
    occurrences: Vec<DateTime<Utc>>,
    last_run_at: DateTime<Utc>,
    next_run_at: Option<DateTime<Utc>>,
}

fn missed_run_policy_tick(
    job: &CronJob,
    scheduled_for: DateTime<Utc>,
    now: DateTime<Utc>,
) -> PolicyTick {
    match job.missed_run_policy {
        MissedRunPolicy::Skip if scheduled_for < now => PolicyTick {
            occurrences: Vec::new(),
            last_run_at: now,
            next_run_at: job.schedule.next_after(now),
        },
        MissedRunPolicy::CatchUp => catch_up_policy_tick(&job.schedule, scheduled_for, now),
        MissedRunPolicy::Skip | MissedRunPolicy::RunOnce => PolicyTick {
            occurrences: vec![scheduled_for],
            last_run_at: now,
            next_run_at: job.schedule.next_after(now),
        },
    }
}

fn catch_up_policy_tick(
    schedule: &Schedule,
    scheduled_for: DateTime<Utc>,
    now: DateTime<Utc>,
) -> PolicyTick {
    let mut occurrences = vec![scheduled_for];
    let mut last_processed = scheduled_for;

    if let Schedule::Duration(duration) = schedule {
        if let Ok(duration) = chrono::Duration::from_std(*duration) {
            while occurrences.len() < CATCH_UP_LIMIT {
                let Some(next) = last_processed.checked_add_signed(duration) else {
                    break;
                };
                if next > now {
                    break;
                }
                occurrences.push(next);
                last_processed = next;
            }
        }
    }

    let next_from = occurrences.last().copied().unwrap_or(now);
    PolicyTick {
        occurrences,
        last_run_at: next_from,
        next_run_at: schedule.next_after(next_from),
    }
}

fn new_task(job: &CronJob) -> NewTask {
    NewTask {
        source: "cron".into(),
        source_id: Some(job.id.clone()),
        title: job.name.clone(),
        body: job.description.clone(),
        payload_kind: map_payload_kind(job.payload_kind),
        payload_json: job.payload_json.clone(),
        priority: 0,
        max_retries: 1,
    }
}

fn map_payload_kind(kind: PayloadKind) -> agent_kanban::PayloadKind {
    match kind {
        PayloadKind::AgentPrompt => agent_kanban::PayloadKind::AgentPrompt,
        PayloadKind::ShellCommand => agent_kanban::PayloadKind::ShellCommand,
    }
}
