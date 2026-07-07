use time::OffsetDateTime;

// 空间模型
#[derive(Debug)]
pub struct Space {
    pub id: i64,                                // 数据库空间id主键
    pub name: String,                           // 空间名称
    pub space_type: SpaceType,                  // 空间类型 (public, private)
    pub owner_id: i64,                          // 空间所有者用户id 
    pub avatar_url: Option<String>,             // 空间头像URL
    pub created_at: OffsetDateTime,             // 创建时间
    pub updated_at: OffsetDateTime,             // 更新时间
}

// 空间成员模型
#[derive(Debug)]
pub struct SpaceMember {
    pub id: i64,                                // 数据库空间成员id主键
    pub space_id: i64,                          // 所属空间id
    pub user_id: i64,                           // 成员用户id
    pub role: SpaceMemberRole,                  // 成员角色 (owner, admin, member)
    pub joined_at: OffsetDateTime,              // 加入时间
}

// 空间消息模型
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SpaceMessage {
    pub id: i64,                                // 数据库空间消息id主键
    pub space_id: i64,                          // 所属空间id
    pub sender_id: i64,                         // 发送者用户id
    pub content: String,                        // 消息内容
    pub created_at: OffsetDateTime,             // 创建时间
    pub status: String,                         // 消息状态 (sent, delivered, read)
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
    pub id: i64,                                // 数据库空间id主键
    pub name: String,                           // 空间名称
    pub space_type: String,                     // 空间类型 (public, private)   
    pub owner_id: i64,                          // 空间所有者用户id
}

// 创建空间信息响应结构体
#[derive(Debug, serde::Deserialize)]
pub struct CreateSpaceMessageRequest {
    pub content: String,                        // 消息内容
}

// 空间添加成员请求结构体
#[derive(Debug, serde::Deserialize)]
pub struct AddSpaceMemberRequest {
    pub user_id: i64,                           // 成员用户id
}

// 空间添加成员响应结构体
#[derive(Debug, serde::Serialize)]
pub struct AddSpaceMemberResponse {
    pub id: i64,                                // 数据库空间成员id主键
    pub space_id: i64,                          // 所属空间id
    pub user_id: i64,                           // 成员用户id
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
            SpaceType::Public => "public",
            SpaceType::Private => "private",
        }
    }
}

// 空间枚举类型转换:
// 转换: 空间类型字符串 -> 空间类型枚举
impl TryFrom<&str> for SpaceMessageStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "public" => Ok(SpaceType::Public),
            "private" => Ok(SpaceType::Private),
            _ => Err(()),
        }
    }
}

// 实现空间模型的响应转换方法
impl Space {
    pub fn into_response(self) -> SpaceResponse {
        SpaceResponse {
            id: self.id,
            name: self.name,
            space_type: self.space_type.as_str().to_string(),
            owner_id: self.owner_id,
        }
    }
}

// 实现空间成员模型的响应转换方法
impl SpaceMember {
    pub fn into_response(self) -> SpaceMemberResponse {
        SpaceMemberResponse {
            id: self.id,
            space_id: self.space_id,
            user_id: self.user_id,
            role: self.role.as_str().to_string(),
        }
    }
}