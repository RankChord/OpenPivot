use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{self, MapAccess};
use std::fmt;

// 登录标识符枚举
#[derive(Debug)]
pub enum LoginIdentifier {
    Username(String),
    Id(i64),
}

/// 注册请求体
#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    pub nickname: String,                                                    // 昵称
    pub username: String,                                                    // 用户名
    pub password: String,                                                    // 密码
}

// 注册响应体
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: i64,                                                             // 用户ID
    pub username: String,                                                    // 用户名
    pub nickname: String,                                                    // 昵称
}

/// 登录请求体
#[derive(Debug)]
pub struct LoginRequest {
    pub identifier: LoginIdentifier,                                         // 登录标识符
    pub password: String,                                                    // 密码
}

// 刷新令牌请求体
#[derive(Deserialize, Debug)]
pub struct RefreshRequest {
    pub refresh_token: String,                                               // 刷新令牌
}

// 令牌响应体 登陆/刷新共用
#[derive(Serialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,                                                // 访问令牌
    pub refresh_token: String,                                               // 刷新令牌
    pub token_type: String,                                                  // 令牌类型
    pub expires_in: u64,                                                     // 过期时间
}

// 获取当前用户信息响应体
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: i64,                                                             // 用户ID 
}

// 注册请求体验证
impl RegisterRequest {
    pub fn validate(&self) -> bool {
        validate_username(&self.username)
            && validate_password(&self.password)
            && validate_nickname(&self.nickname)
    }
}


impl<'de> Deserialize<'de> for LoginRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LoginRequestVisitor;

        impl<'de> serde::de::Visitor<'de> for LoginRequestVisitor {
            type Value = LoginRequest;

            // 提示函数
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON object with 'password' and either 'username' (string) or 'id' (integer)")
            }

            fn visit_map<M>(self, mut map: M) -> Result<LoginRequest, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut id: Option<i64> = None;
                let mut username: Option<String> = None;
                let mut password: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "id" => {
                            if username.is_some() {
                                return Err(de::Error::custom("cannot provide both 'username' and 'id'"));
                            }
                            id = Some(map.next_value()?);
                        }
                        "username" => {
                            if id.is_some() {
                                return Err(de::Error::custom("cannot provide both 'username' and 'id'"));
                            }
                            username = Some(map.next_value()?);
                        }
                        "password" => {
                            password = Some(map.next_value()?);
                        }
                        other => {
                            return Err(de::Error::unknown_field(other, &["username", "id", "password"]));
                        }
                    }
                }

                let identifier = match (username, id) {
                    (Some(u), None) => LoginIdentifier::Username(u),
                    (None, Some(i)) => LoginIdentifier::Id(i),
                    (None, None) => return Err(de::Error::custom("must provide either 'username' or 'id'")),
                    (Some(_), Some(_)) => unreachable!(),
                };

                let password = password.ok_or_else(|| de::Error::missing_field("password"))?;

                Ok(LoginRequest { identifier, password })
            }
        }

        deserializer.deserialize_map(LoginRequestVisitor)
    }
}

// 登录请求体验证          
impl LoginRequest {
    pub fn validate(&self) -> bool {
        match &self.identifier {
            LoginIdentifier::Username(u) => validate_username(u) && validate_password(&self.password),
            LoginIdentifier::Id(_) => validate_password(&self.password),
        }
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

