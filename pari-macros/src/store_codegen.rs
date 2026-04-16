use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::entity_registry::RegistryEntry;

pub struct StoreRegistryParts {
    pub make_stub_arms: Vec<TokenStream2>,
    pub all_refs_arms: Vec<TokenStream2>,
    pub initialize_into_arms: Vec<TokenStream2>,
    pub merge_dirty_into_arms: Vec<TokenStream2>,
    pub has_dirty_arms: Vec<TokenStream2>,
    pub dirty_fields_arms: Vec<TokenStream2>,
    pub reset_dirty_arms: Vec<TokenStream2>,
    pub is_stub_arms: Vec<TokenStream2>,
    pub is_field_loaded_arms: Vec<TokenStream2>,
}

pub fn generate_store_registry_parts(
    entries: &[RegistryEntry],
    tracked_names: &[syn::Ident],
) -> StoreRegistryParts {
    let make_stub_arms = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, tname)| {
            let vname = &e.name;
            quote! {
                AnyEntityRef::#vname(r) => TrackedEntity::#vname(#tname::make_stub(r.clone())),
            }
        })
        .collect();

    let all_refs_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.all_refs(),
            }
        })
        .collect();

    let initialize_into_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                (TrackedEntity::#vname(src), TrackedEntity::#vname(dst)) => src.initialize_into(dst),
            }
        })
        .collect();

    let merge_dirty_into_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                (TrackedEntity::#vname(src), TrackedEntity::#vname(dst)) => src.merge_dirty_into(dst),
            }
        })
        .collect();

    let has_dirty_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.has_dirty_fields(),
            }
        })
        .collect();

    let dirty_fields_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.dirty_fields(),
            }
        })
        .collect();

    let reset_dirty_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.reset_dirty(),
            }
        })
        .collect();

    let is_stub_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.is_stub(),
            }
        })
        .collect();

    let is_field_loaded_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.is_field_loaded(field),
            }
        })
        .collect();

    StoreRegistryParts {
        make_stub_arms,
        all_refs_arms,
        initialize_into_arms,
        merge_dirty_into_arms,
        has_dirty_arms,
        dirty_fields_arms,
        reset_dirty_arms,
        is_stub_arms,
        is_field_loaded_arms,
    }
}
