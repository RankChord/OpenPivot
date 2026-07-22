use serde::{Deserialize, Serialize};

/// 注册请求体
#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    pub nickname: String,                                                    // 昵称
    pub username: String,                                                    // 用户名
    pub password: String,                                                    // 密码
}

/// 登录请求体
#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    pub username: String,                                                    // 用户名
    pub password: String,                                                    // 密码
}

// 刷新令牌请求体
#[derive(Serialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,                                                // 访问令牌
    pub refresh_token: String,                                               // 刷新令牌
    pub token_type: String,                                                  // 令牌类型
    pub expires_in: u64,                                                     // 过期时间
}

// 注册响应体
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: i64,                                                             // 用户ID
    pub username: String,                                                    // 用户名
    pub nickname: String,                                                    // 昵称
}

// 登录响应体
#[derive(Deserialize, Debug)]
pub struct RefreshRequest {
    pub refresh_token: String,                                               // 刷新令牌
}

// 获取当前用户信息响应体
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user_id: i64,                                                        // 用户ID 
}

// 注册请求体验证
impl RegisterRequest {
    pub fn validate(&self) -> bool {
        validate_username(&self.username)
            && validate_password(&self.password)
            && validate_nickname(&self.nickname)
    }
}

// 登录请求体验证
impl LoginRequest {
    pub fn validate(&self) -> bool {
        validate_username(&self.username)
            && validate_password(&self.password)
    }
}

// 刷新令牌请求体验证
fn validate_username(username: &str) -> bool {
    let len = username.chars().count();

    len >= 3
        && len <= 16
        && username.chars().all(|c| c.is_ascii_alphanumeric())
}

// 密码验证
fn validate_password(password: &str) -> bool {
    password.chars().count() >= 8
}

// 昵称验证
fn validate_nickname(nickname: &str) -> bool {
    let len = nickname.chars().count();
    len >= 1 && len <= 64
}

