//! Backend error normalization: cpal's unified [`cpal::Error`] → the
//! port's [`DeviceError`].
//!
//! Mapping table source:
//! `docs/designs/audio-device-boundary-and-poc.md` — "マッピング設計
//! （adapter 内部責務）". All call sites funnel through
//! [`map_error`]; no caller carries its own mapping rule.
//!
//! [`DeviceError`] is `Copy` + `&'static str` payloads: a value built on
//! the error-callback path never allocates.

use cpal::ErrorKind;
use ruviform_audio_core::device::DeviceError;

/// Which call path an error arrived on — the same [`ErrorKind`] maps to
/// a different [`DeviceError`] per path (see the mapping table in the
/// design document).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ErrorCtx {
    /// Control path: enumeration, config queries, open, start/stop,
    /// `now`, `buffer_size`.
    Control,
    /// The stream error callback (RT-adjacent).
    ErrorCallback,
}

/// Normalizes a cpal error onto the device-boundary error type.
///
/// Per the design mapping table:
///
/// - `DeviceNotAvailable` → [`DeviceError::DeviceNotFound`] on the
///   control path (vanished after enumeration), but
///   [`DeviceError::StreamInvalidated`] on the error-callback path
///   (the stream lost its device).
/// - `DeviceChanged` → [`DeviceError::Backend`]`("device_changed")` — a
///   non-fatal advisory: streams built from the default device are
///   re-routed by the backend and keep running.
/// - `Xrun` → [`DeviceError::Xrun`] on the error-callback path only;
///   it cannot fire on the control path in normal cpal operation, so
///   a control-path `Xrun` is unclassifiable and falls to
///   [`DeviceError::Backend`]`("other")`.
/// - `HostUnavailable`/`PermissionDenied`/`InvalidInput`/
///   `ResourceExhausted`/`BackendError`/`Other` and any future variant
///   (cpal [`ErrorKind`] is `#[non_exhaustive]`) →
///   [`DeviceError::Backend`]`("…")` classification names.
pub(crate) fn map_error(e: &cpal::Error, ctx: ErrorCtx) -> DeviceError {
    match e.kind() {
        ErrorKind::DeviceBusy => DeviceError::DeviceBusy,
        ErrorKind::DeviceNotAvailable => match ctx {
            ErrorCtx::Control => DeviceError::DeviceNotFound,
            ErrorCtx::ErrorCallback => DeviceError::StreamInvalidated,
        },
        ErrorKind::DeviceChanged => DeviceError::Backend("device_changed"),
        ErrorKind::UnsupportedConfig => DeviceError::UnsupportedConfig,
        ErrorKind::UnsupportedOperation => DeviceError::Unsupported,
        ErrorKind::RealtimeDenied => DeviceError::RealtimeDenied,
        ErrorKind::StreamInvalidated => DeviceError::StreamInvalidated,
        ErrorKind::Xrun => match ctx {
            ErrorCtx::ErrorCallback => DeviceError::Xrun,
            // Xrun never fires on the control path in normal cpal
            // operation — if it somehow does, it is unclassifiable
            // and falls to the wildcard bucket.
            ErrorCtx::Control => DeviceError::Backend("other"),
        },
        ErrorKind::HostUnavailable => DeviceError::Backend("host_unavailable"),
        ErrorKind::PermissionDenied => DeviceError::Backend("permission_denied"),
        ErrorKind::InvalidInput => DeviceError::Backend("invalid_input"),
        ErrorKind::ResourceExhausted => DeviceError::Backend("resource_exhausted"),
        ErrorKind::BackendError => DeviceError::Backend("backend"),
        // `ErrorKind::Other` and any variant added in future cpal
        // releases (the enum is `#[non_exhaustive]`).
        ErrorKind::Other | _ => DeviceError::Backend("other"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cpal::Error::new` is a public constructor, so every branch of
    /// the mapping is exercisable with synthesized errors.
    fn err(kind: ErrorKind) -> cpal::Error {
        cpal::Error::new(kind)
    }

    #[test]
    fn device_not_available_maps_per_context() {
        assert_eq!(
            map_error(&err(ErrorKind::DeviceNotAvailable), ErrorCtx::Control),
            DeviceError::DeviceNotFound,
        );
        assert_eq!(
            map_error(&err(ErrorKind::DeviceNotAvailable), ErrorCtx::ErrorCallback,),
            DeviceError::StreamInvalidated,
        );
    }

    #[test]
    fn device_changed_is_a_non_fatal_advisory() {
        for ctx in [ErrorCtx::Control, ErrorCtx::ErrorCallback] {
            assert_eq!(
                map_error(&err(ErrorKind::DeviceChanged), ctx),
                DeviceError::Backend("device_changed"),
            );
        }
    }

    #[test]
    fn direct_variants_map_identically_on_both_paths() {
        let cases = [
            (ErrorKind::DeviceBusy, DeviceError::DeviceBusy),
            (ErrorKind::UnsupportedConfig, DeviceError::UnsupportedConfig),
            (ErrorKind::UnsupportedOperation, DeviceError::Unsupported),
            (ErrorKind::RealtimeDenied, DeviceError::RealtimeDenied),
            (ErrorKind::StreamInvalidated, DeviceError::StreamInvalidated),
        ];
        for (kind, expected) in cases {
            for ctx in [ErrorCtx::Control, ErrorCtx::ErrorCallback] {
                assert_eq!(map_error(&err(kind), ctx), expected, "{kind:?} on {ctx:?}");
            }
        }
    }

    #[test]
    fn xrun_is_xrun_only_on_the_error_callback_path() {
        assert_eq!(
            map_error(&err(ErrorKind::Xrun), ErrorCtx::ErrorCallback),
            DeviceError::Xrun,
        );
        // A control-path Xrun is unclassifiable → wildcard bucket.
        assert_eq!(
            map_error(&err(ErrorKind::Xrun), ErrorCtx::Control),
            DeviceError::Backend("other"),
        );
    }

    #[test]
    fn unclassifiable_variants_map_to_backend_classifications() {
        let cases = [
            (ErrorKind::HostUnavailable, "host_unavailable"),
            (ErrorKind::PermissionDenied, "permission_denied"),
            (ErrorKind::InvalidInput, "invalid_input"),
            (ErrorKind::ResourceExhausted, "resource_exhausted"),
            (ErrorKind::BackendError, "backend"),
            (ErrorKind::Other, "other"),
        ];
        for (kind, name) in cases {
            for ctx in [ErrorCtx::Control, ErrorCtx::ErrorCallback] {
                assert_eq!(
                    map_error(&err(kind), ctx),
                    DeviceError::Backend(name),
                    "{kind:?} on {ctx:?}",
                );
            }
        }
    }

    #[test]
    fn mapped_errors_never_allocate_and_stay_displayable() {
        // `DeviceError` is `Copy` — the error-callback path cannot
        // allocate. Also keep `Display` exercised.
        let error = map_error(&err(ErrorKind::Xrun), ErrorCtx::ErrorCallback);
        let copy = error;
        assert_eq!(error, copy);
        assert_eq!(error.to_string(), "buffer under/overrun (xrun)");
    }
}
