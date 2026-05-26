use agent_tools::{PermissionResult, Tool, ToolUseContext};
use read_file::ReadFileTool;

#[tokio::test]
async fn denies_reading_file_outside_workspace() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(outside.path(), "secret").unwrap();

    let tool = ReadFileTool::new();
    let ctx = ToolUseContext {
        session_id: "session",
        tool_use_id: "tool-use",
        workspace_dir: workspace.path(),
        can_use: &|_| PermissionResult::Allow,
    };

    let result = tool
        .call(
            serde_json::json!({"path": outside.path().display().to_string()}),
            &ctx,
            &|_| {},
        )
        .await;

    assert!(!result.ok);
    assert!(result.error.unwrap().contains("outside workspace"));
}
