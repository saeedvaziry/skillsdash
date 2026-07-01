use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Provider {
    Claude,
    Agents,
}

impl Provider {
    pub const ALL: [Provider; 2] = [Provider::Claude, Provider::Agents];

    pub fn label(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::Agents => "agents",
        }
    }

    pub fn dir_name(self) -> &'static str {
        match self {
            Provider::Claude => ".claude",
            Provider::Agents => ".agents",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scope {
    Global,
    Project,
}

impl Scope {
    pub const ALL: [Scope; 2] = [Scope::Global, Scope::Project];

    pub fn label(self) -> &'static str {
        match self {
            Scope::Global => "global",
            Scope::Project => "project",
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInstance {
    pub provider: Provider,
    pub scope: Scope,
    pub dir: PathBuf,
    pub skill_md: PathBuf,
    pub is_symlink: bool,
}

impl SkillInstance {
    pub fn key(&self) -> (Provider, Scope) {
        (self.provider, self.scope)
    }
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub instances: Vec<SkillInstance>,
}

impl Skill {
    pub fn has(&self, provider: Provider, scope: Scope) -> bool {
        self.instances
            .iter()
            .any(|i| i.provider == provider && i.scope == scope)
    }

    pub fn instance(&self, provider: Provider, scope: Scope) -> Option<&SkillInstance> {
        self.instances
            .iter()
            .find(|i| i.provider == provider && i.scope == scope)
    }

    pub fn primary(&self) -> Option<&SkillInstance> {
        self.instances.first()
    }

    pub fn providers_in_scope(&self, scope: Scope) -> Vec<Provider> {
        Provider::ALL
            .iter()
            .copied()
            .filter(|p| self.has(*p, scope))
            .collect()
    }

    pub fn scopes(&self) -> Vec<Scope> {
        Scope::ALL
            .iter()
            .copied()
            .filter(|s| self.instances.iter().any(|i| i.scope == *s))
            .collect()
    }
}
