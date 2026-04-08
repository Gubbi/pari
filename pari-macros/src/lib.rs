//! `pari-macros` — proc-macro crate for pari entities.
//!
//! Provides:
//! - `#[derive(Tracked)]` — change-tracking structs/enums (original)
//! - `#[derive(Entity)]` + `#[entity(...)]` — full entity infrastructure

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericParam, Ident, Token, Type};

// ---------------------------------------------------------------------------
// Public derive entry point
// ---------------------------------------------------------------------------

/// Generates a `TrackedX` struct/enum alongside `From<X> for TrackedX` and a
/// `dirty_fields() -> Vec<&'static str>` method.
///
/// Three cases:
/// - **Flat struct** (no generic params): each field `f: T` becomes `f: Tracked<T>`.
/// - **Struct with generic params**: fields typed as a bare generic param `S` become
///   `TS` (no `Tracked<>` wrap); fields annotated `#[tracked(map_key = "id")]` with
///   type `Vec<Elem>` become `TrackedMap<String, TS>`; all other fields wrap normally.
/// - **Enum** (generic or not): each variant's inner type `SomeType` becomes
///   `TrackedSomeType` (prepend "Tracked"); `Box<T>` handled transparently.
#[proc_macro_derive(Tracked, attributes(tracked))]
pub fn derive_tracked(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let expanded = match &ast.data {
        Data::Struct(_) => derive_struct(&ast),
        Data::Enum(_) => derive_enum(&ast),
        Data::Union(_) => {
            return syn::Error::new_spanned(&ast.ident, "Tracked does not support unions")
                .to_compile_error()
                .into();
        }
    };

    expanded.into()
}

// ---------------------------------------------------------------------------
// Naming helpers
// ---------------------------------------------------------------------------

fn tracked_ident(name: &Ident) -> Ident {
    Ident::new(&format!("Tracked{name}"), name.span())
}

/// For a generic type param `S`, the tracked param name is `TS`.
fn tracked_param_ident(param: &Ident) -> Ident {
    Ident::new(&format!("T{param}"), param.span())
}

/// Collect all type-param idents from a generics declaration.
fn type_param_idents(generics: &syn::Generics) -> Vec<Ident> {
    generics
        .params
        .iter()
        .filter_map(|p| {
            if let GenericParam::Type(tp) = p {
                Some(tp.ident.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Returns true if `ty` is exactly the path `Foo` matching one of `generic_params`.
fn is_bare_generic_param(ty: &Type, generic_params: &[Ident]) -> bool {
    if let Type::Path(tp) = ty {
        if tp.qself.is_none() && tp.path.segments.len() == 1 {
            let seg = &tp.path.segments[0];
            if seg.arguments.is_none() || matches!(seg.arguments, syn::PathArguments::None) {
                return generic_params.iter().any(|p| *p == seg.ident);
            }
        }
    }
    false
}

/// For a field whose type is exactly a generic param `S`, return the tracked
/// param ident `TS`.
fn bare_generic_to_tracked(ty: &Type, generic_params: &[Ident]) -> Option<Ident> {
    if let Type::Path(tp) = ty {
        if tp.qself.is_none() && tp.path.segments.len() == 1 {
            let seg = &tp.path.segments[0];
            if seg.arguments.is_none() || matches!(seg.arguments, syn::PathArguments::None) {
                for p in generic_params {
                    if *p == seg.ident {
                        return Some(tracked_param_ident(p));
                    }
                }
            }
        }
    }
    None
}

/// Extract the inner type from `Vec<Inner>`, returning `Some(inner)`.
fn vec_element_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if tp.path.segments.len() == 1 {
            let seg = &tp.path.segments[0];
            if seg.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if args.args.len() == 1 {
                        if let syn::GenericArgument::Type(inner) = &args.args[0] {
                            return Some(inner);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Given `Vec<Elem>` where `Elem` contains a generic param, return the tracked
/// generic param that corresponds to `Elem`.  For `Vec<Step<S>>` with generic
/// param `S`, this returns `TS`.
fn vec_elem_tracked_param(ty: &Type, generic_params: &[Ident]) -> Option<Ident> {
    let elem = vec_element_type(ty)?;
    // If the element is exactly a generic param
    if let Some(tp) = bare_generic_to_tracked(elem, generic_params) {
        return Some(tp);
    }
    // If the element is a parameterized type like `Step<S>` containing a generic param
    if let Type::Path(tp) = elem {
        if tp.path.segments.len() == 1 {
            let seg = &tp.path.segments[0];
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                for arg in &args.args {
                    if let syn::GenericArgument::Type(inner) = arg {
                        if let Some(tp) = bare_generic_to_tracked(inner, generic_params) {
                            return Some(tp);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extract the element type from `Vec<Elem>` inside a type.
fn vec_elem_type_for_bound(ty: &Type) -> Option<&Type> {
    vec_element_type(ty)
}

/// Check if `#[tracked(map_key = "id")]` (or any value) is present on a field.
fn has_map_key_attr(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("tracked") {
            // Parse the attribute argument as `map_key = "..."`
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("map_key") {
                    Ok(())
                } else {
                    Err(meta.error("unknown tracked attribute"))
                }
            });
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Struct derivation
// ---------------------------------------------------------------------------

fn derive_struct(ast: &DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let tracked_name = tracked_ident(name);
    let generic_params = type_param_idents(&ast.generics);

    let fields = match &ast.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return syn::Error::new_spanned(name, "Tracked only supports named-field structs")
                    .to_compile_error();
            }
        },
        _ => unreachable!(),
    };

    // Build tracked struct field declarations
    let tracked_field_decls: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let field_name = &f.ident;
            let vis = &f.vis;
            let ty = &f.ty;

            if has_map_key_attr(&f.attrs) {
                // Vec<Elem> → TrackedMap<String, TX>
                let tracked_param = vec_elem_tracked_param(ty, &generic_params)
                    .unwrap_or_else(|| Ident::new("TS", Span::call_site()));
                quote! { #vis #field_name: crate::tracked::TrackedMap<String, #tracked_param>, }
            } else if let Some(tracked_param) = bare_generic_to_tracked(ty, &generic_params) {
                // bare generic param S → TS (no Tracked<> wrap)
                quote! { #vis #field_name: #tracked_param, }
            } else {
                // concrete type → Tracked<T>
                quote! { #vis #field_name: crate::tracked::Tracked<#ty>, }
            }
        })
        .collect();

    // Build From impl field conversions
    let from_field_inits: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let field_name = &f.ident;
            let ty = &f.ty;

            if has_map_key_attr(&f.attrs) {
                // TrackedMap::from_vec using a closure to avoid turbofish parsing issues
                let tracked_param = vec_elem_tracked_param(ty, &generic_params)
                    .unwrap_or_else(|| Ident::new("TS", Span::call_site()));
                let elem_ty = vec_elem_type_for_bound(ty);
                if let Some(elem_ty) = elem_ty {
                    quote! {
                        #field_name: crate::tracked::TrackedMap::from_vec(plain.#field_name, |item| <#tracked_param as From<#elem_ty>>::from(item)),
                    }
                } else {
                    quote! {
                        #field_name: crate::tracked::TrackedMap::from_vec(plain.#field_name, |item| #tracked_param::from(item)),
                    }
                }
            } else if bare_generic_to_tracked(ty, &generic_params).is_some() {
                // bare generic S → TS::from(plain.field) using UFCS
                let tracked_param = bare_generic_to_tracked(ty, &generic_params).unwrap();
                quote! { #field_name: <#tracked_param as From<#ty>>::from(plain.#field_name), }
            } else {
                quote! { #field_name: crate::tracked::Tracked::new(plain.#field_name), }
            }
        })
        .collect();

    // Build dirty_fields method body
    let dirty_checks: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let field_name = &f.ident;
            let field_name_str = field_name
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_default();
            let ty = &f.ty;

            if has_map_key_attr(&f.attrs) {
                // TrackedMap field: check has_changes()
                quote! {
                    if self.#field_name.has_changes() { fields.push(#field_name_str); }
                }
            } else if is_bare_generic_param(ty, &generic_params) {
                // bare generic param: skip in dirty_fields
                quote! {}
            } else {
                // Tracked<T> field: check is_dirty()
                quote! {
                    if self.#field_name.is_dirty() { fields.push(#field_name_str); }
                }
            }
        })
        .collect();

    // Build tracked generic params for the tracked struct (S → TS)
    let tracked_struct_params: Vec<TokenStream2> = generic_params
        .iter()
        .map(|p| {
            let tp = tracked_param_ident(p);
            quote! { #tp }
        })
        .collect();

    // Build From impl generic params (both original S and tracked TS)
    // Also include where clauses from the original struct
    let from_impl_params: Vec<TokenStream2> = generic_params
        .iter()
        .flat_map(|p| {
            let tp = tracked_param_ident(p);
            vec![quote! { #p }, quote! { #tp }]
        })
        .collect();

    // Original struct where clause predicates (forward them to From impl)
    let original_where_preds: Vec<TokenStream2> = ast
        .generics
        .where_clause
        .as_ref()
        .map(|wc| wc.predicates.iter().map(|p| quote! { #p }).collect())
        .unwrap_or_default();

    // Extra bounds for the From impl
    let mut extra_bounds: Vec<TokenStream2> = Vec::new();
    for f in fields.iter() {
        let ty = &f.ty;
        if has_map_key_attr(&f.attrs) {
            // TS: From<ElemType>  +  ElemType: HasId
            if let Some(elem_ty) = vec_elem_type_for_bound(ty) {
                let tracked_param = vec_elem_tracked_param(ty, &generic_params)
                    .unwrap_or_else(|| Ident::new("TS", Span::call_site()));
                extra_bounds.push(quote! { #tracked_param: From<#elem_ty> });
                extra_bounds.push(quote! { #elem_ty: crate::tracked::HasId });
            }
        } else if let Some(tracked_param) = bare_generic_to_tracked(ty, &generic_params) {
            // TS: From<S>
            extra_bounds.push(quote! { #tracked_param: From<#ty> });
        }
    }

    let all_where_preds: Vec<TokenStream2> = original_where_preds
        .into_iter()
        .chain(extra_bounds.into_iter())
        .collect();

    let where_clause = if all_where_preds.is_empty() {
        quote! {}
    } else {
        quote! { where #(#all_where_preds),* }
    };

    // Original struct args: <S> (for use in From<OrigStruct<S>>)
    let orig_struct_args: Vec<TokenStream2> = generic_params
        .iter()
        .map(|p| quote! { #p })
        .collect();

    // Tracked struct generic args: <TS> (for struct def and tracked impl block)
    let tracked_args = if tracked_struct_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#tracked_struct_params),*> }
    };

    // Original struct generic args for From source: <S> or empty
    let orig_args = if orig_struct_args.is_empty() {
        quote! {}
    } else {
        quote! { <#(#orig_struct_args),*> }
    };

    // Full impl generics header: <S, TS> or empty
    let full_from_generics = if from_impl_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#from_impl_params),*> }
    };

    // Visibility of the tracked struct matches the original
    let vis = &ast.vis;

    quote! {
        #vis struct #tracked_name #tracked_args {
            #(#tracked_field_decls)*
        }

        impl #full_from_generics From<#name #orig_args> for #tracked_name #tracked_args
        #where_clause
        {
            fn from(plain: #name #orig_args) -> Self {
                Self {
                    #(#from_field_inits)*
                }
            }
        }

        impl #tracked_args #tracked_name #tracked_args {
            pub fn dirty_fields(&self) -> Vec<&'static str> {
                let mut fields: Vec<&'static str> = Vec::new();
                #(#dirty_checks)*
                fields
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Enum derivation
// ---------------------------------------------------------------------------

/// Transform an inner variant type for the tracked enum.
/// `SomeType` → `TrackedSomeType`
/// `Box<SomeType>` → `Box<TrackedSomeType>`
/// `SomeType<S>` where S is a generic param → `TrackedSomeType<TS>`
fn transform_variant_inner_type(ty: &Type, generic_params: &[Ident]) -> TokenStream2 {
    match ty {
        Type::Path(tp) => {
            if tp.qself.is_none() && tp.path.segments.len() == 1 {
                let seg = &tp.path.segments[0];
                let orig_name = &seg.ident;

                // Check if this is `Box<Inner>`
                if orig_name == "Box" {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        if args.args.len() == 1 {
                            if let syn::GenericArgument::Type(inner) = &args.args[0] {
                                let transformed = transform_variant_inner_type(inner, generic_params);
                                return quote! { Box<#transformed> };
                            }
                        }
                    }
                }

                let tracked_type_name = tracked_ident(orig_name);

                // Transform generic arguments: replace param S → TS
                match &seg.arguments {
                    syn::PathArguments::None => {
                        quote! { #tracked_type_name }
                    }
                    syn::PathArguments::AngleBracketed(args) => {
                        let new_args: Vec<TokenStream2> = args
                            .args
                            .iter()
                            .map(|arg| match arg {
                                syn::GenericArgument::Type(inner_ty) => {
                                    if let Some(tp) =
                                        bare_generic_to_tracked(inner_ty, generic_params)
                                    {
                                        quote! { #tp }
                                    } else {
                                        transform_variant_inner_type(inner_ty, generic_params)
                                    }
                                }
                                _ => quote! { #arg },
                            })
                            .collect();
                        quote! { #tracked_type_name<#(#new_args),*> }
                    }
                    _ => quote! { #tracked_type_name },
                }
            } else {
                // multi-segment path or other — just quote as-is
                quote! { #ty }
            }
        }
        _ => quote! { #ty },
    }
}

fn derive_enum(ast: &DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let tracked_name = tracked_ident(name);
    let generic_params = type_param_idents(&ast.generics);

    let variants = match &ast.data {
        Data::Enum(e) => &e.variants,
        _ => unreachable!(),
    };

    // Build tracked enum variant declarations
    let tracked_variants: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let vname = &v.ident;
            match &v.fields {
                Fields::Unnamed(f) => {
                    let transformed: Vec<TokenStream2> = f
                        .unnamed
                        .iter()
                        .map(|field| transform_variant_inner_type(&field.ty, &generic_params))
                        .collect();
                    quote! { #vname(#(#transformed),*) }
                }
                Fields::Unit => quote! { #vname },
                Fields::Named(_) => {
                    // Named-field variants: not expected for our entities
                    quote! { #vname }
                }
            }
        })
        .collect();

    // Build From match arms
    let from_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let vname = &v.ident;
            match &v.fields {
                Fields::Unnamed(f) => {
                    if f.unnamed.len() == 1 {
                        let inner_ty = &f.unnamed[0].ty;
                        let converted = convert_variant_value(inner_ty, &generic_params);
                        quote! {
                            #name::#vname(val) => #tracked_name::#vname(#converted),
                        }
                    } else {
                        let bindings: Vec<_> = (0..f.unnamed.len())
                            .map(|i| Ident::new(&format!("v{i}"), Span::call_site()))
                            .collect();
                        let inits: Vec<TokenStream2> = f.unnamed.iter().zip(bindings.iter())
                            .map(|(field, b)| convert_variant_value_named(&field.ty, &generic_params, b))
                            .collect();
                        quote! {
                            #name::#vname(#(#bindings),*) => #tracked_name::#vname(#(#inits),*),
                        }
                    }
                }
                Fields::Unit => quote! {
                    #name::#vname => #tracked_name::#vname,
                },
                _ => quote! {},
            }
        })
        .collect();

    // Build dirty_fields match arms
    let dirty_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let vname = &v.ident;
            match &v.fields {
                Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                    let inner_ty = &f.unnamed[0].ty;
                    // Box<T> case: need to deref
                    if is_box_type(inner_ty) {
                        quote! {
                            Self::#vname(val) => val.dirty_fields(),
                        }
                    } else {
                        quote! {
                            Self::#vname(val) => val.dirty_fields(),
                        }
                    }
                }
                Fields::Unit => quote! {
                    Self::#vname => vec![],
                },
                _ => quote! {
                    Self::#vname(..) => vec![],
                },
            }
        })
        .collect();

    // Build where clause for From impl (one From bound per variant's inner type)
    // For Box<Inner>: bound is on Inner (not Box<Tracked>): TrackedInner: From<Inner>
    let mut from_bounds: Vec<TokenStream2> = Vec::new();
    for v in variants.iter() {
        if let Fields::Unnamed(f) = &v.fields {
            if f.unnamed.len() == 1 {
                let inner_ty = &f.unnamed[0].ty;
                if is_box_type(inner_ty) {
                    // Box<Inner>: generate TrackedInner: From<Inner>
                    if let Type::Path(tp) = inner_ty {
                        if let syn::PathArguments::AngleBracketed(args) =
                            &tp.path.segments[0].arguments
                        {
                            if let Some(syn::GenericArgument::Type(boxed_inner)) =
                                args.args.first()
                            {
                                let transformed =
                                    transform_variant_inner_type(boxed_inner, &generic_params);
                                from_bounds
                                    .push(quote! { #transformed: From<#boxed_inner> });
                            }
                        }
                    }
                } else {
                    let transformed = transform_variant_inner_type(inner_ty, &generic_params);
                    from_bounds.push(quote! { #transformed: From<#inner_ty> });
                }
            }
        }
    }

    // Tracked struct generic params (S → TS)
    let tracked_struct_params: Vec<TokenStream2> = generic_params
        .iter()
        .map(|p| {
            let tp = tracked_param_ident(p);
            quote! { #tp }
        })
        .collect();

    // From impl generic params (both S and TS)
    let from_impl_params: Vec<TokenStream2> = generic_params
        .iter()
        .flat_map(|p| {
            let tp = tracked_param_ident(p);
            vec![quote! { #p }, quote! { #tp }]
        })
        .collect();

    let original_where_preds: Vec<TokenStream2> = ast
        .generics
        .where_clause
        .as_ref()
        .map(|wc| wc.predicates.iter().map(|p| quote! { #p }).collect())
        .unwrap_or_default();

    let all_where_preds: Vec<TokenStream2> = original_where_preds
        .into_iter()
        .chain(from_bounds.into_iter())
        .collect();

    let where_clause = if all_where_preds.is_empty() {
        quote! {}
    } else {
        quote! { where #(#all_where_preds),* }
    };

    // Original enum args: <S> or empty
    let orig_enum_args: Vec<TokenStream2> = generic_params.iter().map(|p| quote! { #p }).collect();

    let tracked_args = if tracked_struct_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#tracked_struct_params),*> }
    };

    let orig_args = if orig_enum_args.is_empty() {
        quote! {}
    } else {
        quote! { <#(#orig_enum_args),*> }
    };

    let full_from_generics = if from_impl_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#from_impl_params),*> }
    };

    let vis = &ast.vis;

    quote! {
        #[allow(clippy::large_enum_variant)]
        #vis enum #tracked_name #tracked_args {
            #(#tracked_variants,)*
        }

        impl #full_from_generics From<#name #orig_args> for #tracked_name #tracked_args
        #where_clause
        {
            fn from(plain: #name #orig_args) -> Self {
                match plain {
                    #(#from_arms)*
                }
            }
        }

        impl #tracked_args #tracked_name #tracked_args {
            pub fn dirty_fields(&self) -> Vec<&'static str> {
                match self {
                    #(#dirty_arms)*
                }
            }
        }
    }
}

/// Generate the conversion expression for a single variant value `val`.
/// Uses UFCS `<TrackedType as From<PlainType>>::from(val)` to avoid ambiguous
/// comparison-operator parsing when `TrackedType` has generic args.
fn convert_variant_value(ty: &Type, generic_params: &[Ident]) -> TokenStream2 {
    if is_box_type(ty) {
        // Box<Inner>: unbox, convert, re-box
        if let Type::Path(tp) = ty {
            if let syn::PathArguments::AngleBracketed(args) = &tp.path.segments[0].arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    let transformed = transform_variant_inner_type(inner, generic_params);
                    return quote! { Box::new(<#transformed as From<#inner>>::from(*val)) };
                }
            }
        }
    }
    let transformed = transform_variant_inner_type(ty, generic_params);
    quote! { <#transformed as From<#ty>>::from(val) }
}

fn convert_variant_value_named(
    ty: &Type,
    generic_params: &[Ident],
    binding: &Ident,
) -> TokenStream2 {
    if is_box_type(ty) {
        if let Type::Path(tp) = ty {
            if let syn::PathArguments::AngleBracketed(args) = &tp.path.segments[0].arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    let transformed = transform_variant_inner_type(inner, generic_params);
                    return quote! { Box::new(<#transformed as From<#inner>>::from(*#binding)) };
                }
            }
        }
    }
    let transformed = transform_variant_inner_type(ty, generic_params);
    quote! { <#transformed as From<#ty>>::from(#binding) }
}

fn is_box_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if tp.path.segments.len() == 1 {
            return tp.path.segments[0].ident == "Box";
        }
    }
    false
}

// ===========================================================================
// #[entity(...)] attribute macro — stores kind/parent for #[derive(Entity)]
// ===========================================================================

// ===========================================================================
// #[derive(Entity)] — generates TrackedX, From, accessors, setters, dirty ops,
//                     Entity impl, TrackedFor impl
// ===========================================================================

#[proc_macro_derive(Entity, attributes(entity))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    derive_entity_impl(ast).into()
}

fn derive_entity_impl(ast: DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let tracked_name = Ident::new(&format!("Tracked{name}"), name.span());

    // --- Parse #[entity(kind = ..., parent = ..., no_dispatch, schema = ...)] ---
    let (kind_expr, parent_type, no_dispatch, schema_fn) = parse_entity_attr(&ast);

    // --- Extract fields ---
    let fields = match &ast.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return syn::Error::new_spanned(name, "Entity only supports named-field structs")
                    .to_compile_error()
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "Entity only supports structs")
                .to_compile_error()
        }
    };

    // Separate entity_ref field from domain fields
    let entity_ref_field = fields.iter().find(|f| {
        f.ident.as_ref().map(|i| i == "entity_ref").unwrap_or(false)
    });
    let domain_fields: Vec<_> = fields
        .iter()
        .filter(|f| f.ident.as_ref().map(|i| i != "entity_ref").unwrap_or(true))
        .collect();

    let entity_ref_type = entity_ref_field.map(|f| &f.ty);

    // --- Build TrackedX struct ---
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

    // --- Build From<X> for TrackedX ---
    let from_field_inits: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            quote! {
                #fname: ::std::sync::Arc::new(
                    ::pari::tracked::TrackedField::new_initialized(plain.#fname)
                ),
            }
        })
        .collect();

    let entity_ref_from = if entity_ref_field.is_some() {
        quote! { entity_ref: plain.entity_ref, }
    } else {
        quote! {}
    };

    // --- Build entity_ref accessor ---
    let entity_ref_accessor = if let Some(ty) = entity_ref_type {
        quote! {
            pub fn entity_ref(&self) -> &#ty {
                &self.entity_ref
            }
        }
    } else {
        quote! {}
    };

    // --- Build async accessors ---
    let accessors: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let ty = &f.ty;
            let (ret_type, map_expr) = accessor_return_type(ty);
            quote! {
                pub async fn #fname(&self) -> ::std::result::Result<#ret_type, ::pari::entity::LoadError> {
                    self.#fname.get_or_load().await #map_expr
                }
            }
        })
        .collect();

    // --- Build async setters ---
    let setters: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let setter_name = Ident::new(&format!("set_{}", fname.as_ref().unwrap()), Span::call_site());
            let ty = &f.ty;
            quote! {
                pub async fn #setter_name(&mut self, value: #ty) -> ::std::result::Result<(), ::pari::entity::SetterError> {
                    self.ensure_mutable().await?;
                    self.#fname = ::std::sync::Arc::new(::pari::tracked::TrackedField::with_value(value));
                    Ok(())
                }
            }
        })
        .collect();

    // --- Build dirty ops ---
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
            let ty = &f.ty;
            quote! {
                if self.#fname.is_dirty() {
                    if let Some(v) = self.#fname.get() {
                        self.#fname = ::std::sync::Arc::new(
                            ::pari::tracked::TrackedField::new_initialized(<#ty as ::std::clone::Clone>::clone(v))
                        );
                    }
                }
            }
        })
        .collect();

    let has_dirty_expr = if has_dirty_checks.is_empty() {
        quote! { false }
    } else {
        quote! { #(#has_dirty_checks)||* }
    };

    // --- Entity impl: to_any_ref and extract (real bodies unless no_dispatch) ---
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
            if let ::pari::entity::StoreEntity::#variant_name(ref t) = entity {
                ::std::option::Option::Some(t)
            } else {
                ::std::option::Option::None
            }
        }
    };

    let vis = &ast.vis;

    // --- Serialize impl ---
    // entity_ref is always serialized first.
    // extensions field is flattened (keys merged into the top-level map).
    // Other fields are included only when initialized (TrackedField::get() is Some).
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

    // --- Deserialize impl ---
    // entity_ref is required; all other fields are optional (absent = uninitialized).
    // x-prefixed keys are collected into the extensions field.

    let er_ty = entity_ref_type;

    let er_accum = if let Some(ty) = er_ty {
        quote! { let mut entity_ref: ::std::option::Option<#ty> = None; }
    } else {
        quote! {}
    };

    let has_extensions_field = domain_fields
        .iter()
        .any(|f| f.ident.as_ref().map(|i| i == "extensions").unwrap_or(false));

    // Accumulators for non-extensions domain fields
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

    // Match arms for non-extensions fields
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

    // TrackedField::new() for each domain field in the struct literal
    let field_arc_inits: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            quote! {
                #fname: ::std::sync::Arc::new(::pari::tracked::TrackedField::new()),
            }
        })
        .collect();

    // initialize() calls for non-extensions fields
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

    // --- Build make_stub method ---
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

    // --- Build all_refs method ---
    // Collect fields whose type is EntityRef<T> (single path segment with generic arg named "EntityRef")
    let all_refs_pushes: Vec<TokenStream2> = domain_fields
        .iter()
        .filter_map(|f| {
            let fname = &f.ident;
            let ty = &f.ty;
            // Check if type is EntityRef<T>
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

    // --- Build initialize_into method ---
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

            async fn ensure_mutable(&mut self) -> ::std::result::Result<(), ::pari::entity::SetterError> {
                // Stub: no-op. Task 09 replaces with EntityServer load-all-fields call.
                Ok(())
            }

            #make_stub_body

            #all_refs_method

            #initialize_into_method
        }

        impl ::pari::entity::Entity for #name {
            const KIND: ::pari::entity::EntityKind = #kind_expr;

            fn validation_schema() -> &'static ::pari::entity::ValidationSchema<Self> {
                static S: ::std::sync::OnceLock<::pari::entity::ValidationSchema<#name>> =
                    ::std::sync::OnceLock::new();
                S.get_or_init(|| #schema_fn)
            }

            type Parent = #parent_type;
            type Tracked = #tracked_name;

            fn to_any_ref(
                entity_ref: &::pari::entity::EntityRef<Self, Self::Parent>,
            ) -> ::pari::entity::AnyEntityRef {
                #to_any_ref_body
            }

            fn extract(
                entity: &::pari::entity::StoreEntity,
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

/// Parse `#[entity(kind = ..., parent = ..., no_dispatch, schema = ...)]` from the derive input's attributes.
/// Returns `(kind_expr, parent_type, no_dispatch, schema_call)`.
fn parse_entity_attr(ast: &DeriveInput) -> (TokenStream2, TokenStream2, bool, TokenStream2) {
    let mut kind_expr: Option<TokenStream2> = None;
    let mut parent_type: Option<TokenStream2> = None;
    let mut no_dispatch = false;
    let mut schema_fn: Option<TokenStream2> = None;

    for attr in &ast.attrs {
        if !attr.path().is_ident("entity") {
            continue;
        }
        // Parse as a list of `key = value` pairs or bare flags
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("kind") {
                let value = meta.value()?;
                // Parse as Expr (stops at comma) rather than greedy TokenStream
                let expr: syn::Expr = value.parse()?;
                kind_expr = Some(quote! { #expr });
            } else if meta.path.is_ident("parent") {
                let value = meta.value()?;
                // Parse as Type (stops at comma)
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

/// Derive the `AnyEntityRef` variant name from an EntityKind expression.
/// e.g. `EntityKind::Role` → `Role`
fn entity_kind_to_any_ref_variant(kind_expr: &TokenStream2) -> TokenStream2 {
    // Extract the last path segment (the variant name) as an Ident
    let s = kind_expr.to_string();
    let variant = s.split("::").last().unwrap_or("").trim().to_string();
    let variant_ident = Ident::new(&variant, Span::call_site());
    quote! { #variant_ident }
}


/// Map a domain field type to the accessor's return type and conversion expression.
///
/// `String`            → `&str`,           `.map(|v| v.as_str())`
/// `Option<String>`    → `Option<&str>`,   `.map(|o| o.as_deref())`
/// `Vec<U>`            → `&[U]`,           `.map(|v| v.as_slice())`
/// `Option<Vec<U>>`    → `Option<&[U]>`,   `.map(|o| o.as_deref())`
/// `T` (other)         → `&T`,             (identity — no map needed)
fn accessor_return_type(ty: &Type) -> (TokenStream2, TokenStream2) {
    match ty {
        Type::Path(tp) if tp.qself.is_none() => {
            let segs = &tp.path.segments;
            if segs.len() == 1 {
                let seg = &segs[0];
                if seg.ident == "String" {
                    return (quote! { &str }, quote! { .map(|v| v.as_str()) });
                }
                if seg.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        if args.args.len() == 1 {
                            if let syn::GenericArgument::Type(inner) = &args.args[0] {
                                // Option<String>
                                if is_type_ident(inner, "String") {
                                    return (
                                        quote! { ::std::option::Option<&str> },
                                        quote! { .map(|o| o.as_deref()) },
                                    );
                                }
                                // Option<Vec<U>>
                                if let Some(elem) = vec_inner_type(inner) {
                                    return (
                                        quote! { ::std::option::Option<&[#elem]> },
                                        quote! { .map(|o| o.as_deref()) },
                                    );
                                }
                                // Option<T>
                                return (
                                    quote! { ::std::option::Option<&#inner> },
                                    quote! { .map(|o| o.as_ref()) },
                                );
                            }
                        }
                    }
                }
                if seg.ident == "Vec" {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        if args.args.len() == 1 {
                            if let syn::GenericArgument::Type(elem) = &args.args[0] {
                                return (quote! { &[#elem] }, quote! { .map(|v| v.as_slice()) });
                            }
                        }
                    }
                }
            }
            (quote! { &#ty }, quote! {})
        }
        _ => (quote! { &#ty }, quote! {}),
    }
}

fn is_type_ident(ty: &Type, name: &str) -> bool {
    if let Type::Path(tp) = ty {
        if tp.qself.is_none() && tp.path.segments.len() == 1 {
            return tp.path.segments[0].ident == name;
        }
    }
    false
}

fn vec_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if tp.path.segments.len() == 1 {
            let seg = &tp.path.segments[0];
            if seg.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if args.args.len() == 1 {
                        if let syn::GenericArgument::Type(inner) = &args.args[0] {
                            return Some(inner);
                        }
                    }
                }
            }
        }
    }
    None
}

// ===========================================================================
// entity_registry! — generates EntityKind, AnyEntityRef, StoreEntity,
//                    SubstrateSchema stub, per-entity schema stubs, load_strategy
// ===========================================================================

struct RegistryEntry {
    name: Ident,
    parent: Ident,
}

struct RegistryInput(Vec<RegistryEntry>);

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

#[proc_macro]
pub fn entity_registry(input: TokenStream) -> TokenStream {
    let registry = parse_macro_input!(input as RegistryInput);
    generate_registry(registry.0).into()
}

/// Convert a CamelCase identifier string to snake_case.
/// e.g. "ArtifactKind" -> "artifact_kind", "Role" -> "role"
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

fn generate_registry(entries: Vec<RegistryEntry>) -> TokenStream2 {
    let variants: Vec<&Ident> = entries.iter().map(|e| &e.name).collect();
    let _parents: Vec<&Ident> = entries.iter().map(|e| &e.parent).collect();
    let tracked_names: Vec<Ident> = entries
        .iter()
        .map(|e| Ident::new(&format!("Tracked{}", e.name), e.name.span()))
        .collect();
    let schema_names: Vec<Ident> = entries
        .iter()
        .map(|e| Ident::new(&format!("{}Schema", e.name), e.name.span()))
        .collect();

    // --- EntityKind ---
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

    // --- AnyEntityRef ---
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

    // parent() arms: WorkflowParent entities point to Workflow; NoParent entities return None
    let parent_arms: Vec<TokenStream2> = entries
        .iter()
        .map(|e| {
            let name = &e.name;
            if e.parent == "WorkflowParent" {
                quote! {
                    AnyEntityRef::#name(r) =>
                        Some(AnyEntityRef::Workflow(EntityRef::new(r.parent.workflow_id.clone()))),
                }
            } else {
                quote! { AnyEntityRef::#name(_) => None, }
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

            /// Returns the parent as an `AnyEntityRef::Workflow` for embedded entities,
            /// or `None` for top-level entities.
            pub fn parent(&self) -> Option<AnyEntityRef> {
                match self { #(#parent_arms)* }
            }
        }
    };

    // --- StoreEntity (store-level enum) ---
    let from_role_methods: Vec<TokenStream2> = entries
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
        .zip(tracked_names.iter())
        .map(|(e, _tname)| {
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
        .zip(tracked_names.iter())
        .map(|(e, _tname)| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => e.all_refs(),
            }
        })
        .collect();

    let initialize_into_arms: Vec<TokenStream2> = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, tname)| {
            let vname = &e.name;
            quote! {
                (StoreEntity::#vname(src), StoreEntity::#vname(dst)) => src.initialize_into(dst),
            }
        })
        .collect();

    let merge_dirty_into_arms: Vec<TokenStream2> = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, _tname)| {
            let vname = &e.name;
            quote! {
                (StoreEntity::#vname(src), StoreEntity::#vname(dst)) => src.merge_dirty_into(dst),
            }
        })
        .collect();

    let has_dirty_arms: Vec<TokenStream2> = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, _tname)| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => e.has_dirty_fields(),
            }
        })
        .collect();

    let dirty_fields_arms: Vec<TokenStream2> = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, _tname)| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => e.dirty_fields(),
            }
        })
        .collect();

    let reset_dirty_arms: Vec<TokenStream2> = entries
        .iter()
        .zip(tracked_names.iter())
        .map(|(e, _tname)| {
            let vname = &e.name;
            quote! {
                StoreEntity::#vname(e) => e.reset_dirty(),
            }
        })
        .collect();

    let store_entity = quote! {
        #[derive(Clone)]
        pub enum StoreEntity {
            #(#variants(#tracked_names),)*
        }

        impl StoreEntity {
            #(#from_role_methods)*

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
        }
    };

    // --- SubstrateSchema stub trait ---
    let substrate_schema = quote! {
        /// Stub trait — Task 10 provides the real definition.
        pub trait SubstrateSchema: Send + Sync {
            fn kind(&self) -> EntityKind;
        }
    };

    // --- Per-entity schema stubs ---
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

    // --- load_strategy ---
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
