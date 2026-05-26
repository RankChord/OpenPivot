use agent_tools::*;
use async_trait::async_trait;
use std::path::PathBuf;

#[derive(Debug)]
struct MockTool {
    def: ToolDefinition,
}

#[async_trait]
impl Tool for MockTool {
    fn definition(&self) -> &ToolDefinition {
        &self.def
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        ToolResult {
            ok: true,
            content: input.to_string(),
            error: None,
        }
    }
}

fn make_tool(name: &str, toolset: &str) -> MockTool {
    MockTool {
        def: ToolDefinition {
            name: name.into(),
            description: format!("Tool {}", name),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
            toolset_name: toolset.into(),
            aliases: vec![],
            max_result_size: 10000,
        },
    }
}

#[tokio::test]
async fn test_execute_tool_resolves_alias_name() {
    let registry = ToolRegistry::new();
    let mut tool = make_tool("execute_command", "terminal");
    tool.def.aliases = vec!["shell".into()];
    registry.register(Box::new(tool)).await;

    let result = registry
        .execute(ToolCall {
            id: "tc_123".into(),
            name: "shell".into(),
            input: serde_json::json!({"command": "ls"}),
            session_id: "sess_1".into(),
            workspace_dir: PathBuf::from("/tmp"),
        })
        .await;

    assert!(result.ok);
    assert_eq!(result.content, r#"{"command":"ls"}"#);
}

#[tokio::test]
async fn test_register_and_get_schemas() {
    let registry = ToolRegistry::new();
    registry
        .register(Box::new(make_tool("read_file", "file")))
        .await;
    registry
        .register(Box::new(make_tool("bash", "terminal")))
        .await;

    let schemas = registry.get_tool_schemas().await;
    assert_eq!(schemas.len(), 2);

    let names: Vec<_> = schemas.iter().map(|s| &s.name).collect();
    assert!(names.contains(&&"read_file".to_string()));
    assert!(names.contains(&&"bash".to_string()));
}

#[tokio::test]
async fn test_execute_tool() {
    let registry = ToolRegistry::new();
    registry
        .register(Box::new(make_tool("read_file", "file")))
        .await;

    let result = registry
        .execute(ToolCall {
            id: "tc_123".into(),
            name: "read_file".into(),
            input: serde_json::json!({"path": "test.txt"}),
            session_id: "sess_1".into(),
            workspace_dir: PathBuf::from("/tmp"),
        })
        .await;

    assert!(result.ok);
    assert_eq!(result.content, r#"{"path":"test.txt"}"#);
}

#[tokio::test]
async fn test_execute_missing_tool() {
    let registry = ToolRegistry::new();

    let result = registry
        .execute(ToolCall {
            id: "tc_123".into(),
            name: "nonexistent".into(),
            input: serde_json::json!({}),
            session_id: "sess_1".into(),
            workspace_dir: PathBuf::from("/tmp"),
        })
        .await;

    assert!(!result.ok);
    assert!(result.error.unwrap().contains("Tool not found"));
}

#[tokio::test]
async fn test_execute_tool_denies_when_permission_callback_denies() {
    let registry = ToolRegistry::new();
    registry
        .register(Box::new(make_tool("write_file", "file")))
        .await;

    let result = registry
        .execute_with_permission(
            ToolCall {
                id: "tc_123".into(),
                name: "write_file".into(),
                input: serde_json::json!({"path": "test.txt"}),
                session_id: "sess_1".into(),
                workspace_dir: PathBuf::from("/tmp"),
            },
            &|check| PermissionResult::Deny(format!("{} denied", check.tool_name)),
        )
        .await;

    assert!(!result.ok);
    assert_eq!(result.content, "");
    assert_eq!(result.error.as_deref(), Some("write_file denied"));
}

#[tokio::test]
async fn test_generation_counter() {
    let registry = ToolRegistry::new();
    assert_eq!(registry.generation(), 0);

    registry
        .register(Box::new(make_tool("tool1", "default")))
        .await;
    assert_eq!(registry.generation(), 1);

    registry
        .register(Box::new(make_tool("tool2", "default")))
        .await;
    assert_eq!(registry.generation(), 2);
}
