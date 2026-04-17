use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::entity_registry::RegistryEntry;

fn generate_registry_validation_dispatch(entries: &[RegistryEntry]) -> Vec<TokenStream2> {
    entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            let ty = &e.name;
            quote! {
                TrackedEntity::#vname(e) => ::pari::validation::run_validations::<#ty>(e, fields, kinds).await,
            }
        })
        .collect()
}

pub fn generate_tracked_entity_validation_impl(entries: &[RegistryEntry]) -> TokenStream2 {
    let run_validations_arms = generate_registry_validation_dispatch(entries);
    quote! {
        impl TrackedEntity {
            pub async fn run_validations(
                &self,
                fields: &[&str],
                kinds: &[::pari::validation::ValidationKind],
            ) -> ::pari::validation::ValidationErrors {
                match self {
                    #(#run_validations_arms)*
                }
            }
        }
    }
}

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
