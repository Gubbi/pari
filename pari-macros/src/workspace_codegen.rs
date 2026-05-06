//! Code generation for the `workspace` layer's per-entity
//! [`XViewer<'ws, Name>`] accessors and the per-entity `Delegate`
//! mutation handle.
//!
//! The generated items belong to the `workspace` layer: they are the
//! runtime expression of the access and mutation patterns described in
//! `docs/design/layers/workspace.md`. The macro crate only hosts the
//! generation; the semantics are owned by `workspace`.
//!
//! Read accessors live on `XViewer<'ws, Name>` so they reach the
//! workspace's dispatcher through the borrowed session. Setters,
//! `commit(self)`, and `undo_checkout(self)` live on `XDelegate`,
//! which carries a clone of the workspace's dispatcher so it can keep
//! issuing requests after checkout.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Field, Ident, Type};

pub struct WorkspaceParts {
    pub viewer_impl: TokenStream2,
    pub delegate_struct: TokenStream2,
    pub delegate_impl: TokenStream2,
}

/// Emit per-field accessors on `XViewer<'_, Name>`, plus the per-entity
/// `XDelegate` struct, its setters, and its `commit` /
/// `undo_checkout` lifecycle.
pub fn generate_workspace_parts(
    entity_name: &Ident,
    tracked_name: &Ident,
    delegate_name: &Ident,
    domain_fields: &[&Field],
) -> WorkspaceParts {
    let accessors: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let ty = &f.ty;
            let (ret_type, map_expr) = accessor_return_type(ty);
            let fname_str = fname.as_ref().unwrap().to_string();
            quote! {
                pub async fn #fname(&self) -> ::std::result::Result<#ret_type, ::pari::error::ActivityError> {
                    if self.tracked().#fname.get().is_none() {
                        let any_ref = self.tracked().entity_ref().to_any_ref();
                        match self.workspace().__dispatcher().dispatch(
                            ::pari::store::WorkspaceRequest::Load {
                                any_ref,
                                field: #fname_str.to_string(),
                            },
                        ).await {
                            ::pari::store::WorkspaceResponse::Unit => {},
                            ::pari::store::WorkspaceResponse::Err(e) => return ::std::result::Result::Err(e),
                            _ => unreachable!(),
                        }
                    }
                    Ok(self.tracked().#fname.get().expect("field not loaded") #map_expr)
                }
            }
        })
        .collect();

    let viewer_impl = quote! {
        impl<'ws> ::pari::workspace::XViewer<'ws, #entity_name> {
            #(#accessors)*
        }
    };

    let setters: Vec<TokenStream2> = domain_fields
        .iter()
        .map(|f| {
            let fname = &f.ident;
            let setter_name =
                Ident::new(&format!("set_{}", fname.as_ref().unwrap()), Span::call_site());
            let ty = &f.ty;
            let fname_str = fname.as_ref().unwrap().to_string();
            quote! {
                pub async fn #setter_name(&mut self, value: #ty) -> ::std::result::Result<(), ::pari::error::ActivityError> {
                    let any_ref = self.inner.entity_ref().to_any_ref();
                    match self.dispatcher.dispatch(
                        ::pari::store::WorkspaceRequest::EnsureMutable {
                            any_ref,
                            field: #fname_str.to_string(),
                        },
                    ).await {
                        ::pari::store::WorkspaceResponse::Unit => {},
                        ::pari::store::WorkspaceResponse::Err(e) => return ::std::result::Result::Err(e),
                        _ => unreachable!(),
                    }

                    let mut candidate = self.inner.clone();
                    candidate.#fname = ::std::sync::Arc::new(::pari::tracked::TrackedField::mutated(value));
                    let validated_field = ::std::sync::Arc::clone(&candidate.#fname);

                    let workspace = ::pari::workspace::Workspace::new(
                        ::std::sync::Arc::clone(&self.dispatcher),
                    );
                    let viewer = workspace.import::<#entity_name>(candidate);

                    ::pari::validation::run_validations::<#entity_name>(
                        &viewer,
                        &[#fname_str],
                        &[
                            ::pari::validation::ValidationKind::Structural,
                            ::pari::validation::ValidationKind::Semantic,
                        ],
                    ).await?;

                    self.inner.#fname = validated_field;
                    Ok(())
                }
            }
        })
        .collect();

    let delegate_struct = quote! {
        /// Mutation handle returned by `Workspace::checkout`. Owns the
        /// checked-out tracked entity and a clone of the workspace's
        /// dispatcher; consumed on `commit` or `undo_checkout`. Not
        /// `Clone` — checkout is single-writer.
        pub struct #delegate_name {
            pub(crate) inner: #tracked_name,
            pub(crate) dispatcher: ::std::sync::Arc<dyn ::pari::store::Dispatcher>,
        }

        impl ::std::convert::From<(#tracked_name, ::std::sync::Arc<dyn ::pari::store::Dispatcher>)>
            for #delegate_name
        {
            fn from(
                (inner, dispatcher): (#tracked_name, ::std::sync::Arc<dyn ::pari::store::Dispatcher>),
            ) -> Self {
                Self { inner, dispatcher }
            }
        }
    };

    let delegate_impl = quote! {
        impl #delegate_name {
            #(#setters)*

            /// Validate the dirty fields, merge the checked-out entity
            /// back into the store, and release the checkout. Consumes
            /// the delegate.
            pub async fn commit(self) -> ::std::result::Result<(), ::pari::error::ActivityError> {
                let entity = <#entity_name as ::pari::entity::Entity>::into_tracked_entity(self.inner);
                match self.dispatcher.dispatch(
                    ::pari::store::WorkspaceRequest::Commit { entity },
                ).await {
                    ::pari::store::WorkspaceResponse::Unit => ::std::result::Result::Ok(()),
                    ::pari::store::WorkspaceResponse::Err(e) => ::std::result::Result::Err(e),
                    _ => unreachable!(),
                }
            }

            /// Discard pending edits and release the checkout. Consumes
            /// the delegate.
            pub async fn undo_checkout(self) -> ::std::result::Result<(), ::pari::error::ActivityError> {
                let any_ref = self.inner.entity_ref().to_any_ref();
                match self.dispatcher.dispatch(
                    ::pari::store::WorkspaceRequest::UndoCheckout { any_ref },
                ).await {
                    ::pari::store::WorkspaceResponse::Unit => ::std::result::Result::Ok(()),
                    ::pari::store::WorkspaceResponse::Err(e) => ::std::result::Result::Err(e),
                    _ => unreachable!(),
                }
            }
        }

        impl ::std::ops::Deref for #delegate_name {
            type Target = #tracked_name;
            fn deref(&self) -> &#tracked_name {
                &self.inner
            }
        }
    };

    WorkspaceParts {
        viewer_impl,
        delegate_struct,
        delegate_impl,
    }
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
