//! `#[derive(ErrorCompose)]` proc macro implementation.

use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

// ---------------------------------------------------------------------------
// darling input types
// ---------------------------------------------------------------------------

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(compose), supports(struct_any, enum_any))]
struct ErrorComposeInput {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<ErrorComposeVariant, ErrorComposeField>,
    // Type-level compose attrs (structs)
    fix: Option<syn::Path>,
    recoverability: Option<syn::Path>,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(compose))]
struct ErrorComposeVariant {
    ident: syn::Ident,
    fields: ast::Fields<ErrorComposeField>,
    // Variant-level attrs for delegating or declaring
    #[darling(default)]
    delegate: bool,
    fix: Option<syn::Path>,
    recoverability: Option<syn::Path>,
}

#[derive(Debug, FromField)]
#[darling(attributes(compose))]
struct ErrorComposeField {
    #[allow(dead_code)]
    ident: Option<syn::Ident>,
    #[allow(dead_code)]
    ty: syn::Type,
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

pub fn derive_error_compose(input: DeriveInput) -> TokenStream {
    let parsed = match ErrorComposeInput::from_derive_input(&input) {
        Ok(v) => v,
        Err(e) => return e.write_errors(),
    };

    let name = &parsed.ident;
    let (impl_generics, ty_generics, where_clause) = parsed.generics.split_for_impl();

    match &parsed.data {
        ast::Data::Struct(_) => {
            // Activity layer: must have fix and recoverability on the type
            let fix = match &parsed.fix {
                Some(p) => p.clone(),
                None => return syn::Error::new_spanned(
                    name,
                    "#[derive(ErrorCompose)] on a struct requires #[compose(fix = ..., recoverability = ...)]",
                ).to_compile_error(),
            };
            let recoverability = match &parsed.recoverability {
                Some(p) => p.clone(),
                None => return syn::Error::new_spanned(
                    name,
                    "#[derive(ErrorCompose)] on a struct requires #[compose(fix = ..., recoverability = ...)]",
                ).to_compile_error(),
            };

            quote! {
                impl #impl_generics ::pari::error::ErrorCompose for #name #ty_generics #where_clause {
                    fn fix_domain(&self) -> ::pari::error::FixDomain {
                        ::pari::error::FixDomain::#fix
                    }
                    fn recoverability(&self) -> ::pari::error::Recoverability {
                        ::pari::error::Recoverability::#recoverability
                    }
                }
            }
        }

        ast::Data::Enum(variants) => {
            let mut fix_arms = Vec::new();
            let mut rec_arms = Vec::new();
            let mut inner_arms = Vec::new();

            for variant in variants {
                let vname = &variant.ident;

                if variant.delegate {
                    // Delegating variant — single field, delegate to inner ErrorCompose
                    let field_count = variant.fields.len();
                    if field_count != 1 {
                        return syn::Error::new_spanned(
                            vname,
                            "#[compose(delegate)] variant must have exactly one field",
                        )
                        .to_compile_error();
                    }
                    fix_arms.push(quote! {
                        Self::#vname(inner) => ::pari::error::ErrorCompose::fix_domain(inner),
                    });
                    rec_arms.push(quote! {
                        Self::#vname(inner) => ::pari::error::ErrorCompose::recoverability(inner),
                    });
                    inner_arms.push(quote! {
                        Self::#vname(inner) => ::std::option::Option::Some(inner as &dyn ::std::any::Any),
                    });
                } else if variant.fix.is_some() && variant.recoverability.is_some() {
                    // Declaring variant — literal values
                    let fix = variant.fix.as_ref().unwrap();
                    let rec = variant.recoverability.as_ref().unwrap();
                    let pattern = variant_wildcard_pattern(vname, &variant.fields);
                    fix_arms.push(quote! {
                        #pattern => ::pari::error::FixDomain::#fix,
                    });
                    rec_arms.push(quote! {
                        #pattern => ::pari::error::Recoverability::#rec,
                    });
                    inner_arms.push(quote! {
                        #pattern => ::std::option::Option::None,
                    });
                } else {
                    return syn::Error::new_spanned(
                        vname,
                        "enum variant must have either #[compose(delegate)] \
                         or #[compose(fix = ..., recoverability = ...)]",
                    )
                    .to_compile_error();
                }
            }

            quote! {
                impl #impl_generics ::pari::error::ErrorCompose for #name #ty_generics #where_clause {
                    fn fix_domain(&self) -> ::pari::error::FixDomain {
                        match self {
                            #(#fix_arms)*
                        }
                    }
                    fn recoverability(&self) -> ::pari::error::Recoverability {
                        match self {
                            #(#rec_arms)*
                        }
                    }
                    fn as_any_inner(&self) -> ::std::option::Option<&dyn ::std::any::Any> {
                        match self {
                            #(#inner_arms)*
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a wildcard match pattern for a variant: `Self::Foo`, `Self::Foo(..)`, or `Self::Foo { .. }`.
fn variant_wildcard_pattern(
    vname: &syn::Ident,
    fields: &ast::Fields<ErrorComposeField>,
) -> TokenStream {
    match fields.style {
        ast::Style::Unit => quote! { Self::#vname },
        ast::Style::Tuple => quote! { Self::#vname(..) },
        ast::Style::Struct => quote! { Self::#vname { .. } },
    }
}
