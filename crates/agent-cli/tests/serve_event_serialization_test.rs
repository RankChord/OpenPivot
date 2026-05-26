use agent_cli::cmd::serve::{SerializableAgentEvent, sse_event_line};
use agent_core::AgentEvent;

#[test]
fn serializes_agent_events_for_http_api() {
    let event = SerializableAgentEvent::from(AgentEvent::ToolResult {
        id: "call_1".into(),
        ok: false,
        content: String::new(),
        error: Some("denied".into()),
    });

    let json = serde_json::to_value(event).unwrap();

    assert_eq!(json["type"], "tool_result");
    assert_eq!(json["id"], "call_1");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"], "denied");
}

#[test]
fn builds_router_with_supplied_config() {
    let mut config = agent_config::AgentConfig::default();
    config.model.name = "custom-model".into();

    let state = agent_cli::cmd::serve::GatewayState::new(9000, config.clone());
    let _router = agent_cli::cmd::serve::build_router(std::sync::Arc::new(state));

    assert_eq!(config.model.name, "custom-model");
}

#[test]
fn formats_agent_event_as_sse_line() {
    let line = sse_event_line(&AgentEvent::ToolCall {
        id: "call_1".into(),
        name: "read_file".into(),
    });

    assert!(line.starts_with("data: "));
    assert!(line.ends_with("\n\n"));
    assert!(line.contains("\"type\":\"tool_call\""));
}

#[test]
fn response_output_is_derived_from_final_event() {
    let output = agent_cli::cmd::serve::output_from_events(
        &[AgentEvent::Final {
            content: "answer".into(),
        }],
        None,
    );

    assert_eq!(output, "answer");
}

#[test]
fn response_output_reports_runtime_error_when_no_final_event() {
    let output = agent_cli::cmd::serve::output_from_events(&[], Some("boom"));

    assert_eq!(output, "[Error] boom");
}
