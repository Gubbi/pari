use std::collections::HashMap;

use crate::substrate::{
    pipeline::{Codec, CodecError, FieldMapping},
    repo::schema::{RepoSlot, SectionContent},
    serde::value_at_path,
};

pub struct RepoCodec;

impl Codec for RepoCodec {
    type Slot = RepoSlot;
    type Encoded = String;

    fn encode(
        &self,
        entity_json: &serde_json::Value,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, CodecError> {
        if schema.len() == 1 && matches!(schema[0].slot, RepoSlot::FileContent) {
            return value_at_path(entity_json, schema[0].key)
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| CodecError::new(schema[0].key, "expected string file content"));
        }

        let mut frontmatter = serde_yaml::Mapping::new();
        let mut title = None;
        let mut description = None;
        let mut sections: Vec<(String, String)> = Vec::new();

        for field in schema {
            let Some(value) = value_at_path(entity_json, field.key) else {
                continue;
            };
            match field.slot {
                RepoSlot::H1 => {
                    title = value.as_str().map(str::to_string);
                }
                RepoSlot::DescriptionParagraph => {
                    description = match value {
                        serde_json::Value::Null => None,
                        serde_json::Value::String(s) => Some(s.clone()),
                        _ => {
                            return Err(CodecError::new(field.key, "description must be a string"))
                        }
                    };
                }
                RepoSlot::FrontmatterKey(key) => {
                    frontmatter.insert(
                        serde_yaml::Value::String(key.to_string()),
                        serde_yaml::to_value(value)
                            .map_err(|e| CodecError::new(field.key, e.to_string()))?,
                    );
                }
                RepoSlot::FrontmatterFlattened => {
                    let obj = value.as_object().ok_or_else(|| {
                        CodecError::new(field.key, "extensions must be a JSON object")
                    })?;
                    for (key, value) in obj {
                        frontmatter.insert(
                            serde_yaml::Value::String(key.clone()),
                            serde_yaml::to_value(value)
                                .map_err(|e| CodecError::new(field.key, e.to_string()))?,
                        );
                    }
                }
                RepoSlot::Section(heading, SectionContent::Paragraph) => {
                    if let Some(text) = value.as_str() {
                        sections.push((heading.to_string(), text.to_string()));
                    }
                }
                RepoSlot::Section(heading, SectionContent::BulletList) => {
                    let items = value.as_array().ok_or_else(|| {
                        CodecError::new(field.key, "section bullet list must be an array")
                    })?;
                    let body = items
                        .iter()
                        .map(|item| {
                            item.as_str()
                                .map(|text| format!("- {text}"))
                                .ok_or_else(|| {
                                    CodecError::new(field.key, "section bullet must be string")
                                })
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .join("\n");
                    sections.push((heading.to_string(), body));
                }
                RepoSlot::FileContent => {
                    return Err(CodecError::new(
                        field.key,
                        "FileContent must be a single-field asset",
                    ));
                }
            }
        }

        let mut rendered = String::new();
        if !frontmatter.is_empty() {
            rendered.push_str("---\n");
            rendered.push_str(
                &serde_yaml::to_string(&frontmatter)
                    .map_err(|e| CodecError::new("frontmatter", e.to_string()))?,
            );
            rendered.push_str("---\n\n");
        }
        if let Some(title) = title {
            rendered.push_str("# ");
            rendered.push_str(&title);
            rendered.push_str("\n\n");
        }
        if let Some(description) = description {
            rendered.push_str(&description);
            rendered.push_str("\n\n");
        }
        for (heading, body) in sections {
            rendered.push_str("## ");
            rendered.push_str(&heading);
            rendered.push_str("\n\n");
            rendered.push_str(&body);
            rendered.push_str("\n\n");
        }

        Ok(rendered.trim_end().to_string() + "\n")
    }

    fn decode(
        &self,
        raw: &Self::Encoded,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<HashMap<String, serde_json::Value>, CodecError> {
        if schema.len() == 1 && matches!(schema[0].slot, RepoSlot::FileContent) {
            return Ok(HashMap::from([(
                schema[0].key.to_string(),
                serde_json::Value::String(raw.clone()),
            )]));
        }

        let (frontmatter, body) =
            split_frontmatter(raw).map_err(|e| CodecError::new("frontmatter", e))?;
        let title = find_h1(body);
        let description = find_description(body);
        let sections = parse_sections(body);

        let claimed_frontmatter: Vec<&str> = schema
            .iter()
            .filter_map(|field| match field.slot {
                RepoSlot::FrontmatterKey(key) => Some(key),
                _ => None,
            })
            .collect();

        let mut out = HashMap::new();
        for field in schema {
            let value = match field.slot {
                RepoSlot::H1 => title
                    .clone()
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),
                RepoSlot::DescriptionParagraph => description
                    .clone()
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),
                RepoSlot::FrontmatterKey(key) => frontmatter
                    .get(key)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                RepoSlot::FrontmatterFlattened => {
                    let mut extra = serde_json::Map::new();
                    for (key, value) in &frontmatter {
                        if !claimed_frontmatter.iter().any(|claimed| claimed == key) {
                            extra.insert(key.clone(), value.clone());
                        }
                    }
                    serde_json::Value::Object(extra)
                }
                RepoSlot::Section(heading, SectionContent::Paragraph) => sections
                    .get(heading)
                    .map(|text| serde_json::Value::String(text.clone()))
                    .unwrap_or(serde_json::Value::Null),
                RepoSlot::Section(heading, SectionContent::BulletList) => {
                    let items = sections
                        .get(heading)
                        .map(|body| {
                            body.lines()
                                .filter_map(|line| {
                                    line.strip_prefix("- ").map(|item| item.to_string())
                                })
                                .map(serde_json::Value::String)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    serde_json::Value::Array(items)
                }
                RepoSlot::FileContent => serde_json::Value::String(raw.clone()),
            };
            out.insert(field.key.to_string(), value);
        }

        Ok(out)
    }
}

fn split_frontmatter(raw: &str) -> Result<(HashMap<String, serde_json::Value>, &str), String> {
    if !raw.starts_with("---\n") {
        return Ok((HashMap::new(), raw));
    }

    let remainder = &raw[4..];
    let Some(end) = remainder.find("\n---\n") else {
        return Err("unterminated YAML frontmatter".to_string());
    };
    let yaml = &remainder[..end];
    let body = &remainder[end + 5..];
    let frontmatter: HashMap<String, serde_json::Value> =
        serde_yaml::from_str::<serde_yaml::Value>(yaml)
            .map_err(|e| e.to_string())
            .and_then(|value| serde_json::to_value(value).map_err(|e| e.to_string()))
            .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()))?;
    Ok((frontmatter, body))
}

fn find_h1(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        line.strip_prefix("# ")
            .map(|title| title.trim().to_string())
    })
}

fn find_description(body: &str) -> Option<String> {
    let mut after_h1 = false;
    let mut collected = Vec::new();
    for line in body.lines() {
        if !after_h1 {
            if line.starts_with("# ") {
                after_h1 = true;
            }
            continue;
        }
        if line.starts_with("## ") {
            break;
        }
        if line.trim().is_empty() {
            if !collected.is_empty() {
                break;
            }
            continue;
        }
        collected.push(line.trim().to_string());
    }
    if collected.is_empty() {
        None
    } else {
        Some(collected.join("\n"))
    }
}

fn parse_sections(body: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current: Option<String> = None;
    let mut collected = Vec::new();

    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some(name) = current.replace(heading.trim().to_string()) {
                sections.insert(name, collected.join("\n").trim().to_string());
                collected.clear();
            }
            continue;
        }
        if current.is_some() {
            collected.push(line.to_string());
        }
    }

    if let Some(name) = current {
        sections.insert(name, collected.join("\n").trim().to_string());
    }

    sections
}
