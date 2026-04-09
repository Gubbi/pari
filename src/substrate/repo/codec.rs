//! `RepoCodec` — encode/decode between field JSON values and markdown+YAML format.

use std::collections::HashMap;
use crate::substrate::pipeline::{Codec, CodecError, FieldMapping};
use super::slot::{RepoSlot, SectionContent};

pub struct RepoCodec;

impl Codec for RepoCodec {
    type Slot = RepoSlot;
    type Encoded = String;

    fn encode(
        &self,
        fields: &HashMap<&str, serde_json::Value>,
        schema: &[FieldMapping<RepoSlot>],
    ) -> Result<String, CodecError> {
        let mut fm: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        let mut h1: Option<String> = None;
        let mut description: Option<String> = None;
        let mut sections: Vec<(&str, SectionContent, serde_json::Value)> = Vec::new();
        let mut file_content: Option<String> = None;

        for mapping in schema {
            let Some(value) = fields.get(mapping.key) else { continue };
            match mapping.slot {
                RepoSlot::H1 => {
                    h1 = value.as_str().map(str::to_string);
                }
                RepoSlot::DescriptionParagraph => {
                    description = value.as_str().map(str::to_string);
                }
                RepoSlot::FrontmatterKey(key) => {
                    fm.insert(key.to_string(), value.clone());
                }
                RepoSlot::FrontmatterFlattened => {
                    if let serde_json::Value::Object(ext) = value {
                        for (k, v) in ext {
                            fm.insert(k.clone(), v.clone());
                        }
                    }
                }
                RepoSlot::Section(heading, content) => {
                    sections.push((heading, content, value.clone()));
                }
                RepoSlot::FileContent => {
                    file_content = value.as_str().map(str::to_string);
                }
            }
        }

        if let Some(raw) = file_content {
            return Ok(raw);
        }

        let mut out = String::new();

        // --- YAML frontmatter ---
        out.push_str("---\n");
        if !fm.is_empty() {
            let yaml = serde_yaml::to_string(&fm)
                .map_err(|e| CodecError::new("frontmatter", e.to_string()))?;
            out.push_str(&yaml);
        }
        out.push_str("---\n");

        // --- H1 ---
        if let Some(name) = &h1 {
            out.push('\n');
            out.push_str("# ");
            out.push_str(name);
            out.push('\n');
        }

        // --- Description paragraph ---
        if let Some(desc) = &description {
            out.push('\n');
            out.push_str(desc);
            out.push('\n');
        }

        // --- Sections ---
        for (heading, content, value) in &sections {
            out.push('\n');
            out.push_str("## ");
            out.push_str(heading);
            out.push_str("\n\n");
            match content {
                SectionContent::Paragraph => {
                    if let Some(text) = value.as_str() {
                        out.push_str(text);
                        out.push('\n');
                    }
                }
                SectionContent::BulletList => {
                    if let Some(items) = value.as_array() {
                        for item in items {
                            if let Some(s) = item.as_str() {
                                out.push_str("- ");
                                out.push_str(s);
                                out.push('\n');
                            }
                        }
                    }
                }
            }
        }

        Ok(out)
    }

    fn decode(
        &self,
        raw: &String,
        schema: &[FieldMapping<RepoSlot>],
    ) -> Result<HashMap<String, serde_json::Value>, CodecError> {
        let (fm_str, body) = split_frontmatter(raw);

        // Parse YAML frontmatter
        let fm: HashMap<String, serde_json::Value> = if fm_str.trim().is_empty() {
            HashMap::new()
        } else {
            let yaml_val: serde_yaml::Value = serde_yaml::from_str(&fm_str)
                .map_err(|e| CodecError::new("frontmatter", e.to_string()))?;
            yaml_to_json_map(yaml_val)
                .map_err(|e| CodecError::new("frontmatter", e))?
        };

        // Collect named keys (to know what's "claimed" for FrontmatterFlattened)
        let mut claimed: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for mapping in schema {
            if let RepoSlot::FrontmatterKey(key) = mapping.slot {
                claimed.insert(key);
            }
        }

        let (h1, body_after_h1) = parse_h1(&body);
        let (description, sections_body) = parse_description(&body_after_h1);
        let section_map = parse_sections(&sections_body);

        let mut result = HashMap::new();

        for mapping in schema {
            match mapping.slot {
                RepoSlot::H1 => {
                    if let Some(name) = &h1 {
                        result.insert(mapping.key.to_string(), serde_json::Value::String(name.clone()));
                    }
                }
                RepoSlot::DescriptionParagraph => {
                    if let Some(desc) = &description {
                        result.insert(mapping.key.to_string(), serde_json::Value::String(desc.clone()));
                    }
                }
                RepoSlot::FrontmatterKey(key) => {
                    if let Some(v) = fm.get(key) {
                        result.insert(mapping.key.to_string(), v.clone());
                    }
                }
                RepoSlot::FrontmatterFlattened => {
                    let ext: serde_json::Map<String, serde_json::Value> = fm
                        .iter()
                        .filter(|(k, _)| !claimed.contains(k.as_str()))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    result.insert(mapping.key.to_string(), serde_json::Value::Object(ext));
                }
                RepoSlot::Section(heading, content) => {
                    if let Some(text) = section_map.get(heading) {
                        let value = match content {
                            SectionContent::Paragraph => {
                                serde_json::Value::String(text.trim().to_string())
                            }
                            SectionContent::BulletList => {
                                let items: Vec<serde_json::Value> = text
                                    .lines()
                                    .filter(|l| l.starts_with("- "))
                                    .map(|l| serde_json::Value::String(l[2..].to_string()))
                                    .collect();
                                serde_json::Value::Array(items)
                            }
                        };
                        result.insert(mapping.key.to_string(), value);
                    }
                }
                RepoSlot::FileContent => {
                    result.insert(mapping.key.to_string(), serde_json::Value::String(raw.clone()));
                }
            }
        }

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Split `---\n<frontmatter>\n---\n<body>` into `(frontmatter, body)`.
fn split_frontmatter(content: &str) -> (String, String) {
    if !content.starts_with("---\n") {
        return (String::new(), content.to_string());
    }
    let after_first = &content[4..]; // skip opening "---\n"
    // Find the closing "---" line
    if let Some(pos) = after_first.find("---\n") {
        let fm = after_first[..pos].to_string();
        let body = after_first[pos + 4..].to_string();
        (fm, body)
    } else if let Some(pos) = after_first.find("---") {
        // Closing "---" at end of file without trailing newline
        let fm = after_first[..pos].to_string();
        (fm, String::new())
    } else {
        (String::new(), content.to_string())
    }
}

/// Extract the H1 heading and return `(h1_text, body_after_h1)`.
fn parse_h1(body: &str) -> (Option<String>, String) {
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            // Find this line in body and return everything after it
            if let Some(pos) = body.find(&format!("# {}\n", rest)) {
                let after = body[pos + rest.len() + 3..].to_string();
                return (Some(rest.to_string()), after);
            }
        }
    }
    (None, body.to_string())
}

/// Extract the first paragraph after H1 (before first `##` or EOF).
/// Returns `(description, remaining_body)`.
fn parse_description(body: &str) -> (Option<String>, String) {
    let mut lines = body.lines().peekable();
    // Skip leading blank lines
    while lines.peek() == Some(&"") {
        lines.next();
    }
    // Collect non-blank lines until `##` heading or blank line after content
    let mut desc_lines: Vec<&str> = Vec::new();
    for line in lines {
        if line.starts_with("## ") {
            // Stop at section heading — reconstruct remaining body
            let remaining = if desc_lines.is_empty() {
                body.to_string()
            } else {
                // Find where section heading starts
                body[body.find(&format!("## ")).unwrap_or(body.len())..].to_string()
            };
            let desc = if desc_lines.is_empty() {
                None
            } else {
                Some(desc_lines.join("\n"))
            };
            return (desc, remaining);
        }
        if line.is_empty() {
            if !desc_lines.is_empty() {
                // End of first paragraph
                break;
            }
        } else {
            desc_lines.push(line);
        }
    }

    if desc_lines.is_empty() {
        return (None, body.to_string());
    }

    let desc = desc_lines.join("\n");
    // Find the section body after the description
    let section_start = body.find("\n## ").map(|p| p + 1).unwrap_or(body.len());
    (Some(desc), body[section_start..].to_string())
}

/// Parse all `## Heading\n\ncontent` sections, returning `HashMap<heading, content>`.
fn parse_sections(body: &str) -> HashMap<String, String> {
    let mut sections: HashMap<String, String> = HashMap::new();
    let mut current_heading: Option<String> = None;
    let mut current_content: Vec<&str> = Vec::new();

    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            // Save previous section
            if let Some(h) = current_heading.take() {
                // Trim trailing blank lines
                let content = current_content
                    .iter()
                    .rev()
                    .skip_while(|l| l.is_empty())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .copied()
                    .collect::<Vec<_>>()
                    .join("\n");
                sections.insert(h, content);
                current_content.clear();
            }
            current_heading = Some(heading.to_string());
        } else if current_heading.is_some() {
            current_content.push(line);
        }
    }

    // Save last section
    if let Some(h) = current_heading {
        let content = current_content
            .iter()
            .rev()
            .skip_while(|l| l.is_empty())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        sections.insert(h, content);
    }

    sections
}

/// Convert a `serde_yaml::Value` (expected to be a mapping) to `HashMap<String, serde_json::Value>`.
fn yaml_to_json_map(val: serde_yaml::Value) -> Result<HashMap<String, serde_json::Value>, String> {
    let json = serde_json::to_value(&val).map_err(|e| e.to_string())?;
    match json {
        serde_json::Value::Object(map) => Ok(map.into_iter().collect()),
        _ => Ok(HashMap::new()),
    }
}
