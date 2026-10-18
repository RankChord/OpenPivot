//! 数据库建表与原始 demo 数据库的增量迁移。
use super::*;

pub(super) fn initialize(conn: &mut Connection) -> Result<()> {
    // SQLite 修改 CHECK 需要重建表。先在事务外关闭外键执行，事务结束后恢复；
    // 迁移内 foreign_key_check 仍检查所有引用，失败时旧数据库完整回滚。
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=OFF;")?;
    let result = initialize_transaction(conn);
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    result
}

fn initialize_transaction(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS agents(id TEXT PRIMARY KEY, name TEXT NOT NULL, responsibility TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS files(path TEXT NOT NULL, version INTEGER NOT NULL, content BLOB NOT NULL,
            sha256 TEXT NOT NULL, created_by TEXT NOT NULL REFERENCES agents(id), created_at INTEGER NOT NULL,
            PRIMARY KEY(path, version));
        CREATE TABLE IF NOT EXISTS tasks(id TEXT PRIMARY KEY, sender TEXT NOT NULL REFERENCES agents(id),
            assignee TEXT NOT NULL REFERENCES agents(id), title TEXT NOT NULL, instructions TEXT NOT NULL,
            reply_to TEXT NOT NULL REFERENCES agents(id), parent_task_id TEXT REFERENCES tasks(id),
            status TEXT NOT NULL CHECK(status IN ('queued','leased','blocked','completed','failed','cancelled')),
            attempt INTEGER NOT NULL DEFAULT 0, max_attempts INTEGER NOT NULL DEFAULT 3,
            lease_token TEXT, lease_until INTEGER, depends_on TEXT REFERENCES tasks(id),
            summary TEXT, result_json TEXT, completed_hash TEXT, last_error TEXT,
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
            collaboration_mode TEXT NOT NULL DEFAULT 'fixed' CHECK(collaboration_mode IN ('automatic','fixed')),
            workflow_id TEXT NOT NULL REFERENCES tasks(id),workflow_title TEXT NOT NULL DEFAULT '');
        CREATE INDEX IF NOT EXISTS task_queue ON tasks(assignee, status, created_at);
        CREATE TABLE IF NOT EXISTS messages(id TEXT PRIMARY KEY, sender TEXT NOT NULL REFERENCES agents(id),
            recipient TEXT NOT NULL REFERENCES agents(id), kind TEXT NOT NULL, task_id TEXT REFERENCES tasks(id),
            body TEXT NOT NULL, payload TEXT NOT NULL, created_at INTEGER NOT NULL, acked_at INTEGER);
        CREATE INDEX IF NOT EXISTS inbox ON messages(recipient, acked_at, created_at);
        CREATE TABLE IF NOT EXISTS dedup(actor TEXT NOT NULL, scope TEXT NOT NULL, key TEXT NOT NULL,
            request_hash TEXT NOT NULL, result_json TEXT NOT NULL, PRIMARY KEY(actor,scope,key));")?;
    let columns = {
        let mut stmt = tx.prepare("PRAGMA table_info(agents)")?;
        stmt.query_map([], |r| r.get::<_, String>(1))?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    // 旧数据原样保留，只添加字段。DDL 名称完全来自常量，不能由请求指定。
    for (column, definition) in [
        ("model", "TEXT NOT NULL DEFAULT ''"),
        ("active", "INTEGER NOT NULL DEFAULT 1"),
        ("is_system", "INTEGER NOT NULL DEFAULT 0"),
        ("token_hash", "TEXT"),
        ("created_at", "INTEGER NOT NULL DEFAULT 0"),
        ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        if !columns.iter().any(|existing| existing == column) {
            tx.execute_batch(&format!(
                "ALTER TABLE agents ADD COLUMN {column} {definition};"
            ))?;
        }
    }
    tx.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS agent_token_hash ON agents(token_hash) WHERE token_hash IS NOT NULL;")?;
    tx.execute(
        "UPDATE agents SET created_at=?1,updated_at=?1 WHERE created_at=0",
        [now()],
    )?;
    tx.execute("INSERT OR IGNORE INTO agents(id,name,responsibility,is_system,active,created_at,updated_at) VALUES(?1,'工作区管理员','管理界面派单与文件维护',1,1,?2,?2)", params![OPERATOR, now()])?;
    migrate_tasks(&tx)?;
    let violations: i64 =
        tx.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })?;
    ensure(
        violations == 0,
        "database migration would leave invalid foreign-key references",
    )?;
    tx.commit()?;
    Ok(())
}

/// 原有任务都标记 fixed，避免把过去提前排好的视频示例误报成自主决策。
/// 新的 automatic 根任务及其后续委派均由业务层显式写入模式和 workflow_id。
fn migrate_tasks(tx: &Transaction<'_>) -> Result<()> {
    let columns = {
        let mut stmt = tx.prepare("PRAGMA table_info(tasks)")?;
        stmt.query_map([], |r| r.get::<_, String>(1))?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    let schema: String = tx.query_row(
        "SELECT sql FROM sqlite_schema WHERE type='table' AND name='tasks'",
        [],
        |r| r.get(0),
    )?;
    let has_mode = columns.iter().any(|c| c == "collaboration_mode");
    let has_workflow = columns.iter().any(|c| c == "workflow_id");
    let has_title = columns.iter().any(|c| c == "workflow_title");
    if has_mode && has_workflow && has_title && schema.contains("'cancelled'") {
        return Ok(());
    }
    // 保存用户增添的显式索引与触发器。SQLite 自动主键索引由新表重新生成。
    let definitions = {
        let mut stmt = tx.prepare("SELECT sql FROM sqlite_schema WHERE tbl_name='tasks' AND type IN ('index','trigger') AND sql IS NOT NULL ORDER BY type,name")?;
        stmt.query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    tx.execute_batch("CREATE TABLE tasks_migration_new(id TEXT PRIMARY KEY, sender TEXT NOT NULL REFERENCES agents(id),
        assignee TEXT NOT NULL REFERENCES agents(id), title TEXT NOT NULL, instructions TEXT NOT NULL,
        reply_to TEXT NOT NULL REFERENCES agents(id), parent_task_id TEXT REFERENCES tasks(id),
        status TEXT NOT NULL CHECK(status IN ('queued','leased','blocked','completed','failed','cancelled')),
        attempt INTEGER NOT NULL DEFAULT 0,max_attempts INTEGER NOT NULL DEFAULT 3,
        lease_token TEXT,lease_until INTEGER,depends_on TEXT REFERENCES tasks(id),
        summary TEXT,result_json TEXT,completed_hash TEXT,last_error TEXT,
        created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,
        collaboration_mode TEXT NOT NULL DEFAULT 'fixed' CHECK(collaboration_mode IN ('automatic','fixed')),
        workflow_id TEXT NOT NULL REFERENCES tasks(id),workflow_title TEXT NOT NULL DEFAULT '');")?;
    let mode = if has_mode {
        "collaboration_mode"
    } else {
        "'fixed'"
    };
    let workflow = if has_workflow {
        "COALESCE(workflow_id,id)"
    } else {
        "id"
    };
    let title = if has_title { "workflow_title" } else { "title" };
    tx.execute_batch(&format!("INSERT INTO tasks_migration_new SELECT id,sender,assignee,title,instructions,reply_to,parent_task_id,status,attempt,max_attempts,lease_token,lease_until,depends_on,summary,result_json,completed_hash,last_error,created_at,updated_at,{mode},{workflow},{title} FROM tasks; DROP TABLE tasks; ALTER TABLE tasks_migration_new RENAME TO tasks;"))?;
    if !has_workflow {
        // 遍历父关系还原旧流程归属；防御旧数据中的循环，有限步结束。
        let parents: std::collections::HashMap<String, Option<String>> = {
            let mut stmt = tx.prepare("SELECT id,parent_task_id FROM tasks")?;
            stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })?
            .collect::<std::result::Result<_, _>>()?
        };
        for task_id in parents.keys() {
            let mut root = task_id.clone();
            let mut seen = std::collections::HashSet::new();
            while seen.insert(root.clone()) {
                match parents.get(&root).and_then(|parent| parent.as_ref()) {
                    Some(parent) if parents.contains_key(parent) => root = parent.clone(),
                    _ => break,
                }
            }
            tx.execute(
                "UPDATE tasks SET workflow_id=?2 WHERE id=?1",
                params![task_id, root],
            )?;
        }
    }
    if !has_title {
        tx.execute_batch("UPDATE tasks SET workflow_title=COALESCE((SELECT root.title FROM tasks root WHERE root.id=tasks.workflow_id),title);")?;
    }
    for definition in definitions {
        tx.execute_batch(&definition)?;
    }
    Ok(())
}
