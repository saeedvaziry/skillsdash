use super::frontmatter::SkillDoc;
use super::skill::{Provider, Scope, Skill, SkillInstance};
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Root {
    pub provider: Provider,
    pub scope: Scope,
    pub skills_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Registry {
    pub roots: Vec<Root>,
    pub skills: Vec<Skill>,
    pub warnings: Vec<String>,
}

impl Registry {
    pub fn discover(project_dir: &Path) -> Registry {
        let mut roots = Vec::new();

        if let Some(home) = dirs::home_dir() {
            for provider in Provider::ALL {
                roots.push(Root {
                    provider,
                    scope: Scope::Global,
                    skills_dir: home.join(provider.dir_name()).join("skills"),
                });
            }
        }

        for provider in Provider::ALL {
            roots.push(Root {
                provider,
                scope: Scope::Project,
                skills_dir: project_dir.join(provider.dir_name()).join("skills"),
            });
        }

        Registry::from_roots(roots)
    }

    pub fn from_roots(roots: Vec<Root>) -> Registry {
        let mut reg = Registry {
            roots,
            skills: Vec::new(),
            warnings: Vec::new(),
        };
        reg.reload();
        reg
    }

    pub fn reload(&mut self) {
        let mut merged: BTreeMap<String, Skill> = BTreeMap::new();
        let mut warnings = Vec::new();

        for root in &self.roots {
            match scan_root(root) {
                Ok(found) => {
                    for (instance, doc) in found {
                        let entry =
                            merged
                                .entry(name_key(&instance, &doc))
                                .or_insert_with(|| Skill {
                                    name: doc.name.clone(),
                                    description: doc.description.clone(),
                                    instances: Vec::new(),
                                });
                        if entry.description.is_empty() && !doc.description.is_empty() {
                            entry.description = doc.description.clone();
                        }
                        entry.instances.push(instance);
                    }
                }
                Err(e) => warnings.push(format!("{}: {e}", root.skills_dir.display())),
            }
        }

        for skill in merged.values_mut() {
            skill.instances.sort_by_key(|i| (i.scope, i.provider));
        }

        self.skills = merged.into_values().collect();
        self.skills.sort_by_key(|s| s.name.to_lowercase());
        self.warnings = warnings;
    }

    pub fn root(&self, provider: Provider, scope: Scope) -> Option<&Root> {
        self.roots
            .iter()
            .find(|r| r.provider == provider && r.scope == scope)
    }

    pub fn skills_dir(&self, provider: Provider, scope: Scope) -> Option<PathBuf> {
        self.root(provider, scope).map(|r| r.skills_dir.clone())
    }
}

fn name_key(instance: &SkillInstance, doc: &SkillDoc) -> String {
    if !doc.name.trim().is_empty() {
        doc.name.trim().to_string()
    } else {
        instance
            .dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    }
}

fn scan_root(root: &Root) -> Result<Vec<(SkillInstance, SkillDoc)>> {
    let mut out = Vec::new();
    if !root.skills_dir.exists() {
        return Ok(out);
    }

    for entry in std::fs::read_dir(&root.skills_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let is_symlink = path
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        let is_dir = std::fs::metadata(&path)
            .map(|m| m.is_dir())
            .unwrap_or(false);

        if !is_dir {
            continue;
        }

        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }

        let doc = match SkillDoc::from_file(&skill_md) {
            Ok(d) => d,
            Err(_) => {
                let name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                SkillDoc::new(name, "(unparseable frontmatter)", String::new())
            }
        };

        let mut doc = doc;
        if doc.name.trim().is_empty() {
            if let Some(dir_name) = path.file_name() {
                doc.name = dir_name.to_string_lossy().to_string();
            }
        }

        out.push((
            SkillInstance {
                provider: root.provider,
                scope: root.scope,
                dir: path,
                skill_md,
                is_symlink,
            },
            doc,
        ));
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_same_name_across_roots() {
        let home = dirs::home_dir().unwrap();
        let claude = home.join(".claude").join("skills");
        if !claude.exists() {
            return;
        }
        let reg = Registry::discover(Path::new("/tmp/nonexistent-skillsdash-test"));
        for skill in &reg.skills {
            assert!(!skill.name.is_empty());
            assert!(!skill.instances.is_empty());
        }
    }
}
