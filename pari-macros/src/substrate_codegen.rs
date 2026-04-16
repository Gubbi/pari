use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::entity_registry::RegistryEntry;

pub struct SubstrateRegistryParts {
    pub any_ref_to_json_arms: Vec<TokenStream2>,
    pub tracked_to_json_arms: Vec<TokenStream2>,
    pub tracked_from_json_arms: Vec<TokenStream2>,
    pub schema_trait: TokenStream2,
    pub load_strategy_fn: TokenStream2,
}

pub fn generate_substrate_registry_parts(
    entries: &[RegistryEntry],
    variants: &[&Ident],
    tracked_names: &[Ident],
    schema_names: &[Ident],
) -> SubstrateRegistryParts {
    let any_ref_to_json_arms = entries
        .iter()
        .map(|e| {
            let name = &e.name;
            quote! {
                AnyEntityRef::#name(r) => ::serde_json::to_value(r),
            }
        })
        .collect();

    let tracked_to_json_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => ::serde_json::to_value(e),
            }
        })
        .collect();

    let tracked_from_json_arms = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, tname)| {
            let vname = &e.name;
            quote! {
                AnyEntityRef::#vname(_) => ::serde_json::from_value::<#tname>(value).map(TrackedEntity::#vname),
            }
        })
        .collect();

    let schema_trait = quote! {
        pub trait SubstrateSchema: Send + Sync {
            fn kind(&self) -> EntityKind;
        }
    };

    let schema_structs: Vec<TokenStream2> = entries
        .iter()
        .zip(schema_names.iter())
        .map(|(e, schema_name)| {
            let kind_variant = &e.name;
            quote! {
                struct #schema_name;
                impl SubstrateSchema for #schema_name {
                    fn kind(&self) -> EntityKind { EntityKind::#kind_variant }
                }
            }
        })
        .collect();

    let load_arms: Vec<TokenStream2> = variants
        .iter()
        .zip(schema_names.iter())
        .map(|(v, s)| quote! { EntityKind::#v => &#s, })
        .collect();

    let load_strategy_fn = quote! {
        pub fn load_strategy(kind: EntityKind) -> &'static dyn SubstrateSchema {
            #(#schema_structs)*
            match kind {
                #(#load_arms)*
            }
        }
    };

    SubstrateRegistryParts {
        any_ref_to_json_arms,
        tracked_to_json_arms,
        tracked_from_json_arms,
        schema_trait,
        load_strategy_fn,
    }
}
