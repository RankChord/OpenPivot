use sqlx::PgPool;
use uuid::Uuid;

use crate::models::message::Message;

pub async fn create_message(
    pool: &PgPool,
    conversation_id: Uuid,
    sender_id: Uuid,
    content: &str,
) -> Result<Message, sqlx::Error> {
    let message_id = Uuid::now_v7();
    let message = sqlx::query_as::<_, Message>(
        r#"
        INSERT INTO messages (id, conversation_id, sender_id, content)
        VALUES ($1, $2, $3, $4)
        RETURNING
            id,
            conversation_id,
            sender_id,
            content,
            created_at
        "#,
    )
    .bind(message_id)
    .bind(conversation_id)
    .bind(sender_id)
    .bind(content)
    .fetch_one(pool)
    .await?;

    Ok(message)
}

pub async fn list_messages(
    pool: &PgPool,
    conversation_id: Uuid,
) -> Result<Vec<Message>, sqlx::Error> {
    let messages = sqlx::query_as::<_, Message>(
        r#"
        SELECT
            id,
            conversation_id,
            sender_id,
            content,
            created_at
        FROM messages
        WHERE conversation_id = $1
        ORDER BY created_at ASC
        LIMIT 100
        "#,
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;

    Ok(messages)
}