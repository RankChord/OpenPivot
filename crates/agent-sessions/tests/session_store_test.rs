use agent_core::AgentEvent;
use agent_llm::{ChatMessage, MessageRole};
use agent_sessions::SessionStore;
use std::str::FromStr;

#[tokio::test]
async fn creates_new_database_file_and_reloads_history() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("nested").join("sessions.db");

    let store = SessionStore::new(&db_path).await.unwrap();
    store
        .append_messages(
            "session-1",
            &[
                ChatMessage {
                    role: MessageRole::User,
                    content: Some("hello".into()),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: MessageRole::Assistant,
                    content: Some("hi".into()),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
        )
        .await
        .unwrap();

    let reopened = SessionStore::new(&db_path).await.unwrap();
    let history = reopened.load_history("session-1").await.unwrap();

    assert_eq!(history.len(), 2);
    assert_eq!(history[0].role, MessageRole::User);
    assert_eq!(history[0].content.as_deref(), Some("hello"));
    assert_eq!(history[1].role, MessageRole::Assistant);
    assert_eq!(history[1].content.as_deref(), Some("hi"));
}

#[tokio::test]
async fn appends_and_reloads_agent_events() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("sessions.db");
    let store = SessionStore::new(&db_path).await.unwrap();

    store
        .append_events(
            "session-1",
            &[
                AgentEvent::ToolCall {
                    id: "call_1".into(),
                    name: "read_file".into(),
                },
                AgentEvent::ToolResult {
                    id: "call_1".into(),
                    ok: false,
                    content: String::new(),
                    error: Some("denied".into()),
                },
            ],
        )
        .await
        .unwrap();

    let events = store.load_events("session-1").await.unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        AgentEvent::ToolCall {
            id: "call_1".into(),
            name: "read_file".into(),
        }
    );
    assert_eq!(
        events[1],
        AgentEvent::ToolResult {
            id: "call_1".into(),
            ok: false,
            content: String::new(),
            error: Some("denied".into()),
        }
    );
}

#[tokio::test]
async fn lists_sessions_from_database() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("sessions.db");
    let store = SessionStore::new(&db_path).await.unwrap();

    store.ensure_session("session-a").await.unwrap();
    store.ensure_session("session-b").await.unwrap();

    let sessions = store.list_sessions().await.unwrap();
    let ids: Vec<_> = sessions.into_iter().map(|session| session.id).collect();

    assert!(ids.contains(&"session-a".to_string()));
    assert!(ids.contains(&"session-b".to_string()));
}

#[tokio::test]
async fn opens_database_created_with_legacy_messages_schema() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("sessions.db");
    let db_url = format!("sqlite://{}", db_path.display());
    let options = sqlx::sqlite::SqliteConnectOptions::from_str(&db_url)
        .unwrap()
        .create_if_missing(true);
    let pool = sqlx::SqlitePool::connect_with(options).await.unwrap();
    sqlx::query("CREATE TABLE sessions (id TEXT PRIMARY KEY, created_at INTEGER NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE messages (
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let store = SessionStore::new(&db_path).await.unwrap();
    store
        .append_messages(
            "legacy-session",
            &[ChatMessage {
                role: MessageRole::User,
                content: Some("hello".into()),
                tool_calls: None,
                tool_call_id: None,
            }],
        )
        .await
        .unwrap();

    let history = store.load_history("legacy-session").await.unwrap();
    assert_eq!(history.last().unwrap().content.as_deref(), Some("hello"));
}

#[tokio::test]
async fn events_can_be_reconstructed_as_chat_history() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("sessions.db");
    let store = SessionStore::new(&db_path).await.unwrap();

    store
        .append_events(
            "session-1",
            &[
                AgentEvent::UserMessage {
                    content: "hello".into(),
                },
                AgentEvent::ToolResult {
                    id: "call_1".into(),
                    ok: true,
                    content: "file content".into(),
                    error: None,
                },
                AgentEvent::Final {
                    content: "answer".into(),
                },
            ],
        )
        .await
        .unwrap();

    let history = store.load_event_history("session-1").await.unwrap();

    assert_eq!(history.len(), 3);
    assert_eq!(history[0].role, MessageRole::User);
    assert_eq!(history[1].role, MessageRole::Tool);
    assert_eq!(history[2].role, MessageRole::Assistant);
}
