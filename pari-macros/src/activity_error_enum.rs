//! `activity_errors! { ... }` — declares the centralized `ActivityError` enum.
//!
//! This macro is the integration point for authoring Pari's Activity tier.
//! It takes a list of variants — each with a doc comment, classification, and
//! an optional hint — and expands to:
//!
//! - a centralized `ActivityError` enum with one variant per declaration,
//! - `component: String` and `cause: PrimitiveError` fields on every variant
//!   (auto-injected; not written at the call site),
//! - `ErrorCompose` and `OTelEmit` impls plumbing the variant into the chain,
//! - accessors (`error_layer()`, `fix_domain()`, `hint()`, etc.) read by
//!   callers and by the OTel cascade.
//!
//! # Why authors only see outcomes + classification
//!
//! An activity variant is a **product-language outcome**:
//! "validation failed", "workspace not clean", "persistence is corrupt". The
//! macro's role is to keep the declaration of a new outcome limited to those
//! product-meaningful properties — name, classification, hint — so that
//! adding a new activity is cheap and every activity ends up with the same
//! shape. Plumbing (the `component`, the `cause`, the propagation hooks)
//! never shows up in the declaration because it is not a per-variant choice.
//!
//! # Input shape
//!
//! ```ignore
//! activity_errors! {
//!     /// Schema or pipeline field-mapping error.
//!     InvalidPersistenceLayout {
//!         fix = Data,
//!         recoverability = OperatorAction,
//!         hint = "check the entity schema definition and field mappings",
//!     }
//!     /// Entity could not be serialized or deserialized.
//!     UnpersistableDefinition {
//!         fix = Data,
//!         recoverability = OperatorAction,
//!     }
//! }
//! ```
//!
//! `hint` is optional. `fix` and `recoverability` are required — an Activity
//! variant without classification would break propagation to the Job tier.
//!
//! # At the call site
//!
//! Orchestration code constructs an activity variant by supplying only the
//! per-instance values: the `component` string identifying the subsystem
//! surfacing the failure, and the concrete `PrimitiveError` that triggered
//! this outcome. Everything classification-related is already baked into the
//! variant by this macro.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Attribute, Ident, LitStr, Result, Token,
};

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

struct ActivityErrorsInput {
    variants: Vec<ActivityVariant>,
}

struct ActivityVariant {
    attrs: Vec<Attribute>,
    ident: Ident,
    fix: Ident,
    recoverability: Ident,
    hint: Option<String>,
}

impl Parse for ActivityErrorsInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut variants = Vec::new();
        while !input.is_empty() {
            variants.push(input.parse()?);
        }
        Ok(Self { variants })
    }
}

impl Parse for ActivityVariant {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let ident: Ident = input.parse()?;

        let body;
        syn::braced!(body in input);

        let mut fix: Option<Ident> = None;
        let mut recoverability: Option<Ident> = None;
        let mut hint: Option<String> = None;

        while !body.is_empty() {
            let key: Ident = body.parse()?;
            body.parse::<Token![=]>()?;

            if key == "fix" {
                fix = Some(body.parse()?);
            } else if key == "recoverability" {
                recoverability = Some(body.parse()?);
            } else if key == "hint" {
                let lit: LitStr = body.parse()?;
                hint = Some(lit.value());
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "unknown activity_errors key; expected `fix`, `recoverability`, or `hint`",
                ));
            }

            if body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
            }
        }

        let fix = fix.ok_or_else(|| {
            syn::Error::new(
                ident.span(),
                "missing `fix = ...` in activity error variant",
            )
        })?;
        let recoverability = recoverability.ok_or_else(|| {
            syn::Error::new(
                ident.span(),
                "missing `recoverability = ...` in activity error variant",
            )
        })?;

        Ok(Self {
            attrs,
            ident,
            fix,
            recoverability,
            hint,
        })
    }
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

pub fn activity_errors(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ActivityErrorsInput);
    expand_activity_errors(input).into()
}

fn expand_activity_errors(input: ActivityErrorsInput) -> proc_macro2::TokenStream {
    let variants = input.variants.iter().map(|v| {
        let attrs = &v.attrs;
        let ident = &v.ident;
        quote! {
            #(#attrs)*
            #ident {
                component: ::std::string::String,
                cause: PrimitiveError,
            }
        }
    });

    let constructor_fns = input.variants.iter().map(|v| {
        let ident = &v.ident;
        let fn_name = Ident::new(&camel_to_snake(&ident.to_string()), Span::call_site());
        quote! {
            pub fn #fn_name(
                component: impl ::std::convert::Into<::std::string::String>,
                cause: PrimitiveError,
            ) -> Self {
                Self::#ident {
                    component: component.into(),
                    cause,
                }
            }
        }
    });

    let error_type_arms = input.variants.iter().map(|v| {
        let ident = &v.ident;
        let error_type = camel_to_snake(&ident.to_string());
        quote! { Self::#ident { .. } => #error_type, }
    });

    let component_arms = input.variants.iter().map(|v| {
        let ident = &v.ident;
        quote! { Self::#ident { component, .. } => component.as_str(), }
    });

    let hint_arms = input.variants.iter().map(|v| {
        let ident = &v.ident;
        let hint_expr = match &v.hint {
            Some(h) => {
                let lit = LitStr::new(h, Span::call_site());
                quote! { ::std::option::Option::Some(#lit) }
            }
            None => quote! { ::std::option::Option::None },
        };
        quote! { Self::#ident { .. } => #hint_expr, }
    });

    let cause_arms = input.variants.iter().map(|v| {
        let ident = &v.ident;
        quote! { Self::#ident { cause, .. } => cause, }
    });

    let into_cause_arms = input.variants.iter().map(|v| {
        let ident = &v.ident;
        quote! { Self::#ident { cause, .. } => cause, }
    });

    let fix_arms = input.variants.iter().map(|v| {
        let ident = &v.ident;
        let fix = &v.fix;
        quote! { Self::#ident { .. } => FixDomain::#fix, }
    });

    let recoverability_arms = input.variants.iter().map(|v| {
        let ident = &v.ident;
        let rec = &v.recoverability;
        quote! { Self::#ident { .. } => Recoverability::#rec, }
    });

    let emit_arms = input.variants.iter().map(|v| {
        let ident = &v.ident;
        let error_type = camel_to_snake(&ident.to_string());
        let hint_expr = match &v.hint {
            Some(h) => {
                let lit = LitStr::new(h, Span::call_site());
                quote! { ::std::option::Option::Some(#lit) }
            }
            None => quote! { ::std::option::Option::<&'static str>::None },
        };
        let is_warn = matches!(
            (
                v.fix.to_string().as_str(),
                v.recoverability.to_string().as_str()
            ),
            ("Infra", "Retryable") | ("Client", "UserAction")
        );
        let log_macro = if is_warn {
            quote! { ::tracing::warn }
        } else {
            quote! { ::tracing::error }
        };
        quote! {
            Self::#ident { component, cause } => {
                #log_macro!(
                    exception.type    = #error_type,
                    exception.message = %cause,
                    "error.component" = %component,
                    "error.hint"      = ?#hint_expr,
                );
                OTelEmit::emit(cause);
            }
        }
    });

    quote! {
        #[derive(Debug)]
        pub enum ActivityError {
            #(#variants),*
        }

        impl ActivityError {
            #(#constructor_fns)*

            pub fn error_layer(&self) -> ErrorLayer {
                ErrorLayer::Activity
            }

            pub fn error_type(&self) -> &'static str {
                match self {
                    #(#error_type_arms)*
                }
            }

            pub fn component(&self) -> &str {
                match self {
                    #(#component_arms)*
                }
            }

            pub fn hint(&self) -> ::std::option::Option<&'static str> {
                match self {
                    #(#hint_arms)*
                }
            }

            pub fn cause(&self) -> &PrimitiveError {
                match self {
                    #(#cause_arms)*
                }
            }

            pub fn into_cause(self) -> PrimitiveError {
                match self {
                    #(#into_cause_arms)*
                }
            }
        }

        impl ::std::fmt::Display for ActivityError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::write!(f, "{}", self.cause())
            }
        }

        impl ::std::error::Error for ActivityError {
            fn source(
                &self,
            ) -> ::std::option::Option<&(dyn ::std::error::Error + 'static)> {
                ::std::option::Option::Some(self.cause())
            }
        }

        impl ErrorCompose for ActivityError {
            fn fix_domain(&self) -> FixDomain {
                match self {
                    #(#fix_arms)*
                }
            }

            fn recoverability(&self) -> Recoverability {
                match self {
                    #(#recoverability_arms)*
                }
            }
        }

        impl OTelEmit for ActivityError {
            fn emit(&self) {
                match self {
                    #(#emit_arms)*
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
