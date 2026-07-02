use skillsdash::market::{SkillContent, SkillFile};
use skillsdash::model::harness::{HarnessKind, HarnessRegistry};
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
    let dir = actions::create_skill(
        &reg,
        "my-skill",
        "does a thing",
        Provider::Claude,
        Scope::Global,
    )
    .unwrap();
    assert!(dir.join("SKILL.md").exists());

    let reg2 = fx.registry();
    let skill = reg2
        .skills
        .iter()
        .find(|s| s.name == "my-skill")
        .expect("skill present");
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
    assert!(
        actions::create_skill(&reg, "bad name!", "d", Provider::Claude, Scope::Global).is_err()
    );
    assert!(actions::create_skill(&reg, "  ", "d", Provider::Claude, Scope::Global).is_err());
}

#[test]
fn share_by_copy_creates_independent_copy() {
    let fx = Fixture::new("copy");
    let reg = fx.registry();
    let src =
        actions::create_skill(&reg, "shared", "orig", Provider::Claude, Scope::Global).unwrap();

    let reg = fx.registry();
    actions::share_skill(
        &reg,
        &src,
        Provider::Agents,
        Scope::Global,
        "shared",
        ShareMethod::Copy,
    )
    .unwrap();

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
    let src =
        actions::create_skill(&reg, "linked", "orig", Provider::Claude, Scope::Global).unwrap();

    let reg = fx.registry();
    actions::share_skill(
        &reg,
        &src,
        Provider::Agents,
        Scope::Global,
        "linked",
        ShareMethod::Symlink,
    )
    .unwrap();

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
    actions::share_skill(
        &reg,
        &src,
        Provider::Agents,
        Scope::Global,
        "victim",
        ShareMethod::Copy,
    )
    .unwrap();

    let reg = fx.registry();
    let skill = reg.skills.iter().find(|s| s.name == "victim").unwrap();
    let agents_dir = skill
        .instance(Provider::Agents, Scope::Global)
        .unwrap()
        .dir
        .clone();
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
    let dir =
        actions::create_skill(&reg, "proj-skill", "d", Provider::Claude, Scope::Project).unwrap();
    assert!(dir.join("SKILL.md").exists());
    assert!(dir_contains(
        &fx.base.join("proj/.claude/skills"),
        "proj-skill"
    ));

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
    let dir = actions::install_skill(
        &reg,
        "netskill",
        &content,
        Provider::Claude,
        Scope::Global,
        false,
    )
    .unwrap();
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
    actions::install_skill(
        &reg,
        "netskill",
        &content,
        Provider::Claude,
        Scope::Global,
        false,
    )
    .unwrap();
    let reg = fx.registry();
    assert!(actions::install_skill(
        &reg,
        "netskill",
        &content,
        Provider::Claude,
        Scope::Global,
        false
    )
    .is_err());
    assert!(actions::install_skill(
        &reg,
        "netskill",
        &content,
        Provider::Claude,
        Scope::Global,
        true
    )
    .is_ok());
}

struct HarnessFixture {
    base: PathBuf,
    home: PathBuf,
    project: PathBuf,
}

impl HarnessFixture {
    fn new(tag: &str) -> HarnessFixture {
        let base = std::env::temp_dir().join(format!("skillsdash-harness-{tag}"));
        let _ = fs::remove_dir_all(&base);
        let home = base.join("home");
        let project = base.join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&project).unwrap();
        HarnessFixture {
            base,
            home,
            project,
        }
    }

    fn registry(&self) -> HarnessRegistry {
        HarnessRegistry::with_home(Some(&self.home), &self.project)
    }
}

impl Drop for HarnessFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[test]
fn harness_lists_memory_files_for_both_providers_and_scopes() {
    let fx = HarnessFixture::new("mem-list");
    let reg = fx.registry();
    let memories: Vec<_> = reg
        .files
        .iter()
        .filter(|f| f.kind == HarnessKind::Memory)
        .collect();
    assert_eq!(memories.len(), 4);
    assert!(memories
        .iter()
        .any(|f| f.name == "CLAUDE.md" && f.provider == Provider::Claude));
    assert!(memories
        .iter()
        .any(|f| f.name == "AGENTS.md" && f.provider == Provider::Agents));
    assert!(memories.iter().all(|f| !f.exists));
}

#[test]
fn harness_memory_paths_land_at_expected_locations() {
    let fx = HarnessFixture::new("mem-path");
    let reg = fx.registry();
    let claude_global = reg
        .files
        .iter()
        .find(|f| {
            f.kind == HarnessKind::Memory
                && f.provider == Provider::Claude
                && f.scope == Scope::Global
        })
        .unwrap();
    assert_eq!(claude_global.path, fx.home.join(".claude/CLAUDE.md"));

    let agents_project = reg
        .files
        .iter()
        .find(|f| {
            f.kind == HarnessKind::Memory
                && f.provider == Provider::Agents
                && f.scope == Scope::Project
        })
        .unwrap();
    assert_eq!(agents_project.path, fx.project.join("AGENTS.md"));
}

#[test]
fn harness_save_creates_and_reads_back() {
    let fx = HarnessFixture::new("save");
    let reg = fx.registry();
    let claude = reg
        .files
        .iter()
        .find(|f| {
            f.kind == HarnessKind::Memory
                && f.provider == Provider::Claude
                && f.scope == Scope::Global
        })
        .unwrap()
        .clone();
    actions::save_harness(&claude.path, "# rules\n\nbe brief").unwrap();
    assert!(claude.path.exists());
    let content = fs::read_to_string(&claude.path).unwrap();
    assert!(content.contains("be brief"));
    assert!(content.ends_with('\n'));

    let reg2 = fx.registry();
    let claude2 = reg2
        .files
        .iter()
        .find(|f| {
            f.provider == Provider::Claude
                && f.scope == Scope::Global
                && f.kind == HarnessKind::Memory
        })
        .unwrap();
    assert!(claude2.exists);
    assert_eq!(actions::read_harness(claude2).unwrap(), content);
}

#[test]
fn harness_link_counterpart_symlinks_other_provider() {
    let fx = HarnessFixture::new("link");
    let reg = fx.registry();
    let claude = reg
        .files
        .iter()
        .find(|f| {
            f.kind == HarnessKind::Memory
                && f.provider == Provider::Claude
                && f.scope == Scope::Global
        })
        .unwrap()
        .clone();
    actions::save_harness(&claude.path, "shared instructions").unwrap();

    let dst = actions::link_counterpart(&reg, &claude, Provider::Agents).unwrap();
    assert_eq!(dst, fx.home.join(".agents/AGENTS.md"));
    assert!(dst.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(
        fs::read_to_string(&dst).unwrap(),
        fs::read_to_string(&claude.path).unwrap()
    );

    let reg2 = fx.registry();
    let agents = reg2
        .files
        .iter()
        .find(|f| {
            f.provider == Provider::Agents
                && f.scope == Scope::Global
                && f.kind == HarnessKind::Memory
        })
        .unwrap();
    assert!(agents.is_symlink);
    assert!(agents.exists);
}

#[test]
fn harness_link_refuses_when_source_missing_or_target_exists() {
    let fx = HarnessFixture::new("link-guard");
    let reg = fx.registry();
    let claude = reg
        .files
        .iter()
        .find(|f| {
            f.kind == HarnessKind::Memory
                && f.provider == Provider::Claude
                && f.scope == Scope::Global
        })
        .unwrap()
        .clone();
    assert!(actions::link_counterpart(&reg, &claude, Provider::Agents).is_err());

    actions::save_harness(&claude.path, "a").unwrap();
    let agents_path = fx.home.join(".agents/AGENTS.md");
    actions::save_harness(&agents_path, "b").unwrap();
    assert!(actions::link_counterpart(&reg, &claude, Provider::Agents).is_err());
    assert_eq!(fs::read_to_string(&agents_path).unwrap(), "b\n");
}

#[test]
fn harness_create_command_and_delete() {
    let fx = HarnessFixture::new("cmd");
    let reg = fx.registry();
    let path = actions::create_command(&reg, "review", Provider::Claude, Scope::Project).unwrap();
    assert_eq!(path, fx.project.join(".claude/commands/review.md"));
    assert!(path.exists());

    let reg2 = fx.registry();
    let cmd = reg2
        .files
        .iter()
        .find(|f| f.kind == HarnessKind::Command && f.name == "review.md")
        .unwrap();
    assert_eq!(cmd.provider, Provider::Claude);
    assert_eq!(cmd.scope, Scope::Project);

    assert!(actions::create_command(&reg2, "review", Provider::Claude, Scope::Project).is_err());

    actions::delete_harness(&path).unwrap();
    assert!(!path.exists());
}

#[test]
fn harness_save_over_symlink_replaces_link_not_target() {
    let fx = HarnessFixture::new("save-symlink");
    let reg = fx.registry();
    let claude = reg
        .files
        .iter()
        .find(|f| {
            f.kind == HarnessKind::Memory
                && f.provider == Provider::Claude
                && f.scope == Scope::Global
        })
        .unwrap()
        .clone();
    actions::save_harness(&claude.path, "original").unwrap();
    let agents_path = actions::link_counterpart(&reg, &claude, Provider::Agents).unwrap();

    actions::save_harness(&agents_path, "diverged").unwrap();
    assert!(!agents_path
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read_to_string(&agents_path).unwrap(), "diverged\n");
    assert_eq!(fs::read_to_string(&claude.path).unwrap(), "original\n");
}

#[test]
fn install_rejects_path_traversal() {
    let fx = Fixture::new("install-traversal");
    let reg = fx.registry();
    let evil = SkillContent {
        files: vec![
            SkillFile {
                relative_path: "SKILL.md".to_string(),
                bytes: b"---\nname: x\ndescription: d\n---\n".to_vec(),
            },
            SkillFile {
                relative_path: "../../escape.txt".to_string(),
                bytes: b"pwned".to_vec(),
            },
        ],
    };
    assert!(
        actions::install_skill(&reg, "x", &evil, Provider::Claude, Scope::Global, false).is_err()
    );
    assert!(!fx.base.join("escape.txt").exists());
}

fn dir_contains(dir: &Path, name: &str) -> bool {
    fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy() == name)
        })
        .unwrap_or(false)
}
