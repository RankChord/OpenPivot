# AI Agent Framework

Rust workspace for a terminal-first AI agent runtime.

## Current Capabilities

- OpenAI-compatible chat completions client with tool-call parsing.
- Agent loop with iteration budget, tool calls, tool results, final responses, and structured runtime events.
- CLI `run` and `chat` commands wired to the runtime.
- TUI timeline messages for tool calls, failed tool results, and final output.
- HTTP service with `/health`, `/run`, and `/run/stream` SSE endpoints.
- Durable cron and Kanban automation backed by `~/.agent/automation.db`.
- HTTP cron/task dashboard routes: `/cron` and `/tasks`.
- SQLite session store for messages, runtime events, session listing, legacy message schema migration, and event-based history reconstruction.
- Tool registry with permission callbacks and tool safety metadata.
- Workspace boundary checks for `read_file` and `write_file`.

## Known Limits

- Anthropic and Google providers currently return explicit unsupported-provider errors instead of issuing incompatible requests.
- `web_search` supports `AGENT_WEB_SEARCH_STATIC_RESULTS` for configured static results; without it, it returns a clear configuration error.
- `delegate_task` supports a minimal local delegate path when `AGENT_DELEGATE_LOCAL=1`; without it, spawn returns a clear unsupported error.

## Configuration

Default config path: `~/.agent/config.toml`.

```toml
[model]
provider = "openai"
name = "gpt-4"
base_url = "https://api.openai.com/v1"
api_key = "..."
max_tokens = 4096
temperature = 0.7

[agent]
max_iterations = 90
stream_mode = "full"
auto_approve_tools = false

[permissions]
auto_approve_tools = false
read_only = false
allow_destructive_tools = false
allowed_tools = ["read_file", "todo", "browse_web"]
ask_tools = ["shell"]
allowed_dirs = []
```

`AGENT_API_KEY` can be used when `model.api_key` is omitted.
`AGENT_WEB_SEARCH_STATIC_RESULTS` may contain a JSON array of `{ "title", "url", "snippet" }` results for the `web_search` tool.
`AGENT_DELEGATE_LOCAL=1` enables the minimal local delegate path for `delegate_task`.

`execute_command` is treated conservatively: simple listing commands such as `ls`, `ls ...`, `pwd`, and `find . -maxdepth 1 -type f` are classified as read-only inputs, while destructive shell commands remain blocked unless `allow_destructive_tools = true`.

## Durable Cron And Kanban

Automation state is stored in `~/.agent/automation.db`.

- Cron jobs define when work should start.
- Due cron jobs create durable Kanban tasks.
- The embedded worker claims queued tasks, writes heartbeat, records runs, and appends task events.
- Agent prompt tasks persist agent runtime events as task events while the agent runs.
- `/cron` shows schedule status.
- `/tasks` shows queued, running, and finished task state.

Security and reliability notes:

- Set `AGENT_HTTP_TOKEN` before binding `serve` to a non-loopback host.
- Protected HTTP routes require `Authorization: Bearer <token>` when a token is configured.
- HTTP task/cron creation rejects `shell_command` payloads.
- Cancelled tasks are durable; workers will not record them as succeeded after cancellation is observed.
- Retryable failures can be requeued until `max_retries` is reached.

Example API payload:

```json
{
  "name": "Every five minutes",
  "description": "Demo scheduled agent task",
  "schedule": { "Duration": { "secs": 300, "nanos": 0 } },
  "payload_kind": "agent_prompt",
  "payload_json": { "prompt": "Say hello from cron" },
  "enabled": true,
  "missed_run_policy": "run_once"
}
```

Useful endpoints:

- `GET /api/cron/jobs`
- `POST /api/cron/jobs`
- `GET /api/tasks`
- `POST /api/tasks`

## Logs

Runtime logs are written to `~/.agent/logs/agent.log`. In TUI mode logs are file-only so the alternate-screen UI is not corrupted by stderr output.

Useful debugging command:

```sh
tail -f ~/.agent/logs/agent.log
```

The log includes agent run start/completion, LLM request/response status, tool calls, tool permission denials, tool failures, and panic hook output.

## Development

Run the standard verification set before treating a change as complete:

```sh
cargo fmt --all
cargo test --workspace
cargo build --workspace
```
