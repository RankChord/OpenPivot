use agent_skills::skill::SkillDefinition;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_parse_skill() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, "---").unwrap();
    writeln!(file, "name: Test Skill").unwrap();
    writeln!(file, "description: A test skill.").unwrap();
    writeln!(file, "version: 1.0.0").unwrap();
    writeln!(file, "author: Test Author").unwrap();
    writeln!(file, "platforms: [linux, macos]").unwrap();
    writeln!(file, "---").unwrap();
    writeln!(file, "# Test Skill Skill").unwrap();
    writeln!(file, "This is the skill body.").unwrap();

    let skill = SkillDefinition::from_file(&path).unwrap();
    assert_eq!(skill.metadata.name, "Test Skill");
    assert_eq!(skill.metadata.description, "A test skill.");
    assert!(skill.body.contains("This is the skill body."));
}
