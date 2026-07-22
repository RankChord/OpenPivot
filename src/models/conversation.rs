use time::OffsetDateTime;

// 会话模型
#[derive(Debug)]
pub struct Conversation {
    pub id: i64,                                                                        // 会话的唯一标识符
    pub conversation_type: ConversationType,                                            // 会话类型
    pub user_low_id: i64,                                                               // 会话中用户ID较小的用户ID 
    pub user_high_id: i64,                                                              // 会话中用户ID较大的用户ID
    pub created_at: OffsetDateTime,                                                     // 会话的创建时间
    pub updated_at: OffsetDateTime,                                                     // 会话的更新时间
}

// 创建直接会话的请求体
#[derive(Debug, serde::Deserialize)]
pub struct CreateDirectConversationRequest {
    pub user_id: i64,                                                                   // 发起直接会话的用户ID
}

// 直接会话响应体
#[derive(Debug, serde::Serialize)]
pub struct ConversationResponse {
    pub id: i64,                                                                        // 会话的唯一标识符
    pub conversation_type: String,                                                      // 会话类型
    pub user_low_id: i64,                                                               // 会话中用户ID较小的用户ID
    pub user_high_id: i64,                                                              // 会话中用户ID较大的用户ID
}

// 会话类型枚举
#[derive(Debug)]
pub enum ConversationType {
    Direct,                                                                             // 直接会话
}

// 会话类型转换
// 转换: 会话类型枚举 -> 会话类型字符串
impl ConversationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversationType::Direct => "direct",
        }
    }
}

// 会话类型转换
// 转换: 会话类型字符串 -> 会话类型枚举
impl TryFrom<&str> for ConversationType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "direct" => Ok(ConversationType::Direct),
            _ => Err(()),
        }
    }
}

// 会话模型转换
// 转换: 会话模型 -> 会话响应体
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