//! `feature = "file"` — staged-file composition for the format modules.
//!
//! Reading opens a plain file and decodes through a buffered reader;
//! writing serializes into a [`ruviform_io_file::StagedFile`], seals
//! and publishes it so the destination only ever receives a complete
//! document.
//!
//! The two error enums keep the failure axes separate and
//! information-lossless: [`DocumentFileWriteError::Publish`] carries
//! the *full* [`ruviform_io_file::PublishFailure`] — effect, phase,
//! durability evidence, recovery info and recovery handle — instead of
//! flattening it into a message.
//!
//! **RT contract:** non-RT only — performs filesystem I/O.

use crate::DocumentError;
use ruviform_io_file::{FileError, PublishFailure};
#[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
use ruviform_io_file::{PublishOptions, PublishReceipt, StagedFile};
use std::fmt;
#[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
use std::io::BufReader;
#[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
use std::path::Path;

/// Failure of a format module's `read_file`.
///
/// The variants are the matchable contract: `Open` means the file could
/// not be opened for reading at all, `Decode` means bytes were read but
/// did not form a valid document.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum DocumentFileReadError {
    /// The file could not be opened.
    Open(std::io::Error),
    /// The file contents failed to decode as the requested document.
    Decode(DocumentError),
}

impl fmt::Display for DocumentFileReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Open(_) => "ruviform-io-structured: failed to open document file",
            Self::Decode(_) => "ruviform-io-structured: failed to decode document file",
        };
        f.write_str(message)
    }
}

impl std::error::Error for DocumentFileReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Open(e) => Some(e),
            Self::Decode(e) => Some(e),
        }
    }
}

/// Failure of a format module's `write_file`.
///
/// - `Create` — the staged file could not be created
///   ([`ruviform_io_file::FileError`]); the destination was not
///   touched.
/// - `Encode` — serialization into the staged file failed; the staged
///   temp is dropped for best-effort cleanup and the destination is
///   never published, so no partial document is installed.
/// - `Publish` — the sealed staged file failed to publish; the full
///   [`PublishFailure`] is preserved — `effect()`, `phase()`,
///   durability evidence, `recovery_info()` and the recovery handle all
///   remain available to the caller.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum DocumentFileWriteError {
    /// `StagedFile::create` failed before any document bytes were
    /// written.
    Create(FileError),
    /// The document failed to serialize into the staged file.
    Encode(DocumentError),
    /// `SealedFile::publish` failed after a successful encode.
    Publish(PublishFailure),
}

impl fmt::Display for DocumentFileWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Create(_) => "ruviform-io-structured: failed to create staged file for document",
            Self::Encode(_) => "ruviform-io-structured: failed to encode document into staged file",
            Self::Publish(_) => "ruviform-io-structured: failed to publish staged document",
        };
        f.write_str(message)
    }
}

impl std::error::Error for DocumentFileWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Create(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Publish(e) => Some(e),
        }
    }
}

/// Shared `read_file` implementation: open + buffered decode.
#[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
pub(crate) fn open_document<T>(
    path: impl AsRef<Path>,
    decode: impl FnOnce(&mut BufReader<std::fs::File>) -> Result<T, DocumentError>,
) -> Result<T, DocumentFileReadError> {
    let file = std::fs::File::open(path).map_err(DocumentFileReadError::Open)?;
    let mut reader = BufReader::new(file);
    decode(&mut reader).map_err(DocumentFileReadError::Decode)
}

/// Shared `write_file` implementation: create staged → encode →
/// seal → publish. An encode failure drops the unsealed staged file,
/// which performs best-effort temp cleanup without touching the
/// destination.
#[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
pub(crate) fn write_document(
    path: impl AsRef<Path>,
    publish: PublishOptions,
    encode: impl FnOnce(&mut StagedFile) -> Result<(), DocumentError>,
) -> Result<PublishReceipt, DocumentFileWriteError> {
    let mut staged = StagedFile::create(path).map_err(DocumentFileWriteError::Create)?;
    encode(&mut staged).map_err(DocumentFileWriteError::Encode)?;
    staged
        .seal()
        .publish(publish)
        .map_err(DocumentFileWriteError::Publish)
}
