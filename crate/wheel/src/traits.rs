//! Boilerplate extension traits.

use {
    std::{
        borrow::Cow,
        convert::Infallible,
        io,
        path::Path,
    },
    async_trait::async_trait,
    crate::{
        Error,
        IoErrorContext,
        Result,
    },
};
#[cfg(windows)] use std::os::windows::process::CommandExt as _;
#[cfg(all(feature = "reqwest", feature = "serde", feature = "serde_json"))] use serde::de::DeserializeOwned;

/// A convenience method for working with infallible results
pub trait ResultNeverExt {
    /// The `Ok` type of the result.
    type Ok;

    /// Returns the `Ok` variant of this result.
    fn never_unwrap(self) -> Self::Ok;
}

impl<T> ResultNeverExt for Result<T, Infallible> {
    type Ok = T;

    fn never_unwrap(self) -> T {
        match self {
            Ok(inner) => inner,
            Err(never) => match never {},
        }
    }
}

/// A convenience method for working with always-error results
pub trait ResultNeverErrExt {
    /// The `Err` type of the result.
    type Err;

    /// Returns the `Err` variant of this result.
    fn never_unwrap_err(self) -> Self::Err;
}

impl<E> ResultNeverErrExt for Result<Infallible, E> {
    type Err = E;

    fn never_unwrap_err(self) -> E {
        match self {
            Ok(never) => match never {},
            Err(inner) => inner,
        }
    }
}

/// Allows converting an [`io::Result`] to a [`Result`], optionally annotating it with the location where the error occurred.
pub trait IoResultExt {
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
    /// Converts an [`Err`] with [`io::ErrorKind::AlreadyExists`] to `Ok(default())`.
    fn exist_ok(self) -> Self where Self::Ok: Default;
    /// Converts an [`Err`] with [`io::ErrorKind::NotFound`] to `Ok(default())`.
    fn missing_ok(self) -> Self where Self::Ok: Default;
}

impl<T> IoResultExt for io::Result<T> {
    type Ok = T;

    fn at_unknown(self) -> Result<T> {
        self.map_err(|inner| Error::Io { inner, context: IoErrorContext::Unknown })
    }

    fn at(self, path: impl AsRef<Path>) -> Result<T> {
        self.map_err(|inner| Error::Io { inner, context: IoErrorContext::Path(path.as_ref().to_owned()) })
    }

    fn at2(self, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<T> {
        self.map_err(|inner| Error::Io { inner, context: IoErrorContext::DoublePath(src.as_ref().to_owned(), dst.as_ref().to_owned()) })
    }

    fn at_command(self, name: impl Into<Cow<'static, str>>) -> Result<T> {
        self.map_err(|inner| Error::Io { inner, context: IoErrorContext::Command(name.into()) })
    }

    fn exist_ok(self) -> Self where T: Default {
        match self {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(T::default()),
            _ => self,
        }
    }

    fn missing_ok(self) -> Self where T: Default {
        match self {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(T::default()),
            _ => self,
        }
    }
}

impl<T> IoResultExt for Result<T> {
    type Ok = T;

    fn at_unknown(self) -> Result<T> {
        match self {
            Err(Error::Io { inner, .. }) => Err(Error::Io { inner, context: IoErrorContext::Unknown }),
            _ => self,
        }
    }

    fn at(self, path: impl AsRef<Path>) -> Result<T> {
        match self {
            Err(Error::Io { inner, .. }) => Err(Error::Io { inner, context: IoErrorContext::Path(path.as_ref().to_owned()) }),
            _ => self,
        }
    }

    fn at2(self, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<T> {
        match self {
            Err(Error::Io { inner, .. }) => Err(Error::Io { inner, context: IoErrorContext::DoublePath(src.as_ref().to_owned(), dst.as_ref().to_owned()) }),
            _ => self,
        }
    }

    fn at_command(self, name: impl Into<Cow<'static, str>>) -> Result<T> {
        match self {
            Err(Error::Io { inner, .. }) => Err(Error::Io { inner, context: IoErrorContext::Command(name.into()) }),
            _ => self,
        }
    }

    fn exist_ok(self) -> Self where T: Default {
        match self {
            Err(Error::Io { inner, .. }) if inner.kind() == io::ErrorKind::AlreadyExists => Ok(T::default()),
            _ => self,
        }
    }

    fn missing_ok(self) -> Self where T: Default {
        match self {
            Err(Error::Io { inner, .. }) if inner.kind() == io::ErrorKind::NotFound => Ok(T::default()),
            _ => self,
        }
    }
}

#[cfg(all(feature = "serde", feature = "serde_json"))]
impl<T> IoResultExt for serde_json::Result<T> {
    type Ok = T;

    fn at_unknown(self) -> Result<T> {
        self.map_err(|inner| Error::Json { inner, context: IoErrorContext::Unknown })
    }

    fn at(self, path: impl AsRef<Path>) -> Result<T> {
        self.map_err(|inner| Error::Json { inner, context: IoErrorContext::Path(path.as_ref().to_owned()) })
    }

    fn at2(self, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<T> {
        self.map_err(|inner| Error::Json { inner, context: IoErrorContext::DoublePath(src.as_ref().to_owned(), dst.as_ref().to_owned()) })
    }

    fn at_command(self, name: impl Into<Cow<'static, str>>) -> Result<T> {
        self.map_err(|inner| Error::Json { inner, context: IoErrorContext::Command(name.into()) })
    }

    fn exist_ok(self) -> Self where T: Default { self }
    fn missing_ok(self) -> Self where T: Default { self }
}

#[cfg_attr(feature = "tokio", doc = "Extension methods for [`tokio::process::Command`] and [`std::process::Command`]")]
#[cfg_attr(not(feature = "tokio"), doc = "Extension methods for [`std::process::Command`]")]
pub trait CommandExt {
    /// Suppresses creating a console window on Windows. Has no effect on other platforms.
    fn create_no_window(&mut self) -> &mut Self;
}

#[cfg(feature = "tokio")]
impl CommandExt for tokio::process::Command {
    fn create_no_window(&mut self) -> &mut tokio::process::Command {
        #[cfg(windows)] { self.creation_flags(0x0800_0000) }
        #[cfg(not(windows))] { self }
    }
}

impl CommandExt for std::process::Command {
    fn create_no_window(&mut self) -> &mut std::process::Command {
        #[cfg(windows)] { self.creation_flags(0x0800_0000) }
        #[cfg(not(windows))] { self }
    }
}

/// Adds a `check` method which errors if the command doesn't exit successfully.
#[async_trait]
pub trait AsyncCommandOutputExt {
    /// The type retrurned by `check` in the success case.
    type Ok;

    /// Errors if the command doesn't exit successfully.
    async fn check(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Ok>;
}

#[cfg(feature = "tokio")]
#[async_trait]
impl AsyncCommandOutputExt for tokio::process::Command {
    type Ok = std::process::Output;

    async fn check(mut self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Ok> {
        (&mut self).check(name).await
    }
}

#[cfg(feature = "tokio")]
#[async_trait]
impl<'a> AsyncCommandOutputExt for &'a mut tokio::process::Command {
    type Ok = std::process::Output;

    async fn check(mut self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Ok> {
        let output = self.output().await.at_command(name.clone())?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::CommandExit { name: name.into(), output })
        }
    }
}

#[cfg(feature = "tokio")]
#[async_trait]
impl AsyncCommandOutputExt for tokio::process::Child {
    type Ok = std::process::Output;

    async fn check(mut self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Ok> {
        let output = self.wait_with_output().await.at_command(name.clone())?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::CommandExit { name: name.into(), output })
        }
    }
}

#[cfg(feature = "tokio")]
#[async_trait]
impl<'a> AsyncCommandOutputExt for &'a mut tokio::process::Child {
    type Ok = std::process::ExitStatus;

    async fn check(mut self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Ok> {
        let status = self.wait().await.at_command(name.clone())?;
        if status.success() {
            Ok(status)
        } else {
            Err(Error::CommandExitStatus { name: name.into(), status })
        }
    }
}

/// Adds a `check` method which errors if the command doesn't exit successfully.
pub trait SyncCommandOutputExt {
    /// The type returned by `check` in the success case.
    type Ok;

    /// Errors if the command doesn't exit successfully.
    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Ok>;
}

impl SyncCommandOutputExt for std::process::Command {
    type Ok = std::process::Output;

    fn check(mut self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Ok> {
        (&mut self).check(name)
    }
}

impl<'a> SyncCommandOutputExt for &'a mut std::process::Command {
    type Ok = std::process::Output;

    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Ok> {
        let output = self.output().at_command(name.clone())?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::CommandExit { name: name.into(), output })
        }
    }
}

impl SyncCommandOutputExt for std::process::Child {
    type Ok = std::process::Output;

    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Ok> {
        let output = self.wait_with_output().at_command(name.clone())?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::CommandExit { name: name.into(), output })
        }
    }
}

impl SyncCommandOutputExt for std::process::ExitStatus {
    type Ok = std::process::ExitStatus;

    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Ok> {
        if self.success() {
            Ok(self)
        } else {
            Err(Error::CommandExitStatus { name: name.into(), status: self })
        }
    }
}

#[cfg(feature = "reqwest")]
#[async_trait]
/// Adds a `detailed_error_for_status` method which includes response headers and text in the error.
pub trait ReqwestResponseExt: Sized {
    /// Like `error_for_status` but includes response headers and text in the error.
    async fn detailed_error_for_status(self) -> Result<Self>;

    #[cfg(all(feature = "serde", feature = "serde_json"))]
    /// Like `json` but include response text in the error.
    async fn json_with_text_in_error<T: DeserializeOwned>(self) -> Result<T>;
}

#[cfg(feature = "reqwest")]
#[async_trait]
impl ReqwestResponseExt for reqwest::Response {
    async fn detailed_error_for_status(self) -> Result<Self> {
        match self.error_for_status_ref() {
            Ok(_) => Ok(self),
            Err(inner) => Err(Error::ResponseStatus {
                headers: self.headers().clone(),
                text: self.text().await,
                inner,
            }),
        }
    }

    #[cfg(all(feature = "serde", feature = "serde_json"))]
    async fn json_with_text_in_error<T: DeserializeOwned>(self) -> Result<T> {
        let text = self.text().await?;
        serde_json::from_str(&text).map_err(|inner| Error::ResponseJson { inner, text })
    }
}

/// A heuristic for whether an error is a network error outside of our control that might be fixed by retrying the operation.
pub trait IsNetworkError {
    /// A heuristic for whether an error is a network error outside of our control that might be fixed by retrying the operation.
    fn is_network_error(&self) -> bool;
}

impl IsNetworkError for Error {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Io { inner, .. } => inner.is_network_error(),
            #[cfg(all(feature = "reqwest", feature = "serde", feature = "serde_json"))] Self::Reqwest(e) => e.is_network_error(),
            #[cfg(feature = "reqwest")] Self::ResponseStatus { inner, .. } => inner.is_network_error(),
            _ => false,
        }
    }
}

impl IsNetworkError for io::Error {
    fn is_network_error(&self) -> bool {
        //TODO io::ErrorKind::NetworkUnreachable should also be considered here, as it can occur during a server reboot, but it is currently unstable, making it impossible to match against. See https://github.com/rust-lang/rust/issues/86442
        matches!(self.kind(), io::ErrorKind::ConnectionAborted | io::ErrorKind::ConnectionRefused | io::ErrorKind::ConnectionReset | io::ErrorKind::TimedOut | io::ErrorKind::UnexpectedEof)
    }
}

#[cfg(feature = "async-proto")]
impl IsNetworkError for async_proto::ReadError {
    fn is_network_error(&self) -> bool {
        match self {
            Self::EndOfStream => true,
            Self::Io(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite")] Self::Tungstenite(e) => e.is_network_error(),
            _ => false,
        }
    }
}

#[cfg(feature = "async-proto")]
impl IsNetworkError for async_proto::WriteError {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Io(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite")] Self::Tungstenite(e) => e.is_network_error(),
            _ => false,
        }
    }
}

#[cfg(feature = "reqwest")]
impl IsNetworkError for reqwest::Error {
    fn is_network_error(&self) -> bool {
        self.is_request() || self.is_connect() || self.is_timeout() || self.status().map_or(false, |status| status.is_server_error())
    }
}

#[cfg(feature = "tungstenite")]
impl IsNetworkError for tungstenite::Error {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Http(resp) => resp.status().is_server_error(),
            Self::Io(e) => e.is_network_error(),
            Self::Protocol(tungstenite::error::ProtocolError::ResetWithoutClosingHandshake) => true,
            _ => false,
        }
    }
}
