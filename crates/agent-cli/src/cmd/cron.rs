use agent_cron::{CronStore, default_legacy_jobs_file};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum CronCommand {
    /// 查看所有定时任务
    List,
    /// 通过 ID 删除任务
    Delete { id: String },
    /// 暂停任务
    Pause { id: String },
    /// 恢复任务
    Resume { id: String },
}

fn automation_db_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("automation.db")
}

pub async fn run(cmd: CronCommand) -> anyhow::Result<()> {
    let store = CronStore::new(&automation_db_file()).await?;
    store
        .import_legacy_jobs_file(&default_legacy_jobs_file())
        .await?;

    match cmd {
        CronCommand::List => {
            let jobs = store.list_jobs().await?;
            if jobs.is_empty() {
                println!("No scheduled tasks found.");
                return Ok(());
            }

            println!(
                "{:<38} | {:<20} | {:<8} | {:<25} | {}",
                "ID", "Name", "Enabled", "Next Run", "Description"
            );
            println!("{}", "-".repeat(120));

            for job in jobs {
                println!(
                    "{:<38} | {:<20} | {:<8} | {:<25} | {}",
                    job.id,
                    job.name.chars().take(20).collect::<String>(),
                    job.enabled,
                    job.next_run_at
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| "-".into()),
                    job.description.chars().take(30).collect::<String>()
                );
            }
        }
        CronCommand::Delete { id } => {
            if store.delete_job(&id).await? {
                println!("Deleted job: {}", id);
            } else {
                println!("Job '{}' not found.", id);
            }
        }
        CronCommand::Pause { id } => {
            if store.load_job(&id).await?.is_some() {
                store.set_enabled(&id, false).await?;
                println!("Paused job: {}", id);
            } else {
                println!("Job '{}' not found.", id);
            }
        }
        CronCommand::Resume { id } => {
            if store.load_job(&id).await?.is_some() {
                store.set_enabled(&id, true).await?;
                println!("Resumed job: {}", id);
            } else {
                println!("Job '{}' not found.", id);
            }
        }
    }
    Ok(())
}
