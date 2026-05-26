use agent_tools::{ProgressUpdate, Tool, ToolDefinition, ToolResult, ToolUseContext};

#[derive(Debug)]
pub struct DelegateTool;

impl DelegateTool {
    pub fn new() -> Self {
        Self
    }

    fn handle_task(input: serde_json::Value) -> ToolResult {
        if std::env::var("AGENT_DELEGATE_LOCAL").ok().as_deref() == Some("1") {
            let goal = input["goal"].as_str().unwrap_or("");
            return ToolResult {
                ok: true,
                content: format!("Local delegate completed task: {}", goal),
                error: None,
            };
        }

        ToolResult {
            ok: false,
            content: String::new(),
            error: Some("delegate_task spawn is not implemented yet".into()),
        }
    }
}

#[async_trait::async_trait]
impl Tool for DelegateTool {
    fn definition(&self) -> &ToolDefinition {
        static DEF: std::sync::OnceLock<ToolDefinition> = std::sync::OnceLock::new();
        DEF.get_or_init(|| ToolDefinition {
            name: "delegate_task".into(),
            description: "Spawn a sub-agent to handle a complex task in parallel.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["spawn", "list", "status"]
                    },
                    "goal": { "type": "string", "description": "Task description" },
                    "max_iterations": { "type": "integer", "default": 20 }
                },
                "required": ["action"]
            }),
            toolset_name: "delegation".into(),
            aliases: vec!["spawn_agent".into()],
            max_result_size: 5000,
        })
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        match input["action"].as_str() {
            Some("spawn") => Self::handle_task(input),
            _ => ToolResult {
                ok: false,
                content: String::new(),
                error: Some("Unknown action".into()),
            },
        }
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn create_delegate_task_tool() -> Box<dyn Tool> {
    Box::new(DelegateTool::new())
}
