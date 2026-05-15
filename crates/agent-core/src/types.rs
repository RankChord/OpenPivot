use agent_llm::ChatMessage;

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
