use crate::skill::SkillDefinition;
use std::path::PathBuf;

pub struct SkillEngine {
    pub skills: Vec<SkillDefinition>,
}

impl SkillEngine {
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }

    pub async fn discover(&mut self, dirs: &[PathBuf]) {
        for dir in dirs {
            if !dir.exists() {
                continue;
            }

            for entry in walkdir::WalkDir::new(dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if let Some(skill) = SkillDefinition::from_file(entry.path()) {
                    self.skills.push(skill);
                }
            }
        }
    }

    /// 将所有已发现技能整合为 System Prompt 片段
    pub fn build_system_prompt_snippet(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }

        let mut prompt = String::from("You have access to the following specialized skills:\n\n");
        for skill in &self.skills {
            prompt.push_str(&skill.to_prompt_snippet());
            prompt.push_str("\n\n");
        }
        prompt.push_str("--- End of Skills ---\n");
        prompt
    }
}
