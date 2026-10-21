//! Error normalization: backend errors → media-io [`Error`].
//!
//! Every backend error crossing into this crate's public paths is
//! normalized into a media-io [`ErrorKind`] classification; the original
//! backend error is preserved only as a type-erased diagnostic reachable
//! through [`std::error::Error::source`]. No backend error type ever
//! appears in the public API (type purity — media-io ADR).

use ruviform_media_io::{Error, ErrorKind};
use symphonia::core::errors::{Error as BackendError, SeekErrorKind};

/// Normalizes a backend error into the media-io [`Error`] model.
///
/// Mapping table (the boundary's normalization contract):
///
/// | backend error | [`ErrorKind`] |
/// | --- | --- |
/// | `IoError` (`UnexpectedEof`) | [`ErrorKind::UnexpectedEos`] — truncated input |
/// | `IoError` (other) | [`ErrorKind::Io`] |
/// | `DecodeError` | [`ErrorKind::InvalidData`] |
/// | `SeekError::Unseekable` / `ForwardOnly` | [`ErrorKind::Unsupported`] — capability gap |
/// | `SeekError::OutOfRange` / `InvalidTrack` | [`ErrorKind::InvalidValue`] — bad request |
/// | `Unsupported` | [`ErrorKind::Unsupported`] |
/// | `LimitError` | [`ErrorKind::InvalidData`] — hostile/over-limit input |
/// | `ResetRequired` | [`ErrorKind::InvalidData`] — mid-stream format change the PoC does not support |
pub(crate) fn map_error(source: BackendError) -> Error {
    let kind = match &source {
        BackendError::IoError(error) => {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                ErrorKind::UnexpectedEos
            } else {
                ErrorKind::Io
            }
        }
        BackendError::SeekError(kind) => match kind {
            SeekErrorKind::Unseekable | SeekErrorKind::ForwardOnly => ErrorKind::Unsupported,
            _ => ErrorKind::InvalidValue,
        },
        BackendError::Unsupported(_) => ErrorKind::Unsupported,
        // `DecodeError` (corrupt or contract-violating input),
        // `LimitError` (hostile/over-limit input), `ResetRequired`
        // (mid-stream format change the proof of concept does not
        // support), and any future `#[non_exhaustive]` variant land on
        // the least-specific normalized class.
        _ => ErrorKind::InvalidData,
    };
    Error::with_source(kind, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symphonia::core::errors::SeekErrorKind as BackendSeekKind;

    #[test]
    fn io_error_maps_to_io_kind() {
        let error = map_error(BackendError::IoError(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "pipe",
        )));
        assert_eq!(error.kind(), ErrorKind::Io);
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn unexpected_eof_maps_to_unexpected_eos() {
        let error = map_error(BackendError::IoError(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "eof",
        )));
        assert_eq!(error.kind(), ErrorKind::UnexpectedEos);
    }

    #[test]
    fn decode_and_unsupported_map_to_their_kinds() {
        let error = map_error(BackendError::DecodeError("bad"));
        assert_eq!(error.kind(), ErrorKind::InvalidData);
        let error = map_error(BackendError::Unsupported("nope"));
        assert_eq!(error.kind(), ErrorKind::Unsupported);
        let error = map_error(BackendError::LimitError("too big"));
        assert_eq!(error.kind(), ErrorKind::InvalidData);
        let error = map_error(BackendError::ResetRequired);
        assert_eq!(error.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn seek_errors_split_between_unsupported_and_invalid_value() {
        let error = map_error(BackendError::SeekError(BackendSeekKind::Unseekable));
        assert_eq!(error.kind(), ErrorKind::Unsupported);
        let error = map_error(BackendError::SeekError(BackendSeekKind::ForwardOnly));
        assert_eq!(error.kind(), ErrorKind::Unsupported);
        let error = map_error(BackendError::SeekError(BackendSeekKind::OutOfRange));
        assert_eq!(error.kind(), ErrorKind::InvalidValue);
        let error = map_error(BackendError::SeekError(BackendSeekKind::InvalidTrack));
        assert_eq!(error.kind(), ErrorKind::InvalidValue);
    }
}
