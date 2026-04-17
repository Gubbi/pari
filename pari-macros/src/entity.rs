use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;

use crate::{
    entity_codegen::generate_entity_derive_parts,
    validation_codegen::generate_validation_schema_access,
    workspace_codegen::generate_accessors_and_setters,
};

pub fn derive_entity_impl(ast: DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let tracked_name = syn::Ident::new(&format!("Tracked{name}"), name.span());

    let (kind_expr, parent_type, no_dispatch, schema_fn) = parse_entity_attr(&ast);
    let validation_schema_method = generate_validation_schema_access(name, &schema_fn);
    let entity_parts = match generate_entity_derive_parts(
        &ast,
        &tracked_name,
        &kind_expr,
        &parent_type,
        no_dispatch,
        validation_schema_method,
    ) {
        Ok(parts) => parts,
        Err(err) => return err,
    };

    let domain_field_refs = entity_parts.domain_fields.iter().collect::<Vec<_>>();
    let (accessors, setters) = generate_accessors_and_setters(name, &domain_field_refs);

    let crate::entity_codegen::EntityDeriveParts {
        tracked_struct,
        tracked_impl,
        serialize_impl,
        deserialize_impl,
        entity_impl,
        tracked_for_impl,
        domain_fields: _,
    } = entity_parts;

    quote! {
        #tracked_struct
        #tracked_impl

        impl #tracked_name {
            #(#accessors)*
            #(#setters)*
        }

        #entity_impl
        #tracked_for_impl
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
