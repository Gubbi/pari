use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, Token};

use crate::entity_codegen::generate_entity_registry_parts;
use crate::store_codegen::generate_store_registry_parts;
use crate::substrate_codegen::generate_substrate_registry_parts;
use crate::validation_codegen::generate_registry_validation_dispatch;

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
    let variants: Vec<&Ident> = entries.iter().map(|e| &e.name).collect();
    let tracked_names: Vec<Ident> = entries
        .iter()
        .map(|e| Ident::new(&format!("Tracked{}", e.name), e.name.span()))
        .collect();
    let schema_names: Vec<Ident> = entries
        .iter()
        .map(|e| Ident::new(&format!("{}Schema", e.name), e.name.span()))
        .collect();

    let as_str_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let v_str = v.to_string();
            quote! { EntityKind::#v => #v_str, }
        })
        .collect();

    let from_str_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let v_str = v.to_string();
            quote! { #v_str => Some(EntityKind::#v), }
        })
        .collect();

    let entity_kind = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum EntityKind {
            #(#variants,)*
        }

        impl EntityKind {
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(#as_str_arms)*
                }
            }

            pub fn from_str(value: &str) -> Option<Self> {
                match value {
                    #(#from_str_arms)*
                    _ => None,
                }
            }
        }
    };

    let any_ref_variants: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let name = &e.name;
            let parent = &e.parent;
            quote! { #name(EntityRef<#name, #parent>) }
        })
        .collect();

    let kind_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| quote! { AnyEntityRef::#v(_) => EntityKind::#v, })
        .collect();

    let id_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| quote! { AnyEntityRef::#v(r) => r.id(), })
        .collect();

    let parent_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let name = &e.name;
            if e.parent == "NoParent" {
                quote! { AnyEntityRef::#name(_) => None, }
            } else {
                quote! { AnyEntityRef::#name(r) => r.parent().map(|p| p.to_any_ref()), }
            }
        })
        .collect();

    let substrate_parts =
        generate_substrate_registry_parts(&entries, &variants, &tracked_names, &schema_names);
    let crate::substrate_codegen::SubstrateRegistryParts {
        any_ref_to_json_arms,
        tracked_to_json_arms,
        tracked_from_json_arms,
        schema_trait,
        load_strategy_fn,
    } = substrate_parts;

    let any_entity_ref = quote! {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum AnyEntityRef {
            #(#any_ref_variants,)*
        }

        impl AnyEntityRef {
            pub fn kind(&self) -> EntityKind {
                match self { #(#kind_arms)* }
            }

            pub fn id(&self) -> &str {
                match self { #(#id_arms)* }
            }

            pub fn parent(&self) -> Option<AnyEntityRef> {
                match self { #(#parent_arms)* }
            }

            pub fn to_json_value(&self) -> ::serde_json::Result<::serde_json::Value> {
                match self {
                    #(#any_ref_to_json_arms)*
                }
            }
        }
    };

    let entity_parts = generate_entity_registry_parts(&entries, &tracked_names);
    let crate::entity_codegen::EntityRegistryParts {
        from_methods,
        any_ref_arms,
    } = entity_parts;

    let store_parts = generate_store_registry_parts(&entries, &tracked_names);
    let crate::store_codegen::StoreRegistryParts {
        make_stub_arms,
        all_refs_arms,
        initialize_into_arms,
        merge_dirty_into_arms,
        has_dirty_arms,
        dirty_fields_arms,
        reset_dirty_arms,
        is_stub_arms,
        is_field_loaded_arms,
    } = store_parts;

    let run_validations_arms: Vec<TokenStream2> =
        generate_registry_validation_dispatch(&entries);

    let tracked_entity = quote! {
        #[derive(Clone)]
        pub enum TrackedEntity {
            #(#variants(#tracked_names),)*
        }

        impl TrackedEntity {
            #(#from_methods)*

            pub fn any_ref(&self) -> AnyEntityRef {
                match self {
                    #(#any_ref_arms)*
                }
            }

            pub fn to_json_value(&self) -> ::serde_json::Result<::serde_json::Value> {
                match self {
                    #(#tracked_to_json_arms)*
                }
            }

            pub fn from_json_value(
                any_ref: &AnyEntityRef,
                value: ::serde_json::Value,
            ) -> ::serde_json::Result<Self> {
                match any_ref {
                    #(#tracked_from_json_arms)*
                }
            }

            pub fn make_stub(any_ref: &AnyEntityRef) -> Self {
                match any_ref {
                    #(#make_stub_arms)*
                }
            }

            pub fn all_refs(&self) -> ::std::vec::Vec<AnyEntityRef> {
                match self {
                    #(#all_refs_arms)*
                }
            }

            pub fn initialize_into(&self, target: &mut TrackedEntity) {
                match (self, target) {
                    #(#initialize_into_arms)*
                    _ => {}
                }
            }

            pub fn merge_dirty_into(&self, target: &mut TrackedEntity) {
                match (self, target) {
                    #(#merge_dirty_into_arms)*
                    _ => {}
                }
            }

            pub fn has_dirty_fields(&self) -> bool {
                match self {
                    #(#has_dirty_arms)*
                }
            }

            pub fn dirty_fields(&self) -> ::std::vec::Vec<&'static str> {
                match self {
                    #(#dirty_fields_arms)*
                }
            }

            pub fn reset_dirty(&mut self) {
                match self {
                    #(#reset_dirty_arms)*
                }
            }

            pub fn is_stub(&self) -> bool {
                match self {
                    #(#is_stub_arms)*
                }
            }

            pub fn is_field_loaded(&self, field: &str) -> bool {
                match self {
                    #(#is_field_loaded_arms)*
                }
            }

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
    };

    quote! {
        #entity_kind
        #any_entity_ref
        #tracked_entity
        #schema_trait
        #load_strategy_fn
    }
}
