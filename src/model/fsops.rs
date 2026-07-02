use anyhow::{anyhow, bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn write_skill_md(dir: &Path, contents: &str) -> Result<PathBuf> {
    fs::create_dir_all(dir).map_err(|e| anyhow!("creating {}: {e}", dir.display()))?;
    let path = dir.join("SKILL.md");
    fs::write(&path, contents).map_err(|e| anyhow!("writing {}: {e}", path.display()))?;
    Ok(path)
}

pub fn write_files(dir: &Path, files: &[(String, Vec<u8>)]) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| anyhow!("creating {}: {e}", dir.display()))?;
    for (rel, bytes) in files {
        let safe = sanitize_relative(rel)
            .ok_or_else(|| anyhow!("refusing unsafe path in downloaded skill: {rel}"))?;
        let target = dir.join(&safe);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow!("creating {}: {e}", parent.display()))?;
        }
        fs::write(&target, bytes).map_err(|e| anyhow!("writing {}: {e}", target.display()))?;
    }
    Ok(())
}

fn sanitize_relative(rel: &str) -> Option<PathBuf> {
    let path = Path::new(rel);
    if path.is_absolute() {
        return None;
    }
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::Normal(c) => out.push(c),
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn delete_dir(dir: &Path) -> Result<()> {
    if dir.symlink_metadata()?.file_type().is_symlink() {
        remove_symlink(dir)?;
    } else {
        fs::remove_dir_all(dir).map_err(|e| anyhow!("deleting {}: {e}", dir.display()))?;
    }
    Ok(())
}

pub fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        bail!("target already exists: {}", dst.display());
    }
    copy_dir_recursive(src, dst)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|e| anyhow!("creating {}: {e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| anyhow!("reading {}: {e}", src.display()))? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(&from)?;
            symlink_path(&target, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| anyhow!("copying {}: {e}", from.display()))?;
        }
    }
    Ok(())
}

pub fn symlink_dir(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() || dst.symlink_metadata().is_ok() {
        bail!("target already exists: {}", dst.display());
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let target = fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    symlink_path(&target, dst)
}

pub fn symlink_file(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        bail!("source does not exist: {}", src.display());
    }
    if dst.exists() || dst.symlink_metadata().is_ok() {
        bail!("target already exists: {}", dst.display());
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let target = fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    symlink_path(&target, dst)
}

pub fn write_text_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| anyhow!("creating {}: {e}", parent.display()))?;
    }
    if path
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        remove_symlink(path)?;
    }
    fs::write(path, contents).map_err(|e| anyhow!("writing {}: {e}", path.display()))?;
    Ok(())
}

pub fn delete_file(path: &Path) -> Result<()> {
    let meta = path
        .symlink_metadata()
        .map_err(|e| anyhow!("reading {}: {e}", path.display()))?;
    if meta.file_type().is_symlink() {
        remove_symlink(path)?;
    } else {
        fs::remove_file(path).map_err(|e| anyhow!("deleting {}: {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn symlink_path(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link)
        .map_err(|e| anyhow!("symlinking {} -> {}: {e}", link.display(), target.display()))
}

#[cfg(windows)]
fn symlink_path(target: &Path, link: &Path) -> Result<()> {
    let res = if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    };
    res.map_err(|e| {
        anyhow!(
            "symlinking {} -> {}: {e} (Windows may require Developer Mode or admin rights for symlinks)",
            link.display(),
            target.display()
        )
    })
}

#[cfg(unix)]
fn remove_symlink(link: &Path) -> Result<()> {
    fs::remove_file(link).map_err(|e| anyhow!("removing symlink {}: {e}", link.display()))
}

#[cfg(windows)]
fn remove_symlink(link: &Path) -> Result<()> {
    let meta = link.symlink_metadata()?;
    let res = if meta.file_type().is_dir() {
        fs::remove_dir(link)
    } else {
        fs::remove_file(link)
    };
    res.map_err(|e| anyhow!("removing symlink {}: {e}", link.display()))
}
