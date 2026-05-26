use agent_cli::cmd::run::format_event_line;
use agent_core::AgentEvent;

#[test]
fn formats_tool_call_event_for_cli_progress() {
    let line = format_event_line(&AgentEvent::ToolCall {
        id: "call_1".into(),
        name: "read_file".into(),
    });

    assert_eq!(line.as_deref(), Some("→ tool: read_file"));
}

#[test]
fn formats_failed_tool_result_for_cli_progress() {
    let line = format_event_line(&AgentEvent::ToolResult {
        id: "call_1".into(),
        ok: false,
        content: String::new(),
        error: Some("permission denied".into()),
    });

    assert_eq!(line.as_deref(), Some("× tool failed: permission denied"));
}
