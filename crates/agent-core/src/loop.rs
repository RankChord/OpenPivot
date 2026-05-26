use crate::budget::IterationBudget;
use crate::types::*;
use agent_llm::{ChatMessage, ChatResponse, LlmError, MessageRole};
use agent_tools::{ToolCall as ToolCallDef, ToolRegistry, ToolResult};
use std::sync::Arc;

#[async_trait::async_trait]
pub trait AgentLlm: Send + Sync {
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[serde_json::Value]>,
    ) -> Result<ChatResponse, LlmError>;
}

#[async_trait::async_trait]
impl AgentLlm for agent_llm::LlmClient {
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[serde_json::Value]>,
    ) -> Result<ChatResponse, LlmError> {
        agent_llm::LlmClient::chat(self, messages, tools).await
    }
}

pub struct Agent {
    llm_client: Arc<dyn AgentLlm>,
    tool_registry: ToolRegistry,
    budget: IterationBudget,
    permissions: agent_config::PermissionPolicy,
}

impl Agent {
    pub fn new(
        llm_client: agent_llm::LlmClient,
        tool_registry: ToolRegistry,
        budget: IterationBudget,
    ) -> Self {
        Self::new_with_llm(Arc::new(llm_client), tool_registry, budget)
    }

    pub fn new_with_llm(
        llm_client: Arc<dyn AgentLlm>,
        tool_registry: ToolRegistry,
        budget: IterationBudget,
    ) -> Self {
        Self {
            llm_client,
            tool_registry,
            budget,
            permissions: agent_config::PermissionPolicy::default(),
        }
    }

    pub fn with_permissions(mut self, permissions: agent_config::PermissionPolicy) -> Self {
        self.permissions = permissions;
        self
    }

    pub async fn run(&self, input: &mut AgentInput) -> Result<AgentOutput, AgentError> {
        let mut collector = AgentEventCollector::new();
        self.run_with_events(input, &mut collector).await
    }

    pub async fn run_with_events(
        &self,
        input: &mut AgentInput,
        events: &mut dyn AgentEventSink,
    ) -> Result<AgentOutput, AgentError> {
        // 1. 将本轮用户消息加入历史
        tracing::info!(session = %input.session_id, "agent run started");
        input.history.push(ChatMessage {
            role: MessageRole::User,
            content: Some(input.user_message.clone()),
            tool_calls: None,
            tool_call_id: None,
        });
        events
            .emit(AgentEvent::UserMessage {
                content: input.user_message.clone(),
            })
            .await?;

        let mut messages = build_api_messages(&input.system_prompt, &input.history);

        // Convert registry tool schemas to OpenAI format
        let tool_schemas: Vec<serde_json::Value> = self
            .tool_registry
            .get_tool_schemas()
            .await
            .iter()
            .map(|ts| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": ts.name,
                        "description": ts.description,
                        "parameters": ts.input_schema
                    }
                })
            })
            .collect();
        let tools_for_llm = if tool_schemas.is_empty() {
            None
        } else {
            Some(tool_schemas)
        };

        loop {
            if self.budget.exhausted() {
                if self.budget.grace_call {
                    // Grace call: let model summarize before ending
                    messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: Some(
                            "Budget exhausted. Please summarize what you've accomplished so far."
                                .into(),
                        ),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                let reason = "Iteration budget exhausted".to_string();
                events
                    .emit(AgentEvent::BudgetExhausted {
                        reason: reason.clone(),
                    })
                    .await?;
                return Ok(AgentOutput::BudgetExhausted(reason));
            }

            let tools_ref = tools_for_llm.as_ref().map(|t| t.as_slice());
            let response = self
                .llm_client
                .chat(messages.clone(), tools_ref)
                .await
                .map_err(|e| AgentError::LlmError(e.to_string()))?;

            if let Some(tool_calls) = &response.message.tool_calls {
                if !tool_calls.is_empty() {
                    for tc in tool_calls {
                        let tool_name = &tc.function.name;
                        tracing::info!(tool = %tool_name, tool_call_id = %tc.id, "tool call requested");
                        events
                            .emit(AgentEvent::ToolCall {
                                id: tc.id.clone(),
                                name: tool_name.clone(),
                            })
                            .await?;
                        let input_args: serde_json::Value = match serde_json::from_str(
                            &tc.function.arguments,
                        ) {
                            Ok(args) => args,
                            Err(e) => {
                                // Report JSON parse error back to LLM instead of silently passing {}
                                let error_result = ToolResult {
                                    ok: false,
                                    content: String::new(),
                                    error: Some(format!("Invalid tool arguments: {}", e)),
                                };
                                messages.push(ChatMessage {
                                    role: MessageRole::Tool,
                                    content: Some(error_result.content.clone()),
                                    tool_calls: None,
                                    tool_call_id: Some(tc.id.clone()),
                                });
                                tracing::warn!(tool = %tool_name, error = %e, "invalid tool arguments");
                                events
                                    .emit(AgentEvent::ToolResult {
                                        id: tc.id.clone(),
                                        ok: false,
                                        content: error_result.content.clone(),
                                        error: error_result.error.clone(),
                                    })
                                    .await?;
                                continue;
                            }
                        };

                        let result = self
                            .execute_tool(
                                tool_name,
                                input_args,
                                &input.session_id,
                                &input.workspace_dir,
                            )
                            .await;

                        messages.push(ChatMessage {
                            role: MessageRole::Tool,
                            content: Some(tool_result_message_content(&result)),
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                        });
                        events
                            .emit(AgentEvent::ToolResult {
                                id: tc.id.clone(),
                                ok: result.ok,
                                content: result.content.clone(),
                                error: result.error.clone(),
                            })
                            .await?;

                        if !result.ok {
                            tracing::warn!(tool = %tool_name, error = ?result.error, "tool execution failed");
                        } else {
                            tracing::info!(tool = %tool_name, "tool execution succeeded");
                        }
                    }

                    self.budget.increment();
                    continue;
                }
            }

            if let Some(text) = &response.message.content {
                tracing::info!(session = %input.session_id, "agent run completed");
                events
                    .emit(AgentEvent::Final {
                        content: text.clone(),
                    })
                    .await?;
                return Ok(AgentOutput::Final(text.clone()));
            }

            events
                .emit(AgentEvent::Final {
                    content: String::new(),
                })
                .await?;
            return Ok(AgentOutput::Final(String::new()));
        }
    }

    async fn execute_tool(
        &self,
        name: &str,
        input: serde_json::Value,
        session_id: &str,
        workspace_dir: &std::path::Path,
    ) -> ToolResult {
        let tool_call = ToolCallDef {
            id: format!("tc_{}", uuid::Uuid::new_v4().simple()),
            name: name.into(),
            input,
            session_id: session_id.into(),
            workspace_dir: workspace_dir.to_path_buf(),
        };

        self.tool_registry
            .execute_with_permission(tool_call, &|check| {
                tool_permission_for_policy(&self.permissions, check)
            })
            .await
    }
}
