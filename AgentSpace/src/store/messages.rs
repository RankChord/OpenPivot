//! 收件箱、确认与幂等通知。
use super::tasks::task;
use super::*;

pub(super) fn execute(
    tx: &Transaction<'_>,
    actor: &str,
    tool: &str,
    args: Value,
    hash: &str,
) -> Result<Value> {
    Ok(match tool {
        "inbox_poll" => {
            let a: Poll = decode(args)?;
            let limit = a.limit.unwrap_or(50);
            ensure((1..=100).contains(&limit), "limit must be 1..100")?;
            let mut stmt=tx.prepare("SELECT id,sender,recipient,kind,task_id,body,payload,created_at FROM messages WHERE recipient=?1 AND acked_at IS NULL ORDER BY created_at,rowid LIMIT ?2")?;
            let messages=stmt.query_map(params![actor,limit],|r| Ok(json!({"id":r.get::<_,String>(0)?,"sender":r.get::<_,String>(1)?,"recipient":r.get::<_,String>(2)?,"kind":r.get::<_,String>(3)?,"task_id":r.get::<_,Option<String>>(4)?,"body":r.get::<_,String>(5)?,"payload":serde_json::from_str::<Value>(&r.get::<_,String>(6)?).unwrap_or(Value::Null),"created_at":r.get::<_,i64>(7)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
            json!({"messages":messages})
        }
        "inbox_ack" => {
            let a: Ack = decode(args)?;
            ensure(a.message_ids.len() <= 100, "at most 100 message IDs")?;
            for message_id in &a.message_ids {
                let owner: Option<String> = tx
                    .query_row(
                        "SELECT recipient FROM messages WHERE id=?1",
                        [message_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                ensure(
                    owner.as_deref() == Some(actor),
                    "message does not belong to caller",
                )?;
                tx.execute(
                    "UPDATE messages SET acked_at=COALESCE(acked_at,?2) WHERE id=?1",
                    params![message_id, now()],
                )?;
            }
            json!({"acked":a.message_ids.len()})
        }
        "message_send" => {
            let a: SendMessage = decode(args)?;
            required(&a.idempotency_key, "idempotency_key", 200)?;
            if let Some(result) = cached(tx, actor, tool, &a.idempotency_key, hash)? {
                result
            } else {
                agent_exists(tx, &a.to)?;
                required(&a.body, "body", 64000)?;
                if let Some(task_id) = &a.task_id {
                    task(tx, task_id, false)?;
                }
                let message_id = message(
                    tx,
                    actor,
                    &a.to,
                    "message",
                    a.task_id.as_deref(),
                    &a.body,
                    &json!({}),
                )?;
                let result = json!({"message_id":message_id});
                cache(tx, actor, tool, &a.idempotency_key, hash, &result)?;
                result
            }
        }
        _ => return Err(WorkspaceError::Rule("unknown tool".into())),
    })
}

/// 写入持久化回执，调用方事务保证它与对应任务/文件修改一起生效。
pub(super) fn message(
    tx: &Transaction<'_>,
    sender: &str,
    recipient: &str,
    kind: &str,
    task_id: Option<&str>,
    body: &str,
    payload: &Value,
) -> Result<String> {
    let message_id = id();
    tx.execute("INSERT INTO messages(id,sender,recipient,kind,task_id,body,payload,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![message_id,sender,recipient,kind,task_id,body,payload.to_string(),now()])?;
    Ok(message_id)
}
