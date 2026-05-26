# Automation Security And Reliability Design

## Goal

Strengthen the durable Cron and Kanban automation layer so it can run safely for long periods and avoid exposing execution APIs without an explicit security boundary. The work focuses on HTTP token authentication, safer task cancellation, automatic retry rules, and diagnostics for automation state.

## Non-Goals

- Do not add a full user system or RBAC.
- Do not expose HTTP shell task creation.
- Do not replace SQLite with an external queue.
- Do not redesign the dashboard UI in this batch.
- Do not implement hard process isolation or sandboxing in this batch.

## Recommended Approach

Use a small reliability and security hardening layer around the existing local automation system.

HTTP APIs should require a bearer token when the server is exposed beyond loopback. The existing shell execution path remains available to internal cron/tool paths, but direct HTTP creation of shell payloads remains forbidden. Worker cancellation should become cooperative: cancellation updates durable state and the worker observes that state before completing work. Retry should become explicit and conservative, using persisted retry counters and task events. Doctor should report enough automation health to diagnose stuck jobs, failed tasks, and legacy migration state.

## HTTP Security

Authentication uses a single bearer token from `AGENT_HTTP_TOKEN`.

Protected routes:

- `/run`
- `/run/stream`
- `/api/*`
- `/webhook/*`

Unprotected routes:

- `/health`

Dashboard routes `/cron` and `/tasks` may remain visible for local use, but their API calls still require a token when auth is active. Future UI work can add token entry/storage.

Startup policy:

- If the server binds to `127.0.0.1` or `localhost`, token auth is optional.
- If the server binds to `0.0.0.0`, `::`, or any non-loopback host, `AGENT_HTTP_TOKEN` is required.
- If a non-loopback host is requested without a token, startup fails with a clear error.

Request policy:

- If a token is configured, protected routes require `Authorization: Bearer <token>`.
- Missing or invalid credentials return `401 Unauthorized` with structured JSON.
- Auth failures are logged.

## Cancellation

Cancellation remains durable and cooperative.

`POST /api/tasks/{id}/cancel` records a cancellation request by setting task status to `cancelled` and appending a `task_cancelled` event. A worker that is executing the task must observe cancellation before writing success.

First implementation semantics:

- Before starting execution, the worker checks whether the task is cancelled.
- During long execution, the worker heartbeat path can check cancellation before each heartbeat.
- If cancellation is observed, the worker completes the run as `cancelled` or returns without overwriting the cancelled task as `succeeded`.
- Shell child process termination is desirable, but if it is too invasive for the first pass, the minimum guarantee is that a cancelled task cannot later be recorded as succeeded.

The durable event stream should include:

- `task_cancelled` when the API requests cancellation.
- `task_completion_ignored` or equivalent if a worker discovers that task state changed before completion.

## Retry Policy

Retry is conservative and event-driven.

Automatic retry happens only for failures that are likely transient. A task should not auto-retry when its error indicates validation, missing input, permission denial, forbidden payload, or cancellation.

Default classification:

- Not retryable if error contains `missing`, `permission`, `forbidden`, `validation`, or `cancelled`, case-insensitive.
- Retryable otherwise, while `retry_count < max_retries`.

When a retry is scheduled:

- `retry_count` increments.
- status returns to `queued`.
- claim fields and completion fields are cleared.
- `last_error` stores the previous error.
- a `task_requeued` event records the reason and retry count.

When retries are exhausted:

- the terminal status remains `failed`, `timed_out`, or `lost`.
- a `task_retry_exhausted` event records the final error and retry count.

Manual `POST /api/tasks/{id}/retry` keeps its current behavior but should return a clear non-success response when the retry limit is reached.

## Diagnostics

`agent doctor` should report automation state:

- Automation DB path.
- Cron job count.
- Task counts grouped by status.
- Legacy jobs file path.
- Whether legacy file exists.
- HTTP token status: configured or missing.

Logs should include:

- HTTP auth rejects.
- Shell payload HTTP rejects.
- Task cancel requests.
- Task auto-requeues.
- Retry exhaustion.
- Legacy import count and skipped malformed jobs when available.

## Error Handling

Important cases:

- Non-loopback serve without token: fail startup.
- Missing token on protected route: `401`.
- Invalid token on protected route: `401`.
- Task cancellation for a missing task: `404`.
- Task retry limit reached: prefer `409 Conflict`.
- Retry classification should be deterministic and tested.

## Testing

Use TDD for every behavior change. Minimum tests:

- Auth accepts valid bearer token.
- Auth rejects missing and invalid bearer token.
- Non-loopback bind without token fails startup guard.
- Loopback bind without token passes startup guard.
- HTTP shell payload rejection still works with auth changes.
- Cancelled task cannot be completed as succeeded by a worker.
- Retryable failure requeues while retry budget remains.
- Non-retryable failure remains failed.
- Retry exhaustion appends an event.
- Doctor summary functions report automation DB, token status, and counts.

Final verification must run:

- `cargo fmt --all`
- `cargo test --workspace`
- `cargo build --workspace`

## Success Criteria

- A server bound to a non-loopback host cannot start without `AGENT_HTTP_TOKEN`.
- Protected routes reject missing or invalid credentials when a token is configured.
- HTTP clients still cannot create shell-command tasks or cron jobs.
- A cancelled task is not later recorded as succeeded.
- Retryable failures can return to queued until retry budget is exhausted.
- Non-retryable failures do not loop.
- `agent doctor` reports useful automation health.
- Full workspace formatting, tests, and build pass.
