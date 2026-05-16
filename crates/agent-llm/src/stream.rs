//! Streaming support for LLM responses

use crate::types::*;
use crate::client::LlmClient;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Streaming chunk from LLM
#[derive(Debug, Clone)]
pub struct ChatStreamChunk {
    pub text: String,
    pub finish_reason: Option<String>,
}

/// Streaming LLM response
pub struct ChatStream {
    stream: Pin<Box<dyn Stream<Item = Result<ChatStreamChunk, LlmError>> + Send>>,
}

impl Stream for ChatStream {
    type Item = Result<ChatStreamChunk, LlmError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx)
    }
}

impl LlmClient {
    /// Streaming chat completion
    pub async fn chat_stream(&self, messages: Vec<ChatMessage>) -> Result<ChatStream, LlmError> {
        let base_url = self.config.base_url.trim_end_matches('/');
        let url = format!("{}/v1/chat/completions", base_url);

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(max_tokens) = self.config.max_tokens {
            body["max_tokens"] = max_tokens.into();
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
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("HTTP {}: {}", status, text)));
        }

        // Parse SSE stream
        let stream = response.bytes_stream();
        
        use futures::StreamExt;
        let chat_stream = stream.filter_map(|chunk| async {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => return Some(Err(LlmError::NetworkError(e))),
            };
            
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data.trim() == "[DONE]" {
                        return None;
                    }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        let text = parsed["choices"][0]["delta"]["content"].as_str().unwrap_or("").to_string();
                        let finish = parsed["choices"][0]["finish_reason"].as_str().map(|s| s.to_string());
                        return Some(Ok(ChatStreamChunk {
                            text,
                            finish_reason: finish,
                        }));
                    }
                }
            }
            None
        });

        Ok(ChatStream {
            stream: Box::pin(chat_stream),
        })
    }
}
