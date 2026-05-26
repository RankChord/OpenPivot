pub mod config;
pub use config::*;

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file not found at {0}")]
    NotFound(PathBuf),
    #[error("failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig {
                provider: "openai".into(),
                name: "gpt-4".into(),
                base_url: None,
                api_key: std::env::var("AGENT_API_KEY").ok(),
                max_tokens: Some(4096),
                temperature: Some(0.7),
            },
            agent: AgentBehaviorConfig {
                max_iterations: 90,
                stream_mode: StreamMode::Full,
                auto_approve_tools: false,
            },
            paths: PathConfig {
                home: None,
                workspace: None,
            },
            display: DisplayConfig {
                verbose: false,
                colors: true,
            },
            permissions: PermissionPolicy::default(),
        }
    }
}

impl AgentConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let loaded: AgentConfig = toml::from_str(&content)?;
        Ok(loaded)
    }

    pub fn agent_home(&self) -> PathBuf {
        self.paths
            .home
            .clone()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".agent"))
    }

    pub fn workspace(&self) -> PathBuf {
        self.paths
            .workspace
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }
}
