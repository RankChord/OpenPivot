# Automation Security Reliability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the durable automation service with HTTP token auth, cooperative cancellation, automatic retry, and useful diagnostics.

**Architecture:** Keep the existing SQLite-backed Cron/Kanban architecture. Add small focused helpers in `serve.rs` for auth/startup guards, add retry/cancellation helpers in `agent-kanban`, and expand `doctor` diagnostics without introducing external services.

**Tech Stack:** Rust 2024, Axum 0.8, Tokio, SQLx SQLite, Serde, existing `agent-cron`, `agent-kanban`, and `agent-cli` crates.

---

## File Structure

- Modify `crates/agent-cli/src/cmd/serve.rs`: HTTP token auth, non-loopback startup guard, protected-route middleware/helper, auth tests.
- Modify `crates/agent-kanban/src/store.rs`: cancellation state checks, retry scheduling, retry exhaustion events, task count helpers.
- Modify `crates/agent-kanban/src/worker.rs`: retry classification and completion handling.
- Modify `crates/agent-kanban/tests/store_test.rs`: cancellation/retry store behavior tests.
- Modify `crates/agent-kanban/tests/worker_test.rs`: auto-retry/non-retry/cancellation behavior tests.
- Modify `crates/agent-cli/src/cmd/doctor.rs`: automation DB counts, legacy file status, token status.
- Modify `README.md`: security/reliability notes.

## Task 1: HTTP Token Auth And Startup Guard

**Files:**
- Modify: `crates/agent-cli/src/cmd/serve.rs`

- [ ] **Step 1: Write failing auth tests**

Add tests inside `automation_tests` in `serve.rs`:

```rust
#[test]
fn loopback_host_allows_missing_http_token() {
    assert!(validate_http_token_for_host("127.0.0.1", None).is_ok());
    assert!(validate_http_token_for_host("localhost", None).is_ok());
}

#[test]
fn non_loopback_host_requires_http_token() {
    let error = validate_http_token_for_host("0.0.0.0", None).unwrap_err();
    assert!(error.to_string().contains("AGENT_HTTP_TOKEN"));
}

#[test]
fn bearer_token_auth_accepts_valid_token() {
    assert!(authorize_bearer_header(Some("Bearer secret"), Some("secret")).is_ok());
}

#[test]
fn bearer_token_auth_rejects_missing_or_invalid_token() {
    assert!(authorize_bearer_header(None, Some("secret")).is_err());
    assert!(authorize_bearer_header(Some("Bearer wrong"), Some("secret")).is_err());
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p agent-cli automation_tests -- --nocapture`

Expected: FAIL with missing `validate_http_token_for_host` and `authorize_bearer_header`.

- [ ] **Step 3: Implement auth helpers**

Add near API helper functions in `serve.rs`:

```rust
fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn validate_http_token_for_host(host: &str, token: Option<&str>) -> anyhow::Result<()> {
    if is_loopback_host(host) || token.filter(|value| !value.is_empty()).is_some() {
        return Ok(());
    }
    anyhow::bail!("AGENT_HTTP_TOKEN is required when binding Agent Gateway to non-loopback host {}", host)
}

fn unauthorized() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "missing or invalid bearer token".into(),
        }),
    )
}

fn authorize_bearer_header(
    authorization: Option<&str>,
    expected_token: Option<&str>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let Some(expected) = expected_token.filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let Some(header) = authorization else {
        tracing::warn!("http auth rejected: missing bearer token");
        return Err(unauthorized());
    };
    if header == format!("Bearer {}", expected) {
        Ok(())
    } else {
        tracing::warn!("http auth rejected: invalid bearer token");
        Err(unauthorized())
    }
}
```

- [ ] **Step 4: Add token to state and enforce startup guard**

Extend `GatewayState` with:

```rust
pub http_token: Option<String>,
```

Update constructors to set `http_token` from `std::env::var("AGENT_HTTP_TOKEN").ok()` or an explicit parameter if cleaner.

In `run`, before binding:

```rust
let http_token = std::env::var("AGENT_HTTP_TOKEN").ok();
validate_http_token_for_host(host, http_token.as_deref())?;
```

Pass `http_token` into `GatewayState`.

- [ ] **Step 5: Protect route handlers**

For each protected handler, add `headers: axum::http::HeaderMap` and call:

```rust
authorize_bearer_header(
    headers.get(axum::http::header::AUTHORIZATION).and_then(|value| value.to_str().ok()),
    state.http_token.as_deref(),
)?;
```

Protect `/run`, `/run/stream`, `/api/*`, and webhook handlers. Leave `/health`, `/cron`, and `/tasks` unprotected.

- [ ] **Step 6: Run verification**

Run: `cargo test -p agent-cli automation_tests -- --nocapture`

Expected: PASS.

Run: `cargo check -p agent-cli`

Expected: PASS.

## Task 2: Cooperative Cancellation Cannot Succeed Later

**Files:**
- Modify: `crates/agent-kanban/src/store.rs`
- Modify: `crates/agent-kanban/src/worker.rs`
- Modify: `crates/agent-kanban/tests/store_test.rs`
- Modify: `crates/agent-kanban/tests/worker_test.rs`

- [ ] **Step 1: Write failing store test**

Add to `store_test.rs`:

```rust
#[tokio::test]
async fn completing_cancelled_task_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let store = KanbanStore::new(&dir.path().join("kanban.db")).await.unwrap();
    let task = store.create_task(new_shell_task("cancel me")).await.unwrap();
    let claimed = store.claim_next_task("worker", 60).await.unwrap().unwrap();
    store.mark_running(&claimed.task.id, &claimed.run.id, &claimed.claim_lock).await.unwrap();
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
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p agent-kanban --test store_test completing_cancelled_task_is_rejected -- --nocapture`

Expected: FAIL if completion currently overwrites cancellation.

- [ ] **Step 3: Implement cancellation guard**

Ensure `complete_run` only updates tasks with status `claimed` or `running`, never `cancelled`. If already true, add a `task_completion_ignored` event in the failure path is not required for this task; keep behavior minimal.

- [ ] **Step 4: Write worker cancellation test**

Add to `worker_test.rs` a delayed executor test that cancels while running and asserts final status remains cancelled.

- [ ] **Step 5: Run verification**

Run: `cargo test -p agent-kanban --test store_test -- --nocapture`

Expected: PASS.

Run: `cargo test -p agent-kanban --test worker_test -- --nocapture`

Expected: PASS.

## Task 3: Automatic Retry Policy

**Files:**
- Modify: `crates/agent-kanban/src/store.rs`
- Modify: `crates/agent-kanban/src/worker.rs`
- Modify: `crates/agent-kanban/tests/store_test.rs`
- Modify: `crates/agent-kanban/tests/worker_test.rs`

- [ ] **Step 1: Write retry classifier tests**

Add to `worker_test.rs`:

```rust
#[test]
fn retry_classifier_rejects_non_retryable_errors() {
    assert!(!agent_kanban::is_retryable_error("missing shell command"));
    assert!(!agent_kanban::is_retryable_error("permission denied"));
    assert!(!agent_kanban::is_retryable_error("cancelled"));
    assert!(agent_kanban::is_retryable_error("temporary network error"));
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p agent-kanban --test worker_test retry_classifier_rejects_non_retryable_errors -- --nocapture`

Expected: FAIL with missing `is_retryable_error`.

- [ ] **Step 3: Implement classifier**

In `worker.rs`:

```rust
pub fn is_retryable_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    !["missing", "permission", "forbidden", "validation", "cancelled"]
        .iter()
        .any(|marker| error.contains(marker))
}
```

- [ ] **Step 4: Add store retry scheduling method**

Add method:

```rust
pub async fn requeue_for_retry(&self, task_id: &str, previous_error: &str) -> Result<bool, sqlx::Error>
```

It should increment `retry_count`, set status `queued`, clear claim/completion fields, set `last_error`, and append `task_requeued` if `retry_count < max_retries`. If exhausted, append `task_retry_exhausted` and return `Ok(false)`.

- [ ] **Step 5: Add worker auto-retry behavior**

After executor returns failed, if `is_retryable_error` and `requeue_for_retry` returns true, do not call `complete_run` as failed. Leave task queued and finish the current run as failed if needed without changing task status back to failed. Keep implementation minimal; if run/task split is awkward, update run to failed and task to queued in `requeue_for_retry`.

- [ ] **Step 6: Run verification**

Run: `cargo test -p agent-kanban --test worker_test -- --nocapture`

Expected: PASS.

Run: `cargo test -p agent-kanban --test store_test -- --nocapture`

Expected: PASS.

## Task 4: Doctor Automation Diagnostics

**Files:**
- Modify: `crates/agent-cli/src/cmd/doctor.rs`

- [ ] **Step 1: Extract summary helpers and write tests if possible**

Add small helper functions:

```rust
fn automation_db_path() -> std::path::PathBuf
fn legacy_jobs_file_path() -> std::path::PathBuf
fn http_token_status() -> &'static str
```

Add unit tests for path suffixes and token status using environment setup if existing test style allows it.

- [ ] **Step 2: Add DB counts**

In `run`, if automation DB exists, open `CronStore` and `KanbanStore` and print:

```text
Cron Jobs: <count>
Tasks Queued: <count>
Tasks Running: <count>
Tasks Failed: <count>
Tasks Lost: <count>
```

Use existing store list methods; avoid failing doctor if DB open fails, print the error instead.

- [ ] **Step 3: Add legacy/token diagnostics**

Print:

```text
Legacy Cron Jobs File: <path>
Legacy Cron Jobs File Status: Exists|Missing
HTTP Token: Configured|Missing
```

- [ ] **Step 4: Run verification**

Run: `cargo check -p agent-cli`

Expected: PASS.

## Task 5: Documentation And Full Verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README**

Add concise notes under Durable Cron And Kanban:

```markdown
Security and reliability notes:
- Set `AGENT_HTTP_TOKEN` before binding `serve` to a non-loopback host.
- Protected HTTP routes require `Authorization: Bearer <token>` when a token is configured.
- HTTP task/cron creation rejects `shell_command` payloads.
- Cancelled tasks are durable; workers will not record them as succeeded after cancellation is observed.
- Retryable failures can be requeued until `max_retries` is reached.
```

- [ ] **Step 2: Run full verification**

Run: `cargo fmt --all`

Expected: PASS with no output.

Run: `cargo test --workspace`

Expected: PASS.

Run: `cargo build --workspace`

Expected: PASS.

## Self-Review Notes

- Spec coverage: Tasks cover HTTP token auth/startup guard, cooperative cancellation, retry classification/requeue, doctor diagnostics, docs, and full verification.
- Scope: Dashboard redesign, full RBAC, shell sandboxing, and external queues are explicitly out of scope.
- Type consistency: The plan uses existing `TaskStatus`, `CronStore`, `KanbanStore`, `WorkerRuntime`, `ErrorResponse`, and Axum handler patterns already present in the codebase.
