use sqlx::{PgPool, PgExecutor};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::space::{
    Space,
    SpaceMember,
    SpaceMemberRequest,
    SpaceMemberRequestStatus,
    SpaceMemberRole,
    SpaceMessage,
    SpaceType,
};


#[derive(sqlx::FromRow)]
struct SpaceRow {
    id: Uuid,
    name: String,
    r#type: String,
    owner_id: Uuid,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<SpaceRow> for Space {
    type Error = ();

    fn try_from(row: SpaceRow) -> Result<Self, Self::Error> {
        let space_type = SpaceType::try_from(row.r#type.as_str())?;

        Ok(Space {
            id: row.id,
            name: row.name,
            space_type,
            avatar_url: None, // 目前没有从数据库中获取 avatar_url
            owner_id: row.owner_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SpaceMemberRow {
    id: Uuid,
    space_id: Uuid,
    actor_id: Uuid,
    role: String,
    joined_at: OffsetDateTime,
}

impl TryFrom<SpaceMemberRow> for SpaceMember {
    type Error = ();

    fn try_from(row: SpaceMemberRow) -> Result<Self, Self::Error> {
        let role = SpaceMemberRole::try_from(row.role.as_str())?;

        Ok(SpaceMember {
            id: row.id,
            space_id: row.space_id,
            actor_id: row.actor_id,
            role,
            joined_at: row.joined_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SpaceMemberRequestRow {
    id: Uuid,
    space_id: Uuid,
    requester_actor_id: Uuid,
    target_actor_id: Uuid,
    status: String,
    reviewer_actor_id: Option<Uuid>,
    reviewed_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<SpaceMemberRequestRow> for SpaceMemberRequest {
    type Error = ();

    fn try_from(row: SpaceMemberRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            space_id: row.space_id,
            requester_actor_id: row.requester_actor_id,
            target_actor_id: row.target_actor_id,
            status: SpaceMemberRequestStatus::try_from(row.status.as_str())?,
            reviewer_actor_id: row.reviewer_actor_id,
            reviewed_at: row.reviewed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn create_space(
    executor: impl PgExecutor<'_>,
    owner_id: Uuid,
    name: &str,
) -> Result<Space, sqlx::Error> {
    let space_id = Uuid::now_v7();
    let row = sqlx::query_as::<_, SpaceRow>(
        r#"
        INSERT INTO spaces (id, name, type, owner_id)
        VALUES ($1, $2, 'group', $3)
        RETURNING
            id,
            name,
            type,
            owner_id,
            created_at,
            updated_at
        "#,
    )
    .bind(space_id)
    .bind(name)
    .bind(owner_id)
    .fetch_one(executor)
    .await?;

    let space = Space::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)?;

    Ok(space)
}

pub async fn add_space_member(
    executor: impl PgExecutor<'_>,
    space_id: Uuid,
    actor_id: Uuid,
    role: SpaceMemberRole,
) -> Result<SpaceMember, sqlx::Error> {
    let member_id = Uuid::now_v7();
    let row = sqlx::query_as::<_, SpaceMemberRow>(
        r#"
        INSERT INTO space_members (id, space_id, actor_id, role)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (space_id, actor_id)
        DO UPDATE SET role = space_members.role
        RETURNING
            id,
            space_id,
            actor_id,
            role,
            joined_at
        "#,
    )
    .bind(member_id)
    .bind(space_id)
    .bind(actor_id)
    .bind(role.as_str())
    .fetch_one(executor)
    .await?;

    let member = SpaceMember::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)?;

    Ok(member)
}

pub async fn insert_space_member(
    executor: impl PgExecutor<'_>,
    space_id: Uuid,
    actor_id: Uuid,
    role: SpaceMemberRole,
) -> Result<SpaceMember, sqlx::Error> {
    let member_id = Uuid::now_v7();

    let row = sqlx::query_as::<_, SpaceMemberRow>(
        r#"
        INSERT INTO space_members (id, space_id, actor_id, role)
        VALUES ($1, $2, $3, $4)
        RETURNING
            id,
            space_id,
            actor_id,
            role,
            joined_at
        "#,
    )
    .bind(member_id)
    .bind(space_id)
    .bind(actor_id)
    .bind(role.as_str())
    .fetch_one(executor)
    .await?;

    SpaceMember::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)
}

pub async fn list_space_members(
    pool: &PgPool,
    space_id: Uuid,
) -> Result<Vec<SpaceMember>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SpaceMemberRow>(
        r#"
        SELECT
            id,
            space_id,
            actor_id,
            role,
            joined_at
        FROM space_members
        WHERE space_id = $1
        ORDER BY joined_at ASC
        "#,
    )
    .bind(space_id)
    .fetch_all(pool)
    .await?;

    let mut members = Vec::with_capacity(rows.len());

    for row in rows {
        let member = SpaceMember::try_from(row)
            .map_err(|_| sqlx::Error::RowNotFound)?;

        members.push(member);
    }

    Ok(members)
}

pub async fn list_my_spaces(
    pool: &PgPool,
    actor_id: Uuid,
) -> Result<Vec<Space>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SpaceRow>(
        r#"
        SELECT
            s.id,
            s.name,
            s.type,
            s.owner_id,
            s.created_at,
            s.updated_at
        FROM spaces s
        JOIN space_members sm ON sm.space_id = s.id
        WHERE sm.actor_id = $1
        ORDER BY s.updated_at DESC
        "#,
    )
    .bind(actor_id)
    .fetch_all(pool)
    .await?;

    let mut spaces = Vec::with_capacity(rows.len());

    for row in rows {
        let space = Space::try_from(row)
            .map_err(|_| sqlx::Error::RowNotFound)?;

        spaces.push(space);
    }

    Ok(spaces)
}

pub async fn is_space_member(
    pool: &PgPool,
    space_id: Uuid,
    actor_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM space_members
            WHERE space_id = $1
              AND actor_id = $2
        )
        "#,
    )
    .bind(space_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

pub async fn is_space_owner_or_admin(
    pool: &PgPool,
    space_id: Uuid,
    actor_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM space_members
            WHERE space_id = $1
              AND actor_id = $2
              AND role IN ('owner', 'admin')
        )
        "#,
    )
    .bind(space_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await
}

pub async fn create_space_message(
    executor: impl PgExecutor<'_>,
    space_id: Uuid,
    sender_id: Uuid,
    content: &str,
) -> Result<SpaceMessage, sqlx::Error> {
    let message_id = Uuid::now_v7();
    let message = sqlx::query_as::<_, SpaceMessage>(
        r#"
        INSERT INTO space_messages (id, space_id, sender_id, content)
        VALUES ($1, $2, $3, $4)
        RETURNING
            id,
            space_id,
            sender_id,
            content,
            created_at
        "#,
    )
    .bind(message_id)
    .bind(space_id)
    .bind(sender_id)
    .bind(content)
    .fetch_one(executor)
    .await?;

    Ok(message)
}

pub async fn create_space_member_request(
    pool: &PgPool,
    space_id: Uuid,
    requester_actor_id: Uuid,
    target_actor_id: Uuid,
) -> Result<SpaceMemberRequest, sqlx::Error> {
    let request_id = Uuid::now_v7();

    let row = sqlx::query_as::<_, SpaceMemberRequestRow>(
        r#"
        INSERT INTO space_member_requests (
            id,
            space_id,
            requester_actor_id,
            target_actor_id
        )
        VALUES ($1, $2, $3, $4)
        RETURNING
            id,
            space_id,
            requester_actor_id,
            target_actor_id,
            status,
            reviewer_actor_id,
            reviewed_at,
            created_at,
            updated_at
        "#,
    )
    .bind(request_id)
    .bind(space_id)
    .bind(requester_actor_id)
    .bind(target_actor_id)
    .fetch_one(pool)
    .await?;

    SpaceMemberRequest::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)
}

pub async fn list_space_messages(
    pool: &PgPool,
    space_id: Uuid,
) -> Result<Vec<SpaceMessage>, sqlx::Error> {
    let messages = sqlx::query_as::<_, SpaceMessage>(
        r#"
        SELECT
            id,
            space_id,
            sender_id,
            content,
            created_at
        FROM space_messages
        WHERE space_id = $1
        ORDER BY created_at ASC
        LIMIT 100
        "#,
    )
    .bind(space_id)
    .fetch_all(pool)
    .await?;

    Ok(messages)
}

pub async fn list_pending_space_member_requests(
    pool: &PgPool,
    space_id: Uuid,
) -> Result<Vec<SpaceMemberRequest>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SpaceMemberRequestRow>(
        r#"
        SELECT
            id,
            space_id,
            requester_actor_id,
            target_actor_id,
            status,
            reviewer_actor_id,
            reviewed_at,
            created_at,
            updated_at
        FROM space_member_requests
        WHERE space_id = $1
          AND status = 'pending'
        ORDER BY created_at ASC
        "#,
    )
    .bind(space_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            SpaceMemberRequest::try_from(row)
                .map_err(|_| sqlx::Error::RowNotFound)
        })
        .collect()
}

pub async fn approve_space_member_request(
    pool: &PgPool,
    request_id: Uuid,
    reviewer_actor_id: Uuid,
) -> Result<Option<SpaceMemberRequest>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, SpaceMemberRequestRow>(
        r#"
        UPDATE space_member_requests AS request
        SET
            status = 'approved',
            reviewer_actor_id = $2,
            reviewed_at = NOW(),
            updated_at = NOW()
        WHERE request.id = $1
          AND request.status = 'pending'
          AND EXISTS (
              SELECT 1
              FROM space_members AS reviewer
              WHERE reviewer.space_id = request.space_id
                AND reviewer.actor_id = $2
                AND reviewer.role IN ('owner', 'admin')
          )
          AND EXISTS (
              SELECT 1
              FROM actors AS target
              WHERE target.id = request.target_actor_id
                AND target.status = 'active'
          )
        RETURNING
            request.id,
            request.space_id,
            request.requester_actor_id,
            request.target_actor_id,
            request.status,
            request.reviewer_actor_id,
            request.reviewed_at,
            request.created_at,
            request.updated_at
        "#,
    )
    .bind(request_id)
    .bind(reviewer_actor_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let request = SpaceMemberRequest::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)?;

    insert_space_member(
        &mut *tx,
        request.space_id,
        request.target_actor_id,
        SpaceMemberRole::Member,
    )
    .await?;

    tx.commit().await?;

    Ok(Some(request))
}

pub async fn reject_space_member_request(
    pool: &PgPool,
    request_id: Uuid,
    reviewer_actor_id: Uuid,
) -> Result<Option<SpaceMemberRequest>, sqlx::Error> {
    let row = sqlx::query_as::<_, SpaceMemberRequestRow>(
        r#"
        UPDATE space_member_requests AS request
        SET
            status = 'rejected',
            reviewer_actor_id = $2,
            reviewed_at = NOW(),
            updated_at = NOW()
        WHERE request.id = $1
          AND request.status = 'pending'
          AND EXISTS (
              SELECT 1
              FROM space_members AS reviewer
              WHERE reviewer.space_id = request.space_id
                AND reviewer.actor_id = $2
                AND reviewer.role IN ('owner', 'admin')
          )
        RETURNING
            request.id,
            request.space_id,
            request.requester_actor_id,
            request.target_actor_id,
            request.status,
            request.reviewer_actor_id,
            request.reviewed_at,
            request.created_at,
            request.updated_at
        "#,
    )
    .bind(request_id)
    .bind(reviewer_actor_id)
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        SpaceMemberRequest::try_from(row)
            .map_err(|_| sqlx::Error::RowNotFound)
    })
    .transpose()
}