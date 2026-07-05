use time::OffsetDateTime;

use serde::Deserialize;

// 用户模型
#[derive(Debug)]
pub struct User {
    pub id: i64,                                    // 数据库用户id主键
    pub username: String,                           // 用户名
    pub nickname: String,                           // 昵称
    pub status: UserStatus,                         // 用户状态 (active, disabled, deleted, pending)
    pub password_hash: String,                      // 密码哈希值
    pub avatar_url: Option<String>,                 // 头像URL
    pub created_at: OffsetDateTime,                 // 创建时间
    pub updated_at: OffsetDateTime,                 // 更新时间
    pub last_login_at: Option<OffsetDateTime>,      // 上次登录时间
}

// 用户状态枚举
#[derive(Debug)]
pub enum UserStatus {
    Active,                                         // 活跃
    Inactive,                                       // 离线
    Disabled,                                       // 禁用
    Deleted,                                        // 已删除
    Pending,                                        // 待审核
}

// 用户搜索结果结构体
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct UserSearchItem {
    pub id: i64,                                    // 用户ID
    pub username: String,                           // 用户名
    pub nickname: String,                           // 昵称
    pub status: String,                             // 用户状态
    pub avatar_url: Option<String>,                 // 头像URL
}

// 用户搜索查询参数结构体
#[derive(Debug, Deserialize)]
pub struct SearchUsersQuery {
    pub q: String,
}

// 用户状态枚举方法:
// 转换: 用户状态枚举 -> 用户状态字符串
impl UserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserStatus::Active => "active",
            UserStatus::Inactive => "inactive",
            UserStatus::Disabled => "disabled",
            UserStatus::Deleted => "deleted",
            UserStatus::Pending => "pending",
        }
    }
}

// 实现用户状态接口:
// 转换: 用户状态字符串 -> 用户状态枚举
impl TryFrom<&str> for UserStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(UserStatus::Active),
            "inactive" => Ok(UserStatus::Inactive),
            "disabled" => Ok(UserStatus::Disabled),
            "deleted" => Ok(UserStatus::Deleted),
            "pending" => Ok(UserStatus::Pending),
            _ => Err(()),
        }
    }
}
