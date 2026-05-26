use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub struct LoggingGuard {
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

pub fn default_log_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("logs")
        .join("agent.log")
}

pub fn append_log_line(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", line)
}

pub fn init_file_logging(env_filter: &str, also_stdout: bool) -> anyhow::Result<LoggingGuard> {
    let log_path = default_log_path();
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file_appender = tracing_appender::rolling::never(
        log_path.parent().unwrap_or_else(|| Path::new(".")),
        log_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("agent.log"),
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_new(env_filter).unwrap_or_else(|_| EnvFilter::new("agent=info"));

    if also_stdout {
        let file_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(non_blocking);
        let stdout_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr.with_max_level(tracing::Level::INFO));
        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .with(stdout_layer)
            .try_init()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(non_blocking)
            .try_init()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    }

    install_panic_hook();
    Ok(LoggingGuard { _guard: guard })
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = append_log_line(&default_log_path(), &format!("panic: {}", panic_info));
        previous(panic_info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_log_line_and_creates_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logs").join("agent.log");

        append_log_line(&path, "hello log").unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("hello log"));
    }
}
