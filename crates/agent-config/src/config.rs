use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    pub model: ModelConfig,
    pub agent: AgentBehaviorConfig,
    pub paths: PathConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    pub provider: String,
    pub name: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AgentBehaviorConfig {
    pub max_iterations: u32,
    pub stream_mode: StreamMode,
    pub auto_approve_tools: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamMode {
    #[default]
    Full,
    Token,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathConfig {
    pub home: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisplayConfig {
    pub verbose: bool,
    pub colors: bool,
}
