//! This crate contains boilerplate that is useful in almost every Rust crate.

#![deny(missing_docs, rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    std::{
        borrow::Cow,
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
    IsVerbose,
    bin,
    lib,
    main,
};

// used in proc macro:
#[doc(hidden)] pub use clap;
#[cfg(feature = "rocket-beta")] #[doc(hidden)] pub use dep_rocket_beta as rocket;
#[cfg(feature = "rocket-master")] #[doc(hidden)] pub use dep_rocket_master as rocket;
#[cfg(feature = "tokio")] #[doc(hidden)] pub use tokio;

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
        $crate::traits::IoResultExt::at_unknown(stdout().flush().and_then(|()| {
            let mut buf = String::default();
            stdin().read_line(&mut buf)?;
            Ok(buf)
        }))
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
        $crate::traits::IoResultExt::at_unknown(stdout().flush())
    }};
}

/// Used in [`Error::Io`] as metadata for where the error occurred.
#[derive(Debug)]
pub enum IoErrorContext {
    /// The error was not annotated with any context.
    Unknown,
    /// The error occurred while working with the given path.
    Path(PathBuf),
    /// The error occurred while working with the two given paths.
    DoublePath(PathBuf, PathBuf),
    /// The error occurred while trying to run a command with the given name.
    Command(Cow<'static, str>),
}

impl fmt::Display for IoErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "I/O error"),
            Self::Path(path) => write!(f, "I/O error at {}", path.display()),
            Self::DoublePath(src, dst) => write!(f, "I/O error at {} and {}", src.display(), dst.display()),
            Self::Command(name) => write!(f, "in command `{name}`"),
        }
    }
}

/// An error that can be returned from the [traits] in this crate.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    #[cfg(all(feature = "reqwest", feature = "serde", feature = "serde_json"))] #[error(transparent)] Reqwest(#[from] reqwest::Error),
    /// A subprocess exited with a non-success status. Output information is available.
    #[error("command `{name}` exited with {}", .output.status)]
    CommandExit {
        /// The name of the subprocess, as indicated by the `check` call.
        name: Cow<'static, str>,
        output: std::process::Output,
    },
    /// A subprocess exited with a non-success status. Output information is unavailable.
    #[error("command `{name}` exited with {}", .status)]
    CommandExitStatus {
        /// The name of the subprocess, as indicated by the `check` call.
        name: Cow<'static, str>,
        status: std::process::ExitStatus,
    },
    #[error("{context}: {inner}")]
    Io {
        #[source]
        inner: io::Error,
        /// The path or command where this error occurred, if known.
        context: IoErrorContext,
    },
    #[cfg(all(feature = "serde", feature = "serde_json"))]
    #[error("{context}: {inner}")]
    Json {
        #[source]
        inner: serde_json::Error,
        /// The path or command where this error occurred, if known.
        context: IoErrorContext,
    },
    #[cfg(all(feature = "reqwest", feature = "serde", feature = "serde_json"))]
    #[error("{inner}, body:\n\n{text}")]
    ResponseJson {
        #[source]
        inner: serde_json::Error,
        text: String,
    },
    #[cfg(feature = "reqwest")]
    #[error("{inner}, body:\n\n{}", .text.as_ref().map(|text| text.clone()).unwrap_or_else(|e| e.to_string()))]
    ResponseStatus {
        #[source]
        inner: reqwest::Error,
        headers: reqwest::header::HeaderMap,
        text: reqwest::Result<String>,
    },
}

/// A shorthand for a result with defaults for both variants (unit and this crate's [`enum@Error`], respectively).
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// Members of this trait can be returned from a main function annotated with [`main`].
pub trait MainOutput {
    /// Exits from the program using this value, displaying it and the given command name (usually `CARGO_PKG_NAME`) in case of an error.
    fn exit(self, cmd_name: &'static str, debug: bool) -> !;
}

impl MainOutput for Never {
    fn exit(self, _: &'static str, _: bool) -> ! {
        match self {}
    }
}

impl MainOutput for () {
    fn exit(self, _: &'static str, _: bool) -> ! {
        std::process::exit(0)
    }
}

impl MainOutput for bool {
    fn exit(self, _: &'static str, _: bool) -> ! {
        std::process::exit(if self { 0 } else { 1 })
    }
}

impl MainOutput for i32 {
    fn exit(self, _: &'static str, _: bool) -> ! {
        std::process::exit(self)
    }
}

impl<T: MainOutput, E: fmt::Debug + fmt::Display> MainOutput for Result<T, E> {
    fn exit(self, cmd_name: &'static str, debug: bool) -> ! {
        match self {
            Ok(x) => x.exit(cmd_name, debug),
            Err(e) => {
                eprintln!("{cmd_name}: {e}");
                if debug {
                    eprintln!("debug info: {e:?}");
                }
                std::process::exit(1)
            }
        }
    }
}

/// Implement this trait for your `main` arguments to use the `verbose_debug` argument on [`main`].
pub trait IsVerbose {
    /// Returns whether a `--verbose` argument is present.
    fn is_verbose(&self) -> bool;
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
pub fn choose<T>(prompt: &str, mut choices: HashMap<String, T>) -> Result<T> {
    let mut label = input!("{prompt} [{}] ", choices.keys().join("/"))?;
    loop {
        if let Some(choice) = choices.remove(label.trim_end_matches(&['\r', '\n'][..])) {
            return Ok(choice)
        }
        label = input!("unrecognized answer, type {}: ", choices.keys().join(" or "))?;
    }
}

/// Repeatedly prompts the user until they answer “yes” or “no”.
pub fn yesno(prompt: &str) -> Result<bool> {
    let mut label = input!("{prompt} [y/n] ")?;
    loop {
        match &*label.trim_end_matches(&['\r', '\n'][..]).to_ascii_lowercase() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => label = input!("unrecognized answer, type “yes” or “no”: ")?,
        }
    }
}
