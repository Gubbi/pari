use crate::substrate::pipeline::LocationResolver;

pub struct InMemoryResolver;

impl LocationResolver for InMemoryResolver {
    type Location = String;

    fn resolve(&self, path_template: &str, entity_json: &serde_json::Value) -> Self::Location {
        let mut resolved = path_template.to_string();
        let id = entity_json["entity_ref"]["id"].as_str().unwrap_or_default();
        resolved = resolved.replace("{id}", id);
        if resolved.contains("{parent.base}") {
            let parent_base = parent_base(&entity_json["entity_ref"]["parent"]);
            resolved = resolved.replace("{parent.base}", &parent_base);
        }
        resolved
    }

    fn base_of(&self, location: &Self::Location) -> String {
        location
            .rsplit_once('/')
            .map(|(base, _)| base.to_string())
            .unwrap_or_default()
    }
}

fn parent_base(parent: &serde_json::Value) -> String {
    let id = parent["id"].as_str().unwrap_or_default();
    let kind = parent["kind"].as_str().unwrap_or_default();
    match kind {
        "Workflow" => format!("workflows/{id}"),
        "ReusableWorkflow" => format!("common/workflows/{id}"),
        "EmbeddedWorkflow" => {
            let base = parent_base(&parent["parent"]);
            format!("{base}/{id}")
        }
        _ => String::new(),
    }
}
