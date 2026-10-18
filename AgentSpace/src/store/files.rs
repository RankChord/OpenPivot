//! 虚拟文件与版本控制。所有写入由调用方的事务统一提交。
use super::*;

pub(super) fn execute(tx: &Transaction<'_>, actor: &str, tool: &str, args: Value) -> Result<Value> {
    Ok(match tool {
        "file_list" => {
            let a: FileList = decode(args)?;
            if !a.prefix.is_empty() {
                let prefix = a.prefix.trim_end_matches('/');
                if prefix != "shared"
                    && prefix != format!("agents/{actor}")
                    && !(actor == OPERATOR
                        && (prefix == "agents"
                            || prefix
                                .strip_prefix("agents/")
                                .is_some_and(|id| agents::validate_id(id).is_ok())))
                {
                    check_path(actor, prefix)?;
                }
            }
            let mut stmt = tx.prepare("SELECT path,MAX(version),sha256,length(content),created_by,created_at FROM files GROUP BY path ORDER BY path")?;
            let rows = stmt.query_map([], |r| Ok(json!({"path":r.get::<_,String>(0)?,"version":r.get::<_,i64>(1)?,"sha256":r.get::<_,String>(2)?,"size":r.get::<_,i64>(3)?,"created_by":r.get::<_,String>(4)?,"created_at":r.get::<_,i64>(5)?})))?;
            let mut files = Vec::new();
            for row in rows {
                let row = row?;
                let path = row["path"].as_str().unwrap();
                if path.starts_with(&a.prefix) && check_path(actor, path).is_ok() {
                    files.push(row);
                }
            }
            json!({"files":files})
        }
        "file_read" => {
            let a: FileRead = decode(args)?;
            check_path(actor, &a.path)?;
            ensure(
                a.version.is_none_or(|v| v > 0 && v <= i64::MAX as u64),
                "invalid file version",
            )?;
            tx.query_row("SELECT path,version,content,sha256,created_by,created_at FROM files WHERE path=?1 AND (?2 IS NULL OR version=?2) ORDER BY version DESC LIMIT 1",params![a.path,a.version.map(|v|v as i64)],|r| {
                    let bytes:Vec<u8>=r.get(2)?;
                    Ok(json!({"path":r.get::<_,String>(0)?,"version":r.get::<_,i64>(1)?,"content":String::from_utf8(bytes.clone()).ok(),"content_base64":STANDARD.encode(&bytes),"size":bytes.len(),"sha256":r.get::<_,String>(3)?,"created_by":r.get::<_,String>(4)?,"created_at":r.get::<_,i64>(5)?}))
                }).optional()?.ok_or_else(||WorkspaceError::Rule("file/version not found".into()))?
        }
        "file_write" => put_file(tx, actor, &decode(args)?)?,
        _ => return Err(WorkspaceError::Rule("unknown tool".into())),
    })
}

/// 校验的是数据库虚拟路径而非磁盘路径；拒绝穿越、绝对路径与跨角色访问。
pub(super) fn check_path(actor: &str, path: &str) -> Result<()> {
    required(path, "path", 512)?;
    ensure(
        !path.contains(['\\', ':', '\0']) && !path.starts_with('/') && !path.ends_with('/'),
        "path must be a relative virtual path using '/' separators",
    )?;
    let parts: Vec<_> = path.split('/').collect();
    ensure(
        parts
            .iter()
            .all(|p| !p.is_empty() && *p != "." && *p != ".." && !p.chars().any(char::is_control)),
        "invalid path component",
    )?;
    ensure(
        (parts[0] == "shared" && parts.len() >= 2)
            || (parts[0] == "agents"
                && parts.len() >= 3
                && agents::validate_id(parts[1]).is_ok()
                && (parts[1] == actor || actor == OPERATOR)),
        "only shared/ and your own agents/<agent_id>/ namespace are accessible",
    )
}
/// 乐观并发控制：客户端必须声明自己读取到的版本，陈旧版本不会覆盖新成果。
pub(super) fn put_file(tx: &Transaction<'_>, actor: &str, a: &FileWrite) -> Result<Value> {
    check_path(actor, &a.path)?;
    // 管理入口仍只能写入已注册角色的目录，避免遗留孤立的知识库路径。
    if let Some(namespace) = a.path.strip_prefix("agents/") {
        let agent = namespace.split('/').next().unwrap();
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE id=?1 AND is_system=0)",
            [agent],
            |r| r.get(0),
        )?;
        ensure(exists, "private namespace belongs to an unknown Agent")?;
    }
    ensure(
        a.expected_version < i64::MAX as u64,
        "invalid expected_version",
    )?;
    ensure(
        a.content.is_some() != a.content_base64.is_some(),
        "provide exactly one of content or content_base64",
    )?;
    let bytes = if let Some(content) = &a.content {
        content.as_bytes().to_vec()
    } else {
        let encoded = a.content_base64.as_deref().unwrap();
        ensure(
            encoded.len() <= MAX_FILE_BYTES * 4 / 3 + 4,
            "file exceeds 1 MiB",
        )?;
        STANDARD
            .decode(encoded)
            .map_err(|_| WorkspaceError::Rule("invalid base64".into()))?
    };
    ensure(bytes.len() <= MAX_FILE_BYTES, "file exceeds 1 MiB")?;
    let current: i64 = tx.query_row(
        "SELECT COALESCE(MAX(version),0) FROM files WHERE path=?1",
        [&a.path],
        |r| r.get(0),
    )?;
    ensure(
        current == a.expected_version as i64,
        &format!(
            "version conflict: expected {}, current {current}",
            a.expected_version
        ),
    )?;
    let version = current + 1;
    let sha = digest(&bytes);
    tx.execute(
        "INSERT INTO files VALUES(?1,?2,?3,?4,?5,?6)",
        params![a.path, version, bytes, sha, actor, now()],
    )?;
    Ok(json!({"path":a.path,"version":version,"sha256":sha,"size":bytes.len()}))
}
