use agent_config::AgentConfig;
use agent_core::{Agent, IterationBudget};
use agent_llm::{LlmClient, LlmClientConfig, LlmProviderType};
use agent_tools::ToolRegistry;
use std::path::PathBuf;

pub fn provider_from_config(provider: &str) -> LlmProviderType {
    match provider {
        "anthropic" => LlmProviderType::Anthropic,
        "google" => LlmProviderType::Google,
        _ => LlmProviderType::OpenAI,
    }
}

pub fn llm_config_from_agent_config(
    config: &AgentConfig,
    override_model: Option<String>,
) -> LlmClientConfig {
    LlmClientConfig {
        provider: provider_from_config(&config.model.provider),
        base_url: config
            .model
            .base_url
            .clone()
            .unwrap_or("https://api.openai.com/v1".into()),
        api_key: config
            .model
            .api_key
            .clone()
            .or_else(|| std::env::var("AGENT_API_KEY").ok())
            .unwrap_or("no-key-provided".into()),
        model: override_model.unwrap_or_else(|| config.model.name.clone()),
        max_tokens: config.model.max_tokens,
        temperature: config.model.temperature,
    }
}

pub fn default_session_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("sessions.db")
}

pub async fn register_default_tools(registry: &ToolRegistry) {
    registry
        .register(Box::new(read_file::ReadFileTool::new()))
        .await;
    registry
        .register(Box::new(write_file::WriteFileTool::new()))
        .await;
    registry
        .register(Box::new(shell_tool::ShellTool::new()))
        .await;
    registry
        .register(Box::new(todo_tool::TodoTool::new()))
        .await;
    registry
        .register(Box::new(cron_job::CronJobTool::new()))
        .await;
    registry
        .register(Box::new(browser_tool::BrowserTool::new()))
        .await;
    registry
        .register(Box::new(delegate_tool::DelegateTool::new()))
        .await;
    registry
        .register(Box::new(web_search::WebSearchTool::new()))
        .await;
}

pub async fn build_agent(config: &AgentConfig, override_model: Option<String>) -> Agent {
    let client = LlmClient::new(llm_config_from_agent_config(config, override_model));
    let registry = ToolRegistry::new();
    register_default_tools(&registry).await;

    Agent::new(
        client,
        registry,
        IterationBudget::new(config.agent.max_iterations),
    )
    .with_permissions(config.permissions.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_unknown_provider_to_openai_compatible() {
        assert!(matches!(
            provider_from_config("openai-compatible"),
            LlmProviderType::OpenAI
        ));
    }

    #[test]
    fn builds_llm_config_with_override_model() {
        let mut config = AgentConfig::default();
        config.model.provider = "openai".into();
        config.model.name = "default-model".into();
        config.model.base_url = Some("http://localhost:8317".into());
        config.model.api_key = Some("test-key".into());

        let llm = llm_config_from_agent_config(&config, Some("override-model".into()));

        assert!(matches!(llm.provider, LlmProviderType::OpenAI));
        assert_eq!(llm.model, "override-model");
        assert_eq!(llm.base_url, "http://localhost:8317");
        assert_eq!(llm.api_key, "test-key");
    }
}
