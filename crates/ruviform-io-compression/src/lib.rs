//! `ruviform-io-compression` — byte-stream compression for Ruviform IO
//! (IO-IMP-06 / Issue #300).
//!
//! One crate, one format module per container format:
//!
//! | module    | format                | feature    |
//! |-----------|-----------------------|------------|
//! | [`gzip`]  | Gzip (RFC 1952)       | `gzip`     |
//! | [`deflate`]| raw DEFLATE (RFC 1951) | `deflate`  |
//!
//! `default = []` — each format is an opt-in feature so downstream
//! crates compile only what they use. A `zstd` module is planned as
//! IO-IMP-07 (Issue #301) and will reuse the same
//! [`DecodeScope`]/[`DecodeLimits`]/receipt vocabulary; it is
//! intentionally not declared here.
//!
//! # Borrowed stream types
//!
//! Both modules expose `Encoder<'w, W: Write + ?Sized>` and
//! `Decoder<'r, R: BufRead + ?Sized>` — borrowed, `?Sized`-friendly
//! adapters that compose with any `std::io` layer (files, `BufReader`,
//! TAR builders, network streams) without owning them:
//!
//! ```
//! # #[cfg(feature = "gzip")]
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use ruviform_io_compression::gzip::{Decoder, DecodeOptions, EncodeOptions, Encoder};
//! use std::io::{BufReader, Read, Write};
//!
//! // TAR -> Gzip -> staged file composition is just Write-in-Write.
//! let mut sink = Vec::new();
//! let mut enc = Encoder::new(&mut sink, EncodeOptions::default())?;
//! enc.write_all(b"payload")?;
//! enc.finish()?; // explicit success proof; Drop writes nothing
//!
//! let mut source = BufReader::new(&sink[..]);
//! let mut dec = Decoder::new(&mut source, DecodeOptions::default());
//! let mut out = String::new();
//! dec.read_to_string(&mut out)?;
//! let receipt = dec.finish()?; // Ok only in VerifiedEnd
//! assert_eq!(out, "payload");
//! assert_eq!(receipt.compressed_bytes_consumed() as usize, sink.len());
//! # Ok(())
//! # }
//! # #[cfg(feature = "gzip")]
//! # example().unwrap();
//! ```
//!
//! # Completion and verification semantics
//!
//! - **Encoder**: `finish()` is the only successful completion proof
//!   (idempotent — it returns the stored [`EncodeReceipt`]). `Drop`
//!   performs no writes; a failed encoder is terminal (`InvalidState`),
//!   never silently retried.
//! - **Decoder**: `Read::read(&mut [])` and `Ok(0)` mid-stream are not
//!   EOF. `finish()` succeeds only in `VerifiedEnd` — after the format
//!   end, trailer/checksum verification and the [`DecodeScope`]
//!   input-boundary contract all hold — and returns `NotExhausted`
//!   while undelivered output may remain.
//! - **Gzip**: header/trailer validated, CRC32+ISIZE verified per
//!   member, concatenated members supported under
//!   [`DecodeScope::WholeInput`], trailing bytes rejected.
//!   [`DecodeScope::SingleUnit`] leaves bytes after the first member
//!   inside the caller's `BufRead`.
//! - **Raw deflate**: no checksum is claimed; a single stream whose
//!   end-boundary is verified.
//!
//! # Limits
//!
//! [`EncodeLimits`]/[`DecodeLimits`] are cumulative *byte and unit*
//! budgets enforced at this layer. They do not claim CPU-time or
//! backend-internal-memory hard limits — those remain caller/platform
//! concerns.
//!
//! # Backend boundary
//!
//! The compression backend is a private implementation detail: no
//! backend type, error or version appears in the public API and
//! backend failures surface only as normalized
//! [`CompressionError`]s (with a type-erased `source()` diagnostic).
//!
//! # Threading and RT contract
//!
//! `Encoder`/`Decoder` are `Send` when the borrowed stream is `Send`,
//! like any borrowed adapter. **Non-RT only**: `Read`/`Write` calls may
//! block, allocate, and perform unbounded-complexity work — never call
//! them from an audio real-time callback.

#![forbid(unsafe_code)]

mod error;
mod options;
mod receipt;

#[cfg(any(feature = "gzip", feature = "deflate"))]
mod stream;

#[cfg(feature = "deflate")]
pub mod deflate;
#[cfg(feature = "gzip")]
pub mod gzip;

pub use error::{CompressionError, CompressionErrorKind};
pub use options::{DecodeLimits, DecodeScope, EncodeLimits};
pub use receipt::{DecodeReceipt, EncodeReceipt, IntegritySummary};
