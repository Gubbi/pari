use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Field, Ident, Type};

pub fn generate_accessors_and_setters(
    entity_name: &Ident,
    domain_fields: &[&Field],
) -> (Vec<TokenStream2>, Vec<TokenStream2>) {
    let accessors: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let ty = &f.ty;
            let (ret_type, map_expr) = accessor_return_type(ty);
            let fname_str = fname.as_ref().unwrap().to_string();
            quote! {
                pub async fn #fname(&self) -> ::std::result::Result<#ret_type, ::pari::entity::LoadError> {
                    if self.#fname.get().is_none() {
                        ::pari::workspace::EntityClient::load(
                            <#entity_name as ::pari::entity::Entity>::to_any_ref(self.entity_ref()),
                            #fname_str,
                        ).await?;
                    }
                    Ok(self.#fname.get().expect("field not loaded") #map_expr)
                }
            }
        })
        .collect();

    let setters: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let setter_name =
                Ident::new(&format!("set_{}", fname.as_ref().unwrap()), Span::call_site());
            let ty = &f.ty;
            let fname_str = fname.as_ref().unwrap().to_string();
            quote! {
                pub async fn #setter_name(&mut self, value: #ty) -> ::std::result::Result<(), ::pari::entity::SetterError> {
                    ::pari::workspace::EntityClient::ensure_mutable(
                        <#entity_name as ::pari::entity::Entity>::to_any_ref(self.entity_ref()),
                        #fname_str,
                    ).await.map_err(|e| match e {
                        ::pari::workspace::LoadError::Substrate(s) => ::pari::entity::SetterError::Substrate(s),
                        other => panic!("ensure_mutable failed unexpectedly: {:?}", other),
                    })?;

                    let mut candidate = self.clone();
                    candidate.#fname = ::std::sync::Arc::new(::pari::tracked::TrackedField::mutated(value));

                    let errors = ::pari::validation::run_validations::<#entity_name>(
                        &candidate,
                        &[#fname_str],
                        &[
                            ::pari::validation::ValidationKind::Structural,
                            ::pari::validation::ValidationKind::Semantic,
                        ],
                    ).await.expect("generated setter field is always in the validation schema");

                    if !errors.is_empty() {
                        return Err(::pari::entity::SetterError::Validation {
                            error_count: errors.errors.len(),
                            errors,
                        });
                    }

                    self.#fname = ::std::sync::Arc::clone(&candidate.#fname);
                    Ok(())
                }
            }
        })
        .collect();

    (accessors, setters)
}

fn accessor_return_type(ty: &Type) -> (TokenStream2, TokenStream2) {
    match ty {
        Type::Path(tp) if tp.qself.is_none() => {
            let segs = &tp.path.segments;
            if segs.len() == 1 {
                let seg = &segs[0];
                if seg.ident == "String" {
                    return (quote! { &str }, quote! { .as_str() });
                }
                if seg.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        if args.args.len() == 1 {
                            if let syn::GenericArgument::Type(inner) = &args.args[0] {
                                if is_type_ident(inner, "String") {
                                    return (
                                        quote! { ::std::option::Option<&str> },
                                        quote! { .as_deref() },
                                    );
                                }
                                if let Some(elem) = vec_inner_type(inner) {
                                    return (
                                        quote! { ::std::option::Option<&[#elem]> },
                                        quote! { .as_deref() },
                                    );
                                }
                                return (
                                    quote! { ::std::option::Option<&#inner> },
                                    quote! { .as_ref() },
                                );
                            }
                        }
                    }
                }
                if seg.ident == "Vec" {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        if args.args.len() == 1 {
                            if let syn::GenericArgument::Type(elem) = &args.args[0] {
                                return (quote! { &[#elem] }, quote! { .as_slice() });
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
