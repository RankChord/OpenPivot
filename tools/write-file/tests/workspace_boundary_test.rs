use agent_tools::{PermissionResult, Tool, ToolUseContext};
use write_file::WriteFileTool;

#[tokio::test]
async fn denies_writing_file_outside_workspace() {
    let workspace = tempfile::tempdir().unwrap();
    let outside_dir = tempfile::tempdir().unwrap();

    let tool = WriteFileTool::new();
    let ctx = ToolUseContext {
        session_id: "session",
        tool_use_id: "tool-use",
        workspace_dir: workspace.path(),
        can_use: &|_| PermissionResult::Allow,
    };

    let result = tool
        .call(
            serde_json::json!({
                "path": outside_dir.path().join("out.txt").display().to_string(),
                "content": "secret"
            }),
            &ctx,
            &|_| {},
        )
        .await;

    assert!(!result.ok);
    assert!(result.error.unwrap().contains("outside workspace"));
}

#[tokio::test]
async fn writes_new_file_inside_existing_workspace_directory() {
    let workspace = tempfile::tempdir().unwrap();

    let tool = WriteFileTool::new();
    let ctx = ToolUseContext {
        session_id: "session",
        tool_use_id: "tool-use",
        workspace_dir: workspace.path(),
        can_use: &|_| PermissionResult::Allow,
    };

    let result = tool
        .call(
            serde_json::json!({
                "path": "out.txt",
                "content": "hello"
            }),
            &ctx,
            &|_| {},
        )
        .await;

    assert!(result.ok, "{:?}", result.error);
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("out.txt")).unwrap(),
        "hello"
    );
}

#[test]
fn write_file_tool_is_not_read_only() {
    let tool = WriteFileTool::new();

    assert!(!tool.is_read_only());
    assert!(tool.is_destructive());
}
