use time::OffsetDateTime;
use serde_json::Value;
use serde::Deserialize;
use uuid::Uuid;

// 角色模型
#[derive(Debug)]
pub struct Actor {
    pub id: Uuid,                                   // Actor 主键
    pub actor_type: String,                         // 注册类型标签 (human, agent)
    pub display_name: String,                       // 显示名称
    pub avatar: Option<String>,                     // 头像URL
    pub status: ActorStatus,                        // Actor状态 (active, disabled, deleted, pending)
    pub profile_json: Value,                        // Actor资料的JSON字符串
    pub created_at: OffsetDateTime,                 // 创建时间
    pub updated_at: OffsetDateTime,                 // 更新时间
}

// 用户状态枚举
#[derive(Debug)]
pub enum ActorStatus {
    Active,                                         // 活跃
    Inactive,                                       // 离线
    Disabled,                                       // 禁用
    Deleted,                                        // 已删除
    Pending,                                        // 待审核
}

// 用户搜索结果结构体
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ActorSearchItem {
    pub id: Uuid,                                   // Actor 主键
    pub username: String,                           // 用户名
    pub display_name: String,                       // 显示名称
    pub status: String,                             // Actor状态
    pub avatar: Option<String>,                     // 头像URL
}

// 用户搜索查询参数结构体
#[derive(Debug, Deserialize)]
pub struct SearchActorsQuery {
    pub q: String,
}

// 用户状态枚举方法:
// 转换: 用户状态枚举 -> 用户状态字符串
impl ActorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Disabled => "disabled",
            Self::Deleted => "deleted",
            Self::Pending => "pending",
        }
    }
}

// 实现用户状态接口:
// 转换: 用户状态字符串 -> 用户状态枚举
impl TryFrom<&str> for ActorStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "disabled" => Ok(Self::Disabled),
            "deleted" => Ok(Self::Deleted),
            "pending" => Ok(Self::Pending),
            _ => Err(()),
        }
    }
}
