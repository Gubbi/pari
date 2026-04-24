//! `primitive_errors! { ... }` — declares the centralized `PrimitiveError` enum.
//!
//! This macro is the integration point for authoring Pari's Primitive tier.
//! It takes a brace-delimited list of variant declarations — each a doc
//! comment plus a struct-like field list — and expands to:
//!
//! - one generated struct per variant (message + typed detail fields,
//!   plus the fixed [`PrimitiveContext`] for diagnostics),
//! - auto-capturing constructors (`new` / `new_with_location`),
//! - a centralized `PrimitiveError` enum wrapping all variants,
//! - the `ErrorCompose` and `OTelEmit` impls that splice primitives into the
//!   wider error chain.
//!
//! # Why authors only see variants
//!
//! The contract behind a primitive — what diagnostics are fixed, how
//! construction captures them, how emission is shaped — is stable across every
//! primitive in the system. A new primitive should cost exactly one
//! declaration with only the *variant-specific* information: the doc comment
//! that becomes the `Display` message source, and the typed detail fields.
//! Everything else is supplied by this macro.
//!
//! # Input shape
//!
//! ```ignore
//! primitive_errors! {
//!     /// Doc comment — becomes the variant's canonical description.
//!     VariantName { field_a: TypeA, field_b: TypeB }
//!
//!     /// Another variant.
//!     OtherName { path: String, line: usize }
//! }
//! ```
//!
//! The macro accepts no `message` field — `message: String` is always part of
//! the generated shape and is supplied at construction time. It accepts no
//! classification — primitives do not declare classification (the Activity
//! tier does).
//!
//! # Relationship to the per-struct macros
//!
//! Mechanical generation for one variant — the struct body, the constructors,
//! the emission body — lives in [`super::primitive_error`]. This enum macro
//! orchestrates those per-variant expansions and stitches them into the
//! centralized enum.
//!
//! [`PrimitiveContext`]: pari::error::primitive::PrimitiveContext

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input, Attribute, Expr, Ident, Result, Token, Type,
};

struct PrimitiveErrorsInput {
    variants: Vec<PrimitiveVariant>,
}

struct PrimitiveVariant {
    attrs: Vec<Attribute>,
    ident: Ident,
    error_type: Option<Ident>,
    fields: Vec<PrimitiveField>,
}

struct PrimitiveField {
    ident: Ident,
    ty: Type,
}

impl Parse for PrimitiveErrorsInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut variants = Vec::new();
        while !input.is_empty() {
            variants.push(input.parse()?);
        }
        Ok(Self { variants })
    }
}

impl Parse for PrimitiveVariant {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let ident: Ident = input.parse()?;
        let error_type = if input.peek(syn::token::Bracket) {
            let content;
            bracketed!(content in input);
            Some(content.parse()?)
        } else {
            None
        };
        let content;
        syn::braced!(content in input);
        let punctuated = content.parse_terminated(PrimitiveField::parse, Token![,])?;
        Ok(Self {
            attrs,
            ident,
            error_type,
            fields: punctuated.into_iter().collect(),
        })
    }
}

impl Parse for PrimitiveField {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            ty: {
                input.parse::<Token![:]>()?;
                input.parse()?
            },
        })
    }
}

pub fn primitive_errors(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as PrimitiveErrorsInput);
    expand_primitive_errors(input).into()
}

fn expand_primitive_errors(input: PrimitiveErrorsInput) -> proc_macro2::TokenStream {
    let variants = input.variants.iter().map(|variant| {
        let attrs = &variant.attrs;
        let ident = &variant.ident;
        let fields = variant.fields.iter().map(|field| {
            let ident = &field.ident;
            let ty = &field.ty;
            quote! { #ident: #ty }
        });
        quote! {
            #(#attrs)*
            #ident {
                context: PrimitiveContext,
                #(#fields),*
            }
        }
    });

    let error_type_arms = input.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let error_type = variant
            .error_type
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| camel_to_snake(&ident.to_string()));
        quote! { Self::#ident { .. } => #error_type, }
    });

    let detail_arms = input.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let fields = variant
            .fields
            .iter()
            .map(|field| &field.ident)
            .collect::<Vec<_>>();
        let pushes = fields.iter().map(|field| {
            let field_name = field.to_string();
            quote! {
                PrimitiveDetail {
                    field_name: #field_name,
                    value: format!("{:?}", #field),
                }
            }
        });
        quote! {
            Self::#ident { #(#fields,)* .. } => {
                vec![#(#pushes),*]
            }
        }
    });

    let context_ref_arms = input.variants.iter().map(|variant| {
        let ident = &variant.ident;
        quote! { Self::#ident { context, .. } => context, }
    });

    let with_location_arms = input.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let fields = variant
            .fields
            .iter()
            .map(|field| &field.ident)
            .collect::<Vec<_>>();
        quote! {
            Self::#ident { mut context, #(#fields),* } => {
                context.location = location;
                Self::#ident { context, #(#fields),* }
            }
        }
    });

    let emit_arms = input.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let fields = variant
            .fields
            .iter()
            .map(|field| &field.ident)
            .collect::<Vec<_>>();
        let error_type = variant
            .error_type
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| camel_to_snake(&ident.to_string()));
        let error_type_expr: Expr = syn::parse_quote! { #error_type };
        let otel_fields = fields.iter().map(|field| {
            let attr_name = format!("error.{}.{}", error_type, field);
            quote! { #attr_name = ?#field, }
        });
        quote! {
            Self::#ident { context #(, #fields)* } => {
                ::tracing::error!(
                    exception.type = #error_type_expr,
                    exception.message = %context.message,
                    exception.stacktrace = %context.backtrace,
                    code.file.path = %context.location.file,
                    code.line.number = context.location.line,
                    code.column.number = context.location.column,
                    #(#otel_fields)*
                );
            }
        }
    });

    quote! {
        #[derive(Debug)]
        pub enum PrimitiveError {
            #(#variants),*
        }

        impl PrimitiveError {
            #[track_caller]
            pub fn context(message: impl Into<String>) -> PrimitiveContext {
                PrimitiveContext {
                    message: message.into(),
                    location: ErrorLocation::caller(),
                    span_trace: tracing_error::SpanTrace::capture(),
                    backtrace: std::backtrace::Backtrace::capture(),
                }
            }

            pub fn context_with_location(
                location: ErrorLocation,
                message: impl Into<String>,
            ) -> PrimitiveContext {
                PrimitiveContext {
                    message: message.into(),
                    location,
                    span_trace: tracing_error::SpanTrace::capture(),
                    backtrace: std::backtrace::Backtrace::capture(),
                }
            }

            pub fn with_location(self, location: ErrorLocation) -> Self {
                match self {
                    #(#with_location_arms),*
                }
            }

            pub fn error_layer(&self) -> ErrorLayer {
                ErrorLayer::Primitive
            }

            pub fn error_type(&self) -> &'static str {
                match self {
                    #(#error_type_arms)*
                }
            }

            pub fn message(&self) -> &str {
                &self.context_ref().message
            }

            pub fn location(&self) -> &ErrorLocation {
                &self.context_ref().location
            }

            pub fn span_trace(&self) -> &tracing_error::SpanTrace {
                &self.context_ref().span_trace
            }

            pub fn backtrace(&self) -> &std::backtrace::Backtrace {
                &self.context_ref().backtrace
            }

            pub fn details(&self) -> Vec<PrimitiveDetail> {
                match self {
                    #(#detail_arms),*
                }
            }

            fn context_ref(&self) -> &PrimitiveContext {
                match self {
                    #(#context_ref_arms)*
                }
            }
        }

        impl std::fmt::Display for PrimitiveError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.message())
            }
        }

        impl std::error::Error for PrimitiveError {}

        impl OTelEmit for PrimitiveError {
            fn emit(&self) {
                match self {
                    #(#emit_arms),*
                }
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
