//! Boilerplate extension traits.

use {
    std::{
        io,
        path::Path,
    },
    async_trait::async_trait,
    crate::{
        Error,
        Result,
    },
};

/// This trait is used by [`IoResultExt`] to convert [`io::Error`] to a generic error type.
pub trait FromIoError {
    /// Constructs a `Self` from the given I/O error and an annotation specifying where in the filesystem the error occurred.
    fn from_io_at(e: io::Error, path: impl AsRef<Path>) -> Self;
    /// Constructs a `Self` from the given I/O error and no annotation.
    fn from_io_at_unknown(e: io::Error) -> Self;
}

/// Allows converting an [`io::Result`] to any [`Result`] type whose [`Err`] variant implements [`FromIoError`], optionally annotating it with the location where the error occurred.
pub trait IoResultExt {
    /// The [`Ok`] variant of the returned [`Result`] type.
    type Ok;

    /// Converts the [`Err`] variant of `self` by annotating it with the given path.
    fn at<E: FromIoError, P: AsRef<Path>>(self, path: P) -> Result<Self::Ok, E>;
    /// Converts the [`Err`] variant of `self` without annotating it with a path.
    fn at_unknown<E: FromIoError>(self) -> Result<Self::Ok, E>;
    /// Converts an [`Err`] with [`io::ErrorKind::AlreadyExists`] to `Ok(default())`.
    fn exist_ok(self) -> Self where Self::Ok: Default;
    /// Converts an [`Err`] with [`io::ErrorKind::NotFound`] to `Ok(default())`.
    fn missing_ok(self) -> Self where Self::Ok: Default;
}

impl<T> IoResultExt for io::Result<T> {
    type Ok = T;

    fn at<E: FromIoError, P: AsRef<Path>>(self, path: P) -> Result<T, E> {
        self.map_err(|e| E::from_io_at(e, path))
    }

    fn at_unknown<E: FromIoError>(self) -> Result<T, E> {
        self.map_err(E::from_io_at_unknown)
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

/// Adds a `check` method which errors if the command doesn't exit successfully.
#[async_trait]
pub trait AsyncCommandOutputExt {
    /// The type retrurned by `check` in the success case.
    type Ok;

    /// Errors if the command doesn't exit successfully.
    async fn check(self, name: &'static str) -> Result<Self::Ok>;
}

#[cfg(feature = "tokio")]
#[async_trait]
impl AsyncCommandOutputExt for tokio::process::Command {
    type Ok = std::process::Output;

    async fn check(mut self, name: &'static str) -> Result<Self::Ok> {
        let output = self.output().await.at_unknown()?; //TODO annotate error with name?
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::CommandExit { name, output })
        }
    }
}

/// Adds a `check` method which errors if the command doesn't exit successfully.
pub trait SyncCommandOutputExt {
    /// The type returned by `check` in the success case.
    type Ok;

    /// Errors if the command doesn't exit successfully.
    fn check(self, name: &'static str) -> Result<Self::Ok>;
}

impl SyncCommandOutputExt for std::process::Command {
    type Ok = std::process::Output;

    fn check(mut self, name: &'static str) -> Result<Self::Ok> {
        (&mut self).check(name)
    }
}

impl<'a> SyncCommandOutputExt for &'a mut std::process::Command {
    type Ok = std::process::Output;

    fn check(self, name: &'static str) -> Result<Self::Ok> {
        let output = self.output().at_unknown()?; //TODO annotate error with name?
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::CommandExit { name, output })
        }
    }
}

impl SyncCommandOutputExt for std::process::Child {
    type Ok = std::process::Output;

    fn check(self, name: &'static str) -> Result<Self::Ok> {
        let output = self.wait_with_output().at_unknown()?; //TODO annotate error with name?
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::CommandExit { name, output })
        }
    }
}
