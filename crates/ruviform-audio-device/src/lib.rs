//! `ruviform-audio-device` — the audio device I/O adapter: an
//! implementation of the device boundary ports
//! ([`ruviform_audio_core::device`]) on the `cpal` backend.
//!
//! The crate is the *only* place cpal is linked: its types, errors and
//! feature names never appear on this crate's public surface — [`Host`]
//! and [`HostId`] wrap the backend handles privately, devices and
//! streams reach the caller only as the port's `Box<dyn Device>` /
//! `Box<dyn OutputStream>` / `Box<dyn InputStream>` trait objects, and
//! every backend error is
//! normalized onto the port's `DeviceError`. A second backend (cubeb
//! etc.) would be a *separate* adapter crate, not a feature switch
//! here.
//!
//! ## Responsibilities
//!
//! - Host enumeration and selection: [`default_host`],
//!   [`available_host_ids`], [`host`].
//! - Format negotiation, first duty: an openable `F32`
//!   [`AudioFormat`](ruviform_audio_core::AudioFormat) is preferred
//!   even when the backend default is another encoding; only
//!   F32-capable ranges are reported as the supported space.
//! - Callback plumbing: the engine writes planar `f32` through a
//!   bounded `AudioBufferViewMut` built on a scratch slab allocated
//!   once at open; the adapter packs to the interleaved backend
//!   boundary — `f32` interleaved exists only at the
//!   backend-facing edge.
//!
//! ## Allowed dependency layers
//!
//! `ruviform-audio-core` (the port), `ruviform-media-core`
//! ([`MediaTime`](ruviform_media_core::MediaTime) for the device
//! clock), and `cpal` (the backend, confined). Adapters never depend
//! on the engine crate (`ruviform-audio`) nor on other adapters.
//!
//! ## RT contract
//!
//! Enumeration, format queries, open/start/stop, `now` and
//! `buffer_size` are control-path APIs — non-RT (they call OS APIs and
//! may allocate or block). The output data callback runs on the
//! backend's real-time thread: allocation-free and bounded by
//! construction (open-time scratch + per-callback borrow, no
//! `Arc`/`Mutex` between the data and error closures). Per-API
//! guarantees are documented as `**RT contract:**` lines. CI success
//! is not a complete proof of RT safety (realtime ADR).
//!
//! Contract source:
//! `docs/designs/audio-device-boundary-and-poc.md` —
//! "`ruviform-audio-device` クレート構成", "マッピング設計",
//! "コールバック配管"; backend selection:
//! `docs/adr/select-audio-device-backend.md`.
//!
//! # Examples
//!
//! ```no_run
//! use ruviform_audio_core::device::DeviceEnumerator;
//!
//! // Enumerate output devices on the OS default host — control path.
//! let host = ruviform_audio_device::default_host().unwrap();
//! for device in host.output_devices().unwrap_or_default() {
//!     println!("{}", device.name().unwrap_or_else(|_| "?".into()));
//! }
//! ```

mod convert;
mod device;
mod error;
// The module is renamed at the declaration so the public `host(id)`
// entry function can live at the crate root beside `mod host`'s items
// — a private `mod host` and a `pub fn host` would collide in the root
// namespace.
#[path = "host.rs"]
mod host_impl;
mod stream;

pub use host_impl::{Host, HostId, available_host_ids, default_host, host};
