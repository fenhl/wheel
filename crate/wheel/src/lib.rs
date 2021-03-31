//! This crate contains boilerplate that is useful in almost every Rust crate.

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

use std::{
    convert::Infallible as Never,
    fmt,
};
pub use wheel_derive::{
    FromArc,
    bin,
    lib,
    main,
};

// used in proc macro:
#[doc(hidden)] pub use paw;
#[cfg(feature = "clap-beta")] #[doc(hidden)] pub use dep_clap_beta as clap; // used in proc macro
#[cfg(feature = "tokio")] #[doc(hidden)] pub use tokio;
#[cfg(feature = "tokio02")] #[doc(hidden)] pub use tokio02 as tokio;
#[cfg(feature = "tokio03")] #[doc(hidden)] pub use tokio03 as tokio;

/// Members of this trait can be returned from a main function annotated with [`wheel::main`].
pub trait MainOutput {
    /// Exits from the program using this value, displaying it and the given command name (usually `CARGO_PKG_NAME`) in case of an error.
    fn exit(self, code: Option<i32>, cmd_name: &'static str) -> !;
}

impl MainOutput for Never {
    fn exit(self, _: Option<i32>, _: &'static str) -> ! {
        match self {}
    }
}

impl MainOutput for () {
    fn exit(self, code: Option<i32>, _: &'static str) -> ! {
        std::process::exit(code.unwrap_or(0))
    }
}

impl<T: MainOutput, E: fmt::Display> MainOutput for Result<T, E> {
    fn exit(self, code: Option<i32>, cmd_name: &'static str) -> ! {
        match self {
            Ok(x) => x.exit(code, cmd_name),
            Err(e) => {
                eprintln!("{}: {}", cmd_name, e);
                std::process::exit(code.unwrap_or(1))
            }
        }
    }
}

/// Use this trait together with a `custom_exit` argument on [`wheel::main`] to customize the exit code of the program based on the value returned from the `main` function.
pub trait CustomExit {
    /// Return the exit code that should be used for this value.
    ///
    /// If this returns `None`, the default (0 for success, 1 for errors) is used.
    fn exit_code(&self) -> Option<i32>;
}

impl<T, E: CustomExit> CustomExit for Result<T, E> {
    fn exit_code(&self) -> Option<i32> {
        match self {
            Ok(_) => None,
            Err(e) => e.exit_code(),
        }
    }
}
