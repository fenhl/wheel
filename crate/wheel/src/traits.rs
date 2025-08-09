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
#[cfg(feature = "chrono")] use {
    std::fmt,
    chrono::prelude::*,
};
#[cfg(all(feature = "reqwest", feature = "serde_json"))] use serde::de::DeserializeOwned;
#[cfg(all(feature = "chrono", feature = "reqwest", feature = "tokio"))] use {
    std::time::Duration,
    tokio::time::sleep,
};

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

#[cfg(feature = "serde_json")]
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

#[cfg(feature = "serde_json")]
impl<T> IoResultExt for serde_json_path_to_error::Result<T> {
    type Ok = T;

    fn at_unknown(self) -> Result<T> {
        self.map_err(|inner| Error::JsonPathToError { inner, context: IoErrorContext::Unknown })
    }

    fn at(self, path: impl AsRef<Path>) -> Result<T> {
        self.map_err(|inner| Error::JsonPathToError { inner, context: IoErrorContext::Path(path.as_ref().to_owned()) })
    }

    fn at2(self, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<T> {
        self.map_err(|inner| Error::JsonPathToError { inner, context: IoErrorContext::DoublePath(src.as_ref().to_owned(), dst.as_ref().to_owned()) })
    }

    fn at_command(self, name: impl Into<Cow<'static, str>>) -> Result<T> {
        self.map_err(|inner| Error::JsonPathToError { inner, context: IoErrorContext::Command(name.into()) })
    }

    fn exist_ok(self) -> Self where T: Default { self }
    fn missing_ok(self) -> Self where T: Default { self }
}

#[cfg_attr(feature = "tokio", doc = "Extension methods for [`tokio::process::Command`] and [`std::process::Command`]")]
#[cfg_attr(not(feature = "tokio"), doc = "Extension methods for [`std::process::Command`]")]
pub trait CommandExt {
    /// Suppresses creating a console window on Windows. Has no effect on other platforms.
    fn create_no_window(&mut self) -> &mut Self;

    /// Suppresses creating a Windows console window if compiled in release mode. No effect if compiled in debug mode.
    fn release_create_no_window(&mut self) -> &mut Self {
        #[cfg(not(debug_assertions))] self.create_no_window();
        self
    }
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

#[cfg(feature = "tokio")]
#[allow(async_fn_in_trait)]
/// Extension methods for [`tokio::process::Command`]
pub trait AsyncCommandExt {
    /// Runs the command, then exits the current process, forwarding the command's exit status.
    ///
    /// Uses the native `exec` on Unix, and an approximation using `check` on other platforms.
    async fn exec(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Infallible>;
}

#[cfg(feature = "tokio")]
impl AsyncCommandExt for tokio::process::Command {
    async fn exec(mut self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Infallible> {
        (&mut self).exec(name).await
    }
}

#[cfg(feature = "tokio")]
impl<'a> AsyncCommandExt for &'a mut tokio::process::Command {
    async fn exec(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Infallible> {
        #[cfg(unix)] { Err(std::os::unix::process::CommandExt::exec(self.as_std_mut())).at_command(name) }
        #[cfg(not(unix))] {
            let name = name.into();
            match self.check(name.clone()).await {
                Ok(output) => std::process::exit(output.status.code().ok_or(Error::CommandExit { name, output })?),
                Err(e) => Err(e),
            }
        }
    }
}

/// Extension methods for [`std::process::Command`]
pub trait SyncCommandExt {
    /// Runs the command, then exits the current process, forwarding the command's exit status.
    ///
    /// Uses the native `exec` on Unix, and an approximation using `check` on other platforms.
    fn exec(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Infallible>;
}

impl SyncCommandExt for std::process::Command {
    fn exec(mut self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Infallible> {
        (&mut self).exec(name)
    }
}

impl<'a> SyncCommandExt for &'a mut std::process::Command {
    fn exec(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Infallible> {
        #[cfg(unix)] { Err(std::os::unix::process::CommandExt::exec(self)).at_command(name) }
        #[cfg(not(unix))] {
            let name = name.into();
            match self.check(name.clone()) {
                Ok(output) => std::process::exit(output.status.code().ok_or(Error::CommandExit { name, output })?),
                Err(e) => Err(e),
            }
        }
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

impl SyncCommandOutputExt for std::process::Output {
    type Ok = std::process::Output;

    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Ok> {
        if self.status.success() {
            Ok(self)
        } else {
            Err(Error::CommandExit { name: name.into(), output: self })
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

#[cfg(all(feature = "chrono", feature = "reqwest", feature = "tokio"))]
/// Adds a `send_github` method which automatically handles the GitHub REST API's rate limits.
#[async_trait]
pub trait RequestBuilderExt {
    /// Like `send` but automatically handles the GitHub REST API's rate limits.
    async fn send_github(self, verbose: bool) -> Result<reqwest::Response, Error>;
}

#[cfg(all(feature = "chrono", feature = "reqwest", feature = "tokio"))]
#[async_trait]
impl RequestBuilderExt for reqwest::RequestBuilder {
    /// Like `send` but automatically handles the GitHub REST API's rate limits.
    ///
    /// # Errors
    ///
    /// In addition to errors from `send` and errors parsing the rate limiting headers, this method will error if the request has a streaming body.
    async fn send_github(self, verbose: bool) -> Result<reqwest::Response, Error> {
        let mut exponential_backoff = Duration::from_secs(60);
        loop {
            match self.try_clone().ok_or(Error::UncloneableGitHubRequest)?.send().await?.detailed_error_for_status().await {
                Ok(response) => break Ok(response),
                Err(Error::ResponseStatus { inner, headers, text }) if inner.status().is_some_and(|status| matches!(status, reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::TOO_MANY_REQUESTS)) => {
                    if let Some(retry_after) = headers.get(reqwest::header::RETRY_AFTER) {
                        let delta = Duration::from_secs(retry_after.to_str()?.parse()?);
                        if verbose {
                            println!("{} Received retry_after, sleeping for {delta:?}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
                        }
                        sleep(delta).await;
                    } else if headers.get("x-ratelimit-remaining").is_some_and(|x_ratelimit_remaining| x_ratelimit_remaining == "0") {
                        let now = Utc::now();
                        let until = DateTime::from_timestamp(headers.get("x-ratelimit-reset").ok_or(Error::MissingRateLimitResetHeader)?.to_str()?.parse()?, 0).ok_or(Error::InvalidDateTime)?;
                        if let Ok(delta) = (until - now).to_std() {
                            if verbose {
                                println!("{} Received x-ratelimit-remaining, sleeping for {delta:?}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
                            }
                            sleep(delta).await;
                        }
                    } else if exponential_backoff >= Duration::from_secs(60 * 60) {
                        break Err(Error::ResponseStatus { inner, headers, text }.into())
                    } else {
                        if verbose {
                            println!("{} Received unspecific rate limit error, sleeping for {exponential_backoff:?}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
                        }
                        sleep(exponential_backoff).await;
                        exponential_backoff *= 2;
                    }
                }
                Err(e) => break Err(e.into()),
            }
        }
    }
}

#[cfg(feature = "reqwest")]
#[async_trait]
/// Adds a `detailed_error_for_status` method which includes response headers and text in the error.
pub trait ReqwestResponseExt: Sized {
    /// Like `error_for_status` but includes response headers and text in the error.
    async fn detailed_error_for_status(self) -> Result<Self>;

    #[cfg(feature = "serde_json")]
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

    #[cfg(feature = "serde_json")]
    async fn json_with_text_in_error<T: DeserializeOwned>(self) -> Result<T> {
        let text = self.text().await?;
        serde_json_path_to_error::from_str(&text).map_err(|inner| Error::ResponseJsonPathToError { inner, text })
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
            #[cfg(all(feature = "reqwest", feature = "serde_json"))] Self::Reqwest(e) => e.is_network_error(),
            #[cfg(feature = "reqwest")] Self::ResponseStatus { inner, .. } => inner.is_network_error(),
            _ => false,
        }
    }
}

impl IsNetworkError for io::Error {
    fn is_network_error(&self) -> bool {
        matches!(self.kind(),
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::HostUnreachable
            | io::ErrorKind::NetworkUnreachable
            | io::ErrorKind::TimedOut
            | io::ErrorKind::UnexpectedEof
        ) || {
            // Some error sources (e.g. tungstenite) don't provide structured information about I/O errors, so we need to check the Display impl
            let display = self.to_string();
            display == "No such host is known. (os error 11001)"
            || display == "failed to lookup address information: Temporary failure in name resolution"
            || display == "failed to lookup address information: No address associated with hostname"
        }
    }
}

#[cfg(feature = "async-proto")]
impl IsNetworkError for async_proto::ReadError {
    fn is_network_error(&self) -> bool {
        match &self.kind {
            async_proto::ReadErrorKind::EndOfStream => true,
            async_proto::ReadErrorKind::Io(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite021")] async_proto::ReadErrorKind::Tungstenite021(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite024")] async_proto::ReadErrorKind::Tungstenite024(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite027")] async_proto::ReadErrorKind::Tungstenite027(e) => e.is_network_error(),
            _ => false,
        }
    }
}

#[cfg(feature = "async-proto")]
impl IsNetworkError for async_proto::WriteError {
    fn is_network_error(&self) -> bool {
        match &self.kind {
            async_proto::WriteErrorKind::Io(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite021")] async_proto::WriteErrorKind::Tungstenite021(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite024")] async_proto::WriteErrorKind::Tungstenite024(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite027")] async_proto::WriteErrorKind::Tungstenite027(e) => e.is_network_error(),
            _ => false,
        }
    }
}

#[cfg(feature = "racetime")]
impl IsNetworkError for racetime::Error {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Custom(_) => false, // can't dynamically downcast to IsNetworkError
            Self::HeaderToStr(_) => false,
            Self::InvalidHeaderValue(_) => false,
            Self::Io(e) => e.is_network_error(),
            Self::Json(_) => false,
            Self::Task(_) => false,
            Self::UrlParse(_) => false,
            Self::EndOfStream => true,
            Self::LocationCategory => false,
            Self::LocationFormat => false,
            Self::MissingLocationHeader => false,
            Self::Reqwest(e) | Self::ResponseStatus { inner: e, .. } => e.is_network_error(),
            Self::Server(_) => false,
            Self::Tungstenite(e) => e.is_network_error(),
            Self::UnexpectedMessageType(_) => false,
        }
    }
}

#[cfg(feature = "reqwest")]
impl IsNetworkError for reqwest::Error {
    fn is_network_error(&self) -> bool {
        self.is_request() || self.is_connect() || self.is_timeout() || self.status().map_or(false, |status| status.is_server_error())
    }
}

#[cfg(feature = "tungstenite021")]
impl IsNetworkError for tungstenite021::Error {
    fn is_network_error(&self) -> bool {
        match self {
            Self::AlreadyClosed => true, // while the tungstenite docs describe this as a programmer error, it is unavoidable when the WebSocket is handled as a concurrent split sink/stream pair
            Self::Http(resp) => resp.status().is_server_error(),
            Self::Io(e) => e.is_network_error(),
            Self::Protocol(tungstenite021::error::ProtocolError::ResetWithoutClosingHandshake) => true,
            _ => false,
        }
    }
}

#[cfg(feature = "tungstenite024")]
impl IsNetworkError for tungstenite024::Error {
    fn is_network_error(&self) -> bool {
        match self {
            Self::AlreadyClosed => true, // while the tungstenite docs describe this as a programmer error, it is unavoidable when the WebSocket is handled as a concurrent split sink/stream pair
            Self::Http(resp) => resp.status().is_server_error(),
            Self::Io(e) => e.is_network_error(),
            Self::Protocol(tungstenite024::error::ProtocolError::ResetWithoutClosingHandshake) => true,
            _ => false,
        }
    }
}

#[cfg(feature = "tungstenite027")]
impl IsNetworkError for tungstenite027::Error {
    fn is_network_error(&self) -> bool {
        match self {
            Self::AlreadyClosed => true, // while the tungstenite docs describe this as a programmer error, it is unavoidable when the WebSocket is handled as a concurrent split sink/stream pair
            Self::Http(resp) => resp.status().is_server_error(),
            Self::Io(e) => e.is_network_error(),
            Self::Protocol(tungstenite027::error::ProtocolError::ResetWithoutClosingHandshake) => true,
            _ => false,
        }
    }
}

#[cfg(feature = "chrono")]
/// Error type returned by [`LocalResultExt::single_ok`].
#[derive(Debug, Clone, Copy)]
pub enum TimeFromLocalError<T> {
    /// Given local time representation is invalid. This may be caused by a positive timezone transition.
    None,
    /// Given local time representation has multiple results and thus ambiguous. This may be caused by a negative timezone transition.
    Ambiguous([T; 2]),
}

#[cfg(feature = "chrono")]
impl<Z: TimeZone> fmt::Display for TimeFromLocalError<DateTime<Z>>
where Z::Offset: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "invalid timestamp"),
            Self::Ambiguous([first, second]) => write!(
                f,
                "ambiguous timestamp: could refer to {} or {} UTC",
                first.with_timezone(&Utc).format("%Y-%m-%d %H:%M:%S"),
                second.with_timezone(&Utc).format("%Y-%m-%d %H:%M:%S"),
            ),
        }
    }
}

#[cfg(feature = "chrono")]
impl<T: fmt::Debug> std::error::Error for TimeFromLocalError<T> where TimeFromLocalError<T>: fmt::Display {}

#[cfg(feature = "chrono")]
/// Allows converting a [`chrono::LocalResult<T>`] to a [`Result<T, TimeFromLocalError<T>>`].
pub trait LocalResultExt {
    /// The [`Ok`] variant of the returned [`Result`] type. Returned when the given local time representation has a single unique result.
    type Ok;

    /// Converts a [`chrono::LocalResult<T>`] to a [`Result<T, TimeFromLocalError<T>>`].
    fn single_ok(self) -> Result<Self::Ok, TimeFromLocalError<Self::Ok>>;
}

#[cfg(feature = "chrono")]
impl<T> LocalResultExt for chrono::LocalResult<T> {
    type Ok = T;

    fn single_ok(self) -> Result<T, TimeFromLocalError<T>> {
        match self {
            Self::None => Err(TimeFromLocalError::None),
            Self::Single(value) => Ok(value),
            Self::Ambiguous(value1, value2) => Err(TimeFromLocalError::Ambiguous([value1, value2])),
        }
    }
}

#[cfg(feature = "tokio")]
/// A more explicit way to ignore when a message is dropped due to a lack of listeners.
pub trait SendResultExt {
    /// The return type of `allow_unreceived`.
    type Ok;

    /// A more explicit way to ignore when a message is dropped due to a lack of listeners.
    fn allow_unreceived(self) -> Self::Ok;
}

#[cfg(feature = "tokio")]
impl<T> SendResultExt for Result<usize, tokio::sync::broadcast::error::SendError<T>> {
    type Ok = usize;

    fn allow_unreceived(self) -> usize {
        match self {
            Ok(n) => n,
            Err(tokio::sync::broadcast::error::SendError(_)) => 0
        }
    }
}

#[cfg(feature = "tokio")]
impl<T> SendResultExt for Result<(), tokio::sync::mpsc::error::SendError<T>> {
    type Ok = ();

    fn allow_unreceived(self) {
        match self {
            Ok(()) => {}
            Err(tokio::sync::mpsc::error::SendError(_)) => {}
        }
    }
}
