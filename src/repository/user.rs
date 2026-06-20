use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

use crate::models::user::{User, UserStatus, UserSearchItem};

#[derive(FromRow)]
struct UserRow {
    id: i64,
    username: String,
    nickname: String,
    password_hash: String,
    status: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    last_login_at: Option<OffsetDateTime>,
}

impl TryFrom<UserRow> for User {
    type Error = ();

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let status = UserStatus::try_from(row.status.as_str())?;

        Ok(User {
            id: row.id,
            username: row.username,
            nickname: row.nickname,
            password_hash: row.password_hash,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_login_at: row.last_login_at,
        })
    }
}

pub async fn create_user(
    pool: &PgPool,
    username: &str,
    nickname: &str,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"
        INSERT INTO users (username, nickname, password_hash)
        VALUES ($1, $2, $3)
        RETURNING
            id,
            username,
            nickname,
            password_hash,
            status,
            created_at,
            updated_at,
            last_login_at
        "#,
    )
    .bind(username)
    .bind(nickname)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;

    let user = User::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)?;

    Ok(user)
}

pub async fn find_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT
            id,
            username,
            nickname,
            password_hash,
            status,
            created_at,
            updated_at,
            last_login_at
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let user = User::try_from(row)
                .map_err(|_| sqlx::Error::RowNotFound)?;

            Ok(Some(user))
        }
        None => Ok(None),
    }
}

pub async fn find_user_by_id(
    pool: &PgPool,
    id: i64,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT
            id,
            username,
            nickname,
            password_hash,
            status,
            created_at,
            updated_at,
            last_login_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let user = User::try_from(row)
                .map_err(|_| sqlx::Error::RowNotFound)?;

            Ok(Some(user))
        }
        None => Ok(None),
    }
}


pub async fn update_last_login_at(
    pool: &PgPool,
    user_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET last_login_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn search_users(
    pool: &PgPool,
    current_user_id: i64,
    keyword: &str,
) -> Result<Vec<UserSearchItem>, sqlx::Error> {
    let pattern = format!("%{}%", keyword);

    let users = sqlx::query_as::<_, UserSearchItem>(
        r#"
        SELECT
            id,
            username,
            nickname
        FROM users
        WHERE id <> $1
          AND status = 'active'
          AND (
              username ILIKE $2
              OR nickname ILIKE $2
          )
        ORDER BY username ASC
        LIMIT 20
        "#,
    )
    .bind(current_user_id)
    .bind(pattern)
    .fetch_all(pool)
    .await?;

    Ok(users)
}