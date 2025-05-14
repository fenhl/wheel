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
                            GenericArgument::Type(type_param) => type_param.clone(),
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
/// * Specify as `#[wheel::main(debug)]` to display the `Debug` output of the value returned from `main`. This is accomplished by passing `true` to the `debug` parameter of `wheel::MainOutput::exit`. This is the default and may be deprecated in the future.
/// * Specify as `#[wheel::main(no_debug)]` to suppress the `Debug` output of the value returned from `main`.
/// * Specify as `#[wheel::main(verbose_debug)]` to only enable `debug` behavior if `wheel::IsVerbose::is_verbose` returns `true` for the parsed command-line arguments.
/// * Specify as `#[wheel::main(rocket)]` to initialize the async runtime using [`rocket::main`](https://docs.rs/rocket/0.5.0/rocket/attr.main.html) instead of [`tokio::main`](https://docs.rs/tokio/latest/tokio/attr.main.html). This requires the `wheel` crate feature `rocket`.
/// * Specify as `#[wheel::main(console = port)]`, where `port` is a [`u16`] literal, to initialize [`console-subscriber`](https://docs.rs/console-subscriber) for Tokio console. Requires `cfg(tokio_unstable)`.
/// * Specify as `#[wheel::main(max_blocking_threads = val)]`, where `val` is an [`i16`] literal, to configure the Tokio runtime's [`max_blocking_threads`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.max_blocking_threads). A value less than one will be added to the [`available_parallelism`](https://doc.rust-lang.org/std/thread/fn.available_parallelism.html), e.g. specifying `#[wheel::main(max_blocking_threads = -1)]` when 16 cores are detected will configure Tokio with 15 `max_blocking_threads`.
///
/// The `custom_exit`, `debug`, `no_debug`, and `verbose_debug` parameters are mutually exclusive, but otherwise parameters can be combined with each other, e.g. `#[wheel::main(no_debug, rocket, console = 6669)]`.
#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, Token![,]>::parse_terminated);
    let mut exit_trait = None;
    let mut debug = Some(true);
    let mut debug_arg = true;
    let mut use_rocket = false;
    let mut console_port = None::<u16>;
    let mut max_blocking_threads = None::<i16>;
    for arg in args {
        if arg.path().is_ident("console") {
            match arg.require_name_value() {
                Ok(MetaNameValue { value, .. }) => if let Expr::Lit(ExprLit { lit: Lit::Int(lit), .. }) = value {
                    if console_port.is_some() {
                        return quote_spanned! {arg.span()=>
                            compile_error!("`#[wheel::main(console)]` specified multiple times");
                        }.into()
                    }
                    match lit.base10_parse() {
                        Ok(port) => console_port = Some(port),
                        Err(e) => return e.into_compile_error().into(),
                    }
                } else {
                    return quote_spanned! {value.span()=>
                        compile_error!("console value must be a u16 literal (the port number)");
                    }.into()
                },
                Err(e) => return e.into_compile_error().into(),
            }
        } else if arg.path().is_ident("custom_exit") {
            if let Err(e) = arg.require_path_only() {
                return e.into_compile_error().into()
            }
            if exit_trait.replace(quote!(::wheel::CustomExit)).is_some() {
                return quote_spanned! {arg.span()=>
                    compile_error!("parameters `custom_exit`, `debug`, `no_debug`, and `verbose_debug` on `#[wheel::main]` are mutually exclusive");
                }.into()
            }
            debug_arg = false;
        } else if arg.path().is_ident("debug") { //TODO deprecate
            if let Err(e) = arg.require_path_only() {
                return e.into_compile_error().into()
            }
            if exit_trait.replace(quote!(::wheel::MainOutput)).is_some() {
                return quote_spanned! {arg.span()=>
                    compile_error!("parameters `custom_exit`, `debug`, `no_debug`, and `verbose_debug` on `#[wheel::main]` are mutually exclusive");
                }.into()
            }
            debug = Some(true);
        } else if arg.path().is_ident("max_blocking_threads") {
            match arg.require_name_value() {
                Ok(MetaNameValue { value, .. }) => if let Expr::Lit(ExprLit { lit: Lit::Int(lit), .. }) = value {
                    if max_blocking_threads.is_some() {
                        return quote_spanned! {arg.span()=>
                            compile_error!("`#[wheel::main(max_blocking_threads)]` specified multiple times");
                        }.into()
                    }
                    match lit.base10_parse() {
                        Ok(val) => max_blocking_threads = Some(val),
                        Err(e) => return e.into_compile_error().into(),
                    }
                } else {
                    return quote_spanned! {value.span()=>
                        compile_error!("max_blocking_threads value must be an i32 literal");
                    }.into()
                },
                Err(e) => return e.into_compile_error().into(),
            }
        } else if arg.path().is_ident("no_debug") {
            if let Err(e) = arg.require_path_only() {
                return e.into_compile_error().into()
            }
            if exit_trait.replace(quote!(::wheel::MainOutput)).is_some() {
                return quote_spanned! {arg.span()=>
                    compile_error!("parameters `custom_exit`, `debug`, `no_debug`, and `verbose_debug` on `#[wheel::main]` are mutually exclusive");
                }.into()
            }
            debug = Some(false);
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
                    compile_error!("parameters `custom_exit`, `debug`, `no_debug`, and `verbose_debug` on `#[wheel::main]` are mutually exclusive");
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
    let init_console_subscriber = if let Some(port) = console_port {
        quote! {
            #[cfg(tokio_unstable)] {
                ::wheel::console_subscriber::ConsoleLayer::builder()
                    .server_addr((::std::net::Ipv4Addr::LOCALHOST, #port))
                    .init();
            }
            #[cfg(not(tokio_unstable))] { compile_error!("#[wheel::main(console)] requires cfg(tokio_unstable)"); }
        }
    } else {
        quote!()
    };
    let (ignore_debug, debug_arg) = if debug_arg {
        (quote!(), quote!(, debug))
    } else {
        (quote!(let _ = debug;), quote!())
    };
    let call_main_inner = if asyncness.is_some() {
        if use_rocket {
            quote!(::wheel::rocket::async_main(__wheel_main_inner(#args)))
        } else {
            let mut builder = quote! {
                ::wheel::tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
            };
            if let Some(max_blocking_threads) = max_blocking_threads {
                builder = if max_blocking_threads > 0 {
                    quote!(#builder.max_blocking_threads(#max_blocking_threads.into()))
                } else {
                    quote!(#builder.max_blocking_threads(::std::thread::available_parallelism().unwrap_or(::std::num::NonZeroUsize::MIN).get().checked_add_signed(#max_blocking_threads.into()).unwrap_or(1)))
                };
            }
            quote! {
                #builder
                    .build().expect("failed to set up tokio runtime in wheel::main")
                    .block_on(__wheel_main_inner(#args))
            }
        }
    } else {
        quote!(__wheel_main_inner(#args))
    };
    TokenStream::from(quote! {
        fn main() {
            #asyncness fn __wheel_main_inner(#arg) #ret #body

            //TODO set up a more friendly panic hook (similar to human-panic but actually showing the panic message)
            #init_console_subscriber
            #parse_args
            #ignore_debug
            let ret_val = #call_main_inner;
            #exit_trait::exit(ret_val, env!("CARGO_PKG_NAME") #debug_arg)
        }
    })
}
