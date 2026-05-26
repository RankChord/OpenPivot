use std::collections::HashMap;
use thiserror::Error;

pub type HookFn = Box<dyn Fn(&HookContext<'_>) -> Result<(), HookError> + Send + Sync>;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("hook failed: {0}")]
    ExecutionError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookPoint {
    PreToolCall,
    PostToolCall,
    PreLlmCall,
    PostLlmCall,
    OnSessionStart,
    OnSessionEnd,
    OnSessionReset,
    SubagentStop,
    PreApprovalRequest,
    PostApprovalResponse,
}

pub struct HookContext<'a> {
    pub hook_point: HookPoint,
    pub plugin_id: &'a str,
    pub session_id: &'a str,
    pub data: Option<&'a serde_json::Value>,
}

pub struct HookRegistry {
    hooks: HashMap<HookPoint, Vec<(String, HookFn)>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin_id: &str, point: HookPoint, func: HookFn) {
        self.hooks
            .entry(point)
            .or_insert_with(Vec::new)
            .push((plugin_id.into(), func));
    }

    pub fn execute(&self, point: HookPoint, ctx: &HookContext<'_>) -> Result<(), HookError> {
        if let Some(handlers) = self.hooks.get(&point) {
            for (id, func) in handlers {
                func(ctx)
                    .map_err(|e| HookError::ExecutionError(format!("Hook {} failed: {}", id, e)))?;
            }
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}
