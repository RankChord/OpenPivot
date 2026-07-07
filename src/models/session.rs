use sqlx::FromRow;
use time::OffsetDateTime;

// 用户会话模型
#[derive(Debug, FromRow)]
pub struct UserSession {
    pub id: i64,                                        // 数据库用户会话id主键
    pub user_id: i64,                                   // 用户id
    pub refresh_token_hash: String,                     // 刷新令牌哈希值
    pub expires_at: OffsetDateTime,                     // 会话过期时间
    pub revoked_at: Option<OffsetDateTime>,             // 会话撤销时间
    pub created_at: OffsetDateTime,                     // 会话创建时间
    pub last_used_at: Option<OffsetDateTime>,           // 会话上次使用时间
    pub user_agent: Option<String>,                     // 用户代理信息
    pub ip_address: Option<String>,                     // IP地址   
}