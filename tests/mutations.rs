use skillsdash::market::{SkillContent, SkillFile};
use skillsdash::model::registry::Root;
use skillsdash::model::{Provider, Registry, Scope};
use skillsdash::ui::actions::{self, ShareMethod};
use std::fs;
use std::path::{Path, PathBuf};

struct Fixture {
    base: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let base = std::env::temp_dir().join(format!("skillsdash-test-{tag}"));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        Fixture { base }
    }

    fn registry(&self) -> Registry {
        let roots = vec![
            Root {
                provider: Provider::Claude,
                scope: Scope::Global,
                skills_dir: self.base.join("claude/skills"),
            },
            Root {
                provider: Provider::Agents,
                scope: Scope::Global,
                skills_dir: self.base.join("agents/skills"),
            },
            Root {
                provider: Provider::Claude,
                scope: Scope::Project,
                skills_dir: self.base.join("proj/.claude/skills"),
            },
            Root {
                provider: Provider::Agents,
                scope: Scope::Project,
                skills_dir: self.base.join("proj/.agents/skills"),
            },
        ];
        Registry::from_roots(roots)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[test]
fn create_then_appears_in_registry() {
    let fx = Fixture::new("create");
    let reg = fx.registry();
    let dir = actions::create_skill(&reg, "my-skill", "does a thing", Provider::Claude, Scope::Global).unwrap();
    assert!(dir.join("SKILL.md").exists());

    let reg2 = fx.registry();
    let skill = reg2.skills.iter().find(|s| s.name == "my-skill").expect("skill present");
    assert_eq!(skill.description, "does a thing");
    assert!(skill.has(Provider::Claude, Scope::Global));
    assert!(!skill.has(Provider::Agents, Scope::Global));
}

#[test]
fn create_rejects_duplicate_and_bad_name() {
    let fx = Fixture::new("dupe");
    let reg = fx.registry();
    actions::create_skill(&reg, "dup", "d", Provider::Claude, Scope::Global).unwrap();
    let reg = fx.registry();
    assert!(actions::create_skill(&reg, "dup", "d", Provider::Claude, Scope::Global).is_err());
    assert!(actions::create_skill(&reg, "bad name!", "d", Provider::Claude, Scope::Global).is_err());
    assert!(actions::create_skill(&reg, "  ", "d", Provider::Claude, Scope::Global).is_err());
}

#[test]
fn share_by_copy_creates_independent_copy() {
    let fx = Fixture::new("copy");
    let reg = fx.registry();
    let src = actions::create_skill(&reg, "shared", "orig", Provider::Claude, Scope::Global).unwrap();

    let reg = fx.registry();
    actions::share_skill(&reg, &src, Provider::Agents, Scope::Global, "shared", ShareMethod::Copy).unwrap();

    let reg = fx.registry();
    let skill = reg.skills.iter().find(|s| s.name == "shared").unwrap();
    assert!(skill.has(Provider::Claude, Scope::Global));
    assert!(skill.has(Provider::Agents, Scope::Global));
    let agents_inst = skill.instance(Provider::Agents, Scope::Global).unwrap();
    assert!(!agents_inst.is_symlink);

    actions::save_body(&src.join("SKILL.md"), "changed body").unwrap();
    let agents_md = agents_inst.skill_md.clone();
    let agents_content = fs::read_to_string(&agents_md).unwrap();
    assert!(!agents_content.contains("changed body"));
}

#[cfg(unix)]
#[test]
fn share_by_symlink_tracks_source() {
    let fx = Fixture::new("symlink");
    let reg = fx.registry();
    let src = actions::create_skill(&reg, "linked", "orig", Provider::Claude, Scope::Global).unwrap();

    let reg = fx.registry();
    actions::share_skill(&reg, &src, Provider::Agents, Scope::Global, "linked", ShareMethod::Symlink).unwrap();

    let reg = fx.registry();
    let skill = reg.skills.iter().find(|s| s.name == "linked").unwrap();
    let agents_inst = skill.instance(Provider::Agents, Scope::Global).unwrap();
    assert!(agents_inst.is_symlink);

    actions::save_body(&src.join("SKILL.md"), "propagated").unwrap();
    let via_link = fs::read_to_string(&agents_inst.skill_md).unwrap();
    assert!(via_link.contains("propagated"));
}

#[test]
fn save_frontmatter_preserves_extra_keys() {
    let fx = Fixture::new("frontmatter");
    let reg = fx.registry();
    let dir = actions::create_skill(&reg, "fm", "d", Provider::Claude, Scope::Global).unwrap();
    let md = dir.join("SKILL.md");

    let original = fs::read_to_string(&md).unwrap();
    let injected = original.replacen("---\n", "---\nlicense: MIT\n", 1);
    fs::write(&md, injected).unwrap();

    actions::save_frontmatter(&md, "fm-renamed", "new description").unwrap();
    let updated = fs::read_to_string(&md).unwrap();
    assert!(updated.contains("name: fm-renamed"));
    assert!(updated.contains("new description"));
    assert!(updated.contains("license: MIT"));
}

#[test]
fn delete_removes_only_targeted_instances() {
    let fx = Fixture::new("delete");
    let reg = fx.registry();
    let src = actions::create_skill(&reg, "victim", "d", Provider::Claude, Scope::Global).unwrap();
    let reg = fx.registry();
    actions::share_skill(&reg, &src, Provider::Agents, Scope::Global, "victim", ShareMethod::Copy).unwrap();

    let reg = fx.registry();
    let skill = reg.skills.iter().find(|s| s.name == "victim").unwrap();
    let agents_dir = skill.instance(Provider::Agents, Scope::Global).unwrap().dir.clone();
    actions::delete_instances(&[agents_dir]).unwrap();

    let reg = fx.registry();
    let skill = reg.skills.iter().find(|s| s.name == "victim").unwrap();
    assert!(skill.has(Provider::Claude, Scope::Global));
    assert!(!skill.has(Provider::Agents, Scope::Global));
}

#[test]
fn project_scope_is_discovered() {
    let fx = Fixture::new("project");
    let reg = fx.registry();
    let dir = actions::create_skill(&reg, "proj-skill", "d", Provider::Claude, Scope::Project).unwrap();
    assert!(dir.join("SKILL.md").exists());
    assert!(dir_contains(&fx.base.join("proj/.claude/skills"), "proj-skill"));

    let reg = fx.registry();
    let skill = reg.skills.iter().find(|s| s.name == "proj-skill").unwrap();
    assert!(skill.has(Provider::Claude, Scope::Project));
}

fn sample_content() -> SkillContent {
    SkillContent {
        files: vec![
            SkillFile {
                relative_path: "SKILL.md".to_string(),
                bytes: b"---\nname: netskill\ndescription: from the market\n---\nbody\n".to_vec(),
            },
            SkillFile {
                relative_path: "LICENSE.txt".to_string(),
                bytes: b"MIT".to_vec(),
            },
        ],
    }
}

#[test]
fn install_writes_all_files() {
    let fx = Fixture::new("install");
    let reg = fx.registry();
    let content = sample_content();
    let dir = actions::install_skill(&reg, "netskill", &content, Provider::Claude, Scope::Global, false).unwrap();
    assert!(dir.join("SKILL.md").exists());
    assert!(dir.join("LICENSE.txt").exists());

    let reg = fx.registry();
    let skill = reg.skills.iter().find(|s| s.name == "netskill").unwrap();
    assert_eq!(skill.description, "from the market");
    assert!(skill.has(Provider::Claude, Scope::Global));
}

#[test]
fn install_refuses_clobber_without_overwrite() {
    let fx = Fixture::new("install-clobber");
    let reg = fx.registry();
    let content = sample_content();
    actions::install_skill(&reg, "netskill", &content, Provider::Claude, Scope::Global, false).unwrap();
    let reg = fx.registry();
    assert!(actions::install_skill(&reg, "netskill", &content, Provider::Claude, Scope::Global, false).is_err());
    assert!(actions::install_skill(&reg, "netskill", &content, Provider::Claude, Scope::Global, true).is_ok());
}

#[test]
fn install_rejects_path_traversal() {
    let fx = Fixture::new("install-traversal");
    let reg = fx.registry();
    let evil = SkillContent {
        files: vec![
            SkillFile { relative_path: "SKILL.md".to_string(), bytes: b"---\nname: x\ndescription: d\n---\n".to_vec() },
            SkillFile { relative_path: "../../escape.txt".to_string(), bytes: b"pwned".to_vec() },
        ],
    };
    assert!(actions::install_skill(&reg, "x", &evil, Provider::Claude, Scope::Global, false).is_err());
    assert!(!fx.base.join("escape.txt").exists());
}

fn dir_contains(dir: &Path, name: &str) -> bool {
    fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).any(|e| e.file_name().to_string_lossy() == name))
        .unwrap_or(false)
}
