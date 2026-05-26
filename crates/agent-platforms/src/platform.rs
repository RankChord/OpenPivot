use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMessage {
    pub chat_id: String,
    pub content: String,
    pub user_id: String,
    pub platform: String,
}

#[derive(Debug)]
pub struct SendResult {
    pub message_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("connection error: {0}")]
    ConnectionError(String),
    #[error("send error: {0}")]
    SendError(String),
}

#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    fn name(&self) -> &str;

    async fn connect(&self) -> Result<(), PlatformError>;
    async fn disconnect(&self) -> Result<(), PlatformError>;

    async fn send(&self, chat_id: &str, message: &str) -> Result<SendResult, PlatformError>;

    async fn start_listening(
        &self,
        handler: Box<dyn Fn(PlatformMessage) + Send + Sync>,
    ) -> Result<(), PlatformError>;

    fn show_typing_indicator(&self, _chat_id: &str) {}
}
