//! This crate contains boilerplate that is useful in almost every Rust crate.

use {
    std::{
        borrow::Cow,
        collections::HashMap,
        convert::Infallible as Never,
        fmt,
        io::{
            self,
            prelude::*,
        },
        path::PathBuf,
        process::Stdio,
    },
    itertools::Itertools as _,
    thiserror::Error,
    crate::traits::{
        IoResultExt as _,
        SyncCommandOutputExt as _,
    },
};
pub use wheel_derive::{
    FromArc,
    IsVerbose,
    bin,
    lib,
    main,
};
#[cfg(feature = "pyo3")] use pyo3::{
    exceptions::PyException,
    prelude::*,
};
#[cfg(feature = "tokio")] use {
    tokio::{
        io::AsyncWriteExt as _,
        process::Command,
    },
    crate::traits::AsyncCommandOutputExt as _,
};

// used in proc macro:
#[doc(hidden)] pub use clap;
#[cfg(tokio_unstable)] #[doc(hidden)] pub use console_subscriber;
#[cfg(feature = "rocket")] #[doc(hidden)] pub use rocket;
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
#[cfg_attr(feature = "rocket", derive(rocket_util::Error))]
pub enum Error {
    #[cfg(all(feature = "chrono", feature = "reqwest", feature = "tokio"))] #[error(transparent)] HeaderToStr(#[from] reqwest::header::ToStrError),
    #[cfg(all(feature = "chrono", feature = "reqwest", feature = "tokio"))] #[error(transparent)] ParseInt(#[from] std::num::ParseIntError),
    #[cfg(any(all(feature = "reqwest", feature = "serde_json"), all(feature = "chrono", feature = "reqwest", feature = "tokio")))] #[error(transparent)] Reqwest(#[from] reqwest::Error),
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
    #[cfg(all(feature = "chrono", feature = "reqwest", feature = "tokio"))]
    #[error("x-ratelimit-reset header is out of range for chrono::DateTime")]
    InvalidDateTime,
    #[error("{context}: {inner}")]
    Io {
        #[source]
        inner: io::Error,
        /// The path or command where this error occurred, if known.
        context: IoErrorContext,
    },
    #[cfg(feature = "serde_json")]
    #[error("{context}: {inner}")]
    Json {
        #[source]
        inner: serde_json::Error,
        /// The path or command where this error occurred, if known.
        context: IoErrorContext,
    },
    #[cfg(feature = "serde_json")]
    #[error("{context}: {inner}")]
    JsonPathToError {
        #[source]
        inner: serde_json_path_to_error::Error,
        /// The path or command where this error occurred, if known.
        context: IoErrorContext,
    },
    #[cfg(all(feature = "chrono", feature = "reqwest", feature = "tokio"))]
    #[error("missing x-ratelimit-reset header in GitHub error response")]
    MissingRateLimitResetHeader,
    #[cfg(all(feature = "reqwest", feature = "serde_json"))]
    #[error("{inner}, body:\n\n{text}")]
    ResponseJson {
        #[source]
        inner: serde_json::Error,
        text: String,
    },
    #[cfg(all(feature = "reqwest", feature = "serde_json"))]
    #[error("{inner}, body:\n\n{text}")]
    ResponseJsonPathToError {
        #[source]
        inner: serde_json_path_to_error::Error,
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
    #[cfg(all(feature = "chrono", feature = "reqwest", feature = "tokio"))]
    #[error("attempted to send GitHub API request with streamed body")]
    UncloneableGitHubRequest,
}

#[cfg(feature = "pyo3")]
impl From<Error> for PyErr {
    fn from(e: Error) -> Self {
        PyException::new_err(e.to_string()) //TODO different exception classes for different Error variants?
    }
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

/// Converts a [`reqwest::Error`] into an [`io::Error`] with an appropriate [`io::ErrorKind`].
#[cfg(feature = "reqwest")]
pub fn io_error_from_reqwest(e: reqwest::Error) -> io::Error {
    io::Error::new(if e.is_timeout() {
        io::ErrorKind::TimedOut
    } else {
        io::ErrorKind::Other //TODO use an approprriate error kind where possible
    }, e)
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

#[cfg(feature = "tokio")]
/// Report an error to `night`, my personal status monitor system.
///
/// Only works if called on mercredi as a user who has access to `nightd report` via sudo.
pub async fn night_report(path: &str, extra: Option<&str>) -> Result<std::process::Output> {
    let mut cmd = Command::new("sudo");
    cmd.arg("-u").arg("fenhl").arg("/opt/night/bin/nightd").arg("report").arg(path);
    if extra.is_some() {
        cmd.stdin(Stdio::piped());
    }
    let mut child = cmd.spawn().at_command("sudo -u fenhl /opt/night/bin/nightd report")?;
    if let Some(extra) = extra {
        child.stdin.take().expect("configured above").write_all(extra.as_ref()).await.at_command("sudo -u fenhl /opt/night/bin/nightd report")?;
    }
    child.check("sudo -u fenhl /opt/night/bin/nightd report").await
}

/// Report an error to `night`, my personal status monitor system.
///
/// Only works if called on mercredi as a user who has access to `nightd report` via sudo.
pub fn night_report_sync(path: &str, extra: Option<&str>) -> Result<std::process::Output> {
    let mut cmd = std::process::Command::new("sudo");
    cmd.arg("-u").arg("fenhl").arg("/opt/night/bin/nightd").arg("report").arg(path);
    if extra.is_some() {
        cmd.stdin(Stdio::piped());
    }
    let mut child = cmd.spawn().at_command("sudo -u fenhl /opt/night/bin/nightd report")?;
    if let Some(extra) = extra {
        child.stdin.take().expect("configured above").write_all(extra.as_ref()).at_command("sudo -u fenhl /opt/night/bin/nightd report")?;
    }
    child.check("sudo -u fenhl /opt/night/bin/nightd report")
}
