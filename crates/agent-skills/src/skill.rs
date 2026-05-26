use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SkillMetadata {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct SkillDefinition {
    pub metadata: SkillMetadata,
    pub body: String,
    pub path: PathBuf,
}

impl SkillDefinition {
    pub fn from_file(path: &Path) -> Option<Self> {
        if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
            return None;
        }

        let content = fs::read_to_string(path).ok()?;

        let (meta, body) = parse_frontmatter(&content);

        Some(Self {
            metadata: meta,
            body,
            path: path.to_path_buf(),
        })
    }

    /// 生成用于注入 System Prompt 的指令
    pub fn to_prompt_snippet(&self) -> String {
        format!(
            "### Skill: {}\nDescription: {}\nUsage Instructions:\n{}\n",
            self.metadata.name, self.metadata.description, self.body
        )
    }
}

fn parse_frontmatter(content: &str) -> (SkillMetadata, String) {
    if content.starts_with("---") {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() >= 3 {
            let yaml = parts[1];
            let meta = serde_yaml::from_str(yaml).unwrap_or_default();
            return (meta, parts[2].trim().to_string());
        }
    }
    (SkillMetadata::default(), content.to_string())
}
