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
    io,
    path::PathBuf,
};
pub use wheel_derive::{
    FromArc,
    bin,
    lib,
    main,
};

// used in proc macro:
#[doc(hidden)] pub use {
    paw,
    structopt,
};
#[cfg(feature = "clap-beta")] #[doc(hidden)] pub use dep_clap_beta as clap;
#[cfg(feature = "tokio")] #[doc(hidden)] pub use tokio;
#[cfg(feature = "tokio02")] #[doc(hidden)] pub use tokio02 as tokio;
#[cfg(feature = "tokio03")] #[doc(hidden)] pub use tokio03 as tokio;

#[cfg(feature = "tokio")] pub mod fs;
pub mod traits;

/// Prints the given prompt to stdout, then reads and returns a line from stdin.
#[macro_export] macro_rules! input {
    ($($arg:tt)*) => {{
        use std::io::{
            prelude::*,
            stdin,
            stdout,
        };

        print!($($arg)*);
        stdout().flush().and_then(|()| {
            let mut buf = String::default();
            stdin().read_line(&mut buf)?;
            Ok(buf)
        })
    }};
}

/// An error that can be returned from the [traits](crate::traits) in this crate.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Error {
    /// A subprocess exited with a non-success status.
    CommandExit {
        /// The name of the subprocess, as indicated by the `check` call.
        name: &'static str,
        output: std::process::Output,
    },
    Io {
        inner: io::Error,
        /// The path where this error occurred, if known.
        at: Option<PathBuf>,
    },
}

impl traits::FromIoError for Error {
    fn from_io_at(inner: io::Error, path: impl AsRef<std::path::Path>) -> Self {
        Self::Io { inner, at: Some(path.as_ref().to_owned()) }
    }

    fn from_io_at_unknown(inner: io::Error) -> Self {
        Self::Io { inner, at: None }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandExit { name, output } => write!(f, "command `{}` exited with {}", name, output.status),
            Self::Io { inner, at: Some(path) } => write!(f, "I/O error at {}: {}", path.display(), inner),
            Self::Io { inner, at: None } => write!(f, "I/O error: {}", inner),
        }
    }
}

impl std::error::Error for Error {}

/// A shorthand for a result with defaults for both variants (unit and this crate's [`Error`], respectively).
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// Members of this trait can be returned from a main function annotated with [`main`].
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

/// Use this trait together with a `custom_exit` argument on [`main`] to customize the behavior of the program when exiting with an error.
pub trait CustomExit {
    /// Exits from the program using this value, displaying it and the given command name (usually `CARGO_PKG_NAME`) in case of an error.
    fn exit(self, cmd_name: &'static str) -> !;
}

impl CustomExit for Never {
    fn exit(self, _: &'static str) -> ! {
        match self {}
    }
}

impl CustomExit for () {
    fn exit(self, _: &'static str) -> ! {
        std::process::exit(0)
    }
}

impl<T: CustomExit, E: CustomExit> CustomExit for Result<T, E> {
    fn exit(self, cmd_name: &'static str) -> ! {
        match self {
            Ok(x) => x.exit(cmd_name),
            Err(e) => e.exit(cmd_name),
        }
    }
}
