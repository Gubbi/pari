//! `#[primitive_error(...)]` attribute macro implementation.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::Parser,
    parse_macro_input, punctuated::Punctuated, Expr, Fields, FieldsNamed, ItemStruct, Lit, MetaNameValue,
    Token,
};

#[derive(Default)]
struct PrimitiveErrorArgs {
    error_type: Option<String>,
}

pub fn primitive_error(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match parse_args(args) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    let item = parse_macro_input!(input as ItemStruct);
    expand_primitive_error(args, item).into()
}

fn parse_args(args: TokenStream) -> syn::Result<PrimitiveErrorArgs> {
    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    let metas = parser.parse(args)?;
    let mut parsed = PrimitiveErrorArgs::default();

    for meta in metas {
        if meta.path.is_ident("error_type") {
            match meta.value {
                Expr::Lit(expr_lit) => match expr_lit.lit {
                    Lit::Str(lit) => parsed.error_type = Some(lit.value()),
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "primitive_error(error_type = ...) expects a string literal",
                        ))
                    }
                },
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "primitive_error(error_type = ...) expects a string literal",
                    ))
                }
            }
        } else {
            return Err(syn::Error::new_spanned(
                meta,
                "unsupported primitive_error argument",
            ));
        }
    }

    Ok(parsed)
}

fn expand_primitive_error(args: PrimitiveErrorArgs, item: ItemStruct) -> proc_macro2::TokenStream {
    let attrs = &item.attrs;
    let vis = &item.vis;
    let name = &item.ident;
    let generics = &item.generics;
    let where_clause = &item.generics.where_clause;

    let fields = match &item.fields {
        Fields::Named(fields) => fields,
        other => {
            return syn::Error::new_spanned(
                other,
                "#[primitive_error] currently supports only structs with named fields",
            )
            .to_compile_error()
        }
    };

    let FieldsNamed { named, .. } = fields;
    let user_fields = named.iter().collect::<Vec<_>>();
    let user_field_idents = user_fields
        .iter()
        .map(|field| field.ident.as_ref().expect("named field"))
        .collect::<Vec<_>>();
    let user_field_types = user_fields.iter().map(|field| &field.ty).collect::<Vec<_>>();

    let detail_pushes = user_field_idents.iter().map(|ident| {
        let field_name = ident.to_string();
        quote! {
            primitive_details.push(::pari::error::PrimitiveDetail {
                field_name: #field_name,
                value: ::std::format!("{:?}", &#ident),
            });
        }
    });
    let error_type = args
        .error_type
        .unwrap_or_else(|| camel_to_snake(&name.to_string()));
    let error_type_lit = syn::LitStr::new(&error_type, proc_macro2::Span::call_site());
    let otel_detail_fields = user_field_idents.iter().map(|ident| {
        let attr_name = format!("error.{}.{}", error_type, ident);
        let attr_lit = syn::LitStr::new(&attr_name, proc_macro2::Span::call_site());
        quote! {
            #attr_lit = ?self.#ident,
        }
    });
    let primitive_details_ident = format_ident!("primitive_details");
    let (impl_generics, ty_generics, _) = generics.split_for_impl();

    quote! {
        #(#attrs)*
        #vis struct #name #generics #where_clause {
            message: ::std::string::String,
            location: ::pari::error::ErrorLocation,
            span_trace: ::tracing_error::SpanTrace,
            backtrace: ::std::backtrace::Backtrace,
            #primitive_details_ident: ::std::vec::Vec<::pari::error::PrimitiveDetail>,
            #(#user_fields),*
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #[track_caller]
            pub fn new(
                message: impl ::std::convert::Into<::std::string::String>,
                #(#user_field_idents: #user_field_types),*
            ) -> Self {
                Self::new_with_location(
                    ::pari::error::ErrorLocation::caller(),
                    message,
                    #(#user_field_idents),*
                )
            }

            pub fn new_with_location(
                location: ::pari::error::ErrorLocation,
                message: impl ::std::convert::Into<::std::string::String>,
                #(#user_field_idents: #user_field_types),*
            ) -> Self {
                let mut primitive_details = ::std::vec::Vec::new();
                #(#detail_pushes)*

                Self {
                    message: message.into(),
                    location,
                    span_trace: ::tracing_error::SpanTrace::capture(),
                    backtrace: ::std::backtrace::Backtrace::capture(),
                    #primitive_details_ident: primitive_details,
                    #(#user_field_idents),*
                }
            }

            pub fn error_layer(&self) -> ::pari::error::ErrorLayer {
                ::pari::error::ErrorLayer::Primitive
            }

            pub fn error_type(&self) -> &'static str {
                #error_type_lit
            }

            pub fn message(&self) -> &str {
                &self.message
            }

            pub fn location(&self) -> &::pari::error::ErrorLocation {
                &self.location
            }

            pub fn span_trace(&self) -> &::tracing_error::SpanTrace {
                &self.span_trace
            }

            pub fn backtrace(&self) -> &::std::backtrace::Backtrace {
                &self.backtrace
            }

            pub fn details(&self) -> &[::pari::error::PrimitiveDetail] {
                &self.#primitive_details_ident
            }
        }

        impl #impl_generics ::std::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::write!(f, "{}", self.message)
            }
        }

        impl #impl_generics ::std::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(::std::stringify!(#name))
                    .field("message", &self.message)
                    .field("location", &self.location)
                    .field("span_trace", &self.span_trace)
                    .field("backtrace", &self.backtrace)
                    #(.field(::std::stringify!(#user_field_idents), &self.#user_field_idents))*
                    .finish()
            }
        }

        impl #impl_generics ::std::error::Error for #name #ty_generics #where_clause {}

        impl #impl_generics ::pari::error::OTelEmit for #name #ty_generics #where_clause {
            fn emit(&self) {
                ::tracing::error!(
                    exception.type = #error_type_lit,
                    exception.message = %self.message,
                    exception.stacktrace = %self.backtrace,
                    "code.file.path" = %self.location.file,
                    "code.line.number" = self.location.line,
                    "code.column.number" = self.location.column,
                    #(#otel_detail_fields)*
                );
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
