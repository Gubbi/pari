//! `RepoLocationResolver` — expand path templates into absolute `PathBuf` locations.

use std::path::PathBuf;
use crate::substrate::pipeline::LocationResolver;

#[derive(Clone)]
pub struct RepoLocationResolver {
    root: PathBuf,
}

impl RepoLocationResolver {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Returns the parent directory of a location as a `String`.
    pub fn base_of(location: &std::path::Path) -> String {
        location
            .parent()
            .unwrap_or(location)
            .to_string_lossy()
            .into_owned()
    }
}

impl LocationResolver for RepoLocationResolver {
    type Location = PathBuf;

    fn resolve(&self, path_template: &str, entity: &serde_json::Value) -> PathBuf {
        let expanded = expand_template(path_template, entity);
        self.root.join(expanded)
    }
}

/// Expand `{id}`, `{parent.base}`, and `{field.subfield}` placeholders.
///
/// | Placeholder      | Resolution                                                             |
/// |------------------|------------------------------------------------------------------------|
/// | `{id}`           | `entity["entity_ref"]["id"]`                                           |
/// | `{parent.base}`  | `workflows/<workflow_id>` derived from `entity["entity_ref"]["workflow_id"]` |
/// | `{field.sub}`    | `entity["field"]["sub"]`                                               |
fn expand_template(template: &str, entity: &serde_json::Value) -> String {
    let mut result = template.to_string();

    // Replace {id}
    if let Some(id) = entity
        .get("entity_ref")
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
    {
        result = result.replace("{id}", id);
    }

    // Replace {parent.base} — resolves to workflows/<workflow_id> for embedded entities
    if result.contains("{parent.base}") {
        if let Some(workflow_id) = entity
            .get("entity_ref")
            .and_then(|r| r.get("workflow_id"))
            .and_then(|v| v.as_str())
        {
            let parent_base = format!("workflows/{}", workflow_id);
            result = result.replace("{parent.base}", &parent_base);
        }
    }

    // Replace remaining {field.subfield} patterns
    while let Some(start) = result.find('{') {
        let Some(end) = result[start..].find('}') else { break };
        let placeholder = &result[start + 1..start + end].to_string();
        let parts: Vec<&str> = placeholder.split('.').collect();
        let mut val = entity;
        let mut resolved = None;
        for part in &parts {
            match val.get(part) {
                Some(v) => val = v,
                None => break,
            }
            resolved = val.as_str();
        }
        if let Some(s) = resolved {
            result.replace_range(start..start + end + 1, s);
        } else {
            break;
        }
    }

    result
}
