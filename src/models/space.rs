use time::OffsetDateTime;
use uuid::Uuid;

// 空间模型
#[derive(Debug)]
pub struct Space {
    pub id: Uuid,                               // 数据库空间id主键
    pub name: String,                           // 空间名称
    pub space_type: SpaceType,                  // 空间类型 (public, private)
    pub owner_id: Uuid,                         // 空间所有者用户id 
    pub avatar_url: Option<String>,             // 空间头像URL
    pub created_at: OffsetDateTime,             // 创建时间
    pub updated_at: OffsetDateTime,             // 更新时间
}

// 空间成员模型
#[derive(Debug)]
pub struct SpaceMember {
    pub id: Uuid,                               // 数据库空间成员id主键
    pub space_id: Uuid,                         // 所属空间id
    pub actor_id: Uuid,                          // 成员用户id
    pub role: SpaceMemberRole,                  // 成员角色 (owner, admin, member)
    pub joined_at: OffsetDateTime,              // 加入时间
}

// 空间消息模型
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SpaceMessage {
    pub id: Uuid,                               // 数据库空间消息id主键
    pub space_id: Uuid,                         // 所属空间id
    pub sender_id: Uuid,                        // 发送者用户id
    pub content: String,                        // 消息内容
    pub created_at: OffsetDateTime,             // 创建时间
}

// 空间创建请求结构体
#[derive(Debug, serde::Deserialize)]
pub struct CreateSpaceRequest {
    pub name: String,                           // 空间名称
    pub space_type: String,                     // 空间类型 (public, private)
    pub avatar_url: Option<String>,             // 空间头像URL
}

// 空间消息创建请求结构体
#[derive(Debug, serde::Serialize)]
pub struct CreateSpaceResponse {
    pub id: Uuid,                               // 数据库空间id主键
    pub name: String,                           // 空间名称
    pub space_type: String,                     // 空间类型 (public, private)   
    pub owner_id: Uuid,                         // 空间所有者用户id
}

// 创建空间信息响应结构体
#[derive(Debug, serde::Deserialize)]
pub struct CreateSpaceMessageRequest {
    pub content: String,                        // 消息内容
}

// 空间添加成员请求结构体
#[derive(Debug, serde::Deserialize)]
pub struct AddSpaceMemberRequest {
    pub actor_id: Uuid,                          // 成员用户id
}

// 空间添加成员响应结构体
#[derive(Debug, serde::Serialize)]
pub struct AddSpaceMemberResponse {
    pub id: Uuid,                               // 数据库空间成员id主键
    pub space_id: Uuid,                         // 所属空间id
    pub actor_id: Uuid,                          // 成员用户id
    pub role: String,                           // 成员角色 (owner, admin, member)
}


// 空间类型枚举(联邦制预留)
#[derive(Debug)]
pub enum SpaceType {
    Public,                                     // 公开
    Private,                                    // 私有
}

// 空间成员角色枚举
#[derive(Debug)]
pub enum SpaceMemberRole {
    Owner,                                      // 所有者
    Admin,                                      // 管理员
    Member,                                     // 成员 
}

// 空间成员请求状态枚举
#[derive(Debug)]
pub enum SpaceMemberRequestStatus {
    Pending,
    Approved,
    Rejected,
}

// 空间成员申请模型
#[derive(Debug)]
pub struct SpaceMemberRequest {
    pub id: Uuid,
    pub space_id: Uuid,
    pub requester_actor_id: Uuid,
    pub target_actor_id: Uuid,
    pub status: SpaceMemberRequestStatus,
    pub reviewer_actor_id: Option<Uuid>,
    pub reviewed_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// 提交空间成员申请请求
#[derive(Debug, serde::Deserialize)]
pub struct CreateSpaceMemberRequest {
    pub actor_id: Uuid,
}

// 空间成员申请响应
#[derive(Debug, serde::Serialize)]
pub struct SpaceMemberRequestResponse {
    pub id: Uuid,
    pub space_id: Uuid,
    pub requester_actor_id: Uuid,
    pub target_actor_id: Uuid,
    pub status: String,
    pub reviewer_actor_id: Option<Uuid>,
    pub reviewed_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// 空间信息状态枚举
#[derive(Debug)]
pub enum SpaceMessageStatus {
    Normal,                                     // 正常
    Recalled,                                   // 撤回
}

// 空间枚举类型转换:
// 转换: 空间类型枚举 -> 空间类型字符串
impl SpaceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpaceType::Public => "public",
            SpaceType::Private => "private",
        }
    }
}

// 空间枚举类型转换:
// 转换: 空间类型字符串 -> 空间类型枚举
impl TryFrom<&str> for SpaceType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "public" => Ok(SpaceType::Public),
            "private" => Ok(SpaceType::Private),
            _ => Err(()),
        }
    }
}

// 空间成员角色枚举类型转换:
// 转换: 空间成员角色枚举 -> 空间成员角色字符串
impl SpaceMemberRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpaceMemberRole::Owner => "owner",
            SpaceMemberRole::Admin => "admin",
            SpaceMemberRole::Member => "member",
        }
    }
}

// 空间成员角色枚举类型转换:
// 转换: 空间成员角色字符串 -> 空间成员角色枚举
impl TryFrom<&str> for SpaceMemberRole {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "owner" => Ok(SpaceMemberRole::Owner),
            "admin" => Ok(SpaceMemberRole::Admin),
            "member" => Ok(SpaceMemberRole::Member),
            _ => Err(()),
        }
    }
}

// 空间枚举类型转换:
// 转换: 空间类型枚举 -> 空间类型字符串
impl SpaceMessageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpaceMessageStatus::Normal => "normal",
            SpaceMessageStatus::Recalled => "recalled",
        }
    }
}

// 空间枚举类型转换:
// 转换: 空间类型字符串 -> 空间类型枚举
impl TryFrom<&str> for SpaceMessageStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "normal" => Ok(SpaceMessageStatus::Normal),
            "recalled" => Ok(SpaceMessageStatus::Recalled),
            _ => Err(()),
        }
    }
}

// 实现空间模型的响应转换方法
impl Space {
    pub fn into_response(self) -> CreateSpaceResponse {
        CreateSpaceResponse {
            id: self.id,
            name: self.name,
            space_type: self.space_type.as_str().to_string(),
            owner_id: self.owner_id,
        }
    }
}

// 实现空间成员模型的响应转换方法
impl SpaceMember {
    pub fn into_response(self) -> AddSpaceMemberResponse {
        AddSpaceMemberResponse {
            id: self.id,
            space_id: self.space_id,
            actor_id: self.actor_id,
            role: self.role.as_str().to_string(),
        }
    }
}

// 实现空间成员请求状态枚举的转换方法
impl SpaceMemberRequestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

// 实现空间成员请求状态枚举的字符串转换方法
impl TryFrom<&str> for SpaceMemberRequestStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            _ => Err(()),
        }
    }
}

// 实现空间成员请求模型的响应转换方法
impl SpaceMemberRequest {
    pub fn into_response(self) -> SpaceMemberRequestResponse {
        SpaceMemberRequestResponse {
            id: self.id,
            space_id: self.space_id,
            requester_actor_id: self.requester_actor_id,
            target_actor_id: self.target_actor_id,
            status: self.status.as_str().to_string(),
            reviewer_actor_id: self.reviewer_actor_id,
            reviewed_at: self.reviewed_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}