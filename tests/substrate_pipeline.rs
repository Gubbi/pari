#![cfg(any())]
// TODO: Re-enable after the substrate API settles around the current exists()/persist() contract and we decide the long-term pipeline test shape.

use pari::{
    entities::role::Role,
    entity::{AnyEntityRef, EntityRef},
    store::EntityChange,
    substrate::{
        pipeline::{
            AssetDef, EntitySchema, FieldMapping, RefAssetDef, Slot, MARKDOWN_FILE, RAW_FILE,
        },
        Substrate, VoidSubstrate,
    },
};

// ---------------------------------------------------------------------------
// VoidSubstrate
// ---------------------------------------------------------------------------

#[tokio::test]
async fn void_substrate_exists_returns_false() {
    let sub = VoidSubstrate;
    let r: EntityRef<Role> = EntityRef::new("eng-lead");
    let any = AnyEntityRef::Role(r);
    assert!(!sub.exists(&any).await.unwrap());
}

#[tokio::test]
async fn void_substrate_persist_succeeds() {
    let sub = VoidSubstrate;
    let changes: &[EntityChange<'_>] = &[];
    sub.atomic_persist(changes).await.unwrap();
}

// ---------------------------------------------------------------------------
// EntitySchema::load_strategy_for
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum TestSlot {
    H1,
    FrontmatterKey(&'static str),
    FileContent,
}
impl Slot for TestSlot {}

#[test]
fn load_strategy_for_ref_asset_field() {
    let schema: EntitySchema<TestSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "roles/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping {
                    key: "name",
                    slot: TestSlot::H1,
                },
                FieldMapping {
                    key: "purpose",
                    slot: TestSlot::FrontmatterKey("purpose"),
                },
            ],
        },
        assets: &[],
    };
    let strategy = schema.load_strategy_for("name");
    assert!(strategy.prerequisites.is_empty());
    assert!(
        !strategy.mutable_without_load,
        "ref_asset field must load first"
    );
}

#[test]
fn load_strategy_for_single_field_asset() {
    let schema: EntitySchema<TestSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "tasks/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[FieldMapping {
                key: "name",
                slot: TestSlot::H1,
            }],
        },
        assets: &[AssetDef {
            path_template: "tasks/{id}/template.md",
            kind: &RAW_FILE,
            fields: &[FieldMapping {
                key: "template_content",
                slot: TestSlot::FileContent,
            }],
            path_deps: &[],
        }],
    };
    let strategy = schema.load_strategy_for("template_content");
    assert!(
        strategy.mutable_without_load,
        "single-field asset should be mutable without load"
    );
}

#[test]
fn load_strategy_for_unknown_field_is_mutable_without_load() {
    let schema: EntitySchema<TestSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "x/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[],
        },
        assets: &[],
    };
    let strategy = schema.load_strategy_for("nonexistent_field");
    assert!(strategy.mutable_without_load);
    assert!(strategy.prerequisites.is_empty());
}
