use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default)]
    pub agent: AgentBehaviorConfig,
    #[serde(default)]
    pub paths: PathConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub permissions: PermissionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub name: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "openai".into(),
            name: "gpt-4".into(),
            base_url: None,
            api_key: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamMode {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "token")]
    Token,
}

impl Default for StreamMode {
    fn default() -> Self {
        StreamMode::Full
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBehaviorConfig {
    #[serde(default)]
    pub max_iterations: u32,
    #[serde(default)]
    pub stream_mode: StreamMode,
    #[serde(default)]
    pub auto_approve_tools: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathConfig {
    pub home: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    #[serde(default = "default_true")]
    pub auto_approve_tools: bool,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub allow_destructive_tools: bool,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub ask_tools: Vec<String>,
    #[serde(default)]
    pub allowed_dirs: Vec<PathBuf>,
}

fn default_true() -> bool {
    true
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            auto_approve_tools: false,
            read_only: false,
            allow_destructive_tools: false,
            allowed_tools: vec!["read_file".into(), "todo".into(), "browse_web".into()],
            ask_tools: vec![],
            allowed_dirs: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisplayConfig {
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub colors: bool,
}
