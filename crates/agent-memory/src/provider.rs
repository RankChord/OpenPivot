use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not initialized")]
    NotInitialized,
    #[error("provider error: {0}")]
    Provider(String),
}

#[async_trait]
pub trait MemoryProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn initialize(
        &mut self,
        session_id: &str,
        workspace: &std::path::Path,
    ) -> Result<(), MemoryError>;

    async fn prefetch(&mut self, query: &str) -> Result<String, MemoryError>;

    fn queue_prefetch(&self, query: String);

    async fn sync_turn(&self, user: &str, assistant: &str) -> Result<(), MemoryError>;

    async fn system_prompt_block(&self) -> Result<String, MemoryError>;

    async fn shutdown(&mut self) -> Result<(), MemoryError>;
}
