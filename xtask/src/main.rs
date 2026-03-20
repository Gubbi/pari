use std::{fs, path::PathBuf};

use pari::schema::{
    entities::{
        hook::{Hook, HookInput},
        role::Role,
        team::{Team, TeamMember},
        workflow::{ReviewStep, SharedWorkflow, Step, WorkStep, WorkStepDefinition, Workflow},
    },
    types::{
        Artifact, HookInvocation, HookInvocationValue, Raci, RelayStateSemantic, StateMapEntry,
        TaskSemantic, TaskStateEntry, WorkflowSemantic, WorkflowStateEntry,
    },
};
use schemars::{schema_for, JsonSchema};

fn write_schema<T: JsonSchema>(schemas_dir: &PathBuf, filename: &str) {
    let schema = schema_for!(T);
    let json = serde_json::to_string_pretty(&schema).expect("Failed to serialize schema");
    let path = schemas_dir.join(filename);
    fs::write(&path, json + "\n").unwrap_or_else(|e| panic!("Failed to write {filename}: {e}"));
    println!("  wrote {filename}");
}

/// Post-process entity schemas: add `additionalProperties: false` to any schema that has
/// `patternProperties`. schemars 0.8 does not emit `additionalProperties: false` when a
/// struct has a `#[serde(flatten)]` field, even with `deny_unknown_fields`. This ensures
/// JSON Schema validators correctly reject unknown keys that don't match `^x-`.
fn add_additional_properties_false(schemas_dir: &PathBuf, filename: &str) {
    let path = schemas_dir.join(filename);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {filename}: {e}"));
    let mut schema: serde_json::Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));

    if schema.get("patternProperties").is_some() {
        schema["additionalProperties"] = serde_json::json!(false);
        let json = serde_json::to_string_pretty(&schema)
            .unwrap_or_else(|e| panic!("Failed to serialize {filename}: {e}"));
        fs::write(&path, json + "\n").unwrap_or_else(|e| panic!("Failed to write {filename}: {e}"));
        println!("  post-processed {filename} (additionalProperties: false)");
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schemas_dir = manifest_dir.parent().unwrap().join("schemas");

    fs::create_dir_all(&schemas_dir).expect("Failed to create schemas dir");

    println!("Generating schemas to {}", schemas_dir.display());

    // Shared types
    write_schema::<Raci>(&schemas_dir, "raci.json");
    write_schema::<HookInvocation>(&schemas_dir, "hook_invocation.json");
    write_schema::<HookInvocationValue>(&schemas_dir, "hooks_map_value.json");
    write_schema::<WorkStepDefinition>(&schemas_dir, "work_step_definition.json");
    write_schema::<WorkStep>(&schemas_dir, "work_step.json");
    write_schema::<ReviewStep>(&schemas_dir, "review_step.json");
    write_schema::<Step>(&schemas_dir, "step.json");
    write_schema::<WorkflowSemantic>(&schemas_dir, "workflow_semantic.json");
    write_schema::<TaskSemantic>(&schemas_dir, "task_semantic.json");
    write_schema::<RelayStateSemantic>(&schemas_dir, "relay_state_semantic.json");
    write_schema::<WorkflowStateEntry>(&schemas_dir, "workflow_state_entry.json");
    write_schema::<TaskStateEntry>(&schemas_dir, "task_state_entry.json");
    write_schema::<StateMapEntry>(&schemas_dir, "state_map_entry.json");
    write_schema::<Artifact>(&schemas_dir, "artifact.json");

    // Entity types (Task and Relay are embedded-only; no standalone schema generated)
    write_schema::<Role>(&schemas_dir, "role.json");
    write_schema::<HookInput>(&schemas_dir, "hook_input.json");
    write_schema::<Hook>(&schemas_dir, "hook.json");
    write_schema::<TeamMember>(&schemas_dir, "team_member.json");
    write_schema::<Team>(&schemas_dir, "team.json");
    write_schema::<Workflow>(&schemas_dir, "workflow.json");
    write_schema::<SharedWorkflow>(&schemas_dir, "shared_workflow.json");

    // Post-process entity schemas: schemars 0.8 does not emit additionalProperties: false
    // alongside patternProperties when flatten fields are present. Add it explicitly.
    println!("Post-processing entity schemas...");
    for filename in &[
        "role.json",
        "hook.json",
        "team.json",
        "workflow.json",
        "shared_workflow.json",
    ] {
        add_additional_properties_false(&schemas_dir, filename);
    }

    println!("Done.");
}
