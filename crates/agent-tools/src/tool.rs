use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Tool definition metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub toolset_name: String,
    pub aliases: Vec<String>,
    pub max_result_size: usize,
}

/// Permission check request
pub struct PermissionCheck {
    pub tool_name: String,
    pub input: serde_json::Value,
}

/// Permission decision
#[derive(Debug, Clone)]
pub enum PermissionResult {
    Allow,
    Deny(String),
    Ask(String),
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub ok: bool,
    pub content: String,
    pub error: Option<String>,
}

/// Progress update during tool execution
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub tool_id: String,
    pub message: String,
    pub is_done: bool,
}

/// Interrupt behavior
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterruptPolicy {
    Cancel,
    Block,
}

/// Tool execution context
pub struct ToolUseContext<'a> {
    pub session_id: &'a str,
    pub tool_use_id: &'a str,
    pub workspace_dir: &'a std::path::Path,
    pub can_use: &'a (dyn Fn(&PermissionCheck) -> PermissionResult + Sync),
}

#[async_trait]
pub trait Tool: Send + Sync + std::fmt::Debug {
    fn definition(&self) -> &ToolDefinition;

    async fn call(
        &self,
        input: serde_json::Value,
        ctx: &ToolUseContext<'_>,
        on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult;

    fn is_concurrency_safe(&self) -> bool { false }
    fn is_read_only(&self) -> bool { false }
    fn is_destructive(&self) -> bool { false }
    fn interrupt_behavior(&self) -> InterruptPolicy { InterruptPolicy::Block }
    fn requirements_check(&self) -> bool { true }
    
    /// Tool-level permission check (default: defer to general permission system)
    async fn check_permissions(
        &self,
        _input: &serde_json::Value,
        _ctx: &PermissionContext,
    ) -> PermissionResult {
        PermissionResult::Allow
    }
    
    /// Dynamic prompt description based on context
    async fn prompt_description(&self, _ctx: &ToolPromptContext<'_>) -> String {
        self.definition().description.clone()
    }
}

/// Permission context passed to tools for permission decisions
pub struct PermissionContext {
    pub mode: String,
    pub workspace_dir: std::path::PathBuf,
}

/// Context for generating tool prompt descriptions
pub struct ToolPromptContext<'a> {
    pub is_non_interactive: bool,
    pub permission_context: &'a PermissionContext,
    pub tools: &'a [&'a dyn Tool],
}
