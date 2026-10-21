//! [`Host`] — the adapter's concrete entry point — and the opaque
//! [`HostId`] used for host selection.
//!
//! `Host` wraps the backend's host object privately and implements the
//! port's [`DeviceEnumerator`], so the engine can use it directly or
//! behind `Box<dyn DeviceEnumerator>`. No cpal type appears on this
//! surface.
//!
//! **Thread model:** enumeration and default-device queries are
//! control-thread operations — non-RT (they call OS APIs and
//! allocate). Nothing in this module is RT-suitable.

use cpal::traits::HostTrait;
use ruviform_audio_core::device::{Device, DeviceEnumerator, DeviceResult};

use crate::device::CpalDevice;
use crate::error::{ErrorCtx, map_error};

/// A backend audio host (e.g. WASAPI on Windows) implementing the
/// port's device enumeration entry point.
///
/// The backend host object is held privately: no cpal type appears in
/// this type's public API.
///
/// **RT contract:** non-RT — a control-thread object.
///
/// # Examples
///
/// ```no_run
/// use ruviform_audio_core::device::DeviceEnumerator;
///
/// let host = ruviform_audio_device::default_host();
/// let devices = host.output_devices().unwrap_or_default();
/// println!("{} output device(s) on host {}", devices.len(), host.name());
/// ```
pub struct Host {
    inner: cpal::Host,
}

/// An opaque identifier selecting one backend audio host.
///
/// Obtained via [`available_host_ids`] or [`Host::id`]; consumed by
/// [`host`]. The string representation is an implementation detail —
/// display or persist via [`HostId::name`], never interpret it.
///
/// **RT contract:** non-RT — `Copy` identifier, obtained through
/// allocating enumeration calls.
///
/// # Examples
///
/// ```
/// // Enumerating host IDs does not touch audio hardware.
/// for id in ruviform_audio_device::available_host_ids() {
///     println!("host: {}", id.name());
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HostId {
    inner: cpal::HostId,
}

impl Host {
    /// The backend display name of this host (e.g. `"WASAPI"`).
    ///
    /// **RT contract:** RT-suitable — returns a static string.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.inner.id().name()
    }

    /// This host's opaque [`HostId`] — usable with [`host`] to
    /// re-acquire the same backend host later.
    ///
    /// **RT contract:** RT-suitable — `Copy` value.
    #[must_use]
    pub fn id(&self) -> HostId {
        HostId {
            inner: self.inner.id(),
        }
    }
}

impl HostId {
    /// The backend display name (same value as [`Host::name`]).
    ///
    /// **RT contract:** RT-suitable — returns a static string.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.inner.name()
    }
}

/// The OS default audio host — `WASAPI` on Windows, `CoreAudio` on
/// macOS/iOS, `ALSA` on Linux (in the default feature set).
///
/// **RT contract:** non-RT only — constructs the backend host.
///
/// # Examples
///
/// ```no_run
/// let host = ruviform_audio_device::default_host();
/// ```
#[must_use]
pub fn default_host() -> Host {
    Host {
        inner: cpal::default_host(),
    }
}

/// The host identifiers currently usable on this system — only hosts
/// enabled by the compiled backend features are listed (in the default
/// feature set, the single OS-native host).
///
/// **RT contract:** non-RT only — allocates.
///
/// # Examples
///
/// ```
/// let ids = ruviform_audio_device::available_host_ids();
/// assert!(!ids.is_empty()); // every supported platform has a default host
/// ```
#[must_use]
pub fn available_host_ids() -> Vec<HostId> {
    cpal::available_hosts()
        .into_iter()
        .map(|inner| HostId { inner })
        .collect()
}

/// Acquires the host identified by `id` (from [`available_host_ids`]).
///
/// **RT contract:** non-RT only — constructs the backend host.
///
/// # Errors
///
/// [`ruviform_audio_core::device::DeviceError::Backend`]
/// `("host_unavailable")` when the host is not currently reachable —
/// e.g. the audio daemon went away after enumeration.
///
/// # Examples
///
/// ```no_run
/// let id = ruviform_audio_device::available_host_ids()[0];
/// let host = ruviform_audio_device::host(id).unwrap();
/// ```
pub fn host(id: HostId) -> DeviceResult<Host> {
    cpal::host_from_id(id.inner)
        .map(|inner| Host { inner })
        .map_err(|e| map_error(&e, ErrorCtx::Control))
}

impl DeviceEnumerator for Host {
    fn output_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
        self.inner
            .output_devices()
            .map(|devices| {
                devices
                    .map(|device| Box::new(CpalDevice::new(device)) as Box<dyn Device>)
                    .collect()
            })
            .map_err(|e| map_error(&e, ErrorCtx::Control))
    }

    fn default_output_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
        // cpal reports "no default device" as `None` — the same shape
        // the port uses; the query itself is infallible.
        Ok(self
            .inner
            .default_output_device()
            .map(|device| Box::new(CpalDevice::new(device)) as Box<dyn Device>))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_id_exposes_a_static_name() {
        for id in available_host_ids() {
            assert_ne!(id.name(), "");
        }
    }

    #[test]
    fn host_is_a_device_enumerator() {
        // Compile-time trait conformance — no hardware touched.
        fn assert_enumerator(_: &dyn DeviceEnumerator) {}
        assert_enumerator(&default_host());
    }

    #[test]
    fn default_host_reports_a_consistent_id() {
        let host = default_host();
        assert_eq!(host.name(), host.id().name());
    }
}
