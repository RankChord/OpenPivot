use agent_core::AgentEvent;
use agent_llm::{ChatMessage, MessageRole};
use chrono::Utc;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

// DB 连接封装
#[derive(Clone)]
pub struct SessionStore {
    pool: Arc<SqlitePool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub id: String,
    pub created_at: i64,
}

impl SessionStore {
    pub async fn new(db_path: &Path) -> Result<Self, sqlx::Error> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let db_url = format!("sqlite://{}", db_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(options).await?;

        // 初始化表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                sequence INTEGER NOT NULL,
                FOREIGN KEY(session_id) REFERENCES sessions(id)
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query("ALTER TABLE messages ADD COLUMN sequence INTEGER NOT NULL DEFAULT 0")
            .execute(&pool)
            .await
            .ok();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS events (
                session_id TEXT NOT NULL,
                event_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                sequence INTEGER NOT NULL,
                FOREIGN KEY(session_id) REFERENCES sessions(id)
            )",
        )
        .execute(&pool)
        .await?;

        // 创建索引以加速查询
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_messages_session 
             ON messages(session_id, created_at, sequence)",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_events_session 
             ON events(session_id, created_at, sequence)",
        )
        .execute(&pool)
        .await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// 确保 Session 存在
    pub async fn ensure_session(&self, session_id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().timestamp();
        sqlx::query("INSERT OR IGNORE INTO sessions (id, created_at) VALUES (?, ?)")
            .bind(session_id)
            .bind(now)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    /// 加载历史消息
    pub async fn load_history(&self, session_id: &str) -> Result<Vec<ChatMessage>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct Row {
            role: String,
            content: String,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT role, content FROM messages 
             WHERE session_id = ? 
             ORDER BY created_at ASC, sequence ASC",
        )
        .bind(session_id)
        .fetch_all(&*self.pool)
        .await?;

        let history = rows
            .into_iter()
            .map(|row| ChatMessage {
                role: match row.role.as_str() {
                    "user" => agent_llm::MessageRole::User,
                    "assistant" => agent_llm::MessageRole::Assistant,
                    _ => agent_llm::MessageRole::User, // Default fallback
                },
                content: Some(row.content),
                tool_calls: None,
                tool_call_id: None,
            })
            .collect();

        Ok(history)
    }

    /// 追加消息
    pub async fn append_messages(
        &self,
        session_id: &str,
        messages: &[ChatMessage],
    ) -> Result<(), sqlx::Error> {
        self.ensure_session(session_id).await?;

        let now = Utc::now().timestamp();
        for (sequence, msg) in messages.iter().enumerate() {
            if msg.content.is_none() && msg.tool_calls.is_none() {
                continue;
            }
            let text = msg.content.clone().unwrap_or_default();
            let role_str = match msg.role {
                agent_llm::MessageRole::User => "user",
                agent_llm::MessageRole::Assistant => "assistant",
                _ => continue,
            };

            sqlx::query(
                "INSERT INTO messages (session_id, role, content, created_at, sequence) 
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(session_id)
            .bind(role_str)
            .bind(text)
            .bind(now)
            .bind(sequence as i64)
            .execute(&*self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn append_events(
        &self,
        session_id: &str,
        events: &[AgentEvent],
    ) -> Result<(), sqlx::Error> {
        self.ensure_session(session_id).await?;

        let now = Utc::now().timestamp();
        for (sequence, event) in events.iter().enumerate() {
            let event_json =
                serde_json::to_string(event).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
            sqlx::query(
                "INSERT INTO events (session_id, event_json, created_at, sequence)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(session_id)
            .bind(event_json)
            .bind(now)
            .bind(sequence as i64)
            .execute(&*self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn load_events(&self, session_id: &str) -> Result<Vec<AgentEvent>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct Row {
            event_json: String,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT event_json FROM events
             WHERE session_id = ?
             ORDER BY created_at ASC, sequence ASC",
        )
        .bind(session_id)
        .fetch_all(&*self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                serde_json::from_str(&row.event_json)
                    .map_err(|e| sqlx::Error::Protocol(e.to_string()))
            })
            .collect()
    }

    pub async fn load_event_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        let events = self.load_events(session_id).await?;
        Ok(events
            .into_iter()
            .filter_map(|event| match event {
                AgentEvent::UserMessage { content } => Some(ChatMessage {
                    role: MessageRole::User,
                    content: Some(content),
                    tool_calls: None,
                    tool_call_id: None,
                }),
                AgentEvent::ToolResult {
                    id, content, error, ..
                } => Some(ChatMessage {
                    role: MessageRole::Tool,
                    content: Some(if content.is_empty() {
                        error.unwrap_or_default()
                    } else {
                        content
                    }),
                    tool_calls: None,
                    tool_call_id: Some(id),
                }),
                AgentEvent::Final { content } => Some(ChatMessage {
                    role: MessageRole::Assistant,
                    content: Some(content),
                    tool_calls: None,
                    tool_call_id: None,
                }),
                AgentEvent::ToolCall { .. } | AgentEvent::BudgetExhausted { .. } => None,
            })
            .collect())
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            created_at: i64,
        }

        let rows: Vec<Row> =
            sqlx::query_as("SELECT id, created_at FROM sessions ORDER BY created_at DESC, id ASC")
                .fetch_all(&*self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|row| SessionSummary {
                id: row.id,
                created_at: row.created_at,
            })
            .collect())
    }
}
