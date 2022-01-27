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

use {
    std::{
        collections::HashMap,
        convert::Infallible as Never,
        fmt,
        io,
        path::PathBuf,
    },
    itertools::Itertools as _,
    thiserror::Error,
};
pub use wheel_derive::{
    FromArc,
    bin,
    lib,
    main,
};

// used in proc macro:
#[doc(hidden)] pub use clap;
#[cfg(feature = "rocket-beta")] #[doc(hidden)] pub use dep_rocket_beta as rocket;
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

/// Prints the given text to stdout, without appending a newline but making sure it's actually displayed
#[macro_export] macro_rules! print_flush {
    ($($arg:tt)*) => {{
        use std::io::{
            prelude::*,
            stdout,
        };

        print!($($arg)*);
        stdout().flush()
    }};
}

/// An error that can be returned from the [traits](crate::traits) in this crate.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    /// A subprocess exited with a non-success status.
    #[error("command `{name}` exited with {}", .output.status)]
    CommandExit {
        /// The name of the subprocess, as indicated by the `check` call.
        name: &'static str,
        output: std::process::Output,
    },
    #[error("I/O error{}: {inner}", if let Some(path) = .at { format!(" at {}", path.display()) } else { format!("") })]
    Io {
        #[source]
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

/// Repeatedly prompts the user until they input a valid choice.
pub fn choose<T>(prompt: &str, mut choices: HashMap<String, T>) -> io::Result<T> {
    let mut label = input!("{} [{}] ", prompt, choices.keys().join("/"))?;
    loop {
        if let Some(choice) = choices.remove(label.trim_end_matches(&['\r', '\n'][..])) {
            return Ok(choice)
        }
        label = input!("unrecognized answer, type {}: ", choices.keys().join(" or "))?;
    }
}
