# read-path

**Owning layer: `substrate`**

---

## Purpose

The `load` and `exists` default implementations on the `Substrate` trait form the read pipeline. They use `self.resolver()`, `self.codec()`, and `self.executor()` directly — no separate Orchestrator struct. AssetMapper and EntitySchema are substrate-layer internals that these implementations use to drive the pipeline.

---

## Pipelines

```
load:                 AssetMapper → self.resolver() → self.executor() → self.codec() → TrackedEntity
exists:               AssetMapper → self.resolver() → self.executor() → Vec<bool>
```

---

## exists (HEAD pipeline)

```rust
async fn exists(&self, refs: &[AnyEntityRef]) -> Result<Vec<bool>, SubstrateError> {
    let requests: Vec<_> = refs.iter()
        .map(|any_ref| {
            let schema = Self::schema_for(any_ref.kind());
            let stub_json = any_ref_to_stub_json(any_ref);
            let location = self.resolver().resolve(schema.ref_asset.path_template, &stub_json);
            AssetRequest { location, op: AssetOp::Head }
        })
        .collect();

    let responses = self.executor().execute(requests)?;

    responses.iter().map(|r| match r {
        AssetResponse::Exists(b) => Ok(*b),
        _ => unreachable!(),
    }).collect()
}
```

Only the `ref_asset` is checked — if the primary asset exists, the entity exists.

---

## load (GET pipeline)

```rust
async fn load(&self, entity: &TrackedEntity, fields: &[&str]) -> Result<TrackedEntity, SubstrateError> {
    let schema = Self::schema_for(entity.kind());
    let entity_json = serde_json::to_value(entity)?;

    // Select assets covering the requested fields (all assets if fields is empty)
    let assets_to_fetch = AssetMapper::select_for_read(schema, fields);

    let requests: Vec<_> = assets_to_fetch.iter()
        .map(|asset| {
            let location = self.resolver().resolve(asset.path_template, &entity_json);
            AssetRequest { location, op: AssetOp::Get }
        })
        .collect();

    let responses = self.executor().execute(requests)?;

    // Decode each response and merge into a partial TrackedEntity
    let mut result = TrackedEntity::stub_from(entity);
    for (asset, response) in assets_to_fetch.iter().zip(responses) {
        let encoded = match response { AssetResponse::Data(e) => e, _ => unreachable!() };
        let field_map = self.codec().decode(&encoded, asset.fields)?;
        merge_json_into(&mut result, field_map);
    }

    Ok(result)
}
```

`fields = &[]` means "fetch all" — all assets in the schema are fetched. `fields = &["name"]` fetches only the asset(s) that contain `name`.

---

## Asset Selection

When `fields` is non-empty, the minimal set of assets covering the requested fields is selected:

```
requested: ["template_content"]
  → template_content is in AssetDef (the template file)
  → fetch that asset only
  → ref_asset not fetched (name, purpose, etc. not requested)
```
