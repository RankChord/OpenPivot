use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageRecord {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: String,
    pub tool_call_id: String,
    pub created_at: i64,
}

pub struct SessionStore {
    pool: SqlitePool,
}

impl SessionStore {
    pub async fn new(db_path: &Path) -> Result<Self, sqlx::Error> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT,
                tool_calls TEXT DEFAULT '',
                tool_call_id TEXT DEFAULT '',
                created_at INTEGER NOT NULL
            )"
        ).execute(&pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                platform TEXT, 
                chat_type TEXT,
                chat_id TEXT,
                created_at INTEGER NOT NULL,
                last_active INTEGER NOT NULL
            )"
        ).execute(&pool).await?;

        sqlx::query(
            "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                content, content_rowid=id
            )"
        ).execute(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn save_message(
        &self,
        msg: &MessageRecord,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO messages (id, session_id, role, content, tool_calls, tool_call_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&msg.id)
        .bind(&msg.session_id)
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(&msg.tool_calls)
        .bind(&msg.tool_call_id)
        .bind(msg.created_at)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_session_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<MessageRecord>, sqlx::Error> {
        let limit = limit.unwrap_or(100);
        sqlx::query_as::<_, MessageRecord>(
            "SELECT id, session_id, role, content, tool_calls, tool_call_id, created_at 
             FROM messages WHERE session_id = ? ORDER BY created_at ASC LIMIT ?"
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn search_messages(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<MessageRecord>, sqlx::Error> {
        let limit = limit.unwrap_or(20);
        sqlx::query_as::<_, MessageRecord>(
            "SELECT m.id, m.session_id, m.role, m.content, m.tool_calls, m.tool_call_id, m.created_at 
             FROM messages m 
             WHERE m.content LIKE ? 
             ORDER BY m.created_at DESC LIMIT ?"
        )
        .bind(format!("%{}%", query))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_sessions(&self) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT session_id FROM messages ORDER BY MAX(created_at) DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn create_session(
        &self,
        id: &str,
        platform: &str,
        chat_type: &str,
        chat_id: &str,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp() as i64;
        sqlx::query(
            "INSERT INTO sessions (id, platform, chat_type, chat_id, created_at, last_active) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(platform)
        .bind(chat_type)
        .bind(chat_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
