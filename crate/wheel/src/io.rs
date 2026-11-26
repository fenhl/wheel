#![cfg_attr(not(feature = "tokio"), doc = "Extension traits for [`std::io`].")]
#![cfg_attr(feature = "tokio", doc = "Extension traits for [`std::io`] and [`tokio::io`].")]

use {
    std::{
        borrow::Cow,
        fmt,
        io,
        path::{
            Path,
            PathBuf,
        },
    },
    crate::traits::IsNetworkError,
};

/// Used in [`Error`] as metadata for where the error occurred.
#[derive(Debug)]
pub enum ErrorContext {
    /// The error was not annotated with any context.
    Unknown,
    /// The error occurred while working with the given path.
    Path(PathBuf),
    /// The error occurred while working with the two given paths.
    DoublePath(PathBuf, PathBuf),
    /// The error occurred while trying to run a command with the given name.
    Command(Cow<'static, str>),
    /// The error occurred while interacting with the Rust program's standard input.
    Stdin,
    /// The error occurred while interacting with the Rust program's standard output.
    Stdout,
    /// The error occurred while interacting with the Rust program's standard error.
    Stderr,
}

/// A subprocess exited with a non-success status.
#[derive(Debug, thiserror::Error)]
#[error("{context}: {inner}")]
pub struct Error {
    #[source]
    pub inner: io::Error,
    /// The path or command where this error occurred, if known.
    pub context: ErrorContext,
}

impl IsNetworkError for Error {
    fn is_network_error(&self) -> bool {
        self.inner.is_network_error()
    }
}

pub type Result<T = ()> = std::result::Result<T, Error>;

pub trait ResultExt {
    fn exist_ok(self) -> Self;
    fn missing_ok(self) -> Self;
}

impl<T: Default> ResultExt for io::Result<T> {
    fn exist_ok(self) -> Self {
        match self {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(T::default()),
            _ => self,
        }
    }

    fn missing_ok(self) -> Self {
        match self {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(T::default()),
            _ => self,
        }
    }
}

impl<T: Default> ResultExt for Result<T> {
    fn exist_ok(self) -> Self {
        match self {
            Err(e) if e.inner.kind() == io::ErrorKind::AlreadyExists => Ok(T::default()),
            _ => self,
        }
    }

    fn missing_ok(self) -> Self {
        match self {
            Err(e) if e.inner.kind() == io::ErrorKind::NotFound => Ok(T::default()),
            _ => self,
        }
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "I/O error"),
            Self::Path(path) => write!(f, "I/O error at {}", path.display()),
            Self::DoublePath(src, dst) => write!(f, "I/O error at {} and {}", src.display(), dst.display()),
            Self::Command(name) => write!(f, "I/O error in command `{name}`"),
            Self::Stdin => write!(f, "I/O error at stdin"),
            Self::Stdout => write!(f, "I/O error at stdout"),
            Self::Stderr => write!(f, "I/O error at stderr"),
        }
    }
}

/// Allows converting an [`io::Result`] to a [`Result`], optionally annotating it with the location where the error occurred.
pub trait Annotate {
    /// The [`Ok`] variant of the returned [`Result`] type.
    type Ok;

    /// Converts the [`Err`] variant of `self` without annotating it with a path or command context.
    fn at_unknown(self) -> Result<Self::Ok>;
    /// Converts the [`Err`] variant of `self` by annotating it with the given path.
    fn at(self, path: impl AsRef<Path>) -> Result<Self::Ok>;
    /// Converts the [`Err`] variant of `self` by annotating it with the two given paths.
    fn at2(self, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<Self::Ok>;
    /// Converts the [`Err`] variant of `self` by annotating it with the given command name.
    fn at_command(self, name: impl Into<Cow<'static, str>>) -> Result<Self::Ok>;
    /// Converts the [`Err`] variant of `self` by annotating it with the Rust program's standard input.
    fn at_stdin(self) -> Result<Self::Ok>;
    /// Converts the [`Err`] variant of `self` by annotating it with the Rust program's standard output.
    fn at_stdout(self) -> Result<Self::Ok>;
    /// Converts the [`Err`] variant of `self` by annotating it with the Rust program's standard error.
    fn at_stderr(self) -> Result<Self::Ok>;
}

impl<T> Annotate for io::Result<T> {
    type Ok = T;

    fn at_unknown(self) -> Result<T> {
        self.map_err(|inner| Error { inner, context: ErrorContext::Unknown })
    }

    fn at(self, path: impl AsRef<Path>) -> Result<T> {
        self.map_err(|inner| Error { inner, context: ErrorContext::Path(path.as_ref().to_owned()) })
    }

    fn at2(self, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<T> {
        self.map_err(|inner| Error { inner, context: ErrorContext::DoublePath(src.as_ref().to_owned(), dst.as_ref().to_owned()) })
    }

    fn at_command(self, name: impl Into<Cow<'static, str>>) -> Result<T> {
        self.map_err(|inner| Error { inner, context: ErrorContext::Command(name.into()) })
    }

    fn at_stdin(self) -> Result<T> {
        self.map_err(|inner| Error { inner, context: ErrorContext::Stdin })
    }

    fn at_stdout(self) -> Result<T> {
        self.map_err(|inner| Error { inner, context: ErrorContext::Stdout })
    }

    fn at_stderr(self) -> Result<T> {
        self.map_err(|inner| Error { inner, context: ErrorContext::Stderr })
    }
}
