//! `entity_registry!` — the one macro invocation that fans the entity list
//! out across four layers.
//!
//! Given `Name => Parent` entries, this macro emits the `entity`-layer
//! aggregates (`EntityKind`, `AnyEntityRef`, `TrackedEntity`) together with
//! the `TrackedEntity` dispatch impls owned by `store`, `substrate`, and
//! `validation`. Keeping the fan-out in one macro means adding a new entity
//! is a one-line edit and every layer's dispatch stays exhaustive
//! automatically.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, Token};

use crate::{
    entity_codegen::generate_entity_registry_parts, store_codegen::generate_store_registry_parts,
    substrate_codegen::generate_substrate_registry_parts,
    validation_codegen::generate_workspace_validate_tracked,
};

pub struct RegistryEntry {
    pub name: Ident,
    pub parent: Ident,
}

pub struct RegistryInput(pub Vec<RegistryEntry>);

impl syn::parse::Parse for RegistryInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut entries = Vec::new();
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            input.parse::<Token![=>]>()?;
            let parent: Ident = input.parse()?;
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
            entries.push(RegistryEntry { name, parent });
        }
        Ok(RegistryInput(entries))
    }
}

pub fn generate_registry(entries: Vec<RegistryEntry>) -> TokenStream2 {
    let tracked_names: Vec<Ident> = entries
        .iter()
        .map(|e| Ident::new(&format!("Tracked{}", e.name), e.name.span()))
        .collect();
    let schema_names: Vec<Ident> = entries
        .iter()
        .map(|e| Ident::new(&format!("{}Schema", e.name), e.name.span()))
        .collect();
    let variants: Vec<&Ident> = entries.iter().map(|e| &e.name).collect();

    let entity_parts = generate_entity_registry_parts(&entries, &tracked_names);
    let crate::entity_codegen::EntityRegistryParts {
        entity_kind,
        any_entity_ref,
        tracked_entity,
        tracked_entity_impl: entity_tracked_entity_impl,
    } = entity_parts;

    let store_parts = generate_store_registry_parts(&entries, &tracked_names);
    let crate::store_codegen::StoreRegistryParts {
        tracked_entity_impl: store_tracked_entity_impl,
    } = store_parts;

    let substrate_parts =
        generate_substrate_registry_parts(&entries, &variants, &tracked_names, &schema_names);
    let crate::substrate_codegen::SubstrateRegistryParts {
        any_entity_ref_impl,
        tracked_entity_impl: substrate_tracked_entity_impl,
        schema_trait,
        load_strategy_fn,
    } = substrate_parts;

    let validate_tracked_impl = generate_workspace_validate_tracked(&entries);

    quote! {
        #entity_kind
        #any_entity_ref
        #tracked_entity
        #entity_tracked_entity_impl
        #store_tracked_entity_impl
        #substrate_tracked_entity_impl
        #validate_tracked_impl
        #any_entity_ref_impl
        #schema_trait
        #load_strategy_fn
    }
}
