//! `pari-macros` — proc-macro crate providing `#[derive(Tracked)]` for pari entities.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, GenericParam, Ident, Type,
};

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
