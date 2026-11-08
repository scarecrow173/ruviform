//! Bounded binary data transfer — the `rvf://data` pull side of the
//! 3-path message boundary.
//!
//! **Shared semantics, WebView-bound wire (Issue #465):** the
//! registry/range-pull contract is driver-neutral, but the `rvf://data`
//! URL scheme and paging headers are served by the `ruviform-ui-webview`
//! adapter only; a driver-neutral delivery contract is a P2 follow-up.
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
//!
//! ## Release contract
//!
//! Registrations are released two ways (Issue #171):
//!
//! - Explicitly: [`DataRegistry::unregister`] removes one name,
//!   [`DataRegistry::clear`] removes all of them (view/session reset).
//! - Bounded: [`DataRegistryLimits::max_sources`] caps the number of
//!   live names; `register` calls past the cap evict the least
//!   recently used name first (register/read both count as uses).
//!
//! Release removes the name from the registry — further pulls and
//! `rvf://data` requests for it fail with [`DataError::NotFound`]
//! (404 on the wire). The payload's memory is freed once the last
//! `Arc` clone drops: reads already holding the source finish
//! normally, so unregister never corrupts an in-flight pull.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, PoisonError, RwLock};

/// Default upper bound per pull (bytes). Pulls are capped to this even
/// when the request asks for more; callers page through the source.
pub const DEFAULT_MAX_CHUNK: u64 = 16 * 1024 * 1024;

/// Limits governing a [`DataRegistry`]: the per-pull byte cap plus an
/// optional bound on how many sources stay registered at once.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone)]
pub struct DataRegistryLimits {
    /// Maximum bytes a single pull may return
    /// ([`DEFAULT_MAX_CHUNK`] when not overridden).
    pub max_chunk: u64,
    /// Maximum number of simultaneously registered source names.
    ///
    /// `None` (the default) keeps every registration until
    /// [`DataRegistry::unregister`]/[`DataRegistry::clear`]. With
    /// `Some(n)`, a [`DataRegistry::register`] call that would exceed
    /// the bound evicts the least recently used names until the
    /// registry is back at `n`. `Some(0)` retains nothing — every
    /// registration is evicted immediately.
    pub max_sources: Option<usize>,
}

impl Default for DataRegistryLimits {
    fn default() -> Self {
        Self {
            max_chunk: DEFAULT_MAX_CHUNK,
            max_sources: None,
        }
    }
}

impl DataRegistryLimits {
    /// Limits with the default chunk cap and an LRU bound of
    /// `max_sources` registered names.
    #[must_use]
    pub fn with_max_sources(max_sources: usize) -> Self {
        Self {
            max_sources: Some(max_sources),
            ..Self::default()
        }
    }
}

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

/// One registered source plus the access stamp used for LRU eviction.
struct Entry {
    source: Arc<dyn DataSource>,
    /// Monotonic counter value of the most recent register/read —
    /// the smallest value is the next eviction victim.
    last_used: u64,
}

/// Mutable registry state behind the shared lock.
struct Inner {
    map: BTreeMap<String, Entry>,
    /// Monotonic access counter; every register/get/read bumps it.
    tick: u64,
}

impl Inner {
    fn touch(&mut self) -> u64 {
        self.tick += 1;
        self.tick
    }
}

/// Shared registry of named data sources. Cheaply cloneable (`Arc`
/// inside): the runtime, job workers and the scheme handler all hold
/// clones of one registry.
///
/// Registrations live until [`DataRegistry::unregister`],
/// [`DataRegistry::clear`], or LRU eviction under
/// [`DataRegistryLimits::max_sources`] — see the module-level release
/// contract.
///
/// **RT contract:** non-RT only.
#[derive(Clone)]
pub struct DataRegistry {
    inner: Arc<RwLock<Inner>>,
    limits: DataRegistryLimits,
}

impl Default for DataRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DataRegistry {
    /// Empty registry with the default pull cap
    /// ([`DEFAULT_MAX_CHUNK`]) and no bound on registered names.
    #[must_use]
    pub fn new() -> Self {
        Self::with_limits(DataRegistryLimits::default())
    }

    /// Empty registry with a custom pull cap (bytes) and no bound on
    /// registered names.
    #[must_use]
    pub fn with_max_chunk(max_chunk: u64) -> Self {
        Self::with_limits(DataRegistryLimits {
            max_chunk,
            ..DataRegistryLimits::default()
        })
    }

    /// Empty registry with the default pull cap and an LRU bound of
    /// `max_sources` registered names.
    #[must_use]
    pub fn with_max_sources(max_sources: usize) -> Self {
        Self::with_limits(DataRegistryLimits::with_max_sources(max_sources))
    }

    /// Empty registry under explicit [`DataRegistryLimits`].
    #[must_use]
    pub fn with_limits(limits: DataRegistryLimits) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                map: BTreeMap::new(),
                tick: 0,
            })),
            limits,
        }
    }

    /// Maximum bytes a single pull may return.
    #[must_use]
    pub fn max_chunk(&self) -> u64 {
        self.limits.max_chunk
    }

    /// Bound on simultaneously registered names (`None` = unbounded).
    /// Past the bound, `register` evicts least recently used first.
    #[must_use]
    pub fn max_sources(&self) -> Option<usize> {
        self.limits.max_sources
    }

    /// Registers `source` under `name`, replacing any previous entry
    /// and marking it most recently used. When the registry exceeds
    /// [`DataRegistryLimits::max_sources`], least recently used names
    /// are evicted until the bound holds again.
    pub fn register(&self, name: &str, source: Arc<dyn DataSource>) {
        let mut inner = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        let tick = inner.touch();
        inner.map.insert(
            name.to_string(),
            Entry {
                source,
                last_used: tick,
            },
        );
        if let Some(max) = self.limits.max_sources {
            while inner.map.len() > max {
                let Some(victim) = inner
                    .map
                    .iter()
                    .min_by_key(|(_, e)| e.last_used)
                    .map(|(k, _)| k.clone())
                else {
                    break;
                };
                inner.map.remove(&victim);
            }
        }
    }

    /// Removes `name` from the registry; returns whether it was
    /// registered. The payload's memory is freed once the last `Arc`
    /// clone drops — reads already holding the source still finish.
    pub fn unregister(&self, name: &str) -> bool {
        self.inner
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .map
            .remove(name)
            .is_some()
    }

    /// Removes every registered source (view/session reset). Same
    /// deferred-free semantics as [`DataRegistry::unregister`].
    pub fn clear(&self) {
        self.inner
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .map
            .clear();
    }

    /// Number of currently registered sources.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .map
            .len()
    }

    /// Whether no sources are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Registered source names (for capability reporting and tests).
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.inner
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .map
            .keys()
            .cloned()
            .collect()
    }

    /// Resolves `name` to its source, marking it most recently used
    /// for LRU purposes.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn DataSource>> {
        let mut inner = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        let tick = inner.touch();
        let entry = inner.map.get_mut(name)?;
        entry.last_used = tick;
        Some(entry.source.clone())
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
        let want = req
            .len
            .unwrap_or(avail)
            .min(avail)
            .min(self.limits.max_chunk);
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
