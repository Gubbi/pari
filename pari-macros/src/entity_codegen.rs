use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Field, Fields, Ident, Type};

use crate::entity_registry::RegistryEntry;

pub struct EntityDeriveParts {
    pub tracked_struct: TokenStream2,
    pub tracked_impl: TokenStream2,
    pub serialize_impl: TokenStream2,
    pub deserialize_impl: TokenStream2,
    pub entity_impl: TokenStream2,
    pub tracked_for_impl: TokenStream2,
    pub domain_fields: Vec<Field>,
}

pub fn generate_entity_derive_parts(
    ast: &DeriveInput,
    tracked_name: &Ident,
    kind_expr: &TokenStream2,
    parent_type: &TokenStream2,
    no_dispatch: bool,
    validation_schema_method: TokenStream2,
) -> Result<EntityDeriveParts, TokenStream2> {
    let name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Entity only supports named-field structs",
                )
                .to_compile_error());
            }
        },
        _ => {
            return Err(
                syn::Error::new_spanned(name, "Entity only supports structs").to_compile_error(),
            );
        }
    };

    let entity_ref_field = fields
        .iter()
        .find(|f| f.ident.as_ref().map(|i| i == "entity_ref").unwrap_or(false));
    let domain_fields: Vec<Field> = fields
        .iter()
        .filter(|f| f.ident.as_ref().map(|i| i != "entity_ref").unwrap_or(true))
        .cloned()
        .collect();
    let domain_field_refs: Vec<&Field> = domain_fields.iter().collect();
    let entity_ref_type = entity_ref_field.map(|f| &f.ty);
    let vis = &ast.vis;
    let variant_name = entity_kind_to_any_ref_variant(kind_expr);

    let tracked_struct =
        generate_tracked_struct(vis, tracked_name, entity_ref_type, &domain_field_refs);
    let tracked_impl = generate_tracked_impl(
        name,
        tracked_name,
        entity_ref_type,
        entity_ref_field.is_some(),
        &domain_field_refs,
    );
    let serialize_impl =
        generate_serialize_impl(tracked_name, entity_ref_field.is_some(), &domain_field_refs);
    let deserialize_impl = generate_deserialize_impl(
        tracked_name,
        entity_ref_type,
        entity_ref_field.is_some(),
        &domain_field_refs,
    );
    let entity_impl = generate_entity_impl(
        name,
        tracked_name,
        kind_expr,
        parent_type,
        no_dispatch,
        validation_schema_method,
        variant_name,
    );
    let tracked_for_impl = quote! {
        impl ::pari::entity::TrackedFor for #tracked_name {
            type Entity = #name;
        }
    };

    Ok(EntityDeriveParts {
        tracked_struct,
        tracked_impl,
        serialize_impl,
        deserialize_impl,
        entity_impl,
        tracked_for_impl,
        domain_fields,
    })
}

pub struct EntityRegistryParts {
    pub entity_kind: TokenStream2,
    pub any_entity_ref: TokenStream2,
    pub tracked_entity: TokenStream2,
    pub tracked_entity_impl: TokenStream2,
}

pub fn generate_entity_registry_parts(
    entries: &[RegistryEntry],
    tracked_names: &[Ident],
) -> EntityRegistryParts {
    let variants: Vec<&Ident> = entries.iter().map(|e| &e.name).collect();

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
        .collect::<Vec<_>>();

    let any_ref_arms = entries
        .iter()
        .map(|e| {
            let vname = &e.name;
            quote! {
                TrackedEntity::#vname(e) => AnyEntityRef::#vname(e.entity_ref().clone()),
            }
        })
        .collect::<Vec<_>>();

    let tracked_entity = quote! {
        #[derive(Clone)]
        pub enum TrackedEntity {
            #(#variants(#tracked_names),)*
        }
    };

    let tracked_entity_impl = quote! {
        impl TrackedEntity {
            #(#from_methods)*

            pub fn any_ref(&self) -> AnyEntityRef {
                match self {
                    #(#any_ref_arms)*
                }
            }
        }
    };

    EntityRegistryParts {
        entity_kind,
        any_entity_ref,
        tracked_entity,
        tracked_entity_impl,
    }
}

fn generate_tracked_struct(
    vis: &syn::Visibility,
    tracked_name: &Ident,
    entity_ref_type: Option<&Type>,
    domain_fields: &[&Field],
) -> TokenStream2 {
    let tracked_field_decls: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let vis = &f.vis;
            let ty = &f.ty;
            quote! { #vis #fname: ::std::sync::Arc<::pari::tracked::TrackedField<#ty>>, }
        })
        .collect();

    let entity_ref_decl = if let Some(ty) = entity_ref_type {
        quote! { pub entity_ref: #ty, }
    } else {
        quote! {}
    };

    quote! {
        #[derive(Clone)]
        #vis struct #tracked_name {
            #entity_ref_decl
            #(#tracked_field_decls)*
        }
    }
}

fn generate_tracked_impl(
    name: &Ident,
    tracked_name: &Ident,
    entity_ref_type: Option<&Type>,
    has_entity_ref: bool,
    domain_fields: &[&Field],
) -> TokenStream2 {
    let from_field_inits: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            quote! {
                #fname: ::std::sync::Arc::new(
                    ::pari::tracked::TrackedField::loaded(plain.#fname)
                ),
            }
        })
        .collect();

    let entity_ref_from = if has_entity_ref {
        quote! { entity_ref: plain.entity_ref, }
    } else {
        quote! {}
    };

    let entity_ref_accessor = if let Some(ty) = entity_ref_type {
        quote! {
            pub fn entity_ref(&self) -> &#ty {
                &self.entity_ref
            }
        }
    } else {
        quote! {}
    };

    let has_dirty_checks: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            quote! { self.#fname.is_dirty() }
        })
        .collect();

    let dirty_field_checks: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let fname_str = fname.as_ref().unwrap().to_string();
            quote! {
                if self.#fname.is_dirty() { out.push(#fname_str); }
            }
        })
        .collect();

    let merge_stmts: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            quote! {
                if self.#fname.is_dirty() {
                    target.#fname = ::std::sync::Arc::clone(&self.#fname);
                }
            }
        })
        .collect();

    let reset_stmts: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            quote! {
                self.#fname.reset_dirty();
            }
        })
        .collect();

    let has_dirty_expr = if has_dirty_checks.is_empty() {
        quote! { false }
    } else {
        quote! { #(#has_dirty_checks)||* }
    };

    let is_stub_impl = {
        let first_required = domain_fields.iter().find(|f| {
            !matches!(&f.ty, syn::Type::Path(tp)
                if tp.path.segments.last().map(|s| s.ident == "Option").unwrap_or(false))
        });
        if let Some(f) = first_required {
            let fname = &f.ident;
            quote! {
                pub fn is_stub(&self) -> bool {
                    self.#fname.get().is_none()
                }
            }
        } else {
            quote! {
                pub fn is_stub(&self) -> bool { false }
            }
        }
    };

    let make_stub_body = if let Some(ty) = entity_ref_type {
        let stub_field_inits: Vec<TokenStream2> = domain_fields
            .iter()
            .map(|f| {
                let fname = &f.ident;
                quote! {
                    #fname: ::std::sync::Arc::new(::pari::tracked::TrackedField::new()),
                }
            })
            .collect();
        quote! {
            pub fn make_stub(entity_ref: #ty) -> Self {
                #tracked_name {
                    entity_ref,
                    #(#stub_field_inits)*
                }
            }
        }
    } else {
        quote! {}
    };

    let all_refs_pushes: Vec<TokenStream2> = domain_fields
        .iter()
        .filter_map(|f| {
            let fname = &f.ident;
            let ty = &f.ty;
            if let Type::Path(tp) = ty {
                if tp.qself.is_none() && tp.path.segments.len() == 1 {
                    let seg = &tp.path.segments[0];
                    if seg.ident == "EntityRef" {
                        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                            if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                if let Type::Path(inner_tp) = inner {
                                    if inner_tp.path.segments.len() == 1 {
                                        let entity_name = &inner_tp.path.segments[0].ident;
                                        return Some(quote! {
                                            if let Some(r) = self.#fname.get() {
                                                refs.push(::pari::entity::AnyEntityRef::#entity_name(r.clone()));
                                            }
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None
        })
        .collect();

    let all_refs_method = quote! {
        pub fn all_refs(&self) -> ::std::vec::Vec<::pari::entity::AnyEntityRef> {
            let mut refs = ::std::vec::Vec::new();
            #(#all_refs_pushes)*
            refs
        }
    };

    let initialize_into_stmts: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            quote! {
                if let Some(v) = self.#fname.get() {
                    target.#fname.initialize(v.clone());
                }
            }
        })
        .collect();

    let initialize_into_method = quote! {
        pub fn initialize_into(&self, target: &mut #tracked_name) {
            #(#initialize_into_stmts)*
        }
    };

    let field_loaded_arms: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let fname_str = fname.as_ref().unwrap().to_string();
            quote! { #fname_str => self.#fname.get().is_some(), }
        })
        .collect();

    let is_field_loaded_method = quote! {
        pub fn is_field_loaded(&self, field: &str) -> bool {
            match field {
                #(#field_loaded_arms)*
                _ => false,
            }
        }
    };

    quote! {
        impl ::std::convert::From<#name> for #tracked_name {
            fn from(plain: #name) -> Self {
                Self {
                    #entity_ref_from
                    #(#from_field_inits)*
                }
            }
        }

        impl #tracked_name {
            #entity_ref_accessor

            pub fn has_dirty_fields(&self) -> bool {
                #has_dirty_expr
            }

            pub fn dirty_fields(&self) -> ::std::vec::Vec<&'static str> {
                let mut out: ::std::vec::Vec<&'static str> = ::std::vec::Vec::new();
                #(#dirty_field_checks)*
                out
            }

            pub fn merge_dirty_into(&self, target: &mut #tracked_name) {
                #(#merge_stmts)*
            }

            pub fn reset_dirty(&mut self) {
                #(#reset_stmts)*
            }

            #is_stub_impl
            #make_stub_body
            #all_refs_method
            #initialize_into_method
            #is_field_loaded_method
        }
    }
}

fn generate_serialize_impl(
    tracked_name: &Ident,
    has_entity_ref: bool,
    domain_fields: &[&Field],
) -> TokenStream2 {
    let er_serialize = if has_entity_ref {
        quote! {
            map.insert(
                "entity_ref".to_string(),
                ::serde_json::to_value(&self.entity_ref).map_err(::serde::ser::Error::custom)?
            );
        }
    } else {
        quote! {}
    };

    let field_serializes: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let fname_str = fname.as_ref().unwrap().to_string();
            if fname_str == "extensions" {
                quote! {
                    if let Some(ext) = self.#fname.get() {
                        for (k, v) in ext {
                            map.insert(k.clone(), v.clone());
                        }
                    }
                }
            } else {
                quote! {
                    if let Some(v) = self.#fname.get() {
                        map.insert(
                            #fname_str.to_string(),
                            ::serde_json::to_value(v).map_err(::serde::ser::Error::custom)?
                        );
                    }
                }
            }
        })
        .collect();

    quote! {
        impl ::serde::Serialize for #tracked_name {
            fn serialize<S: ::serde::Serializer>(&self, s: S)
                -> ::std::result::Result<S::Ok, S::Error>
            {
                let mut map = ::serde_json::Map::new();
                #er_serialize
                #(#field_serializes)*
                ::serde_json::Value::Object(map).serialize(s)
            }
        }
    }
}

fn generate_deserialize_impl(
    tracked_name: &Ident,
    entity_ref_type: Option<&Type>,
    has_entity_ref: bool,
    domain_fields: &[&Field],
) -> TokenStream2 {
    let er_accum = if let Some(ty) = entity_ref_type {
        quote! { let mut entity_ref: ::std::option::Option<#ty> = None; }
    } else {
        quote! {}
    };

    let has_extensions_field = domain_fields
        .iter()
        .any(|f| f.ident.as_ref().map(|i| i == "extensions").unwrap_or(false));

    let field_accums: Vec<TokenStream2> = domain_fields
        .iter()
        .filter(|f| f.ident.as_ref().map(|i| i != "extensions").unwrap_or(true))
        .map(|f| {
            let fname = &f.ident;
            let ty = &f.ty;
            quote! { let mut #fname: ::std::option::Option<#ty> = None; }
        })
        .collect();

    let extensions_accum = if has_extensions_field {
        quote! {
            let mut extensions: ::std::option::Option<
                ::std::collections::HashMap<::std::string::String, ::serde_json::Value>
            > = None;
        }
    } else {
        quote! {}
    };

    let field_match_arms: Vec<TokenStream2> = domain_fields
        .iter()
        .filter(|f| f.ident.as_ref().map(|i| i != "extensions").unwrap_or(true))
        .map(|f| {
            let fname = &f.ident;
            let fname_str = fname.as_ref().unwrap().to_string();
            quote! { #fname_str => #fname = Some(map.next_value()?), }
        })
        .collect();

    let extensions_x_arm = if has_extensions_field {
        quote! {
            k if k.starts_with("x-") => {
                let v: ::serde_json::Value = map.next_value()?;
                extensions
                    .get_or_insert_with(::std::collections::HashMap::new)
                    .insert(k.to_string(), v);
            }
        }
    } else {
        quote! {}
    };

    let field_arc_inits: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            quote! {
                #fname: ::std::sync::Arc::new(::pari::tracked::TrackedField::new()),
            }
        })
        .collect();

    let field_init_calls: Vec<TokenStream2> = domain_fields
        .iter()
        .filter(|f| f.ident.as_ref().map(|i| i != "extensions").unwrap_or(true))
        .map(|f| {
            let fname = &f.ident;
            quote! { if let Some(v) = #fname { tracked.#fname.initialize(v); } }
        })
        .collect();

    let extensions_init_call = if has_extensions_field {
        quote! { if let Some(v) = extensions { tracked.extensions.initialize(v.into()); } }
    } else {
        quote! {}
    };

    let er_required = if has_entity_ref {
        quote! {
            let entity_ref = entity_ref
                .ok_or_else(|| ::serde::de::Error::missing_field("entity_ref"))?;
        }
    } else {
        quote! {}
    };

    let er_struct_field = if has_entity_ref {
        quote! { entity_ref, }
    } else {
        quote! {}
    };

    let tracked_name_str = tracked_name.to_string();

    quote! {
        impl<'de> ::serde::Deserialize<'de> for #tracked_name {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D)
                -> ::std::result::Result<Self, D::Error>
            {
                use ::serde::de::{MapAccess, Visitor};

                struct V;

                impl<'de> Visitor<'de> for V {
                    type Value = #tracked_name;

                    fn expecting(
                        &self,
                        f: &mut ::std::fmt::Formatter,
                    ) -> ::std::fmt::Result {
                        write!(f, "{} object", #tracked_name_str)
                    }

                    fn visit_map<A: MapAccess<'de>>(
                        self,
                        mut map: A,
                    ) -> ::std::result::Result<#tracked_name, A::Error> {
                        #er_accum
                        #(#field_accums)*
                        #extensions_accum

                        while let Some(key) = map.next_key::<String>()? {
                            match key.as_str() {
                                "entity_ref" => entity_ref = Some(map.next_value()?),
                                #(#field_match_arms)*
                                #extensions_x_arm
                                _ => { let _: ::serde_json::Value = map.next_value()?; }
                            }
                        }

                        #er_required

                        let tracked = #tracked_name {
                            #er_struct_field
                            #(#field_arc_inits)*
                        };
                        #(#field_init_calls)*
                        #extensions_init_call
                        Ok(tracked)
                    }
                }

                d.deserialize_map(V)
            }
        }
    }
}

fn generate_entity_impl(
    name: &Ident,
    tracked_name: &Ident,
    kind_expr: &TokenStream2,
    parent_type: &TokenStream2,
    no_dispatch: bool,
    validation_schema_method: TokenStream2,
    variant_name: TokenStream2,
) -> TokenStream2 {
    let to_any_ref_body = if no_dispatch {
        quote! {
            let _ = entity_ref;
            unimplemented!("to_any_ref: no_dispatch is set")
        }
    } else {
        quote! {
            ::pari::entity::AnyEntityRef::#variant_name(entity_ref.clone())
        }
    };

    let extract_body = if no_dispatch {
        quote! {
            let _ = entity;
            unimplemented!("extract: no_dispatch is set")
        }
    } else {
        quote! {
            if let ::pari::entity::TrackedEntity::#variant_name(ref t) = entity {
                ::std::option::Option::Some(t)
            } else {
                ::std::option::Option::None
            }
        }
    };

    quote! {
        impl ::pari::entity::Entity for #name {
            const KIND: ::pari::entity::EntityKind = #kind_expr;

            #validation_schema_method

            type Parent = #parent_type;
            type Tracked = #tracked_name;

            fn to_any_ref(
                entity_ref: &::pari::entity::EntityRef<Self, Self::Parent>,
            ) -> ::pari::entity::AnyEntityRef {
                #to_any_ref_body
            }

            fn extract(
                entity: &::pari::entity::TrackedEntity,
            ) -> ::std::option::Option<&Self::Tracked> {
                #extract_body
            }
        }
    }
}

fn entity_kind_to_any_ref_variant(kind_expr: &TokenStream2) -> TokenStream2 {
    let s = kind_expr.to_string();
    let variant = s.split("::").last().unwrap_or("").trim().to_string();
    let variant_ident = Ident::new(&variant, proc_macro2::Span::call_site());
    quote! { #variant_ident }
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
