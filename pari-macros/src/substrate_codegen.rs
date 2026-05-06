use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::entity_registry::RegistryEntry;

pub struct SubstrateRegistryParts {
    pub any_entity_ref_impl: TokenStream2,
    pub tracked_entity_impl: TokenStream2,
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
        .collect::<Vec<_>>();

    let any_ref_from_json_arms = entries
        .iter()
        .map(|e| {
            let name = &e.name;
            quote! {
                EntityKind::#name => ::serde_json::from_value(value).map(AnyEntityRef::#name),
            }
        })
        .collect::<Vec<_>>();

    let tracked_to_json_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => ::serde_json::to_value(e),
            }
        })
        .collect::<Vec<_>>();

    let tracked_from_json_arms = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, tname)| {
            let vname = &e.name;
            quote! {
                AnyEntityRef::#vname(_) => ::serde_json::from_value::<#tname>(value).map(TrackedEntity::#vname),
            }
        })
        .collect::<Vec<_>>();

    let any_entity_ref_impl = quote! {
        impl AnyEntityRef {
            pub(crate) fn to_json_value(&self) -> ::serde_json::Result<::serde_json::Value> {
                match self {
                    #(#any_ref_to_json_arms)*
                }
            }

            /// JSON → `AnyEntityRef` per-kind dispatch. Reads the
            /// `kind` discriminator from the value and deserializes
            /// into the matching typed `EntityRef` variant.
            pub(crate) fn from_json_value(
                value: ::serde_json::Value,
            ) -> ::std::result::Result<Self, ::pari::error::primitive::PrimitiveError> {
                let kind_str = value
                    .get("kind")
                    .and_then(::serde_json::Value::as_str)
                    .ok_or_else(|| {
                        ::pari::error::primitive::PrimitiveError::partial_payload_deserialization(
                            "missing or invalid `kind` in entity_ref payload",
                            "<unknown>".to_string(),
                            "no `kind` discriminator".to_string(),
                        )
                    })?
                    .to_owned();
                let kind = EntityKind::from_str(&kind_str).ok_or_else(|| {
                    ::pari::error::primitive::PrimitiveError::partial_payload_deserialization(
                        "unknown entity kind in entity_ref payload",
                        "<unknown>".to_string(),
                        kind_str.clone(),
                    )
                })?;
                let kind_label = kind_str;
                match kind {
                    #(#any_ref_from_json_arms)*
                }
                .map_err(|e| {
                    ::pari::error::primitive::PrimitiveError::partial_payload_deserialization(
                        "entity_ref deserialization failed",
                        kind_label,
                        e.to_string(),
                    )
                })
            }
        }
    };

    let tracked_entity_impl = quote! {
        impl TrackedEntity {
            pub(crate) fn to_json_value(&self) -> ::serde_json::Result<::serde_json::Value> {
                match self {
                    #(#tracked_to_json_arms)*
                }
            }

            /// JSON → `TrackedEntity` per-kind dispatch. Reachable
            /// only through the store's JSON pipeline.
            pub(crate) fn from_json_value(
                any_ref: &AnyEntityRef,
                value: ::serde_json::Value,
            ) -> ::serde_json::Result<Self> {
                match any_ref {
                    #(#tracked_from_json_arms)*
                }
            }
        }
    };

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
        any_entity_ref_impl,
        tracked_entity_impl,
        schema_trait,
        load_strategy_fn,
    }
}
