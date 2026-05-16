use agent_llm::{ChatMessage, MessageRole};
use agent_tools::{ToolRegistry, ToolCall as ToolCallDef, ToolResult};
use crate::types::*;
use crate::budget::IterationBudget;

pub struct Agent {
    llm_client: agent_llm::LlmClient,
    tool_registry: ToolRegistry,
    budget: IterationBudget,
}

impl Agent {
    pub fn new(
        llm_client: agent_llm::LlmClient,
        tool_registry: ToolRegistry,
        budget: IterationBudget,
    ) -> Self {
        Self {
            llm_client,
            tool_registry,
            budget,
        }
    }

    pub async fn run(&self, mut input: AgentInput) -> Result<AgentOutput, AgentError> {
        let mut messages = build_api_messages(
            &input.system_prompt,
            &input.user_message,
            &input.history,
        );

        loop {
            if self.budget.exhausted() {
                if self.budget.grace_call {
                    // Grace call: let model summarize before ending
                    messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: Some("Budget exhausted. Please summarize what you've accomplished so far.".into()),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                return Ok(AgentOutput::BudgetExhausted(
                    "Iteration budget exhausted".into()
                ));
            }

            let response = self.llm_client.chat(messages.clone()).await
                .map_err(|e| AgentError::LlmError(e.to_string()))?;

            if let Some(tool_calls) = &response.message.tool_calls {
                if !tool_calls.is_empty() {
                    for tc in tool_calls {
                        let tool_name = &tc.function.name;
                        let input_args: serde_json::Value =
                            match serde_json::from_str(&tc.function.arguments) {
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
                                    continue;
                                }
                            };

                        let result = self.execute_tool(tool_name, input_args, &input.session_id, &input.workspace_dir).await;

                        messages.push(ChatMessage {
                            role: MessageRole::Tool,
                            content: Some(result.content.clone()),
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                        });

                        if !result.ok {
                            tracing::warn!(tool = %tool_name, error = ?result.error, "tool execution failed");
                        }
                    }

                    self.budget.increment();
                    continue;
                }
            }

            if let Some(text) = &response.message.content {
                return Ok(AgentOutput::Final(text.clone()));
            }

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

        self.tool_registry.execute(tool_call).await
    }
}
