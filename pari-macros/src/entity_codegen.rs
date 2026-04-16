use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::entity_registry::RegistryEntry;

pub struct EntityRegistryParts {
    pub from_methods: Vec<TokenStream2>,
    pub any_ref_arms: Vec<TokenStream2>,
}

pub fn generate_entity_registry_parts(
    entries: &[RegistryEntry],
    tracked_names: &[Ident],
) -> EntityRegistryParts {
    let from_methods = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, tname)| {
            let vname = &e.name;
            let fn_name = Ident::new(
                &format!("from_{}", to_snake_case(&vname.to_string())),
                vname.span(),
            );
            quote! {
                pub fn #fn_name(e: #tname) -> Self { TrackedEntity::#vname(e) }
            }
        })
        .collect();

    let any_ref_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => AnyEntityRef::#vname(e.entity_ref().clone()),
            }
        })
        .collect();

    EntityRegistryParts {
        from_methods,
        any_ref_arms,
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    result
}
