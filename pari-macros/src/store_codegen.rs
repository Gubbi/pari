use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::entity_registry::RegistryEntry;

pub struct StoreRegistryParts {
    pub tracked_entity_impl: TokenStream2,
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
        .collect::<Vec<_>>();

    let all_refs_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.all_refs(),
            }
        })
        .collect::<Vec<_>>();

    let initialize_into_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                (TrackedEntity::#vname(src), TrackedEntity::#vname(dst)) => src.initialize_into(dst),
            }
        })
        .collect::<Vec<_>>();

    let merge_dirty_into_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                (TrackedEntity::#vname(src), TrackedEntity::#vname(dst)) => src.merge_dirty_into(dst),
            }
        })
        .collect::<Vec<_>>();

    let has_dirty_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.has_dirty_fields(),
            }
        })
        .collect::<Vec<_>>();

    let dirty_fields_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.dirty_fields(),
            }
        })
        .collect::<Vec<_>>();

    let reset_dirty_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.reset_dirty(),
            }
        })
        .collect::<Vec<_>>();

    let is_stub_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.is_stub(),
            }
        })
        .collect::<Vec<_>>();

    let is_field_loaded_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => e.is_field_loaded(field),
            }
        })
        .collect::<Vec<_>>();

    let tracked_entity_impl = quote! {
        impl TrackedEntity {
            pub(crate) fn make_stub(any_ref: &AnyEntityRef) -> Self {
                match any_ref {
                    #(#make_stub_arms)*
                }
            }

            pub(crate) fn all_refs(&self) -> ::std::vec::Vec<AnyEntityRef> {
                match self {
                    #(#all_refs_arms)*
                }
            }

            pub(crate) fn initialize_into(&self, target: &mut TrackedEntity) {
                match (self, target) {
                    #(#initialize_into_arms)*
                    _ => {}
                }
            }

            pub(crate) fn merge_dirty_into(&self, target: &mut TrackedEntity) {
                match (self, target) {
                    #(#merge_dirty_into_arms)*
                    _ => {}
                }
            }

            pub(crate) fn has_dirty_fields(&self) -> bool {
                match self {
                    #(#has_dirty_arms)*
                }
            }

            pub(crate) fn dirty_fields(&self) -> ::std::vec::Vec<&'static str> {
                match self {
                    #(#dirty_fields_arms)*
                }
            }

            pub(crate) fn reset_dirty(&mut self) {
                match self {
                    #(#reset_dirty_arms)*
                }
            }

            pub(crate) fn is_stub(&self) -> bool {
                match self {
                    #(#is_stub_arms)*
                }
            }

            pub(crate) fn is_field_loaded(&self, field: &str) -> bool {
                match self {
                    #(#is_field_loaded_arms)*
                }
            }
        }
    };

    StoreRegistryParts {
        tracked_entity_impl,
    }
}
