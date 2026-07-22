use time::OffsetDateTime;

// 协作请求模型
#[derive(Debug)]
pub struct CollaboratorRequest {
    pub id: i64,                                                    // 协作请求的唯一标识符
    pub requester_id: i64,                                          // 发起协作请求的用户ID
    pub addressee_id: i64,                                          // 接收协作请求的用户ID
    pub status: CollaboratorRequestStatus,                          // 协作请求的状态
    pub message: Option<String>,                                    // 协作请求的附加信息
    pub created_at: OffsetDateTime,                                 // 协作请求的创建时间
    pub updated_at: OffsetDateTime,                                 // 协作请求的更新时间  
}

// 创建协作请求的请求体
#[derive(Debug, serde::Deserialize)]
pub struct CreateCollaboratorRequest {
    pub user_id: i64,                                               // 发起协作请求的用户ID
    pub message: Option<String>,                                    // 协作请求的附加信息    
}

// 协作请求响应体
#[derive(Debug, serde::Serialize)]
pub struct CollaboratorRequestResponse {
    pub id: i64,                                                    // 协作请求的唯一标识符
    pub requester_id: i64,                                          // 发起协作请求的用户ID
    pub addressee_id: i64,                                          // 接收协作请求的用户ID
    pub status: String,                                             // 协作请求的状态
    pub message: Option<String>,                                    // 协作请求的附加信息
}

// 协作请求列表响应体
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CollaboratorItem {
    pub id: i64,                                                    // 协作请求的唯一标识符
    pub username: String,                                           // 用户名
    pub nickname: String,                                           // 用户昵称 
}


// 协作请求状态枚举
#[derive(Debug)]
pub enum CollaboratorRequestStatus {
    Pending,                                                        // 请求待处理
    Accepted,                                                       // 请求已接受
    Rejected,                                                       // 请求已拒绝      
    Canceled,                                                       // 请求已取消  
}

// 协作请求状态类型转换:
// 转换: 请求状态枚举 -> 请求状态字符串
impl CollaboratorRequestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CollaboratorRequestStatus::Pending => "pending",
            CollaboratorRequestStatus::Accepted => "accepted",
            CollaboratorRequestStatus::Rejected => "rejected",
            CollaboratorRequestStatus::Canceled => "canceled",
        }
    }
}

// 协作请求状态类型转换:
// 转换: 请求状态字符串 -> 请求状态枚举
impl TryFrom<&str> for CollaboratorRequestStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(CollaboratorRequestStatus::Pending),
            "accepted" => Ok(CollaboratorRequestStatus::Accepted),
            "rejected" => Ok(CollaboratorRequestStatus::Rejected),
            "canceled" => Ok(CollaboratorRequestStatus::Canceled),
            _ => Err(()),
        }
    }
}

// 协作请求模型转换:
// 转换: 协作请求模型 -> 协作请求响应体
impl CollaboratorRequest {
    pub fn into_response(self) -> CollaboratorRequestResponse {
        CollaboratorRequestResponse {
            id: self.id,
            requester_id: self.requester_id,
            addressee_id: self.addressee_id,
            status: self.status.as_str().to_string(),
            message: self.message,
        }
    }
}
