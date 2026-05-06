//! Code generation for the `workspace` layer's per-entity
//! [`XViewer<'ws, Name>`] accessors and [`XEditor<'ws, Name>`]
//! mutation handle.
//!
//! The generated items belong to the `workspace` layer: they are the
//! runtime expression of the access and mutation patterns described in
//! `docs/design/layers/workspace.md`. The macro crate only hosts the
//! generation; the semantics are owned by `workspace`.
//!
//! Read accessors live on `XViewer<'ws, Name>` so they reach the
//! workspace's dispatcher through the borrowed session. Setters and
//! the `commit(self)` / `undo_checkout(self)` lifecycle live on
//! `XEditor<'ws, Name>`, which `Deref`s to `XViewer` for read access.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Field, Ident, Type};

pub struct WorkspaceParts {
    pub viewer_impl: TokenStream2,
    pub editor_impl: TokenStream2,
}

/// Emit per-field accessors on `XViewer<'_, Name>` and per-field
/// setters + lifecycle on `XEditor<'_, Name>`.
pub fn generate_workspace_parts(entity_name: &Ident, domain_fields: &[&Field]) -> WorkspaceParts {
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
                    let any_ref = self.tracked().entity_ref().to_any_ref();
                    match self.workspace().__dispatcher().dispatch(
                        ::pari::store::WorkspaceRequest::EnsureMutable {
                            any_ref,
                            field: #fname_str.to_string(),
                        },
                    ).await {
                        ::pari::store::WorkspaceResponse::Unit => {},
                        ::pari::store::WorkspaceResponse::Err(e) => return ::std::result::Result::Err(e),
                        _ => unreachable!(),
                    }

                    // Cheap clone: per-field Arc<TrackedField> refcounts bump,
                    // no field data is copied. Swap the target field for the
                    // mutated candidate; remaining fields keep their existing
                    // Arcs so validation sees a consistent snapshot.
                    let mut candidate = self.tracked().clone();
                    candidate.#fname = ::std::sync::Arc::new(::pari::tracked::TrackedField::mutated(value));
                    let validated_field = ::std::sync::Arc::clone(&candidate.#fname);

                    // Wrap candidate as a transient viewer over the editor's
                    // workspace and validate it. The viewer borrows
                    // self.workspace() for the duration of the await; the
                    // borrow ends before __viewer_mut() below.
                    let viewer = self.workspace().import::<#entity_name>(candidate);
                    ::pari::validation::run_validations::<#entity_name>(
                        &viewer,
                        &[#fname_str],
                        &[
                            ::pari::validation::ValidationKind::Structural,
                            ::pari::validation::ValidationKind::Semantic,
                        ],
                    ).await?;

                    self.__viewer_mut().__tracked_mut().#fname = validated_field;
                    Ok(())
                }
            }
        })
        .collect();

    let editor_impl = quote! {
        impl<'ws> ::pari::workspace::XEditor<'ws, #entity_name> {
            #(#setters)*

            /// Validate the dirty fields, merge the checked-out entity
            /// back into the store, and release the checkout. Consumes
            /// the editor.
            pub async fn commit(self) -> ::std::result::Result<(), ::pari::error::ActivityError> {
                let dispatcher = ::std::sync::Arc::clone(self.workspace().__dispatcher());
                let viewer = self.__into_viewer();
                let tracked = viewer.__into_inner();
                let entity = <#entity_name as ::pari::entity::Entity>::into_tracked_entity(tracked);
                match dispatcher.dispatch(
                    ::pari::store::WorkspaceRequest::Commit { entity },
                ).await {
                    ::pari::store::WorkspaceResponse::Unit => ::std::result::Result::Ok(()),
                    ::pari::store::WorkspaceResponse::Err(e) => ::std::result::Result::Err(e),
                    _ => unreachable!(),
                }
            }

            /// Discard pending edits and release the checkout. Consumes
            /// the editor.
            pub async fn undo_checkout(self) -> ::std::result::Result<(), ::pari::error::ActivityError> {
                let any_ref = self.tracked().entity_ref().to_any_ref();
                let dispatcher = ::std::sync::Arc::clone(self.workspace().__dispatcher());
                match dispatcher.dispatch(
                    ::pari::store::WorkspaceRequest::UndoCheckout { any_ref },
                ).await {
                    ::pari::store::WorkspaceResponse::Unit => ::std::result::Result::Ok(()),
                    ::pari::store::WorkspaceResponse::Err(e) => ::std::result::Result::Err(e),
                    _ => unreachable!(),
                }
            }
        }
    };

    WorkspaceParts {
        viewer_impl,
        editor_impl,
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
