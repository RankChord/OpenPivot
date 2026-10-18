//! 持久化任务队列、租约与依赖状态机。
//!
//! queued → leased → completed/failed；可重试失败和过期租约回到 queued。
//! leased → blocked 表示等待依赖；依赖完成重新排队，依赖终止失败或取消递归传播。
//! automatic 只创建当前已确定的工作；fixed 一次提交预先安排的所有步骤。
//! 租约 token 是 fencing token：旧执行器即使稍后返回，也不能覆盖新执行结果。
use super::*;

pub(super) fn execute(
    tx: &Transaction<'_>,
    actor: &str,
    tool: &str,
    args: Value,
    hash: &str,
) -> Result<Value> {
    Ok(match tool {
        "task_enqueue" => {
            let a: Enqueue = decode(args)?;
            required(&a.idempotency_key, "idempotency_key", 200)?;
            if let Some(result) = cached(tx, actor, tool, &a.idempotency_key, hash)? {
                // 老版本幂等缓存没有协作字段；从任务记录补齐，不重复派单。
                if result.get("collaboration_mode").is_some() {
                    result
                } else {
                    let task_id = result["id"]
                        .as_str()
                        .ok_or_else(|| WorkspaceError::Rule("invalid cached task result".into()))?;
                    task(tx, task_id, false)?
                }
            } else {
                agent_available(tx, &a.assignee)?;
                let reply_to = a.reply_to.as_deref().unwrap_or(actor);
                agent_exists(tx, reply_to)?;
                required(&a.title, "title", 500)?;
                required(&a.instructions, "instructions", 64000)?;
                let task_id = id();
                let (mode, workflow_id, workflow_title) = if let Some(parent_id) = &a.parent_task_id
                {
                    let parent = task(tx, parent_id, false)?;
                    let mode = parent["collaboration_mode"].as_str().unwrap();
                    ensure(
                        mode == "automatic",
                        "fixed workflows cannot be extended with task_enqueue; use workflow_create",
                    )?;
                    ensure(
                        parent["status"] != "failed" && parent["status"] != "cancelled",
                        "cannot delegate from a failed or cancelled parent",
                    )?;
                    ensure(
                        actor == OPERATOR || parent["assignee"] == actor,
                        "only the parent assignee can delegate an automatic task",
                    )?;
                    if let Some(requested) = &a.collaboration_mode {
                        ensure(
                            requested == mode,
                            "child collaboration_mode must match its parent",
                        )?;
                    }
                    (
                        mode.to_string(),
                        parent["workflow_id"].as_str().unwrap().to_string(),
                        parent["workflow_title"].as_str().unwrap().to_string(),
                    )
                } else {
                    let mode = a.collaboration_mode.as_deref().unwrap_or("automatic");
                    ensure(
                        mode == "automatic",
                        "fixed tasks must be created with workflow_create",
                    )?;
                    (mode.to_string(), task_id.clone(), a.title.clone())
                };
                let status = if let Some(dependency_id) = &a.depends_on {
                    let dependency = task(tx, dependency_id, false)?;
                    ensure(
                        dependency["status"] != "failed" && dependency["status"] != "cancelled",
                        "dependency has already failed or been cancelled",
                    )?;
                    if dependency["status"] == "completed" {
                        "queued"
                    } else {
                        "blocked"
                    }
                } else {
                    "queued"
                };
                let time = now();
                tx.execute("INSERT INTO tasks(id,sender,assignee,title,instructions,reply_to,parent_task_id,status,created_at,updated_at,collaboration_mode,workflow_id,depends_on,workflow_title) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,?10,?11,?12,?13)",params![task_id,actor,a.assignee,a.title,a.instructions,reply_to,a.parent_task_id,status,time,mode,workflow_id,a.depends_on,workflow_title])?;
                let result = task(tx, &task_id, false)?;
                cache(tx, actor, tool, &a.idempotency_key, hash, &result)?;
                result
            }
        }
        "workflow_create" => {
            let a: WorkflowCreate = decode(args)?;
            required(&a.idempotency_key, "idempotency_key", 200)?;
            if let Some(result) = cached(tx, actor, tool, &a.idempotency_key, hash)? {
                result
            } else {
                required(&a.title, "title", 500)?;
                ensure(
                    (1..=32).contains(&a.steps.len()),
                    "steps must contain 1..32 items",
                )?;
                let reply_to = a.reply_to.as_deref().unwrap_or(actor);
                agent_exists(tx, reply_to)?;
                // 先验证全部节点，再一起写入；事务同时保护检查与插入。
                for step in &a.steps {
                    agent_available(tx, &step.assignee)?;
                    required(&step.title, "title", 500)?;
                    required(&step.instructions, "instructions", 64000)?;
                }
                let workflow_id = id();
                let mut previous: Option<String> = None;
                let mut created = Vec::with_capacity(a.steps.len());
                let time = now();
                for step in a.steps {
                    let task_id = if previous.is_none() {
                        workflow_id.clone()
                    } else {
                        id()
                    };
                    let status = if previous.is_none() {
                        "queued"
                    } else {
                        "blocked"
                    };
                    tx.execute("INSERT INTO tasks(id,sender,assignee,title,instructions,reply_to,status,depends_on,collaboration_mode,workflow_id,created_at,updated_at,workflow_title) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'fixed',?9,?10,?10,?11)", params![task_id,actor,step.assignee,step.title,step.instructions,reply_to,status,previous,workflow_id,time,a.title])?;
                    created.push(task(tx, &task_id, false)?);
                    previous = Some(task_id);
                }
                let result = json!({"workflow_id":workflow_id,"collaboration_mode":"fixed","title":a.title,"tasks":created});
                cache(tx, actor, tool, &a.idempotency_key, hash, &result)?;
                result
            }
        }
        "task_cancel" => {
            let a: Cancel = decode(args)?;
            required(&a.reason, "reason", 64000)?;
            let current = task(tx, &a.task_id, false)?;
            ensure(
                actor == OPERATOR || current["sender"] == actor,
                "only the task creator can cancel it",
            )?;
            ensure(
                ["queued", "blocked", "cancelled"].contains(&current["status"].as_str().unwrap()),
                "only queued or blocked tasks can be cancelled",
            )?;
            terminal_cancel(tx, &a.task_id, &a.reason)?;
            task(tx, &a.task_id, false)?
        }
        "task_list" => {
            let a: TaskList = decode(args)?;
            json!({"tasks":list(tx, &a)?})
        }
        "task_get" => {
            let a: TaskGet = decode(args)?;
            task(tx, &a.task_id, false)?
        }
        "task_claim" => {
            // 调用方事务使用 IMMEDIATE，跨数据库连接也不能重复领取同一任务。
            let a: Claim = decode(args)?;
            let duration = lease_duration(a.lease_seconds)?;
            expire_leases(tx)?;
            let task_id:Option<String>=tx.query_row("SELECT id FROM tasks WHERE assignee=?1 AND status='queued' ORDER BY created_at,rowid LIMIT 1",[actor],|r|r.get(0)).optional()?;
            if let Some(task_id) = task_id {
                tx.execute("UPDATE tasks SET status='leased',attempt=attempt+1,lease_token=?2,lease_until=?3,updated_at=?4 WHERE id=?1",params![task_id,id(),now()+duration,now()])?;
                task(tx, &task_id, true)?
            } else {
                json!({"task":null})
            }
        }
        "task_heartbeat" => {
            let a: Heartbeat = decode(args)?;
            let duration = lease_duration(a.lease_seconds)?;
            valid_lease(tx, actor, &a.task_id, &a.lease_token)?;
            tx.execute(
                "UPDATE tasks SET lease_until=?2,updated_at=?3 WHERE id=?1",
                params![a.task_id, now() + duration, now()],
            )?;
            task(tx, &a.task_id, true)?
        }
        "task_wait_on" => {
            // 依赖只能在有效租约期间登记；追踪依赖链，拒绝循环等待。
            let a: WaitOn = decode(args)?;
            valid_lease(tx, actor, &a.task_id, &a.lease_token)?;
            let current = task(tx, &a.task_id, false)?;
            if current["collaboration_mode"] == "fixed" {
                ensure(
                    current["depends_on"] == a.dependency_task_id,
                    "fixed stages cannot change their configured dependency",
                )?;
            }
            let dependency = task(tx, &a.dependency_task_id, false)?;
            let mut cursor = Some(a.dependency_task_id.clone());
            let mut visited = std::collections::HashSet::new();
            while let Some(current) = cursor {
                ensure(
                    current != a.task_id && visited.insert(current.clone()),
                    "dependency cycle rejected",
                )?;
                cursor =
                    tx.query_row("SELECT depends_on FROM tasks WHERE id=?1", [current], |r| {
                        r.get(0)
                    })?;
            }
            tx.execute("UPDATE tasks SET status=?2,depends_on=?3,lease_token=NULL,lease_until=NULL,updated_at=?4 WHERE id=?1",params![a.task_id,if dependency["status"]=="completed" {"queued"} else {"blocked"},a.dependency_task_id,now()])?;
            // 等待前置输入不算执行失败，因此不消耗重试次数。
            tx.execute(
                "UPDATE tasks SET attempt=attempt-1 WHERE id=?1",
                [&a.task_id],
            )?;
            if dependency["status"] == "failed" {
                terminal_fail(tx, &a.task_id, "dependency failed")?;
            } else if dependency["status"] == "cancelled" {
                terminal_cancel(tx, &a.task_id, "dependency cancelled")?;
            }
            task(tx, &a.task_id, false)?
        }
        "task_complete" => {
            // 文件、完成状态、通知与依赖唤醒处于同一事务。
            // 任一输出版本冲突都会全部回滚，客户端可安全重试相同完成请求。
            let a: Complete = decode(args)?;
            required(&a.summary, "summary", 64000)?;
            ensure(a.outputs.len() <= 16, "at most 16 outputs per completion")?;
            let current = task(tx, &a.task_id, true)?;
            if current["status"] == "completed" {
                let old_hash: Option<String> = tx.query_row(
                    "SELECT completed_hash FROM tasks WHERE id=?1",
                    [&a.task_id],
                    |r| r.get(0),
                )?;
                ensure(
                    current["assignee"] == actor
                        && current["lease_token"] == a.lease_token
                        && old_hash.as_deref() == Some(hash),
                    "completion replay differs from accepted completion",
                )?;
                json!({"task":task(tx,&a.task_id,false)?,"artifacts":current["result"]["artifacts"]})
            } else {
                valid_lease(tx, actor, &a.task_id, &a.lease_token)?;
                let mut paths = std::collections::HashSet::new();
                let mut artifacts = Vec::new();
                for output in &a.outputs {
                    ensure(paths.insert(&output.path), "duplicate output path")?;
                    artifacts.push(put_file(tx, actor, output)?);
                }
                let result = json!({"summary":a.summary,"artifacts":artifacts});
                tx.execute("UPDATE tasks SET status='completed',summary=?2,result_json=?3,completed_hash=?4,lease_until=NULL,updated_at=?5 WHERE id=?1",params![a.task_id,a.summary,result.to_string(),hash,now()])?;
                message(
                    tx,
                    actor,
                    current["reply_to"].as_str().unwrap(),
                    "task_completed",
                    Some(&a.task_id),
                    &a.summary,
                    &result,
                )?;
                tx.execute("UPDATE tasks SET status='queued',updated_at=?2 WHERE status='blocked' AND depends_on=?1",params![a.task_id,now()])?;
                json!({"task":task(tx,&a.task_id,false)?,"artifacts":artifacts})
            }
        }
        "task_fail" => {
            let a: Fail = decode(args)?;
            required(&a.reason, "reason", 64000)?;
            valid_lease(tx, actor, &a.task_id, &a.lease_token)?;
            let current = task(tx, &a.task_id, false)?;
            if a.retryable
                && current["attempt"].as_i64().unwrap() < current["max_attempts"].as_i64().unwrap()
            {
                tx.execute("UPDATE tasks SET status='queued',lease_token=NULL,lease_until=NULL,last_error=?2,updated_at=?3 WHERE id=?1",params![a.task_id,a.reason,now()])?;
            } else {
                terminal_fail(tx, &a.task_id, &a.reason)?;
            }
            task(tx, &a.task_id, false)?
        }
        _ => return Err(WorkspaceError::Rule("unknown tool".into())),
    })
}

pub(super) fn task(tx: &Transaction<'_>, task_id: &str, secret: bool) -> Result<Value> {
    let mut value=tx.query_row("SELECT id,sender,assignee,title,instructions,reply_to,parent_task_id,status,attempt,max_attempts,lease_token,lease_until,depends_on,summary,result_json,last_error,created_at,updated_at,collaboration_mode,workflow_id,workflow_title FROM tasks WHERE id=?1",[task_id],|r| {
        Ok(json!({"id":r.get::<_,String>(0)?,"sender":r.get::<_,String>(1)?,"assignee":r.get::<_,String>(2)?,"title":r.get::<_,String>(3)?,"instructions":r.get::<_,String>(4)?,"reply_to":r.get::<_,String>(5)?,"parent_task_id":r.get::<_,Option<String>>(6)?,"status":r.get::<_,String>(7)?,"attempt":r.get::<_,i64>(8)?,"max_attempts":r.get::<_,i64>(9)?,"lease_until":r.get::<_,Option<i64>>(11)?,"depends_on":r.get::<_,Option<String>>(12)?,"summary":r.get::<_,Option<String>>(13)?,"result":r.get::<_,Option<String>>(14)?.and_then(|s|serde_json::from_str::<Value>(&s).ok()),"last_error":r.get::<_,Option<String>>(15)?,"created_at":r.get::<_,i64>(16)?,"updated_at":r.get::<_,i64>(17)?,"collaboration_mode":r.get::<_,String>(18)?,"workflow_id":r.get::<_,String>(19)?,"workflow_title":r.get::<_,String>(20)?,"lease_token":if secret {r.get::<_,Option<String>>(10)?} else {None}}))
    }).optional()?.ok_or_else(||WorkspaceError::Rule("task not found".into()))?;
    if !secret {
        value.as_object_mut().unwrap().remove("lease_token");
    }
    Ok(value)
}
/// 查询全工作区任务；assignee/status 仅用于筛选，不改变执行权限。
pub(super) fn list(tx: &Transaction<'_>, args: &TaskList) -> Result<Vec<Value>> {
    if let Some(status) = &args.status {
        ensure(
            [
                "queued",
                "leased",
                "blocked",
                "completed",
                "failed",
                "cancelled",
            ]
            .contains(&status.as_str()),
            "invalid task status",
        )?;
    }
    let mut stmt = tx.prepare("SELECT id FROM tasks WHERE (?1 IS NULL OR assignee=?1) AND (?2 IS NULL OR status=?2) ORDER BY created_at DESC,rowid DESC")?;
    let ids = stmt
        .query_map(params![args.assignee, args.status], |r| {
            r.get::<_, String>(0)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ids.into_iter().map(|id| task(tx, &id, false)).collect()
}
pub(super) fn lease_duration(seconds: Option<u64>) -> Result<i64> {
    let seconds = seconds.unwrap_or(120);
    ensure(
        (5..=3600).contains(&seconds),
        "lease_seconds must be 5..3600",
    )?;
    Ok(seconds as i64)
}
/// 只有拥有当前未过期租约的 assignee 能续租、等待、提交或失败。
pub(super) fn valid_lease(
    tx: &Transaction<'_>,
    actor: &str,
    task_id: &str,
    token: &str,
) -> Result<()> {
    let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1 AND assignee=?2 AND status='leased' AND lease_token=?3 AND lease_until>?4)",params![task_id,actor,token,now()],|r|r.get(0))?;
    ensure(valid, "invalid, expired, or foreign task lease")
}
pub(super) fn terminal_fail(tx: &Transaction<'_>, task_id: &str, reason: &str) -> Result<()> {
    // 使用迭代队列传播失败，避免深依赖链递归耗尽调用栈。
    let mut pending = vec![(task_id.to_string(), reason.to_string())];
    while let Some((task_id, reason)) = pending.pop() {
        let current = task(tx, &task_id, false)?;
        if current["status"] == "failed"
            || current["status"] == "completed"
            || current["status"] == "cancelled"
        {
            continue;
        }
        tx.execute("UPDATE tasks SET status='failed',last_error=?2,lease_token=NULL,lease_until=NULL,updated_at=?3 WHERE id=?1",params![task_id,reason,now()])?;
        message(
            tx,
            current["assignee"].as_str().unwrap(),
            current["reply_to"].as_str().unwrap(),
            "task_failed",
            Some(&task_id),
            &reason,
            &json!({"reason":reason}),
        )?;
        let mut stmt =
            tx.prepare("SELECT id FROM tasks WHERE status='blocked' AND depends_on=?1")?;
        let children = stmt
            .query_map([&task_id], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        pending.extend(
            children
                .into_iter()
                .map(|child| (child, format!("dependency {task_id} failed"))),
        );
    }
    Ok(())
}
/// 取消只向等待当前结果的依赖节点传播，保留已完成交付与独立分支。
/// 调用入口先验证创建者权限；这里也服务于“等待已取消依赖”的状态转换。
pub(super) fn terminal_cancel(tx: &Transaction<'_>, task_id: &str, reason: &str) -> Result<()> {
    let mut pending = vec![(task_id.to_string(), reason.to_string())];
    while let Some((task_id, reason)) = pending.pop() {
        let current = task(tx, &task_id, false)?;
        if ["completed", "failed", "cancelled"].contains(&current["status"].as_str().unwrap()) {
            continue;
        }
        tx.execute("UPDATE tasks SET status='cancelled',last_error=?2,lease_token=NULL,lease_until=NULL,updated_at=?3 WHERE id=?1", params![task_id,reason,now()])?;
        message(
            tx,
            current["assignee"].as_str().unwrap(),
            current["reply_to"].as_str().unwrap(),
            "task_cancelled",
            Some(&task_id),
            &reason,
            &json!({"reason":reason}),
        )?;
        let mut stmt =
            tx.prepare("SELECT id FROM tasks WHERE status='blocked' AND depends_on=?1")?;
        let children = stmt
            .query_map([&task_id], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        pending.extend(
            children
                .into_iter()
                .map(|child| (child, format!("dependency {task_id} cancelled"))),
        );
    }
    Ok(())
}
/// 领取和总览时回收过期租约；用尽重试次数的任务进入终止失败。
pub(super) fn expire_leases(tx: &Transaction<'_>) -> Result<()> {
    let mut stmt = tx.prepare(
        "SELECT id,attempt,max_attempts FROM tasks WHERE status='leased' AND lease_until<=?1",
    )?;
    let expired = stmt
        .query_map([now()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for (task_id, attempt, max) in expired {
        if attempt >= max {
            terminal_fail(tx, &task_id, "maximum attempts reached after lease expiry")?;
        } else {
            tx.execute("UPDATE tasks SET status='queued',lease_token=NULL,lease_until=NULL,last_error='lease expired',updated_at=?2 WHERE id=?1",params![task_id,now()])?;
        }
    }
    Ok(())
}
