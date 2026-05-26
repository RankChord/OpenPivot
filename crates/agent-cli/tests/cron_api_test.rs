use agent_cli::cmd::serve::{CronJobResponse, TaskEventResponse, TaskResponse, TaskRunResponse};
use agent_cron::{CronJob, MissedRunPolicy, PayloadKind as CronPayloadKind, Schedule};
use agent_kanban::{
    PayloadKind as TaskPayloadKind, TaskEventRecord, TaskRecord, TaskRunRecord, TaskStatus,
};
use chrono::{TimeZone, Utc};

#[test]
fn cron_job_response_contains_expected_fields() {
    let next_run_at = Utc.with_ymd_and_hms(2026, 5, 25, 12, 0, 0).unwrap();
    let last_run_at = Utc.with_ymd_and_hms(2026, 5, 25, 11, 0, 0).unwrap();
    let job = CronJob {
        id: "job-1".into(),
        name: "Nightly".into(),
        description: "Run every night".into(),
        schedule: Schedule::Cron("0 0 * * *".into()),
        payload_kind: CronPayloadKind::AgentPrompt,
        payload_json: serde_json::json!({"prompt": "go"}),
        enabled: true,
        missed_run_policy: MissedRunPolicy::Skip,
        last_run_at: Some(last_run_at),
        next_run_at: Some(next_run_at),
        created_at: last_run_at,
        updated_at: next_run_at,
    };

    let json = serde_json::to_value(CronJobResponse::from(job)).unwrap();

    assert_eq!(json["id"], "job-1");
    assert_eq!(json["name"], "Nightly");
    assert_eq!(json["description"], "Run every night");
    assert_eq!(json["enabled"], true);
    assert_eq!(json["schedule"], serde_json::json!({"Cron": "0 0 * * *"}));
    assert_eq!(json["payload_kind"], "agent_prompt");
    assert_eq!(json["payload_json"], serde_json::json!({"prompt": "go"}));
    assert_eq!(json["missed_run_policy"], "skip");
    assert_eq!(json["next_run_at"], "2026-05-25T12:00:00Z");
    assert_eq!(json["last_run_at"], "2026-05-25T11:00:00Z");
    assert_eq!(json["created_at"], "2026-05-25T11:00:00Z");
    assert_eq!(json["updated_at"], "2026-05-25T12:00:00Z");
}

#[test]
fn task_response_contains_expected_fields() {
    let created_at = Utc.with_ymd_and_hms(2026, 5, 25, 10, 0, 0).unwrap();
    let updated_at = Utc.with_ymd_and_hms(2026, 5, 25, 10, 30, 0).unwrap();
    let task = TaskRecord {
        id: "task-1".into(),
        source: "cron".into(),
        source_id: Some("job-1".into()),
        title: "Run Nightly".into(),
        body: "body".into(),
        payload_kind: TaskPayloadKind::AgentPrompt,
        payload_json: serde_json::json!({"prompt": "go"}),
        status: TaskStatus::Failed,
        priority: 10,
        claim_lock: None,
        claim_expires_at: None,
        assigned_worker: None,
        last_heartbeat_at: None,
        retry_count: 1,
        max_retries: 3,
        last_error: Some("boom".into()),
        created_at,
        updated_at,
        started_at: None,
        completed_at: None,
    };

    let json = serde_json::to_value(TaskResponse::from(task)).unwrap();

    assert_eq!(json["id"], "task-1");
    assert_eq!(json["source"], "cron");
    assert_eq!(json["source_id"], "job-1");
    assert_eq!(json["title"], "Run Nightly");
    assert_eq!(json["status"], "failed");
    assert_eq!(json["last_error"], "boom");
    assert_eq!(json["created_at"], "2026-05-25T10:00:00Z");
    assert_eq!(json["updated_at"], "2026-05-25T10:30:00Z");
}

#[test]
fn task_run_response_contains_expected_fields() {
    let started_at = Utc.with_ymd_and_hms(2026, 5, 25, 10, 0, 0).unwrap();
    let finished_at = Utc.with_ymd_and_hms(2026, 5, 25, 10, 1, 0).unwrap();
    let run = TaskRunRecord {
        id: "run-1".into(),
        task_id: "task-1".into(),
        worker_id: "worker-a".into(),
        status: TaskStatus::Succeeded,
        started_at,
        finished_at: Some(finished_at),
        duration_ms: Some(60_000),
        session_id: Some("session-1".into()),
        output_summary: Some("ok".into()),
        error: None,
        log_path: Some("/tmp/run.log".into()),
    };

    let json = serde_json::to_value(TaskRunResponse::from(run)).unwrap();

    assert_eq!(json["id"], "run-1");
    assert_eq!(json["task_id"], "task-1");
    assert_eq!(json["worker_id"], "worker-a");
    assert_eq!(json["status"], "succeeded");
    assert_eq!(json["started_at"], "2026-05-25T10:00:00Z");
    assert_eq!(json["finished_at"], "2026-05-25T10:01:00Z");
    assert_eq!(json["duration_ms"], 60_000);
    assert_eq!(json["session_id"], "session-1");
    assert_eq!(json["output_summary"], "ok");
    assert_eq!(json["error"], serde_json::Value::Null);
    assert_eq!(json["log_path"], "/tmp/run.log");
}

#[test]
fn task_event_response_contains_expected_fields() {
    let created_at = Utc.with_ymd_and_hms(2026, 5, 25, 10, 2, 0).unwrap();
    let event = TaskEventRecord {
        id: "event-1".into(),
        task_id: "task-1".into(),
        run_id: Some("run-1".into()),
        event_type: "agent_event".into(),
        payload_json: serde_json::json!({"type": "final", "content": "done"}),
        created_at,
        sequence: 3,
    };

    let json = serde_json::to_value(TaskEventResponse::from(event)).unwrap();

    assert_eq!(json["id"], "event-1");
    assert_eq!(json["task_id"], "task-1");
    assert_eq!(json["run_id"], "run-1");
    assert_eq!(json["event_type"], "agent_event");
    assert_eq!(
        json["payload_json"],
        serde_json::json!({"type": "final", "content": "done"})
    );
    assert_eq!(json["created_at"], "2026-05-25T10:02:00Z");
    assert_eq!(json["sequence"], 3);
}
