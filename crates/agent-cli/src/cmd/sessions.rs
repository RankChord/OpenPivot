use agent_sessions::SessionStore;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum SessionsCommand {
    /// List session history
    List,
}

pub async fn run(_cmd: SessionsCommand) -> anyhow::Result<()> {
    println!("Session ID\t\t\tTitle\t\t\tLast Active");
    println!("{}", "-".repeat(85));
    let db_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("sessions.db");
    let store = SessionStore::new(&db_path).await?;
    let sessions = store.list_sessions().await?;
    if sessions.is_empty() {
        println!("No sessions found.");
    } else {
        for session in sessions {
            println!(
                "{}\t\t\t{}\t\t\t{}",
                session.id, "Session", session.created_at
            );
        }
    }
    Ok(())
}
