use std::fs;
use std::path::PathBuf;

use pari::schema::entities::hook::{Hook, HookInput};
use pari::schema::entities::relay::Relay;
use pari::schema::entities::role::Role;
use pari::schema::entities::task::Task;
use pari::schema::entities::team::{Team, TeamMember};
use pari::schema::entities::workflow::Workflow;
use pari::schema::types::{
    Artifact, HookInvocation, HookInvocationValue, Raci, RelayStateSemantic, StateMapEntry, Step,
    TaskSemantic, TaskStateEntry, WorkflowSemantic, WorkflowStateEntry, WorkStep, ReviewStep,
};
use schemars::{schema_for, JsonSchema};

fn write_schema<T: JsonSchema>(schemas_dir: &PathBuf, filename: &str) {
    let schema = schema_for!(T);
    let json = serde_json::to_string_pretty(&schema).expect("Failed to serialize schema");
    let path = schemas_dir.join(filename);
    fs::write(&path, json + "\n").unwrap_or_else(|e| panic!("Failed to write {filename}: {e}"));
    println!("  wrote {filename}");
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

    // Entity types
    write_schema::<Role>(&schemas_dir, "role.json");
    write_schema::<HookInput>(&schemas_dir, "hook_input.json");
    write_schema::<Hook>(&schemas_dir, "hook.json");
    write_schema::<TeamMember>(&schemas_dir, "team_member.json");
    write_schema::<Team>(&schemas_dir, "team.json");
    write_schema::<Workflow>(&schemas_dir, "workflow.json");
    write_schema::<Task>(&schemas_dir, "task.json");
    write_schema::<Relay>(&schemas_dir, "relay.json");

    println!("Done.");
}
