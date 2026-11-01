use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;
use crate::models::session::ActorSession;

pub async fn create_session(
    pool: &PgPool,
    actor_id: Uuid,
    refresh_token_hash: &str,
    expires_at: OffsetDateTime,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<ActorSession, sqlx::Error> {
    let session_id = Uuid::now_v7();
    let session = sqlx::query_as::<_, ActorSession>(
        r#"
        INSERT INTO actor_sessions (id, actor_id, refresh_token_hash, expires_at, user_agent, ip_address)
        VALUES ($1, $2, $3, $4, $5, $6::INET)
        RETURNING
            id,
            actor_id,
            refresh_token_hash,
            expires_at,
            revoked_at,
            created_at,
            last_used_at,
            user_agent,
            ip_address::TEXT AS ip_address
        "#,
    )
    .bind(session_id)
    .bind(actor_id)
    .bind(refresh_token_hash)
    .bind(expires_at)
    .bind(user_agent)
    .bind(ip_address)
    .fetch_one(pool)
    .await
    .map_err(|err| {
        eprintln!("[数据库管理]: 创建 session 失败: {:?}", err);
        err
    })?;

    Ok(session)
}

pub async fn find_session_by_refresh_token_hash(
    pool: &PgPool,
    refresh_token_hash: &str,
) -> Result<Option<ActorSession>, sqlx::Error> {
    let session = sqlx::query_as::<_, ActorSession>(
        r#"
        SELECT
            id,
            actor_id,
            refresh_token_hash,
            expires_at,
            revoked_at,
            created_at,
            last_used_at,
            user_agent,
            ip_address::TEXT AS ip_address
        FROM actor_sessions
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
    session_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE actor_sessions
        SET revoked_at = NOW()
        WHERE id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(session_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn revoke_all_sessions_for_actor(
    pool: &PgPool,
    actor_id: Uuid,
) -> Result<(), sqlx::Error> {
     sqlx::query(
        r#"
        UPDATE actor_sessions
        SET revoked_at = NOW()
        WHERE actor_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(actor_id)
    .execute(pool)
    .await?;

    Ok(())
}