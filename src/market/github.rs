use super::{SkillContent, SkillFile};
use crate::model::SkillDoc;
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

pub fn find_skill_dir(tree_json: &str, skill_id: &str) -> Option<String> {
    let dirs = skill_md_dirs(tree_json).ok()?;

    let suffix = format!("/{skill_id}");
    let mut candidates: Vec<String> = dirs
        .into_iter()
        .filter(|d| d == skill_id || d.ends_with(&suffix))
        .collect();

    candidates.sort_by_key(|p| p.matches('/').count());
    candidates.into_iter().next()
}

fn skill_md_dirs(tree_json: &str) -> Result<Vec<String>> {
    let json: serde_json::Value = serde_json::from_str(tree_json)?;
    let tree = json
        .get("tree")
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow!("unexpected tree response"))?;

    let mut dirs: Vec<String> = tree
        .iter()
        .filter(|e| e.get("type").and_then(|t| t.as_str()) != Some("tree"))
        .filter_map(|e| e.get("path").and_then(|p| p.as_str()))
        .filter(|p| *p == "SKILL.md" || p.ends_with("/SKILL.md"))
        .map(|p| p.strip_suffix("SKILL.md").unwrap().trim_end_matches('/').to_string())
        .collect();

    dirs.sort_by_key(|p| p.matches('/').count());
    Ok(dirs)
}

fn resolve_skill_dir(
    http: &dyn Http,
    owner: &str,
    repo: &str,
    branch: &str,
    tree_json: &str,
    skill_id: &str,
) -> Result<String> {
    if let Some(dir) = find_skill_dir(tree_json, skill_id) {
        return Ok(dir);
    }

    for dir in skill_md_dirs(tree_json)? {
        let path = if dir.is_empty() {
            "SKILL.md".to_string()
        } else {
            format!("{dir}/SKILL.md")
        };
        let raw_url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path}");
        let Ok(contents) = http.get_string(&raw_url) else {
            continue;
        };
        let Ok(doc) = SkillDoc::parse(&contents) else {
            continue;
        };
        if doc.name == skill_id {
            return Ok(dir);
        }
    }

    Err(anyhow!(
        "no SKILL.md found for '{skill_id}' in this repository"
    ))
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

pub fn fetch_skill(
    http: &dyn Http,
    owner: &str,
    repo: &str,
    skill_id: &str,
) -> Result<SkillContent> {
    let branch = default_branch(http, owner, repo)?;
    let tree_url =
        format!("https://api.github.com/repos/{owner}/{repo}/git/trees/{branch}?recursive=1");
    let tree_bytes = http.get_bytes(&tree_url)?;
    let tree_json = String::from_utf8(tree_bytes)
        .map_err(|e| anyhow!("reading {tree_url}: invalid UTF-8: {e}"))?;

    let dir = resolve_skill_dir(http, owner, repo, &branch, &tree_json, skill_id)?;
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
        let raw_url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path}");
        let bytes = http.get_bytes(&raw_url)?;
        let relative = path.strip_prefix(&dir_prefix).unwrap_or(&path).to_string();
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
    fn missing_skill_returns_none() {
        assert!(find_skill_dir(ANTHROPIC_TREE, "does-not-exist").is_none());
    }

    #[test]
    fn lists_only_direct_files() {
        let files = dir_files(ANTHROPIC_TREE, "skills/frontend-design").unwrap();
        assert_eq!(
            files,
            vec![
                "skills/frontend-design/LICENSE.txt".to_string(),
                "skills/frontend-design/SKILL.md".to_string(),
            ]
        );
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

    const MISMATCHED_TREE: &str = r#"{
        "tree": [
            {"path": "skills", "type": "tree"},
            {"path": "skills/remotion", "type": "tree"},
            {"path": "skills/remotion/SKILL.md", "type": "blob"},
            {"path": "skills/remotion/rules/assets.md", "type": "blob"},
            {"path": "README.md", "type": "blob"}
        ]
    }"#;

    struct StubHttp {
        skill_md: String,
    }

    impl Http for StubHttp {
        fn get_string(&self, url: &str) -> Result<String> {
            if url.ends_with("/skills/remotion/SKILL.md") {
                Ok(self.skill_md.clone())
            } else {
                bail!("unexpected url: {url}")
            }
        }
        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>> {
            bail!("not used")
        }
    }

    #[test]
    fn resolves_by_frontmatter_name_when_dir_differs() {
        let http = StubHttp {
            skill_md: "---\nname: remotion-best-practices\ndescription: d\n---\nbody\n"
                .to_string(),
        };
        let dir = resolve_skill_dir(
            &http,
            "remotion-dev",
            "skills",
            "main",
            MISMATCHED_TREE,
            "remotion-best-practices",
        )
        .unwrap();
        assert_eq!(dir, "skills/remotion");
    }

    #[test]
    fn resolve_prefers_directory_name_without_fetching() {
        let http = StubHttp {
            skill_md: String::new(),
        };
        let dir = resolve_skill_dir(
            &http,
            "owner",
            "repo",
            "main",
            ANTHROPIC_TREE,
            "frontend-design",
        )
        .unwrap();
        assert_eq!(dir, "skills/frontend-design");
    }

    #[test]
    fn resolve_errors_when_no_frontmatter_matches() {
        let http = StubHttp {
            skill_md: "---\nname: something-else\ndescription: d\n---\nbody\n".to_string(),
        };
        let err = resolve_skill_dir(
            &http,
            "remotion-dev",
            "skills",
            "main",
            MISMATCHED_TREE,
            "remotion-best-practices",
        )
        .unwrap_err();
        assert!(err.to_string().contains("no SKILL.md found"));
    }
}
