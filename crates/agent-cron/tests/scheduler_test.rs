use agent_cron::{CronScheduler, CronStore, MissedRunPolicy, NewCronJob, PayloadKind, Schedule};
use agent_kanban::{KanbanStore, PayloadKind as KanbanPayloadKind, TaskStatus};
use chrono::{Duration, TimeZone, Utc};
use serde_json::json;

#[tokio::test]
async fn due_cron_job_creates_one_kanban_task_idempotently() {
    let dir = tempfile::tempdir().unwrap();
    let cron = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let kanban = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();

    let scheduled_for = Utc::now() + Duration::seconds(1);
    let job = cron
        .create_job(NewCronJob {
            name: "Run once".into(),
            description: "create a task".into(),
            schedule: Schedule::Once(scheduled_for),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::RunOnce,
        })
        .await
        .unwrap();

    let scheduler = CronScheduler::new(cron.clone(), kanban.clone());

    let now = scheduled_for + Duration::seconds(1);
    assert_eq!(scheduler.tick_once(now).await.unwrap(), 1);
    assert_eq!(scheduler.tick_once(now).await.unwrap(), 0);

    let tasks = kanban.list_tasks(Some(TaskStatus::Queued)).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].source, "cron");
    assert_eq!(tasks[0].source_id.as_deref(), Some(job.id.as_str()));
    assert_eq!(tasks[0].payload_kind, KanbanPayloadKind::ShellCommand);

    let triggers = cron.list_triggers_for_job(&job.id).await.unwrap();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].task_id, tasks[0].id);
    assert_eq!(triggers[0].status, "created");
}

#[tokio::test]
async fn duplicate_created_trigger_does_not_create_task_and_advances_stale_due_job() {
    let dir = tempfile::tempdir().unwrap();
    let cron = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let kanban = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();

    let scheduled_for = Utc::now() + Duration::seconds(1);
    let job = cron
        .create_job(NewCronJob {
            name: "Every second".into(),
            description: "create a task".into(),
            schedule: Schedule::parse("1s").unwrap(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::RunOnce,
        })
        .await
        .unwrap();
    cron.advance_job(&job.id, Utc::now(), Some(scheduled_for))
        .await
        .unwrap();

    let scheduler = CronScheduler::new(cron.clone(), kanban.clone());
    let first_tick = scheduled_for + Duration::seconds(1);
    assert_eq!(scheduler.tick_once(first_tick).await.unwrap(), 1);
    let advanced = cron.load_job(&job.id).await.unwrap().unwrap();
    assert!(advanced.next_run_at.unwrap() > scheduled_for);

    cron.advance_job(&job.id, first_tick, Some(scheduled_for))
        .await
        .unwrap();
    let stale_due_at = cron.load_job(&job.id).await.unwrap().unwrap().next_run_at;
    assert_eq!(
        scheduler
            .tick_once(first_tick + Duration::seconds(10))
            .await
            .unwrap(),
        0
    );

    let tasks = kanban.list_tasks(Some(TaskStatus::Queued)).await.unwrap();
    assert_eq!(tasks.len(), 1);
    let loaded = cron.load_job(&job.id).await.unwrap().unwrap();
    assert!(loaded.next_run_at.unwrap() > stale_due_at.unwrap());
}

#[tokio::test]
async fn created_trigger_with_task_advances_stale_due_job_without_duplicate_task() {
    let dir = tempfile::tempdir().unwrap();
    let cron = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let kanban = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();

    let scheduled_for = Utc::now() + Duration::seconds(1);
    let job = cron
        .create_job(NewCronJob {
            name: "Every second".into(),
            description: "create a task".into(),
            schedule: Schedule::parse("1s").unwrap(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::RunOnce,
        })
        .await
        .unwrap();
    cron.advance_job(&job.id, Utc::now(), Some(scheduled_for))
        .await
        .unwrap();
    let trigger = cron
        .record_trigger(&job.id, scheduled_for, "created-task")
        .await
        .unwrap()
        .unwrap();
    kanban
        .create_task_with_id(
            trigger.task_id,
            agent_kanban::NewTask {
                source: "cron".into(),
                source_id: Some(job.id.clone()),
                title: job.name.clone(),
                body: job.description.clone(),
                payload_kind: KanbanPayloadKind::ShellCommand,
                payload_json: json!({"command": "pwd"}),
                priority: 0,
                max_retries: 1,
            },
        )
        .await
        .unwrap();
    cron.mark_trigger_created(&job.id, scheduled_for)
        .await
        .unwrap();

    let scheduler = CronScheduler::new(cron.clone(), kanban.clone());
    let now = scheduled_for + Duration::seconds(10);
    assert_eq!(scheduler.tick_once(now).await.unwrap(), 0);

    let tasks = kanban.list_tasks(Some(TaskStatus::Queued)).await.unwrap();
    assert_eq!(tasks.len(), 1);
    let loaded = cron.load_job(&job.id).await.unwrap().unwrap();
    assert!(loaded.next_run_at.unwrap() > scheduled_for);
}

#[tokio::test]
async fn reserved_trigger_without_task_is_recovered_with_stored_task_id() {
    let dir = tempfile::tempdir().unwrap();
    let cron = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let kanban = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();

    let scheduled_for = Utc::now() + Duration::seconds(1);
    let job = cron
        .create_job(NewCronJob {
            name: "Recover me".into(),
            description: "create a task".into(),
            schedule: Schedule::Once(scheduled_for),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::RunOnce,
        })
        .await
        .unwrap();
    let trigger = cron
        .record_trigger(&job.id, scheduled_for, "reserved-task")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(trigger.status, "reserved");

    let scheduler = CronScheduler::new(cron.clone(), kanban.clone());
    let now = scheduled_for + Duration::seconds(1);
    assert_eq!(scheduler.tick_once(now).await.unwrap(), 1);

    let tasks = kanban.list_tasks(Some(TaskStatus::Queued)).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, "reserved-task");

    let triggers = cron.list_triggers_for_job(&job.id).await.unwrap();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].status, "created");
    assert_eq!(triggers[0].task_id, "reserved-task");

    let loaded = cron.load_job(&job.id).await.unwrap().unwrap();
    assert_eq!(loaded.next_run_at, None);
}

#[tokio::test]
async fn skip_policy_advances_missed_job_without_creating_task() {
    let dir = tempfile::tempdir().unwrap();
    let cron = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let kanban = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();

    let scheduled_for = Utc.timestamp_opt(Utc::now().timestamp() - 5, 0).unwrap();
    let job = cron
        .create_job(NewCronJob {
            name: "Skip missed".into(),
            description: "do not backfill".into(),
            schedule: Schedule::parse("1s").unwrap(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::Skip,
        })
        .await
        .unwrap();
    cron.advance_job(&job.id, scheduled_for, Some(scheduled_for))
        .await
        .unwrap();

    let scheduler = CronScheduler::new(cron.clone(), kanban.clone());
    let now = scheduled_for + Duration::seconds(5);
    assert_eq!(scheduler.tick_once(now).await.unwrap(), 0);

    let tasks = kanban.list_tasks(Some(TaskStatus::Queued)).await.unwrap();
    assert_eq!(tasks.len(), 0);
    let triggers = cron.list_triggers_for_job(&job.id).await.unwrap();
    assert_eq!(triggers.len(), 0);
    let loaded = cron.load_job(&job.id).await.unwrap().unwrap();
    assert!(loaded.next_run_at.unwrap() > now);
}

#[tokio::test]
async fn run_once_policy_creates_one_task_for_missed_window() {
    let dir = tempfile::tempdir().unwrap();
    let cron = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let kanban = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();

    let scheduled_for = Utc.timestamp_opt(Utc::now().timestamp() - 5, 0).unwrap();
    let job = cron
        .create_job(NewCronJob {
            name: "Run once missed".into(),
            description: "create one backfill".into(),
            schedule: Schedule::parse("1s").unwrap(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::RunOnce,
        })
        .await
        .unwrap();
    cron.advance_job(&job.id, scheduled_for, Some(scheduled_for))
        .await
        .unwrap();

    let scheduler = CronScheduler::new(cron.clone(), kanban.clone());
    let now = scheduled_for + Duration::seconds(5);
    assert_eq!(scheduler.tick_once(now).await.unwrap(), 1);

    let tasks = kanban.list_tasks(Some(TaskStatus::Queued)).await.unwrap();
    assert_eq!(tasks.len(), 1);
    let triggers = cron.list_triggers_for_job(&job.id).await.unwrap();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].scheduled_for, scheduled_for);
    let loaded = cron.load_job(&job.id).await.unwrap().unwrap();
    assert!(loaded.next_run_at.unwrap() > now);
}

#[tokio::test]
async fn catch_up_policy_creates_bounded_tasks_for_missed_intervals() {
    let dir = tempfile::tempdir().unwrap();
    let cron = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let kanban = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();

    let scheduled_for = Utc.timestamp_opt(Utc::now().timestamp() - 20, 0).unwrap();
    let job = cron
        .create_job(NewCronJob {
            name: "Catch up missed".into(),
            description: "backfill missed intervals".into(),
            schedule: Schedule::parse("1s").unwrap(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            enabled: true,
            missed_run_policy: MissedRunPolicy::CatchUp,
        })
        .await
        .unwrap();
    cron.advance_job(&job.id, scheduled_for, Some(scheduled_for))
        .await
        .unwrap();

    let scheduler = CronScheduler::new(cron.clone(), kanban.clone());
    let now = scheduled_for + Duration::seconds(20);
    assert_eq!(scheduler.tick_once(now).await.unwrap(), 10);

    let tasks = kanban.list_tasks(Some(TaskStatus::Queued)).await.unwrap();
    assert_eq!(tasks.len(), 10);
    let triggers = cron.list_triggers_for_job(&job.id).await.unwrap();
    assert_eq!(triggers.len(), 10);
    assert_eq!(triggers[0].scheduled_for, scheduled_for);
    assert_eq!(
        triggers[9].scheduled_for,
        scheduled_for + Duration::seconds(9)
    );
    let loaded = cron.load_job(&job.id).await.unwrap().unwrap();
    assert_eq!(
        loaded.next_run_at,
        Some(scheduled_for + Duration::seconds(10))
    );
}
