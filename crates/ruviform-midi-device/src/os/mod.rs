//! Platform dispatch for the MIDI backends.
//!
//! `windows.rs`/`windows_output.rs` hold the `WinMM` input/output
//! implementations; `unsupported.rs` is the explicit
//! `DeviceError::Unsupported` boundary for every other target.
//! `SystemBackend` reaches whichever `enumerate_ports` /
//! `enumerate_output_ports` the target provides — a plain `Vec` on
//! Windows (infallible: devices vanishing mid-scan are skipped) and a
//! `DeviceResult` elsewhere.

#[cfg(not(windows))]
pub(crate) mod unsupported;
#[cfg(windows)]
pub(crate) mod windows;
#[cfg(windows)]
pub(crate) mod windows_output;

#[cfg(not(windows))]
pub(crate) use unsupported::{enumerate_output_ports, enumerate_ports};
#[cfg(windows)]
pub(crate) use windows::enumerate_ports;
#[cfg(windows)]
pub(crate) use windows_output::enumerate_output_ports;
