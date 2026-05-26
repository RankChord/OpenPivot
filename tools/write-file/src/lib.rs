use agent_tools::{ProgressUpdate, Tool, ToolDefinition, ToolResult, ToolUseContext};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct WriteFileTool {
    def: ToolDefinition,
}

impl WriteFileTool {
    pub fn new() -> Self {
        Self {
            def: ToolDefinition {
                name: "write_file".into(),
                description: "Write content to a file. Creates parent directories if needed. Supports append mode.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Path to the file to write"},
                        "content": {"type": "string", "description": "Content to write"},
                        "append": {"type": "boolean", "description": "If true, append to file instead of overwriting"}
                    },
                    "required": ["path", "content"]
                }),
                toolset_name: "file".into(),
                aliases: vec![],
                max_result_size: 1000,
            },
        }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.def
    }

    async fn call(
        &self,
        input: serde_json::Value,
        ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        let path = match input["path"].as_str() {
            Some(p) => p,
            None => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some("Missing required parameter: path".into()),
                };
            }
        };
        let content = match input["content"].as_str() {
            Some(c) => c,
            None => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some("Missing required parameter: content".into()),
                };
            }
        };
        let append = input["append"].as_bool().unwrap_or(false);

        let workspace_dir = match ctx.workspace_dir.canonicalize() {
            Ok(path) => path,
            Err(e) => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Invalid workspace directory: {}", e)),
                };
            }
        };

        let full_path = if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            workspace_dir.join(path)
        };

        if let Some(parent) = full_path.parent() {
            let parent = match parent.canonicalize() {
                Ok(path) => path,
                Err(e) => {
                    return ToolResult {
                        ok: false,
                        content: String::new(),
                        error: Some(format!("Failed to resolve parent directory: {}", e)),
                    };
                }
            };

            if !parent.starts_with(&workspace_dir) {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!(
                        "Path is outside workspace: {}",
                        full_path.display()
                    )),
                };
            }

            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Failed to create directory: {}", e)),
                };
            }
        }

        let write_result = match append {
            true => {
                match tokio::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(&full_path)
                    .await
                {
                    Ok(mut f) => f.write_all(content.as_bytes()).await,
                    Err(e) => Err(e),
                }
            }
            false => tokio::fs::write(&full_path, content).await,
        };

        match write_result {
            Ok(_) => ToolResult {
                ok: true,
                content: format!("Wrote {} bytes to {}", content.len(), full_path.display()),
                error: None,
            },
            Err(e) => ToolResult {
                ok: false,
                content: String::new(),
                error: Some(format!("Failed to write file: {}", e)),
            },
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }
}

#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn create_write_file_tool() -> Box<dyn Tool> {
    Box::new(WriteFileTool::new())
}
