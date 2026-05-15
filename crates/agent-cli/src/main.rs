use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agent", version, about = "AI Agent Framework")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive agent
    Run {
        /// First prompt message
        #[arg()]
        prompt: Option<String>,
    },
    /// Show configuration
    Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let env_filter = if cli.verbose {
        "agent_cli=debug,agent_core=debug,agent_tools=debug,agent_llm=debug"
    } else {
        "agent_cli=info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    let config_path = cli.config.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".agent")
            .join("config.toml")
    });

    match cli.command {
        Some(Commands::Run { prompt }) => {
            run_agent(config_path, prompt).await?;
        }
        Some(Commands::Config) => {
            show_config(&config_path)?;
        }
        None => {
            run_agent(config_path, None).await?;
        }
    }

    Ok(())
}

async fn run_agent(config_path: PathBuf, prompt: Option<String>) -> anyhow::Result<()> {
    println!("Loading config from {:?}", config_path);

    let config = if config_path.exists() {
        agent_config::AgentConfig::from_file(&config_path)?
    } else {
        println!("No config found, using defaults");
        agent_config::AgentConfig::default()
    };

    let prompt = prompt.unwrap_or_else(|| {
        println!("Enter your prompt (or /help for commands):");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        input.trim().to_string()
    });

    println!("Agent: {:?}", prompt);
    println!("Model: {} ({})", config.model.provider, config.model.name);
    println!("Max iterations: {}", config.agent.max_iterations);

    // For Phase 1, just show that the agent started
    // Full tool loop requires registered tools
    tracing::info!("Agent initialized");

    Ok(())
}

fn show_config(config_path: &PathBuf) -> anyhow::Result<()> {
    if config_path.exists() {
        let config = agent_config::AgentConfig::from_file(config_path)?;
        println!("Config loaded from {:?}", config_path);
        println!("Model: {} - {}", config.model.provider, config.model.name);
        println!("Base URL: {:?}", config.model.base_url);
    } else {
        println!("Config file not found at {:?}", config_path);
        println!("Using defaults:");
        let config = agent_config::AgentConfig::default();
        println!("  Model: {} - {}", config.model.provider, config.model.name);
    }
    Ok(())
}
