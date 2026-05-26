use crate::platform::*;
use async_trait::async_trait;

#[derive(Debug)]
pub struct DiscordAdapter {
    pub bot_token: String,
}

impl DiscordAdapter {
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot_token: bot_token.into(),
        }
    }
}

#[async_trait]
impl PlatformAdapter for DiscordAdapter {
    fn name(&self) -> &str {
        "discord"
    }

    async fn connect(&self) -> Result<(), PlatformError> {
        tracing::info!("Discord adapter connecting...");
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), PlatformError> {
        tracing::info!("Discord adapter disconnecting");
        Ok(())
    }

    async fn send(&self, channel_id: &str, _message: &str) -> Result<SendResult, PlatformError> {
        // Discord API: POST /channels/{channel_id}/messages
        let _url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            channel_id
        );

        tracing::info!("Sending Discord message to {}", channel_id);

        Ok(SendResult { message_id: None })
    }

    async fn start_listening(
        &self,
        _handler: Box<dyn Fn(PlatformMessage) + Send + Sync>,
    ) -> Result<(), PlatformError> {
        // Discord Gateway/Webhook
        tracing::info!("Discord adapter starting to listen");
        Ok(())
    }
}
