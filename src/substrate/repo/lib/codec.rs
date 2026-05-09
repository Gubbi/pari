use std::collections::{HashMap, HashSet};

use crate::{
    error::primitive::PrimitiveError,
    substrate::{
        lib::serde::{insert_path_value, value_at_path},
        pipeline::{Codec, FieldMapping},
        repo::lib::schema::{RepoSlot, SectionContent},
    },
};

pub struct RepoCodec;

impl Codec for RepoCodec {
    type Slot = RepoSlot;
    type Encoded = String;

    fn encode(
        &self,
        entity_json: &serde_json::Value,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, PrimitiveError> {
        if schema.len() == 1 && matches!(schema[0].slot, RepoSlot::FileContent) {
            let Some(value) = value_at_path(entity_json, schema[0].key) else {
                return Err(PrimitiveError::expected_scalar_value(
                    "expected scalar value",
                    schema[0].key,
                    "missing",
                ));
            };

            return value.as_str().map(str::to_string).ok_or_else(|| {
                PrimitiveError::expected_scalar_value(
                    "expected scalar value",
                    schema[0].key,
                    json_type_name(value),
                )
            });
        }

        let mut frontmatter = serde_yaml::Mapping::new();
        let mut title = None;
        let mut description = None;
        let mut sections: Vec<(String, String)> = Vec::new();

        // Phase 1: write each field whose slot owns a specific position
        // in the on-disk artifact. Flatten slots are deferred — they
        // absorb whatever's left at the wire's top level.
        for field in schema {
            match field.slot {
                RepoSlot::FrontmatterFlattened(_) | RepoSlot::SectionFlattened(_, _) => continue,
                _ => {}
            }
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
                            return Err(PrimitiveError::expected_scalar_value(
                                "expected scalar value",
                                field.key,
                                json_type_name(value),
                            ));
                        }
                    };
                }
                RepoSlot::FrontmatterKey(key) => {
                    frontmatter.insert(
                        serde_yaml::Value::String(key.to_string()),
                        serde_yaml::to_value(value).map_err(|e| {
                            PrimitiveError::json_encoding(
                                "json encoding failed",
                                field.key,
                                e.to_string(),
                            )
                        })?,
                    );
                }
                RepoSlot::Section(heading, SectionContent::Paragraph) => {
                    if let Some(text) = value.as_str() {
                        sections.push((heading.to_string(), text.to_string()));
                    }
                }
                RepoSlot::Section(heading, SectionContent::BulletList) => {
                    let items = value.as_array().ok_or_else(|| {
                        PrimitiveError::expected_array_value(
                            "expected array value",
                            field.key,
                            json_type_name(value),
                        )
                    })?;
                    let body = items
                        .iter()
                        .map(|item| {
                            item.as_str()
                                .map(|text| format!("- {text}"))
                                .ok_or_else(|| {
                                    PrimitiveError::expected_scalar_value(
                                        "expected scalar value",
                                        field.key,
                                        json_type_name(item),
                                    )
                                })
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .join("\n");
                    sections.push((heading.to_string(), body));
                }
                RepoSlot::FileContent => {
                    return Err(PrimitiveError::unsupported_slot_composition(
                        "unsupported slot composition",
                        "file_content",
                        field.key,
                    ));
                }
                RepoSlot::FrontmatterFlattened(_) | RepoSlot::SectionFlattened(_, _) => {
                    unreachable!("flatten slots deferred above")
                }
            }
        }

        // Phase 2: route entity_json's unclaimed top-level wire keys
        // through the asset's flatten slots by longest-prefix-match.
        // A key with no matching rule is a codec-level error — this
        // asset doesn't know how to persist it.
        if let Some(obj) = entity_json.as_object() {
            let claimed = claimed_top_level_keys(schema);
            for (key, value) in obj {
                if claimed.contains(key.as_str()) {
                    continue;
                }
                let Some(slot) = best_flatten_match(schema, key) else {
                    return Err(PrimitiveError::unsupported_slot_composition(
                        "no flatten slot accepts this wire key",
                        "flattened",
                        key,
                    ));
                };
                match slot {
                    RepoSlot::FrontmatterFlattened(_) => {
                        frontmatter.insert(
                            serde_yaml::Value::String(key.clone()),
                            serde_yaml::to_value(value).map_err(|e| {
                                PrimitiveError::json_encoding(
                                    "json encoding failed",
                                    key,
                                    e.to_string(),
                                )
                            })?,
                        );
                    }
                    RepoSlot::SectionFlattened(_, content) => {
                        let body = render_section_body(key, value, content)?;
                        sections.push((key.clone(), body));
                    }
                    _ => unreachable!("best_flatten_match returns only flatten slots"),
                }
            }
        }

        let mut rendered = String::new();
        if !frontmatter.is_empty() {
            rendered.push_str("---\n");
            rendered.push_str(&serde_yaml::to_string(&frontmatter).map_err(|e| {
                PrimitiveError::frontmatter_serialization(
                    "frontmatter serialization failed",
                    "frontmatter",
                    e.to_string(),
                )
            })?);
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
    ) -> Result<serde_json::Value, PrimitiveError> {
        if schema.len() == 1 && matches!(schema[0].slot, RepoSlot::FileContent) {
            let mut out = serde_json::Map::new();
            insert_path_value(
                &mut out,
                schema[0].key,
                serde_json::Value::String(raw.clone()),
            );
            return Ok(serde_json::Value::Object(out));
        }

        let (frontmatter, body) = split_frontmatter(raw).map_err(|_| {
            PrimitiveError::malformed_frontmatter("malformed frontmatter", raw.clone())
        })?;
        let title = find_h1(body);
        let description = find_description(body);
        let sections = parse_sections(body);

        let claimed_frontmatter: HashSet<&str> = schema
            .iter()
            .filter_map(|field| match field.slot {
                RepoSlot::FrontmatterKey(key) => Some(key),
                _ => None,
            })
            .collect();
        let claimed_sections: HashSet<&str> = schema
            .iter()
            .filter_map(|field| match field.slot {
                RepoSlot::Section(heading, _) => Some(heading),
                _ => None,
            })
            .collect();

        let mut out = serde_json::Map::new();

        // Phase 1: position-owning slots. Each non-flatten slot writes
        // to its declared field path (with dot-path nesting where the
        // schema asks for it).
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
                RepoSlot::Section(heading, SectionContent::Paragraph) => sections
                    .get(heading)
                    .map(|text| serde_json::Value::String(text.clone()))
                    .unwrap_or(serde_json::Value::Null),
                RepoSlot::Section(heading, SectionContent::BulletList) => {
                    let items = sections
                        .get(heading)
                        .map(|body| parse_bullet_list(body))
                        .unwrap_or_default();
                    serde_json::Value::Array(items)
                }
                RepoSlot::FileContent => serde_json::Value::String(raw.clone()),
                RepoSlot::FrontmatterFlattened(_) | RepoSlot::SectionFlattened(_, _) => continue,
            };
            insert_path_value(&mut out, field.key, value);
        }

        // Phase 2: flatten slots absorb unclaimed disk entries. Each
        // entry is routed by longest-prefix-match against this asset's
        // flatten slots of the matching target type. An entry with no
        // matching rule is a codec-level rejection — the on-disk
        // artifact carries something this asset can't represent.
        for (key, value) in &frontmatter {
            if claimed_frontmatter.contains(key.as_str()) {
                continue;
            }
            if best_flatten_target_match(schema, key, FlattenTarget::Frontmatter).is_none() {
                return Err(PrimitiveError::unsupported_slot_composition(
                    "no flatten slot accepts this frontmatter key",
                    "frontmatter_flattened",
                    key,
                ));
            }
            out.insert(key.clone(), value.clone());
        }

        for (heading, body) in &sections {
            if claimed_sections.contains(heading.as_str()) {
                continue;
            }
            let Some(slot) = best_flatten_target_match(schema, heading, FlattenTarget::Section)
            else {
                return Err(PrimitiveError::unsupported_slot_composition(
                    "no flatten slot accepts this section heading",
                    "section_flattened",
                    heading,
                ));
            };
            let value = match slot {
                RepoSlot::SectionFlattened(_, SectionContent::Paragraph) => {
                    serde_json::Value::String(body.clone())
                }
                RepoSlot::SectionFlattened(_, SectionContent::BulletList) => {
                    serde_json::Value::Array(parse_bullet_list(body))
                }
                _ => unreachable!("best_flatten_target_match scoped to Section"),
            };
            out.insert(heading.clone(), value);
        }

        Ok(serde_json::Value::Object(out))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FlattenTarget {
    Frontmatter,
    Section,
}

/// First-dot-segment of every non-flatten field key in the asset, plus
/// `entity_ref`. These are the top-level wire keys an encode call has
/// already routed through a named slot — anything else is fed to the
/// asset's flatten slots.
fn claimed_top_level_keys(schema: &[FieldMapping<RepoSlot>]) -> HashSet<&'static str> {
    let mut claimed: HashSet<&'static str> = HashSet::new();
    claimed.insert("entity_ref");
    for field in schema {
        match field.slot {
            RepoSlot::FrontmatterFlattened(_) | RepoSlot::SectionFlattened(_, _) => {}
            _ => {
                let head = field
                    .key
                    .split_once('.')
                    .map(|(head, _)| head)
                    .unwrap_or(field.key);
                claimed.insert(head);
            }
        }
    }
    claimed
}

/// Longest-prefix-match across every flatten slot in the asset.
fn best_flatten_match(schema: &[FieldMapping<RepoSlot>], key: &str) -> Option<RepoSlot> {
    schema
        .iter()
        .filter_map(|field| match field.slot {
            RepoSlot::FrontmatterFlattened(rule) | RepoSlot::SectionFlattened(rule, _) => {
                rule.match_len(key).map(|len| (len, field.slot))
            }
            _ => None,
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, slot)| slot)
}

/// Longest-prefix-match restricted to a target kind. Used by decode,
/// where a frontmatter entry can only be claimed by `FrontmatterFlattened`
/// and a section heading only by `SectionFlattened`.
fn best_flatten_target_match(
    schema: &[FieldMapping<RepoSlot>],
    key: &str,
    target: FlattenTarget,
) -> Option<RepoSlot> {
    schema
        .iter()
        .filter_map(|field| match (field.slot, target) {
            (RepoSlot::FrontmatterFlattened(rule), FlattenTarget::Frontmatter) => {
                rule.match_len(key).map(|len| (len, field.slot))
            }
            (RepoSlot::SectionFlattened(rule, _), FlattenTarget::Section) => {
                rule.match_len(key).map(|len| (len, field.slot))
            }
            _ => None,
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, slot)| slot)
}

fn render_section_body(
    key: &str,
    value: &serde_json::Value,
    content: SectionContent,
) -> Result<String, PrimitiveError> {
    match content {
        SectionContent::Paragraph => match value {
            serde_json::Value::String(s) => Ok(s.clone()),
            serde_json::Value::Null => Ok(String::new()),
            other => Err(PrimitiveError::expected_scalar_value(
                "expected scalar value",
                key,
                json_type_name(other),
            )),
        },
        SectionContent::BulletList => {
            let items = value.as_array().ok_or_else(|| {
                PrimitiveError::expected_array_value(
                    "expected array value",
                    key,
                    json_type_name(value),
                )
            })?;
            let body = items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(|text| format!("- {text}"))
                        .ok_or_else(|| {
                            PrimitiveError::expected_scalar_value(
                                "expected scalar value",
                                key,
                                json_type_name(item),
                            )
                        })
                })
                .collect::<Result<Vec<_>, _>>()?
                .join("\n");
            Ok(body)
        }
    }
}

fn parse_bullet_list(body: &str) -> Vec<serde_json::Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("- ").map(|item| item.to_string()))
        .map(serde_json::Value::String)
        .collect()
}

fn json_type_name(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
    .to_string()
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

#[cfg(test)]
mod tests {
    //! Unit coverage for the repo codec's pure markdown parsers.
    //! Functional tests exercise the happy paths via persist+reload;
    //! these pin malformed and edge-case inputs that don't fit the
    //! end-to-end shape.

    use rstest::rstest;
    use serde_json::json;

    use super::*;

    // -----------------------------------------------------------------------
    // split_frontmatter
    // -----------------------------------------------------------------------

    #[test]
    fn split_frontmatter_no_fence_returns_empty_map_and_full_body() {
        let (fm, body) = split_frontmatter("just a body\n").unwrap();
        assert!(fm.is_empty());
        assert_eq!(body, "just a body\n");
    }

    #[test]
    fn split_frontmatter_empty_input_returns_empty_map_and_empty_body() {
        let (fm, body) = split_frontmatter("").unwrap();
        assert!(fm.is_empty());
        assert_eq!(body, "");
    }

    #[test]
    fn split_frontmatter_single_key_value() {
        let (fm, body) = split_frontmatter("---\nkey: val\n---\nbody\n").unwrap();
        assert_eq!(fm.get("key"), Some(&json!("val")));
        assert_eq!(body, "body\n");
    }

    #[test]
    fn split_frontmatter_multi_key() {
        let raw = "---\nname: a\ncount: 7\nflag: true\n---\nbody";
        let (fm, body) = split_frontmatter(raw).unwrap();
        assert_eq!(fm.get("name"), Some(&json!("a")));
        assert_eq!(fm.get("count"), Some(&json!(7)));
        assert_eq!(fm.get("flag"), Some(&json!(true)));
        assert_eq!(body, "body");
    }

    #[test]
    fn split_frontmatter_unterminated_fence_errors() {
        let raw = "---\nkey: val\nbody without closing fence";
        let err = split_frontmatter(raw).unwrap_err();
        assert!(
            err.contains("unterminated"),
            "expected unterminated message, got: {err}"
        );
    }

    #[test]
    fn split_frontmatter_malformed_yaml_errors() {
        let raw = "---\n: : not: valid: yaml\n---\nbody";
        assert!(split_frontmatter(raw).is_err());
    }

    #[test]
    fn split_frontmatter_first_closing_fence_terminates() {
        // A second `---` in the body must not re-enter frontmatter.
        let raw = "---\nkey: val\n---\nbody line\n---\nmore body";
        let (fm, body) = split_frontmatter(raw).unwrap();
        assert_eq!(fm.get("key"), Some(&json!("val")));
        assert_eq!(body, "body line\n---\nmore body");
    }

    // -----------------------------------------------------------------------
    // find_h1
    // -----------------------------------------------------------------------

    #[rstest]
    #[case::simple("# Title", Some("Title"))]
    #[case::trailing_whitespace("# Title   ", Some("Title"))]
    #[case::leading_blank_line("\n# Title", Some("Title"))]
    #[case::after_other_text("preamble\n# Title", Some("Title"))]
    #[case::no_h1("body without heading", None)]
    #[case::only_h2("## sub-heading", None)]
    #[case::missing_space_after_hash("#NoSpace", None)]
    #[case::empty("", None)]
    #[case::multiple_h1s_picks_first("# First\n# Second", Some("First"))]
    fn find_h1_cases(#[case] input: &str, #[case] expected: Option<&str>) {
        assert_eq!(find_h1(input).as_deref(), expected);
    }

    // -----------------------------------------------------------------------
    // find_description
    // -----------------------------------------------------------------------

    #[test]
    fn find_description_returns_paragraph_after_h1() {
        let body = "# Title\n\nA description paragraph.\n\n## Section\n";
        assert_eq!(
            find_description(body).as_deref(),
            Some("A description paragraph.")
        );
    }

    #[test]
    fn find_description_skips_blank_lines_between_h1_and_text() {
        let body = "# Title\n\n\n\nThe description.\n\n## Section";
        assert_eq!(find_description(body).as_deref(), Some("The description."));
    }

    #[test]
    fn find_description_collects_multiline_paragraph_until_blank() {
        let body = "# Title\n\nLine one.\nLine two.\n\nLine three (separate paragraph).";
        assert_eq!(
            find_description(body).as_deref(),
            Some("Line one.\nLine two.")
        );
    }

    #[test]
    fn find_description_breaks_at_section_heading() {
        let body = "# Title\n\nDescription.\n## Section";
        assert_eq!(find_description(body).as_deref(), Some("Description."));
    }

    #[test]
    fn find_description_returns_none_when_no_h1() {
        let body = "Text without heading.";
        assert_eq!(find_description(body), None);
    }

    #[test]
    fn find_description_returns_none_when_only_h1() {
        assert_eq!(find_description("# Title\n").as_deref(), None);
    }

    #[test]
    fn find_description_returns_none_when_only_h1_then_section() {
        assert_eq!(find_description("# Title\n## Section").as_deref(), None);
    }

    // -----------------------------------------------------------------------
    // parse_sections
    // -----------------------------------------------------------------------

    #[test]
    fn parse_sections_empty_body_returns_empty_map() {
        assert!(parse_sections("").is_empty());
    }

    #[test]
    fn parse_sections_no_h2_returns_empty_map() {
        assert!(parse_sections("# Title\n\nDescription.").is_empty());
    }

    #[test]
    fn parse_sections_single_section() {
        let body = "## Notes\n\ncontent line";
        let sections = parse_sections(body);
        assert_eq!(
            sections.get("Notes").map(String::as_str),
            Some("content line")
        );
    }

    #[test]
    fn parse_sections_multiple_sections() {
        let body = "## A\n\nbody A\n\n## B\n\nbody B";
        let sections = parse_sections(body);
        assert_eq!(sections.get("A").map(String::as_str), Some("body A"));
        assert_eq!(sections.get("B").map(String::as_str), Some("body B"));
    }

    #[test]
    fn parse_sections_section_with_multiline_body() {
        let body = "## Notes\n\nline 1\nline 2\nline 3";
        let sections = parse_sections(body);
        assert_eq!(
            sections.get("Notes").map(String::as_str),
            Some("line 1\nline 2\nline 3")
        );
    }

    #[test]
    fn parse_sections_duplicate_heading_keeps_last() {
        // HashMap insert overwrites — second occurrence wins.
        let body = "## A\n\nfirst\n\n## A\n\nsecond";
        let sections = parse_sections(body);
        assert_eq!(sections.get("A").map(String::as_str), Some("second"));
    }

    #[test]
    fn parse_sections_ignores_content_before_first_heading() {
        let body = "# Title\n\npreamble that should be ignored\n\n## A\n\nbody A";
        let sections = parse_sections(body);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections.get("A").map(String::as_str), Some("body A"));
    }

    #[test]
    fn parse_sections_trims_section_body_whitespace() {
        let body = "## A\n\n\n  content with surrounding blanks\n\n";
        let sections = parse_sections(body);
        assert_eq!(
            sections.get("A").map(String::as_str),
            Some("content with surrounding blanks")
        );
    }

    // -----------------------------------------------------------------------
    // parse_bullet_list
    // -----------------------------------------------------------------------

    #[test]
    fn parse_bullet_list_empty_body() {
        assert!(parse_bullet_list("").is_empty());
    }

    #[test]
    fn parse_bullet_list_single_item() {
        assert_eq!(parse_bullet_list("- one"), vec![json!("one")]);
    }

    #[test]
    fn parse_bullet_list_multiple_items() {
        assert_eq!(
            parse_bullet_list("- one\n- two\n- three"),
            vec![json!("one"), json!("two"), json!("three")]
        );
    }

    #[test]
    fn parse_bullet_list_drops_non_bullet_lines() {
        // The parser only collects lines beginning with `- `; everything
        // else (headings, prose, blank lines) is silently skipped.
        let body = "preamble\n- a\n\n- b\nrandom text\n- c";
        assert_eq!(
            parse_bullet_list(body),
            vec![json!("a"), json!("b"), json!("c")]
        );
    }

    #[test]
    fn parse_bullet_list_requires_space_after_dash() {
        // `-foo` is not a bullet entry; dropped.
        assert!(parse_bullet_list("-foo").is_empty());
    }

    #[test]
    fn parse_bullet_list_preserves_inner_whitespace() {
        // Strip is only the literal `- ` prefix; trailing/leading
        // whitespace on the item is kept verbatim.
        assert_eq!(
            parse_bullet_list("-   spaced item   "),
            vec![json!("  spaced item   ")]
        );
    }

    // -----------------------------------------------------------------------
    // claimed_top_level_keys
    // -----------------------------------------------------------------------

    use crate::substrate::pipeline::FlattenRule;

    fn fields(entries: &[(&'static str, RepoSlot)]) -> Vec<FieldMapping<RepoSlot>> {
        entries
            .iter()
            .map(|(k, s)| FieldMapping { key: k, slot: *s })
            .collect()
    }

    #[test]
    fn claimed_keys_includes_entity_ref_implicitly() {
        // `entity_ref` is always claimed even when no field declares
        // it — every entity has one and the codec must not absorb it
        // into a flatten slot.
        let s = fields(&[]);
        assert!(claimed_top_level_keys(&s).contains("entity_ref"));
    }

    #[test]
    fn claimed_keys_includes_simple_field_keys() {
        let s = fields(&[
            ("name", RepoSlot::H1),
            ("purpose", RepoSlot::FrontmatterKey("purpose")),
        ]);
        let claimed = claimed_top_level_keys(&s);
        assert!(claimed.contains("name"));
        assert!(claimed.contains("purpose"));
    }

    #[test]
    fn claimed_keys_dot_path_uses_first_segment() {
        // Task's `artifact.kind` claims top-level `artifact` so the
        // flatten slot doesn't try to absorb `artifact` as an
        // extension key.
        let s = fields(&[("artifact.kind", RepoSlot::FrontmatterKey("artifact"))]);
        let claimed = claimed_top_level_keys(&s);
        assert!(claimed.contains("artifact"));
        assert!(!claimed.contains("artifact.kind"));
    }

    #[test]
    fn claimed_keys_skips_flatten_slots() {
        // Flatten slots are catch-alls for unclaimed keys — their
        // own `key` ("extensions") shouldn't be in the claimed set
        // (it's a struct-field handle, not a wire key).
        let s = fields(&[
            ("name", RepoSlot::H1),
            (
                "extensions",
                RepoSlot::FrontmatterFlattened(FlattenRule::Prefix("x-")),
            ),
        ]);
        let claimed = claimed_top_level_keys(&s);
        assert!(claimed.contains("name"));
        assert!(!claimed.contains("extensions"));
    }

    // -----------------------------------------------------------------------
    // best_flatten_match — longest-prefix-match across both targets
    // -----------------------------------------------------------------------

    #[test]
    fn best_flatten_match_returns_none_when_no_flatten_slots() {
        let s = fields(&[("name", RepoSlot::H1)]);
        assert!(best_flatten_match(&s, "x-foo").is_none());
    }

    #[test]
    fn best_flatten_match_returns_none_when_no_rule_matches() {
        let s = fields(&[(
            "extensions",
            RepoSlot::FrontmatterFlattened(FlattenRule::Prefix("x-")),
        )]);
        assert!(best_flatten_match(&s, "rogue").is_none());
    }

    #[test]
    fn best_flatten_match_picks_only_matching_slot() {
        let s = fields(&[(
            "extensions",
            RepoSlot::FrontmatterFlattened(FlattenRule::Prefix("x-")),
        )]);
        assert!(matches!(
            best_flatten_match(&s, "x-color"),
            Some(RepoSlot::FrontmatterFlattened(_))
        ));
    }

    #[test]
    fn best_flatten_match_longest_prefix_wins_across_targets() {
        // Same asset declares both a frontmatter and a section
        // flatten slot. `x-doc-rationale` matches both (the section
        // prefix is more specific) — the section wins.
        let s = fields(&[
            (
                "extensions",
                RepoSlot::FrontmatterFlattened(FlattenRule::Prefix("x-")),
            ),
            (
                "extensions",
                RepoSlot::SectionFlattened(
                    FlattenRule::Prefix("x-doc-"),
                    SectionContent::Paragraph,
                ),
            ),
        ]);
        assert!(matches!(
            best_flatten_match(&s, "x-doc-rationale"),
            Some(RepoSlot::SectionFlattened(_, _))
        ));
        // The non-doc key only matches the general rule.
        assert!(matches!(
            best_flatten_match(&s, "x-color"),
            Some(RepoSlot::FrontmatterFlattened(_))
        ));
    }

    // -----------------------------------------------------------------------
    // best_flatten_target_match — decode-side, restricted to a target
    // -----------------------------------------------------------------------

    #[test]
    fn target_match_frontmatter_only_considers_frontmatter_slots() {
        // Both flatten slots match `x-doc-rationale` by prefix, but a
        // frontmatter entry on disk can only ever come from the
        // frontmatter slot — the section slot must be invisible.
        let s = fields(&[
            (
                "extensions",
                RepoSlot::FrontmatterFlattened(FlattenRule::Prefix("x-")),
            ),
            (
                "extensions",
                RepoSlot::SectionFlattened(
                    FlattenRule::Prefix("x-doc-"),
                    SectionContent::Paragraph,
                ),
            ),
        ]);
        assert!(matches!(
            best_flatten_target_match(&s, "x-doc-rationale", FlattenTarget::Frontmatter),
            Some(RepoSlot::FrontmatterFlattened(_))
        ));
    }

    #[test]
    fn target_match_section_only_considers_section_slots() {
        let s = fields(&[
            (
                "extensions",
                RepoSlot::FrontmatterFlattened(FlattenRule::Prefix("x-")),
            ),
            (
                "extensions",
                RepoSlot::SectionFlattened(
                    FlattenRule::Prefix("x-doc-"),
                    SectionContent::Paragraph,
                ),
            ),
        ]);
        // A section heading "x-color" doesn't match any
        // SectionFlattened rule (the only one wants `x-doc-`); the
        // FrontmatterFlattened rule doesn't apply to sections.
        assert!(best_flatten_target_match(&s, "x-color", FlattenTarget::Section).is_none());
        // `x-doc-rationale` as a section heading DOES match.
        assert!(matches!(
            best_flatten_target_match(&s, "x-doc-rationale", FlattenTarget::Section),
            Some(RepoSlot::SectionFlattened(_, _))
        ));
    }

    #[test]
    fn target_match_no_flatten_slots_returns_none() {
        let s = fields(&[("name", RepoSlot::H1)]);
        assert!(best_flatten_target_match(&s, "x-foo", FlattenTarget::Frontmatter).is_none());
        assert!(best_flatten_target_match(&s, "x-foo", FlattenTarget::Section).is_none());
    }
}
