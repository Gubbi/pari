use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/entity");
    println!("cargo:rerun-if-changed=xtask/src/main.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=xtask/Cargo.toml");
    println!("cargo:rerun-if-changed=pari-macros/src");

    if env::var_os("PARI_SKIP_SCHEMA_BUILD").is_some()
        || env::var_os("CARGO_FEATURE_SKIP_SCHEMA_BUILD").is_some()
    {
        return;
    }

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let schema_target_dir = manifest_dir.join("target/schema-build");

    let status = Command::new(cargo)
        .current_dir(&manifest_dir)
        .env("PARI_SKIP_SCHEMA_BUILD", "1")
        .env("CARGO_TARGET_DIR", &schema_target_dir)
        .args(["run", "-p", "xtask", "--", "generate-schemas"])
        .status()
        .expect("failed to run schema generator");

    assert!(status.success(), "schema generation failed");
}
