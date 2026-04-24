//! `#[derive(CollectRefs)]` — walk a plain struct or enum and push every
//! contained [`EntityRef`](pari::entity::EntityRef) into an accumulator with
//! a dot-notation path.
//!
//! The blanket impls for containers (`Option`, `Vec`, `HashMap`,
//! `IndexMap<String, _>`) and for primitive leaves live in
//! `src/entity/collect_refs.rs`; this derive handles user-defined types by
//! delegating to each field's impl and composing paths.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

pub fn derive_collect_refs_impl(ast: DeriveInput) -> TokenStream2 {
    let name = &ast.ident;

    let body = match &ast.data {
        Data::Struct(s) => generate_struct_body(&s.fields),
        Data::Enum(e) => generate_enum_body(name, e),
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "CollectRefs cannot be derived for unions")
                .to_compile_error();
        }
    };

    quote! {
        impl ::pari::entity::collect_refs::CollectRefs for #name {
            fn collect_refs(
                &self,
                prefix: &str,
                refs: &mut ::std::vec::Vec<(::std::string::String, ::pari::entity::AnyEntityRef)>,
            ) {
                #body
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Struct body: call collect_refs on every named/unnamed field.
// ---------------------------------------------------------------------------

fn generate_struct_body(fields: &Fields) -> TokenStream2 {
    match fields {
        Fields::Named(named) => {
            let calls: Vec<TokenStream2> = named
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    let name_str = ident.to_string();
                    quote! {
                        ::pari::entity::collect_refs::CollectRefs::collect_refs(
                            &self.#ident,
                            &::std::format!("{prefix}.{}", #name_str),
                            refs,
                        );
                    }
                })
                .collect();
            quote! { #(#calls)* }
        }
        Fields::Unnamed(unnamed) => {
            let calls: Vec<TokenStream2> = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let idx = syn::Index::from(i);
                    let name_str = i.to_string();
                    quote! {
                        ::pari::entity::collect_refs::CollectRefs::collect_refs(
                            &self.#idx,
                            &::std::format!("{prefix}.{}", #name_str),
                            refs,
                        );
                    }
                })
                .collect();
            quote! { #(#calls)* }
        }
        Fields::Unit => quote! {},
    }
}

// ---------------------------------------------------------------------------
// Enum body: one match arm per variant.
// ---------------------------------------------------------------------------

fn generate_enum_body(enum_name: &syn::Ident, data: &syn::DataEnum) -> TokenStream2 {
    let arms: Vec<TokenStream2> = data
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.ident;
            match &v.fields {
                Fields::Named(named) => {
                    let field_idents: Vec<&syn::Ident> = named
                        .named
                        .iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect();
                    let calls: Vec<TokenStream2> = field_idents
                        .iter()
                        .map(|ident| {
                            let name_str = ident.to_string();
                            quote! {
                                ::pari::entity::collect_refs::CollectRefs::collect_refs(
                                    #ident,
                                    &::std::format!("{prefix}.{}", #name_str),
                                    refs,
                                );
                            }
                        })
                        .collect();
                    quote! {
                        #enum_name::#variant_name { #(#field_idents,)* .. } => {
                            #(#calls)*
                        }
                    }
                }
                Fields::Unnamed(unnamed) => {
                    let bindings: Vec<syn::Ident> = (0..unnamed.unnamed.len())
                        .map(|i| quote::format_ident!("_f{}", i))
                        .collect();
                    let calls: Vec<TokenStream2> = bindings
                        .iter()
                        .enumerate()
                        .map(|(i, ident)| {
                            let name_str = i.to_string();
                            quote! {
                                ::pari::entity::collect_refs::CollectRefs::collect_refs(
                                    #ident,
                                    &::std::format!("{prefix}.{}", #name_str),
                                    refs,
                                );
                            }
                        })
                        .collect();
                    quote! {
                        #enum_name::#variant_name(#(#bindings),*) => {
                            #(#calls)*
                        }
                    }
                }
                Fields::Unit => {
                    quote! {
                        #enum_name::#variant_name => {}
                    }
                }
            }
        })
        .collect();

    quote! {
        match self {
            #(#arms)*
        }
    }
}
