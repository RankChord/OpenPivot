use crate::provider::{MemoryProvider, MemoryError};
use std::path::Path;

pub struct MemoryManager {
    builtin: Option<Box<dyn MemoryProvider>>,
    external: Option<Box<dyn MemoryProvider>>,
    active_session: String,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            builtin: None,
            external: None,
            active_session: String::new(),
        }
    }

    pub fn register_builtin(&mut self, provider: Box<dyn MemoryProvider>) {
        self.builtin = Some(provider);
    }

    pub fn register_external(&mut self, provider: Box<dyn MemoryProvider>) {
        // At most ONE external provider
        self.external = Some(provider);
    }

    pub async fn initialize(&mut self, session_id: &str, workspace: &Path) -> Result<(), MemoryError> {
        self.active_session = session_id.into();

        if let Some(builtin) = &mut self.builtin {
            builtin.initialize(session_id, workspace).await?;
        }
        if let Some(external) = &mut self.external {
            external.initialize(session_id, workspace).await?;
        }
        Ok(())
    }

    pub async fn build_context_block(&mut self, query: &str) -> Result<String, MemoryError> {
        let mut block = String::new();

        if let Some(builtin) = &self.builtin {
            let builtin_block = builtin.system_prompt_block().await?;
            if !builtin_block.is_empty() {
                block.push_str(&builtin_block);
            }
        }

        if let Some(external) = &mut self.external {
            let query = query.to_string();
            let external_block = external.prefetch(&query).await?;
            if !external_block.is_empty() {
                block.push_str(&format!("<external-memory>\n{}\n</external-memory>\n", external_block));
            }
        }

        Ok(block)
    }

    pub async fn sync_turn(&self, user: &str, assistant: &str) -> Result<(), MemoryError> {
        if let Some(builtin) = &self.builtin {
            builtin.sync_turn(user, assistant).await?;
        }
        if let Some(external) = &self.external {
            external.sync_turn(user, assistant).await?;
        }
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), MemoryError> {
        if let Some(builtin) = &mut self.builtin {
            builtin.shutdown().await?;
        }
        if let Some(external) = &mut self.external {
            external.shutdown().await?;
        }
        Ok(())
    }
}

impl Default for MemoryManager {
    fn default() -> Self { Self::new() }
}
