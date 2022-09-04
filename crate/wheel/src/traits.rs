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
