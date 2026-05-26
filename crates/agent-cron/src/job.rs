use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schedule: Schedule,
    pub payload_kind: PayloadKind,
    pub payload_json: serde_json::Value,
    pub enabled: bool,
    pub missed_run_policy: MissedRunPolicy,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCronJob {
    pub name: String,
    pub description: String,
    pub schedule: Schedule,
    pub payload_kind: PayloadKind,
    pub payload_json: serde_json::Value,
    pub enabled: bool,
    pub missed_run_policy: MissedRunPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadKind {
    AgentPrompt,
    ShellCommand,
}

impl PayloadKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AgentPrompt => "agent_prompt",
            Self::ShellCommand => "shell_command",
        }
    }
}

impl TryFrom<&str> for PayloadKind {
    type Error = ScheduleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "agent_prompt" => Ok(Self::AgentPrompt),
            "shell_command" => Ok(Self::ShellCommand),
            other => Err(ScheduleError::InvalidPayloadKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicy {
    Skip,
    RunOnce,
    CatchUp,
}

impl MissedRunPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Skip => "skip",
            Self::RunOnce => "run_once",
            Self::CatchUp => "catch_up",
        }
    }
}

impl TryFrom<&str> for MissedRunPolicy {
    type Error = ScheduleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "skip" => Ok(Self::Skip),
            "run_once" => Ok(Self::RunOnce),
            "catch_up" => Ok(Self::CatchUp),
            other => Err(ScheduleError::InvalidMissedRunPolicy(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Schedule {
    /// Duration: "30m", "2h", "1d"
    Duration(std::time::Duration),
    /// Every: "every 2h", "every monday 9am"
    Every(String),
    /// Standard 5-field cron: "0 9 * * *"
    Cron(String),
    /// One-shot ISO timestamp: "2026-06-01T09:00:00Z"
    Once(DateTime<Utc>),
}

impl Schedule {
    pub fn parse(input: &str) -> Result<Self, ScheduleError> {
        if let Ok(d) = parse_duration(input) {
            return Ok(Schedule::Duration(d));
        }
        if input.starts_with("every ") {
            return Ok(Schedule::Every(input.into()));
        }
        if is_cron_expression(input) {
            return Ok(Schedule::Cron(input.into()));
        }
        if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
            return Ok(Schedule::Once(dt.with_timezone(&Utc)));
        }
        Err(ScheduleError::InvalidFormat(input.into()))
    }

    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Schedule::Duration(d) => Some(after + chrono::Duration::from_std(*d).ok()?),
            Schedule::Once(dt) => {
                if *dt > after {
                    Some(*dt)
                } else {
                    None
                }
            }
            Schedule::Every(value) => parse_every_interval(value)
                .and_then(|duration| chrono::Duration::from_std(duration).ok())
                .map(|duration| after + duration)
                .or_else(|| Some(after + chrono::Duration::hours(1))),
            Schedule::Cron(_) => Some(after + chrono::Duration::hours(1)),
        }
    }
}

fn parse_duration(input: &str) -> Result<std::time::Duration, ()> {
    let input = input.trim();
    if input.is_empty() {
        return Err(());
    }
    let (num_str, suffix) = input.split_at(input.len() - 1);
    let num: u64 = num_str.parse().map_err(|_| ())?;
    match suffix {
        "s" => Ok(std::time::Duration::from_secs(num)),
        "m" => Ok(std::time::Duration::from_secs(num * 60)),
        "h" => Ok(std::time::Duration::from_secs(num * 3600)),
        "d" => Ok(std::time::Duration::from_secs(num * 86400)),
        _ => Err(()),
    }
}

fn parse_every_interval(input: &str) -> Option<std::time::Duration> {
    input
        .strip_prefix("every ")
        .and_then(|rest| parse_duration(rest).ok())
}

fn is_cron_expression(input: &str) -> bool {
    let parts: Vec<&str> = input.split_whitespace().collect();
    parts.len() == 5
        && parts.iter().all(|p| {
            p.chars()
                .all(|c| c.is_ascii_digit() || c == '*' || c == ',' || c == '/' || c == '-')
        })
}

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("invalid schedule format: {0}")]
    InvalidFormat(String),
    #[error("invalid payload kind: {0}")]
    InvalidPayloadKind(String),
    #[error("invalid missed-run policy: {0}")]
    InvalidMissedRunPolicy(String),
}
