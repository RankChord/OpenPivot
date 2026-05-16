//! Built-in memory: file-based (MEMORY.md, USER.md, SOUL.md, AGENTS.md)

use crate::provider::{MemoryProvider, MemoryError};
use std::path::{Path, PathBuf};
use async_trait::async_trait;

pub const MEMORY_FILES: &[&str] = &[
    "MEMORY.md",
    "USER.md",
    "SOUL.md",
    "AGENTS.md",
    "TOOLS.md",
];

#[derive(Debug)]
pub struct BuiltinMemory {
    session_id: Option<String>,
    workspace: PathBuf,
    last_prefetch: Option<String>,
}

impl BuiltinMemory {
    pub fn new(workspace: &Path) -> Self {
        Self {
            session_id: None,
            workspace: workspace.to_path_buf(),
            last_prefetch: None,
        }
    }

    pub async fn read_all(&self) -> Result<String, MemoryError> {
        let mut content = String::new();
        for file in MEMORY_FILES {
            let path = self.workspace.join(file);
            match tokio::fs::read_to_string(&path).await {
                Ok(text) => {
                    content.push_str(&format!("# {}\n\n{}\n\n", file, text));
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // File doesn't exist — skip silently
                }
                Err(e) => {
                    tracing::warn!("Failed to read {}: {}", file, e);
                }
            }
        }
        Ok(content)
    }

    pub async fn update_file(&self, filename: &str, content: &str) -> Result<(), MemoryError> {
        let path = self.workspace.join(filename);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, content).await?;
        Ok(())
    }
}

#[async_trait]
impl MemoryProvider for BuiltinMemory {
    fn name(&self) -> &str { "builtin" }

    async fn initialize(&mut self, session_id: &str, workspace: &Path) -> Result<(), MemoryError> {
        self.session_id = Some(session_id.into());
        self.workspace = workspace.to_path_buf();

        // Create default files if they don't exist
        for file in MEMORY_FILES {
            let path = self.workspace.join(file);
            if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
                tokio::fs::write(&path, format!("# {}\n\n", file)).await.ok();
            }
        }
        Ok(())
    }

    async fn prefetch(&mut self, _query: &str) -> Result<String, MemoryError> {
        let content = self.read_all().await?;
        self.last_prefetch = Some(content.clone());
        Ok(content)
    }

    fn queue_prefetch(&self, _query: String) {
        // In a real implementation, this would spawn a background task
    }

    async fn sync_turn(&self, _user: &str, _assistant: &str) -> Result<(), MemoryError> {
        // Builtin memory doesn't automatically sync turns
        Ok(())
    }

    async fn system_prompt_block(&self) -> Result<String, MemoryError> {
        let content = self.read_all().await?;
        if content.is_empty() {
            Ok(String::new())
        } else {
            Ok(format!("<memory-context>\n{}\n</memory-context>\n", content))
        }
    }

    async fn shutdown(&mut self) -> Result<(), MemoryError> {
        Ok(())
    }
}
