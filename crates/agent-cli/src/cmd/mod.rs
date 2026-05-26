use agent_config::AgentConfig;
use std::path::PathBuf;

pub mod chat;
pub mod cron;
pub mod doctor;
pub mod run;
pub mod serve;
pub mod sessions;
pub mod skills;
pub mod tools;

pub fn load_config(path: Option<PathBuf>) -> AgentConfig {
    let config_path = path.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".agent")
            .join("config.toml")
    });

    if config_path.exists() {
        match AgentConfig::from_file(&config_path) {
            Ok(c) => {
                tracing::debug!("Loaded config from {:?}", config_path);
                c
            }
            Err(e) => {
                tracing::warn!("Config parse error: {}", e);
                AgentConfig::default()
            }
        }
    } else {
        tracing::debug!("No config found, using defaults");
        AgentConfig::default()
    }
}
