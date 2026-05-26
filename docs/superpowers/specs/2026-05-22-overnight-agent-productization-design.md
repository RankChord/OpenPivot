# Overnight Agent Productization Design

## Goal

Turn the current Rust agent workspace into a more coherent, stable product by morning without taking unsafe rewrite risks. The work focuses on shared runtime construction, consistent session behavior, predictable tool permissions, TUI stability, and documentation that matches reality.

## Non-Goals

- Do not replace the whole architecture.
- Do not claim full Anthropic or Google provider support until request/response adapters exist.
- Do not claim real web search unless a real provider is implemented and tested.
- Do not claim full multi-agent delegation unless there is a durable task model and execution boundary.

## Recommended Approach

Use an incremental core-consolidation refactor.

Alternative A, a full rewrite, is too risky for an unattended night session. Alternative B, only fixing visible bugs, leaves duplicated builders and inconsistent behavior. The selected approach keeps the existing crates but centralizes repeated construction paths so CLI, TUI, and HTTP behave the same.

## Architecture

The stable boundaries remain:

- `agent-core`: agent loop, events, permissions, tool execution feedback.
- `agent-llm`: provider-normalized chat client.
- `agent-tools`: registry, aliases, permission metadata.
- `agent-sessions`: SQLite state, events, history reconstruction.
- `agent-cli`: command surfaces, HTTP service, TUI launcher.
- `agent-tui`: interactive terminal UI.

The main structural change is to add shared construction helpers in `agent-cli` for:

- model config to `LlmClientConfig`
- default tool registry
- `Agent` creation with permissions
- session DB path and session history loading

CLI `run`, CLI `chat`, TUI, and HTTP service should call those helpers instead of duplicating registration and config logic.

## Data Flow

User input flows into `AgentInput`. Before a model request, history is loaded from session events/messages. Runtime emits `AgentEvent`s for user messages, tool calls, tool results, budget exhaustion, and final responses. Consumers choose how to display or stream events, but event generation stays in `agent-core`.

Session persistence stores runtime events as the authoritative execution trace. Reconstructed chat history is derived from events where possible. Existing message storage remains for compatibility.

## Permission And Tool Behavior

Permissions remain conservative by default:

- `execute_command` is allowed only for read-only inputs unless destructive tools are explicitly enabled.
- Shell aliases resolve to the canonical `execute_command` tool.
- Known read-only shell commands such as `ls`, `pwd`, and simple listing commands are allowed as read-only inputs.
- Destructive commands remain blocked unless configuration explicitly permits them.

## TUI Stability

TUI input must treat cursor positions as character positions and convert to byte indices only when mutating strings. This prevents UTF-8 panics for Chinese and other multi-byte input.

TUI should avoid garbled status output by keeping tool failures as normal timeline messages and leaving terminal cleanup reliable.

## Testing

Use TDD for every behavior change. Add or preserve tests for:

- shared builder functions produce the expected provider/model/tool registry behavior
- registry alias resolution
- read-only shell command permission behavior
- session event history reconstruction
- TUI multi-byte input editing
- HTTP event serialization/SSE formatting

Final verification must run:

- `cargo fmt --all`
- `cargo test --workspace`
- `cargo build --workspace`

## Success Criteria

- Full workspace tests and build pass.
- CLI/TUI/API all construct agents through shared helpers or equivalent centralized code.
- Asking for current directory contents no longer fails due to `execute_command` permission denial for `ls`.
- UTF-8 TUI input does not panic.
- README and doctor describe actual supported behavior.
- Remaining limitations are explicit and not hidden behind fake success.
