use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use agent_config::AgentConfig;
use agent_cron::default_legacy_jobs_file;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub async fn run() -> anyhow::Result<()> {
    println!("=== Agent Diagnostics ===\n");

    // 1. Environment
    let _rust_env = env::var("RUST_VERSION").unwrap_or_else(|_| "Unknown".to_string());
    println!(
        "Environment: {}",
        if cfg!(target_os = "linux") {
            "Linux"
        } else {
            "Other"
        }
    );

    // 2. Config
    let config_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("config.toml");
    if config_path.exists() {
        println!("Config Path: {:?}", config_path);
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                println!("Config Status: {}", config_status_from_content(&content));
                if content.contains("[model]") {
                    if let Ok(config) = AgentConfig::from_file(&config_path) {
                        print_config_summary(&config);
                    }
                }
            }
            Err(error) => println!(
                "Config Status: {}",
                config_status_from_read_result(Err(error))
            ),
        }
    } else {
        println!("Config Status: MISSING");
    }

    // 3. Api Key
    println!(
        "API Key: {}",
        if env_secret_status(env::var("AGENT_API_KEY")) == "Configured" {
            "Found in Environment"
        } else {
            "Not Found (Check config.toml)"
        }
    );

    // 4. Automation
    let automation_path = automation_db_path();
    println!(
        "Automation DB: {}",
        if automation_path.exists() {
            "Exists"
        } else {
            "Not Initialized"
        }
    );
    println!("Automation DB Path: {}", automation_path.display());
    if automation_path.exists() {
        print_automation_counts(&automation_path).await;
    }
    let legacy_jobs_path = legacy_jobs_file_path();
    println!("Legacy Cron Jobs File: {}", legacy_jobs_path.display());
    println!(
        "Legacy Cron Jobs File Status: {}",
        if legacy_jobs_path.exists() {
            "Exists"
        } else {
            "Missing"
        }
    );
    println!("HTTP Token: {}", http_token_status());
    println!("Cron Dashboard: /cron");
    println!("Task Dashboard: /tasks");

    let sessions_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("sessions.db");
    println!(
        "Sessions DB: {}",
        if sessions_path.exists() {
            "Exists"
        } else {
            "Not Initialized"
        }
    );

    println!(
        "Web Search: {}",
        if env::var("AGENT_WEB_SEARCH_STATIC_RESULTS").is_ok() {
            "Static provider configured"
        } else {
            "Not configured"
        }
    );
    println!(
        "Delegate Task: {}",
        if env::var("AGENT_DELEGATE_LOCAL").ok().as_deref() == Some("1") {
            "Local delegate enabled"
        } else {
            "Local delegate disabled"
        }
    );

    let log_path = crate::logging::default_log_path();
    println!("Log File: {}", log_path.display());
    println!("Log Tail: tail -f {}", log_path.display());

    println!("\nDone.");
    Ok(())
}

fn automation_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("automation.db")
}

fn legacy_jobs_file_path() -> PathBuf {
    default_legacy_jobs_file()
}

fn http_token_status() -> &'static str {
    env_secret_status(env::var("AGENT_HTTP_TOKEN"))
}

fn env_secret_status(value: Result<String, env::VarError>) -> &'static str {
    if value
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
    {
        "Configured"
    } else {
        "Missing"
    }
}

fn config_status_from_content(content: &str) -> &'static str {
    if content.contains("[model]") {
        "OK"
    } else {
        "INVALID (Missing [model])"
    }
}

fn config_status_from_read_result(result: Result<String, std::io::Error>) -> String {
    match result {
        Ok(content) => config_status_from_content(&content).to_string(),
        Err(error) => format!("ERROR ({})", error),
    }
}

fn cron_jobs_count_query() -> &'static str {
    "SELECT COUNT(*) FROM cron_jobs"
}

fn task_count_query(_status: &str) -> &'static str {
    "SELECT COUNT(*) FROM tasks WHERE status = ?"
}

async fn print_automation_counts(db_path: &std::path::Path) {
    match open_read_only_sqlite(db_path).await {
        Ok(pool) => {
            print_count(&pool, "Cron Jobs", cron_jobs_count_query()).await;
            print_task_count(&pool, "Tasks Queued", "queued").await;
            print_task_count(&pool, "Tasks Running", "running").await;
            print_task_count(&pool, "Tasks Failed", "failed").await;
            print_task_count(&pool, "Tasks Lost", "lost").await;
        }
        Err(error) => println!("Automation Counts Error: {}", error),
    }
}

async fn open_read_only_sqlite(db_path: &std::path::Path) -> Result<sqlx::SqlitePool, sqlx::Error> {
    let db_url = format!("sqlite://{}", db_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(false)
        .read_only(true);
    SqlitePoolOptions::new().connect_with(options).await
}

async fn print_count(pool: &sqlx::SqlitePool, label: &str, query: &str) {
    match sqlx::query_scalar::<_, i64>(query).fetch_one(pool).await {
        Ok(count) => println!("{}: {}", label, count),
        Err(error) => println!("{} Error: {}", label, error),
    }
}

async fn print_task_count(pool: &sqlx::SqlitePool, label: &str, status: &str) {
    match sqlx::query_scalar::<_, i64>(task_count_query(status))
        .bind(status)
        .fetch_one(pool)
        .await
    {
        Ok(count) => println!("{}: {}", label, count),
        Err(error) => println!("{} Error: {}", label, error),
    }
}

fn print_config_summary(config: &AgentConfig) {
    println!("Model Provider: {}", config.model.provider);
    println!(
        "Provider Status: {}",
        match config.model.provider.as_str() {
            "openai" => "Supported",
            "anthropic" | "google" => "Unsupported in current runtime",
            _ => "Treated as OpenAI-compatible",
        }
    );
    println!("Permission Read-only: {}", config.permissions.read_only);
    println!("Ask Tools: {}", config.permissions.ask_tools.join(", "));
    println!(
        "Destructive Tools: {}",
        if config.permissions.allow_destructive_tools {
            "Allowed"
        } else {
            "Denied by default"
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_db_path_uses_agent_home_suffix() {
        assert!(automation_db_path().ends_with(PathBuf::from(".agent").join("automation.db")));
    }

    #[test]
    fn legacy_jobs_file_path_uses_agent_home_suffix() {
        assert!(
            legacy_jobs_file_path()
                .ends_with(PathBuf::from(".agent").join("cron").join("jobs.json"))
        );
    }

    #[test]
    fn http_token_status_matches_current_environment() {
        let expected = if env::var("AGENT_HTTP_TOKEN").is_ok() {
            "Configured"
        } else {
            "Missing"
        };

        assert_eq!(http_token_status(), expected);
    }

    #[test]
    fn env_secret_status_treats_empty_values_as_missing() {
        assert_eq!(env_secret_status(Ok(String::new())), "Missing");
        assert_eq!(env_secret_status(Ok("   ".to_string())), "Missing");
        assert_eq!(env_secret_status(Ok("secret".to_string())), "Configured");
        assert_eq!(env_secret_status(Err(env::VarError::NotPresent)), "Missing");
    }

    #[test]
    fn config_status_reports_read_errors_without_failing() {
        let error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");

        assert_eq!(config_status_from_read_result(Err(error)), "ERROR (denied)");
    }

    #[test]
    fn automation_count_queries_are_read_only_selects() {
        assert_eq!(cron_jobs_count_query(), "SELECT COUNT(*) FROM cron_jobs");
        assert_eq!(
            task_count_query("queued"),
            "SELECT COUNT(*) FROM tasks WHERE status = ?"
        );
    }
}
