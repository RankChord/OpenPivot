use sqlx::PgPool;
use time::OffsetDateTime;

use crate::models::session::UserSession;

pub async fn create_session(
    pool: &PgPool,
    user_id: i64,
    refresh_token_hash: &str,
    expires_at: OffsetDateTime,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<UserSession, sqlx::Error> {
    let session = sqlx::query_as::<_, UserSession>(
        r#"
        INSERT INTO user_sessions (user_id, refresh_token_hash, expires_at, user_agent, ip_address)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            id,
            user_id,
            refresh_token_hash,
            expires_at,
            revoked_at,
            created_at,
            last_used_at,
            user_agent,
            ip_address
        "#,
    )
    .bind(user_id)
    .bind(refresh_token_hash)
    .bind(expires_at)
    .bind(user_agent)
    .bind(ip_address)
    .fetch_one(pool)
    .await?;

    Ok(session)
}

pub async fn find_session_by_refresh_token_hash(
    pool: &PgPool,
    refresh_token_hash: &str,
) -> Result<Option<UserSession>, sqlx::Error> {
    let session = sqlx::query_as::<_, UserSession>(
        r#"
        SELECT
            id,
            user_id,
            refresh_token_hash,
            expires_at,
            revoked_at,
            created_at,
            last_used_at,
            user_agent,
            ip_address
        FROM user_sessions
        WHERE refresh_token_hash = $1
        "#,
    )
    .bind(refresh_token_hash)
    .fetch_optional(pool)
    .await?;

    Ok(session)
}

pub async fn revoke_session(
    pool: &PgPool,
    session_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE user_sessions
        SET revoked_at = NOW()
        WHERE id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(session_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn revoke_all_sessions_for_user(
    pool: &PgPool,
    user_id: i64,
) -> Result<(), sqlx::Error> {
     sqlx::query(
        r#"
        UPDATE user_sessions
        SET revoked_at = NOW()
        WHERE user_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}