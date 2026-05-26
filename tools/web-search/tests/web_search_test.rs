use agent_tools::{PermissionResult, Tool, ToolUseContext};
use web_search::WebSearchTool;

#[tokio::test]
async fn web_search_reports_unsupported_instead_of_fake_success() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = WebSearchTool::new();
    let ctx = ToolUseContext {
        session_id: "session",
        tool_use_id: "tool-use",
        workspace_dir: workspace.path(),
        can_use: &|_| PermissionResult::Allow,
    };

    let result = tool
        .call(
            serde_json::json!({"query": "rust agent framework"}),
            &ctx,
            &|_| {},
        )
        .await;

    assert!(!result.ok);
    assert!(result.error.unwrap().contains("not configured"));
}

#[tokio::test]
async fn web_search_uses_static_results_from_environment() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = WebSearchTool::new();
    let ctx = ToolUseContext {
        session_id: "session",
        tool_use_id: "tool-use",
        workspace_dir: workspace.path(),
        can_use: &|_| PermissionResult::Allow,
    };
    unsafe {
        std::env::set_var(
            "AGENT_WEB_SEARCH_STATIC_RESULTS",
            r#"[{"title":"Rust","url":"https://www.rust-lang.org","snippet":"Rust language"}]"#,
        );
    }

    let result = tool
        .call(serde_json::json!({"query": "rust"}), &ctx, &|_| {})
        .await;
    unsafe {
        std::env::remove_var("AGENT_WEB_SEARCH_STATIC_RESULTS");
    }

    assert!(result.ok);
    assert!(result.content.contains("Rust"));
    assert!(result.content.contains("https://www.rust-lang.org"));
}
