# Stable Cron And Kanban Architecture Design

## Goal

Build a long-running, locally hosted automation layer for the Rust agent workspace. The system must keep cron schedules, background tasks, agent runs, logs, and UI state available across process restarts. Cron should answer "when should work start" while Kanban should answer "how is work executed, recovered, retried, and inspected".

## Non-Goals

- Do not make cron jobs directly run long agent tasks as the durable path.
- Do not depend on an external queue service for the first version.
- Do not require a full React dashboard before the backend state model is stable.
- Do not claim distributed execution until multiple worker processes are tested with durable claiming.

## Recommended Approach

Use OpenClaw's reliability goals as the target architecture, Hermes-style local implementation as the first delivery path, and a Kanban task ledger as the durable execution boundary.

This means cron jobs create durable tasks instead of directly running agent work. Workers claim tasks from SQLite, execute them through the existing agent runtime, write events and heartbeats, and update task state. The dashboard reads the same stores used by the worker, so what the user sees reflects recoverable state rather than in-memory scheduler state.

## Architecture

The architecture has six bounded units:

- `CronStore`: persists cron job definitions, next run times, and trigger history.
- `CronScheduler`: periodically scans due jobs and creates exactly one task per job occurrence.
- `KanbanStore`: persists tasks, task runs, task events, claim locks, heartbeats, retry data, and current status.
- `WorkerRuntime`: claims queued tasks, executes them, refreshes heartbeats, handles timeout/lost recovery, and records run outcomes.
- `AgentExecutor`: converts a task into `AgentInput`, calls `Agent::run_with_events`, and streams `AgentEvent`s into the task event log.
- `Dashboard/API`: exposes cron jobs, tasks, runs, events, logs, and operational controls.

The first implementation can keep these units inside existing crates to avoid churn, but the boundaries should be explicit. `agent-cron` should own cron schedules and triggering. A new `agent-kanban` crate, or a focused module if crate creation is delayed, should own durable task storage and worker state transitions. `agent-cli::cmd::serve` should compose the stores, scheduler, worker runtime, API, and HTML dashboard.

## Data Model

Use SQLite for durable runtime state. Existing JSON cron files may be read as an import or compatibility source, but the stable system should treat SQLite as authoritative for scheduling and execution state.

`cron_jobs` stores user-defined schedules:

- `id`
- `name`
- `description`
- `schedule`
- `payload_kind`: `agent_prompt` or `shell_command`
- `payload_json`
- `enabled`
- `next_run_at`
- `last_run_at`
- `created_at`
- `updated_at`

`cron_triggers` stores each scheduled occurrence:

- `id`
- `cron_job_id`
- `scheduled_for`
- `triggered_at`
- `task_id`
- `status`
- `error`

`tasks` stores the current task state:

- `id`
- `source`: `manual`, `cron`, or future source names
- `source_id`
- `title`
- `body`
- `payload_kind`
- `payload_json`
- `status`: `queued`, `claimed`, `running`, `succeeded`, `failed`, `blocked`, `cancelled`, `timed_out`, or `lost`
- `priority`
- `claim_lock`
- `claim_expires_at`
- `assigned_worker`
- `last_heartbeat_at`
- `retry_count`
- `max_retries`
- `last_error`
- `created_at`
- `updated_at`
- `started_at`
- `completed_at`

`task_runs` stores every execution attempt:

- `id`
- `task_id`
- `worker_id`
- `status`
- `started_at`
- `finished_at`
- `duration_ms`
- `session_id`
- `output_summary`
- `error`
- `log_path`

`task_events` stores append-only task history:

- `id`
- `task_id`
- `run_id`
- `event_type`
- `payload_json`
- `created_at`

## Data Flow

Cron creation:

1. User creates a cron job through CLI, API, tool call, or dashboard.
2. `CronStore` validates the schedule and computes `next_run_at`.
3. The job is persisted before it is visible to the scheduler.

Cron trigger:

1. `CronScheduler` ticks every 10 to 30 seconds.
2. It finds enabled jobs with `next_run_at <= now`.
3. For each due occurrence, it creates a `cron_triggers` row using a uniqueness constraint on `(cron_job_id, scheduled_for)`.
4. It creates a durable `tasks` row linked to that trigger.
5. It advances `next_run_at` according to the schedule.

Task execution:

1. `WorkerRuntime` atomically claims a queued task with a lock token and expiry.
2. It creates a `task_runs` row and moves the task to `running`.
3. It starts heartbeat updates while execution is active.
4. `AgentExecutor` or a shell executor performs the payload.
5. Agent events, tool events, stdout/stderr references, and status changes are appended to `task_events`.
6. Completion updates `tasks`, `task_runs`, and the originating `cron_triggers` row.

Recovery:

1. On startup, the worker scans `claimed` and `running` tasks.
2. If the task has no fresh heartbeat or the claim expired, it becomes `lost`, `timed_out`, or returns to `queued` based on retry policy.
3. Recovery decisions append task events so the dashboard shows why state changed.

## Scheduling Semantics

Cron must not create duplicate tasks for the same scheduled occurrence. The uniqueness key is `(cron_job_id, scheduled_for)`.

Missed runs should use an explicit policy per job or global default:

- `skip`: advance to the next future occurrence.
- `run_once`: create one catch-up task for the latest missed occurrence.
- `catch_up`: create tasks for each missed occurrence up to a bounded limit.

The first version should default to `run_once` because it preserves useful work without flooding the queue after downtime.

## Worker Semantics

Workers must use atomic claim operations. A task is claimable only if it is queued, retryable, or has an expired claim. A successful claim writes a unique `claim_lock`, worker id, expiry, and event.

Heartbeat is the liveness signal. A `running` task without recent heartbeat is not trusted. The worker should refresh heartbeat every 10 to 30 seconds during long agent execution.

Retry policy should be conservative:

- `max_retries` defaults to 0 or 1 for shell commands.
- Agent tasks may use 1 retry for transient runtime errors.
- Permission denials and validation failures should not retry automatically.
- Requeued tasks must include the previous error in `task_events`.

## API And Dashboard

The HTTP service should expose stable APIs before investing in a rich UI:

- `GET /api/cron/jobs`
- `POST /api/cron/jobs`
- `GET /api/cron/jobs/:id`
- `PATCH /api/cron/jobs/:id`
- `POST /api/cron/jobs/:id/run`
- `POST /api/cron/jobs/:id/pause`
- `POST /api/cron/jobs/:id/resume`
- `DELETE /api/cron/jobs/:id`
- `GET /api/tasks`
- `POST /api/tasks`
- `GET /api/tasks/:id`
- `GET /api/tasks/:id/runs`
- `GET /api/tasks/:id/events`
- `POST /api/tasks/:id/cancel`
- `POST /api/tasks/:id/retry`

The first dashboard can be server-rendered or static HTML with polling. It should provide:

- `/cron`: cron job list, next run, last run, last status, pause/resume, run now, delete.
- `/tasks`: Kanban columns for queued, running, blocked, failed, succeeded, timed out, and lost.
- `/tasks/:id`: task body, current status, run history, event timeline, agent events, logs, and retry/cancel actions.

## Error Handling

All failed transitions should write an event. Silent failure is not acceptable for a long-running automation system.

Important failure cases:

- Invalid schedule: reject at create/update time.
- Duplicate scheduled trigger: do not create another task; record or ignore idempotently.
- Task claim conflict: return no task to the losing worker.
- Worker crash: recover by heartbeat expiry and claim expiry.
- Agent runtime error: write `task_runs.error`, `tasks.last_error`, and a task event.
- Permission denial: mark failed or blocked depending on whether human approval is later added.
- Dashboard/API read failure: return structured errors and log details.

## Testing

Use TDD for each behavior change. Minimum coverage:

- Schedule parsing and `next_run_at` computation.
- Cron job create/update/pause/resume/delete persistence.
- Due cron job creates one task per scheduled occurrence.
- Duplicate trigger attempts are idempotent.
- Worker atomic claim prevents double execution.
- Heartbeat updates keep a running task alive.
- Expired heartbeat/claim moves a task to `lost`, `timed_out`, or `queued` according to policy.
- Agent events are appended to `task_events` during execution.
- API serialization for cron jobs, tasks, runs, and events.
- Dashboard route returns useful HTML without requiring external assets.

Final verification for implementation batches must run:

- `cargo fmt --all`
- `cargo test --workspace`
- `cargo build --workspace`

## Migration Path

Start from the existing lightweight cron pieces:

1. Add durable `CronStore` and tests.
2. Add `KanbanStore` with task, run, and event tables.
3. Change cron triggering to create tasks instead of executing commands directly.
4. Add an embedded worker loop in `serve` for local single-process operation.
5. Wire `AgentExecutor` into the worker for agent prompt tasks.
6. Expose API and dashboard routes.
7. Later split worker into an independent process and support multiple workers.

Existing `~/.agent/cron/jobs.json` should either be imported into SQLite on startup or kept as a legacy CLI source until a migration command exists. The stable system should not keep writing independent JSON and SQLite state for the same job.

## Success Criteria

- A cron job can be created for "every 5 minutes" and persists across service restarts.
- When due, the cron job creates a durable task instead of only running in memory.
- The task can be claimed, executed, heartbeated, completed, and inspected after restart.
- The dashboard shows cron status, task status, run history, and event history.
- Duplicate cron ticks do not create duplicate tasks for the same occurrence.
- A crashed or stopped worker leaves enough state for recovery and diagnosis.
- The implementation keeps OpenClaw-level reliability as the target while delivering the first version with local SQLite and simple HTML.
