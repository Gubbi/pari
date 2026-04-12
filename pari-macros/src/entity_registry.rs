use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, Token};

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
        }
    };

    let from_methods: Vec<TokenStream2> = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, tname)| {
            let vname = &e.name;
            let fn_name = Ident::new(
                &format!("from_{}", to_snake_case(&vname.to_string())),
                vname.span(),
            );
            quote! {
                pub fn #fn_name(e: #tname) -> Self { StoreEntity::#vname(e) }
            }
        })
        .collect();

    let any_ref_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => AnyEntityRef::#vname(e.entity_ref().clone()),
            }
        })
        .collect();

    let make_stub_arms: Vec<TokenStream2> = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, tname)| {
            let vname = &e.name;
            quote! {
                AnyEntityRef::#vname(r) => StoreEntity::#vname(#tname::make_stub(r.clone())),
            }
        })
        .collect();

    let all_refs_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => e.all_refs(),
            }
        })
        .collect();

    let initialize_into_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                (StoreEntity::#vname(src), StoreEntity::#vname(dst)) => src.initialize_into(dst),
            }
        })
        .collect();

    let merge_dirty_into_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                (StoreEntity::#vname(src), StoreEntity::#vname(dst)) => src.merge_dirty_into(dst),
            }
        })
        .collect();

    let has_dirty_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => e.has_dirty_fields(),
            }
        })
        .collect();

    let dirty_fields_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => e.dirty_fields(),
            }
        })
        .collect();

    let reset_dirty_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => e.reset_dirty(),
            }
        })
        .collect();

    let is_stub_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => e.is_stub(),
            }
        })
        .collect();

    let store_entity = quote! {
        #[derive(Clone)]
        pub enum StoreEntity {
            #(#variants(#tracked_names),)*
        }

        impl StoreEntity {
            #(#from_methods)*

            pub fn any_ref(&self) -> AnyEntityRef {
                match self {
                    #(#any_ref_arms)*
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

            pub fn initialize_into(&self, target: &mut StoreEntity) {
                match (self, target) {
                    #(#initialize_into_arms)*
                    _ => {}
                }
            }

            pub fn merge_dirty_into(&self, target: &mut StoreEntity) {
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
        }
    };

    let substrate_schema = quote! {
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

    let load_strategy = quote! {
        pub fn load_strategy(kind: EntityKind) -> &'static dyn SubstrateSchema {
            #(#schema_structs)*
            match kind {
                #(#load_arms)*
            }
        }
    };

    quote! {
        #entity_kind
        #any_entity_ref
        #store_entity
        #substrate_schema
        #load_strategy
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
