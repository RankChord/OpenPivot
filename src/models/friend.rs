use time::OffsetDateTime;

#[derive(Debug)]
pub enum FriendRequestStatus {
    Pending,
    Accepted,
    Rejected,
    Canceled,
}

impl FriendRequestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FriendRequestStatus::Pending => "pending",
            FriendRequestStatus::Accepted => "accepted",
            FriendRequestStatus::Rejected => "rejected",
            FriendRequestStatus::Canceled => "canceled",
        }
    }
}

impl TryFrom<&str> for FriendRequestStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(FriendRequestStatus::Pending),
            "accepted" => Ok(FriendRequestStatus::Accepted),
            "rejected" => Ok(FriendRequestStatus::Rejected),
            "canceled" => Ok(FriendRequestStatus::Canceled),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct FriendRequest {
    pub id: i64,
    pub requester_id: i64,
    pub addressee_id: i64,
    pub status: FriendRequestStatus,
    pub message: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateFriendRequest {
    pub user_id: i64,
    pub message: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct FriendRequestResponse {
    pub id: i64,
    pub requester_id: i64,
    pub addressee_id: i64,
    pub status: String,
    pub message: Option<String>,
}

impl FriendRequest {
    pub fn into_response(self) -> FriendRequestResponse {
        FriendRequestResponse {
            id: self.id,
            requester_id: self.requester_id,
            addressee_id: self.addressee_id,
            status: self.status.as_str().to_string(),
            message: self.message,
        }
    }
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct FriendItem {
    pub id: i64,
    pub username: String,
    pub nickname: String,
}
