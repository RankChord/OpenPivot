use crate::hooks::{HookFn, HookPoint};

/// Registration data returned by plugin's C entry point
pub struct PluginRegistration {
    pub id: String,
    pub version: String,
    pub description: String,
    pub tools: Vec<ToolBuilder>,
    pub hooks: Vec<(HookPoint, HookFn)>,
}

// Re-export ToolBuilder from agent-tools to avoid circular deps
// Plugin SDK defines ToolBuilder as a simple schema+constructor pair
// Actual Tool trait lives in agent-tools (which depends on this SDK)

pub struct ToolBuilder {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub toolset_name: String,
    pub aliases: Vec<String>,
    pub max_result_size: usize,
    pub is_concurrency_safe: bool,
    pub is_read_only: bool,
    pub is_destructive: bool,
}

impl ToolBuilder {
    pub fn new(name: &str, schema: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            input_schema: schema,
            toolset_name: "default".into(),
            aliases: vec![],
            max_result_size: 10000,
            is_concurrency_safe: true,
            is_read_only: false,
            is_destructive: false,
        }
    }
    
    pub fn description(mut self, desc: &str) -> Self { self.description = desc.into(); self }
    pub fn toolset(mut self, name: &str) -> Self { self.toolset_name = name.into(); self }
    pub fn concurrency_safe(mut self, val: bool) -> Self { self.is_concurrency_safe = val; self }
    pub fn read_only(mut self, val: bool) -> Self { self.is_read_only = val; self }
    pub fn destructive(mut self, val: bool) -> Self { self.is_destructive = val; self }
    pub fn max_result(mut self, size: usize) -> Self { self.max_result_size = size; self }
}
