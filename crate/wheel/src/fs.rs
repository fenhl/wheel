//! A wrapper around `tokio::fs` with error types that include relevant paths.

use {
    std::{
        io::{
            self,
            IoSlice,
        },
        path::{
            Path,
            PathBuf,
        },
        pin::Pin,
        task::{
            Context,
            Poll,
        },
    },
    tokio::{
        fs::OpenOptions,
        io::{
            AsyncRead,
            AsyncSeek,
            AsyncWrite,
        },
    },
    crate::{
        Result,
        traits::IoResultExt as _,
    },
};

/// A wrapper around [`tokio::fs::File`].
#[derive(Debug)]
pub struct File {
    path: PathBuf,
    inner: tokio::fs::File,
}

impl File {
    /// A wrapper around [`tokio::fs::File::open`].
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        Ok(Self {
            inner: tokio::fs::File::open(path).await.at(path)?,
            path: path.to_owned(),
        })
    }

    /// A wrapper around [`tokio::fs::File::create`].
    pub async fn create(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        Ok(Self {
            inner: tokio::fs::File::create(path).await.at(path)?,
            path: path.to_owned(),
        })
    }

    /// A wrapper around [`tokio::fs::OpenOptions::open`].
    pub async fn from_options(options: &OpenOptions, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        Ok(Self {
            inner: options.open(path).await.at(path)?,
            path: path.to_owned(),
        })
    }

    /// A wrapper around [`tokio::fs::File::sync_all`].
    pub async fn sync_all(&self) -> Result {
        self.inner.sync_all().await.at(&self.path)
    }
}

impl AsyncRead for File {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut tokio::io::ReadBuf<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf) //TODO include path in error?
    }
}

impl AsyncSeek for File {
    fn start_seek(mut self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        Pin::new(&mut self.inner).start_seek(position) //TODO include path in error?
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Pin::new(&mut self.inner).poll_complete(cx) //TODO include path in error?
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

/// A wrapper around [`tokio::fs::create_dir`].
pub async fn create_dir(path: impl AsRef<Path>) -> Result {
    let path = path.as_ref();
    tokio::fs::create_dir(path).await.at(path)
}

/// A wrapper around [`tokio::fs::create_dir_all`].
pub async fn create_dir_all(path: impl AsRef<Path>) -> Result {
    let path = path.as_ref();
    tokio::fs::create_dir_all(path).await.at(path)
}

/// A wrapper around [`tokio::fs::read`].
pub async fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = path.as_ref();
    tokio::fs::read(path).await.at(path)
}

/// A wrapper around [`tokio::fs::read_to_string`].
pub async fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    tokio::fs::read_to_string(path).await.at(path)
}

/// A wrapper around [`tokio::fs::write`].
pub async fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result {
    let path = path.as_ref();
    tokio::fs::write(path, contents).await.at(path)
}
