//! A wrapper around `tokio::fs` with error types that include relevant paths.

use {
    std::{
        io::{
            self,
            IoSlice,
        },
        path::Path,
        pin::Pin,
        task::{
            Context,
            Poll,
        },
    },
    tokio::io::AsyncWrite,
    crate::{
        Result,
        traits::IoResultExt as _,
    },
};

/// A wrapper around [`tokio::fs::File`].
pub struct File {
    //path: PathBuf,
    inner: tokio::fs::File,
}

impl File {
    /// A wrapper around [`tokio::fs::File::create`].
    pub async fn create(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        Ok(Self {
            inner: tokio::fs::File::create(path).await.at(path)?,
            //path,
        })
    }
}

impl AsyncWrite for File {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf) //TODO include path in error?
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx) //TODO include path in error?
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx) //TODO include path in error?
    }

    fn poll_write_vectored(mut self: Pin<&mut Self>, cx: &mut Context<'_>, bufs: &[IoSlice<'_>]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write_vectored(cx, bufs) //TODO include path in error?
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}

/// A wrapper around [`tokio::fs::create_dir_all`].
pub async fn create_dir_all(path: impl AsRef<Path>) -> Result {
    let path = path.as_ref();
    tokio::fs::create_dir_all(path).await.at(path)
}
