use agent_config::{AgentConfig, StreamMode};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_default_config() {
    let config = AgentConfig::default();
    assert_eq!(config.model.provider, "openai");
    assert_eq!(config.agent.max_iterations, 90);
    assert!(matches!(config.agent.stream_mode, StreamMode::Full));
}

#[test]
fn test_load_config_from_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"
[model]
provider = "anthropic"
name = "claude-3-opus"

[agent]
max_iterations = 50
stream_mode = "token"
"#).unwrap();
    
    let config = AgentConfig::from_file(file.path()).unwrap();
    assert_eq!(config.model.provider, "anthropic");
    assert_eq!(config.model.name, "claude-3-opus");
    assert_eq!(config.agent.max_iterations, 50);
    assert!(matches!(config.agent.stream_mode, StreamMode::Token));
}

#[test]
fn test_invalid_config_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "invalid toml {{{{").unwrap();
    
    let result = AgentConfig::from_file(file.path());
    assert!(result.is_err());
}
