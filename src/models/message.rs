use time::OffsetDateTime;
use uuid::Uuid;

// 消息模型
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,                                   // 数据库消息id主键
    pub conversation_id: Uuid,                      // 所属会话id
    pub sender_id: Uuid,                            // 发送者用户id
    pub content: String,                            // 消息内容
    pub created_at: OffsetDateTime,                 // 创建时间
}

// 消息发送请求结构体
#[derive(Debug, serde::Deserialize)]
pub struct SendMessageRequest {
    pub content: String,                            // 消息内容
}