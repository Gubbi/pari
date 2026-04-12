//! `pari-macros` — proc-macro crate for pari entities.

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod entity;
mod entity_registry;
mod error_compose;
mod otel_emit;

#[proc_macro_derive(Entity, attributes(entity))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    entity::derive_entity_impl(ast).into()
}

#[proc_macro]
pub fn entity_registry(input: TokenStream) -> TokenStream {
    let registry = parse_macro_input!(input as entity_registry::RegistryInput);
    entity_registry::generate_registry(registry.0).into()
}

#[proc_macro_derive(ErrorCompose, attributes(compose))]
pub fn derive_error_compose(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    error_compose::derive_error_compose(input).into()
}

#[proc_macro_derive(OTelEmit, attributes(otel, compose))]
pub fn derive_otel_emit(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    otel_emit::derive_otel_emit(input).into()
}
