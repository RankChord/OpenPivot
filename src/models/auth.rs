
use serde::{Deserialize, Debug};

/// 注册请求体
#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    pub nickname: String,
    pub username: String,
    pub password: String,
}
