use sqlx::{FromRow, PgPool, PgExecutor};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    models::{
        actor::ActorStatus,
        credential::{AuthenticatedActor, Credential},
    },
};

// 登陆凭证模型
#[derive(Debug, FromRow)]
struct CredentialRow {
    actor_id: Uuid,
    username: String,
    password_hash: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    last_login_at: Option<OffsetDateTime>,
}

// 
impl From<CredentialRow> for Credential {
    fn from(row: CredentialRow) -> Self {
        Self {
            actor_id: row.actor_id,
            username: row.username,
            password_hash: row.password_hash,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_login_at: row.last_login_at,
        }
    }
}

// 
#[derive(Debug, FromRow)]
struct AuthenticatedActorRow {
    actor_id: Uuid,
    username: String,
    password_hash: String,
    status: String,
}

// 
impl TryFrom<AuthenticatedActorRow> for AuthenticatedActor {
    type Error = ();

    fn try_from(row: AuthenticatedActorRow) -> Result<Self, Self::Error> {
        Ok(Self {
            actor_id: row.actor_id,
            username: row.username,
            password_hash: row.password_hash,
            status: ActorStatus::try_from(row.status.as_str())?,
        })
    }
}

// 创建新凭证
pub async fn create_credential(
    executor: impl PgExecutor<'_>,
    actor_id: Uuid,
    username: &str,
    password_hash: &str,
) -> Result<Credential, sqlx::Error> {
    let row = sqlx::query_as::<_, CredentialRow>(
        r#"
        INSERT INTO credentials (
            actor_id,
            username,
            password_hash
        )
        VALUES ($1, $2, $3)
        RETURNING
            actor_id,
            username,
            password_hash,
            created_at,
            updated_at,
            last_login_at
        "#,
    )
    .bind(actor_id)
    .bind(username)
    .bind(password_hash)
    .fetch_one(executor)
    .await?;

    Ok(row.into())
}

// 查询已认证的用户信息
pub async fn find_authenticated_actor_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<AuthenticatedActor>, sqlx::Error> {
    let row = sqlx::query_as::<_, AuthenticatedActorRow>(
        r#"
        SELECT
            c.actor_id,
            c.username,
            c.password_hash,
            a.status
        FROM credentials AS c
        JOIN actors AS a ON a.id = c.actor_id
        WHERE c.username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    row.map(AuthenticatedActor::try_from)
        .transpose()
        .map_err(|_| sqlx::Error::RowNotFound)
}

// 查询已认证的用户信息
pub async fn find_authenticated_actor_by_id(
    pool: &PgPool,
    actor_id: Uuid,
) -> Result<Option<AuthenticatedActor>, sqlx::Error> {
    let row = sqlx::query_as::<_, AuthenticatedActorRow>(
        r#"
        SELECT
            c.actor_id,
            c.username,
            c.password_hash,
            a.status
        FROM credentials AS c
        JOIN actors AS a ON a.id = c.actor_id
        WHERE c.actor_id = $1
        "#,
    )
    .bind(actor_id)
    .fetch_optional(pool)
    .await?;

    row.map(AuthenticatedActor::try_from)
        .transpose()
        .map_err(|_| sqlx::Error::RowNotFound)
}

// 更新用户最后登录时间
pub async fn update_last_login_at(
    executor: impl PgExecutor<'_>,
    actor_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE credentials
        SET
            last_login_at = NOW(),
            updated_at = NOW()
        WHERE actor_id = $1
        "#,
    )
    .bind(actor_id)
    .execute(executor)
    .await?;

    Ok(())
}