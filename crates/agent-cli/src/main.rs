use agent_cli::cmd;
use agent_cli::logging;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agent", version, about = "AI Agent Framework")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(long, global = true)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start background WebSocket/HTTP gateway server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "9093")]
        port: u16,
        /// Bind address
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Open full-screen TUI interface
    Tui,

    /// Execute a one-shot prompt (can pipe via stdin)
    Run {
        /// Message to send
        #[arg(value_name = "MESSAGE", trailing_var_arg = true)]
        message: Vec<String>,

        /// Continue the last active session
        #[arg(short, long)]
        continue_session: bool,

        /// Resume specific session by ID
        #[arg(long)]
        session: Option<String>,

        /// Model format: provider/model
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Interactive chat with system prompt history
    Chat {
        /// Model format: provider/model
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Manage scheduled cron tasks
    #[command(subcommand)]
    Cron(crate::cmd::cron::CronCommand),

    /// List, restore, or delete sessions
    #[command(subcommand)]
    Sessions(crate::cmd::sessions::SessionsCommand),

    /// Show registered tools
    Tools,

    /// List available skills
    #[command(subcommand)]
    Skills(crate::cmd::skills::SkillsCommand),

    /// Diagnose environment and configuration
    Doctor,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let env_filter = if cli.verbose {
        "agent=debug,agent_core=debug,agent_tools=debug,agent_llm=debug"
    } else {
        "agent=info"
    };

    let _logging_guard =
        logging::init_file_logging(env_filter, !matches!(cli.command, Commands::Tui))?;

    match cli.command {
        Commands::Tui => agent_tui::run()?,
        Commands::Serve { port, host } => cmd::serve::run(port, &host, cli.config).await?,
        Commands::Run {
            message,
            continue_session,
            session,
            model,
        } => {
            cmd::run::run(cli.config, message, continue_session, session, model).await?;
        }
        Commands::Chat { model } => cmd::chat::run(cli.config, model).await?,
        Commands::Cron(sub) => cmd::cron::run(sub).await?,
        Commands::Sessions(sub) => cmd::sessions::run(sub).await?,
        Commands::Tools => cmd::tools::run()?,
        Commands::Skills(cmd) => cmd::skills::run(cmd)?,
        Commands::Doctor => cmd::doctor::run().await?,
    }

    Ok(())
}
