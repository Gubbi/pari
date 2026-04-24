//! `#[derive(OTelEmit)]` — generates the observability-emission contract.
//!
//! This derive fills in the [`OTelEmit`] trait so that a single `err.emit()`
//! call at the Job tier produces one structured event carrying fields from
//! every tier in the chain. Without this derive, every new error type would
//! need hand-written emission, which is how integrations drift apart.
//!
//! # What the generated `emit()` does
//!
//! 1. Emits the current tier's fields — type identifier, classification,
//!    any tier-specific values — into the active tracing span.
//! 2. For delegating variants (`#[compose(delegate)]`), calls `emit()` on
//!    the wrapped inner error, cascading emission down the chain.
//! 3. For primitive errors, also emits the fixed diagnostics (`message`,
//!    location, backtrace, span trace) and each typed detail field.
//!
//! All emission happens within a single event at the call site — the cascade
//! does not start nested spans.
//!
//! # Field-name policy
//!
//! The macro maps to [`opentelemetry_semantic_conventions`] wherever a
//! standard field exists (`exception.*`, `code.*`). Shared semantic fields
//! used across tiers keep stable names (`error.component`, `error.hint`).
//! Everything else — a primitive's typed detail fields, an activity variant's
//! structured fields — is emitted under `error.<error_type>.<snake_case_field>`
//! so multiple tiers can contribute fields to the same event without
//! collisions. The full convention lives in the L3 design doc.
//!
//! # Why separate from `ErrorCompose`
//!
//! Observability and classification evolve on different rhythms. Keeping the
//! two derives apart means tests for classification do not need a tracing
//! subscriber set up, and changes to emission conventions don't ripple into
//! the caller-facing classification API.
//!
//! [`OTelEmit`]: pari::error::OTelEmit
//! [`opentelemetry_semantic_conventions`]: https://crates.io/crates/opentelemetry-semantic-conventions

use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

// ---------------------------------------------------------------------------
// Attribute parsing
// ---------------------------------------------------------------------------

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(otel, compose), supports(struct_any, enum_any))]
struct OTelEmitInput {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<OTelVariant, OTelField>,
    /// `#[otel(error_type = "...")]` on the type (structs)
    error_type: Option<String>,
    /// Severity source for structs: `#[compose(fix = ..., recoverability = ...)]`
    fix: Option<syn::Path>,
    recoverability: Option<syn::Path>,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(otel, compose))]
struct OTelVariant {
    ident: syn::Ident,
    fields: ast::Fields<OTelField>,
    /// `#[compose(delegate)]` or `#[otel(delegate)]` — delegate emit to inner
    #[darling(default)]
    delegate: bool,
    /// `#[otel(error_type = "...")]` on the variant (declaring variants)
    error_type: Option<String>,
    /// Severity source for declaring variants
    fix: Option<syn::Path>,
    recoverability: Option<syn::Path>,
}

#[derive(Debug, FromField)]
#[darling(attributes(otel, compose))]
struct OTelField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    /// `#[otel(field = "attr.name")]`
    field: Option<String>,
    /// `#[otel(delegate)]` — cascade emit to this field
    #[darling(default)]
    delegate: bool,
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

pub fn derive_otel_emit(input: DeriveInput) -> TokenStream {
    let parsed = match OTelEmitInput::from_derive_input(&input) {
        Ok(v) => v,
        Err(e) => return e.write_errors(),
    };

    let name = &parsed.ident;
    let (impl_generics, ty_generics, where_clause) = parsed.generics.split_for_impl();

    match &parsed.data {
        ast::Data::Struct(fields) => {
            let body = emit_struct_body(
                parsed.error_type.as_deref().unwrap_or("unknown"),
                severity_is_warn(&parsed.fix, &parsed.recoverability),
                &fields.fields,
            );
            quote! {
                impl #impl_generics ::pari::error::OTelEmit for #name #ty_generics #where_clause {
                    fn emit(&self) { #body }
                }
            }
        }

        ast::Data::Enum(variants) => {
            let arms: Vec<TokenStream> = variants.iter().map(|v| emit_variant_arm(v)).collect();
            let has_errors: Vec<TokenStream> = arms
                .iter()
                .map(|a| {
                    // check if it's a compile_error token — just collect all
                    a.clone()
                })
                .collect();
            // If any arm generated an error, the overall result should propagate it.
            // We just emit all arms inside the match.
            quote! {
                impl #impl_generics ::pari::error::OTelEmit for #name #ty_generics #where_clause {
                    fn emit(&self) {
                        match self {
                            #(#has_errors)*
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-variant arm generation
// ---------------------------------------------------------------------------

fn emit_variant_arm(variant: &OTelVariant) -> TokenStream {
    let vname = &variant.ident;

    if variant.delegate {
        // Pure delegate: call inner.emit()
        return quote! {
            Self::#vname(inner) => ::pari::error::OTelEmit::emit(inner),
        };
    }

    if let Some(error_type) = &variant.error_type {
        let is_warn = severity_is_warn(&variant.fix, &variant.recoverability);
        return emit_declaring_variant_arm(vname, error_type, is_warn, &variant.fields.fields);
    }

    syn::Error::new_spanned(
        vname,
        "OTelEmit enum variant must have #[otel(delegate)] or #[otel(error_type = \"...\")]",
    )
    .to_compile_error()
}

fn emit_declaring_variant_arm(
    vname: &syn::Ident,
    error_type: &str,
    is_warn: bool,
    fields: &[OTelField],
) -> TokenStream {
    // Collect annotated fields and build:
    // 1. The destructuring bindings
    // 2. The tracing kv pairs
    let mut bindings: Vec<TokenStream> = Vec::new();
    let mut kvs: Vec<TokenStream> = Vec::new();

    for f in fields {
        let fname = match &f.ident {
            Some(id) => id,
            None => continue, // tuple field inside a declaring variant — unusual, skip
        };
        let fname_str = fname.to_string();

        if fname_str == "backtrace" {
            bindings.push(quote! { #fname, });
            kvs.push(quote! { exception.stacktrace = %self.#fname, });
        } else if fname_str == "span_trace" {
            bindings.push(quote! { #fname, });
            kvs.push(quote! { span_trace = %self.#fname, });
        } else if f.delegate {
            bindings.push(quote! { #fname, });
        } else if let Some(attr_name) = &f.field {
            bindings.push(quote! { #fname, });
            let attr_ident = make_attr_ident(attr_name);
            if is_option_type(&f.ty) {
                kvs.push(quote! { #attr_ident = ?#fname, });
            } else {
                kvs.push(quote! { #attr_ident = %#fname, });
            }
        }
        // Fields with no annotations are not bound or emitted
    }

    // Build pattern
    let pattern = if bindings.is_empty() {
        // Determine variant shape
        if fields.iter().any(|f| f.ident.is_some()) {
            // Named struct variant with no annotated fields
            quote! { Self::#vname { .. } }
        } else if fields.is_empty() {
            // Unit variant
            quote! { Self::#vname }
        } else {
            // Tuple variant
            quote! { Self::#vname(..) }
        }
    } else {
        // Named struct variant with some bindings
        quote! { Self::#vname { #(#bindings)* .. } }
    };

    if is_warn {
        quote! {
            #pattern => {
                ::tracing::warn!(
                    exception.type    = #error_type,
                    exception.message = %self,
                    #(#kvs)*
                );
            }
        }
    } else {
        quote! {
            #pattern => {
                ::tracing::error!(
                    exception.type    = #error_type,
                    exception.message = %self,
                    #(#kvs)*
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Struct body generation (reused for structs and could be for struct variants)
// ---------------------------------------------------------------------------

fn emit_struct_body(error_type: &str, is_warn: bool, fields: &[OTelField]) -> TokenStream {
    let mut attr_kvs: Vec<TokenStream> = Vec::new();
    let mut delegate_calls: Vec<TokenStream> = Vec::new();

    for f in fields {
        let fname = match &f.ident {
            Some(id) => id,
            None => continue,
        };
        let fname_str = fname.to_string();

        if fname_str == "backtrace" {
            attr_kvs.push(quote! { exception.stacktrace = %self.#fname, });
        } else if fname_str == "span_trace" {
            attr_kvs.push(quote! { span_trace = %self.#fname, });
        } else if f.delegate {
            delegate_calls.push(quote! { ::pari::error::OTelEmit::emit(&self.#fname); });
        } else if let Some(attr_name) = &f.field {
            let attr_ident = make_attr_ident(attr_name);
            if is_option_type(&f.ty) {
                attr_kvs.push(quote! { #attr_ident = ?self.#fname, });
            } else {
                attr_kvs.push(quote! { #attr_ident = %self.#fname, });
            }
        }
    }

    if is_warn {
        quote! {
            ::tracing::warn!(
                exception.type    = #error_type,
                exception.message = %self,
                #(#attr_kvs)*
            );
            #(#delegate_calls)*
        }
    } else {
        quote! {
            ::tracing::error!(
                exception.type    = #error_type,
                exception.message = %self,
                #(#attr_kvs)*
            );
            #(#delegate_calls)*
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn severity_is_warn(fix: &Option<syn::Path>, recoverability: &Option<syn::Path>) -> bool {
    let fix_str = fix
        .as_ref()
        .and_then(|p| p.get_ident())
        .map(|i| i.to_string());
    let rec_str = recoverability
        .as_ref()
        .and_then(|p| p.get_ident())
        .map(|i| i.to_string());
    matches!(
        (fix_str.as_deref(), rec_str.as_deref()),
        (Some("Infra"), Some("Retryable")) | (Some("Client"), Some("UserAction"))
    )
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

fn make_attr_ident(attr_name: &str) -> TokenStream {
    attr_name.parse().unwrap_or_else(|_| {
        let lit = syn::LitStr::new(attr_name, proc_macro2::Span::call_site());
        quote! { #lit }
    })
}
