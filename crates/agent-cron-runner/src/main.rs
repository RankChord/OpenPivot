use agent_cron::{CronScheduler, CronStore};
use agent_kanban::KanbanStore;

fn automation_db_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("automation.db")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let db_path = automation_db_path();
    let cron_store = CronStore::new(&db_path).await?;
    let kanban_store = KanbanStore::new(&db_path).await?;
    let scheduler = CronScheduler::new(cron_store, kanban_store);

    tracing::info!(path = %db_path.display(), "cron runner started, polling every 30s");
    scheduler.tick_loop(30).await;
    Ok(())
}
