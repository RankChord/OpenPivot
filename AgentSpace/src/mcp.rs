//! MCP 协议适配层：工具元数据 → 已验证 Agent 身份 → 共用 Store 业务入口。
//! 不在这里实现另一套任务状态机；REST 和 MCP 的行为由同一事务代码决定。
use rmcp::{
    ErrorData, RoleServer,
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
    },
    service::RequestContext,
};
use serde_json::{Map, Value, json};

pub use crate::auth::Principal;
use crate::store::Store;

#[derive(Clone)]
pub struct McpServer {
    store: Store,
}

impl McpServer {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

/// MCP 只公布 Agent 协作能力；注册、凭证与管理视图不作为 Agent 工具。
fn tool_defs() -> Vec<Tool> {
    let rows: [(&str, &str, Value); 18] = [
        (
            "agent_list",
            "List dynamically registered agents and their responsibilities.",
            object_schema(&[], &[]),
        ),
        (
            "agent_get",
            "Get one agent by its agent_id.",
            object_schema(&[("agent_id", "string")], &["agent_id"]),
        ),
        (
            "file_list",
            "List files, optionally below a prefix.",
            object_schema(&[("prefix", "string")], &[]),
        ),
        (
            "file_read",
            "Read a file at its current version or the requested historical version.",
            object_schema(&[("path", "string"), ("version", "integer")], &["path"]),
        ),
        (
            "file_write",
            "Write file bytes using exactly one content encoding. expected_version is required; use the current version to prevent lost updates.",
            object_schema(
                &[
                    ("path", "string"),
                    ("content", "string"),
                    ("content_base64", "string"),
                    ("expected_version", "integer"),
                ],
                &["path", "expected_version"],
            ),
        ),
        (
            "task_enqueue",
            "Create an automatic task. Start with only the next known action; its assignee may choose another agent after reviewing responsibilities and results. With parent_task_id, only the parent assignee can delegate; mode and workflow identity inherit from the parent. Optional depends_on blocks until that task completes. Use workflow_create for a preplanned fixed flow. idempotency_key makes retries safe.",
            object_schema(
                &[
                    ("assignee", "string"),
                    ("title", "string"),
                    ("instructions", "string"),
                    ("idempotency_key", "string"),
                    ("reply_to", "string"),
                    ("parent_task_id", "string"),
                    ("collaboration_mode", "string"),
                    ("depends_on", "string"),
                ],
                &["assignee", "title", "instructions", "idempotency_key"],
            ),
        ),
        (
            "workflow_create",
            "Atomically create a fixed, preplanned linear workflow with 1..32 ordered steps. The first step is queued; each later step waits for the preceding step to complete. Steps retain their configured agents and dependencies. Returns the workflow ID and all tasks. idempotency_key makes retries safe.",
            object_schema(
                &[
                    ("title", "string"),
                    ("idempotency_key", "string"),
                    ("reply_to", "string"),
                    ("steps", "array"),
                ],
                &["title", "idempotency_key", "steps"],
            ),
        ),
        (
            "task_cancel",
            "Cancel a queued or blocked task created by the authenticated agent. Already cancelled tasks return idempotently. Leased, completed, and failed tasks cannot be cancelled. Cancellation propagates to blocked dependent tasks and invalidates lease tokens.",
            object_schema(
                &[("task_id", "string"), ("reason", "string")],
                &["task_id", "reason"],
            ),
        ),
        (
            "task_list",
            "List project tasks, optionally filtered by assignee and status. Lease tokens are hidden.",
            object_schema(&[("assignee", "string"), ("status", "string")], &[]),
        ),
        (
            "task_get",
            "Get task status by task_id.",
            object_schema(&[("task_id", "string")], &["task_id"]),
        ),
        (
            "task_claim",
            "Claim one available task. The returned lease_token is required for heartbeat, dependency, completion, and failure operations.",
            object_schema(&[("lease_seconds", "integer")], &[]),
        ),
        (
            "task_heartbeat",
            "Renew a task lease before it expires; stale lease tokens are rejected.",
            object_schema(
                &[
                    ("task_id", "string"),
                    ("lease_token", "string"),
                    ("lease_seconds", "integer"),
                ],
                &["task_id", "lease_token"],
            ),
        ),
        (
            "task_wait_on",
            "Record that the leased task depends on another task; this does not send a message. Automatic tasks may discover dependencies while working. Fixed stages cannot change their configured dependency. Failed or cancelled dependencies propagate that terminal state.",
            object_schema(
                &[
                    ("task_id", "string"),
                    ("lease_token", "string"),
                    ("dependency_task_id", "string"),
                ],
                &["task_id", "lease_token", "dependency_task_id"],
            ),
        ),
        (
            "task_complete",
            "Complete a leased task with a summary and optional version-checked output file writes.",
            object_schema(
                &[
                    ("task_id", "string"),
                    ("lease_token", "string"),
                    ("summary", "string"),
                    ("outputs", "array"),
                ],
                &["task_id", "lease_token", "summary"],
            ),
        ),
        (
            "task_fail",
            "Fail a leased task. retryable controls whether it may be claimed again.",
            object_schema(
                &[
                    ("task_id", "string"),
                    ("lease_token", "string"),
                    ("reason", "string"),
                    ("retryable", "boolean"),
                ],
                &["task_id", "lease_token", "reason"],
            ),
        ),
        (
            "inbox_poll",
            "Poll messages addressed to the authenticated agent; polling does not acknowledge them.",
            object_schema(&[("limit", "integer")], &[]),
        ),
        (
            "inbox_ack",
            "Acknowledge message IDs after processing them. Messages are separate from task state.",
            object_schema(&[("message_ids", "array")], &["message_ids"]),
        ),
        (
            "message_send",
            "Send an inbox message to an agent. idempotency_key makes retries safe; a message is separate from an assigned task.",
            object_schema(
                &[
                    ("to", "string"),
                    ("body", "string"),
                    ("idempotency_key", "string"),
                    ("task_id", "string"),
                ],
                &["to", "body", "idempotency_key"],
            ),
        ),
    ];
    rows.into_iter()
        .map(|(name, description, schema)| {
            Tool::new(
                name,
                description,
                schema.as_object().expect("object schema").clone(),
            )
        })
        .collect()
}

fn object_schema(properties: &[(&str, &str)], required: &[&str]) -> Value {
    let mut props = Map::new();
    for (name, kind) in properties {
        let mut spec = json!({"type": kind});
        if *name == "version"
            || *name == "expected_version"
            || *name == "lease_seconds"
            || *name == "limit"
        {
            spec["minimum"] = json!(0);
        }
        if *name == "outputs" {
            spec["items"] = json!({
                "type":"object", "additionalProperties":false,
                "properties": {"path":{"type":"string"}, "content":{"type":"string"}, "content_base64":{"type":"string"}, "expected_version":{"type":"integer","minimum":0}},
                "required":["path","expected_version"]
            });
        } else if *name == "message_ids" {
            spec["items"] = json!({"type":"string"});
        } else if *name == "steps" {
            spec["minItems"] = json!(1);
            spec["maxItems"] = json!(32);
            spec["items"] = object_schema(
                &[
                    ("assignee", "string"),
                    ("title", "string"),
                    ("instructions", "string"),
                ],
                &["assignee", "title", "instructions"],
            );
        } else if *name == "collaboration_mode" {
            // task_enqueue 仅创建自动任务；固定编排有独立的 workflow_create。
            spec["enum"] = json!(["automatic"]);
        } else if *name == "status" {
            spec["enum"] = json!([
                "queued",
                "leased",
                "blocked",
                "completed",
                "failed",
                "cancelled"
            ]);
        }
        props.insert((*name).into(), spec);
    }
    json!({"type":"object", "properties":props, "required":required, "additionalProperties":false})
}

fn to_json_value(args: Option<rmcp::model::JsonObject>) -> Value {
    Value::Object(args.unwrap_or_default())
}

fn tool_error(message: impl Into<String>, value: Value) -> CallToolResponse {
    let text = serde_json::to_string(&value).unwrap_or_else(|_| message.into());
    let mut result = CallToolResult::error(vec![ContentBlock::text(text)]);
    result.structured_content = Some(value);
    result.into()
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("agent-space-demo", "0.1.0"))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tool_defs()))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        tool_defs().into_iter().find(|tool| tool.name == name)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let actor = context
            .extensions
            .get::<http::request::Parts>()
            .and_then(|parts| parts.extensions.get::<Principal>())
            .map(|p| p.0.clone())
            .ok_or_else(|| {
                ErrorData::internal_error(
                    "authenticated principal missing from request context",
                    None,
                )
            })?;
        let args = to_json_value(request.arguments);
        if let Err(error) = validate_arguments(&request.name, &args) {
            let body = json!({"error": error.to_string()});
            return Ok(tool_error(error.to_string(), body));
        }
        let store = self.store.clone();
        let name = request.name.to_string();
        let result = tokio::task::spawn_blocking(move || store.execute(&actor, &name, args))
            .await
            .map_err(|e| {
                ErrorData::internal_error(format!("workspace worker failed: {e}"), None)
            })?;
        match result {
            Ok(value) => {
                let text = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_owned());
                let mut response = CallToolResult::success(vec![ContentBlock::text(text)]);
                response.structured_content = Some(value);
                Ok(response.into())
            }
            Err(error) => {
                let body = json!({"error": error.to_string()});
                Ok(tool_error(error.to_string(), body))
            }
        }
    }
}

fn validate_arguments(tool: &str, args: &Value) -> Result<(), String> {
    let Some(obj) = args.as_object() else {
        return Err("arguments must be an object".into());
    };
    let schema = tool_defs()
        .into_iter()
        .find(|t| t.name == tool)
        .ok_or_else(|| format!("unknown tool: {tool}"))?;
    let schema_value = Value::Object((*schema.input_schema).clone());
    let props = schema_value["properties"]
        .as_object()
        .expect("generated object schema");
    for key in obj.keys() {
        if !props.contains_key(key) {
            return Err(format!("unexpected argument: {key}"));
        }
    }
    for required in schema_value["required"].as_array().expect("required array") {
        let key = required.as_str().unwrap();
        if !obj.contains_key(key) {
            return Err(format!("missing required argument: {key}"));
        }
    }
    for (key, value) in obj {
        let kind = props[key]["type"].as_str().unwrap_or("");
        let valid = match kind {
            "string" => value.is_string(),
            "integer" => value.as_u64().is_some(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            _ => false,
        };
        if !valid {
            return Err(format!("argument {key} must be {kind}"));
        }
    }
    Ok(())
}
