//! Proc macros for the `wheel` crate.

#![deny(
    missing_docs,
    rust_2018_idioms, // this lint is actually about idioms that are *outdated* in Rust 2018
    unused,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    warnings
)]

use {
    proc_macro::TokenStream,
    quote::{
        quote,
        quote_spanned
    },
    syn::{
        FnArg,
        ItemFn,
        parse_macro_input,
        spanned::Spanned as _
    }
};

/// Attribute macro for binary crates.
///
/// This sets some lints to deny, including `warnings`.
///
/// Currently only works on nightly Rust due to <https://github.com/rust-lang/rust/issues/54726>.
#[proc_macro_attribute]
pub fn bin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = proc_macro2::TokenStream::from(item);
    //TODO allow #[bin(gui)] to add #[windows_subsystem = "windows"]
    //TODO add #[forbid(unsafe_code)], allow #[bin(unsafe)] to bypass
    TokenStream::from(quote! {
        #[deny(
            rust_2018_idioms, // this lint is actually about idioms that are *outdated* in Rust 2018
            unused,
            unused_import_braces,
            unused_lifetimes,
            unused_qualifications,
            warnings
        )]
        #item
    })
}

/// Attribute macro for library crates.
///
/// This sets some lints to deny, including `missing_docs` and `warnings`.
///
/// Currently only works on nightly Rust due to <https://github.com/rust-lang/rust/issues/54726>.
#[proc_macro_attribute]
pub fn lib(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = proc_macro2::TokenStream::from(item);
    //TODO allow #[lib(allow_missing_docs)] to bypass #[deny(missing_docs)]
    //TODO add #[forbid(unsafe_code)], allow #[lib(unsafe)] to bypass
    TokenStream::from(quote! {
        #[deny(
            missing_docs,
            rust_2018_idioms, // this lint is actually about idioms that are *outdated* in Rust 2018
            unused,
            unused_import_braces,
            unused_lifetimes,
            unused_qualifications,
            warnings
        )]
        #item
    })
}

/// Annotate your `main` function with this.
///
/// * It can be a `fn` or an `async fn`. In the latter case, `tokio`'s threaded runtime will be used.
/// * It may take a single parameter that implements `paw::ParseArgs` with an `Error` that implements `Display`. If it does, command-line arguments will be parsed into it.
/// * It must return a `Result<(), E>`, for some `E` that implements `Display` (not necessarily the same as the `paw` error).
/// * Any error returned from argument parsing or the function body will be displayed and the process will exit with status code `1`.
#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let main_fn = parse_macro_input!(item as ItemFn);
    let asyncness = &main_fn.sig.asyncness;
    let awaitness = asyncness.as_ref().map(|_| quote!(.await));
    let mut args_iter = main_fn.sig.inputs.iter();
    let arg = match args_iter.next() {
        Some(FnArg::Typed(arg)) => Some(arg),
        Some(FnArg::Receiver(_)) => return quote_spanned! {main_fn.sig.inputs.span()=>
            compile_error!("main should not take self")
        }.into(),
        None => None,
    };
    if args_iter.next().is_some() { return quote_spanned! {main_fn.sig.inputs.span()=>
        compile_error!("main should take one or zero arguments")
    }.into() }
    let ret = main_fn.sig.output;
    let body = main_fn.block;
    TokenStream::from(if let Some(arg) = arg {
        let arg_ty = &arg.ty;
        quote! {
            use ::wheel::tokio;

            #asyncness fn main_inner(#arg) #ret #body

            #[tokio::main]
            #asyncness fn main() {
                //TODO set up a more friendly panic hook (similar to human-panic but actually showing the panic message)
                match <#arg_ty as ::wheel::paw::ParseArgs>::parse_args() {
                    Ok(args) => match main_inner(args)#awaitness {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("{}: {}", env!("CARGO_PKG_NAME"), e);
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: error parsing command line arguments: {}", env!("CARGO_PKG_NAME"), e);
                        std::process::exit(1);
                    }
                }
            }
        }
    } else {
        quote! {
            use ::wheel::tokio;

            #asyncness fn main_inner() #ret #body

            #[tokio::main]
            #asyncness fn main() {
                //TODO set up a more friendly panic hook (similar to human-panic but actually showing the panic message)
                match main_inner()#awaitness {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("{}: {}", env!("CARGO_PKG_NAME"), e);
                        std::process::exit(1);
                    }
                }
            }
        }
    })
}
