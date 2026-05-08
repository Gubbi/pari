use std::{fs, path::PathBuf};

use pari::entity::entities::{
    artifact_kind::ArtifactKind,
    hook::Hook,
    relay::Relay,
    role::Role,
    task::Task,
    team::Team,
    workflow::{EmbeddedWorkflow, ReusableWorkflow, Workflow},
};
use schemars::{schema_for, JsonSchema};

fn write_schema<T: JsonSchema>(schemas_dir: &PathBuf, filename: &str) {
    let schema = schema_for!(T);
    let json = serde_json::to_string_pretty(&schema).expect("failed to serialize schema");
    let path = schemas_dir.join(filename);
    let next = json + "\n";

    if fs::read_to_string(&path).ok().as_deref() == Some(next.as_str()) {
        println!("  unchanged {filename}");
        return;
    }

    fs::write(&path, next).unwrap_or_else(|e| panic!("failed to write {filename}: {e}"));
    println!("  wrote {filename}");
}

fn prune_stale_files(schemas_dir: &PathBuf, keep: &[&str]) {
    let entries = fs::read_dir(schemas_dir).expect("failed to read schemas dir");
    for entry in entries {
        let entry = entry.expect("failed to read schemas entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if keep.contains(&filename) {
            continue;
        }

        fs::remove_file(&path).unwrap_or_else(|e| panic!("failed to remove {filename}: {e}"));
        println!("  removed stale {filename}");
    }
}

fn generate_schemas() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schemas_dir = manifest_dir.parent().unwrap().join("schemas");
    let schema_files = [
        "artifact_kind.json",
        "hook.json",
        "role.json",
        "team.json",
        "workflow.json",
        "reusable_workflow.json",
        "task.json",
        "relay.json",
        "embedded_workflow.json",
    ];

    fs::create_dir_all(&schemas_dir).expect("failed to create schemas dir");

    println!("Generating schemas to {}", schemas_dir.display());
    prune_stale_files(&schemas_dir, &schema_files);

    write_schema::<ArtifactKind>(&schemas_dir, "artifact_kind.json");
    write_schema::<Hook>(&schemas_dir, "hook.json");
    write_schema::<Role>(&schemas_dir, "role.json");
    write_schema::<Team>(&schemas_dir, "team.json");
    write_schema::<Workflow>(&schemas_dir, "workflow.json");
    write_schema::<ReusableWorkflow>(&schemas_dir, "reusable_workflow.json");
    write_schema::<Task>(&schemas_dir, "task.json");
    write_schema::<Relay>(&schemas_dir, "relay.json");
    write_schema::<EmbeddedWorkflow>(&schemas_dir, "embedded_workflow.json");

    println!("Done.");
}

/// Structural invariants every entity schema must satisfy at the top
/// level. Closes the entity to unknown bare keys while still admitting
/// `x-` prefixed extensions. Independent of whether the on-disk file
/// matches the runtime — drift is the `git diff` step's job.
fn check_schemas() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schemas_dir = manifest_dir.parent().unwrap().join("schemas");
    let schema_files = [
        "artifact_kind.json",
        "embedded_workflow.json",
        "hook.json",
        "relay.json",
        "reusable_workflow.json",
        "role.json",
        "task.json",
        "team.json",
        "workflow.json",
    ];

    println!("Checking schemas in {}", schemas_dir.display());

    let mut failures: Vec<String> = Vec::new();

    for filename in schema_files {
        let path = schemas_dir.join(filename);
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                failures.push(format!("{filename}: read failed: {e}"));
                continue;
            }
        };
        let schema: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{filename}: parse failed: {e}"));
                continue;
            }
        };

        match schema.get("additionalProperties") {
            Some(serde_json::Value::Bool(false)) => {}
            Some(other) => failures.push(format!(
                "{filename}: top-level additionalProperties must be false, got {other}"
            )),
            None => failures.push(format!(
                "{filename}: missing top-level additionalProperties"
            )),
        }

        match schema
            .get("patternProperties")
            .and_then(serde_json::Value::as_object)
        {
            Some(pp) if pp.contains_key("^x-") => {}
            Some(pp) => failures.push(format!(
                "{filename}: top-level patternProperties must include '^x-', got keys {:?}",
                pp.keys().collect::<Vec<_>>()
            )),
            None => failures.push(format!("{filename}: missing top-level patternProperties")),
        }

        println!("  checked {filename}");
    }

    if !failures.is_empty() {
        eprintln!("\nSchema check failed:");
        for f in &failures {
            eprintln!("  - {f}");
        }
        std::process::exit(1);
    }

    println!("Done.");
}

fn main() {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "generate-schemas".to_string());

    match command.as_str() {
        "generate-schemas" => generate_schemas(),
        "check-schemas" => check_schemas(),
        _ => panic!("unknown xtask command: {command}"),
    }
}
