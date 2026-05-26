use crate::cmd::load_config;
use crate::runtime::{build_agent, default_session_db_path};
use agent_core::{AgentInput, AgentOutput};
use agent_llm::{ChatMessage, MessageRole};
use agent_sessions::SessionStore;
use agent_skills::SkillEngine;
use std::io::{self, Write};
use std::path::PathBuf;

pub async fn run(
    config_path: Option<PathBuf>,
    override_model: Option<String>,
) -> anyhow::Result<()> {
    let config = load_config(config_path);

    // 1. 初始化 Session Store
    let db_path = default_session_db_path();
    let store = SessionStore::new(&db_path).await?;

    // 1.5 扫描 Skills
    let mut skills_engine = SkillEngine::new();
    let skill_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("skills");
    skills_engine.discover(&[skill_dir]).await;
    let skills_prompt = skills_engine.build_system_prompt_snippet();

    let session_id = "default-chat".to_string();
    let mut history = store.load_history(&session_id).await?;
    history.extend(store.load_event_history(&session_id).await?);

    let agent = build_agent(&config, override_model).await;

    println!("Session loaded ({} messages)", history.len());

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input_str = String::new();
        io::stdin().read_line(&mut input_str)?;
        let input_text = input_str.trim().to_string();

        if input_text.is_empty() || input_text.eq_ignore_ascii_case("exit") {
            break;
        }

        // 存入 User Message 到 history (内存中)
        history.push(ChatMessage {
            role: MessageRole::User,
            content: Some(input_text.clone()),
            tool_calls: None,
            tool_call_id: None,
        });
        // 异步保存到 DB
        let _ = store
            .append_messages(&session_id, &[history.last().unwrap().clone()])
            .await;

        let mut agent_input = AgentInput {
            user_message: input_text,
            session_id: session_id.clone(),
            system_prompt: format!("You are an AI assistant with skills:\n{}", skills_prompt),
            history: history.clone(),
            workspace_dir: std::env::current_dir()?,
        };

        let mut events = agent_core::AgentEventCollector::new();
        match agent.run_with_events(&mut agent_input, &mut events).await {
            Ok(AgentOutput::Final(response_text)) => {
                println!("\n{}\n", response_text);
                let _ = store.append_events(&session_id, events.events()).await;

                // 存入 Assistant Message
                history.push(ChatMessage {
                    role: MessageRole::Assistant,
                    content: Some(response_text),
                    tool_calls: None,
                    tool_call_id: None,
                });
                let _ = store
                    .append_messages(&session_id, &[history.last().unwrap().clone()])
                    .await;
            }
            Ok(AgentOutput::BudgetExhausted(msg)) => {
                let _ = store.append_events(&session_id, events.events()).await;
                eprintln!("\n{}\n", msg);
                break;
            }
            Err(e) => {
                eprintln!("\nError: {}\n", e);
            }
            _ => {}
        }
    }
    Ok(())
}
