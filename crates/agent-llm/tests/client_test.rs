use agent_llm::{LlmClientConfig, LlmError};

#[test]
fn test_client_creation() {
    let config = LlmClientConfig {
        base_url: "https://api.openai.com".into(),
        api_key: "test-key".into(),
        model: "gpt-4".into(),
        max_tokens: Some(4096),
        temperature: Some(0.7),
    };
    
    let _client = agent_llm::LlmClient::new(config);
    // Verify no panic - client created successfully
}

#[test]
fn test_config_defaults() {
    let config = LlmClientConfig {
        base_url: "http://localhost:11434".into(),
        api_key: "ollama".into(),
        model: "llama3".into(),
        max_tokens: None,
        temperature: None,
    };
    
    assert_eq!(config.base_url, "http://localhost:11434");
    assert!(config.max_tokens.is_none());
}
