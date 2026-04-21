//! `#[activity_error(...)]` attribute macro implementation.
//!
//! Generates the Activity-layer error infrastructure for a struct. Auto-adds `component` and
//! `cause` fields. `hint` is a compile-time constant declared in the macro args. Generates
//! `new()`, accessor methods, `Display`, `Debug`, `Error`, `ErrorCompose`, and `OTelEmit` impls.
//!
//! Usage:
//! ```rust,ignore
//! #[activity_error(fix = Data, recoverability = OperatorAction)]
//! pub struct BadDefinitionError {}
//!
//! // With hint and explicit error_type:
//! #[activity_error(
//!     fix = Infra,
//!     recoverability = OperatorAction,
//!     hint = "stale .part/ dir may exist — safe to remove",
//!     error_type = "corrupt_persistence_state",
//! )]
//! pub struct CorruptPersistenceState {}
//! ```
//!
//! Generated constructor:
//! ```rust,ignore
//! CorruptPersistenceState::new(component: ActivityComponent, cause: PrimitiveError)
//! ```

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, Expr, Fields, FieldsNamed, Ident,
    ItemStruct, Lit, MetaNameValue, Token,
};

#[derive(Default)]
struct ActivityErrorArgs {
    fix: Option<Ident>,
    recoverability: Option<Ident>,
    hint: Option<String>,
    error_type: Option<String>,
}

pub fn activity_error(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match parse_args(args) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    let item = parse_macro_input!(input as ItemStruct);
    expand_activity_error(args, item).into()
}

fn parse_args(args: TokenStream) -> syn::Result<ActivityErrorArgs> {
    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    let metas = parser.parse(args)?;
    let mut parsed = ActivityErrorArgs::default();

    for meta in &metas {
        if meta.path.is_ident("fix") {
            match &meta.value {
                Expr::Path(expr_path) => {
                    let ident = expr_path.path.get_ident().ok_or_else(|| {
                        syn::Error::new_spanned(
                            &expr_path.path,
                            "activity_error(fix = ...) expects a simple identifier like `Data`",
                        )
                    })?;
                    parsed.fix = Some(ident.clone());
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "activity_error(fix = ...) expects a simple identifier like `Data`",
                    ))
                }
            }
        } else if meta.path.is_ident("recoverability") {
            match &meta.value {
                Expr::Path(expr_path) => {
                    let ident = expr_path.path.get_ident().ok_or_else(|| {
                        syn::Error::new_spanned(
                            &expr_path.path,
                            "activity_error(recoverability = ...) expects a simple identifier like `OperatorAction`",
                        )
                    })?;
                    parsed.recoverability = Some(ident.clone());
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "activity_error(recoverability = ...) expects a simple identifier like `OperatorAction`",
                    ))
                }
            }
        } else if meta.path.is_ident("hint") {
            match &meta.value {
                Expr::Lit(expr_lit) => match &expr_lit.lit {
                    Lit::Str(lit) => parsed.hint = Some(lit.value()),
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "activity_error(hint = ...) expects a string literal",
                        ))
                    }
                },
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "activity_error(hint = ...) expects a string literal",
                    ))
                }
            }
        } else if meta.path.is_ident("error_type") {
            match &meta.value {
                Expr::Lit(expr_lit) => match &expr_lit.lit {
                    Lit::Str(lit) => parsed.error_type = Some(lit.value()),
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "activity_error(error_type = ...) expects a string literal",
                        ))
                    }
                },
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "activity_error(error_type = ...) expects a string literal",
                    ))
                }
            }
        } else {
            return Err(syn::Error::new_spanned(
                &meta.path,
                "unsupported activity_error argument; expected `fix`, `recoverability`, `hint`, or `error_type`",
            ));
        }
    }

    Ok(parsed)
}

fn expand_activity_error(args: ActivityErrorArgs, item: ItemStruct) -> proc_macro2::TokenStream {
    let attrs = &item.attrs;
    let vis = &item.vis;
    let name = &item.ident;
    let generics = &item.generics;
    let where_clause = &item.generics.where_clause;

    let fix = match args.fix {
        Some(f) => f,
        None => {
            return syn::Error::new_spanned(
                name,
                "#[activity_error] requires `fix = ...` argument (e.g. `fix = Data`)",
            )
            .to_compile_error()
        }
    };
    let recoverability = match args.recoverability {
        Some(r) => r,
        None => {
            return syn::Error::new_spanned(
                name,
                "#[activity_error] requires `recoverability = ...` argument (e.g. `recoverability = OperatorAction`)",
            )
            .to_compile_error()
        }
    };

    let fields = match &item.fields {
        Fields::Named(fields) => fields,
        Fields::Unit => &FieldsNamed {
            brace_token: syn::token::Brace::default(),
            named: Default::default(),
        },
        other => {
            return syn::Error::new_spanned(
                other,
                "#[activity_error] supports only structs with named fields or unit structs",
            )
            .to_compile_error()
        }
    };

    let FieldsNamed { named, .. } = fields;
    let user_fields: Vec<_> = named.iter().collect();
    let user_field_idents: Vec<_> = user_fields
        .iter()
        .map(|f| f.ident.as_ref().expect("named field"))
        .collect();
    let user_field_types: Vec<_> = user_fields.iter().map(|f| &f.ty).collect();
    let user_field_attrs: Vec<_> = user_fields.iter().map(|f| &f.attrs).collect();

    let error_type = args
        .error_type
        .unwrap_or_else(|| camel_to_snake(&name.to_string()));
    let error_type_lit = syn::LitStr::new(&error_type, Span::call_site());

    let hint_expr = match args.hint {
        Some(h) => {
            let lit = syn::LitStr::new(&h, Span::call_site());
            quote! { ::std::option::Option::Some(#lit) }
        }
        None => quote! { ::std::option::Option::None },
    };

    let is_warn = matches!(
        (
            fix.to_string().as_str(),
            recoverability.to_string().as_str()
        ),
        ("Infra", "Retryable") | ("Client", "UserAction")
    );

    let log_macro = if is_warn {
        quote! { ::tracing::warn! }
    } else {
        quote! { ::tracing::error! }
    };

    let (impl_generics, ty_generics, _) = generics.split_for_impl();

    quote! {
        #(#attrs)*
        #vis struct #name #generics #where_clause {
            pub component: ::pari::error::ActivityComponent,
            #(
                #(#user_field_attrs)*
                pub #user_field_idents: #user_field_types,
            )*
            pub cause: ::pari::error::primitive::PrimitiveError,
        }

        impl #impl_generics #name #ty_generics #where_clause {
            pub fn new(
                component: ::pari::error::ActivityComponent,
                #(#user_field_idents: #user_field_types,)*
                cause: ::pari::error::primitive::PrimitiveError,
            ) -> Self {
                Self {
                    component,
                    #(#user_field_idents,)*
                    cause,
                }
            }

            pub fn error_layer(&self) -> ::pari::error::ErrorLayer {
                ::pari::error::ErrorLayer::Activity
            }

            pub fn error_type(&self) -> &'static str {
                #error_type_lit
            }

            pub fn component(&self) -> ::pari::error::ActivityComponent {
                self.component
            }

            pub fn hint(&self) -> ::std::option::Option<&'static str> {
                #hint_expr
            }

            pub fn cause(&self) -> &::pari::error::primitive::PrimitiveError {
                &self.cause
            }
        }

        impl #impl_generics ::std::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::write!(f, "{}", self.cause)
            }
        }

        impl #impl_generics ::std::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(::std::stringify!(#name))
                    .field("component", &self.component)
                    #(.field(::std::stringify!(#user_field_idents), &self.#user_field_idents))*
                    .field("hint", &#hint_expr)
                    .field("cause", &self.cause)
                    .finish()
            }
        }

        impl #impl_generics ::std::error::Error for #name #ty_generics #where_clause {
            fn source(&self) -> ::std::option::Option<&(dyn ::std::error::Error + 'static)> {
                ::std::option::Option::Some(&self.cause)
            }
        }

        impl #impl_generics ::pari::error::ErrorCompose for #name #ty_generics #where_clause {
            fn fix_domain(&self) -> ::pari::error::FixDomain {
                ::pari::error::FixDomain::#fix
            }

            fn recoverability(&self) -> ::pari::error::Recoverability {
                ::pari::error::Recoverability::#recoverability
            }
        }

        impl #impl_generics ::pari::error::OTelEmit for #name #ty_generics #where_clause {
            fn emit(&self) {
                #log_macro!(
                    exception.type      = #error_type_lit,
                    exception.message   = %self,
                    "error.component"   = %self.component,
                    "error.hint"        = #hint_expr,
                );
                ::pari::error::OTelEmit::emit(&self.cause);
            }
        }
    }
}

fn camel_to_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (i, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}
