use crate::task::{DelegateRole, DelegateTask};

#[derive(Debug, thiserror::Error)]
pub enum DelegateError {
    #[error("max spawn depth exceeded")]
    MaxDepthExceeded,
    #[error("max concurrent children exceeded")]
    MaxConcurrentExceeded,
    #[error("leaf agent cannot delegate")]
    LeafCannotDelegate,
    #[error("timeout after {0}s")]
    Timeout(u64),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone)]
pub struct DelegateConfig {
    pub max_concurrent_children: u32,
    pub max_spawn_depth: u32,
    pub child_timeout_seconds: u64,
    pub orchestrator_enabled: bool,
}

impl Default for DelegateConfig {
    fn default() -> Self {
        Self {
            max_concurrent_children: 3,
            max_spawn_depth: 2,
            child_timeout_seconds: 300,
            orchestrator_enabled: true,
        }
    }
}

pub struct DelegateExecutor {
    config: DelegateConfig,
    current_depth: u32,
    active_children: u32,
}

impl DelegateExecutor {
    pub fn new(config: DelegateConfig) -> Self {
        Self {
            config,
            current_depth: 0,
            active_children: 0,
        }
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.current_depth = depth;
        self
    }

    pub fn can_delegate(&self, role: &DelegateRole) -> Result<(), DelegateError> {
        match role {
            DelegateRole::Leaf => Err(DelegateError::LeafCannotDelegate),
            DelegateRole::Orchestrator => {
                if !self.config.orchestrator_enabled {
                    return Err(DelegateError::LeafCannotDelegate);
                }
                if self.current_depth >= self.config.max_spawn_depth {
                    return Err(DelegateError::MaxDepthExceeded);
                }
                if self.active_children >= self.config.max_concurrent_children {
                    return Err(DelegateError::MaxConcurrentExceeded);
                }
                Ok(())
            }
        }
    }

    pub fn spawn(&mut self, task: &DelegateTask) -> Result<SpawnHandle, DelegateError> {
        self.can_delegate(&task.role)?;
        self.active_children += 1;

        Ok(SpawnHandle {
            id: uuid::Uuid::new_v4().to_string(),
            depth: self.current_depth + 1,
        })
    }

    pub fn complete(&mut self, _handle: &SpawnHandle) {
        if self.active_children > 0 {
            self.active_children -= 1;
        }
    }

    pub fn active_children_count(&self) -> u32 {
        self.active_children
    }

    pub fn child_config(&self, depth: u32) -> DelegateConfig {
        DelegateConfig {
            max_spawn_depth: self.config.max_spawn_depth.saturating_sub(depth),
            ..self.config.clone()
        }
    }
}

pub struct SpawnHandle {
    pub id: String,
    pub depth: u32,
}

#[cfg(test)]
mod tests {}
