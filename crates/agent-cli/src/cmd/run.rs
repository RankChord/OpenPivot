use crate::cmd::load_config;
use crate::runtime::{build_agent, default_session_db_path};
use agent_core::{AgentEvent, AgentEventCollector, AgentInput, AgentOutput};
use agent_sessions::SessionStore;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

pub async fn run(
    config_path: Option<PathBuf>,
    message_args: Vec<String>,
    _continue_session: bool,
    session_id: Option<String>,
    override_model: Option<String>,
) -> anyhow::Result<()> {
    let config = load_config(config_path);
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let agent = build_agent(&config, override_model).await;

    // 4. 收集提示词（来自参数或管道）
    let mut prompt = message_args.join(" ");

    if prompt.trim().is_empty() {
        // 尝试从标准输入读取管道内容
        if !io::stdin().is_terminal() {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            prompt = buffer.trim().to_string();
        }

        // 如果还是没有输入，进入交互模式
        if prompt.trim().is_empty() {
            print!("> ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut prompt)?;
            prompt = prompt.trim().to_string();
        }
    }

    // 5. 运行
    let sid = session_id.unwrap_or_else(|| "adhoc-session".into());
    let history = load_event_history(&sid).await;
    let mut input = AgentInput {
        user_message: prompt,
        session_id: sid.clone(),
        system_prompt: "You are a helpful assistant capable of using tools to solve problems in a terminal environment.".to_string(),
        history,
        workspace_dir: workspace.clone(),
    };

    // Agent::run 目前是单轮执行，这里我们简单处理，只调用一次
    let mut events = AgentEventCollector::new();
    match agent.run_with_events(&mut input, &mut events).await {
        Ok(AgentOutput::Final(response)) => {
            persist_events(&sid, events.events()).await;
            for event in events.events() {
                if let Some(line) = format_event_line(event) {
                    eprintln!("{}", line);
                }
            }
            println!("\n{}", response);
        }
        Ok(AgentOutput::BudgetExhausted(msg)) => {
            persist_events(&sid, events.events()).await;
            eprintln!("\n{}", msg);
        }
        Ok(AgentOutput::Cancelled) => {
            eprintln!("\nCancelled by user.");
        }
        Err(e) => {
            eprintln!("\nError: {}", e);
        }
    }

    Ok(())
}

async fn persist_events(session_id: &str, events: &[AgentEvent]) {
    let db_path = default_session_db_path();
    if let Ok(store) = SessionStore::new(&db_path).await {
        let _ = store.append_events(session_id, events).await;
    }
}

async fn load_event_history(session_id: &str) -> Vec<agent_llm::ChatMessage> {
    let db_path = default_session_db_path();
    match SessionStore::new(&db_path).await {
        Ok(store) => store
            .load_event_history(session_id)
            .await
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn format_event_line(event: &AgentEvent) -> Option<String> {
    match event {
        AgentEvent::ToolCall { name, .. } => Some(format!("→ tool: {}", name)),
        AgentEvent::ToolResult { ok, error, .. } if !ok => Some(format!(
            "× tool failed: {}",
            error.as_deref().unwrap_or("unknown error")
        )),
        AgentEvent::BudgetExhausted { reason } => Some(format!("× budget exhausted: {}", reason)),
        _ => None,
    }
}
