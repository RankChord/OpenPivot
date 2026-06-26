use time::OffsetDateTime;

use serde::Deserialize;

#[derive(Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub status: UserStatus,
    pub password_hash: String,
    pub avatar_url: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub last_login_at: Option<OffsetDateTime>,
}

#[derive(Debug)]
pub enum UserStatus {
    Active,
    Disabled,
    Deleted,
    Pending,
}


#[derive(Debug, Deserialize)]
pub struct SearchUsersQuery {
    pub q: String,
}

impl UserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserStatus::Active => "active",
            UserStatus::Disabled => "disabled",
            UserStatus::Deleted => "deleted",
            UserStatus::Pending => "pending",
        }
    }
}

impl TryFrom<&str> for UserStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(UserStatus::Active),
            "disabled" => Ok(UserStatus::Disabled),
            "deleted" => Ok(UserStatus::Deleted),
            "pending" => Ok(UserStatus::Pending),
            _ => Err(()),
        }
    }
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct UserSearchItem {
    pub id: i64,
    pub username: String,
    pub nickname: String,
}