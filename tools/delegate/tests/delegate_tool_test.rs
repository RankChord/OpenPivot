use agent_tools::{PermissionResult, Tool, ToolUseContext};
use delegate_tool::DelegateTool;

#[tokio::test]
async fn delegate_spawn_reports_unsupported_instead_of_fake_success() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = DelegateTool::new();
    let ctx = ToolUseContext {
        session_id: "session",
        tool_use_id: "tool-use",
        workspace_dir: workspace.path(),
        can_use: &|_| PermissionResult::Allow,
    };

    let result = tool
        .call(
            serde_json::json!({"action": "spawn", "goal": "inspect project"}),
            &ctx,
            &|_| {},
        )
        .await;

    assert!(!result.ok);
    assert!(result.error.unwrap().contains("not implemented"));
}

#[tokio::test]
async fn delegate_spawn_runs_local_echo_task_when_enabled() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = DelegateTool::new();
    let ctx = ToolUseContext {
        session_id: "session",
        tool_use_id: "tool-use",
        workspace_dir: workspace.path(),
        can_use: &|_| PermissionResult::Allow,
    };
    unsafe {
        std::env::set_var("AGENT_DELEGATE_LOCAL", "1");
    }

    let result = tool
        .call(
            serde_json::json!({"action": "spawn", "goal": "inspect project"}),
            &ctx,
            &|_| {},
        )
        .await;
    unsafe {
        std::env::remove_var("AGENT_DELEGATE_LOCAL");
    }

    assert!(result.ok);
    assert!(result.content.contains("inspect project"));
}
