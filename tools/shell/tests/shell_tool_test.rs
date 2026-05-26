use agent_tools::Tool;
use shell_tool::ShellTool;

#[test]
fn shell_tool_is_destructive() {
    let tool = ShellTool::new();

    assert!(tool.is_destructive());
    assert!(!tool.is_read_only());
}

#[test]
fn shell_tool_treats_listing_commands_as_read_only() {
    let tool = ShellTool::new();

    assert!(tool.is_input_read_only(&serde_json::json!({"command": "ls"})));
    assert!(tool.is_input_read_only(&serde_json::json!({"command": "pwd"})));
    assert!(!tool.is_input_read_only(&serde_json::json!({"command": "rm -rf tmp"})));
}
