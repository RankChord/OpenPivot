use time::OffsetDateTime;

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub sender_id: i64,
    pub content: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, serde::Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}