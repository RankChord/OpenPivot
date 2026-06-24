use sqlx::PgPool;
use time::OffsetDateTime;

use crate::models::space::{
    Space,
    SpaceMember,
    SpaceMemberRole,
    SpaceMessage,
    SpaceType,
};


#[derive(sqlx::FromRow)]
struct SpaceRow {
    id: i64,
    name: String,
    r#type: String,
    owner_id: i64,
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
            owner_id: row.owner_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SpaceMemberRow {
    id: i64,
    space_id: i64,
    user_id: i64,
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
            user_id: row.user_id,
            role,
            joined_at: row.joined_at,
        })
    }
}

pub async fn create_space(
    pool: &PgPool,
    owner_id: i64,
    name: &str,
) -> Result<Space, sqlx::Error> {
    let row = sqlx::query_as::<_, SpaceRow>(
        r#"
        INSERT INTO spaces (name, type, owner_id)
        VALUES ($1, 'group', $2)
        RETURNING
            id,
            name,
            type,
            owner_id,
            created_at,
            updated_at
        "#,
    )
    .bind(name)
    .bind(owner_id)
    .fetch_one(pool)
    .await?;

    let space = Space::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)?;

    Ok(space)
}

pub async fn add_space_member(
    pool: &PgPool,
    space_id: i64,
    user_id: i64,
    role: SpaceMemberRole,
) -> Result<SpaceMember, sqlx::Error> {
    let row = sqlx::query_as::<_, SpaceMemberRow>(
        r#"
        INSERT INTO space_members (space_id, user_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (space_id, user_id)
        DO UPDATE SET role = space_members.role
        RETURNING
            id,
            space_id,
            user_id,
            role,
            joined_at
        "#,
    )
    .bind(space_id)
    .bind(user_id)
    .bind(role.as_str())
    .fetch_one(pool)
    .await?;

    let member = SpaceMember::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)?;

    Ok(member)
}

pub async fn list_space_members(
    pool: &PgPool,
    space_id: i64,
) -> Result<Vec<SpaceMember>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SpaceMemberRow>(
        r#"
        SELECT
            id,
            space_id,
            user_id,
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
    user_id: i64,
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
        WHERE sm.user_id = $1
        ORDER BY s.updated_at DESC
        "#,
    )
    .bind(user_id)
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
    space_id: i64,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM space_members
            WHERE space_id = $1
              AND user_id = $2
        )
        "#,
    )
    .bind(space_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

pub async fn create_space_message(
    pool: &PgPool,
    space_id: i64,
    sender_id: i64,
    content: &str,
) -> Result<SpaceMessage, sqlx::Error> {
    let message = sqlx::query_as::<_, SpaceMessage>(
        r#"
        INSERT INTO space_messages (space_id, sender_id, content)
        VALUES ($1, $2, $3)
        RETURNING
            id,
            space_id,
            sender_id,
            content,
            created_at
        "#,
    )
    .bind(space_id)
    .bind(sender_id)
    .bind(content)
    .fetch_one(pool)
    .await?;

    Ok(message)
}

pub async fn list_space_messages(
    pool: &PgPool,
    space_id: i64,
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