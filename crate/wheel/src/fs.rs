//! A wrapper around `tokio::fs` with error types that include relevant paths.

use {
    std::{
        io::{
            self,
            IoSlice,
        },
        ops::{
            Deref,
            DerefMut,
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
    futures::stream::{
        self,
        Stream,
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
pub use {
    std::fs::{
        Metadata,
        Permissions,
    },
    tokio::fs::DirEntry,
};
#[cfg(all(feature = "serde", feature = "serde_json"))] use serde::Deserialize;

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

    /// Returns the underlying [`tokio::fs::File`].
    pub fn into_inner(self) -> tokio::fs::File {
        self.inner
    }

    /// A wrapper around [`tokio::fs::File::into_std`].
    pub async fn into_std(self) -> std::fs::File {
        self.inner.into_std().await
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

impl Deref for File {
    type Target = tokio::fs::File;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for File {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// A wrapper around [`tokio::fs::canonicalize`].
pub async fn canonicalize(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    tokio::fs::canonicalize(path).await.at(path)
}

/// A wrapper around [`tokio::fs::copy`].
pub async fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    tokio::fs::copy(from, to).await.at2(from, to)
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

/// A wrapper around [`tokio::fs::try_exists`].
pub async fn exists(path: impl AsRef<Path>) -> Result<bool> {
    let path = path.as_ref();
    tokio::fs::try_exists(path).await.at(path)
}

/// A wrapper around [`tokio::fs::metadata`].
pub async fn metadata(path: impl AsRef<Path>) -> Result<Metadata> {
    let path = path.as_ref();
    tokio::fs::metadata(path).await.at(path)
}

/// A wrapper around [`tokio::fs::read`].
pub async fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = path.as_ref();
    tokio::fs::read(path).await.at(path)
}

#[cfg(feature = "futures")]
/// A wrapper around [`tokio::fs::read_dir`].
pub fn read_dir(path: impl AsRef<Path>) -> impl Stream<Item = Result<DirEntry>> + Send {
    enum State {
        Init(PathBuf),
        Continued(PathBuf, tokio::fs::ReadDir),
    }

    stream::try_unfold(State::Init(path.as_ref().to_owned()), |state| async move {
        Ok(match state {
            State::Init(path) => {
                let mut read_dir = tokio::fs::read_dir(&path).await.at(&path)?;
                read_dir.next_entry().await.at(&path)?.map(|entry| (entry, State::Continued(path, read_dir)))
            }
            State::Continued(path, mut read_dir) => read_dir.next_entry().await.at(&path)?.map(|entry| (entry, State::Continued(path, read_dir))),
        })
    })
}

#[cfg(all(feature = "serde", feature = "serde_json"))]
/// A convenience method for reading and deserializing a JSON file. Loads the contents of the file into memory during deserializaton.
pub async fn read_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let buf = tokio::fs::read(path).await.at(path)?;
    serde_json::from_slice(&buf).at(path)
}

/// A wrapper around [`tokio::fs::read_link`].
pub async fn read_link(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    tokio::fs::read_link(path).await.at(path)
}

/// A wrapper around [`tokio::fs::read_to_string`].
pub async fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    tokio::fs::read_to_string(path).await.at(path)
}

/// A wrapper around [`tokio::fs::remove_dir`].
pub async fn remove_dir(path: impl AsRef<Path>) -> Result {
    let path = path.as_ref();
    tokio::fs::remove_dir(path).await.at(path)
}

/// A wrapper around [`tokio::fs::remove_dir_all`].
pub async fn remove_dir_all(path: impl AsRef<Path>) -> Result {
    let path = path.as_ref();
    tokio::fs::remove_dir_all(path).await.at(path)
}

/// A wrapper around [`tokio::fs::remove_file`].
pub async fn remove_file(path: impl AsRef<Path>) -> Result {
    let path = path.as_ref();
    tokio::fs::remove_file(path).await.at(path)
}

/// A wrapper around [`tokio::fs::rename`].
pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result {
    let from = from.as_ref();
    let to = to.as_ref();
    tokio::fs::rename(from, to).await.at2(from, to)
}

/// A wrapper around [`tokio::fs::set_permissions`].
pub async fn set_permissions(path: impl AsRef<Path>, perm: Permissions) -> Result {
    let path = path.as_ref();
    tokio::fs::set_permissions(path, perm).await.at(path)
}

#[cfg(unix)]
/// A wrapper around [`tokio::fs::symlink`].
pub async fn symlink(original: impl AsRef<Path>, link: impl AsRef<Path>) -> Result {
    let original = original.as_ref();
    let link = link.as_ref();
    tokio::fs::symlink(original, link).await.at2(original, link)
}

#[cfg(windows)]
/// A wrapper around [`tokio::fs::symlink_dir`].
pub async fn symlink_dir(original: impl AsRef<Path>, link: impl AsRef<Path>) -> Result {
    let original = original.as_ref();
    let link = link.as_ref();
    tokio::fs::symlink_dir(original, link).await.at2(original, link)
}

#[cfg(windows)]
/// A wrapper around [`tokio::fs::symlink_file`].
pub async fn symlink_file(original: impl AsRef<Path>, link: impl AsRef<Path>) -> Result {
    let original = original.as_ref();
    let link = link.as_ref();
    tokio::fs::symlink_file(original, link).await.at2(original, link)
}

/// A wrapper around [`tokio::fs::symlink_metadata`].
pub async fn symlink_metadata(path: impl AsRef<Path>) -> Result<Metadata> {
    let path = path.as_ref();
    tokio::fs::symlink_metadata(path).await.at(path)
}

/// A wrapper around [`tokio::fs::write`].
pub async fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result {
    let path = path.as_ref();
    tokio::fs::write(path, contents).await.at(path)
}
