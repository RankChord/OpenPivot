use crate::types::*;
use reqwest::Client;

pub struct LlmClientConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

pub struct LlmClient {
    http: Client,
    config: LlmClientConfig,
}

impl LlmClient {
    pub fn new(config: LlmClientConfig) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }
    
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LlmError> {
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
}
