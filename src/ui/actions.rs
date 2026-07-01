use crate::market::SkillContent;
use crate::model::frontmatter::SkillDoc;
use crate::model::{fsops, Provider, Registry, Scope};
use anyhow::{anyhow, bail, Result};
use std::path::{Path, PathBuf};

pub fn install_skill(
    registry: &Registry,
    name: &str,
    content: &SkillContent,
    provider: Provider,
    scope: Scope,
    overwrite: bool,
) -> Result<PathBuf> {
    if content.skill_md().is_none() {
        bail!("downloaded skill has no SKILL.md");
    }
    let skills_dir = registry
        .skills_dir(provider, scope)
        .ok_or_else(|| anyhow!("no skills directory for {provider}/{scope}"))?;
    let dir = skills_dir.join(name);
    if dir.exists() {
        if overwrite {
            fsops::delete_dir(&dir)?;
        } else {
            bail!("'{name}' already exists in {provider}/{scope}");
        }
    }
    let files: Vec<(String, Vec<u8>)> = content
        .files
        .iter()
        .map(|f| (f.relative_path.clone(), f.bytes.clone()))
        .collect();
    fsops::write_files(&dir, &files)?;
    Ok(dir)
}

pub fn create_skill(
    registry: &Registry,
    name: &str,
    description: &str,
    provider: Provider,
    scope: Scope,
) -> Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        bail!("skill name cannot be empty");
    }
    if !is_valid_name(name) {
        bail!("skill name may only contain letters, digits, '-' and '_'");
    }

    let skills_dir = registry
        .skills_dir(provider, scope)
        .ok_or_else(|| anyhow!("no skills directory for {provider}/{scope}"))?;
    let dir = skills_dir.join(name);
    if dir.exists() {
        bail!("skill '{name}' already exists at {}", dir.display());
    }

    let body = format!(
        "# {name}\n\n{}\n",
        if description.is_empty() {
            "Describe what this skill does."
        } else {
            description
        }
    );
    let doc = SkillDoc::new(name, description, body);
    fsops::write_skill_md(&dir, &doc.to_markdown()?)?;
    Ok(dir)
}

pub fn delete_instances(dirs: &[PathBuf]) -> Result<()> {
    let mut errors = Vec::new();
    for dir in dirs {
        if let Err(e) = fsops::delete_dir(dir) {
            errors.push(e.to_string());
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        bail!(errors.join("; "))
    }
}

pub enum ShareMethod {
    Copy,
    Symlink,
}

pub fn share_skill(
    registry: &Registry,
    source_dir: &Path,
    target_provider: Provider,
    target_scope: Scope,
    skill_name: &str,
    method: ShareMethod,
) -> Result<PathBuf> {
    let skills_dir = registry
        .skills_dir(target_provider, target_scope)
        .ok_or_else(|| anyhow!("no skills directory for {target_provider}/{target_scope}"))?;
    let dst = skills_dir.join(skill_name);
    if dst.exists() {
        bail!("target already exists: {}", dst.display());
    }
    match method {
        ShareMethod::Copy => fsops::copy_dir(source_dir, &dst)?,
        ShareMethod::Symlink => fsops::symlink_dir(source_dir, &dst)?,
    }
    Ok(dst)
}

pub fn save_body(skill_md: &Path, new_body: &str) -> Result<()> {
    let mut doc = SkillDoc::from_file(skill_md)?;
    doc.body = ensure_trailing_newline(new_body);
    std::fs::write(skill_md, doc.to_markdown()?)
        .map_err(|e| anyhow!("writing {}: {e}", skill_md.display()))?;
    Ok(())
}

pub fn save_frontmatter(skill_md: &Path, name: &str, description: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        bail!("skill name cannot be empty");
    }
    let mut doc = SkillDoc::from_file(skill_md)?;
    doc.name = name.to_string();
    doc.description = description.trim().to_string();
    std::fs::write(skill_md, doc.to_markdown()?)
        .map_err(|e| anyhow!("writing {}: {e}", skill_md.display()))?;
    Ok(())
}

fn ensure_trailing_newline(s: &str) -> String {
    if s.is_empty() || s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{s}\n")
    }
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
