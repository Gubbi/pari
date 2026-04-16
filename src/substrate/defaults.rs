use crate::entity::{AnyEntityRef, EntityKind, TrackedEntity};
use crate::store::EntityChange;
use crate::substrate::schema_registry::SchemaBackedSubstrate;
use crate::substrate::serde::{
    any_ref_to_stub_json, entity_to_json, merge_field_map_into,
};
use crate::substrate::pipeline::{Codec, Executor, LocationResolver};
use crate::substrate::{pipeline, SubstrateError};
use std::collections::HashSet;

pub(crate) fn load_strategy<Sub>(
    entity_kind: EntityKind,
    field: &str,
) -> Result<pipeline::LoadStrategy, SubstrateError>
where
    Sub: SchemaBackedSubstrate,
{
    Sub::schema_for(entity_kind)
        .load_strategy_for(field)
        .map_err(SubstrateError::from)
}

pub(crate) async fn exists<Sub>(substrate: &Sub, refs: &[AnyEntityRef]) -> Result<Vec<bool>, SubstrateError>
where
    Sub: SchemaBackedSubstrate,
{
    let requests = refs
        .iter()
        .map(|any_ref| {
            let schema = Sub::schema_for(any_ref.kind());
            let stub_json = any_ref_to_stub_json(any_ref);
            let location = substrate
                .resolver()
                .resolve(schema.ref_asset.path_template, &stub_json);
            pipeline::AssetRequest {
                location,
                op: pipeline::AssetOp::Head,
            }
        });

    let responses = substrate
        .executor()
        .execute(requests)
        .map_err(collapse_executor_errors)?;

    responses
        .into_iter()
        .map(|response| match response {
            pipeline::AssetResponse::Exists(value) => Ok(value),
            _ => unreachable!(),
        })
        .collect()
}

pub(crate) async fn load<Sub>(
    substrate: &Sub,
    entity: &TrackedEntity,
    fields: &[&str],
) -> Result<TrackedEntity, SubstrateError>
where
    Sub: SchemaBackedSubstrate,
{
    let schema = Sub::schema_for(entity.any_ref().kind());
    let entity_json = entity_to_json(entity)?;
    let assets_to_fetch = pipeline::AssetMapper::select_for_read(schema, fields)
        .map_err(SubstrateError::from)?;

    let requests = assets_to_fetch
        .iter()
        .map(|asset| {
            let location = substrate.resolver().resolve(asset.path_template(), &entity_json);
            pipeline::AssetRequest {
                location,
                op: pipeline::AssetOp::Get,
            }
        });

    let responses = substrate
        .executor()
        .execute(requests)
        .map_err(collapse_executor_errors)?;

    let mut result = entity.clone();
    for (asset, response) in assets_to_fetch.iter().zip(responses.into_iter()) {
        let encoded = match response {
            pipeline::AssetResponse::Data(encoded) => encoded,
            _ => unreachable!(),
        };
        let field_map = substrate
            .codec()
            .decode(&encoded, asset.fields())
            .map_err(SubstrateError::from)?;
        merge_field_map_into(&mut result, field_map)?;
    }

    Ok(result)
}

pub(crate) async fn persist<'a, Sub>(
    substrate: &'a Sub,
    changes: impl Iterator<Item = EntityChange<'a>> + Send + 'a,
) -> Result<(), Vec<SubstrateError>>
where
    Sub: SchemaBackedSubstrate,
{
    let mut ops = Vec::new();
    let mut errors = Vec::new();

    for change in changes {
        match change {
            EntityChange::Removed(any_ref) => {
                let schema = Sub::schema_for(any_ref.kind());
                let stub_json = any_ref_to_stub_json(any_ref);
                for asset in delete_assets(schema) {
                    let location = substrate
                        .resolver()
                        .resolve(asset.path_template(), &stub_json);
                    ops.push(pipeline::AssetRequest {
                        location,
                        op: pipeline::AssetOp::Delete,
                    });
                }
            }
            EntityChange::Added(entity) => {
                let schema = Sub::schema_for(entity.any_ref().kind());
                if let Err(mut es) = build_write_ops(substrate, entity, schema, None, &mut ops) {
                    errors.append(&mut es);
                }
            }
            EntityChange::Modified(entity, dirty_fields) => {
                let schema = Sub::schema_for(entity.any_ref().kind());
                if let Err(mut es) =
                    build_write_ops(substrate, entity, schema, Some(dirty_fields.as_slice()), &mut ops)
                {
                    errors.append(&mut es);
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    substrate
        .executor()
        .execute(ops)
        .map(|_| ())
        .map_err(|errs| errs.into_iter().map(SubstrateError::from).collect())
}

fn build_write_ops<'a, Sub>(
    substrate: &Sub,
    entity: &TrackedEntity,
    schema: &'static pipeline::EntitySchema<Sub::Slot>,
    dirty_fields: Option<&'a [&'static str]>,
    ops: &mut Vec<pipeline::AssetRequest<Sub::Location, Sub::Encoded>>,
) -> Result<(), Vec<SubstrateError>>
where
    Sub: SchemaBackedSubstrate,
{
    let entity_json = entity_to_json(entity).map_err(|e| vec![e])?;
    let is_create = dirty_fields.is_none();

    for asset in pipeline::AssetMapper::select_for_write(schema, dirty_fields)
        .map_err(SubstrateError::from)
        .map_err(|e| vec![e])?
    {
        let location = substrate.resolver().resolve(asset.path_template(), &entity_json);
        let is_partial = dirty_fields
            .map(|dirty| asset_write_is_partial(asset.fields(), dirty))
            .unwrap_or(false);
        let encoded = substrate
            .codec()
            .encode(&entity_json, asset.fields())
            .map_err(SubstrateError::from)
            .map_err(|e| vec![e])?;
        ops.push(pipeline::AssetRequest {
            location,
            op: asset.kind().write_op(is_create, is_partial, encoded),
        });
    }

    Ok(())
}

fn delete_assets<'a, S: pipeline::Slot>(
    schema: &'a pipeline::EntitySchema<S>,
) -> Vec<pipeline::SchemaAsset<'a, S>> {
    schema.all_assets().collect()
}

fn asset_write_is_partial<S: pipeline::Slot>(
    fields: &[pipeline::FieldMapping<S>],
    dirty_fields: &[&str],
) -> bool {
    let asset_fields: HashSet<&str> = fields.iter().map(|field| field.key).collect();
    let dirty_fields: HashSet<&str> = dirty_fields.iter().copied().collect();
    let dirty_in_asset = asset_fields.intersection(&dirty_fields).count();
    dirty_in_asset < asset_fields.len()
}

fn collapse_executor_errors(
    errors: Vec<pipeline::ExecutorError>,
) -> SubstrateError {
    let error = errors
        .into_iter()
        .next()
        .unwrap_or_else(|| pipeline::ExecutorError::new("substrate", "executor batch failed"));
    SubstrateError::from(error)
}
