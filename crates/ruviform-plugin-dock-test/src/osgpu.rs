//! OS display-adapter observation for the real-window dock lane
//! (Issue #631 gpu-field slice — feature `real-dock`).
//!
//! Query-only `EnumDisplayDevicesW` reads recording the session's
//! display adapters — the AC4 'OS and GPU' field's GPU portion —
//! for `real_dock` to stamp into every real case's
//! `environment.display_adapters`. The same boundary convention as
//! the sibling `osoccl`/`ospix`/`ospres`/`osmut` modules: this
//! module never creates, destroys, moves, re-styles, or messages a
//! window; it only enumerates display devices.
//!
//! Two enumeration passes:
//!
//! - Adapter pass — `EnumDisplayDevicesW(NULL, idevnum, …, 0)`
//!   walks every display adapter the session exposes (GDI's view
//!   of the GPUs: hardware adapters plus software/remote display
//!   drivers). Each record carries `DeviceName` (`\\.\DISPLAYn`),
//!   `DeviceString` (the adapter's descriptive name — the GPU
//!   model for hardware devices), `DeviceID` (the PNP hardware id,
//!   `PCI\VEN_…` for hardware adapters), `DeviceKey` (its registry
//!   driver key), and the raw `StateFlags` with the
//!   `PRIMARY`/`ATTACHED_TO_DESKTOP` bits decoded.
//! - Monitor pass — `EnumDisplayDevicesW(<adapter>, i, …,
//!   EDD_GET_DEVICE_INTERFACE_NAME)` enumerates the monitors
//!   attached to each adapter, recording the monitor's device
//!   interface name plus its string/id/key — the display topology
//!   the session's windows rendered on.
//!
//! The record is deliberately the GDI display-adapter view, never
//! a renderer claim: it does not identify which device GPUI's
//! compositor actually rendered on (that would need DXGI adapter
//! enumeration or renderer info the GPUI layer does not expose —
//! the slice's ADR records that trade-off).
//!
//! All values crossing the module boundary are plain integers,
//! booleans, and strings — no Win32 type reaches the caller (the
//! same convention as `osoccl`/`ospix`/`ospres`).
//!
//! **RT contract:** non-RT only. **Test-only:** nothing here ships.

/// One enumerated monitor attached to a display adapter —
/// `EnumDisplayDevicesW(<adapter>, i, …,
/// EDD_GET_DEVICE_INTERFACE_NAME)` facts, plain values only.
#[derive(Debug, Clone)]
pub(crate) struct DisplayMonitor {
    /// Monitor index within the adapter's enumeration.
    pub index: u32,
    /// `DeviceName` — the monitor's device interface name
    /// (`EDD_GET_DEVICE_INTERFACE_NAME` form,
    /// `\\?\DISPLAY#…#{…}`).
    pub device_name: String,
    /// `DeviceString` — the monitor's descriptive string.
    pub device_string: String,
    /// `DeviceID` — the monitor's PNP hardware id.
    pub device_id: String,
    /// `DeviceKey` — the monitor's registry driver key.
    pub device_key: String,
    /// `StateFlags` raw value.
    pub state_flags: u32,
}

/// One enumerated display adapter —
/// `EnumDisplayDevicesW(NULL, idevnum, …, 0)` facts plus its
/// monitor census, plain values only.
#[derive(Debug, Clone)]
pub(crate) struct DisplayAdapter {
    /// `idevnum` the adapter enumerated at.
    pub index: u32,
    /// `DeviceName` — the OS display device name
    /// (`\\.\DISPLAYn`).
    pub device_name: String,
    /// `DeviceString` — the adapter's descriptive name (the GPU
    /// model for hardware adapters).
    pub device_string: String,
    /// `DeviceID` — the adapter's PNP hardware id (`PCI\VEN_…`
    /// for hardware adapters).
    pub device_id: String,
    /// `DeviceKey` — the adapter's registry driver key.
    pub device_key: String,
    /// `StateFlags` raw value.
    pub state_flags: u32,
    /// `DISPLAY_DEVICE_ATTACHED_TO_DESKTOP` decoded.
    pub attached_to_desktop: bool,
    /// `DISPLAY_DEVICE_PRIMARY_DEVICE` decoded.
    pub primary: bool,
    /// Monitors enumerated on this adapter (the second pass —
    /// empty when the adapter reports none or the pass fails).
    pub monitors: Vec<DisplayMonitor>,
}

/// The session's display-adapter inventory.
#[derive(Debug)]
pub(crate) struct DisplayAdapters {
    /// `false` only on the non-Windows stub — callers on the real
    /// lane are always inside a Windows session, so this is a
    /// defensive flag, not a second oracle.
    pub supported: bool,
    /// The enumerated adapters in `idevnum` order, capped at
    /// [`ADAPTER_CAP`].
    pub adapters: Vec<DisplayAdapter>,
    /// `true` when the enumeration exceeded [`ADAPTER_CAP`] —
    /// `adapters` is then a prefix, not the whole set.
    pub truncated: bool,
    /// Failure detail when the very first enumeration call
    /// (`idevnum = 0`) failed — `None` on success. A failed first
    /// call is a real degradation, so the record carries the error
    /// rather than silently reporting an empty list.
    pub error: Option<String>,
}

/// Enumerate the session's display adapters and their monitors —
/// see the module docs. Query-only; never mutates anything.
pub(crate) fn display_adapters() -> DisplayAdapters {
    platform::display_adapters()
}

#[cfg(windows)]
#[expect(unsafe_code)]
mod platform {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Graphics::Gdi::{
        DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE, DISPLAY_DEVICEW,
        EnumDisplayDevicesW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::EDD_GET_DEVICE_INTERFACE_NAME;

    use super::{DisplayAdapter, DisplayAdapters, DisplayMonitor};

    /// Safety cap on the adapter enumeration — a real session has
    /// a handful of adapters; hitting it records
    /// [`DisplayAdapters::truncated`] rather than looping on a
    /// corrupted enumeration.
    const ADAPTER_CAP: u32 = 64;
    /// Safety cap on the per-adapter monitor enumeration — a real
    /// adapter has a handful of outputs; hitting it drops the
    /// remainder rather than looping.
    const MONITOR_CAP: u32 = 16;

    pub(super) fn display_adapters() -> DisplayAdapters {
        let mut adapters = Vec::new();
        for index in 0..ADAPTER_CAP {
            let mut device = device();
            // SAFETY: `device` is a valid `cb`-sized out-param; a
            // null `lpDevice` enumerates adapters by `idevnum`.
            let ok = unsafe { EnumDisplayDevicesW(std::ptr::null(), index, &raw mut device, 0) };
            if ok == 0 {
                // `idevnum` past the last adapter is the normal
                // end-of-enumeration; the *first* call failing is a
                // real degradation — record its error detail
                // instead of reporting a silent empty list.
                return DisplayAdapters {
                    supported: true,
                    adapters,
                    error: (index == 0).then(|| last_error("EnumDisplayDevicesW")),
                    truncated: false,
                };
            }
            adapters.push(adapter(index, &device));
        }
        DisplayAdapters {
            supported: true,
            adapters,
            error: None,
            truncated: true,
        }
    }

    /// One adapter's record plus its monitor census — the second
    /// `EnumDisplayDevicesW` pass keyed by the adapter's own
    /// `DeviceName`.
    fn adapter(index: u32, device: &DISPLAY_DEVICEW) -> DisplayAdapter {
        DisplayAdapter {
            index,
            device_name: utf16_trimmed(&device.DeviceName),
            device_string: utf16_trimmed(&device.DeviceString),
            device_id: utf16_trimmed(&device.DeviceID),
            device_key: utf16_trimmed(&device.DeviceKey),
            state_flags: device.StateFlags,
            attached_to_desktop: device.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP != 0,
            primary: device.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE != 0,
            monitors: monitors(&device.DeviceName),
        }
    }

    /// Monitors attached to `device_name`'s adapter —
    /// `EDD_GET_DEVICE_INTERFACE_NAME` makes `DeviceName` carry the
    /// monitor's device interface name.
    fn monitors(device_name: &[u16]) -> Vec<DisplayMonitor> {
        // The adapter's `DeviceName` as a null-terminated buffer —
        // the API contract null-terminates it inside the fixed
        // array, but re-terminating here keeps the second pass
        // from ever reading past the field if that contract is
        // violated.
        let len = device_name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(device_name.len());
        let mut lp_device = device_name[..len].to_vec();
        lp_device.push(0);
        let mut monitors = Vec::new();
        for index in 0..MONITOR_CAP {
            let mut device = device();
            // SAFETY: `lp_device` is a live null-terminated UTF-16
            // buffer that outlives this call; `device` is a valid
            // `cb`-sized out-param.
            let ok = unsafe {
                EnumDisplayDevicesW(
                    lp_device.as_ptr(),
                    index,
                    &raw mut device,
                    EDD_GET_DEVICE_INTERFACE_NAME,
                )
            };
            if ok == 0 {
                break;
            }
            monitors.push(DisplayMonitor {
                index,
                device_name: utf16_trimmed(&device.DeviceName),
                device_string: utf16_trimmed(&device.DeviceString),
                device_id: utf16_trimmed(&device.DeviceID),
                device_key: utf16_trimmed(&device.DeviceKey),
                state_flags: device.StateFlags,
            });
        }
        monitors
    }

    /// A zeroed `DISPLAY_DEVICEW` with `cb` sized — the API
    /// contract requires `cb = sizeof(DISPLAY_DEVICEW)` before the
    /// call.
    fn device() -> DISPLAY_DEVICEW {
        DISPLAY_DEVICEW {
            cb: u32::try_from(size_of::<DISPLAY_DEVICEW>()).expect("DISPLAY_DEVICEW size fits u32"),
            ..DISPLAY_DEVICEW::default()
        }
    }

    /// The null-terminated UTF-16 string inside a fixed-size field
    /// — content after the first NUL is padding, never part of
    /// the string.
    fn utf16_trimmed(field: &[u16]) -> String {
        let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
        String::from_utf16_lossy(&field[..end])
    }

    /// `GetLastError` as a message string naming the failing call.
    fn last_error(call: &str) -> String {
        // SAFETY: `GetLastError` reads the calling thread's error
        // value; no preconditions.
        let code = unsafe { GetLastError() };
        if code == 0 {
            format!("{call} failed (no last-error recorded)")
        } else {
            format!("{call} failed (win32 error {code})")
        }
    }
}

/// Non-Windows stub — the field is stamped only inside a live
/// real-window session (Windows-only today), so this path is
/// unreachable in practice; it exists so the crate compiles on
/// every target and reports `supported: false` with empty fields,
/// never fabricated observations.
#[cfg(not(windows))]
mod platform {
    use super::DisplayAdapters;

    pub(super) fn display_adapters() -> DisplayAdapters {
        DisplayAdapters {
            supported: false,
            adapters: Vec::new(),
            truncated: false,
            error: None,
        }
    }
}
