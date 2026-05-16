use async_trait::async_trait;
use agent_tools::{Tool, ToolDefinition, ToolUseContext, ToolResult, ProgressUpdate, InterruptPolicy};

#[derive(Debug)]
pub struct ShellTool {
    def: ToolDefinition,
}

impl ShellTool {
    pub fn new() -> Self {
        Self {
            def: ToolDefinition {
                name: "execute_command".into(),
                description: "Execute a shell command in the terminal. Returns stdout + stderr. Command runs in the workspace directory.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "The command to execute"},
                        "timeout_seconds": {"type": "integer", "description": "Max execution time in seconds (default: 60)"}
                    },
                    "required": ["command"]
                }),
                toolset_name: "terminal".into(),
                aliases: vec!["bash".into(), "shell".into(), "run".into()],
                max_result_size: 30000,
            },
        }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn definition(&self) -> &ToolDefinition { &self.def }

    async fn call(
        &self,
        input: serde_json::Value,
        ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        let command = match input["command"].as_str() {
            Some(c) => c,
            None => return ToolResult {
                ok: false,
                content: String::new(),
                error: Some("Missing required parameter: command".into()),
            },
        };

        let timeout_secs = input["timeout_seconds"].as_u64().unwrap_or(60);
        let workspace_dir = ctx.workspace_dir.to_path_buf();

        #[cfg(unix)]
        {
            use tokio::process::Command;
            use std::time::Duration;

            let output = tokio::time::timeout(
                Duration::from_secs(timeout_secs),
                Command::new("bash")
                    .arg("-c")
                    .arg(command)
                    .current_dir(&workspace_dir)
                    .output(),
            ).await;

            match output {
                Ok(Ok(out)) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let mut result = String::new();
                    if out.status.success() {
                        result.push_str(&format!("Exit code: {}\n", out.status.code().unwrap_or(-1)));
                        if !stdout.trim().is_empty() {
                            result.push_str("=== stdout ===\n");
                            result.push_str(&stdout);
                        }
                    } else {
                        result.push_str(&format!("Exit code: {}\n", out.status.code().unwrap_or(-1)));
                        if !stderr.trim().is_empty() {
                            result.push_str(&format!("=== stderr ===\n{}", stderr));
                        }
                        if !stdout.trim().is_empty() {
                            result.push_str(&format!("\n=== stdout ===\n{}", stdout));
                        }
                    }
                    ToolResult {
                        ok: out.status.success(),
                        content: result,
                        error: if !out.status.success() { Some("Command failed".into()) } else { None },
                    }
                }
                Ok(Err(e)) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Command execution error: {}", e)),
                },
                Err(_) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Command timed out after {} seconds", timeout_secs)),
                },
            }
        }

        #[cfg(not(unix))]
        {
            let _ = (command, timeout_secs, workspace_dir);
            ToolResult {
                ok: false,
                content: String::new(),
                error: Some("Shell tool not supported on this platform".into()),
            }
        }
    }

    fn is_concurrency_safe(&self) -> bool { false }
    fn is_read_only(&self) -> bool { false }
    fn is_destructive(&self) -> bool { true }
    fn interrupt_behavior(&self) -> InterruptPolicy { InterruptPolicy::Cancel }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_tool() -> Box<dyn Tool> {
    Box::new(ShellTool::new())
}
