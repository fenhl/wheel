//! Proc macros for the `wheel` crate.

#![deny(
    missing_docs,
    rust_2018_idioms, // this lint is actually about idioms that are *outdated* in Rust 2018
    unused,
    unused_crate_dependencies,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    warnings,
)]

use {
    itertools::Itertools as _,
    proc_macro::TokenStream,
    quote::{
        quote,
        quote_spanned,
    },
    syn::{
        AttributeArgs,
        Data,
        DataEnum,
        DeriveInput,
        Fields,
        FieldsUnnamed,
        FnArg,
        GenericArgument,
        ItemFn,
        Meta,
        NestedMeta,
        PathArguments,
        Type,
        parse_macro_input,
        spanned::Spanned as _,
    },
};

/// Implements `From<T>` for enum variants with fields of type `Arc<T>`.
///
/// The conversion is only implemented for variants tagged with `#[from_arc]`.
#[proc_macro_derive(FromArc, attributes(from_arc))]
pub fn from_arc(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = input.ident;
    let derives = match input.data {
        Data::Enum(DataEnum { variants, .. }) => variants.iter()
            .filter(|variant| variant.attrs.iter().any(|attr| attr.path.get_ident().map_or(false, |ident| ident == "from_arc")))
            .map(|variant| {
                let variant_name = &variant.ident;
                let arc_ty = match variant.fields {
                    Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => unnamed.iter()
                        .exactly_one().ok().expect("enum variant tagged with #[from_arc] must have exactly one field")
                        .ty.clone(),
                    _ => panic!("enum variant tagged with #[from_arc] must have an unnamed field"),
                };
                let field_ty = match arc_ty {
                    Type::Path(path) => match path.path.segments.iter().last().expect("empty type path").arguments {
                        PathArguments::AngleBracketed(ref args) => match args.args.iter().exactly_one().ok().expect("field type must have exactly one type argument") {
                            GenericArgument::Type(ref type_param) => type_param.clone(),
                            _ => panic!("field type must have a type parameter"),
                        },
                        _ => panic!("field type must be of the form Arc<T>"),
                    },
                    _ => panic!("field type must be of the form Arc<T>"),
                };
                quote! {
                    impl From<#field_ty> for #ty {
                        fn from(x: #field_ty) -> #ty {
                            #ty::#variant_name(::std::sync::Arc::new(x))
                        }
                    }
                }
            })
            .collect_vec(),
        _ => return quote!(compile_error!("derive(FromArc) is only implemented for enums")).into(),
    };
    TokenStream::from(quote! {
        #(#derives)*
    })
}

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
            unused_crate_dependencies,
            unused_import_braces,
            unused_lifetimes,
            unused_qualifications,
            warnings,
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
            unused_crate_dependencies,
            unused_import_braces,
            unused_lifetimes,
            unused_qualifications,
            warnings,
        )]
        #item
    })
}

enum ParseMode {
    Clap,
    Paw,
}

/// Annotate your `main` function with this.
///
/// * It can be a `fn` or an `async fn`. In the latter case, `tokio`'s threaded runtime will be used. (This requires the `tokio` feature, which is on by default.)
/// * It may take a single parameter that implements `paw::ParseArgs` with an `Error` that implements `Display`. If it does, command-line arguments will be parsed into it.
/// * It must return `()` or a `Result<(), E>`, for some `E` that implements `Display` (not necessarily the same as the `paw` error).
/// * Any error returned from argument parsing or the function body will be displayed and the process will exit with status code `1`.
///
/// The attribute can be specified as `#[wheel::main(clap)]` to parse arguments using the [`clap` 3 beta](https://docs.rs/clap/3.0.0-beta.2) instead of `paw`. This requires the unstable `wheel` crate feature `clap-beta`.
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(attr as AttributeArgs);
    let parse_mode = match &attr_args[..] {
        [] => ParseMode::Paw,
        [NestedMeta::Meta(Meta::Path(path))] if path.get_ident().map_or(false, |ident| ident == "clap") => ParseMode::Clap,
        [arg] => return quote_spanned! {arg.span()=>
            compile_error!("unexpected args parse mode")
        }.into(),
        [_, arg, ..] => return quote_spanned! {arg.span()=>
            compile_error!("unexpected wheel::main attribute argument")
        }.into(),
    };
    let main_fn = parse_macro_input!(item as ItemFn);
    let asyncness = &main_fn.sig.asyncness;
    let use_tokio = asyncness.as_ref().map(|_| quote!(use ::wheel::tokio;));
    let main_prefix = asyncness.as_ref().map(|async_keyword| quote!(#[tokio::main] #async_keyword));
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
    let (arg, args_match, args_pat, args, err_arm) = if let Some(arg) = arg {
        let arg_ty = &arg.ty;
        match parse_mode {
            ParseMode::Clap => (quote!(#arg), quote!(<#arg_ty as ::wheel::clap::Clap>::parse()), quote!(args), quote!(args), quote!()),
            ParseMode::Paw => (quote!(#arg), quote!(<#arg_ty as ::wheel::paw::ParseArgs>::parse_args()), quote!(Ok(args)), quote!(args), quote!(Err(e) => {
                eprintln!("{}: error parsing command line arguments: {}", env!("CARGO_PKG_NAME"), e);
                std::process::exit(1);
            })),
        }
    } else {
        (quote!(), quote!(()), quote!(()), quote!(), quote!())
    };
    let ret = main_fn.sig.output;
    let body = main_fn.block;
    TokenStream::from(quote! {
        #use_tokio

        #asyncness fn main_inner(#arg) #ret #body

        #main_prefix fn main() {
            //TODO set up a more friendly panic hook (similar to human-panic but actually showing the panic message)
            match #args_match {
                #args_pat => ::wheel::MainOutput::exit(main_inner(#args)#awaitness, env!("CARGO_PKG_NAME")),
                #err_arm
            }
        }
    })
}
