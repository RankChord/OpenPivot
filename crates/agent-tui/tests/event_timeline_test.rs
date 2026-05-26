use agent_core::AgentEvent;
use agent_tui::{AppState, event_to_timeline_message};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

#[test]
fn maps_tool_call_to_timeline_message() {
    let message = event_to_timeline_message(&AgentEvent::ToolCall {
        id: "call_1".into(),
        name: "read_file".into(),
    });

    assert_eq!(message, Some(("Tool".into(), "Running read_file".into())));
}

#[test]
fn maps_failed_tool_result_to_timeline_message() {
    let message = event_to_timeline_message(&AgentEvent::ToolResult {
        id: "call_1".into(),
        ok: false,
        content: String::new(),
        error: Some("permission denied".into()),
    });

    assert_eq!(
        message,
        Some(("Tool".into(), "Failed: permission denied".into()))
    );
}

#[test]
fn accepts_chinese_input_without_panicking() {
    let mut app = AppState::new();

    app.handle_input(
        KeyEventKind::Press,
        KeyCode::Char('你'),
        KeyModifiers::empty(),
    );
    app.handle_input(
        KeyEventKind::Press,
        KeyCode::Char('好'),
        KeyModifiers::empty(),
    );

    let submitted = app.handle_input(KeyEventKind::Press, KeyCode::Enter, KeyModifiers::empty());

    assert_eq!(submitted, Some("你好".into()));
}

#[test]
fn edits_chinese_input_at_character_boundaries() {
    let mut app = AppState::new();

    app.handle_input(
        KeyEventKind::Press,
        KeyCode::Char('你'),
        KeyModifiers::empty(),
    );
    app.handle_input(
        KeyEventKind::Press,
        KeyCode::Char('好'),
        KeyModifiers::empty(),
    );
    app.handle_input(KeyEventKind::Press, KeyCode::Left, KeyModifiers::empty());
    app.handle_input(
        KeyEventKind::Press,
        KeyCode::Char('很'),
        KeyModifiers::empty(),
    );
    app.handle_input(
        KeyEventKind::Press,
        KeyCode::Backspace,
        KeyModifiers::empty(),
    );

    let submitted = app.handle_input(KeyEventKind::Press, KeyCode::Enter, KeyModifiers::empty());

    assert_eq!(submitted, Some("你好".into()));
}
