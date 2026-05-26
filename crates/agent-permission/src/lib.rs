use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    /// Whether to require user approval for tools
    #[serde(default)]
    pub auto_approve_tools: bool,

    /// Allowed tool names
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Allowed working directories
    #[serde(default)]
    pub allowed_dirs: Vec<PathBuf>,
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            auto_approve_tools: false,
            allowed_tools: vec!["read_file".into(), "todo".into(), "web_search".into()],
            allowed_dirs: vec![std::env::current_dir().unwrap_or_default()],
        }
    }
}

pub fn check_permission(policy: &PermissionPolicy, tool_name: &str, _work_dir: &PathBuf) -> bool {
    // 1. If auto approve, return true
    if policy.auto_approve_tools {
        return true;
    }

    // 2. Explicitly allowed list
    policy.allowed_tools.contains(&tool_name.to_string())
}
