use agent_llm::ChatMessage;
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

#[derive(Debug, Clone)]
pub struct AgentInput {
    pub user_message: String,
    pub session_id: String,
    pub system_prompt: String,
    pub history: Vec<ChatMessage>,
    pub workspace_dir: PathBuf,
}

pub fn build_api_messages(
    system_prompt: &str,
    user_message: &str,
    history: &[ChatMessage],
) -> Vec<ChatMessage> {
    let mut messages = Vec::new();

    messages.push(ChatMessage {
        role: agent_llm::MessageRole::System,
        content: Some(system_prompt.into()),
        tool_calls: None,
        tool_call_id: None,
    });

    messages.extend(history.iter().cloned());

    messages.push(ChatMessage {
        role: agent_llm::MessageRole::User,
        content: Some(user_message.into()),
        tool_calls: None,
        tool_call_id: None,
    });

    messages
}
