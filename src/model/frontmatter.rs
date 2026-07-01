use anyhow::{anyhow, Result};
use serde_norway::{Mapping, Value};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SkillDoc {
    pub name: String,
    pub description: String,
    pub extra: Mapping,
    pub body: String,
}

impl SkillDoc {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        SkillDoc {
            name: name.into(),
            description: description.into(),
            extra: Mapping::new(),
            body: body.into(),
        }
    }

    pub fn parse(raw: &str) -> Result<SkillDoc> {
        let (frontmatter, body) = split_frontmatter(raw)?;
        let mut map: Mapping = if frontmatter.trim().is_empty() {
            Mapping::new()
        } else {
            serde_norway::from_str(&frontmatter)
                .map_err(|e| anyhow!("invalid YAML frontmatter: {e}"))?
        };

        let name = take_string(&mut map, "name").unwrap_or_default();
        let description = take_string(&mut map, "description").unwrap_or_default();

        Ok(SkillDoc {
            name,
            description,
            extra: map,
            body,
        })
    }

    pub fn from_file(path: &Path) -> Result<SkillDoc> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("reading {}: {e}", path.display()))?;
        SkillDoc::parse(&raw)
    }

    pub fn to_markdown(&self) -> Result<String> {
        let mut map = Mapping::new();
        map.insert(Value::from("name"), Value::from(self.name.clone()));
        map.insert(
            Value::from("description"),
            Value::from(self.description.clone()),
        );
        for (k, v) in &self.extra {
            map.insert(k.clone(), v.clone());
        }

        let yaml = serde_norway::to_string(&Value::Mapping(map))
            .map_err(|e| anyhow!("serializing frontmatter: {e}"))?;

        let mut out = String::new();
        out.push_str("---\n");
        out.push_str(&yaml);
        if !yaml.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("---\n");
        if !self.body.is_empty() {
            if !self.body.starts_with('\n') {
                out.push('\n');
            }
            out.push_str(&self.body);
        }
        Ok(out)
    }
}

fn take_string(map: &mut Mapping, key: &str) -> Option<String> {
    match map.remove(Value::from(key)) {
        Some(Value::String(s)) => Some(s),
        Some(other) => Some(scalar_to_string(&other)),
        None => None,
    }
}

fn scalar_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => serde_norway::to_string(v)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn split_frontmatter(raw: &str) -> Result<(String, String)> {
    let normalized = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let trimmed_start = normalized.trim_start_matches(['\r', '\n']);

    if !trimmed_start.starts_with("---") {
        return Ok((String::new(), raw.to_string()));
    }

    let after_open = match trimmed_start.find('\n') {
        Some(i) => &trimmed_start[i + 1..],
        None => return Ok((String::new(), String::new())),
    };

    let mut fm = String::new();
    let mut rest_start = None;
    let mut idx = 0usize;
    for line in after_open.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if content == "---" || content == "..." {
            rest_start = Some(idx + line.len());
            break;
        }
        fm.push_str(line);
        idx += line.len();
    }

    match rest_start {
        Some(pos) => {
            let body = after_open[pos..]
                .trim_start_matches(['\r', '\n'])
                .to_string();
            Ok((fm, body))
        }
        None => Err(anyhow!("frontmatter opened with --- but never closed")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic() {
        let raw = "---\nname: foo\ndescription: does foo\n---\n\n# Foo\nbody here\n";
        let doc = SkillDoc::parse(raw).unwrap();
        assert_eq!(doc.name, "foo");
        assert_eq!(doc.description, "does foo");
        assert_eq!(doc.body, "# Foo\nbody here\n");
    }

    #[test]
    fn preserves_extra_keys() {
        let raw = "---\nname: foo\ndescription: d\nlicense: MIT\nmetadata:\n  author: me\n  version: \"1.0.0\"\n---\nbody\n";
        let doc = SkillDoc::parse(raw).unwrap();
        assert!(doc.extra.contains_key(Value::from("license")));
        assert!(doc.extra.contains_key(Value::from("metadata")));
        let out = doc.to_markdown().unwrap();
        assert!(out.contains("license: MIT"));
        assert!(out.contains("author: me"));
        let reparsed = SkillDoc::parse(&out).unwrap();
        assert_eq!(reparsed.name, "foo");
        assert_eq!(reparsed.description, "d");
        assert!(reparsed.extra.contains_key(Value::from("metadata")));
    }

    #[test]
    fn handles_folded_description() {
        let raw = "---\nname: r\ndescription: >-\n  line one\n  line two\n---\nbody\n";
        let doc = SkillDoc::parse(raw).unwrap();
        assert_eq!(doc.name, "r");
        assert!(doc.description.contains("line one"));
        assert!(doc.description.contains("line two"));
    }

    #[test]
    fn no_frontmatter_is_all_body() {
        let raw = "# Just a heading\nno frontmatter\n";
        let doc = SkillDoc::parse(raw).unwrap();
        assert_eq!(doc.name, "");
        assert_eq!(doc.body, raw);
    }

    #[test]
    fn round_trip_stable() {
        let raw = "---\nname: foo\ndescription: d\nlicense: MIT\n---\nhello\n";
        let doc = SkillDoc::parse(raw).unwrap();
        let out = doc.to_markdown().unwrap();
        let doc2 = SkillDoc::parse(&out).unwrap();
        let out2 = doc2.to_markdown().unwrap();
        assert_eq!(out, out2);
    }
}
