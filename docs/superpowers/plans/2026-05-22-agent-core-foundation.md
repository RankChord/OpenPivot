# Agent Core Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the current Rust agent prototype into a reliable core foundation that can safely support future TUI, service, and multi-agent features.

**Architecture:** Stabilize the existing workspace instead of rewriting it. Keep `agent-core` responsible for the loop, `agent-llm` for provider-normalized chat, `agent-tools` for registry and permission-aware execution, and `agent-sessions` for durable conversation state.

**Tech Stack:** Rust 2024, Tokio, Reqwest, SQLx SQLite, Clap, Axum, Ratatui, Cargo workspace tests.

---

## File Map

- `crates/agent-llm/src/client.rs`: provider config, request construction, response normalization.
- `crates/agent-llm/tests/client_test.rs`: config and provider behavior tests that do not require network.
- `crates/agent-core/src/types.rs`: message construction rules.
- `crates/agent-core/src/loop.rs`: agent loop, tool result handling, iteration budget behavior.
- `crates/agent-core/tests/core_test.rs`: message construction and loop-adjacent behavior tests.
- `crates/agent-sessions/src/store.rs`: SQLite initialization and stable history ordering.
- `crates/agent-sessions/tests/session_store_test.rs`: durable store tests using temp dirs.
- `crates/agent-tools/src/registry.rs`: permission-aware tool execution.
- `crates/agent-tools/tests/registry_test.rs`: permission denial and execution behavior.
- `tools/read-file/src/lib.rs`, `tools/write-file/src/lib.rs`: workspace path containment.
- `crates/agent-cli/src/cmd/mod.rs`, `crates/agent-cli/src/main.rs`, command modules: consistent config loading.
- `crates/agent-tui/src/lib.rs`: provider selection from config.

## Tasks

### Task 1: Establish Baseline

**Files:** none

- [ ] Run `cargo test --workspace` and record failures.
- [ ] Run `cargo build --workspace` and record failures.
- [ ] Fix only compile blockers first, preserving existing behavior unless covered by a failing behavior test.

### Task 2: LLM Config Consistency

**Files:**
- Modify: `crates/agent-llm/tests/client_test.rs`
- Modify: `crates/agent-llm/src/client.rs`

- [ ] Add/repair tests proving `LlmClientConfig` always includes `provider` and defaults to OpenAI.
- [ ] Verify tests fail for the current mismatch.
- [ ] Implement the smallest fix so config construction and tests agree.
- [ ] Run `cargo test -p agent-llm`.

### Task 3: Message Construction Does Not Duplicate Current User Input

**Files:**
- Modify: `crates/agent-core/tests/core_test.rs`
- Modify: `crates/agent-core/src/types.rs`
- Modify: `crates/agent-core/src/loop.rs`

- [ ] Add a failing test for `build_api_messages` with history that already contains the current user input.
- [ ] Change message construction so callers choose whether history already includes the current user message.
- [ ] Update `Agent::run` to avoid duplicate user messages.
- [ ] Run `cargo test -p agent-core`.

### Task 4: Session Store Creates New SQLite Files Reliably

**Files:**
- Create: `crates/agent-sessions/tests/session_store_test.rs`
- Modify: `crates/agent-sessions/Cargo.toml`
- Modify: `crates/agent-sessions/src/store.rs`

- [ ] Add a failing test that creates a temp directory, points `SessionStore::new` at a nonexistent `sessions.db`, appends messages, and reloads them.
- [ ] Replace unsafe `canonicalize().unwrap()` path logic with a SQLite URL that works before the file exists.
- [ ] Add a stable ordering column or deterministic tie-breaker for history.
- [ ] Run `cargo test -p agent-sessions`.

### Task 5: Tool Registry Honors Permissions

**Files:**
- Modify: `crates/agent-tools/src/registry.rs`
- Modify: `crates/agent-tools/tests/registry_test.rs`

- [ ] Add a failing test that executes a registered mock tool with a permission callback returning `Deny`.
- [ ] Add an execution path that accepts a permission callback while preserving the existing `execute` convenience method.
- [ ] Return a failed `ToolResult` when permission denies execution.
- [ ] Run `cargo test -p agent-tools`.

### Task 6: File Tools Stay Inside Workspace

**Files:**
- Add tests where practical under `tools/read-file/tests` and `tools/write-file/tests`, or crate-local unit tests.
- Modify: `tools/read-file/src/lib.rs`
- Modify: `tools/write-file/src/lib.rs`

- [ ] Add failing tests proving `../outside` and absolute paths outside `workspace_dir` are denied.
- [ ] Canonicalize existing paths for reads and parent paths for writes without requiring target file existence.
- [ ] Return clear `ToolResult` errors for outside-workspace access.
- [ ] Run `cargo test -p tool-read-file -p tool-write-file`.

### Task 7: Config Loading Is Consistent

**Files:**
- Modify: `crates/agent-cli/src/main.rs`
- Modify: `crates/agent-cli/src/cmd/mod.rs`
- Modify command modules as needed.
- Modify: `crates/agent-tui/src/lib.rs`

- [ ] Ensure global `--config` reaches commands that load config.
- [ ] Ensure TUI respects configured provider rather than hardcoding OpenAI.
- [ ] Run `cargo check -p agent-cli -p agent-tui`.

### Task 8: Final Verification

**Files:** all touched files

- [ ] Run `cargo fmt --all`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `cargo build --workspace`.
- [ ] Summarize remaining limitations explicitly, especially stubbed web search, delegate, service integrations, and TUI polish.

## Self-Review

- Scope is intentionally limited to the core foundation; TUI redesign and service platform work are deferred.
- Every behavior change above has a test-first step.
- No task requires committing; commits are left to the user unless explicitly requested.
