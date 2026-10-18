//! 工作区业务入口。
//!
//! MCP 和管理 HTTP API 共用本层，因此权限、版本和状态转换规则不会各自实现一份。
//! 每次业务调用使用 SQLite IMMEDIATE 事务：输出文件、任务状态、回执、幂等记录
//! 要么一起提交，要么一起回滚。网络层只负责鉴权与协议适配。
mod agents;
mod dedup;
mod files;
mod messages;
mod models;
mod schema;
mod tasks;

use base64::{Engine, engine::general_purpose::STANDARD};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

use agents::{agent_available, agent_exists};
use dedup::{cache, cached};
use files::put_file;
use messages::message;
use models::*;

const MAX_FILE_BYTES: usize = 1024 * 1024;
/// 仅管理 API 可用的内部身份。它不是 Agent，也不能用 MCP token 登录。
const OPERATOR: &str = "__operator";

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("{0}")]
    Rule(String),
    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("invalid arguments: {0}")]
    Json(#[from] serde_json::Error),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
}
type Result<T> = std::result::Result<T, WorkspaceError>;

fn ensure(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(WorkspaceError::Rule(message.into()))
    }
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn digest(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T> {
    Ok(serde_json::from_value(value)?)
}
fn required(text: &str, label: &str, max: usize) -> Result<()> {
    ensure(
        !text.trim().is_empty() && text.len() <= max,
        &format!("{label} must contain 1..{max} bytes"),
    )
}

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

impl Store {
    /// 打开数据库并执行兼容迁移；新空间不预设 Agent 数量或职责。
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        schema::initialize(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Agent 工具入口。actor 来自鉴权层，而不是客户端 JSON 参数。
    pub fn execute(&self, actor: &str, tool: &str, args: Value) -> Result<Value> {
        ensure(actor != OPERATOR, "system identity cannot call Agent tools")?;
        self.execute_as(actor, tool, args)
    }

    /// 管理 UI 入口。调用方必须已经通过独立的管理员鉴权。
    pub fn admin_execute(&self, tool: &str, args: Value) -> Result<Value> {
        self.execute_as(OPERATOR, tool, args)
    }

    fn execute_as(&self, actor: &str, tool: &str, args: Value) -> Result<Value> {
        self.transaction(|tx| {
            agent_exists(tx, actor)?;
            let hash = digest(serde_json::to_string(&args)?.as_bytes());
            match tool {
                "agent_list" | "agent_get" => agents::execute(tx, actor, tool, args),
                "file_list" | "file_read" | "file_write" => files::execute(tx, actor, tool, args),
                "task_enqueue" | "task_list" | "task_get" | "task_claim" | "task_heartbeat"
                | "task_wait_on" | "task_complete" | "task_fail" | "task_cancel"
                | "workflow_create" => tasks::execute(tx, actor, tool, args, &hash),
                "inbox_poll" | "inbox_ack" | "message_send" => {
                    messages::execute(tx, actor, tool, args, &hash)
                }
                _ => Err(WorkspaceError::Rule("unknown tool".into())),
            }
        })
    }

    /// 统一事务边界：错误返回时 Transaction 的 Drop 自动回滚。
    fn transaction<T>(&self, operation: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| WorkspaceError::Rule("database lock poisoned".into()))?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let result = operation(&tx)?;
        tx.commit()?;
        Ok(result)
    }

    /// 为前端提供一致的总览快照，租约 token 永远不在总览中出现。
    pub fn snapshot(&self) -> Result<Value> {
        self.transaction(|tx| {
            tasks::expire_leases(tx)?;
            let agents = agents::list(tx)?;
            let tasks = tasks::list(
                tx,
                &TaskList {
                    assignee: None,
                    status: None,
                },
            )?;
            let mut stats = json!({"agents":agents.len()});
            for status in [
                "queued",
                "leased",
                "blocked",
                "completed",
                "failed",
                "cancelled",
            ] {
                let count: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM tasks WHERE status=?1",
                    [status],
                    |r| r.get(0),
                )?;
                stats[status] = json!(count);
            }
            stats["files"] = json!(tx.query_row(
                "SELECT COUNT(DISTINCT path) FROM files",
                [],
                |r| r.get::<_, i64>(0)
            )?);
            stats["unread_messages"] = json!(tx.query_row(
                "SELECT COUNT(*) FROM messages WHERE acked_at IS NULL",
                [],
                |r| r.get::<_, i64>(0)
            )?);
            Ok(json!({"agents":agents,"tasks":tasks,"stats":stats}))
        })
    }
}
