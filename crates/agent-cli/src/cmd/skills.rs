use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum SkillsCommand {
    /// List available skills
    List,
    /// View skill details
    View { name: String },
}

pub fn list_skills() -> Vec<(String, String)> {
    let paths = ["skills", "~/.agent/skills"];

    let skills = vec![];
    for p in paths {
        let path = PathBuf::from(p);
        if path.is_dir() {
            // Walk directories to find SKILL.md
        }
    }
    skills
}

pub fn run(cmd: SkillsCommand) -> anyhow::Result<()> {
    match cmd {
        SkillsCommand::List => {
            let skills = list_skills();
            if skills.is_empty() {
                println!("No skills found.");
            } else {
                for (name, desc) in skills {
                    println!("• {}\n  {}", name, desc);
                }
            }
        }
        SkillsCommand::View { name } => {
            // Implementation details...
            println!("Skill: {}", name);
        }
    }
    Ok(())
}
