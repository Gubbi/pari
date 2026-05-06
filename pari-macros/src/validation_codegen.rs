//! Per-entity validation glue plus the type-erased dispatch on
//! `Workspace::validate_tracked`.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::entity_registry::RegistryEntry;

pub fn generate_validation_schema_access(
    entity_name: &Ident,
    schema_fn: &TokenStream2,
) -> TokenStream2 {
    quote! {
        fn validation_schema() -> &'static ::pari::entity::ValidationSchema<Self> {
            static S: ::std::sync::OnceLock<::pari::entity::ValidationSchema<#entity_name>> =
                ::std::sync::OnceLock::new();
            S.get_or_init(|| #schema_fn)
        }
    }
}

/// Emit `Workspace::validate_tracked` — the single crate-internal
/// dispatch that runs validation against a type-erased
/// [`TrackedEntity`]. Each match arm constructs a typed
/// `XViewer<'_, Kind>` momentarily; no type-erased viewer is reified.
///
/// Used by [`StoreServer`](crate::store::StoreServer) at the
/// validation gates (insert, commit, load) where the entity is held
/// in its type-erased form.
pub fn generate_workspace_validate_tracked(entries: &[RegistryEntry]) -> TokenStream2 {
    let arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let v = &e.name;
            quote! {
                TrackedEntity::#v(t) => self
                    .import::<#v>(t)
                    .validate_with(fields, kinds)
                    .await,
            }
        })
        .collect();

    quote! {
        impl ::pari::workspace::Workspace {
            /// Run validation against a type-erased
            /// [`TrackedEntity`]. Per-kind dispatch wraps the inner
            /// tracked state as a typed `XViewer` for the duration of
            /// the call.
            pub(crate) async fn validate_tracked(
                &self,
                tracked: TrackedEntity,
                fields: &[&str],
                kinds: &[::pari::validation::ValidationKind],
            ) -> ::std::result::Result<(), ::pari::error::ActivityError> {
                match tracked {
                    #(#arms)*
                }
            }
        }
    }
}
