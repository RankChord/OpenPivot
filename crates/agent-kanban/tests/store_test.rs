use agent_kanban::{KanbanStore, NewTask, PayloadKind, TaskStatus};
use serde_json::json;

#[tokio::test]
async fn lists_task_events_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Event order".into(),
            body: "body".into(),
            payload_kind: PayloadKind::AgentPrompt,
            payload_json: json!({"prompt": "say hello"}),
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

#[tokio::test]
async fn lists_rapid_task_events_by_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Rapid events".into(),
            body: "body".into(),
            payload_kind: PayloadKind::AgentPrompt,
            payload_json: json!({"prompt": "say hello"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    for index in 0..5 {
        store
            .append_task_event(
                &task.id,
                None,
                &format!("agent_event_{index}"),
                json!({"index": index}),
            )
            .await
            .unwrap();
    }

    let events = store.list_task_events(&task.id).await.unwrap();

    assert_eq!(events.len(), 6);
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.sequence, index as i64);
    }
    assert_eq!(events[0].event_type, "task_created");
    for index in 0..5 {
        assert_eq!(events[index + 1].event_type, format!("agent_event_{index}"));
    }
}

#[tokio::test]
async fn concurrent_task_event_appends_get_unique_ordered_sequences() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Concurrent events".into(),
            body: "body".into(),
            payload_kind: PayloadKind::AgentPrompt,
            payload_json: json!({"prompt": "say hello"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    let mut handles = Vec::new();
    for index in 0..25 {
        let store = store.clone();
        let task_id = task.id.clone();
        handles.push(tokio::spawn(async move {
            store
                .append_task_event(
                    &task_id,
                    None,
                    &format!("agent_event_{index}"),
                    json!({"index": index}),
                )
                .await
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    let events = store.list_task_events(&task.id).await.unwrap();
    let sequences: Vec<i64> = events.iter().map(|event| event.sequence).collect();
    let mut unique_sequences = sequences.clone();
    unique_sequences.sort_unstable();
    unique_sequences.dedup();

    assert_eq!(events.len(), 26);
    assert_eq!(unique_sequences.len(), 26);
    assert_eq!(sequences, (0..26).collect::<Vec<_>>());
}

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
async fn creates_task_with_caller_provided_id() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();

    let task = store
        .create_task_with_id(
            "task-from-cron".into(),
            NewTask {
                source: "cron".into(),
                source_id: Some("cron-job".into()),
                title: "Demo".into(),
                body: "Run demo".into(),
                payload_kind: PayloadKind::ShellCommand,
                payload_json: json!({"command": "pwd"}),
                priority: 0,
                max_retries: 1,
            },
        )
        .await
        .unwrap();

    assert_eq!(task.id, "task-from-cron");
    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(task.payload_kind, PayloadKind::ShellCommand);

    let events = store.list_task_events(&task.id).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "task_created");
    assert_eq!(events[0].task_id, "task-from-cron");
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
            priority: 1,
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

#[tokio::test]
async fn claim_is_atomic_and_creates_run() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Claim me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 1,
            max_retries: 0,
        })
        .await
        .unwrap();

    let first = store
        .claim_next_task("worker-a", 60)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.task.id, task.id);
    assert_eq!(first.task.status, TaskStatus::Claimed);
    assert_eq!(first.task.assigned_worker.as_deref(), Some("worker-a"));
    assert_eq!(
        first.task.claim_lock.as_deref(),
        Some(first.claim_lock.as_str())
    );
    assert!(first.task.claim_expires_at.is_some());
    assert!(first.task.started_at.is_some());
    assert_eq!(first.run.task_id, task.id);
    assert_eq!(first.run.status, TaskStatus::Claimed);

    let second = store.claim_next_task("worker-b", 60).await.unwrap();
    assert!(second.is_none());
}

#[tokio::test]
async fn second_claim_can_claim_another_queued_task() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let first_task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "First".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 1,
            max_retries: 0,
        })
        .await
        .unwrap();
    let second_task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Second".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    let first_claim = store
        .claim_next_task("worker-a", 60)
        .await
        .unwrap()
        .unwrap();
    let second_claim = store
        .claim_next_task("worker-b", 60)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(first_claim.task.id, first_task.id);
    assert_eq!(second_claim.task.id, second_task.id);
}

#[tokio::test]
async fn heartbeat_and_completion_update_task_and_run() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
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

    let claimed = store
        .claim_next_task("worker-a", 60)
        .await
        .unwrap()
        .unwrap();
    store
        .mark_running(&claimed.task.id, &claimed.run.id, &claimed.claim_lock)
        .await
        .unwrap();
    store
        .heartbeat(&claimed.task.id, &claimed.claim_lock, 60)
        .await
        .unwrap();
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
    assert_eq!(loaded.claim_expires_at, None);
    assert_eq!(loaded.assigned_worker, None);
    assert_eq!(loaded.last_error, None);

    let runs = store.list_task_runs(&claimed.task.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, TaskStatus::Succeeded);
    assert!(runs[0].finished_at.is_some());
    assert_eq!(runs[0].output_summary.as_deref(), Some("ok"));
}

#[tokio::test]
async fn stale_claim_lock_does_not_mark_running_or_completed() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Guard me".into(),
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

    let running_error = store
        .mark_running(&claimed.task.id, &claimed.run.id, "stale-lock")
        .await
        .unwrap_err();
    assert!(matches!(running_error, sqlx::Error::RowNotFound));

    let task = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(task.status, TaskStatus::Claimed);
    let runs = store.list_task_runs(&claimed.task.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, TaskStatus::Claimed);
    assert_eq!(runs[0].finished_at, None);

    let complete_error = store
        .complete_run(
            &claimed.task.id,
            &claimed.run.id,
            "stale-lock",
            TaskStatus::Succeeded,
            Some("should not apply"),
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(complete_error, sqlx::Error::RowNotFound));

    let task = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(task.status, TaskStatus::Claimed);
    assert!(task.completed_at.is_none());
    let runs = store.list_task_runs(&claimed.task.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, TaskStatus::Claimed);
    assert_eq!(runs[0].finished_at, None);
    assert_eq!(runs[0].output_summary, None);
}

#[tokio::test]
async fn stale_claim_lock_heartbeat_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Heartbeat guard".into(),
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
    let heartbeat_error = store
        .heartbeat(&claimed.task.id, "stale-lock", 60)
        .await
        .unwrap_err();

    assert!(matches!(heartbeat_error, sqlx::Error::RowNotFound));
}

#[tokio::test]
async fn wrong_run_id_does_not_leave_task_running_or_completed() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Run guard".into(),
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
    let running_error = store
        .mark_running(&claimed.task.id, "missing-run", &claimed.claim_lock)
        .await
        .unwrap_err();
    assert!(matches!(running_error, sqlx::Error::RowNotFound));

    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Claimed);
    let runs = store.list_task_runs(&claimed.task.id).await.unwrap();
    assert_eq!(runs[0].status, TaskStatus::Claimed);

    store
        .mark_running(&claimed.task.id, &claimed.run.id, &claimed.claim_lock)
        .await
        .unwrap();
    let complete_error = store
        .complete_run(
            &claimed.task.id,
            "missing-run",
            &claimed.claim_lock,
            TaskStatus::Succeeded,
            Some("should not apply"),
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(complete_error, sqlx::Error::RowNotFound));

    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Running);
    assert!(loaded.completed_at.is_none());
    assert_eq!(
        loaded.claim_lock.as_deref(),
        Some(claimed.claim_lock.as_str())
    );
    let runs = store.list_task_runs(&claimed.task.id).await.unwrap();
    assert_eq!(runs[0].status, TaskStatus::Running);
    assert_eq!(runs[0].finished_at, None);
    assert_eq!(runs[0].output_summary, None);
}

#[tokio::test]
async fn expired_claim_can_be_recovered_to_lost() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
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

    let claimed = store
        .claim_next_task("worker-a", -1)
        .await
        .unwrap()
        .unwrap();
    let recovered = store.recover_expired_claims().await.unwrap();
    assert_eq!(recovered, 1);

    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Lost);
    assert_eq!(loaded.claim_lock, None);
    assert_eq!(loaded.claim_expires_at, None);
    assert_eq!(loaded.assigned_worker, None);
    assert!(loaded.last_error.unwrap().contains("expired claim"));

    let runs = store.list_task_runs(&claimed.task.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, TaskStatus::Lost);
    assert!(runs[0].finished_at.is_some());
    assert!(runs[0].error.as_deref().unwrap().contains("expired claim"));
}

#[tokio::test]
async fn cancel_task_marks_active_task_cancelled_and_records_event() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Cancel me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();

    let cancelled = store.cancel_task(&task.id).await.unwrap();

    assert!(cancelled);
    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Cancelled);
    assert!(loaded.completed_at.is_some());
    assert_eq!(loaded.claim_lock, None);
    assert_eq!(loaded.claim_expires_at, None);
    assert_eq!(loaded.assigned_worker, None);

    let events = store.list_task_events(&task.id).await.unwrap();
    assert_eq!(events.last().unwrap().event_type, "task_cancelled");
}

#[tokio::test]
async fn completing_cancelled_task_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    let task = store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "cancel me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 0,
        })
        .await
        .unwrap();
    let claimed = store.claim_next_task("worker", 60).await.unwrap().unwrap();
    store
        .mark_running(&claimed.task.id, &claimed.run.id, &claimed.claim_lock)
        .await
        .unwrap();
    assert!(store.cancel_task(&task.id).await.unwrap());

    let result = store
        .complete_run(
            &claimed.task.id,
            &claimed.run.id,
            &claimed.claim_lock,
            TaskStatus::Succeeded,
            Some("ok"),
            None,
        )
        .await;

    assert!(matches!(result, Err(sqlx::Error::RowNotFound)));
    let loaded = store.load_task(&task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Cancelled);
}

#[tokio::test]
async fn cancel_task_returns_false_for_completed_task() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Finish first".into(),
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
        .complete_run(
            &claimed.task.id,
            &claimed.run.id,
            &claimed.claim_lock,
            TaskStatus::Succeeded,
            None,
            None,
        )
        .await
        .unwrap();

    let cancelled = store.cancel_task(&claimed.task.id).await.unwrap();

    assert!(!cancelled);
}

#[tokio::test]
async fn retry_task_requeues_failed_task_and_records_event() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Retry me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 2,
        })
        .await
        .unwrap();
    let claimed = store
        .claim_next_task("worker-a", 60)
        .await
        .unwrap()
        .unwrap();
    store
        .complete_run(
            &claimed.task.id,
            &claimed.run.id,
            &claimed.claim_lock,
            TaskStatus::Failed,
            None,
            Some("boom"),
        )
        .await
        .unwrap();

    let retried = store.retry_task(&claimed.task.id).await.unwrap();

    assert!(retried);
    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Queued);
    assert_eq!(loaded.retry_count, 1);
    assert_eq!(loaded.last_error, None);
    assert_eq!(loaded.completed_at, None);
    assert_eq!(loaded.claim_lock, None);
    assert_eq!(loaded.claim_expires_at, None);
    assert_eq!(loaded.assigned_worker, None);

    let events = store.list_task_events(&claimed.task.id).await.unwrap();
    assert_eq!(events.last().unwrap().event_type, "task_retried");
}

#[tokio::test]
async fn retry_task_returns_false_when_max_retries_reached() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Retry once".into(),
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
        .complete_run(
            &claimed.task.id,
            &claimed.run.id,
            &claimed.claim_lock,
            TaskStatus::Failed,
            None,
            Some("boom"),
        )
        .await
        .unwrap();

    let retried = store.retry_task(&claimed.task.id).await.unwrap();

    assert!(!retried);
    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Failed);
    assert_eq!(loaded.retry_count, 0);
}

#[tokio::test]
async fn requeue_for_retry_requeues_running_task_and_records_event() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Auto retry me".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 2,
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

    let requeued = store
        .requeue_for_retry(
            &claimed.task.id,
            &claimed.run.id,
            &claimed.claim_lock,
            "temporary network error",
        )
        .await
        .unwrap();

    assert!(requeued);
    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Queued);
    assert_eq!(loaded.retry_count, 1);
    assert_eq!(
        loaded.last_error.as_deref(),
        Some("temporary network error")
    );
    assert_eq!(loaded.completed_at, None);
    assert_eq!(loaded.claim_lock, None);
    assert_eq!(loaded.claim_expires_at, None);
    assert_eq!(loaded.assigned_worker, None);

    let runs = store.list_task_runs(&claimed.task.id).await.unwrap();
    assert_eq!(runs[0].status, TaskStatus::Failed);
    assert!(runs[0].finished_at.is_some());
    assert_eq!(runs[0].error.as_deref(), Some("temporary network error"));

    let events = store.list_task_events(&claimed.task.id).await.unwrap();
    let event = events.last().unwrap();
    assert_eq!(event.event_type, "task_requeued");
    assert_eq!(event.payload_json["retry_count"], 1);
    assert_eq!(event.payload_json["error"], "temporary network error");
}

#[tokio::test]
async fn requeue_for_retry_records_exhaustion_when_max_retries_reached() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Exhaust retry".into(),
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

    let requeued = store
        .requeue_for_retry(
            &claimed.task.id,
            &claimed.run.id,
            &claimed.claim_lock,
            "temporary network error",
        )
        .await
        .unwrap();

    assert!(!requeued);
    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Failed);
    assert_eq!(loaded.retry_count, 0);
    assert_eq!(
        loaded.last_error.as_deref(),
        Some("temporary network error")
    );

    let runs = store.list_task_runs(&claimed.task.id).await.unwrap();
    assert_eq!(runs[0].status, TaskStatus::Failed);
    assert!(runs[0].finished_at.is_some());
    assert_eq!(runs[0].error.as_deref(), Some("temporary network error"));

    let events = store.list_task_events(&claimed.task.id).await.unwrap();
    let event = events.last().unwrap();
    assert_eq!(event.event_type, "task_retry_exhausted");
    assert_eq!(event.payload_json["retry_count"], 0);
    assert_eq!(event.payload_json["error"], "temporary network error");
}

#[tokio::test]
async fn stale_claim_lock_cannot_requeue_or_record_retry_event() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db"))
        .await
        .unwrap();
    store
        .create_task(NewTask {
            source: "manual".into(),
            source_id: None,
            title: "Protect retry ownership".into(),
            body: "body".into(),
            payload_kind: PayloadKind::ShellCommand,
            payload_json: json!({"command": "pwd"}),
            priority: 0,
            max_retries: 2,
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

    let error = store
        .requeue_for_retry(
            &claimed.task.id,
            &claimed.run.id,
            "stale-claim-lock",
            "temporary network error",
        )
        .await
        .unwrap_err();

    assert!(matches!(error, sqlx::Error::RowNotFound));
    let loaded = store.load_task(&claimed.task.id).await.unwrap().unwrap();
    assert_eq!(loaded.status, TaskStatus::Running);
    assert_eq!(loaded.retry_count, 0);
    assert_eq!(loaded.last_error, None);

    let runs = store.list_task_runs(&claimed.task.id).await.unwrap();
    assert_eq!(runs[0].status, TaskStatus::Running);
    assert_eq!(runs[0].finished_at, None);
    assert_eq!(runs[0].error, None);

    let events = store.list_task_events(&claimed.task.id).await.unwrap();
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == "task_requeued"
                || event.event_type == "task_retry_exhausted")
    );
}
