//! A wrapper around `tokio::fs` with error types that include relevant paths.

use {
    std::{
        io::IoSlice,
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
    tokio::{
        fs::OpenOptions,
        io::{
            AsyncRead,
            AsyncSeek,
            AsyncWrite,
            AsyncWriteExt as _,
        },
    },
    crate::io::{
        self,
        Annotate as _,
    },
};
pub use {
    std::fs::{
        Metadata,
        Permissions,
    },
    tokio::fs::DirEntry,
};
#[cfg(feature = "futures")] use futures::stream::{
    self,
    Stream,
};
#[cfg(feature = "rocket")] use rocket::{
    request::Request,
    response::Responder,
};
#[cfg(feature = "serde_json")] use {
    serde::{
        Deserialize,
        Serialize,
    },
    crate::traits::IsNetworkError,
};

/// A wrapper around [`tokio::fs::File`].
#[derive(Debug)]
pub struct File {
    path: PathBuf,
    inner: tokio::fs::File,
}

impl File {
    /// A wrapper around [`tokio::fs::File::open`].
    pub async fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        Ok(Self {
            inner: tokio::fs::File::open(path).await.at(path)?,
            path: path.to_owned(),
        })
    }

    /// A wrapper around [`tokio::fs::File::create`].
    pub async fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        Ok(Self {
            inner: tokio::fs::File::create(path).await.at(path)?,
            path: path.to_owned(),
        })
    }

    /// A wrapper around [`tokio::fs::File::create_new`].
    pub async fn create_new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        Ok(Self {
            inner: tokio::fs::File::create_new(path).await.at(path)?,
            path: path.to_owned(),
        })
    }

    /// A wrapper around [`tokio::fs::OpenOptions::open`].
    pub async fn from_options(options: &OpenOptions, path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        Ok(Self {
            inner: options.open(path).await.at(path)?,
            path: path.to_owned(),
        })
    }

    /// A wrapper around [`tokio::fs::metadata`].
    pub async fn metadata(&self) -> io::Result<Metadata> {
        self.inner.metadata().await.at(&self.path)
    }

    /// A wrapper around [`tokio::fs::File::sync_all`].
    pub async fn sync_all(&self) -> io::Result {
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
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut tokio::io::ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf) //TODO include path in error?
    }
}

impl AsyncSeek for File {
    fn start_seek(mut self: Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
        Pin::new(&mut self.inner).start_seek(position) //TODO include path in error?
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        Pin::new(&mut self.inner).poll_complete(cx) //TODO include path in error?
    }
}

impl AsyncWrite for File {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf) //TODO include path in error?
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx) //TODO include path in error?
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx) //TODO include path in error?
    }

    fn poll_write_vectored(mut self: Pin<&mut Self>, cx: &mut Context<'_>, bufs: &[IoSlice<'_>]) -> Poll<std::io::Result<usize>> {
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

#[cfg(feature = "rocket")]
impl<'r> Responder<'r, 'static> for File {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        self.inner.respond_to(request)
    }
}

/// A wrapper around [`tokio::fs::canonicalize`].
pub async fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref();
    tokio::fs::canonicalize(path).await.at(path)
}

/// A wrapper around [`tokio::fs::copy`].
pub async fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    tokio::fs::copy(from, to).await.at2(from, to)
}

/// Like [`copy`] but return [`std::io::ErrorKind::AlreadyExists`] if `to` exists.
pub async fn copy_new(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    tokio::io::copy(&mut File::open(from).await?, &mut File::create_new(to).await?).await.at2(from, to)
}

/// A wrapper around [`tokio::fs::create_dir`].
pub async fn create_dir(path: impl AsRef<Path>) -> io::Result {
    let path = path.as_ref();
    tokio::fs::create_dir(path).await.at(path)
}

/// A wrapper around [`tokio::fs::create_dir_all`].
pub async fn create_dir_all(path: impl AsRef<Path>) -> io::Result {
    let path = path.as_ref();
    tokio::fs::create_dir_all(path).await.at(path)
}

/// A wrapper around [`tokio::fs::try_exists`].
pub async fn exists(path: impl AsRef<Path>) -> io::Result<bool> {
    let path = path.as_ref();
    tokio::fs::try_exists(path).await.at(path)
}

/// A wrapper around [`tokio::fs::metadata`].
pub async fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let path = path.as_ref();
    tokio::fs::metadata(path).await.at(path)
}

/// A wrapper around [`tokio::fs::read`].
pub async fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let path = path.as_ref();
    tokio::fs::read(path).await.at(path)
}

#[cfg(feature = "futures")]
/// A wrapper around [`tokio::fs::read_dir`].
pub fn read_dir(path: impl AsRef<Path>) -> impl Stream<Item = io::Result<DirEntry>> + Send {
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

#[cfg(feature = "serde_json")]
#[derive(Debug, thiserror::Error)]
pub enum JsonError {
    #[error(transparent)] Io(#[from] io::Error),
    #[error("JSON error at {}: {source}", .path.display())]
    Json {
        source: serde_json::Error,
        path: PathBuf,
    },
}

#[cfg(feature = "serde_json")]
impl IsNetworkError for JsonError {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Io(e) => e.is_network_error(),
            Self::Json { .. } => false,
        }
    }
}

#[cfg(feature = "serde_json")]
/// A convenience method for reading and deserializing a JSON file. Loads the contents of the file into memory during deserializaton.
pub async fn read_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, JsonError> {
    let path = path.as_ref();
    let buf = tokio::fs::read(path).await.at(path)?;
    serde_json::from_slice(&buf).map_err(|source| JsonError::Json { source, path: path.to_owned() })
}

/// A wrapper around [`tokio::fs::read_link`].
pub async fn read_link(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref();
    tokio::fs::read_link(path).await.at(path)
}

/// A wrapper around [`tokio::fs::read_to_string`].
pub async fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    let path = path.as_ref();
    tokio::fs::read_to_string(path).await.at(path)
}

/// A wrapper around [`tokio::fs::remove_dir`].
pub async fn remove_dir(path: impl AsRef<Path>) -> io::Result {
    let path = path.as_ref();
    tokio::fs::remove_dir(path).await.at(path)
}

/// A wrapper around [`tokio::fs::remove_dir_all`].
pub async fn remove_dir_all(path: impl AsRef<Path>) -> io::Result {
    let path = path.as_ref();
    tokio::fs::remove_dir_all(path).await.at(path)
}

/// A wrapper around [`tokio::fs::remove_file`].
pub async fn remove_file(path: impl AsRef<Path>) -> io::Result {
    let path = path.as_ref();
    tokio::fs::remove_file(path).await.at(path)
}

/// A wrapper around [`tokio::fs::rename`].
pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result {
    let from = from.as_ref();
    let to = to.as_ref();
    tokio::fs::rename(from, to).await.at2(from, to)
}

/// A wrapper around [`tokio::fs::set_permissions`].
pub async fn set_permissions(path: impl AsRef<Path>, perm: Permissions) -> io::Result {
    let path = path.as_ref();
    tokio::fs::set_permissions(path, perm).await.at(path)
}

#[cfg(unix)]
/// A wrapper around [`tokio::fs::symlink`].
pub async fn symlink(original: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result {
    let original = original.as_ref();
    let link = link.as_ref();
    tokio::fs::symlink(original, link).await.at2(original, link)
}

#[cfg(windows)]
/// A wrapper around [`tokio::fs::symlink_dir`].
pub async fn symlink_dir(original: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result {
    let original = original.as_ref();
    let link = link.as_ref();
    tokio::fs::symlink_dir(original, link).await.at2(original, link)
}

#[cfg(windows)]
/// A wrapper around [`tokio::fs::symlink_file`].
pub async fn symlink_file(original: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result {
    let original = original.as_ref();
    let link = link.as_ref();
    tokio::fs::symlink_file(original, link).await.at2(original, link)
}

/// A wrapper around [`tokio::fs::symlink_metadata`].
pub async fn symlink_metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let path = path.as_ref();
    tokio::fs::symlink_metadata(path).await.at(path)
}

/// A wrapper around [`tokio::fs::write`].
pub async fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result {
    let path = path.as_ref();
    tokio::fs::write(path, contents).await.at(path)
}

/// Like [`write()`] but return [`std::io::ErrorKind::AlreadyExists`] if the file exists.
pub async fn write_new(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result {
    let path = path.as_ref();
    File::create_new(path).await?.write_all(contents.as_ref()).await.at(path)
}

#[cfg(feature = "serde_json")]
/// A convenience method for serializing and writing a JSON file with proper indentation and a trailing newline.
pub async fn write_json(path: impl AsRef<Path>, value: impl Serialize) -> Result<(), JsonError> {
    let path = path.as_ref();
    let mut inner_serializer = serde_json::Serializer::with_formatter(Vec::default(), serde_json::ser::PrettyFormatter::with_indent(b"    "));
    let mut track = serde_path_to_error::Track::new();
    let serializer = serde_path_to_error::Serializer::new(&mut inner_serializer, &mut track);
    let res = value.serialize(serializer);
    res.map_err(|source| JsonError::Json { source: serde_path_to_error::Error::new(track.path(), source), path: path.to_owned() })?;
    let mut buf = inner_serializer.into_inner();
    buf.push(b'\n');
    Ok(write(path, buf).await?)
}

#[cfg(feature = "serde_json")]
/// Like [`write_json`] but return [`std::io::ErrorKind::AlreadyExists`] if the file exists.
pub async fn write_json_new(path: impl AsRef<Path>, value: impl Serialize) -> Result<(), JsonError> {
    let path = path.as_ref();
    let mut inner_serializer = serde_json::Serializer::with_formatter(Vec::default(), serde_json::ser::PrettyFormatter::with_indent(b"    "));
    let mut track = serde_path_to_error::Track::new();
    let serializer = serde_path_to_error::Serializer::new(&mut inner_serializer, &mut track);
    let res = value.serialize(serializer);
    res.map_err(|source| JsonError::Json { source: serde_path_to_error::Error::new(track.path(), source), path: path.to_owned() })?;
    let mut buf = inner_serializer.into_inner();
    buf.push(b'\n');
    Ok(write_new(path, buf).await?)
}
