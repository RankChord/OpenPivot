use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum DelegateMode {
    Single,
    Batch {
        tasks: Vec<DelegateTask>,
        max_concurrent: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateTask {
    pub goal: String,
    pub context: Option<String>,
    pub toolsets: Option<Vec<String>>,
    pub role: DelegateRole,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DelegateRole {
    Leaf,
    Orchestrator,
}

impl DelegateTask {
    pub fn new(goal: &str) -> Self {
        Self {
            goal: goal.into(),
            context: None,
            toolsets: None,
            role: DelegateRole::Leaf,
            timeout_seconds: None,
        }
    }

    pub fn role(mut self, role: DelegateRole) -> Self {
        self.role = role;
        self
    }

    pub fn context(mut self, ctx: &str) -> Self {
        self.context = Some(ctx.into());
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_seconds = Some(secs);
        self
    }
}
