# Overnight Agent Productization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate the current Rust agent into a stable, coherent product foundation by centralizing runtime construction, session handling, permissions, and surface behavior across CLI, TUI, and HTTP.

**Architecture:** Keep the existing crate boundaries and avoid a rewrite. Add shared construction helpers in `agent-cli` so commands and service code stop duplicating tool registration, LLM config, and agent creation. Preserve conservative permissions while allowing read-only shell listing commands.

**Tech Stack:** Rust 2024, Tokio, Reqwest, SQLx SQLite, Axum, Ratatui, Clap, Cargo workspace tests.

---

## File Map

- Create: `crates/agent-cli/src/runtime.rs` for shared config-to-runtime builders.
- Modify: `crates/agent-cli/src/lib.rs` to expose the runtime module.
- Modify: `crates/agent-cli/src/cmd/run.rs` to use shared runtime and session helpers.
- Modify: `crates/agent-cli/src/cmd/chat.rs` to use shared runtime and session helpers.
- Modify: `crates/agent-cli/src/cmd/serve.rs` to use shared runtime builder.
- Modify: `crates/agent-tui/src/lib.rs` to reuse or mirror shared runtime where crate layering allows, and keep UTF-8-safe input handling.
- Modify: `crates/agent-tools/src/registry.rs` and tests for alias behavior if needed.
- Modify: `tools/shell/src/lib.rs` and tests for read-only command classification if needed.
- Modify: `README.md` and `crates/agent-cli/src/cmd/doctor.rs` so docs match reality.

## Task 1: Shared Runtime Builder

**Files:**
- Create: `crates/agent-cli/src/runtime.rs`
- Modify: `crates/agent-cli/src/lib.rs`
- Test: add focused tests where public pure helpers exist

- [ ] Create `runtime.rs` with functions:
  - `provider_from_config(provider: &str) -> agent_llm::LlmProviderType`
  - `llm_config_from_agent_config(config: &AgentConfig, override_model: Option<String>) -> LlmClientConfig`
  - `default_session_db_path() -> PathBuf`
  - `register_default_tools(registry: &ToolRegistry) -> impl Future<Output = ()>`
  - `build_agent(config: &AgentConfig, override_model: Option<String>) -> Agent`
- [ ] Export `pub mod runtime;` from `crates/agent-cli/src/lib.rs`.
- [ ] Add tests that prove OpenAI-compatible provider and override model are preserved.
- [ ] Run `cargo test -p agent-cli runtime`.

## Task 2: Use Shared Runtime In CLI Run

**Files:**
- Modify: `crates/agent-cli/src/cmd/run.rs`

- [ ] Replace manual tool registry construction with `runtime::build_agent`.
- [ ] Replace manual LLM config construction with shared helper.
- [ ] Keep stdin/message parsing behavior unchanged.
- [ ] Keep event persistence and progress output unchanged.
- [ ] Run `cargo check -p agent-cli`.

## Task 3: Use Shared Runtime In CLI Chat

**Files:**
- Modify: `crates/agent-cli/src/cmd/chat.rs`

- [ ] Replace manual tool registry and LLM construction with `runtime::build_agent`.
- [ ] Keep skill prompt and session loading behavior unchanged.
- [ ] Ensure events are persisted after final and budget-exhausted outputs.
- [ ] Run `cargo check -p agent-cli`.

## Task 4: Use Shared Runtime In HTTP Serve

**Files:**
- Modify: `crates/agent-cli/src/cmd/serve.rs`

- [ ] Replace local `build_agent` implementation with shared runtime builder.
- [ ] Keep `/run`, `/run/stream`, and `/health` route behavior unchanged.
- [ ] Run `cargo test -p agent-cli serve_event_serialization_test`.

## Task 5: TUI Stability And Runtime Consistency

**Files:**
- Modify: `crates/agent-tui/src/lib.rs`
- Test: `crates/agent-tui/tests/event_timeline_test.rs`

- [ ] Preserve UTF-8-safe cursor handling.
- [ ] Ensure TUI agent initialization uses the same provider mapping and permissions semantics as CLI runtime.
- [ ] Keep registered tools aligned with runtime defaults where dependencies allow.
- [ ] Run `cargo test -p agent-tui`.

## Task 6: Permission And Shell Listing Behavior

**Files:**
- Modify if needed: `crates/agent-core/src/types.rs`
- Modify if needed: `crates/agent-tools/src/registry.rs`
- Modify if needed: `tools/shell/src/lib.rs`
- Tests: `crates/agent-core/tests/core_test.rs`, `crates/agent-tools/tests/registry_test.rs`, `tools/shell/tests/shell_tool_test.rs`

- [ ] Confirm `execute_command` aliases resolve.
- [ ] Confirm `ls` and `pwd` are classified as read-only inputs.
- [ ] Confirm read-only input to a destructive tool type is allowed when the tool is allowlisted.
- [ ] Confirm destructive shell commands remain blocked unless explicitly enabled.
- [ ] Run targeted tests for these cases.

## Task 7: Docs And Diagnostics

**Files:**
- Modify: `README.md`
- Modify: `crates/agent-cli/src/cmd/doctor.rs`

- [ ] Remove duplicate README known-limit line.
- [ ] Document `execute_command` read-only behavior.
- [ ] Ensure doctor reports configured model/provider and permission status.
- [ ] Run `cargo check -p agent-cli`.

## Task 8: Final Verification

**Files:** all touched files

- [ ] Run `cargo fmt --all`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `cargo build --workspace`.
- [ ] If a command fails, fix with a focused regression test where behavior changed.
- [ ] Final response must report exact verification commands and remaining honest limitations.
