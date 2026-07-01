use super::github::{self, Http};
use super::{MarketSkill, SkillContent};
use anyhow::{anyhow, Result};
use std::time::Duration;

const USER_AGENT: &str = "skillsdash-tui";

pub trait MarketClient {
    fn search(&self, query: &str, limit: u32) -> Result<Vec<MarketSkill>>;
    fn fetch(&self, skill: &MarketSkill) -> Result<SkillContent>;
}

pub struct UreqClient {
    agent: ureq::Agent,
}

impl Default for UreqClient {
    fn default() -> Self {
        UreqClient::new()
    }
}

impl UreqClient {
    pub fn new() -> UreqClient {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(20))
            .build();
        UreqClient { agent }
    }
}

impl Http for UreqClient {
    fn get_string(&self, url: &str) -> Result<String> {
        let resp = self
            .agent
            .get(url)
            .set("User-Agent", USER_AGENT)
            .set("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| map_err(url, e))?;
        resp.into_string()
            .map_err(|e| anyhow!("reading {url}: {e}"))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self
            .agent
            .get(url)
            .set("User-Agent", USER_AGENT)
            .call()
            .map_err(|e| map_err(url, e))?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut resp.into_reader(), &mut buf)
            .map_err(|e| anyhow!("downloading {url}: {e}"))?;
        Ok(buf)
    }
}

impl MarketClient for UreqClient {
    fn search(&self, query: &str, limit: u32) -> Result<Vec<MarketSkill>> {
        let url = format!(
            "https://www.skills.sh/api/search?q={}&limit={}",
            urlencode(query),
            limit
        );
        let body = self.get_string(&url)?;
        parse_search(&body)
    }

    fn fetch(&self, skill: &MarketSkill) -> Result<SkillContent> {
        let (owner, repo) = skill
            .owner_repo()
            .ok_or_else(|| anyhow!("cannot parse GitHub repo from source '{}'", skill.source))?;
        github::fetch_skill(self, &owner, &repo, &skill.skill_id)
    }
}

fn map_err(url: &str, e: ureq::Error) -> anyhow::Error {
    match e {
        ureq::Error::Status(code, _) => {
            if code == 403 && url.contains("api.github.com") {
                anyhow!("GitHub rate limit hit (HTTP 403) — try again in a few minutes")
            } else if code == 404 {
                anyhow!("not found (HTTP 404): {url}")
            } else {
                anyhow!("request failed (HTTP {code}): {url}")
            }
        }
        ureq::Error::Transport(t) => anyhow!("network error: {t}"),
    }
}

pub fn parse_search(body: &str) -> Result<Vec<MarketSkill>> {
    let json: serde_json::Value = serde_json::from_str(body)?;
    let skills = json
        .get("skills")
        .and_then(|s| s.as_array())
        .ok_or_else(|| anyhow!("unexpected search response"))?;

    let mut out = Vec::new();
    for entry in skills {
        let id = str_field(entry, "id");
        let skill_id = str_field(entry, "skillId");
        let name = str_field(entry, "name");
        let source = str_field(entry, "source");
        let installs = entry
            .get("installs")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if name.is_empty() || id.is_empty() {
            continue;
        }
        out.push(MarketSkill {
            id,
            skill_id,
            name,
            installs,
            source,
        });
    }
    Ok(out)
}

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_live_shaped_response() {
        let body = r#"{
            "query":"frontend","searchType":"fuzzy",
            "skills":[
                {"id":"anthropics/skills/frontend-design","skillId":"frontend-design","name":"frontend-design","installs":612248,"source":"anthropics/skills"},
                {"id":"vercel-labs/agent-eval/frontend-design","skillId":"frontend-design","name":"frontend-design","installs":680,"source":"vercel-labs/agent-eval"}
            ],
            "count":2,"duration_ms":388
        }"#;
        let skills = parse_search(body).unwrap();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "frontend-design");
        assert_eq!(skills[0].installs, 612248);
        assert_eq!(skills[0].owner_repo(), Some(("anthropics".to_string(), "skills".to_string())));
        assert_eq!(skills[1].source, "vercel-labs/agent-eval");
    }

    #[test]
    fn tolerates_missing_fields() {
        let body = r#"{"skills":[{"id":"a/b/c","name":"c"}]}"#;
        let skills = parse_search(body).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].installs, 0);
    }

    #[test]
    fn urlencodes_query() {
        assert_eq!(urlencode("a b/c"), "a%20b%2Fc");
    }
}
