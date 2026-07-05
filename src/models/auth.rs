use serde::{Deserialize, Serialize};

/// 注册请求体
#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    pub nickname: String,
    pub username: String,
    pub password: String,
}

/// 登录请求体
#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: i64,
    pub username: String,
    pub nickname: String,
}

#[derive(Deserialize, Debug)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user_id: i64,
}

impl RegisterRequest {
    pub fn validate(&self) -> bool {
        validate_username(&self.username)
            && validate_password(&self.password)
            && validate_nickname(&self.nickname)
    }
}

impl LoginRequest {
    pub fn validate(&self) -> bool {
        validate_username(&self.username)
            && validate_password(&self.password)
    }
}

fn validate_username(username: &str) -> bool {
    let len = username.chars().count();

    len >= 3
        && len <= 16
        && username.chars().all(|c| c.is_ascii_alphanumeric())
}

fn validate_password(password: &str) -> bool {
    password.chars().count() >= 8
}

fn validate_nickname(nickname: &str) -> bool {
    let len = nickname.chars().count();
    len >= 1 && len <= 64
}

