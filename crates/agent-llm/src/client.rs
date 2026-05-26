use crate::types::*;
use reqwest::Client;

#[derive(Debug, Clone)]
pub enum LlmProviderType {
    OpenAI,
    Anthropic,
    Google,
}

impl Default for LlmProviderType {
    fn default() -> Self {
        Self::OpenAI
    }
}

pub struct LlmClientConfig {
    pub provider: LlmProviderType,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct OpenAiChatRequest {
    pub url: String,
    pub body: serde_json::Value,
}

pub fn build_openai_chat_request(
    config: &LlmClientConfig,
    messages: Vec<ChatMessage>,
    tools: Option<&[serde_json::Value]>,
) -> OpenAiChatRequest {
    let base_url = config.base_url.trim_end_matches('/');
    let path = if base_url.ends_with("/v1") {
        "/chat/completions"
    } else {
        "/v1/chat/completions"
    };

    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages,
    });

    if let Some(max_tokens) = config.max_tokens {
        body["max_tokens"] = max_tokens.into();
    }
    if let Some(temp) = config.temperature {
        body["temperature"] = temp.into();
    }

    if let Some(tools) = tools {
        if !tools.is_empty() {
            body["tools"] = serde_json::Value::Array(tools.to_vec());
            body["tool_choice"] = serde_json::json!("auto");
        }
    }

    OpenAiChatRequest {
        url: format!("{}{}", base_url, path),
        body,
    }
}

pub fn parse_openai_chat_response(json: serde_json::Value) -> Result<ChatResponse, LlmError> {
    let choices = json["choices"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| LlmError::ApiError("Empty choices in response".into()))?;
    let message_data = &choices["message"];

    let message = ChatMessage {
        role: MessageRole::Assistant,
        content: message_data["content"].as_str().map(|s| s.to_string()),
        tool_calls: if message_data["tool_calls"].is_array() {
            Some(
                serde_json::from_value(message_data["tool_calls"].clone())
                    .ok()
                    .unwrap_or_default(),
            )
        } else {
            None
        },
        tool_call_id: None,
    };

    let usage = if json["usage"].is_object() {
        Some(
            serde_json::from_value(json["usage"].clone())
                .ok()
                .unwrap_or_default(),
        )
    } else {
        None
    };

    Ok(ChatResponse {
        message,
        usage,
        finish_reason: choices["finish_reason"].as_str().map(|s| s.to_string()),
    })
}

pub struct LlmClient {
    pub(crate) http: Client,
    pub(crate) config: LlmClientConfig,
}

impl LlmClient {
    pub fn new(config: LlmClientConfig) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }

    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[serde_json::Value]>,
    ) -> Result<ChatResponse, LlmError> {
        match self.config.provider {
            LlmProviderType::OpenAI => {}
            LlmProviderType::Anthropic => {
                return Err(LlmError::UnsupportedProvider("anthropic".into()));
            }
            LlmProviderType::Google => {
                return Err(LlmError::UnsupportedProvider("google".into()));
            }
        }

        let request = build_openai_chat_request(&self.config, messages, tools);
        tracing::info!(url = %request.url, model = %self.config.model, "sending llm chat request");

        let mut req = self.http.post(&request.url);

        req = req.header("Authorization", format!("Bearer {}", self.config.api_key));

        req = req
            .header("Content-Type", "application/json")
            .json(&request.body);

        let response = req.send().await?;

        if response.status() == 429 {
            return Err(LlmError::RateLimit);
        }
        if response.status() == 401 {
            return Err(LlmError::AuthFailed);
        }
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("HTTP {}: {}", status, text)));
        }

        let json: serde_json::Value = response.json().await?;
        tracing::info!(model = %self.config.model, "received llm chat response");

        parse_openai_chat_response(json)
    }
}
