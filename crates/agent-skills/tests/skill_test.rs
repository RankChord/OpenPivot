use agent_skills::skill::SkillDefinition;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_parse_skill() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "---").unwrap();
    writeln!(file, "name: Test Skill").unwrap();
    writeln!(file, "description: A test skill.").unwrap();
    writeln!(file, "version: 1.0.0").unwrap();
    writeln!(file, "author: Test Author").unwrap();
    writeln!(file, "platforms: [linux, macos]").unwrap();
    writeln!(file, "---").unwrap();
    writeln!(file, "# Test Skill Skill").unwrap();
    writeln!(file, "This is the skill body.").unwrap();

    let skill = SkillDefinition::from_file(file.path()).unwrap();
    assert_eq!(skill.name, "Test Skill");
    assert_eq!(skill.description, "A test skill.");
    assert!(skill.platforms_match());
}
