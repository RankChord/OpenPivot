use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, FromRow)]
pub struct UserSession {
    pub id: i64,
    pub user_id: i64,
    pub refresh_token_hash: String,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}