use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::conversation::{
    Conversation,
    ConversationType,
};

#[derive(sqlx::FromRow)]
struct ConversationRow {
    id: Uuid,
    r#type: String,
    actor_low_id: Uuid,
    actor_high_id: Uuid,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<ConversationRow> for Conversation {
    type Error = ();

    fn try_from(row: ConversationRow) -> Result<Self, Self::Error> {
        let conversation_type = ConversationType::try_from(row.r#type.as_str())?;

        Ok(Conversation {
            id: row.id,
            conversation_type,
            actor_low_id: row.actor_low_id,
            actor_high_id: row.actor_high_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn create_or_get_direct_conversation(
    pool: &PgPool,
    actor_a_id: Uuid,
    actor_b_id: Uuid,
) -> Result<Conversation, sqlx::Error> {
    let actor_low_id = actor_a_id.min(actor_b_id);
    let actor_high_id = actor_a_id.max(actor_b_id);
    let conversation_id = Uuid::now_v7();

    let row = sqlx::query_as::<_, ConversationRow>(
        r#"
        INSERT INTO conversations (id, type, actor_low_id, actor_high_id)
        VALUES ($1, 'direct', $2, $3)
        ON CONFLICT (actor_low_id, actor_high_id)
        DO UPDATE SET updated_at = conversations.updated_at
        RETURNING
            id,
            type,
            actor_low_id,
            actor_high_id,
            created_at,
            updated_at
        "#,
    )
    .bind(conversation_id)
    .bind(actor_low_id)
    .bind(actor_high_id)
    .fetch_one(pool)
    .await?;

    let conversation = Conversation::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)?;

    Ok(conversation)
}

pub async fn actor_in_conversation(
    pool: &PgPool,
    conversation_id: Uuid,
    actor_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM conversations
            WHERE id = $1
              AND (actor_low_id = $2 OR actor_high_id = $2)
        )
        "#,
    )
    .bind(conversation_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

pub async fn list_conversations(
    pool: &PgPool,
    current_actor_id: Uuid,
) -> Result<Vec<Conversation>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ConversationRow>(
        r#"
        SELECT
            id,
            type,
            actor_low_id,
            actor_high_id,
            created_at,
            updated_at
        FROM conversations
        WHERE actor_low_id = $1
           OR actor_high_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(current_actor_id)
    .fetch_all(pool)
    .await?;

    let mut conversations = Vec::with_capacity(rows.len());

    for row in rows {
        let conversation = Conversation::try_from(row)
            .map_err(|_| sqlx::Error::RowNotFound)?;

        conversations.push(conversation);
    }

    Ok(conversations)
}