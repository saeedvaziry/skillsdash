use super::{SkillContent, SkillFile};
use anyhow::{anyhow, bail, Result};

pub trait Http {
    fn get_string(&self, url: &str) -> Result<String>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}

pub fn default_branch(http: &dyn Http, owner: &str, repo: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}");
    let body = http.get_string(&url)?;
    let json: serde_json::Value = serde_json::from_str(&body)?;
    json.get("default_branch")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("repo {owner}/{repo} not found or has no default branch"))
}

pub fn find_skill_dir(tree_json: &str, skill_id: &str) -> Result<String> {
    let json: serde_json::Value = serde_json::from_str(tree_json)?;
    let tree = json
        .get("tree")
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow!("unexpected tree response"))?;

    let suffix = format!("/{skill_id}/SKILL.md");
    let flat = format!("{skill_id}/SKILL.md");

    let mut candidates: Vec<String> = tree
        .iter()
        .filter_map(|e| e.get("path").and_then(|p| p.as_str()))
        .filter(|p| p.ends_with(&suffix) || *p == flat)
        .map(|p| p.trim_end_matches("/SKILL.md").to_string())
        .collect();

    candidates.sort_by_key(|p| p.matches('/').count());

    candidates
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no SKILL.md found for '{skill_id}' in this repository"))
}

pub fn dir_files(tree_json: &str, dir: &str) -> Result<Vec<String>> {
    let json: serde_json::Value = serde_json::from_str(tree_json)?;
    let tree = json
        .get("tree")
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow!("unexpected tree response"))?;

    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };

    let mut files: Vec<String> = tree
        .iter()
        .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("blob"))
        .filter_map(|e| e.get("path").and_then(|p| p.as_str()))
        .filter(|p| p.starts_with(&prefix))
        .filter(|p| !p[prefix.len()..].contains('/'))
        .map(|p| p.to_string())
        .collect();
    files.sort();
    Ok(files)
}

pub fn fetch_skill(http: &dyn Http, owner: &str, repo: &str, skill_id: &str) -> Result<SkillContent> {
    let branch = default_branch(http, owner, repo)?;
    let tree_url =
        format!("https://api.github.com/repos/{owner}/{repo}/git/trees/{branch}?recursive=1");
    let tree_json = http.get_string(&tree_url)?;

    let dir = find_skill_dir(&tree_json, skill_id)?;
    let file_paths = dir_files(&tree_json, &dir)?;
    if file_paths.is_empty() {
        bail!("skill directory '{dir}' is empty");
    }

    let dir_prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };

    let mut files = Vec::new();
    for path in file_paths {
        let raw_url =
            format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path}");
        let bytes = http.get_bytes(&raw_url)?;
        let relative = path
            .strip_prefix(&dir_prefix)
            .unwrap_or(&path)
            .to_string();
        files.push(SkillFile {
            relative_path: relative,
            bytes,
        });
    }

    Ok(SkillContent { files })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANTHROPIC_TREE: &str = r#"{
        "tree": [
            {"path": "README.md", "type": "blob"},
            {"path": "skills", "type": "tree"},
            {"path": "skills/frontend-design", "type": "tree"},
            {"path": "skills/frontend-design/SKILL.md", "type": "blob"},
            {"path": "skills/frontend-design/LICENSE.txt", "type": "blob"},
            {"path": "skills/other/SKILL.md", "type": "blob"}
        ]
    }"#;

    const NESTED_TREE: &str = r#"{
        "tree": [
            {"path": "packages/playground/.agents/skills/frontend-design/SKILL.md", "type": "blob"},
            {"path": "packages/playground/.agents/skills/frontend-design/refs/guide.md", "type": "blob"},
            {"path": "README.md", "type": "blob"}
        ]
    }"#;

    #[test]
    fn finds_top_level_skill_dir() {
        let dir = find_skill_dir(ANTHROPIC_TREE, "frontend-design").unwrap();
        assert_eq!(dir, "skills/frontend-design");
    }

    #[test]
    fn finds_deeply_nested_skill_dir() {
        let dir = find_skill_dir(NESTED_TREE, "frontend-design").unwrap();
        assert_eq!(dir, "packages/playground/.agents/skills/frontend-design");
    }

    #[test]
    fn missing_skill_errors() {
        assert!(find_skill_dir(ANTHROPIC_TREE, "does-not-exist").is_err());
    }

    #[test]
    fn lists_only_direct_files() {
        let files = dir_files(ANTHROPIC_TREE, "skills/frontend-design").unwrap();
        assert_eq!(files, vec![
            "skills/frontend-design/LICENSE.txt".to_string(),
            "skills/frontend-design/SKILL.md".to_string(),
        ]);
    }

    #[test]
    fn nested_dir_excludes_subfolders() {
        let dir = "packages/playground/.agents/skills/frontend-design";
        let files = dir_files(NESTED_TREE, dir).unwrap();
        assert_eq!(files, vec![format!("{dir}/SKILL.md")]);
    }

    #[test]
    fn prefers_shallowest_when_ambiguous() {
        let tree = r#"{"tree":[
            {"path":"a/b/c/dup/SKILL.md","type":"blob"},
            {"path":"dup/SKILL.md","type":"blob"}
        ]}"#;
        assert_eq!(find_skill_dir(tree, "dup").unwrap(), "dup");
    }
}
