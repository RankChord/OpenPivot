use sqlx::PgPool;

use crate::models::message::Message;

pub async fn create_message(
    pool: &PgPool,
    conversation_id: i64,
    sender_id: i64,
    content: &str,
) -> Result<Message, sqlx::Error> {
    let message = sqlx::query_as::<_, Message>(
        r#"
        INSERT INTO messages (conversation_id, sender_id, content)
        VALUES ($1, $2, $3)
        RETURNING
            id,
            conversation_id,
            sender_id,
            content,
            created_at
        "#,
    )
    .bind(conversation_id)
    .bind(sender_id)
    .bind(content)
    .fetch_one(pool)
    .await?;

    Ok(message)
}

pub async fn list_messages(
    pool: &PgPool,
    conversation_id: i64,
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