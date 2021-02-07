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
    bin,
    lib,
    main,
};

// used in proc macro:
#[doc(hidden)] pub use paw;
#[cfg(feature = "tokio")] #[doc(hidden)] pub use tokio;
#[cfg(feature = "tokio02")] #[doc(hidden)] pub use tokio02 as tokio;
#[cfg(feature = "tokio03")] #[doc(hidden)] pub use tokio03 as tokio;

/// Members of this trait can be returned from a main function annotated with [`wheel::main`].
pub trait MainOutput {
    /// Exits from the program using this value, displaying it and the given command name (usually `CARGO_PKG_NAME`) in case of an error.
    fn exit(self, cmd_name: &'static str) -> !;
}

impl MainOutput for Never {
    fn exit(self, _: &'static str) -> ! {
        match self {}
    }
}

impl MainOutput for () {
    fn exit(self, _: &'static str) -> ! {
        std::process::exit(0)
    }
}

impl<T: MainOutput, E: fmt::Display> MainOutput for Result<T, E> {
    fn exit(self, cmd_name: &'static str) -> ! {
        match self {
            Ok(x) => x.exit(cmd_name),
            Err(e) => {
                eprintln!("{}: {}", cmd_name, e);
                std::process::exit(1)
            }
        }
    }
}
