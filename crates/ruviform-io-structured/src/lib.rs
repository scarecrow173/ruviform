//! `ruviform-io-structured` — Serde-typed structured document I/O
//! (IO-02 / Issue #284, IO-IMP-03 / Issue #297).
//!
//! Each document format lives in its own feature-gated module:
//!
//! | module | feature | backend (private) |
//! | --- | --- | --- |
//! | [`json`] | `json` | `serde_json` |
//! | [`yaml`] | `yaml` | `serde-saphyr` |
//! | [`toml`] | `toml` | `toml` |
//! | [`csv`] | `csv` | `csv-core` + `csv` |
//!
//! Every format exposes the same shape:
//!
//! - `from_reader<T: DeserializeOwned>(&mut impl Read, &XxxReadOptions)`
//!   — single document, **strict end-of-input** (non-whitespace data
//!   after the document is
//!   [`DocumentErrorKind::TrailingData`]), input byte budget via
//!   [`InputLimits::max_bytes`]
//!   ([`DocumentErrorKind::LimitExceeded`]).
//! - `to_writer<T: Serialize + ?Sized>(&mut impl Write, &T, &XxxWriteOptions)`
//!   — borrowed writer; success means only that the serializer's output
//!   reached the writer. `Write::flush`, file sync and publish are
//!   separate guarantees; on failure the writer may hold a partial
//!   document.
//! - `read_file` / `write_file` (`feature = "file"`) — convenience on
//!   top of [`ruviform_io_file`]'s `StagedFile`: serialize → `seal()` →
//!   `publish(PublishOptions)`, with
//!   [`DocumentFileReadError`] / [`DocumentFileWriteError`] keeping the
//!   encode vs. publish failure axes separate.
//!
//! # Backend boundary
//!
//! `serde::{Serialize, de::DeserializeOwned}` is the *only* third-party
//! surface in the public API (an explicit IO-02 design agreement). All
//! backend types, errors and feature names stay private — backend
//! diagnostics are reachable only as `dyn Error` through
//! [`std::error::Error::source`].
//!
//! # Errors
//!
//! All failures normalize to [`DocumentError`] with a matchable
//! [`DocumentErrorKind`], the producing [`DocumentFormat`], an optional
//! [`DocumentPosition`] and an optional [`DocumentLimit`]. `Display`
//! never embeds backend-rendered text or input snippets.
//!
//! # RT contract
//!
//! **Non-RT only** — every operation may block on I/O and allocate.
//! Never call this API from an RT audio callback.
#![allow(clippy::module_name_repetitions)]

mod error;
#[cfg(any(feature = "json", feature = "yaml", feature = "toml", feature = "csv"))]
mod input;
mod limits;

pub use error::{
    DocumentError, DocumentErrorKind, DocumentFormat, DocumentLimit, DocumentPosition,
};
pub use limits::{DEFAULT_MAX_INPUT_BYTES, InputLimits};

#[cfg(feature = "csv")]
pub mod csv;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "toml")]
pub mod toml;
#[cfg(feature = "yaml")]
pub mod yaml;

#[cfg(feature = "file")]
mod file;
#[cfg(feature = "file")]
pub use file::{DocumentFileReadError, DocumentFileWriteError};
