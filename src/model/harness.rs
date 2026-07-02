use super::skill::{Provider, Scope};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HarnessKind {
    Memory,
    Command,
}

impl HarnessKind {
    pub fn label(self) -> &'static str {
        match self {
            HarnessKind::Memory => "memory",
            HarnessKind::Command => "command",
        }
    }
}

pub fn memory_file_name(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "CLAUDE.md",
        Provider::Agents => "AGENTS.md",
    }
}

#[derive(Debug, Clone)]
pub struct HarnessFile {
    pub kind: HarnessKind,
    pub provider: Provider,
    pub scope: Scope,
    pub name: String,
    pub path: PathBuf,
    pub exists: bool,
    pub is_symlink: bool,
    pub link_target: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct HarnessRegistry {
    roots: Vec<HarnessRoot>,
    pub files: Vec<HarnessFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct HarnessRoot {
    provider: Provider,
    scope: Scope,
    memory_dir: PathBuf,
    commands_dir: PathBuf,
}

impl HarnessRegistry {
    pub fn discover(project_dir: &Path) -> HarnessRegistry {
        Self::with_home(dirs::home_dir().as_deref(), project_dir)
    }

    pub fn with_home(home: Option<&Path>, project_dir: &Path) -> HarnessRegistry {
        let mut reg = HarnessRegistry {
            roots: Self::roots(home, project_dir),
            files: Vec::new(),
            warnings: Vec::new(),
        };
        reg.reload();
        reg
    }

    fn roots(home: Option<&Path>, project_dir: &Path) -> Vec<HarnessRoot> {
        let mut roots = Vec::new();

        if let Some(home) = home {
            for provider in Provider::ALL {
                roots.push(HarnessRoot {
                    provider,
                    scope: Scope::Global,
                    memory_dir: home.join(provider.dir_name()),
                    commands_dir: home.join(provider.dir_name()).join("commands"),
                });
            }
        }

        let project_is_home = home.map(|h| same_dir(h, project_dir)).unwrap_or(false);

        if !project_is_home {
            for provider in Provider::ALL {
                roots.push(HarnessRoot {
                    provider,
                    scope: Scope::Project,
                    memory_dir: project_dir.to_path_buf(),
                    commands_dir: project_dir.join(provider.dir_name()).join("commands"),
                });
            }
        }

        roots
    }

    pub fn reload(&mut self) {
        let mut files = Vec::new();
        let mut warnings = Vec::new();

        for root in &self.roots {
            let mem_name = memory_file_name(root.provider);
            let mem_path = root.memory_dir.join(mem_name);
            files.push(describe_file(
                HarnessKind::Memory,
                root.provider,
                root.scope,
                mem_name.to_string(),
                mem_path,
            ));

            match scan_commands(root) {
                Ok(found) => files.extend(found),
                Err(e) => warnings.push(format!("{}: {e}", root.commands_dir.display())),
            }
        }

        files.sort_by(|a, b| {
            a.kind
                .cmp(&b.kind)
                .then(a.scope.cmp(&b.scope))
                .then(a.provider.cmp(&b.provider))
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        self.files = files;
        self.warnings = warnings;
    }

    pub fn commands_dir(&self, provider: Provider, scope: Scope) -> Option<PathBuf> {
        self.roots
            .iter()
            .find(|r| r.provider == provider && r.scope == scope)
            .map(|r| r.commands_dir.clone())
    }

    pub fn memory_dir(&self, provider: Provider, scope: Scope) -> Option<PathBuf> {
        self.roots
            .iter()
            .find(|r| r.provider == provider && r.scope == scope)
            .map(|r| r.memory_dir.clone())
    }
}

fn scan_commands(root: &HarnessRoot) -> std::io::Result<Vec<HarnessFile>> {
    let mut out = Vec::new();
    if !root.commands_dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(&root.commands_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let is_md = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("md"))
            .unwrap_or(false);
        if !is_md {
            continue;
        }
        let is_file_or_symlink = std::fs::symlink_metadata(&path)
            .map(|m| {
                let file_type = m.file_type();
                file_type.is_file() || file_type.is_symlink()
            })
            .unwrap_or(false);
        if !is_file_or_symlink {
            continue;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(describe_file(
            HarnessKind::Command,
            root.provider,
            root.scope,
            name,
            path,
        ));
    }
    Ok(out)
}

fn describe_file(
    kind: HarnessKind,
    provider: Provider,
    scope: Scope,
    name: String,
    path: PathBuf,
) -> HarnessFile {
    let sym_meta = path.symlink_metadata();
    let is_symlink = sym_meta
        .as_ref()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    let exists = path.exists();
    let link_target = if is_symlink {
        std::fs::read_link(&path).ok()
    } else {
        None
    };
    HarnessFile {
        kind,
        provider,
        scope,
        name,
        path,
        exists,
        is_symlink,
        link_target,
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}
