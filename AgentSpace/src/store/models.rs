//! 请求结构：拒绝未知字段，避免客户端把 actor 等权限字段偷偷塞入请求。
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentGet {
    pub(super) agent_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FileList {
    #[serde(default)]
    pub(super) prefix: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FileRead {
    pub(super) path: String,
    pub(super) version: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FileWrite {
    pub(super) path: String,
    pub(super) content: Option<String>,
    pub(super) content_base64: Option<String>,
    pub(super) expected_version: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Enqueue {
    pub(super) assignee: String,
    pub(super) title: String,
    pub(super) instructions: String,
    pub(super) idempotency_key: String,
    pub(super) reply_to: Option<String>,
    pub(super) parent_task_id: Option<String>,
    pub(super) collaboration_mode: Option<String>,
    pub(super) depends_on: Option<String>,
}
/// 固定编排一次提交整条线性流程，避免只写入前几步的半成品。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkflowCreate {
    pub(super) title: String,
    pub(super) idempotency_key: String,
    pub(super) reply_to: Option<String>,
    pub(super) steps: Vec<WorkflowStep>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkflowStep {
    pub(super) assignee: String,
    pub(super) title: String,
    pub(super) instructions: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cancel {
    pub(super) task_id: String,
    pub(super) reason: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaskGet {
    pub(super) task_id: String,
}
/// 面向项目总览的查询，不暴露任何任务租约凭证。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaskList {
    pub(super) assignee: Option<String>,
    pub(super) status: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Claim {
    pub(super) lease_seconds: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Heartbeat {
    pub(super) task_id: String,
    pub(super) lease_token: String,
    pub(super) lease_seconds: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WaitOn {
    pub(super) task_id: String,
    pub(super) lease_token: String,
    pub(super) dependency_task_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Complete {
    pub(super) task_id: String,
    pub(super) lease_token: String,
    pub(super) summary: String,
    #[serde(default)]
    pub(super) outputs: Vec<FileWrite>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Fail {
    pub(super) task_id: String,
    pub(super) lease_token: String,
    pub(super) reason: String,
    #[serde(default)]
    pub(super) retryable: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Poll {
    pub(super) limit: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ack {
    pub(super) message_ids: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SendMessage {
    pub(super) to: String,
    pub(super) body: String,
    pub(super) idempotency_key: String,
    pub(super) task_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Empty {}
