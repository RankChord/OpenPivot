use crate::tool::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

/// Tool schema for LLM API
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Tool call from LLM response
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
    pub session_id: String,
    pub workspace_dir: std::path::PathBuf,
}

/// Tool set definition
#[derive(Debug, Clone)]
pub struct ToolSet {
    pub name: String,
    pub tools: Vec<String>,
}

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Box<dyn Tool>>>,
    aliases: RwLock<HashMap<String, String>>,
    toolsets: RwLock<HashMap<String, ToolSet>>,
    generation: AtomicU64,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            aliases: RwLock::new(HashMap::new()),
            toolsets: RwLock::new(HashMap::new()),
            generation: AtomicU64::new(0),
        }
    }

    pub async fn register(&self, tool: Box<dyn Tool>) {
        let name = tool.definition().name.clone();
        let aliases = tool.definition().aliases.clone();
        let toolset_name = tool.definition().toolset_name.clone();

        self.tools.write().await.insert(name.clone(), tool);
        let mut alias_map = self.aliases.write().await;
        for alias in aliases {
            alias_map.insert(alias, name.clone());
        }
        self.generation.fetch_add(1, Ordering::Relaxed);

        let mut toolsets = self.toolsets.write().await;
        let toolset = toolsets
            .entry(toolset_name.clone())
            .or_insert_with(|| ToolSet {
                name: toolset_name,
                tools: Vec::new(),
            });
        toolset.tools.push(name);
    }

    pub async fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        let tools = self.tools.read().await;
        tools
            .values()
            .filter(|t| t.requirements_check())
            .map(|t| ToolSchema {
                name: t.definition().name.clone(),
                description: t.definition().description.clone(),
                input_schema: t.definition().input_schema.clone(),
            })
            .collect()
    }

    pub async fn execute(&self, tool_call: ToolCall) -> ToolResult {
        self.execute_with_permission(tool_call, &|_| PermissionResult::Allow)
            .await
    }

    pub async fn execute_with_permission(
        &self,
        tool_call: ToolCall,
        can_use: &(dyn Fn(&PermissionCheck) -> PermissionResult + Sync),
    ) -> ToolResult {
        let tools = self.tools.read().await;
        let canonical_name = self
            .aliases
            .read()
            .await
            .get(&tool_call.name)
            .cloned()
            .unwrap_or_else(|| tool_call.name.clone());
        let tool = match tools.get(&canonical_name) {
            Some(t) => t,
            None => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Tool not found: {}", tool_call.name)),
                };
            }
        };

        match can_use(&PermissionCheck {
            tool_name: canonical_name.clone(),
            input: tool_call.input.clone(),
            is_read_only: tool.is_input_read_only(&tool_call.input),
            is_destructive: tool.is_destructive(),
        }) {
            PermissionResult::Allow => {}
            PermissionResult::Deny(reason) | PermissionResult::Ask(reason) => {
                tracing::warn!(tool = %canonical_name, reason = %reason, "tool permission denied");
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(reason),
                };
            }
        }

        let ctx = ToolUseContext {
            session_id: &tool_call.session_id,
            tool_use_id: &tool_call.id,
            workspace_dir: &tool_call.workspace_dir,
            can_use,
        };

        let progress_fn = |_update: ProgressUpdate| {
            tracing::debug!(tool_id = %_update.tool_id, msg = %_update.message, "tool progress");
        };

        tool.call(tool_call.input, &ctx, &progress_fn).await
    }

    pub async fn get_enabled_tools(&self, enabled_sets: &[String]) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools
            .values()
            .filter(|t| {
                let toolset = &t.definition().toolset_name;
                enabled_sets.is_empty() || enabled_sets.contains(toolset)
            })
            .map(|t| t.definition().clone())
            .collect()
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }
}
