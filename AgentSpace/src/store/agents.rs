//! 动态 Agent 注册表。逻辑身份长期稳定，模型与 token 可以独立替换。
use super::*;

const PROFILE_COLUMNS: &str = "id,name,responsibility,model,active";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Register {
    id: String,
    name: String,
    responsibility: String,
    #[serde(default)]
    model: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    name: Option<String>,
    responsibility: Option<String>,
    model: Option<String>,
    active: Option<bool>,
}

impl Store {
    /// 启动时避免管理员把既有 Agent 凭证误配置为管理凭证；也检查停用身份。
    /// 这样恢复旧身份时不会意外授予管理权限。
    pub fn validate_admin_credential(&self, token: &str) -> Result<()> {
        self.transaction(|tx| {
            let collision: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM agents WHERE token_hash=?1 AND is_system=0)",
                [digest(token.as_bytes())],
                |row| row.get(0),
            )?;
            ensure(
                !collision,
                "administrator credential must differ from every existing Agent credential",
            )
        })
    }

    /// 注册任意数量的 Agent，同时初始化独立记忆与知识目录。
    /// 明文 token 只由此返回，数据库仅保存 SHA-256 摘要。
    pub fn register_agent(&self, args: Value) -> Result<Value> {
        let a: Register = decode(args)?;
        validate_id(&a.id)?;
        required(&a.name, "name", 200)?;
        required(&a.responsibility, "responsibility", 16000)?;
        ensure(a.model.len() <= 200, "model must contain at most 200 bytes")?;
        self.transaction(|tx| {
            let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM agents WHERE id=?1)", [&a.id], |r| r.get(0))?;
            ensure(!exists, "agent ID already exists")?;
            let token = new_token();
            tx.execute("INSERT INTO agents(id,name,responsibility,model,active,is_system,token_hash,created_at,updated_at) VALUES(?1,?2,?3,?4,1,0,?5,?6,?6)", params![a.id,a.name,a.responsibility,a.model,digest(token.as_bytes()),now()])?;
            for (suffix, content) in [
                ("identity.md", identity_document(&a.id, &a.name, &a.responsibility, &a.model)),
                ("memory/context.md", "# 工作记忆\n\n保存事实、决策、待办和产物版本引用；请勿保存密钥。\n".into()),
                ("knowledge/README.md", "# 知识库\n\n保存可复用的项目知识与操作指南。\n".into()),
            ] {
                put_file(tx, &a.id, &FileWrite { path: format!("agents/{}/{suffix}",a.id), content: Some(content), content_base64: None, expected_version: 0 })?;
            }
            Ok(json!({"agent":get(tx,&a.id)?,"token":token}))
        })
    }

    /// 停用是可逆的；不删除角色、任务历史、记忆或成果引用。
    pub fn update_agent(&self, agent_id: &str, args: Value) -> Result<Value> {
        let a: Update = decode(args)?;
        if let Some(name) = &a.name {
            required(name, "name", 200)?;
        }
        if let Some(role) = &a.responsibility {
            required(role, "responsibility", 16000)?;
        }
        if let Some(model) = &a.model {
            ensure(model.len() <= 200, "model must contain at most 200 bytes")?;
        }
        self.transaction(|tx| {
            get(tx, agent_id)?;
            if a.active == Some(false) {
                let unfinished: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE assignee=?1 AND status NOT IN ('completed','failed','cancelled'))", [agent_id], |r| r.get(0))?;
                ensure(!unfinished, "cannot disable Agent with unfinished assigned tasks")?;
            }
            tx.execute("UPDATE agents SET name=COALESCE(?2,name),responsibility=COALESCE(?3,responsibility),model=COALESCE(?4,model),active=COALESCE(?5,active),updated_at=?6 WHERE id=?1 AND is_system=0", params![agent_id,a.name,a.responsibility,a.model,a.active,now()])?;
            let profile = get(tx, agent_id)?;
            // 身份文档同步更新成一个新版本；知识库和工作记忆沿用原位置。
            let path = format!("agents/{agent_id}/identity.md");
            let version: i64 = tx.query_row("SELECT COALESCE(MAX(version),0) FROM files WHERE path=?1", [&path], |r| r.get(0))?;
            put_file(tx, OPERATOR, &FileWrite {
                path,
                content: Some(identity_document(agent_id,profile["name"].as_str().unwrap(),profile["responsibility"].as_str().unwrap(),profile["model"].as_str().unwrap())),
                content_base64: None,
                expected_version: version as u64,
            })?;
            Ok(profile)
        })
    }

    /// 轮换 token 后旧凭证立即失效，逻辑身份与历史数据仍使用原 ID。
    pub fn rotate_agent_token(&self, agent_id: &str) -> Result<Value> {
        self.transaction(|tx| {
            let profile = get(tx, agent_id)?;
            let token = new_token();
            tx.execute(
                "UPDATE agents SET token_hash=?2,updated_at=?3 WHERE id=?1 AND is_system=0",
                params![agent_id, digest(token.as_bytes()), now()],
            )?;
            Ok(json!({"agent":profile,"token":token}))
        })
    }

    /// 将 bearer token 映射为服务器可信身份；系统身份与停用角色永远不匹配。
    pub fn authenticate(&self, token: &str) -> Result<Option<String>> {
        if token.is_empty() || !token.bytes().all(|b| b.is_ascii_graphic()) {
            return Ok(None);
        }
        self.transaction(|tx| {
            Ok(tx
                .query_row(
                    "SELECT id FROM agents WHERE token_hash=?1 AND active=1 AND is_system=0",
                    [digest(token.as_bytes())],
                    |r| r.get(0),
                )
                .optional()?)
        })
    }

    /// 兼容既有部署中的显式 token 配置；只允许为已注册角色导入唯一可见 ASCII token。
    pub fn import_agent_token(&self, agent_id: &str, token: &str) -> Result<()> {
        ensure(
            !token.is_empty() && token.bytes().all(|b| b.is_ascii_graphic()),
            "token must be non-empty visible ASCII",
        )?;
        self.transaction(|tx| {
            get(tx, agent_id)?;
            let hash = digest(token.as_bytes());
            let duplicate: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM agents WHERE token_hash=?1 AND id<>?2)",
                params![hash, agent_id],
                |r| r.get(0),
            )?;
            ensure(!duplicate, "token is already assigned to another Agent")?;
            tx.execute(
                "UPDATE agents SET token_hash=?2,updated_at=?3 WHERE id=?1 AND is_system=0",
                params![agent_id, hash, now()],
            )?;
            Ok(())
        })
    }
}

pub(super) fn execute(
    tx: &Transaction<'_>,
    _actor: &str,
    tool: &str,
    args: Value,
) -> Result<Value> {
    match tool {
        "agent_list" => {
            let _: Empty = decode(args)?;
            Ok(json!({"agents":list(tx)?}))
        }
        "agent_get" => {
            let a: AgentGet = decode(args)?;
            get(tx, &a.agent_id)
        }
        _ => Err(WorkspaceError::Rule("unknown tool".into())),
    }
}

pub(super) fn list(tx: &Transaction<'_>) -> Result<Vec<Value>> {
    let mut stmt = tx.prepare(&format!(
        "SELECT {PROFILE_COLUMNS} FROM agents WHERE is_system=0 ORDER BY id"
    ))?;
    Ok(stmt
        .query_map([], agent_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}
fn get(tx: &Transaction<'_>, agent: &str) -> Result<Value> {
    tx.query_row(
        &format!("SELECT {PROFILE_COLUMNS} FROM agents WHERE id=?1 AND is_system=0"),
        [agent],
        agent_row,
    )
    .optional()?
    .ok_or_else(|| WorkspaceError::Rule("unknown agent".into()))
}
fn agent_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let agent_id: String = r.get(0)?;
    Ok(
        json!({"id":agent_id,"name":r.get::<_,String>(1)?,"responsibility":r.get::<_,String>(2)?,"model":r.get::<_,String>(3)?,"active":r.get::<_,bool>(4)?,"identity_path":format!("agents/{agent_id}/identity.md"),"knowledge_prefix":format!("agents/{agent_id}/knowledge/"),"memory_prefix":format!("agents/{agent_id}/memory/")}),
    )
}
pub(super) fn agent_exists(tx: &Transaction<'_>, agent: &str) -> Result<()> {
    let active: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM agents WHERE id=?1 AND active=1)",
        [agent],
        |r| r.get(0),
    )?;
    ensure(active, "unknown or disabled agent")
}
pub(super) fn agent_available(tx: &Transaction<'_>, agent: &str) -> Result<()> {
    let available: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM agents WHERE id=?1 AND active=1 AND is_system=0)",
        [agent],
        |r| r.get(0),
    )?;
    ensure(available, "unknown, disabled, or system assignee")
}
pub(super) fn validate_id(agent: &str) -> Result<()> {
    ensure(
        !agent.is_empty()
            && agent.len() <= 64
            && agent != OPERATOR
            && agent
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-'),
        "agent ID must use 1..64 ASCII letters, digits, '_' or '-', and cannot be __operator",
    )
}
fn new_token() -> String {
    format!("as_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}
fn identity_document(agent: &str, name: &str, role: &str, model: &str) -> String {
    format!(
        "# {agent}: {name}\n\n职责：{role}\n\n模型备注：{model}\n\nagent_id 是稳定的逻辑身份。模型和执行器可以替换。知识库与记忆存储位置保持稳定。\n"
    )
}
