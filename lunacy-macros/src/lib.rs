//! Compile-time entry-point support, re-exported by lunacy's `macros` feature.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, Meta, Path, Token, parse_macro_input, punctuated::Punctuated};

/// Supplies lunacy's C entry point, libc allocator, and aborting panic handler.
///
/// Available through lunacy's `macros` feature, enabled by default. Apply
/// `#[lunacy::main]` once to a synchronous `fn(Args<'_>) -> i32` in a
/// `#![no_std]`, `#![no_main]` executable. The function may have any name and
/// remains callable from Rust.
/// Set `panic = "abort"` in the application's Cargo profiles.
///
/// ```ignore
/// #![no_std]
/// #![no_main]
///
/// #[lunacy::main]
/// fn main(args: lunacy::Args<'_>) -> i32 {
///     let _ = lunacy::println!("{} arguments", args.len());
///     0
/// }
/// ```
///
/// Expands to the original function and a `lunacy_main!` invocation; the
/// runtime behavior and signature requirements are the same as that macro.
/// A dependency renamed to `luna` can use `#[luna::main(crate = luna)]`.
/// Conditional compilation on the function also applies to its runtime setup.
#[proc_macro_attribute]
pub fn main(attributes: TokenStream, item: TokenStream) -> TokenStream {
    let mut crate_path: Option<Path> = None;
    let options = syn::meta::parser(|meta| {
        if !meta.path.is_ident("crate") {
            return Err(meta.error("expected `crate = path`"));
        }
        if crate_path.is_some() {
            return Err(meta.error("duplicate `crate` option"));
        }
        crate_path = Some(meta.value()?.parse()?);
        Ok(())
    });
    parse_macro_input!(attributes with options);
    let function = parse_macro_input!(item as ItemFn);
    let crate_path = crate_path.unwrap_or_else(|| syn::parse_quote!(::lunacy));
    match expand(function, crate_path) {
        Ok(output) => output,
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand(function: ItemFn, crate_path: Path) -> syn::Result<TokenStream> {
    let signature = &function.sig;
    if signature.asyncness.is_some()
        || signature.constness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
    {
        return Err(syn::Error::new_spanned(
            signature,
            "lunacy::main requires a safe, synchronous, non-generic Rust function",
        ));
    }
    if signature.inputs.len() != 1
        || signature
            .inputs
            .iter()
            .any(|arg| matches!(arg, syn::FnArg::Receiver(_)))
    {
        return Err(syn::Error::new_spanned(
            signature,
            "lunacy::main requires one Args<'_> argument and an i32 return value",
        ));
    }

    // Keep only conditional compilation on the runtime invocation. Other
    // attributes (inline, docs, lint settings, etc.) belong to the function.
    let mut conditions = Vec::new();
    for attribute in &function.attrs {
        if let Some(condition) = compilation_condition(&attribute.meta)? {
            conditions.push(condition);
        }
    }
    let name = &signature.ident;
    Ok(quote! {
        #function
        #(#[#conditions])*
        #crate_path::lunacy_main!(#name);
    }
    .into())
}

fn compilation_condition(meta: &Meta) -> syn::Result<Option<Meta>> {
    if meta.path().is_ident("cfg") {
        return Ok(Some(meta.clone()));
    }
    if let Meta::List(list) = meta {
        if list.path.is_ident("cfg_attr") {
            let arguments =
                list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            let mut arguments = arguments.into_iter();
            let predicate = arguments.next().ok_or_else(|| {
                syn::Error::new_spanned(meta, "cfg_attr requires a predicate and attributes")
            })?;
            let mut conditions = Vec::new();
            for argument in arguments {
                if let Some(condition) = compilation_condition(&argument)? {
                    conditions.push(condition);
                }
            }
            if !conditions.is_empty() {
                return Ok(Some(
                    syn::parse_quote!(cfg_attr(#predicate, #(#conditions),*)),
                ));
            }
        }
    }
    Ok(None)
}
