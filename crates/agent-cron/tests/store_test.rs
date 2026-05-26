use agent_cron::{CronStore, MissedRunPolicy, NewCronJob, PayloadKind, Schedule};
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

#[tokio::test]
async fn trigger_can_be_deleted_for_failed_task_creation() {
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

    store
        .record_trigger(&job.id, scheduled_for, "task-1")
        .await
        .unwrap();
    store.delete_trigger(&job.id, scheduled_for).await.unwrap();

    let replacement = store
        .record_trigger(&job.id, scheduled_for, "task-2")
        .await
        .unwrap();

    assert!(replacement.is_some());
    assert_eq!(replacement.unwrap().task_id, "task-2");
}

#[tokio::test]
async fn legacy_import_missing_file_returns_zero() {
    let dir = tempfile::tempdir().unwrap();
    let store = CronStore::new(&dir.path().join("cron.db")).await.unwrap();

    let imported = store
        .import_legacy_jobs_file(&dir.path().join("missing-jobs.json"))
        .await
        .unwrap();

    assert_eq!(imported, 0);
}

#[tokio::test]
async fn legacy_import_imports_old_job_and_preserves_id() {
    let dir = tempfile::tempdir().unwrap();
    let store = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let legacy_file = dir.path().join("jobs.json");
    std::fs::write(
        &legacy_file,
        serde_json::to_string(&json!([{
            "id": "legacy-job-1",
            "name": "Legacy prompt",
            "description": "imported from jobs.json",
            "schedule": "5m",
            "command": "summarize inbox",
            "workdir": "/tmp",
            "enabled": false,
            "last_run": null,
            "next_run": null
        }]))
        .unwrap(),
    )
    .unwrap();

    let imported = store.import_legacy_jobs_file(&legacy_file).await.unwrap();

    assert_eq!(imported, 1);
    let job = store.load_job("legacy-job-1").await.unwrap().unwrap();
    assert_eq!(job.id, "legacy-job-1");
    assert_eq!(job.name, "Legacy prompt");
    assert_eq!(job.description, "imported from jobs.json");
    assert!(matches!(job.schedule, Schedule::Duration(_)));
    assert_eq!(job.payload_kind, PayloadKind::AgentPrompt);
    assert_eq!(job.payload_json, json!({"prompt": "summarize inbox"}));
    assert!(!job.enabled);
    assert_eq!(job.missed_run_policy, MissedRunPolicy::RunOnce);
}

#[tokio::test]
async fn legacy_import_second_import_does_not_duplicate() {
    let dir = tempfile::tempdir().unwrap();
    let store = CronStore::new(&dir.path().join("cron.db")).await.unwrap();
    let legacy_file = dir.path().join("jobs.json");
    std::fs::write(
        &legacy_file,
        serde_json::to_string(&json!([{
            "id": "legacy-job-1",
            "name": "Legacy prompt",
            "description": "imported from jobs.json",
            "schedule": { "Every": "every 2h" },
            "command": "summarize inbox",
            "enabled": true
        }]))
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        store.import_legacy_jobs_file(&legacy_file).await.unwrap(),
        1
    );
    assert_eq!(
        store.import_legacy_jobs_file(&legacy_file).await.unwrap(),
        0
    );

    let jobs = store.list_jobs().await.unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, "legacy-job-1");
    assert!(matches!(jobs[0].schedule, Schedule::Every(_)));
}
