use agent_llm::ChatMessage;
use agent_tools::{PermissionCheck, PermissionResult, ToolResult};
use std::path::PathBuf;

#[derive(Debug)]
pub enum AgentError {
    LlmError(String),
    ToolError(String),
    ConfigError(String),
    IoError(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::LlmError(e) => write!(f, "LLM error: {}", e),
            AgentError::ToolError(e) => write!(f, "Tool error: {}", e),
            AgentError::ConfigError(e) => write!(f, "Config error: {}", e),
            AgentError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for AgentError {}

#[derive(Debug)]
pub enum AgentOutput {
    Final(String),
    BudgetExhausted(String),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    UserMessage {
        content: String,
    },
    ToolCall {
        id: String,
        name: String,
    },
    ToolResult {
        id: String,
        ok: bool,
        content: String,
        error: Option<String>,
    },
    Final {
        content: String,
    },
    BudgetExhausted {
        reason: String,
    },
}

#[derive(Debug, Default)]
pub struct AgentEventCollector {
    events: Vec<AgentEvent>,
}

#[async_trait::async_trait]
pub trait AgentEventSink: Send {
    async fn emit(&mut self, event: AgentEvent) -> Result<(), AgentError>;
}

impl AgentEventCollector {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn events(&self) -> &[AgentEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<AgentEvent> {
        self.events
    }
}

#[async_trait::async_trait]
impl AgentEventSink for AgentEventCollector {
    async fn emit(&mut self, event: AgentEvent) -> Result<(), AgentError> {
        self.events.push(event);
        Ok(())
    }
}

pub struct CallbackEventSink<F>
where
    F: FnMut(AgentEvent) + Send,
{
    callback: F,
}

#[async_trait::async_trait]
impl<F> AgentEventSink for CallbackEventSink<F>
where
    F: FnMut(AgentEvent) + Send,
{
    async fn emit(&mut self, event: AgentEvent) -> Result<(), AgentError> {
        (self.callback)(event);
        Ok(())
    }
}

pub fn callback_event_sink<F>(callback: F) -> CallbackEventSink<F>
where
    F: FnMut(AgentEvent) + Send,
{
    CallbackEventSink { callback }
}

#[derive(Debug, Clone)]
pub struct AgentInput {
    pub user_message: String,
    pub session_id: String,
    pub system_prompt: String,
    pub history: Vec<ChatMessage>,
    pub workspace_dir: PathBuf,
}

pub fn build_api_messages(system_prompt: &str, history: &[ChatMessage]) -> Vec<ChatMessage> {
    let mut messages = Vec::new();

    messages.push(ChatMessage {
        role: agent_llm::MessageRole::System,
        content: Some(system_prompt.into()),
        tool_calls: None,
        tool_call_id: None,
    });

    messages.extend(history.iter().cloned());

    messages
}

pub fn tool_permission_for_policy(
    policy: &agent_config::PermissionPolicy,
    check: &PermissionCheck,
) -> PermissionResult {
    if policy.read_only && !check.is_read_only {
        return PermissionResult::Deny(format!(
            "Tool not allowed in read-only mode: {}",
            check.tool_name
        ));
    }

    if check.is_destructive && !check.is_read_only && !policy.allow_destructive_tools {
        return PermissionResult::Deny(format!(
            "Destructive tool not allowed by policy: {}",
            check.tool_name
        ));
    }

    if !policy.auto_approve_tools && !policy.allowed_tools.contains(&check.tool_name) {
        if policy.ask_tools.contains(&check.tool_name) {
            return PermissionResult::Ask(format!("Tool requires approval: {}", check.tool_name));
        }
        return PermissionResult::Deny(format!("Tool not allowed by policy: {}", check.tool_name));
    }

    if let Some(path) = check.input.get("path").and_then(|value| value.as_str()) {
        if !policy.allowed_dirs.is_empty() {
            let requested_path = std::path::PathBuf::from(path);
            let requested = match requested_path.canonicalize().or_else(|_| {
                requested_path
                    .parent()
                    .ok_or_else(|| std::io::Error::other("path has no parent"))?
                    .canonicalize()
            }) {
                Ok(path) => path,
                Err(error) => {
                    return PermissionResult::Deny(format!(
                        "Tool path cannot be resolved: {} ({})",
                        path, error
                    ));
                }
            };

            let allowed = policy.allowed_dirs.iter().any(|dir| {
                dir.canonicalize()
                    .map(|allowed_dir| requested.starts_with(allowed_dir))
                    .unwrap_or(false)
            });

            if !allowed {
                return PermissionResult::Deny(format!(
                    "Tool path outside allowed directories: {}",
                    requested.display()
                ));
            }
        }
    }

    PermissionResult::Allow
}

pub fn tool_result_message_content(result: &ToolResult) -> String {
    if !result.content.is_empty() {
        result.content.clone()
    } else {
        result.error.clone().unwrap_or_default()
    }
}
