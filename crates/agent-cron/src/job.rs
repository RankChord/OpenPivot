use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: Schedule,
    pub prompt: String,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    /// Model override for this job
    pub model_override: Option<String>,
    /// Skills to load for this job
    pub skills: Vec<String>,
    /// Whether this runs without an agent (script only)
    pub no_agent: bool,
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
            Schedule::Every(_) | Schedule::Cron(_) => {
                Some(after + chrono::Duration::hours(1))
            }
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
}
