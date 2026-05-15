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

    pub async fn run(&self, input: AgentInput) -> Result<AgentOutput, Box<dyn std::error::Error + Send + Sync>> {
        let mut messages = build_api_messages(
            &input.system_prompt,
            &input.user_message,
            &input.history,
        );

        loop {
            if self.budget.exhausted() {
                return Ok(AgentOutput::BudgetExhausted(
                    "Iteration budget exhausted".into()
                ));
            }

            let response = self.llm_client.chat(messages.clone()).await?;

            if let Some(tool_calls) = &response.message.tool_calls {
                if !tool_calls.is_empty() {
                    for tc in tool_calls {
                        let tool_name = &tc.function.name;
                        let input_args: serde_json::Value =
                            serde_json::from_str(&tc.function.arguments)
                                .unwrap_or(serde_json::json!({}));

                        let result = self.execute_tool(tool_name, input_args, &input.session_id).await;

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
    ) -> ToolResult {
        let tool_call = ToolCallDef {
            id: format!("tc_{}", uuid::Uuid::new_v4().simple()),
            name: name.into(),
            input,
            session_id: session_id.into(),
            workspace_dir: std::env::current_dir().unwrap_or_default(),
        };

        self.tool_registry.execute(tool_call).await
    }
}
