//! Platform dispatch for the MIDI input backend.
//!
//! `windows.rs` holds the `WinMM` implementation; `unsupported.rs` is the
//! explicit `DeviceError::Unsupported` boundary for every other
//! target. `SystemBackend` reaches whichever `enumerate_ports` the
//! target provides — a plain `Vec` on Windows (infallible: devices
//! vanishing mid-scan are skipped) and a `DeviceResult` elsewhere.

#[cfg(not(windows))]
pub(crate) mod unsupported;
#[cfg(windows)]
pub(crate) mod windows;

#[cfg(not(windows))]
pub(crate) use unsupported::enumerate_ports;
#[cfg(windows)]
pub(crate) use windows::enumerate_ports;
