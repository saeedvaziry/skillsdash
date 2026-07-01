pub mod client;
pub mod github;

pub use client::{MarketClient, UreqClient};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSkill {
    pub id: String,
    pub skill_id: String,
    pub name: String,
    pub installs: u64,
    pub source: String,
}

impl MarketSkill {
    pub fn owner_repo(&self) -> Option<(String, String)> {
        let mut parts = self.source.splitn(2, '/');
        let owner = parts.next()?.to_string();
        let repo = parts.next()?.to_string();
        if owner.is_empty() || repo.is_empty() {
            None
        } else {
            Some((owner, repo))
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillFile {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SkillContent {
    pub files: Vec<SkillFile>,
}

impl SkillContent {
    pub fn skill_md(&self) -> Option<&SkillFile> {
        self.files.iter().find(|f| f.relative_path == "SKILL.md")
    }
}
