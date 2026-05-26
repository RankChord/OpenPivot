use agent_llm::{
    ChatMessage, LlmClient, LlmClientConfig, LlmError, LlmProviderType, MessageRole,
    build_openai_chat_request, parse_openai_chat_response,
};

#[test]
fn test_client_creation() {
    let config = LlmClientConfig {
        provider: LlmProviderType::OpenAI,
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
        provider: LlmProviderType::OpenAI,
        base_url: "http://localhost:11434".into(),
        api_key: "ollama".into(),
        model: "llama3".into(),
        max_tokens: None,
        temperature: None,
    };

    assert_eq!(config.base_url, "http://localhost:11434");
    assert!(config.max_tokens.is_none());
}

#[test]
fn test_provider_default_is_openai_compatible() {
    assert!(matches!(
        LlmProviderType::default(),
        LlmProviderType::OpenAI
    ));
}

#[test]
fn test_build_openai_chat_request_uses_v1_chat_completions() {
    let config = LlmClientConfig {
        provider: LlmProviderType::OpenAI,
        base_url: "https://example.com/v1".into(),
        api_key: "test-key".into(),
        model: "gpt-test".into(),
        max_tokens: Some(123),
        temperature: Some(0.2),
    };

    let request = build_openai_chat_request(
        &config,
        vec![ChatMessage {
            role: MessageRole::User,
            content: Some("hello".into()),
            tool_calls: None,
            tool_call_id: None,
        }],
        Some(&[serde_json::json!({
            "type": "function",
            "function": {"name": "read_file", "parameters": {"type": "object"}}
        })]),
    );

    assert_eq!(request.url, "https://example.com/v1/chat/completions");
    assert_eq!(request.body["model"], "gpt-test");
    assert_eq!(request.body["max_tokens"], 123);
    let temperature = request.body["temperature"].as_f64().unwrap();
    assert!((temperature - 0.2).abs() < 0.000001);
    assert_eq!(request.body["tool_choice"], "auto");
    assert_eq!(request.body["tools"].as_array().unwrap().len(), 1);
}

#[test]
fn test_parse_openai_chat_response_extracts_message_usage_and_tool_calls() {
    let response = parse_openai_chat_response(serde_json::json!({
        "choices": [{
            "message": {
                "content": "I'll read it.",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "read_file", "arguments": "{\"path\":\"README.md\"}"}
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
    }))
    .unwrap();

    assert_eq!(response.message.role, MessageRole::Assistant);
    assert_eq!(response.message.content.as_deref(), Some("I'll read it."));
    assert_eq!(response.message.tool_calls.as_ref().unwrap().len(), 1);
    assert_eq!(response.usage.unwrap().total_tokens, 15);
    assert_eq!(response.finish_reason.as_deref(), Some("tool_calls"));
}

#[tokio::test]
async fn test_anthropic_provider_reports_not_implemented_instead_of_using_wrong_parser() {
    let client = LlmClient::new(LlmClientConfig {
        provider: LlmProviderType::Anthropic,
        base_url: "https://api.anthropic.com".into(),
        api_key: "test-key".into(),
        model: "claude-test".into(),
        max_tokens: Some(100),
        temperature: None,
    });

    let error = client.chat(Vec::new(), None).await.unwrap_err();

    assert!(matches!(error, LlmError::UnsupportedProvider(_)));
}

#[tokio::test]
async fn test_google_provider_reports_not_implemented_instead_of_using_wrong_parser() {
    let client = LlmClient::new(LlmClientConfig {
        provider: LlmProviderType::Google,
        base_url: "https://generativelanguage.googleapis.com".into(),
        api_key: "test-key".into(),
        model: "gemini-test".into(),
        max_tokens: Some(100),
        temperature: None,
    });

    let error = client.chat(Vec::new(), None).await.unwrap_err();

    assert!(matches!(error, LlmError::UnsupportedProvider(_)));
}
