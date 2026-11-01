use crate::models::actor::ActorStatus;
use time::OffsetDateTime;
use uuid::Uuid;

// 登陆凭证模型
#[derive(Debug)]
pub struct Credential {
    pub actor_id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub last_login_at: Option<OffsetDateTime>,
}

// 登陆成功的用户信息模型
#[derive(Debug)]
pub struct AuthenticatedActor {
    pub actor_id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub status: ActorStatus,
}