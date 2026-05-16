use crate::platform::*;
use async_trait::async_trait;

#[derive(Debug)]
pub struct TelegramAdapter {
    pub bot_token: String,
}

impl TelegramAdapter {
    pub fn new(bot_token: &str) -> Self {
        Self { bot_token: bot_token.into() }
    }
}

#[async_trait]
impl PlatformAdapter for TelegramAdapter {
    fn name(&self) -> &str { "telegram" }

    async fn connect(&self) -> Result<(), PlatformError> {
        tracing::info!("Telegram adapter connecting...");
        // In production, would call Telegram Webhook/LongPolling API
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), PlatformError> {
        tracing::info!("Telegram adapter disconnecting");
        Ok(())
    }

    async fn send(&self, chat_id: &str, message: &str) -> Result<SendResult, PlatformError> {
        // Telegram Bot API: POST /bot{token}/sendMessage
        let _url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let _body = serde_json::json!({
            "chat_id": chat_id,
            "text": message,
        });

        // Placeholder: in real impl, use reqwest to send
        tracing::info!("Sending Telegram message to {}", chat_id);

        Ok(SendResult {
            message_id: None,
        })
    }

    async fn start_listening(
        &self,
        _handler: Box<dyn Fn(PlatformMessage) + Send + Sync>,
    ) -> Result<(), PlatformError> {
        // Telegram long-polling or webhook
        tracing::info!("Telegram adapter starting to listen");
        Ok(())
    }
}
