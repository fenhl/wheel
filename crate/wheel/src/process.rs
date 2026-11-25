#![cfg_attr(not(feature = "tokio"), doc = "Extension traits for [`std::process`].")]
#![cfg_attr(feature = "tokio", doc = "Extension traits for [`std::io`] and [`tokio::process`].")]

use {
    std::{
        borrow::Cow,
        convert::Infallible,
        process::{
            ExitStatus,
            Output,
        },
    },
    crate::{
        io::Annotate as _,
        traits::IsNetworkError,
    },
};

pub trait AsExitStatus {
    fn exit_status(&self) -> ExitStatus;
}

impl AsExitStatus for ExitStatus {
    fn exit_status(&self) -> ExitStatus {
        *self
    }
}

impl AsExitStatus for Output {
    fn exit_status(&self) -> ExitStatus {
        self.status
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error<O: AsExitStatus = Output> {
    #[error(transparent)] Io(#[from] crate::io::Error),
    /// A subprocess exited with a non-success status. Output information is available.
    #[error("command `{name}` exited with {}", .output.exit_status())]
    Exit {
        /// The name of the subprocess, as indicated by the `check` call.
        name: Cow<'static, str>,
        output: O,
    },
}

impl<O: AsExitStatus> IsNetworkError for Error<O> {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Io(e) => e.is_network_error(),
            Self::Exit { .. } => false,
        }
    }
}

pub type Result<O = Output> = std::result::Result<O, Error<O>>;

/// Adds a `check` method which errors if the command doesn't exit successfully.
pub trait AsyncCommandOutputExt {
    /// The type returned by `check` in the success case.
    type Output: AsExitStatus;

    /// Errors if the command doesn't exit successfully.
    async fn check(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Output>;
}

#[cfg(feature = "tokio")]
impl AsyncCommandOutputExt for tokio::process::Command {
    type Output = Output;

    async fn check(mut self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Output> {
        (&mut self).check(name).await
    }
}

#[cfg(feature = "tokio")]
impl<'a> AsyncCommandOutputExt for &'a mut tokio::process::Command {
    type Output = Output;

    async fn check(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Output> {
        let output = self.output().await.at_command(name.clone())?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::Exit { name: name.into(), output })
        }
    }
}

#[cfg(feature = "tokio")]
impl AsyncCommandOutputExt for tokio::process::Child {
    type Output = Output;

    async fn check(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Output> {
        let output = self.wait_with_output().await.at_command(name.clone())?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::Exit { name: name.into(), output })
        }
    }
}

#[cfg(feature = "tokio")]
impl<'a> AsyncCommandOutputExt for &'a mut tokio::process::Child {
    type Output = ExitStatus;

    async fn check(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> Result<Self::Output> {
        let status = self.wait().await.at_command(name.clone())?;
        if status.success() {
            Ok(status)
        } else {
            Err(Error::Exit { name: name.into(), output: status })
        }
    }
}

/// Adds a `check` method which errors if the command doesn't exit successfully.
pub trait SyncCommandOutputExt {
    /// The type returned by `check` in the success case.
    type Output: AsExitStatus;

    /// Errors if the command doesn't exit successfully.
    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Output>;
}

impl SyncCommandOutputExt for std::process::Command {
    type Output = Output;

    fn check(mut self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Output> {
        (&mut self).check(name)
    }
}

impl<'a> SyncCommandOutputExt for &'a mut std::process::Command {
    type Output = Output;

    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Output> {
        let output = self.output().at_command(name.clone())?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::Exit { name: name.into(), output })
        }
    }
}

impl SyncCommandOutputExt for std::process::Child {
    type Output = Output;

    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Output> {
        let output = self.wait_with_output().at_command(name.clone())?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::Exit { name: name.into(), output })
        }
    }
}

impl<'a> SyncCommandOutputExt for &'a mut std::process::Child {
    type Output = ExitStatus;

    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Output> {
        let status = self.wait().at_command(name.clone())?;
        if status.success() {
            Ok(status)
        } else {
            Err(Error::Exit { name: name.into(), output: status })
        }
    }
}

impl SyncCommandOutputExt for Output {
    type Output = Self;

    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Output> {
        if self.status.success() {
            Ok(self)
        } else {
            Err(Error::Exit { name: name.into(), output: self })
        }
    }
}

impl SyncCommandOutputExt for ExitStatus {
    type Output = Self;

    fn check(self, name: impl Into<Cow<'static, str>> + Clone) -> Result<Self::Output> {
        if self.success() {
            Ok(self)
        } else {
            Err(Error::Exit { name: name.into(), output: self })
        }
    }
}

#[cfg(feature = "tokio")]
/// Extension methods for [`tokio::process::Command`]
pub trait AsyncCommandExt {
    /// Runs the command, then exits the current process, forwarding the command's exit status.
    ///
    /// Uses the native `exec` on Unix, and an approximation using `check` on other platforms.
    async fn exec(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> std::result::Result<Infallible, Error>;
}

#[cfg(feature = "tokio")]
impl AsyncCommandExt for tokio::process::Command {
    async fn exec(mut self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> std::result::Result<Infallible, Error> {
        (&mut self).exec(name).await
    }
}

#[cfg(feature = "tokio")]
impl<'a> AsyncCommandExt for &'a mut tokio::process::Command {
    async fn exec(self, name: impl Into<Cow<'static, str>> + Clone + Send + 'static) -> std::result::Result<Infallible, Error> {
        #[cfg(unix)] { Err(std::os::unix::process::CommandExt::exec(self.as_std_mut())).at_command(name).map_err(Error::Io) }
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
    fn exec(self, name: impl Into<Cow<'static, str>> + Clone) -> std::result::Result<Infallible, Error>;
}

impl SyncCommandExt for std::process::Command {
    fn exec(mut self, name: impl Into<Cow<'static, str>> + Clone) -> std::result::Result<Infallible, Error> {
        (&mut self).exec(name)
    }
}

impl<'a> SyncCommandExt for &'a mut std::process::Command {
    fn exec(self, name: impl Into<Cow<'static, str>> + Clone) -> std::result::Result<Infallible, Error> {
        #[cfg(unix)] { Err(std::os::unix::process::CommandExt::exec(self)).at_command(name).map_err(Error::Io) }
        #[cfg(not(unix))] {
            let name = name.into();
            match self.check(name.clone()) {
                Ok(output) => std::process::exit(output.status.code().ok_or(Error::CommandExit { name, output })?),
                Err(e) => Err(e),
            }
        }
    }
}
