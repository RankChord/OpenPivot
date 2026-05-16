use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SKILL.md frontmatter metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub related_skills: Option<Vec<String>>,
    pub config: Option<Vec<SkillConfigVar>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfigVar {
    pub key: String,
    pub description: String,
}

/// Parsed SKILL.md
#[derive(Debug, Clone)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub platforms: Vec<String>,
    pub metadata: Option<SkillMetadata>,
    pub body: String,
    pub dir: PathBuf,
}

impl SkillDefinition {
    /// Parse a SKILL.md file
    pub fn from_file(path: &std::path::Path) -> Result<Self, SkillError> {
        let content = std::fs::read_to_string(path)?;

        // Split frontmatter from body
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err(SkillError::InvalidFormat(path.to_path_buf()));
        }

        let frontmatter = parts[1];
        let body = parts[2].trim().to_string();

        // Parse frontmatter (simple YAML subset)
        let (name, description, version, author, platforms, metadata) = parse_frontmatter(frontmatter)
            .map_err(|e| SkillError::ParseError(e))?;

        Ok(Self {
            name,
            description,
            version,
            author,
            platforms,
            metadata,
            body,
            dir: path.parent().unwrap_or(std::path::Path::new("")).to_path_buf(),
        })
    }

    pub fn platforms_match(&self) -> bool {
        if self.platforms.is_empty() { return true; }
        let current = std::env::consts::OS;
        self.platforms.iter().any(|p| {
            match p.as_str() {
                "linux" => current == "linux",
                "macos" => current == "macos",
                "windows" => current == "windows",
                _ => true,
            }
        })
    }

    pub fn is_loadable(&self, _required_tools: &[String], disabled_skills: &[String]) -> bool {
        if !self.platforms_match() { return false; }
        if disabled_skills.iter().any(|d| d == &self.name) { return false; }

        if let Some(meta) = &self.metadata {
            if let Some(reqs) = &meta.config {
                // All required config vars must be present (simplified check)
                if !reqs.is_empty() {
                    // In real impl, check config values
                }
            }
        }

        true
    }

    pub fn system_prompt(&self) -> String {
        format!("# {}\n\n{}\n\n{}\n", self.name, self.description, self.body)
    }
}

fn parse_frontmatter(fm: &str) -> Result<(String, String, String, String, Vec<String>, Option<SkillMetadata>), String> {
    let mut name = String::new();
    let mut description = String::new();
    let mut version = "1.0.0".into();
    let mut author = String::new();
    let mut platforms = Vec::new();

    for line in fm.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name:") {
            name = rest.trim().trim_matches('"').trim_matches('\'').into();
        } else if let Some(rest) = line.strip_prefix("description:") {
            description = rest.trim().trim_matches('"').trim_matches('\'').into();
        } else if let Some(rest) = line.strip_prefix("version:") {
            version = rest.trim().trim_matches('"').trim_matches('\'').into();
        } else if let Some(rest) = line.strip_prefix("author:") {
            author = rest.trim().trim_matches('"').trim_matches('\'').into();
        } else if let Some(rest) = line.strip_prefix("platforms:") {
            // Parse YAML array: [linux, macos]
            let arr = rest.trim().trim_start_matches('[').trim_end_matches(']');
            platforms = arr.split(',').map(|s| s.trim().trim_matches('"').to_string()).filter(|s| !s.is_empty()).collect();
        }
    }

    if name.is_empty() {
        return Err("Missing required field: name".into());
    }

    Ok((name, description, version, author, platforms, None))
}

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid SKILL.md format: {0}")]
    InvalidFormat(std::path::PathBuf),
    #[error("parse error: {0}")]
    ParseError(String),
}
