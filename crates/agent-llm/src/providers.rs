use crate::types::*;
use reqwest::Client;

pub struct LlmProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

pub struct LlmProviderClient {
    http: Client,
    config: LlmProviderConfig,
}

impl LlmProviderClient {
    pub fn new(config: LlmProviderConfig) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LlmError> {
        match self.config.provider.as_str() {
            "openai" | "ollama" => self.openai_compatible_chat(messages).await,
            "anthropic" => self.anthropic_chat(messages).await,
            other => Err(LlmError::ApiError(format!("Unknown provider: {}", other))),
        }
    }

    async fn openai_compatible_chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LlmError> {
        let base_url = self.config.base_url.trim_end_matches('/');
        let url = format!("{}/v1/chat/completions", base_url);

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
        });

        if let Some(max_tokens) = self.config.max_tokens {
            body["max_tokens"] = max_tokens.into();
        }
        if let Some(temp) = self.config.temperature {
            body["temperature"] = temp.into();
        }

        let response = self.http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

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

        let choices = json["choices"].as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| LlmError::ApiError("Empty choices in response".into()))?;
        let message_data = &choices["message"];

        let message = ChatMessage {
            role: MessageRole::Assistant,
            content: message_data["content"].as_str().map(|s| s.to_string()),
            tool_calls: if message_data["tool_calls"].is_array() {
                Some(serde_json::from_value(message_data["tool_calls"].clone()).ok().unwrap_or_default())
            } else {
                None
            },
            tool_call_id: None,
        };

        let usage = if json["usage"].is_object() {
            Some(serde_json::from_value(json["usage"].clone()).ok().unwrap_or_default())
        } else {
            None
        };

        Ok(ChatResponse {
            message,
            usage,
            finish_reason: choices["finish_reason"].as_str().map(|s| s.to_string()),
        })
    }

    async fn anthropic_chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LlmError> {
        let base_url = self.config.base_url.trim_end_matches('/');
        let url = format!("{}/v1/messages", base_url);

        let anthropic_messages: Vec<serde_json::Value> = messages.iter().map(|m| {
            let role = match m.role {
                MessageRole::User | MessageRole::System => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "user",
            };
            serde_json::json!({
                "role": role,
                "content": m.content.as_deref().unwrap_or("")
            })
        }).collect();

        let system_message = messages.iter()
            .find(|m| m.role == MessageRole::System)
            .and_then(|m| m.content.as_deref())
            .unwrap_or("");

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": anthropic_messages,
            "max_tokens": self.config.max_tokens.unwrap_or(4096),
            "system": system_message,
        });

        if let Some(temp) = self.config.temperature {
            body["temperature"] = temp.into();
        }

        let response = self.http
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

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

        let message = ChatMessage {
            role: MessageRole::Assistant,
            content: json["content"][0]["text"].as_str().map(|s| s.to_string()),
            tool_calls: None,
            tool_call_id: None,
        };

        let usage = if json["usage"].is_object() {
            Some(serde_json::from_value(json["usage"].clone()).ok().unwrap_or_default())
        } else {
            None
        };

        Ok(ChatResponse {
            message,
            usage,
            finish_reason: json["stop_reason"].as_str().map(|s| s.to_string()),
        })
    }
}
