//! Curator — auto-archives stale agent-created skills

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub use_count: u64,
    pub last_used: chrono::DateTime<chrono::Utc>,
    pub created_by: String, // "agent" or "user"
    pub is_pinned: bool,
}

pub struct Curator {
    usage_log: std::collections::HashMap<String, UsageStats>,
    #[allow(dead_code)]
    archive_dir: PathBuf,
    #[allow(dead_code)]
    skills_dir: PathBuf,
    pub stale_after_days: u32,
    pub archive_after_days: u32,
}

impl Curator {
    pub fn new(skills_dir: &PathBuf, archive_dir: &PathBuf) -> Self {
        Self {
            usage_log: std::collections::HashMap::new(),
            archive_dir: archive_dir.clone(),
            skills_dir: skills_dir.clone(),
            stale_after_days: 30,
            archive_after_days: 90,
        }
    }

    pub fn record_use(&mut self, skill_name: &str) {
        let stats = self
            .usage_log
            .entry(skill_name.into())
            .or_insert_with(|| UsageStats {
                use_count: 0,
                last_used: chrono::Utc::now(),
                created_by: "user".into(),
                is_pinned: false,
            });
        stats.use_count += 1;
        stats.last_used = chrono::Utc::now();
    }

    pub fn run(&self) -> Result<Vec<String>, std::io::Error> {
        // In real impl, would move stale skills to archive dir
        Ok(vec![])
    }

    #[allow(dead_code)]
    fn should_archive(&self, skill_name: &str) -> bool {
        if let Some(stats) = self.usage_log.get(skill_name) {
            if stats.is_pinned {
                return false;
            }
            let days = (chrono::Utc::now() - stats.last_used).num_days() as u64;
            return days >= self.archive_after_days as u64 && stats.created_by == "agent";
        }
        false
    }
}
