use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;
use serde_json::Value;
use crate::models::actor::{Actor, ActorStatus, ActorSearchItem};

// 角色模型
#[derive(FromRow)]
struct ActorRow {
    id: Uuid,
    actor_type: String,
    display_name: String,
    avatar: Option<String>,
    status: String,
    profile_json: Value,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

// 用户状态枚举方法:
// 转换: 用户状态字符串 -> 用户状态枚举
impl TryFrom<ActorRow> for Actor {
    type Error = ();

    fn try_from(row: ActorRow) -> Result<Self, Self::Error> {
        Ok(Actor {
            id: row.id,
            actor_type: row.actor_type,
            display_name: row.display_name,
            avatar: row.avatar,
            status: ActorStatus::try_from(row.status.as_str())?,
            profile_json: row.profile_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

// 创建新用户
pub async fn create_actor(
    pool: &PgPool,
    actor_type: &str,
    display_name: &str,
) -> Result<Actor, sqlx::Error> {
    let actor_id = Uuid::now_v7();

    let row = sqlx::query_as::<_, ActorRow>(
        r#"
        INSERT INTO actors (id, type, display_name)
        VALUES ($1, $2, $3)
        RETURNING
            id,
            type AS actor_type,
            display_name,
            avatar,
            status,
            profile_json,
            created_at,
            updated_at
        "#,
    )
    .bind(actor_id)
    .bind(actor_type)
    .bind(display_name)
    .fetch_one(pool)
    .await?;

    Actor::try_from(row).map_err(|_| sqlx::Error::RowNotFound)
}

// 查询用户信息
pub async fn find_actor_by_id(
    pool: &PgPool,
    actor_id: Uuid,
) -> Result<Option<Actor>, sqlx::Error> {
    let row = sqlx::query_as::<_, ActorRow>(
        r#"
        SELECT
            id,
            type AS actor_type,
            display_name,
            avatar,
            status,
            profile_json,
            created_at,
            updated_at
        FROM actors
        WHERE id = $1
        "#,
    )
    .bind(actor_id)
    .fetch_optional(pool)
    .await?;

    row.map(Actor::try_from)
        .transpose()
        .map_err(|_| sqlx::Error::RowNotFound)
}

// 搜索用户
pub async fn search_actors(
    pool: &PgPool,
    current_actor_id: Uuid,
    keyword: &str,
) -> Result<Vec<ActorSearchItem>, sqlx::Error> {
    let pattern = format!("%{keyword}%");

    sqlx::query_as::<_, ActorSearchItem>(
        r#"
        SELECT
            a.id,
            c.username,
            a.display_name,
            a.status,
            a.avatar
        FROM actors AS a
        JOIN credentials AS c ON c.actor_id = a.id
        WHERE a.id <> $1
          AND a.status = 'active'
          AND (
              c.username ILIKE $2
              OR a.display_name ILIKE $2
          )
        ORDER BY c.username ASC
        LIMIT 20
        "#,
    )
    .bind(current_actor_id)
    .bind(pattern)
    .fetch_all(pool)
    .await
}