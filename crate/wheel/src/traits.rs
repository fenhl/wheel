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
        Result,
    },
};
#[cfg(windows)] use std::os::windows::process::CommandExt as _;

/// A convenience method for working with infallible results
pub trait ResultNeverExt<T> {
    /// Returns the `Ok` variant of this result.
    fn never_unwrap(self) -> T;
}

impl<T> ResultNeverExt<T> for Result<T, Infallible> {
    fn never_unwrap(self) -> T {
        match self {
            Ok(inner) => inner,
            Err(never) => match never {},
        }
    }
}

/// This trait is used by [`IoResultExt`] to convert [`io::Error`] to a generic error type.
pub trait FromIoError {
    /// Constructs a `Self` from the given I/O error and no annotation.
    fn from_io_at_unknown(e: io::Error) -> Self;
    /// Constructs a `Self` from the given I/O error and an annotation specifying where in the filesystem the error occurred.
    fn from_io_at(e: io::Error, path: impl AsRef<Path>) -> Self;
    /// Constructs a `Self` from the given I/O error and an annotation specifying that the error occurred when trying to execute the named command.
    fn from_io_at_command(e: io::Error, name: impl Into<Cow<'static, str>>) -> Self;
}

/// Allows converting an [`io::Result`] to any [`Result`] type whose [`Err`] variant implements [`FromIoError`], optionally annotating it with the location where the error occurred.
pub trait IoResultExt {
    /// The [`Ok`] variant of the returned [`Result`] type.
    type Ok;

    /// Converts the [`Err`] variant of `self` without annotating it with a path or command context.
    fn at_unknown<E: FromIoError>(self) -> Result<Self::Ok, E>;
    /// Converts the [`Err`] variant of `self` by annotating it with the given path.
    fn at<E: FromIoError, P: AsRef<Path>>(self, path: P) -> Result<Self::Ok, E>;
    /// Converts the [`Err`] variant of `self` by annotating it with the given command name.
    fn at_command<E: FromIoError, S: Into<Cow<'static, str>>>(self, name: S) -> Result<Self::Ok, E>;
    /// Converts an [`Err`] with [`io::ErrorKind::AlreadyExists`] to `Ok(default())`.
    fn exist_ok(self) -> Self where Self::Ok: Default;
    /// Converts an [`Err`] with [`io::ErrorKind::NotFound`] to `Ok(default())`.
    fn missing_ok(self) -> Self where Self::Ok: Default;
}

impl<T> IoResultExt for io::Result<T> {
    type Ok = T;

    fn at_unknown<E: FromIoError>(self) -> Result<T, E> {
        self.map_err(E::from_io_at_unknown)
    }

    fn at<E: FromIoError, P: AsRef<Path>>(self, path: P) -> Result<T, E> {
        self.map_err(|e| E::from_io_at(e, path))
    }

    fn at_command<E: FromIoError, S: Into<Cow<'static, str>>>(self, name: S) -> Result<T, E> {
        self.map_err(|e| E::from_io_at_command(e, name))
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
