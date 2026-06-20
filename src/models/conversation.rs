use time::OffsetDateTime;

#[derive(Debug)]
pub enum ConversationType {
    Direct,
}

impl ConversationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversationType::Direct => "direct",
        }
    }
}

impl TryFrom<&str> for ConversationType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "direct" => Ok(ConversationType::Direct),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct Conversation {
    pub id: i64,
    pub conversation_type: ConversationType,
    pub user_low_id: i64,
    pub user_high_id: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateDirectConversationRequest {
    pub user_id: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct ConversationResponse {
    pub id: i64,
    pub conversation_type: String,
    pub user_low_id: i64,
    pub user_high_id: i64,
}

impl Conversation {
    pub fn into_response(self) -> ConversationResponse {
        ConversationResponse {
            id: self.id,
            conversation_type: self.conversation_type.as_str().to_string(),
            user_low_id: self.user_low_id,
            user_high_id: self.user_high_id,
        }
    }
}