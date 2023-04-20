//! Proc macros for the `wheel` crate.

#![deny(missing_docs, rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    itertools::Itertools as _,
    proc_macro::TokenStream,
    quote::{
        quote,
        quote_spanned,
    },
    syn::{
        *,
        punctuated::Punctuated,
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
            .filter(|variant| variant.attrs.iter().any(|attr| attr.path().get_ident().map_or(false, |ident| ident == "from_arc")))
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
        _ => return quote!(compile_error!("derive(FromArc) is only implemented for enums");).into(),
    };
    TokenStream::from(quote! {
        #(#derives)*
    })
}

/// Implements the `IsVerbose` trait for a struct with a `verbose: bool` field.
///
/// This trait is used with `#[wheel::main(verbose_debug)]`.
#[proc_macro_derive(IsVerbose)]
pub fn is_verbose(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = input.ident;
    TokenStream::from(quote! {
        impl ::wheel::IsVerbose for #ty {
            fn is_verbose(&self) -> bool {
                self.verbose
            }
        }
    })
}

/// Attribute macro for binary crates.
///
/// This sets some lints to deny, including `warnings`.
///
/// Currently only works on nightly Rust due to <https://github.com/rust-lang/rust/issues/54726>.
#[proc_macro_attribute]
pub fn bin(_: TokenStream, item: TokenStream) -> TokenStream {
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
pub fn lib(_: TokenStream, item: TokenStream) -> TokenStream {
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

/// Annotate your `main` function with this.
///
/// * It can be a `fn` or an `async fn`. In the latter case, `tokio`'s threaded runtime will be used. (This requires the `tokio` feature, which is on by default.)
/// * It may take a single parameter that implements `clap::Parser`. If it does, command-line arguments will be parsed into it.
///     * If the parameter is omitted, a simple argument parser will be used to add support for `--help` and `--version`, and to reject any other arguments.
/// * It must return `()` or a `Result<(), E>`, for some `E` that implements `Display`.
/// * Any error returned from argument parsing or the function body will be displayed and the process will exit with status code `1`.
///
/// The attribute takes optional parameters to modify its behavior:
///
/// * Specify as `#[wheel::main(custom_exit)]` to handle the `main` function's return value using the `wheel::CustomExit` trait instead of `wheel::MainOutput`, allowing to customize error handling behavior.
/// * Specify as `#[wheel::main(debug)]` to display the `Debug` output of the value returned from `main` in addition to using `wheel::MainOutput`.
/// * Specify as `#[wheel::main(rocket)]` to initialize the async runtime using [`rocket::main`](https://docs.rs/rocket/0.5.0-rc.1/rocket/attr.main.html) instead of [`tokio::main`](https://docs.rs/tokio/latest/tokio/attr.main.html). This requires the unstable `wheel` crate feature `rocket-beta`.
/// * Specify as `#[wheel::main(verbose_debug)]` to enable `debug` behavior if `wheel::IsVerbose::is_verbose` returns `true` for the
///
/// The `rocket` parameter can also be combined with one of the others, e.g. `#[wheel::main(debug, rocket)]`.
#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, Token![,]>::parse_terminated);
    let mut exit_trait = None;
    let mut debug = Some(false);
    let mut use_rocket = false;
    for arg in args {
        if arg.path().is_ident("custom_exit") {
            if let Err(e) = arg.require_path_only() {
                return e.into_compile_error().into()
            }
            if exit_trait.replace(quote!(::wheel::CustomExit)).is_some() {
                return quote_spanned! {arg.span()=>
                    compile_error!("parameters `custom_exit`, `debug`, and `verbose_debug` on `#[wheel::main]` are mutually exclusive");
                }.into()
            }
        } else if arg.path().is_ident("debug") {
            if let Err(e) = arg.require_path_only() {
                return e.into_compile_error().into()
            }
            if exit_trait.replace(quote!(::wheel::MainOutput)).is_some() {
                return quote_spanned! {arg.span()=>
                    compile_error!("parameters `custom_exit`, `debug`, and `verbose_debug` on `#[wheel::main]` are mutually exclusive");
                }.into()
            }
            debug = Some(true);
        } else if arg.path().is_ident("rocket") {
            if let Err(e) = arg.require_path_only() {
                return e.into_compile_error().into()
            }
            if use_rocket {
                return quote_spanned! {arg.span()=>
                    compile_error!("`#[wheel::main(rocket)]` specified multiple times");
                }.into()
            }
            use_rocket = true;
        } else if arg.path().is_ident("verbose_debug") {
            if let Err(e) = arg.require_path_only() {
                return e.into_compile_error().into()
            }
            if exit_trait.replace(quote!(::wheel::MainOutput)).is_some() {
                return quote_spanned! {arg.span()=>
                    compile_error!("parameters `custom_exit`, `debug`, and `verbose_debug` on `#[wheel::main]` are mutually exclusive");
                }.into()
            }
            debug = None;
        } else {
            return quote_spanned! {arg.span()=>
                compile_error!("unexpected wheel::main attribute argument");
            }.into()
        }
    }
    let exit_trait = exit_trait.unwrap_or(quote!(::wheel::MainOutput));
    let main_fn = parse_macro_input!(item as ItemFn);
    let asyncness = &main_fn.sig.asyncness;
    let use_tokio = asyncness.as_ref().map(|_| if use_rocket { quote!(use ::wheel::rocket;) } else { quote!(use ::wheel::tokio;) });
    let main_prefix = asyncness.as_ref().map(|async_keyword| if use_rocket { quote!(#[rocket::main] #async_keyword) } else { quote!(#[tokio::main] #async_keyword) });
    let awaitness = asyncness.as_ref().map(|_| quote!(.await));
    let (arg, parse_args, args) = match main_fn.sig.inputs.iter().at_most_one() {
        Ok(Some(FnArg::Typed(arg))) => {
            let arg_ty = &arg.ty;
            let debug = match debug {
                Some(true) => quote!(true),
                Some(false) => quote!(false),
                None => quote!(::wheel::IsVerbose::is_verbose(&args)),
            };
            let parse_args = quote_spanned! {arg.ty.span()=>
                let args = <#arg_ty as ::wheel::clap::Parser>::parse();
                let debug = #debug;
            };
            (quote!(#arg), parse_args, quote!(args))
        }
        Ok(Some(FnArg::Receiver(_))) => return quote_spanned! {main_fn.sig.inputs.span()=>
            compile_error!("main should not take self");
        }.into(),
        Ok(None) => {
            let command = quote!(::wheel::clap::Command::new(env!("CARGO_PKG_NAME")).version(env!("CARGO_PKG_VERSION")));
            let parse_args = match debug {
                Some(true) => quote! {
                    #command.get_matches();
                    let debug = true;
                },
                Some(false) => quote! {
                    #command.get_matches();
                    let debug = false;
                },
                None => quote! {
                    let matches = #command.arg(::wheel::clap::Arg::new("verbose").short('v').long("verbose").help("Display debug info if an error occurs")).get_matches();
                    let debug = matches.is_present("verbose");
                },
            };
            (quote!(), parse_args, quote!())
        }
        Err(_) => return quote_spanned! {main_fn.sig.inputs.span()=>
            compile_error!("main should take one or zero arguments");
        }.into(),
    };
    let ret = main_fn.sig.output;
    let body = main_fn.block;
    TokenStream::from(quote! {
        #use_tokio

        #asyncness fn main_inner(#arg) #ret #body

        #main_prefix fn main() {
            //TODO set up a more friendly panic hook (similar to human-panic but actually showing the panic message)
            #parse_args
            let ret_val = main_inner(#args)#awaitness;
            #exit_trait::exit(ret_val, env!("CARGO_PKG_NAME"), debug)
        }
    })
}
