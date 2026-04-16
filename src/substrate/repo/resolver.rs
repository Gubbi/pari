use std::path::{Path, PathBuf};

use crate::{
    entity::EntityKind,
    substrate::{
        pipeline::LocationResolver, repo::substrate::RepoSubstrate,
        schema_registry::SchemaBackedSubstrate,
    },
};

pub struct RepoLocationResolver {
    root: PathBuf,
}

impl RepoLocationResolver {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl LocationResolver for RepoLocationResolver {
    type Location = PathBuf;

    fn resolve(&self, path_template: &str, entity_json: &serde_json::Value) -> Self::Location {
        let mut resolved = path_template.to_string();
        let id = entity_json["entity_ref"]["id"].as_str().unwrap_or_default();
        resolved = resolved.replace("{id}", id);
        if resolved.contains("{parent.base}") {
            let parent_base = parent_base(&entity_json["entity_ref"]["parent"]);
            resolved = resolved.replace("{parent.base}", &parent_base);
        }
        self.root.join(resolved)
    }

    fn base_of(&self, location: &Self::Location) -> String {
        location
            .strip_prefix(&self.root)
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

fn parent_base(parent: &serde_json::Value) -> String {
    let Some(kind) = parent["kind"].as_str().and_then(EntityKind::from_str) else {
        return String::new();
    };

    let template = <RepoSubstrate as SchemaBackedSubstrate>::schema_for(kind)
        .ref_asset
        .path_template;
    let base_template = template
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or(template);
    let mut resolved = base_template.to_string();
    let id = parent["id"].as_str().unwrap_or_default();
    resolved = resolved.replace("{id}", id);
    if resolved.contains("{parent.base}") {
        resolved = resolved.replace("{parent.base}", &parent_base(&parent["parent"]));
    }
    resolved
}
