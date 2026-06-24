use time::OffsetDateTime;

#[derive(Debug)]
pub enum SpaceType {
    Group,
    Workflow,
}

impl SpaceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpaceType::Group => "group",
            SpaceType::Workflow => "workflow",
        }
    }
}

impl TryFrom<&str> for SpaceType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "group" => Ok(SpaceType::Group),
            "workflow" => Ok(SpaceType::Workflow),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum SpaceMemberRole {
    Owner,
    Admin,
    Member,
}

impl SpaceMemberRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpaceMemberRole::Owner => "owner",
            SpaceMemberRole::Admin => "admin",
            SpaceMemberRole::Member => "member",
        }
    }
}

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

#[derive(Debug)]
pub struct Space {
    pub id: i64,
    pub name: String,
    pub space_type: SpaceType,
    pub owner_id: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct SpaceMember {
    pub id: i64,
    pub space_id: i64,
    pub user_id: i64,
    pub role: SpaceMemberRole,
    pub joined_at: OffsetDateTime,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SpaceMessage {
    pub id: i64,
    pub space_id: i64,
    pub sender_id: i64,
    pub content: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateSpaceRequest {
    pub name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SpaceResponse {
    pub id: i64,
    pub name: String,
    pub space_type: String,
    pub owner_id: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct AddSpaceMemberRequest {
    pub user_id: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct SpaceMemberResponse {
    pub id: i64,
    pub space_id: i64,
    pub user_id: i64,
    pub role: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateSpaceMessageRequest {
    pub content: String,
}

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