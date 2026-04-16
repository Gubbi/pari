use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Type};

use crate::{
    validation_codegen::generate_validation_schema_access,
    workspace_codegen::generate_accessors_and_setters,
};

pub fn derive_entity_impl(ast: DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let tracked_name = Ident::new(&format!("Tracked{name}"), name.span());

    let (kind_expr, parent_type, no_dispatch, schema_fn) = parse_entity_attr(&ast);

    let fields = match &ast.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return syn::Error::new_spanned(name, "Entity only supports named-field structs")
                    .to_compile_error()
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "Entity only supports structs").to_compile_error()
        }
    };

    let entity_ref_field = fields
        .iter()
        .find(|f| f.ident.as_ref().map(|i| i == "entity_ref").unwrap_or(false));
    let domain_fields: Vec<_> = fields
        .iter()
        .filter(|f| f.ident.as_ref().map(|i| i != "entity_ref").unwrap_or(true))
        .collect();

    let entity_ref_type = entity_ref_field.map(|f| &f.ty);

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

    let entity_ref_from = if entity_ref_field.is_some() {
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

    let (accessors, setters) = generate_accessors_and_setters(name, &domain_fields);

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

    let variant_name = entity_kind_to_any_ref_variant(&kind_expr);

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

    let vis = &ast.vis;

    let er_serialize = if entity_ref_field.is_some() {
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

    let serialize_impl = quote! {
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
    };

    let er_ty = entity_ref_type;
    let er_accum = if let Some(ty) = er_ty {
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
        quote! { if let Some(v) = extensions { tracked.extensions.initialize(v); } }
    } else {
        quote! {}
    };

    let er_required = if entity_ref_field.is_some() {
        quote! {
            let entity_ref = entity_ref
                .ok_or_else(|| ::serde::de::Error::missing_field("entity_ref"))?;
        }
    } else {
        quote! {}
    };

    let er_struct_field = if entity_ref_field.is_some() {
        quote! { entity_ref, }
    } else {
        quote! {}
    };

    let tracked_name_str = tracked_name.to_string();

    let deserialize_impl = quote! {
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

    let validation_schema_method = generate_validation_schema_access(name, &schema_fn);

    quote! {
        #[derive(Clone)]
        #vis struct #tracked_name {
            #entity_ref_decl
            #(#tracked_field_decls)*
        }

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
            #(#accessors)*
            #(#setters)*

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

        impl ::pari::entity::TrackedFor for #tracked_name {
            type Entity = #name;
        }

        #serialize_impl
        #deserialize_impl
    }
}

fn parse_entity_attr(ast: &DeriveInput) -> (TokenStream2, TokenStream2, bool, TokenStream2) {
    let mut kind_expr: Option<TokenStream2> = None;
    let mut parent_type: Option<TokenStream2> = None;
    let mut no_dispatch = false;
    let mut schema_fn: Option<TokenStream2> = None;

    for attr in &ast.attrs {
        if !attr.path().is_ident("entity") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("kind") {
                let value = meta.value()?;
                let expr: syn::Expr = value.parse()?;
                kind_expr = Some(quote! { #expr });
            } else if meta.path.is_ident("parent") {
                let value = meta.value()?;
                let ty: syn::Type = value.parse()?;
                parent_type = Some(quote! { #ty });
            } else if meta.path.is_ident("no_dispatch") {
                no_dispatch = true;
            } else if meta.path.is_ident("schema") {
                let value = meta.value()?;
                let path: syn::Path = value.parse()?;
                schema_fn = Some(quote! { #path });
            }
            Ok(())
        });
    }

    let kind = kind_expr.unwrap_or_else(|| {
        quote! { compile_error!("#[entity(kind = EntityKind::...)] is required") }
    });
    let parent = parent_type.unwrap_or_else(|| quote! { ::pari::entity::NoParent });
    let schema_call = match &schema_fn {
        Some(path) => quote! { #path() },
        None => quote! { ::pari::entity::ValidationSchema::empty() },
    };

    (kind, parent, no_dispatch, schema_call)
}

fn entity_kind_to_any_ref_variant(kind_expr: &TokenStream2) -> TokenStream2 {
    let s = kind_expr.to_string();
    let variant = s.split("::").last().unwrap_or("").trim().to_string();
    let variant_ident = Ident::new(&variant, Span::call_site());
    quote! { #variant_ident }
}
