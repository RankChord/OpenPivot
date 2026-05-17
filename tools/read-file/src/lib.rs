use async_trait::async_trait;
use agent_tools::{Tool, ToolDefinition, ToolUseContext, ToolResult, ProgressUpdate};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ReadFileTool {
    def: ToolDefinition,
}

impl ReadFileTool {
    pub fn new() -> Self {
        Self {
            def: ToolDefinition {
                name: "read_file".into(),
                description: "Read the contents of a file at the given path. Supports line_range to read specific lines.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Path to the file to read"},
                        "limit": {"type": "integer", "description": "Max number of lines to read (default: 1000)"},
                        "offset": {"type": "integer", "description": "Starting line number (default: 0)"}
                    },
                    "required": ["path"]
                }),
                toolset_name: "file".into(),
                aliases: vec!["cat".into()],
                max_result_size: 50000,
            },
        }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> &ToolDefinition { &self.def }

    async fn call(
        &self,
        input: serde_json::Value,
        ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        let path_str = match input["path"].as_str() {
            Some(p) => p.to_string(),
            None => return ToolResult {
                ok: false,
                content: String::new(),
                error: Some("Missing required parameter: path".into()),
            },
        };
        let limit = input["limit"].as_u64().unwrap_or(1000) as usize;
        let offset = input["offset"].as_u64().unwrap_or(0) as usize;
        let workspace_dir = ctx.workspace_dir.to_path_buf();

        let full_path = if path_str.starts_with('/') {
            PathBuf::from(path_str)
        } else {
            workspace_dir.join(path_str)
        };

        match tokio::fs::read_to_string(&full_path).await {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let total_lines = lines.len();
                let sliced: Vec<&str> = lines.into_iter().skip(offset).take(limit).collect();
                let result = sliced.join("\n");

                let header = if offset > 0 || sliced.len() < total_lines {
                    format!("// Read {} lines from {} (total: {} lines)\n", sliced.len(), full_path.display(), total_lines)
                } else {
                    String::new()
                };

                ToolResult {
                    ok: true,
                    content: format!("{}{}", header, result),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                ok: false,
                content: String::new(),
                error: Some(format!("Failed to read file: {}", e)),
            },
        }
    }

    fn is_concurrency_safe(&self) -> bool { true }
    fn is_read_only(&self) -> bool { true }
}

#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn create_tool() -> Box<dyn Tool> {
    Box::new(ReadFileTool::new())
}
