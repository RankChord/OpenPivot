//! 启动入口只组装配置、存储和路由。具体规则请看各业务模块。
use agent_space_demo::{config::Config, http, store::Store};
use serde_json::Value;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env().map_err(std::io::Error::other)?;
    let store = Store::open(&config.database)?;
    store.validate_admin_credential(&config.admin_token)?;
    // 可选迁移入口：只给已有角色替换凭证；Agent 的创建由管理端负责。
    if let Some(path) = &config.agent_token_import {
        let values: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        let object = values
            .as_object()
            .ok_or("AGENT_SPACE_TOKENS must contain a JSON object")?;
        for (agent_id, token) in object {
            let token = token.as_str().ok_or("Agent tokens must be strings")?;
            if token == config.admin_token {
                return Err(
                    "Agent credentials must differ from the administrator credential".into(),
                );
            }
            if !config.bind.ip().is_loopback() && token.len() < 32 {
                return Err(
                    "public listeners require imported Agent tokens of at least 32 characters"
                        .into(),
                );
            }
            store.import_agent_token(agent_id, token)?;
        }
    }
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    eprintln!(
        "Agent Space: http://{} ; MCP: /mcp ; database: {}",
        listener.local_addr()?,
        config.database.display()
    );
    axum::serve(listener, http::router(store, config))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("agent-space-demo: {error}");
        std::process::exit(1);
    }
}
