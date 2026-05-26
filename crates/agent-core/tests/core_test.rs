use agent_core::budget::IterationBudget;
use agent_core::r#loop::AgentLlm;
use agent_llm::{ChatMessage, ChatResponse, FunctionCall, LlmError, LlmToolCall};
use agent_tools::{ProgressUpdate, Tool, ToolDefinition, ToolResult, ToolUseContext};
use std::sync::Mutex;

struct ImmediateLlm;

#[async_trait::async_trait]
impl AgentLlm for ImmediateLlm {
    async fn chat(
        &self,
        _messages: Vec<ChatMessage>,
        _tools: Option<&[serde_json::Value]>,
    ) -> Result<ChatResponse, LlmError> {
        Ok(ChatResponse {
            message: ChatMessage {
                role: agent_llm::MessageRole::Assistant,
                content: Some("done".into()),
                tool_calls: None,
                tool_call_id: None,
            },
            usage: None,
            finish_reason: Some("stop".into()),
        })
    }
}

struct ScriptedLlm {
    responses: Mutex<Vec<ChatResponse>>,
}

#[async_trait::async_trait]
impl AgentLlm for ScriptedLlm {
    async fn chat(
        &self,
        _messages: Vec<ChatMessage>,
        _tools: Option<&[serde_json::Value]>,
    ) -> Result<ChatResponse, LlmError> {
        let mut responses = self.responses.lock().unwrap();
        Ok(responses.remove(0))
    }
}

#[derive(Debug)]
struct EchoTool {
    def: ToolDefinition,
}

#[async_trait::async_trait]
impl Tool for EchoTool {
    fn definition(&self) -> &ToolDefinition {
        &self.def
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        ToolResult {
            ok: true,
            content: input["text"].as_str().unwrap_or_default().to_string(),
            error: None,
        }
    }
}

fn echo_tool() -> EchoTool {
    EchoTool {
        def: ToolDefinition {
            name: "echo".into(),
            description: "Echo input text".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"]
            }),
            toolset_name: "test".into(),
            aliases: vec![],
            max_result_size: 1000,
        },
    }
}

fn tool_call_response(arguments: &str) -> ChatResponse {
    ChatResponse {
        message: ChatMessage {
            role: agent_llm::MessageRole::Assistant,
            content: None,
            tool_calls: Some(vec![LlmToolCall {
                id: "call_1".into(),
                call_type: "function".into(),
                function: FunctionCall {
                    name: "echo".into(),
                    arguments: arguments.into(),
                },
            }]),
            tool_call_id: None,
        },
        usage: None,
        finish_reason: Some("tool_calls".into()),
    }
}

fn final_response(content: &str) -> ChatResponse {
    ChatResponse {
        message: ChatMessage {
            role: agent_llm::MessageRole::Assistant,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        },
        usage: None,
        finish_reason: Some("stop".into()),
    }
}

#[tokio::test]
async fn test_agent_can_run_with_injected_llm_trait() {
    let registry = agent_tools::ToolRegistry::new();
    let agent = agent_core::Agent::new_with_llm(
        std::sync::Arc::new(ImmediateLlm),
        registry,
        IterationBudget::new(3),
    );
    let mut input = agent_core::AgentInput {
        user_message: "hello".into(),
        session_id: "session".into(),
        system_prompt: "system".into(),
        history: Vec::new(),
        workspace_dir: std::env::current_dir().unwrap(),
    };

    let output = agent.run(&mut input).await.unwrap();

    assert!(matches!(output, agent_core::AgentOutput::Final(text) if text == "done"));
}

#[tokio::test]
async fn test_agent_emits_tool_and_final_events_with_scripted_llm() {
    let registry = agent_tools::ToolRegistry::new();
    registry.register(Box::new(echo_tool())).await;

    let llm = ScriptedLlm {
        responses: Mutex::new(vec![
            tool_call_response("{\"text\":\"hello tool\"}"),
            final_response("done"),
        ]),
    };

    let agent = agent_core::Agent::new_with_llm(
        std::sync::Arc::new(llm),
        registry,
        IterationBudget::new(3),
    )
    .with_permissions(agent_config::PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec!["echo".into()],
        ask_tools: vec![],
        allowed_dirs: vec![],
    });
    let mut input = agent_core::AgentInput {
        user_message: "use echo".into(),
        session_id: "session".into(),
        system_prompt: "system".into(),
        history: Vec::new(),
        workspace_dir: std::env::current_dir().unwrap(),
    };
    let mut events = agent_core::AgentEventCollector::new();

    let output = agent
        .run_with_events(&mut input, &mut events)
        .await
        .unwrap();

    assert!(matches!(output, agent_core::AgentOutput::Final(text) if text == "done"));
    assert!(events.events().contains(&agent_core::AgentEvent::ToolCall {
        id: "call_1".into(),
        name: "echo".into(),
    }));
    assert!(
        events
            .events()
            .contains(&agent_core::AgentEvent::ToolResult {
                id: "call_1".into(),
                ok: true,
                content: "hello tool".into(),
                error: None,
            })
    );
    assert!(events.events().contains(&agent_core::AgentEvent::Final {
        content: "done".into(),
    }));
}

#[tokio::test]
async fn test_agent_emits_permission_denied_tool_result_and_recovers() {
    let registry = agent_tools::ToolRegistry::new();
    registry.register(Box::new(echo_tool())).await;
    let llm = ScriptedLlm {
        responses: Mutex::new(vec![
            tool_call_response("{\"text\":\"blocked\"}"),
            final_response("recovered"),
        ]),
    };
    let agent = agent_core::Agent::new_with_llm(
        std::sync::Arc::new(llm),
        registry,
        IterationBudget::new(3),
    )
    .with_permissions(agent_config::PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec![],
        ask_tools: vec![],
        allowed_dirs: vec![],
    });
    let mut input = agent_core::AgentInput {
        user_message: "use echo".into(),
        session_id: "session".into(),
        system_prompt: "system".into(),
        history: Vec::new(),
        workspace_dir: std::env::current_dir().unwrap(),
    };
    let mut events = agent_core::AgentEventCollector::new();

    let output = agent
        .run_with_events(&mut input, &mut events)
        .await
        .unwrap();

    assert!(matches!(output, agent_core::AgentOutput::Final(text) if text == "recovered"));
    let denied = events.events().iter().find_map(|event| match event {
        agent_core::AgentEvent::ToolResult { ok, error, .. } if !ok => error.as_deref(),
        _ => None,
    });
    assert!(denied.unwrap().contains("Tool not allowed by policy"));
}

#[tokio::test]
async fn test_agent_emits_invalid_tool_arguments_and_recovers() {
    let registry = agent_tools::ToolRegistry::new();
    registry.register(Box::new(echo_tool())).await;
    let llm = ScriptedLlm {
        responses: Mutex::new(vec![
            tool_call_response("not json"),
            final_response("recovered"),
        ]),
    };
    let agent = agent_core::Agent::new_with_llm(
        std::sync::Arc::new(llm),
        registry,
        IterationBudget::new(3),
    )
    .with_permissions(agent_config::PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec!["echo".into()],
        ask_tools: vec![],
        allowed_dirs: vec![],
    });
    let mut input = agent_core::AgentInput {
        user_message: "use echo".into(),
        session_id: "session".into(),
        system_prompt: "system".into(),
        history: Vec::new(),
        workspace_dir: std::env::current_dir().unwrap(),
    };
    let mut events = agent_core::AgentEventCollector::new();

    let output = agent
        .run_with_events(&mut input, &mut events)
        .await
        .unwrap();

    assert!(matches!(output, agent_core::AgentOutput::Final(text) if text == "recovered"));
    let invalid_args = events.events().iter().find_map(|event| match event {
        agent_core::AgentEvent::ToolResult { ok, error, .. } if !ok => error.as_deref(),
        _ => None,
    });
    assert!(invalid_args.unwrap().contains("Invalid tool arguments"));
}

#[tokio::test]
async fn test_agent_emits_budget_exhausted_after_tool_iteration_limit() {
    let registry = agent_tools::ToolRegistry::new();
    registry.register(Box::new(echo_tool())).await;
    let llm = ScriptedLlm {
        responses: Mutex::new(vec![tool_call_response("{\"text\":\"again\"}")]),
    };
    let agent = agent_core::Agent::new_with_llm(
        std::sync::Arc::new(llm),
        registry,
        IterationBudget::new(1),
    )
    .with_permissions(agent_config::PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec!["echo".into()],
        ask_tools: vec![],
        allowed_dirs: vec![],
    });
    let mut input = agent_core::AgentInput {
        user_message: "loop".into(),
        session_id: "session".into(),
        system_prompt: "system".into(),
        history: Vec::new(),
        workspace_dir: std::env::current_dir().unwrap(),
    };
    let mut events = agent_core::AgentEventCollector::new();

    let output = agent
        .run_with_events(&mut input, &mut events)
        .await
        .unwrap();

    assert!(matches!(
        output,
        agent_core::AgentOutput::BudgetExhausted(_)
    ));
    assert!(events.events().iter().any(|event| matches!(
        event,
        agent_core::AgentEvent::BudgetExhausted { reason }
            if reason == "Iteration budget exhausted"
    )));
}

#[test]
fn test_budget_counts_iterations() {
    let budget = IterationBudget::new(3);
    assert!(!budget.exhausted());
    assert_eq!(budget.remaining(), 3);

    budget.increment();
    assert!(!budget.exhausted());
    assert_eq!(budget.remaining(), 2);

    budget.increment();
    budget.increment();
    assert!(budget.exhausted());
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn test_budget_large_value() {
    let budget = IterationBudget::new(9999);
    assert!(!budget.exhausted());
    assert_eq!(budget.remaining(), 9999);
}

#[test]
fn test_build_messages() {
    use agent_core::types::build_api_messages;

    let messages = build_api_messages(
        "You are a helpful assistant",
        &[agent_llm::ChatMessage {
            role: agent_llm::MessageRole::User,
            content: Some("Hello!".into()),
            tool_calls: None,
            tool_call_id: None,
        }],
    );

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, agent_llm::MessageRole::System);
    assert_eq!(messages[1].role, agent_llm::MessageRole::User);
    assert_eq!(messages[1].content.as_deref(), Some("Hello!"));
}

#[test]
fn test_build_messages_does_not_duplicate_current_user_message() {
    use agent_core::types::build_api_messages;
    use agent_llm::{ChatMessage, MessageRole};

    let history = vec![ChatMessage {
        role: MessageRole::User,
        content: Some("Hello!".into()),
        tool_calls: None,
        tool_call_id: None,
    }];

    let messages = build_api_messages("You are a helpful assistant", &history);

    let user_messages: Vec<_> = messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .collect();

    assert_eq!(user_messages.len(), 1);
    assert_eq!(user_messages[0].content.as_deref(), Some("Hello!"));
}

#[test]
fn test_permission_policy_denies_tools_not_in_allowlist() {
    use agent_config::PermissionPolicy;
    use agent_core::types::tool_permission_for_policy;
    use agent_tools::{PermissionCheck, PermissionResult};

    let policy = PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec!["read_file".into()],
        ask_tools: vec![],
        allowed_dirs: vec![],
    };

    let result = tool_permission_for_policy(
        &policy,
        &PermissionCheck {
            tool_name: "write_file".into(),
            input: serde_json::json!({"path": "notes.md"}),
            is_read_only: false,
            is_destructive: false,
        },
    );

    assert!(matches!(result, PermissionResult::Deny(_)));
}

#[test]
fn test_permission_policy_allows_allowlisted_tools() {
    use agent_config::PermissionPolicy;
    use agent_core::types::tool_permission_for_policy;
    use agent_tools::{PermissionCheck, PermissionResult};

    let policy = PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec!["read_file".into()],
        ask_tools: vec![],
        allowed_dirs: vec![],
    };

    let result = tool_permission_for_policy(
        &policy,
        &PermissionCheck {
            tool_name: "read_file".into(),
            input: serde_json::json!({"path": "notes.md"}),
            is_read_only: true,
            is_destructive: false,
        },
    );

    assert!(matches!(result, PermissionResult::Allow));
}

#[test]
fn test_permission_policy_denies_paths_outside_allowed_dirs() {
    use agent_config::PermissionPolicy;
    use agent_core::types::tool_permission_for_policy;
    use agent_tools::{PermissionCheck, PermissionResult};

    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let policy = PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec!["read_file".into()],
        ask_tools: vec![],
        allowed_dirs: vec![dir.path().to_path_buf()],
    };

    let result = tool_permission_for_policy(
        &policy,
        &PermissionCheck {
            tool_name: "read_file".into(),
            input: serde_json::json!({"path": outside.path().join("secret.txt").display().to_string()}),
            is_read_only: true,
            is_destructive: false,
        },
    );

    assert!(matches!(result, PermissionResult::Deny(_)));
}

#[test]
fn test_permission_policy_allows_new_file_under_allowed_dir() {
    use agent_config::PermissionPolicy;
    use agent_core::types::tool_permission_for_policy;
    use agent_tools::{PermissionCheck, PermissionResult};

    let dir = tempfile::tempdir().unwrap();
    let policy = PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec!["write_file".into()],
        ask_tools: vec![],
        allowed_dirs: vec![dir.path().to_path_buf()],
    };

    let result = tool_permission_for_policy(
        &policy,
        &PermissionCheck {
            tool_name: "write_file".into(),
            input: serde_json::json!({"path": dir.path().join("new.txt").display().to_string()}),
            is_read_only: false,
            is_destructive: false,
        },
    );

    assert!(matches!(result, PermissionResult::Allow));
}

#[test]
fn test_permission_policy_read_only_denies_non_read_only_tools() {
    use agent_config::PermissionPolicy;
    use agent_core::types::tool_permission_for_policy;
    use agent_tools::{PermissionCheck, PermissionResult};

    let policy = PermissionPolicy {
        auto_approve_tools: true,
        read_only: true,
        allow_destructive_tools: false,
        allowed_tools: vec!["write_file".into()],
        ask_tools: vec![],
        allowed_dirs: vec![],
    };

    let result = tool_permission_for_policy(
        &policy,
        &PermissionCheck {
            tool_name: "write_file".into(),
            input: serde_json::json!({"path": "notes.md"}),
            is_read_only: false,
            is_destructive: false,
        },
    );

    assert!(matches!(result, PermissionResult::Deny(_)));
}

#[test]
fn test_permission_policy_denies_destructive_tools_by_default() {
    use agent_config::PermissionPolicy;
    use agent_core::types::tool_permission_for_policy;
    use agent_tools::{PermissionCheck, PermissionResult};

    let policy = PermissionPolicy {
        auto_approve_tools: true,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec!["shell".into()],
        ask_tools: vec![],
        allowed_dirs: vec![],
    };

    let result = tool_permission_for_policy(
        &policy,
        &PermissionCheck {
            tool_name: "shell".into(),
            input: serde_json::json!({"command": "rm -rf target/tmp"}),
            is_read_only: false,
            is_destructive: true,
        },
    );

    assert!(matches!(result, PermissionResult::Deny(_)));
}

#[test]
fn test_permission_policy_allows_read_only_input_for_destructive_tool_type() {
    use agent_config::PermissionPolicy;
    use agent_core::types::tool_permission_for_policy;
    use agent_tools::{PermissionCheck, PermissionResult};

    let policy = PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: false,
        allowed_tools: vec!["execute_command".into()],
        ask_tools: vec![],
        allowed_dirs: vec![],
    };

    let result = tool_permission_for_policy(
        &policy,
        &PermissionCheck {
            tool_name: "execute_command".into(),
            input: serde_json::json!({"command": "ls"}),
            is_read_only: true,
            is_destructive: true,
        },
    );

    assert!(matches!(result, PermissionResult::Allow));
}

#[test]
fn test_permission_policy_asks_for_tools_when_not_auto_approved() {
    use agent_config::PermissionPolicy;
    use agent_core::types::tool_permission_for_policy;
    use agent_tools::{PermissionCheck, PermissionResult};

    let policy = PermissionPolicy {
        auto_approve_tools: false,
        read_only: false,
        allow_destructive_tools: true,
        allowed_tools: vec![],
        ask_tools: vec!["shell".into()],
        allowed_dirs: vec![],
    };

    let result = tool_permission_for_policy(
        &policy,
        &PermissionCheck {
            tool_name: "shell".into(),
            input: serde_json::json!({"command": "ls"}),
            is_read_only: false,
            is_destructive: false,
        },
    );

    assert!(matches!(result, PermissionResult::Ask(_)));
}

#[tokio::test]
async fn test_agent_event_collector_records_events_in_order() {
    use agent_core::types::{AgentEvent, AgentEventCollector, AgentEventSink};

    let mut collector = AgentEventCollector::new();
    collector
        .emit(AgentEvent::UserMessage {
            content: "hello".into(),
        })
        .await
        .unwrap();
    collector
        .emit(AgentEvent::Final {
            content: "world".into(),
        })
        .await
        .unwrap();

    assert_eq!(
        collector.events(),
        &[
            AgentEvent::UserMessage {
                content: "hello".into()
            },
            AgentEvent::Final {
                content: "world".into()
            }
        ]
    );
}

#[tokio::test]
async fn test_agent_event_callback_sink_receives_events_immediately() {
    use agent_core::types::{AgentEvent, AgentEventSink, callback_event_sink};
    use std::sync::{Arc, Mutex};

    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let mut sink = callback_event_sink(move |event| {
        captured.lock().unwrap().push(event);
    });

    sink.emit(AgentEvent::Final {
        content: "done".into(),
    })
    .await
    .unwrap();

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[AgentEvent::Final {
            content: "done".into()
        }]
    );
}

#[test]
fn test_tool_result_content_prefers_error_when_content_is_empty() {
    use agent_core::types::tool_result_message_content;
    use agent_tools::ToolResult;

    let result = ToolResult {
        ok: false,
        content: String::new(),
        error: Some("permission denied".into()),
    };

    assert_eq!(tool_result_message_content(&result), "permission denied");
}

#[test]
fn test_tool_result_content_keeps_success_content() {
    use agent_core::types::tool_result_message_content;
    use agent_tools::ToolResult;

    let result = ToolResult {
        ok: true,
        content: "file contents".into(),
        error: None,
    };

    assert_eq!(tool_result_message_content(&result), "file contents");
}
