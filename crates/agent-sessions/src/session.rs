use crate::store::MessageRecord;
use agent_llm::ChatMessage;

pub fn chat_message_to_record(msg: &ChatMessage, session_id: &str) -> MessageRecord {
    MessageRecord {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        role: format!("{:?}", msg.role),
        content: msg.content.clone().unwrap_or_default(),
        tool_calls: msg.tool_calls.as_ref().map(|tc| serde_json::to_string(tc).unwrap_or_default()).unwrap_or_default(),
        tool_call_id: msg.tool_call_id.clone().unwrap_or_default(),
        created_at: chrono::Utc::now().timestamp() as i64,
    }
}
