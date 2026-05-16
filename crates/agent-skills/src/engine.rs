use crate::skill::{SkillDefinition, SkillError};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct SkillEngine {
    skills: HashMap<String, SkillDefinition>,
    config_vars: HashMap<String, String>,
    disabled_skills: Vec<String>,
}

impl SkillEngine {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            config_vars: HashMap::new(),
            disabled_skills: Vec::new(),
        }
    }

    pub fn disable_skill(&mut self, name: &str) {
        self.disabled_skills.push(name.into());
    }

    /// Scan directories for SKILL.md files
    pub fn scan_skills(&mut self, dirs: &[PathBuf]) -> Result<(), SkillError> {
        for dir in dirs {
            if !dir.exists() { continue; }
            for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_name() == "SKILL.md" {
                    match SkillDefinition::from_file(entry.path()) {
                        Ok(skill) => {
                            if skill.is_loadable(&[], &self.disabled_skills) {
                                self.skills.insert(skill.name.clone(), skill);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse SKILL.md at {:?}: {}", entry.path(), e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_skill(&self, name: &str) -> Option<&SkillDefinition> {
        self.skills.get(name)
    }

    pub fn list_skills(&self) -> Vec<&SkillDefinition> {
        self.skills.values().collect()
    }

    pub fn build_prompt(&self, active_skills: &[String]) -> String {
        let mut prompt = String::from("# Active Skills\n\n");
        for name in active_skills {
            if let Some(skill) = self.skills.get(name) {
                prompt.push_str(&skill.system_prompt());
            }
        }
        prompt
    }
}

impl Default for SkillEngine {
    fn default() -> Self { Self::new() }
}
