//! Boilerplate extension traits.

use std::{
    convert::Infallible,
    io,
};
#[cfg(windows)] use std::os::windows::process::CommandExt as _;
#[cfg(feature = "reqwest")] use {
    std::path::Path,
    futures::stream::TryStreamExt as _,
    tokio::io::AsyncWrite,
    tokio_util::io::StreamReader,
    crate::io::Annotate as _,
};
#[cfg(all(feature = "reqwest", feature = "serde_json"))] use serde::de::DeserializeOwned;
pub use wheel_derive::IsNetworkError;

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

#[cfg(feature = "reqwest")]
#[derive(Debug, thiserror::Error)]
#[error("HTTP error{}: {inner}, body:\n\n{}", if let Some(url) = .inner.url() { format!(" at {url}") } else { String::default() }, .text.as_ref().map(|text| text.clone()).unwrap_or_else(|e| e.to_string()))]
pub struct ResponseStatusError {
    #[source]
    pub inner: reqwest::Error,
    pub headers: reqwest::header::HeaderMap,
    pub text: reqwest::Result<String>,
}

#[cfg(feature = "reqwest")]
impl IsNetworkError for ResponseStatusError {
    fn is_network_error(&self) -> bool {
        self.inner.is_network_error()
    }
}

#[cfg(all(feature = "reqwest", feature = "serde_json"))]
#[derive(Debug, thiserror::Error)]
pub enum ResponseJsonError {
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error("{inner}, body:\n\n{text}")]
    Json {
        #[source]
        inner: serde_json::Error,
        text: String,
    },
}

#[cfg(all(feature = "reqwest", feature = "serde_json"))]
impl IsNetworkError for ResponseJsonError {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Reqwest(e) => e.is_network_error(),
            Self::Json { .. } => false,
        }
    }
}

#[cfg(feature = "reqwest")]
/// Adds a `detailed_error_for_status` method which includes response headers and text in the error.
pub trait ReqwestResponseExt: Sized {
    /// Like `error_for_status` but includes response headers and text in the error.
    async fn detailed_error_for_status(self) -> Result<Self, ResponseStatusError>;

    #[cfg(feature = "serde_json")]
    /// Like `json` but include response text in the error.
    async fn json_with_text_in_error<T: DeserializeOwned>(self) -> Result<T, ResponseJsonError>;

    /// Writes the response body to the given writer.
    ///
    /// Returns the size of the response body in bytes.
    fn write_to<'a, W: AsyncWrite + Unpin + 'a>(self, writer: W) -> impl Future<Output = io::Result<u64>> + use<'a, Self, W>;

    /// Downloads the response body to the file system.
    ///
    /// Returns the size of the downloaded file in bytes.
    fn download<P: AsRef<Path>>(self, path: P) -> impl Future<Output = crate::io::Result<u64>> + use<Self, P> {
        let path = path.as_ref().to_owned();
        async move {
            self.write_to(crate::fs::File::create(&path).await?).await.at(path)
        }
    }

    /// Like [`Self::download`] but return [`io::ErrorKind::AlreadyExists`] if the file exists.
    fn download_new<P: AsRef<Path>>(self, path: P) -> impl Future<Output = crate::io::Result<u64>> + use<Self, P> {
        let path = path.as_ref().to_owned();
        async move {
            self.write_to(crate::fs::File::create_new(&path).await?).await.at(path)
        }
    }
}

#[cfg(feature = "reqwest")]
impl ReqwestResponseExt for reqwest::Response {
    async fn detailed_error_for_status(self) -> Result<Self, ResponseStatusError> {
        match self.error_for_status_ref() {
            Ok(_) => Ok(self),
            Err(inner) => Err(ResponseStatusError {
                headers: self.headers().clone(),
                text: self.text().await,
                inner,
            }),
        }
    }

    #[cfg(feature = "serde_json")]
    async fn json_with_text_in_error<T: DeserializeOwned>(self) -> Result<T, ResponseJsonError> {
        let url = self.url().clone();
        let text = self.text().await.map_err(|e| if e.url().is_some() { e } else { e.with_url(url) })?;
        serde_json::from_str(&text).map_err(|inner| ResponseJsonError::Json { inner, text })
    }

    fn write_to<'a, W: AsyncWrite + Unpin + 'a>(self, mut writer: W) -> impl Future<Output = io::Result<u64>> + use<'a, W> {
        async move {
            tokio::io::copy_buf(
                &mut StreamReader::new(self.bytes_stream().map_err(crate::io_error_from_reqwest)),
                &mut writer,
            ).await
        }
    }
}

/// A heuristic for whether an error is a network error outside of our control that might be fixed by retrying the operation.
pub trait IsNetworkError {
    /// A heuristic for whether an error is a network error outside of our control that might be fixed by retrying the operation.
    fn is_network_error(&self) -> bool;
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
        ) || matches!(&*self.to_string(),
            // Some error sources (e.g. tungstenite) don't provide structured information about I/O errors, so we need to check the Display impl
            | "No such host is known. (os error 11001)"
            | "failed to lookup address information: Temporary failure in name resolution"
            | "failed to lookup address information: No address associated with hostname"
            | "failed to lookup address information: nodename nor servname provided, or not known"
        )
    }
}

#[cfg(feature = "async-proto")]
impl IsNetworkError for async_proto::ReadError {
    fn is_network_error(&self) -> bool {
        match &self.kind {
            async_proto::ReadErrorKind::EndOfStream => true,
            async_proto::ReadErrorKind::Io(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite021")] async_proto::ReadErrorKind::MessageKind021(msg) => msg.is_close(),
            #[cfg(feature = "tungstenite024")] async_proto::ReadErrorKind::MessageKind024(msg) => msg.is_close(),
            #[cfg(feature = "tungstenite030")] async_proto::ReadErrorKind::MessageKind030(msg) => msg.is_close(),
            #[cfg(feature = "tungstenite021")] async_proto::ReadErrorKind::Tungstenite021(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite024")] async_proto::ReadErrorKind::Tungstenite024(e) => e.is_network_error(),
            #[cfg(feature = "tungstenite030")] async_proto::ReadErrorKind::Tungstenite030(e) => e.is_network_error(),
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
            #[cfg(feature = "tungstenite030")] async_proto::WriteErrorKind::Tungstenite030(e) => e.is_network_error(),
            _ => false,
        }
    }
}

#[cfg(feature = "racetime")]
impl IsNetworkError for racetime::AuthError {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Http(e) => e.is_network_error(),
            Self::Url(_) => false,
            Self::ResponseStatus { inner, .. } => inner.is_network_error(),
        }
    }
}

#[cfg(feature = "racetime")]
impl IsNetworkError for racetime::EditError {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Http(e) => e.is_network_error(),
            Self::Url(_) => false,
            Self::ResponseStatus { inner, .. } => inner.is_network_error(),
        }
    }
}

#[cfg(feature = "racetime")]
impl IsNetworkError for racetime::handler::SendError {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Elapsed(_) => true,
            Self::Serialize(_) => false,
            Self::WebSocket(e) => e.is_network_error(),
        }
    }
}

#[cfg(feature = "racetime")]
impl IsNetworkError for racetime::StartError {
    fn is_network_error(&self) -> bool {
        match self {
            Self::HeaderToStr(_) => false,
            Self::Http(e) => e.is_network_error(),
            Self::Url(_) => false,
            Self::LocationCategory => false,
            Self::LocationFormat => false,
            Self::MissingLocationHeader => false,
            Self::ResponseStatus { inner, .. } => inner.is_network_error(),
        }
    }
}

#[cfg(feature = "reqwest")]
impl IsNetworkError for reqwest::Error {
    fn is_network_error(&self) -> bool {
        self.is_request() || self.is_connect() || self.is_timeout() || self.status().map_or(false, |status| status.is_server_error()) || (self as &dyn std::error::Error).downcast_ref::<io::Error>().is_some_and(|e| e.is_network_error())
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

#[cfg(feature = "tungstenite030")]
impl IsNetworkError for tungstenite030::Error {
    fn is_network_error(&self) -> bool {
        match self {
            Self::AlreadyClosed => true, // while the tungstenite docs describe this as a programmer error, it is unavoidable when the WebSocket is handled as a concurrent split sink/stream pair
            Self::Http(resp) => resp.status().is_server_error(),
            Self::Io(e) => e.is_network_error(),
            Self::Protocol(tungstenite030::error::ProtocolError::ResetWithoutClosingHandshake) => true,
            _ => false,
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
