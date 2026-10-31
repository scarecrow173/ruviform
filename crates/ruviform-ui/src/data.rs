//! Bounded binary data transfer — the `rvf://data` pull side of the
//! 3-path message boundary.
//!
//! Large payloads (waveform peaks, spectrograms, previews) are never
//! inlined into JSON events. A service registers a named [`DataSource`]
//! in the shared [`DataRegistry`]; the frontend then pulls bounded
//! ranges via `rvf://data/<name>?offset=N&len=M`, served by the
//! adapter's scheme handler (single implementation of range checks,
//! caps and CORS — see `ruviform-ui-webview`).
//!
//! Names are URI-safe slugs chosen by Rust; the frontend cannot mint
//! new sources — it can only read registered ones, within
//! [`DataRegistry::max_chunk`].
//!
//! The wire-level paging contract (`offset`/`len` semantics, the
//! `X-Rvf-Total` header, status mapping, and the frontend paging
//! loop) lives in the serving adapter — see the `rvf://data` paging
//! 契約 section of the `ruviform-ui-webview` README.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, PoisonError, RwLock};

/// Default upper bound per pull (bytes). Pulls are capped to this even
/// when the request asks for more; callers page through the source.
pub const DEFAULT_MAX_CHUNK: u64 = 16 * 1024 * 1024;

/// Errors from bounded reads.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum DataError {
    /// No source registered under the requested name.
    NotFound {
        /// Requested source name.
        name: String,
    },
    /// `offset` exceeds the source length.
    OutOfRange {
        /// Requested source name.
        name: String,
        /// Requested offset.
        offset: u64,
        /// Total source length.
        total: u64,
    },
    /// The source failed to satisfy the read.
    Read {
        /// Requested source name.
        name: String,
        /// Source-reported reason.
        message: String,
    },
}

impl Display for DataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { name } => write!(f, "data source not found: {name:?}"),
            Self::OutOfRange {
                name,
                offset,
                total,
            } => write!(f, "offset {offset} out of range for {name:?} (len {total})"),
            Self::Read { name, message } => write!(f, "read from {name:?} failed: {message}"),
        }
    }
}

impl Error for DataError {}

/// A named, randomly readable binary source.
///
/// Implementations must be cheap to read bounded windows from — the
/// scheme handler calls [`DataSource::read`] on its own thread.
/// Sources are immutable once registered; publish updated data by
/// registering a new name.
///
/// **RT contract:** non-RT only.
pub trait DataSource: Send + Sync {
    /// Total length in bytes.
    fn len(&self) -> u64;

    /// Whether the source is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reads up to `buf.len()` bytes at `offset`; returns the count
    /// actually written (short reads only at end of source).
    ///
    /// # Errors
    /// Returns a human-readable message wrapped by the registry into
    /// [`DataError::Read`].
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, String>;
}

/// [`DataSource`] over an in-memory `Vec<u8>` — the common case for
/// computed payloads (peak buffers, tiles).
pub struct VecDataSource {
    bytes: Vec<u8>,
}

impl VecDataSource {
    /// Wraps `bytes` as a data source.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl From<Vec<u8>> for VecDataSource {
    fn from(bytes: Vec<u8>) -> Self {
        Self::new(bytes)
    }
}

impl DataSource for VecDataSource {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, String> {
        let offset = usize::try_from(offset)
            .map_err(|_| "offset does not fit in address space".to_string())?;
        let Some(tail) = self.bytes.get(offset..) else {
            return Ok(0);
        };
        let n = tail.len().min(buf.len());
        buf[..n].copy_from_slice(&tail[..n]);
        Ok(n)
    }
}

/// A bounded read request against a named source.
#[derive(Debug, Clone)]
pub struct DataRequest {
    /// Registered source name.
    pub name: String,
    /// Start offset in bytes.
    pub offset: u64,
    /// Requested length; `None` means "up to the cap". Always clamped
    /// to [`DataRegistry::max_chunk`] and the source remainder.
    pub len: Option<u64>,
}

/// Result of a bounded read: the bytes plus the total source length so
/// the caller can page (mirrored as `X-Rvf-Total` on the wire).
#[derive(Debug)]
pub struct DataChunk {
    /// Bytes read (length ≤ [`DataRegistry::max_chunk`]).
    pub bytes: Vec<u8>,
    /// Total source length.
    pub total: u64,
}

/// Shared registry of named data sources. Cheaply cloneable (`Arc`
/// inside): the runtime, job workers and the scheme handler all hold
/// clones of one registry.
///
/// **RT contract:** non-RT only.
#[derive(Clone)]
pub struct DataRegistry {
    inner: Arc<RwLock<BTreeMap<String, Arc<dyn DataSource>>>>,
    max_chunk: u64,
}

impl Default for DataRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DataRegistry {
    /// Empty registry with the default pull cap
    /// ([`DEFAULT_MAX_CHUNK`]).
    #[must_use]
    pub fn new() -> Self {
        Self::with_max_chunk(DEFAULT_MAX_CHUNK)
    }

    /// Empty registry with a custom pull cap (bytes).
    #[must_use]
    pub fn with_max_chunk(max_chunk: u64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(BTreeMap::new())),
            max_chunk,
        }
    }

    /// Maximum bytes a single pull may return.
    #[must_use]
    pub fn max_chunk(&self) -> u64 {
        self.max_chunk
    }

    /// Registers `source` under `name`, replacing any previous entry.
    pub fn register(&self, name: &str, source: Arc<dyn DataSource>) {
        let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        map.insert(name.to_string(), source);
    }

    /// Registered source names (for capability reporting and tests).
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.inner
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .keys()
            .cloned()
            .collect()
    }

    /// Resolves `name` to its source.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn DataSource>> {
        self.inner
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(name)
            .cloned()
    }

    /// Performs a bounded read: clamps `req.len` to the cap and the
    /// source remainder, rejects `offset > total`.
    ///
    /// # Errors
    /// [`DataError::NotFound`], [`DataError::OutOfRange`],
    /// [`DataError::Read`].
    pub fn read(&self, req: &DataRequest) -> Result<DataChunk, DataError> {
        let src = self.get(&req.name).ok_or_else(|| DataError::NotFound {
            name: req.name.clone(),
        })?;
        let total = src.len();
        if req.offset > total {
            return Err(DataError::OutOfRange {
                name: req.name.clone(),
                offset: req.offset,
                total,
            });
        }
        let avail = total - req.offset;
        let want = req.len.unwrap_or(avail).min(avail).min(self.max_chunk);
        let mut buf = vec![
            0u8;
            usize::try_from(want).map_err(|_| DataError::Read {
                name: req.name.clone(),
                message: "chunk does not fit in address space".to_string(),
            })?
        ];
        let n = src
            .read(req.offset, &mut buf)
            .map_err(|message| DataError::Read {
                name: req.name.clone(),
                message,
            })?;
        buf.truncate(n);
        Ok(DataChunk { bytes: buf, total })
    }
}
