//! `midi_wms_api_probe` — Issue #962 Slice C stage 1: compiled
//! `Windows.Devices.Midi2` (Windows MIDI Services client API)
//! reachability probe + transient BLOOP creation attempt.
//!
//! Manual diagnostic — never in CI. Run with
//! `cargo run -p ruviform-midi-device --example midi_wms_api_probe`
//! (equivalently `mise exec -- cargo run -p ruviform-midi-device
//! --example midi_wms_api_probe`). Captured output is evidence for
//! `evidence/rf962/` and the `virtual-endpoint-probe` manual workflow.
//!
//! The `windows`/`windows-sys` crates have no generated metadata for the
//! `Windows.Devices.Midi2` namespace, so this probe reaches the API via
//! raw `WinRT` (`RoInitialize` + `RoGetActivationFactory` /
//! `RoActivateInstance`) with hand-declared interface bindings. IIDs and
//! vtable slot order come from the published Microsoft MIDI IDL
//! (`microsoft/MIDI` `src/in-box/Client/WinRT/core/*.idl` +
//! `src/in-box/Inc/midi_sdk_idl_defs.h`); each declared vtable method is
//! called at least once so the binding is exercised rather than assumed.
//!
//! Staged observation — one machine-readable JSON line per observation
//! (`{"stage":"...","result":"...","detail":"...", ...}`):
//! - `presence` — client-component payload / `ActivatableClassId`
//!   registration evidence on this image.
//! - `ro_initialize` — `RoInitialize` (MTA).
//! - `activation` — `RoGetActivationFactory` / `RoActivateInstance` per
//!   runtime class (two legacy names probed as `REGDB_E_CLASSNOTREG`
//!   controls).
//! - `service_ping` — `MidiDiagnostics.PingService` service health.
//! - `session` — `MidiSession.Create` open + `Close`.
//! - `bloop_create` — `MidiBasicLoopbackManager` availability +
//!   `CreateTransientLoopback` (BLOOP = single-endpoint loopback only).
//! - `verdict` — summary of where reachability stopped.
//!
//! Absence is data: any stage failure is a measurement — later stages
//! are skipped and the probe still exits 0. A nonzero exit would mean
//! probe mechanism breakage, not an unreachable API.
//!
//! Contracts it keeps: BLOOP only — a LOOP A/B pair would mis-pair the
//! `tests/loopback.rs` name-match verifier into a false `Fail`
//! (Issue #952 risks). No WMS service config JSON reads/writes
//! (Microsoft-unsupported path — #952 design). No
//! `virtual-os-endpoint` evidence claims — `midisrv` is a service and
//! never presents itself as an OS endpoint.

use std::fmt::Write as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    print_environment();
    #[cfg(windows)]
    win::run();
    #[cfg(not(windows))]
    {
        println!("target-os: non-windows (WinMM + Windows.Devices.Midi2 are unavailable)");
        emit_json(
            "verdict",
            "notrun",
            &[("reason", "non_windows_host".to_string())],
        );
    }
    ExitCode::SUCCESS
}

/// Escapes `text` for embedding in a JSON string literal. The probe
/// emits hand-rolled JSON so it stays dependency-free.
fn json_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(escaped, "\\u{:04x}", u32::from(c));
            }
            c => escaped.push(c),
        }
    }
    escaped
}

/// Emits one machine-readable JSON observation line
/// (`{"stage":"...","result":"...","key":"value",...}`). `result`
/// vocabulary: `pass` / `fail` / `skipped` / `info` / `notrun`.
fn emit_json(stage: &str, result: &str, fields: &[(&str, String)]) {
    let mut line = format!(
        "{{\"stage\":\"{}\",\"result\":\"{}\"",
        json_escape(stage),
        json_escape(result)
    );
    for (key, value) in fields {
        let _ = write!(line, ",\"{}\":\"{}\"", json_escape(key), json_escape(value));
    }
    line.push('}');
    println!("{line}");
}

fn print_environment() {
    println!(
        "=== midi_wms_api_probe — compiled Windows.Devices.Midi2 reachability (Issue #962) ==="
    );
    println!("env-os: {}", std::env::consts::OS);
    println!("env-arch: {}", std::env::consts::ARCH);
}

/// Formats an `HRESULT` as `0x........` without `as` casts
/// (bit-preserving `i32` -> `u32` reinterpretation).
#[cfg(windows)]
fn hresult_hex(code: windows::core::HRESULT) -> String {
    format!("0x{:08X}", u32::from_ne_bytes(code.0.to_ne_bytes()))
}

/// Formats a `windows::core::Error` as the `hresult` + `detail` pair
/// used by every failure observation.
#[cfg(windows)]
fn error_fields(error: &windows::core::Error) -> (String, String) {
    (hresult_hex(error.code()), error.message())
}

#[cfg(windows)]
mod win {
    //! Raw `WinRT` bindings + staged probe body. All `unsafe` sites carry
    //! SAFETY comments (workspace policy: local allowance + per-site
    //! justification — precedent `examples/midi_endpoint_probe.rs`).

    #![expect(unsafe_code)]

    use std::ffi::c_void;
    use std::path::PathBuf;

    use windows::Foundation::IClosable;
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumKeyW, RegOpenKeyExW,
    };
    use windows::Win32::System::WinRT::{
        IActivationFactory, RO_INIT_MULTITHREADED, RoActivateInstance, RoGetActivationFactory,
        RoInitialize, RoUninitialize,
    };
    use windows::core::{
        Error, GUID, HRESULT, HSTRING, IInspectable, IInspectable_Vtbl, Interface, OutRef, Ref,
    };

    use crate::{emit_json, error_fields};

    // ------------------------------------------------------------------
    // Hand-declared `Windows.Devices.Midi2` WinRT bindings.
    //
    // The `#[interface]` proc macro cannot express `IInspectable` as a
    // parent (windows-core 0.62's `IInspectable_Vtbl::new` does not
    // follow the macro's `new<Identity, OFFSET>` + `matches` convention),
    // so the structs, `Interface` impls and `#[repr(C)]` vtables below are
    // declared by hand — the same shape `windows_core`'s `define_interface!`
    // produces. Every WinRT interface vtable embeds `IInspectable_Vtbl`
    // (QueryInterface/AddRef/Release/GetIids/GetRuntimeClassName/
    // GetTrustLevel at slots 0–5); the entries after `base__` follow the
    // IDL declaration order exactly and every one is invoked by the probe
    // so the binding is exercised end to end.
    //
    // IIDs come from `midi_sdk_idl_defs.h` (`8087b303-0519-c0de-31d1-…`;
    // `dd` = default interface, `ee` = statics interface).
    //
    // SAFETY (all `unsafe impl Interface` below): `$name` is
    // `repr(transparent)` over `IInspectable` (itself transparent over a
    // `NonNull<c_void>` chain), so the `as_raw`/`from_raw`/`Option<T>`
    // ABI conversions in `Interface`/`Type` are valid; `$vtbl` mirrors
    // the IInspectable base plus the IDL-ordered vtable for the
    // corresponding `Windows.Devices.Midi2` interface; and `IID` is the
    // interface's IDL UUID. QI/cast uses `T::IID`, and calls dispatch
    // through `Self::Vtable`, so a wrong `Vtable` or `IID` is the only
    // unsoundness source — both are pinned to the published IDL.
    // ------------------------------------------------------------------

    macro_rules! winrt_interface {
        ($(#[$meta:meta])* $name:ident, $vtbl:ident, $iid:literal, {
            $($method:ident : $sig:ty),* $(,)?
        }) => {
            $(#[$meta])*
            #[repr(transparent)]
            #[derive(Clone)]
            struct $name(IInspectable);

            #[repr(C)]
            struct $vtbl {
                base__: IInspectable_Vtbl,
                $($method: $sig,)*
            }

            // SAFETY: see the block comment above — `$name` is
            // transparent over `IInspectable`, `$vtbl` is the real
            // IDL-ordered vtable, `$iid` is the IDL UUID.
            unsafe impl Interface for $name {
                type Vtable = $vtbl;
                const IID: GUID = GUID::from_u128($iid);
            }
        };
    }

    // `Windows.Devices.Midi2.Diagnostics.IMidiDiagnosticsStatics`
    // (`UUID_IMidiDiagnosticsStatics` = `ee0030001000`; IDL order:
    // `DiagnosticsLoopbackAEndpointDeviceId`,
    // `DiagnosticsLoopbackBEndpointDeviceId`, `PingService`,
    // `PingServiceWithTimeout`).
    winrt_interface! {
        /// `Windows.Devices.Midi2.Diagnostics.IMidiDiagnosticsStatics`.
        IMidiDiagnosticsStatics, IMidiDiagnosticsStaticsVtable,
        0x8087b303_0519_c0de_31d1_ee0030001000, {
            diagnostics_loopback_a_endpoint_device_id: unsafe extern "system" fn(
                *mut c_void, OutRef<HSTRING>
            ) -> HRESULT,
            diagnostics_loopback_b_endpoint_device_id: unsafe extern "system" fn(
                *mut c_void, OutRef<HSTRING>
            ) -> HRESULT,
            ping_service: unsafe extern "system" fn(
                *mut c_void, u8, OutRef<IMidiServicePingResponseSummary>
            ) -> HRESULT,
            ping_service_with_timeout: unsafe extern "system" fn(
                *mut c_void, u8, u32, OutRef<IMidiServicePingResponseSummary>
            ) -> HRESULT,
        }
    }

    // `Windows.Devices.Midi2.IMidiServicePingResponseSummary`
    // (`UUID_IMidiServicePingResponseSummary` = `dd0030003000`; IDL
    // order: `Success`, `FailureReason`, `TotalPingRoundTripMidiClock`,
    // `AveragePingRoundTripMidiClock`).
    winrt_interface! {
        /// `Windows.Devices.Midi2.IMidiServicePingResponseSummary`.
        IMidiServicePingResponseSummary, IMidiServicePingResponseSummaryVtable,
        0x8087b303_0519_c0de_31d1_dd0030003000, {
            success: unsafe extern "system" fn(*mut c_void, OutRef<u8>) -> HRESULT,
            failure_reason: unsafe extern "system" fn(*mut c_void, OutRef<HSTRING>) -> HRESULT,
            total_ping_round_trip_midi_clock: unsafe extern "system" fn(
                *mut c_void, OutRef<u64>
            ) -> HRESULT,
            average_ping_round_trip_midi_clock: unsafe extern "system" fn(
                *mut c_void, OutRef<u64>
            ) -> HRESULT,
        }
    }

    // `Windows.Devices.Midi2.IMidiSessionStatics`
    // (`UUID_IMidiSessionStatics` = `ee0010009000`; IDL order:
    // `Create` — truncated after the only member the probe needs;
    // later statics slots are never dispatched).
    winrt_interface! {
        /// `Windows.Devices.Midi2.IMidiSessionStatics`.
        IMidiSessionStatics, IMidiSessionStaticsVtable,
        0x8087b303_0519_c0de_31d1_ee0010009000, {
            create: unsafe extern "system" fn(
                *mut c_void, Ref<HSTRING>, OutRef<IMidiSession>
            ) -> HRESULT,
        }
    }

    // `Windows.Devices.Midi2.IMidiSession`
    // (`UUID_IMidiSession` = `dd0010009000`; IDL order: `SessionId`,
    // `Name`, `IsOpen` — truncated; the remaining slots are never
    // dispatched).
    winrt_interface! {
        /// `Windows.Devices.Midi2.IMidiSession`.
        IMidiSession, IMidiSessionVtable,
        0x8087b303_0519_c0de_31d1_dd0010009000, {
            session_id: unsafe extern "system" fn(*mut c_void, OutRef<GUID>) -> HRESULT,
            name: unsafe extern "system" fn(*mut c_void, OutRef<HSTRING>) -> HRESULT,
            is_open: unsafe extern "system" fn(*mut c_void, OutRef<u8>) -> HRESULT,
        }
    }

    // `Windows.Devices.Midi2.Transports.Loopback.IMidiLoopbackManagerStatics`
    // (`UUID_IMidiLoopbackManagerStatics` = `ee00f0201000`; IDL order:
    // `IsTransportAvailable`, `TransportId` — truncated).
    // Observational only — the LOOP A/B managed loopback is never
    // created (the `loopback.rs` name-match verifier would report a
    // false `Fail` for a mis-paired drain).
    winrt_interface! {
        /// `Windows.Devices.Midi2.Transports.Loopback.IMidiLoopbackManagerStatics`.
        IMidiLoopbackManagerStatics, IMidiLoopbackManagerStaticsVtable,
        0x8087b303_0519_c0de_31d1_ee00f0201000, {
            is_transport_available: unsafe extern "system" fn(
                *mut c_void, OutRef<u8>
            ) -> HRESULT,
            transport_id: unsafe extern "system" fn(*mut c_void, OutRef<GUID>) -> HRESULT,
        }
    }

    // `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackManagerStatics`
    // (`UUID_IMidiBasicLoopbackManagerStatics` = `ee00f0101000`; IDL
    // order: `IsTransportAvailable`, `TransportId`,
    // `CreateTransientLoopback` — truncated; `RemoveTransientLoopback`,
    // `GetActiveLoopbackEntries` et al. are never dispatched).
    winrt_interface! {
        /// `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackManagerStatics`.
        IMidiBasicLoopbackManagerStatics, IMidiBasicLoopbackManagerStaticsVtable,
        0x8087b303_0519_c0de_31d1_ee00f0101000, {
            is_transport_available: unsafe extern "system" fn(
                *mut c_void, OutRef<u8>
            ) -> HRESULT,
            transport_id: unsafe extern "system" fn(*mut c_void, OutRef<GUID>) -> HRESULT,
            create_transient_loopback: unsafe extern "system" fn(
                *mut c_void,
                Ref<IMidiBasicLoopbackCreationConfig>,
                OutRef<IMidiBasicLoopbackCreationResponse>,
            ) -> HRESULT,
        }
    }

    // `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackEndpointDefinition`
    // (`UUID_IMidiBasicLoopbackEndpointDefinition` = `dd00f0108000`;
    // IDL order: `Name` get/set, `UniqueId` get/set, `Description`
    // get/set, `ImageFileName` get/set).
    winrt_interface! {
        /// `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackEndpointDefinition`.
        IMidiBasicLoopbackEndpointDefinition, IMidiBasicLoopbackEndpointDefinitionVtable,
        0x8087b303_0519_c0de_31d1_dd00f0108000, {
            name: unsafe extern "system" fn(*mut c_void, OutRef<HSTRING>) -> HRESULT,
            set_name: unsafe extern "system" fn(*mut c_void, Ref<HSTRING>) -> HRESULT,
            unique_id: unsafe extern "system" fn(*mut c_void, OutRef<HSTRING>) -> HRESULT,
            set_unique_id: unsafe extern "system" fn(*mut c_void, Ref<HSTRING>) -> HRESULT,
            description: unsafe extern "system" fn(*mut c_void, OutRef<HSTRING>) -> HRESULT,
            set_description: unsafe extern "system" fn(*mut c_void, Ref<HSTRING>) -> HRESULT,
            image_file_name: unsafe extern "system" fn(*mut c_void, OutRef<HSTRING>) -> HRESULT,
            set_image_file_name: unsafe extern "system" fn(
                *mut c_void, Ref<HSTRING>
            ) -> HRESULT,
        }
    }

    // `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackCreationConfig`
    // (`UUID_IMidiBasicLoopbackCreationConfig` = `dd00f0102000`; IDL
    // order: `AssociationId`, `EndpointDefinition` get/set — truncated;
    // `IsMuted`/`FeedbackProtection` keep constructor defaults:
    // unmuted, feedback protection on).
    winrt_interface! {
        /// `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackCreationConfig`.
        IMidiBasicLoopbackCreationConfig, IMidiBasicLoopbackCreationConfigVtable,
        0x8087b303_0519_c0de_31d1_dd00f0102000, {
            association_id: unsafe extern "system" fn(*mut c_void, OutRef<GUID>) -> HRESULT,
            endpoint_definition: unsafe extern "system" fn(
                *mut c_void, OutRef<IMidiBasicLoopbackEndpointDefinition>
            ) -> HRESULT,
            set_endpoint_definition: unsafe extern "system" fn(
                *mut c_void, Ref<IMidiBasicLoopbackEndpointDefinition>
            ) -> HRESULT,
        }
    }

    // `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackCreationResponse`
    // (`UUID_IMidiBasicLoopbackCreationResponse` = `dd00f0104000`; IDL
    // order: `Success`, `ErrorCode`, `ErrorMessage`,
    // `CreatedLoopbackEntry`).
    winrt_interface! {
        /// `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackCreationResponse`.
        IMidiBasicLoopbackCreationResponse, IMidiBasicLoopbackCreationResponseVtable,
        0x8087b303_0519_c0de_31d1_dd00f0104000, {
            success: unsafe extern "system" fn(*mut c_void, OutRef<u8>) -> HRESULT,
            error_code: unsafe extern "system" fn(*mut c_void, OutRef<i32>) -> HRESULT,
            error_message: unsafe extern "system" fn(*mut c_void, OutRef<HSTRING>) -> HRESULT,
            created_loopback_entry: unsafe extern "system" fn(
                *mut c_void, OutRef<IMidiBasicLoopbackEntry>
            ) -> HRESULT,
        }
    }

    // `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackEntry`
    // (`UUID_IMidiBasicLoopbackEntry` = `dd00f0107000`; IDL order:
    // `AssociationId`, `EndpointDeviceId`, `Name` — truncated).
    winrt_interface! {
        /// `Windows.Devices.Midi2.Transports.BasicLoopback.IMidiBasicLoopbackEntry`.
        IMidiBasicLoopbackEntry, IMidiBasicLoopbackEntryVtable,
        0x8087b303_0519_c0de_31d1_dd00f0107000, {
            association_id: unsafe extern "system" fn(*mut c_void, OutRef<GUID>) -> HRESULT,
            endpoint_device_id: unsafe extern "system" fn(
                *mut c_void, OutRef<HSTRING>
            ) -> HRESULT,
            name: unsafe extern "system" fn(*mut c_void, OutRef<HSTRING>) -> HRESULT,
        }
    }

    /// Shared call-shape for an IDL-declared method with a single
    /// `OutRef` out-parameter. The generated `unsafe fn` is only sound
    /// to call on a live object whose vtable matches `$name`'s declared
    /// `Vtable` — guaranteed by construction (objects only arrive via
    /// `RoGetActivationFactory`/`RoActivateInstance`/`cast`).
    macro_rules! out_getter {
        ($(#[$meta:meta])* $method:ident -> $ty:ty) => {
            $(#[$meta])*
            unsafe fn $method(&self) -> windows::core::Result<$ty> {
                let mut value = <$ty>::default();
                // SAFETY: vtable call on a live object; `value` is a
                // valid out-parameter slot written by the callee on Ok.
                unsafe { (self.vtable().$method)(self.as_raw(), OutRef::from(&mut value)).ok()? };
                Ok(value)
            }
        };
    }

    /// Shared call-shape for an IDL-declared setter taking a single
    /// `Ref` in-parameter.
    macro_rules! ref_setter {
        ($(#[$meta:meta])* $method:ident, $ty:ty) => {
            $(#[$meta])*
            unsafe fn $method(&self, value: &$ty) -> windows::core::Result<()> {
                // SAFETY: vtable call on a live object; `value` is a
                // valid borrowed in-parameter for the call's duration.
                unsafe { (self.vtable().$method)(self.as_raw(), Ref::from(value)).ok() }
            }
        };
    }

    impl IMidiDiagnosticsStatics {
        out_getter! {
            /// `DiagnosticsLoopbackAEndpointDeviceId` (slot 6).
            diagnostics_loopback_a_endpoint_device_id -> HSTRING
        }
        out_getter! {
            /// `DiagnosticsLoopbackBEndpointDeviceId` (slot 7).
            diagnostics_loopback_b_endpoint_device_id -> HSTRING
        }

        /// `PingService(ping_count)` (slot 8).
        unsafe fn ping_service(
            &self,
            ping_count: u8,
        ) -> windows::core::Result<Option<IMidiServicePingResponseSummary>> {
            let mut summary = None;
            // SAFETY: vtable call on a live statics object; `summary` is
            // a valid `Option<T>` out-parameter slot written on Ok.
            unsafe {
                (self.vtable().ping_service)(self.as_raw(), ping_count, OutRef::from(&mut summary))
                    .ok()?;
            };
            Ok(summary)
        }

        /// `PingServiceWithTimeout(ping_count, timeout_milliseconds)`
        /// (slot 9).
        unsafe fn ping_service_with_timeout(
            &self,
            ping_count: u8,
            timeout_milliseconds: u32,
        ) -> windows::core::Result<Option<IMidiServicePingResponseSummary>> {
            let mut summary = None;
            // SAFETY: vtable call on a live statics object; `summary` is
            // a valid `Option<T>` out-parameter slot written on Ok.
            unsafe {
                (self.vtable().ping_service_with_timeout)(
                    self.as_raw(),
                    ping_count,
                    timeout_milliseconds,
                    OutRef::from(&mut summary),
                )
                .ok()?;
            };
            Ok(summary)
        }
    }

    impl IMidiServicePingResponseSummary {
        out_getter! {
            /// `Success` (slot 6). ABI `BOOLEAN` = `u8`.
            success -> u8
        }
        out_getter! {
            /// `FailureReason` (slot 7).
            failure_reason -> HSTRING
        }
        out_getter! {
            /// `TotalPingRoundTripMidiClock` (slot 8).
            total_ping_round_trip_midi_clock -> u64
        }
        out_getter! {
            /// `AveragePingRoundTripMidiClock` (slot 9).
            average_ping_round_trip_midi_clock -> u64
        }
    }

    impl IMidiSessionStatics {
        /// `MidiSession.Create(session_name)` — the only statics member
        /// (slot 6).
        unsafe fn create(
            &self,
            session_name: &HSTRING,
        ) -> windows::core::Result<Option<IMidiSession>> {
            let mut session = None;
            // SAFETY: vtable call on a live statics object; `session`
            // is a valid `Option<T>` out-parameter slot written on Ok.
            unsafe {
                (self.vtable().create)(
                    self.as_raw(),
                    Ref::from(session_name),
                    OutRef::from(&mut session),
                )
                .ok()?;
            };
            Ok(session)
        }
    }

    impl IMidiSession {
        out_getter! {
            /// `SessionId` (slot 6).
            session_id -> GUID
        }
        out_getter! {
            /// `Name` (slot 7).
            name -> HSTRING
        }
        out_getter! {
            /// `IsOpen` (slot 8). ABI `BOOLEAN` = `u8`.
            is_open -> u8
        }
    }

    impl IMidiLoopbackManagerStatics {
        out_getter! {
            /// `IsTransportAvailable` (slot 6). ABI `BOOLEAN` = `u8`.
            is_transport_available -> u8
        }
        out_getter! {
            /// `TransportId` (slot 7).
            transport_id -> GUID
        }
    }

    impl IMidiBasicLoopbackManagerStatics {
        out_getter! {
            /// `IsTransportAvailable` (slot 6). ABI `BOOLEAN` = `u8`.
            is_transport_available -> u8
        }
        out_getter! {
            /// `TransportId` (slot 7).
            transport_id -> GUID
        }

        /// `CreateTransientLoopback(creation_config)` (slot 8) — the
        /// transient BLOOP provisioning attempt.
        unsafe fn create_transient_loopback(
            &self,
            creation_config: &IMidiBasicLoopbackCreationConfig,
        ) -> windows::core::Result<Option<IMidiBasicLoopbackCreationResponse>> {
            let mut response = None;
            // SAFETY: vtable call on a live statics object;
            // `creation_config` is a valid borrowed in-parameter and
            // `response` a valid `Option<T>` out-parameter slot.
            unsafe {
                (self.vtable().create_transient_loopback)(
                    self.as_raw(),
                    Ref::from(creation_config),
                    OutRef::from(&mut response),
                )
                .ok()?;
            };
            Ok(response)
        }
    }

    impl IMidiBasicLoopbackEndpointDefinition {
        out_getter! {
            /// `Name` get (slot 6).
            name -> HSTRING
        }
        ref_setter! {
            /// `Name` set (slot 7).
            set_name, HSTRING
        }
        out_getter! {
            /// `UniqueId` get (slot 8).
            unique_id -> HSTRING
        }
        ref_setter! {
            /// `UniqueId` set (slot 9).
            set_unique_id, HSTRING
        }
        out_getter! {
            /// `Description` get (slot 10).
            description -> HSTRING
        }
        ref_setter! {
            /// `Description` set (slot 11).
            set_description, HSTRING
        }
        out_getter! {
            /// `ImageFileName` get (slot 12).
            image_file_name -> HSTRING
        }
        ref_setter! {
            /// `ImageFileName` set (slot 13).
            set_image_file_name, HSTRING
        }
    }

    impl IMidiBasicLoopbackCreationConfig {
        out_getter! {
            /// `AssociationId` (slot 6).
            association_id -> GUID
        }

        /// `EndpointDefinition` get (slot 7).
        unsafe fn endpoint_definition(
            &self,
        ) -> windows::core::Result<Option<IMidiBasicLoopbackEndpointDefinition>> {
            let mut definition = None;
            // SAFETY: vtable call on a live object; `definition` is a
            // valid `Option<T>` out-parameter slot written on Ok.
            unsafe {
                (self.vtable().endpoint_definition)(self.as_raw(), OutRef::from(&mut definition))
                    .ok()?;
            };
            Ok(definition)
        }

        /// `EndpointDefinition` set (slot 8).
        unsafe fn set_endpoint_definition(
            &self,
            value: &IMidiBasicLoopbackEndpointDefinition,
        ) -> windows::core::Result<()> {
            // SAFETY: vtable call on a live object; `value` is a valid
            // borrowed in-parameter for the call's duration.
            unsafe { (self.vtable().set_endpoint_definition)(self.as_raw(), Ref::from(value)).ok() }
        }
    }

    impl IMidiBasicLoopbackCreationResponse {
        out_getter! {
            /// `Success` (slot 6). ABI `BOOLEAN` = `u8`.
            success -> u8
        }
        out_getter! {
            /// `ErrorCode` (slot 7). `WinRT` enum ABI = `i32`.
            error_code -> i32
        }
        out_getter! {
            /// `ErrorMessage` (slot 8).
            error_message -> HSTRING
        }

        /// `CreatedLoopbackEntry` (slot 9).
        unsafe fn created_loopback_entry(
            &self,
        ) -> windows::core::Result<Option<IMidiBasicLoopbackEntry>> {
            let mut entry = None;
            // SAFETY: vtable call on a live object; `entry` is a valid
            // `Option<T>` out-parameter slot written on Ok.
            unsafe {
                (self.vtable().created_loopback_entry)(self.as_raw(), OutRef::from(&mut entry))
                    .ok()?;
            };
            Ok(entry)
        }
    }

    impl IMidiBasicLoopbackEntry {
        out_getter! {
            /// `AssociationId` (slot 6).
            association_id -> GUID
        }
        out_getter! {
            /// `EndpointDeviceId` (slot 7).
            endpoint_device_id -> HSTRING
        }
        out_getter! {
            /// `Name` (slot 8).
            name -> HSTRING
        }
    }

    // ------------------------------------------------------------------
    // Probe body.
    // ------------------------------------------------------------------

    /// Session name used for `MidiSession.Create`.
    const SESSION_NAME: &str = "RvfProbe-WMS-API-962";
    /// BLOOP endpoint parameters — `Name`/`UniqueId`/`Description` from
    /// `MidiBasicLoopbackEndpointDefinition`. `UniqueId` must stay unique
    /// per live service session or creation returns `DuplicateUniqueId`.
    const BLOOP_ENDPOINT_NAME: &str = "RvfProbeBLOOP-API";
    const BLOOP_UNIQUE_ID: &str = "ruviform-rf962-bloop";
    const BLOOP_DESCRIPTION: &str =
        "ruviform Issue #962 transient BLOOP probe (compiled WinRT client)";

    /// Expected `MidiBasicLoopbackManager.TransportId` — the BLOOP
    /// transport GUID from `MidiBasicLoopbackManager.idl`.
    const BLOOP_TRANSPORT_ID: &str = "{10088473-9478-4E62-850B-3D2315E135B8}";
    /// Expected `MidiLoopbackManager.TransportId` — the LOOP A/B managed
    /// loopback transport GUID (observational; never created).
    const LOOP_TRANSPORT_ID: &str = "{942BF02D-93C0-4EA8-B03E-D51156A75E34}";

    /// `ERROR_NO_MORE_ITEMS` — normal `RegEnumKeyW` termination; any
    /// other enum failure is recorded as its own observation.
    const ERROR_NO_MORE_ITEMS: u32 = 259;

    pub fn run() {
        presence_stage();
        if !ro_initialize_stage() {
            emit_json(
                "verdict",
                "notrun",
                &[("reason", "ro_initialize_failed".to_string())],
            );
            return;
        }
        // Balances the `RoInitialize` call: COM stays initialized for
        // the whole probe body and is torn down on drop.
        let _uninit = RoUninitGuard;

        let activation = activation_stage();
        let mut service_pinged = false;
        let mut session_opened = false;
        let mut bloop_created = false;

        match &activation.diagnostics {
            Some(diagnostics) => service_pinged = service_ping_stage(diagnostics),
            None => emit_json(
                "service_ping",
                "skipped",
                &[("reason", "diagnostics_statics_unavailable".to_string())],
            ),
        }
        match &activation.session_statics {
            Some(session_statics) => session_opened = session_stage(session_statics),
            None => emit_json(
                "session",
                "skipped",
                &[("reason", "session_statics_unavailable".to_string())],
            ),
        }
        match &activation.bloop_statics {
            Some(bloop_statics) => bloop_created = bloop_stage(bloop_statics),
            None => emit_json(
                "bloop_create",
                "skipped",
                &[("reason", "bloop_statics_unavailable".to_string())],
            ),
        }

        emit_json(
            "verdict",
            if activation.any_reachable() {
                "pass"
            } else {
                "notrun"
            },
            &[
                ("api_reachable", activation.any_reachable().to_string()),
                ("furthest_stage", activation.furthest().to_string()),
                ("service_pinged", service_pinged.to_string()),
                ("session_opened", session_opened.to_string()),
                ("bloop_created", bloop_created.to_string()),
            ],
        );
    }

    /// RAII guard pairing `RoUninitialize` with the `RoInitialize` that
    /// already succeeded — COM is released whichever stage stops the run.
    struct RoUninitGuard;

    impl Drop for RoUninitGuard {
        fn drop(&mut self) {
            // SAFETY: pairs with the successful `RoInitialize` on this
            // thread; called exactly once when the guard drops.
            unsafe { RoUninitialize() };
        }
    }

    /// Formats a `windows::core::GUID` as `{...}` (its `Debug` output).
    fn guid_string(guid: &GUID) -> String {
        format!("{guid:?}")
    }

    /// Stage 1 — filesystem/registry evidence that the *client* half of
    /// Windows MIDI Services exists on this image. rf952 measured the
    /// service half (`midisrv` + `Midi2.*Transport.dll`); the compiled
    /// API additionally needs the `Windows.Devices.Midi2` proxy DLL /
    /// `winmd` and `ActivatableClassId` registrations, which this stage
    /// records before any activation is attempted.
    fn presence_stage() {
        let windir =
            std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        let system32 = windir.join("System32");

        for name in [
            "Windows.Devices.Midi2.dll",
            "Microsoft.Windows.Devices.Midi2.dll",
        ] {
            let path = system32.join(name);
            emit_json(
                "presence",
                if path.exists() { "pass" } else { "info" },
                &[
                    ("component", name.to_string()),
                    ("path", path.display().to_string()),
                    ("present", path.exists().to_string()),
                ],
            );
        }

        let winmd_dir = system32.join("WinMetadata");
        let midi2_winmds: Vec<String> = std::fs::read_dir(&winmd_dir).map_or_else(
            |_| Vec::new(),
            |entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.contains("Midi2"))
                    .collect()
            },
        );
        emit_json(
            "presence",
            if midi2_winmds.is_empty() {
                "info"
            } else {
                "pass"
            },
            &[
                ("component", "winmd".to_string()),
                ("count", midi2_winmds.len().to_string()),
                ("names", midi2_winmds.join(",")),
            ],
        );

        presence_registry_stage();
    }

    /// Enumerates `HKLM\SOFTWARE\Microsoft\WindowsRuntime\ActivatableClassId`
    /// for `Windows.Devices.Midi2.*` registrations — the strongest
    /// pre-activation signal that `RoGetActivationFactory` can succeed.
    fn presence_registry_stage() {
        let class_root = HSTRING::from("SOFTWARE\\Microsoft\\WindowsRuntime\\ActivatableClassId");
        let mut key = HKEY::default();
        // SAFETY: reads a well-known HKLM key with `KEY_READ`; the
        // returned handle is valid until `RegCloseKey` below.
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                &class_root,
                Some(0),
                KEY_READ,
                &raw mut key,
            )
        };
        if status.is_err() {
            let error = Error::from(status);
            let (hresult, detail) = error_fields(&error);
            emit_json(
                "presence",
                "info",
                &[
                    ("component", "activatable_class_id".to_string()),
                    (
                        "detail",
                        format!("registry open failed: {hresult} {detail}"),
                    ),
                ],
            );
            return;
        }

        let mut midi2_keys: Vec<String> = Vec::new();
        let mut enum_error: Option<String> = None;
        let mut index = 0u32;
        let mut buf = [0u16; 512];
        loop {
            // SAFETY: `key` is the open handle above; `buf` is a valid
            // writable buffer passed as a length-carrying slice.
            let status = unsafe { RegEnumKeyW(key, index, Some(buf.as_mut_slice())) };
            if status.is_err() {
                if status.0 != ERROR_NO_MORE_ITEMS {
                    let error = Error::from(status);
                    let (hresult, detail) = error_fields(&error);
                    enum_error = Some(format!("{hresult} {detail}"));
                }
                break;
            }
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let name = String::from_utf16_lossy(&buf[..end]);
            if name.starts_with("Windows.Devices.Midi2") {
                midi2_keys.push(name);
            }
            index += 1;
        }
        // SAFETY: `key` was opened above and is closed exactly once here.
        let _ = unsafe { RegCloseKey(key) };

        emit_json(
            "presence",
            if midi2_keys.is_empty() {
                "info"
            } else {
                "pass"
            },
            &[
                ("component", "activatable_class_id".to_string()),
                ("count", midi2_keys.len().to_string()),
                ("classes", midi2_keys.join(",")),
                ("enum_error", enum_error.unwrap_or_default()),
            ],
        );
    }

    /// Stage 2 — `RoInitialize` on the calling thread (MTA).
    fn ro_initialize_stage() -> bool {
        // SAFETY: called once on this thread before any other WinRT
        // call; the balancing `RoUninitialize` runs on drop of the guard
        // created by the caller.
        match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
            Ok(()) => {
                println!("ro-initialize: OK (MTA)");
                emit_json("ro_initialize", "pass", &[("apartment", "mta".to_string())]);
                true
            }
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                println!("ro-initialize: FAILED {hresult} {detail}");
                emit_json(
                    "ro_initialize",
                    "fail",
                    &[("hresult", hresult), ("detail", detail)],
                );
                false
            }
        }
    }

    /// Factories acquired during stage 3 — a `None` entry means the
    /// owning stage is skipped (absence is data, not failure).
    struct Activation {
        diagnostics: Option<IMidiDiagnosticsStatics>,
        session_statics: Option<IMidiSessionStatics>,
        bloop_statics: Option<IMidiBasicLoopbackManagerStatics>,
    }

    impl Activation {
        fn any_reachable(&self) -> bool {
            self.diagnostics.is_some()
                || self.session_statics.is_some()
                || self.bloop_statics.is_some()
        }

        fn furthest(&self) -> &'static str {
            if self.bloop_statics.is_some() {
                "bloop_statics"
            } else if self.session_statics.is_some() {
                "session_statics"
            } else if self.diagnostics.is_some() {
                "diagnostics_statics"
            } else {
                "activation"
            }
        }
    }

    /// `RoGetActivationFactory(class, T::IID)` — records `hresult` +
    /// `detail` on failure so the artifact distinguishes metadata loss
    /// (`RO_E_METADATA_NAME_NOT_FOUND`), absent registration
    /// (`REGDB_E_CLASSNOTREG`) and permission failures (`E_ACCESSDENIED`).
    fn factory<T: Interface>(class: &str) -> Option<T> {
        let name = HSTRING::from(class);
        // SAFETY: `name` is a valid HSTRING; on success the returned
        // pointer implements `T` by the factory contract.
        match unsafe { RoGetActivationFactory::<T>(&name) } {
            Ok(factory) => {
                emit_json(
                    "activation",
                    "pass",
                    &[
                        ("class", class.to_string()),
                        ("iid", format!("{:?}", T::IID)),
                    ],
                );
                Some(factory)
            }
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "activation",
                    "fail",
                    &[
                        ("class", class.to_string()),
                        ("iid", format!("{:?}", T::IID)),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                None
            }
        }
    }

    /// `RoActivateInstance(class)` for runtime classes that expose a
    /// parameterless `IActivationFactory` ctor — the BLOOP endpoint
    /// definition and creation config.
    fn activate_instance(class: &str) -> Option<IInspectable> {
        let name = HSTRING::from(class);
        // SAFETY: `name` is a valid HSTRING; on success the returned
        // object is a freshly activated instance.
        match unsafe { RoActivateInstance(&name) } {
            Ok(object) => {
                emit_json("activation", "pass", &[("class", class.to_string())]);
                Some(object)
            }
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "activation",
                    "fail",
                    &[
                        ("class", class.to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                None
            }
        }
    }

    /// Stage 3 — activation factory acquisition per runtime class. The
    /// two legacy names (`MidiService`, `MidiLoopbackManager` under
    /// `Windows.Devices.Midi2` — the names the `midi` console docs use)
    /// are probed as negative controls: rf952 showed they do not exist
    /// as runtime classes, and `REGDB_E_CLASSNOTREG` here pins that down
    /// at the `WinRT` ABI layer rather than PowerShell type resolution.
    fn activation_stage() -> Activation {
        println!("activation: requesting statics/factories");

        for class in [
            "Windows.Devices.Midi2.MidiService",
            "Windows.Devices.Midi2.MidiLoopbackManager",
        ] {
            let name = HSTRING::from(class);
            // SAFETY: read-only activation probe; failure is the
            // expected measurement for these control names.
            let result = unsafe { RoGetActivationFactory::<IActivationFactory>(&name) };
            let (hresult, detail) = match &result {
                Ok(_) => (String::new(), "activated (unexpected)".to_string()),
                Err(error) => error_fields(error),
            };
            emit_json(
                "activation",
                "info",
                &[
                    ("class", class.to_string()),
                    ("control", "legacy_class_name".to_string()),
                    ("hresult", hresult),
                    ("detail", detail),
                ],
            );
        }

        let diagnostics =
            factory::<IMidiDiagnosticsStatics>("Windows.Devices.Midi2.Diagnostics.MidiDiagnostics");
        let session_statics = factory::<IMidiSessionStatics>("Windows.Devices.Midi2.MidiSession");

        // LOOP A/B manager — observational only, never used to create.
        if let Some(loopback_statics) = factory::<IMidiLoopbackManagerStatics>(
            "Windows.Devices.Midi2.Transports.Loopback.MidiLoopbackManager",
        ) {
            // SAFETY: vtable getters on a live statics object acquired
            // above; each writes its out-parameter on Ok.
            let reading = unsafe {
                loopback_statics
                    .is_transport_available()
                    .and_then(|available| {
                        loopback_statics
                            .transport_id()
                            .map(|transport| (available, transport))
                    })
            };
            match reading {
                Ok((available, transport)) => emit_json(
                    "bloop_transport",
                    "info",
                    &[
                        ("manager", "loopback_ab".to_string()),
                        ("available", (available != 0).to_string()),
                        ("transport_id", guid_string(&transport)),
                        ("transport_id_expected", LOOP_TRANSPORT_ID.to_string()),
                    ],
                ),
                Err(error) => {
                    let (hresult, detail) = error_fields(&error);
                    emit_json(
                        "bloop_transport",
                        "fail",
                        &[
                            ("manager", "loopback_ab".to_string()),
                            ("hresult", hresult),
                            ("detail", detail),
                        ],
                    );
                }
            }
        }

        let bloop_statics = factory::<IMidiBasicLoopbackManagerStatics>(
            "Windows.Devices.Midi2.Transports.BasicLoopback.MidiBasicLoopbackManager",
        );

        Activation {
            diagnostics,
            session_statics,
            bloop_statics,
        }
    }

    /// Reads a `MidiServicePingResponseSummary` — all four vtable slots
    /// exercised — and emits it as one observation line.
    fn emit_ping_summary(
        call: &str,
        summary: &IMidiServicePingResponseSummary,
    ) -> windows::core::Result<()> {
        // SAFETY: vtable getters on a live response object returned by
        // `PingService*`; each writes its out-parameter on Ok.
        let (success, failure_reason, total_clock, average_clock) = unsafe {
            let success = summary.success()?;
            let failure_reason = summary.failure_reason()?;
            let total_clock = summary.total_ping_round_trip_midi_clock()?;
            let average_clock = summary.average_ping_round_trip_midi_clock()?;
            (success, failure_reason, total_clock, average_clock)
        };
        emit_json(
            "service_ping",
            if success != 0 { "pass" } else { "fail" },
            &[
                ("call", call.to_string()),
                ("success", (success != 0).to_string()),
                ("failure_reason", failure_reason.to_string()),
                ("total_ping_clock", total_clock.to_string()),
                ("average_ping_clock", average_clock.to_string()),
            ],
        );
        Ok(())
    }

    /// Stage 4 — `MidiDiagnostics` service health: diagnostics loopback
    /// endpoint ids plus both `PingService` overloads.
    fn service_ping_stage(diagnostics: &IMidiDiagnosticsStatics) -> bool {
        // SAFETY: vtable getters on a live statics object acquired in
        // stage 3; each writes its out-parameter on Ok.
        let ids = unsafe {
            diagnostics
                .diagnostics_loopback_a_endpoint_device_id()
                .and_then(|a| {
                    diagnostics
                        .diagnostics_loopback_b_endpoint_device_id()
                        .map(|b| (a, b))
                })
        };
        let mut ok = true;
        match &ids {
            Ok((loopback_a, loopback_b)) => emit_json(
                "service_ping",
                "info",
                &[
                    ("call", "diagnostics_endpoints".to_string()),
                    ("loopback_a_id", loopback_a.to_string()),
                    ("loopback_b_id", loopback_b.to_string()),
                ],
            ),
            Err(error) => {
                let (hresult, detail) = error_fields(error);
                emit_json(
                    "service_ping",
                    "fail",
                    &[
                        ("call", "diagnostics_endpoints".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                ok = false;
            }
        }

        for (call, timeout) in [
            ("ping_service", None),
            ("ping_service_with_timeout", Some(15_000)),
        ] {
            // SAFETY: vtable call on a live statics object; the summary
            // out-parameter is a valid slot written on Ok.
            let result = unsafe {
                match timeout {
                    Some(ms) => diagnostics.ping_service_with_timeout(3, ms),
                    None => diagnostics.ping_service(3),
                }
            };
            match result {
                Ok(Some(summary)) => {
                    if emit_ping_summary(call, &summary).is_err() {
                        ok = false;
                    }
                }
                Ok(None) => {
                    emit_json(
                        "service_ping",
                        "fail",
                        &[
                            ("call", call.to_string()),
                            ("detail", "ping returned no summary".to_string()),
                        ],
                    );
                    ok = false;
                }
                Err(error) => {
                    let (hresult, detail) = error_fields(&error);
                    emit_json(
                        "service_ping",
                        "fail",
                        &[
                            ("call", call.to_string()),
                            ("hresult", hresult),
                            ("detail", detail),
                        ],
                    );
                    ok = false;
                }
            }
        }
        ok
    }

    /// Stage 5 — `MidiSession.Create` + `Close` (session open
    /// reachability; no endpoints are attached).
    fn session_stage(session_statics: &IMidiSessionStatics) -> bool {
        // SAFETY: vtable call on a live statics object; the session
        // out-parameter is a valid `Option<T>` slot written on Ok.
        let session = match unsafe { session_statics.create(&HSTRING::from(SESSION_NAME)) } {
            Ok(Some(session)) => session,
            Ok(None) => {
                emit_json(
                    "session",
                    "fail",
                    &[("detail", "Create returned Ok with no session".to_string())],
                );
                return false;
            }
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "session",
                    "fail",
                    &[
                        ("call", "Create".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                return false;
            }
        };

        // SAFETY: vtable getters on a live session object returned by
        // `Create`; each writes its out-parameter on Ok.
        let fields = unsafe {
            session.session_id().and_then(|session_id| {
                session
                    .name()
                    .and_then(|name| session.is_open().map(|is_open| (session_id, name, is_open)))
            })
        };
        match fields {
            Ok((session_id, name, is_open)) => emit_json(
                "session",
                "pass",
                &[
                    ("call", "Create".to_string()),
                    ("session_id", guid_string(&session_id)),
                    ("name", name.to_string()),
                    ("is_open", (is_open != 0).to_string()),
                ],
            ),
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "session",
                    "fail",
                    &[
                        ("call", "read_fields".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
            }
        }

        // The session owns service-side resources — close it through
        // `IClosable` so no session leaks past the probe process.
        if let Ok(closable) = session.cast::<IClosable>()
            && let Err(error) = closable.Close()
        {
            let (hresult, detail) = error_fields(&error);
            emit_json(
                "session",
                "info",
                &[
                    ("call", "Close".to_string()),
                    ("hresult", hresult),
                    ("detail", detail),
                ],
            );
        }
        true
    }

    /// Stage 6 — BLOOP (basic loopback, single endpoint) creation.
    /// `MidiBasicLoopbackManager` availability + transport id, then the
    /// transient creation chain: endpoint definition -> creation config
    /// -> `CreateTransientLoopback` -> response fields -> entry.
    fn bloop_stage(bloop_statics: &IMidiBasicLoopbackManagerStatics) -> bool {
        // SAFETY: vtable getters on a live statics object acquired in
        // stage 3; each writes its out-parameter on Ok.
        let reading = unsafe {
            bloop_statics
                .is_transport_available()
                .and_then(|available| {
                    bloop_statics
                        .transport_id()
                        .map(|transport| (available, transport))
                })
        };
        let available = match reading {
            Ok((available, transport)) => {
                emit_json(
                    "bloop_transport",
                    "info",
                    &[
                        ("manager", "basic_loopback".to_string()),
                        ("available", (available != 0).to_string()),
                        ("transport_id", guid_string(&transport)),
                        ("transport_id_expected", BLOOP_TRANSPORT_ID.to_string()),
                    ],
                );
                available
            }
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "bloop_transport",
                    "fail",
                    &[
                        ("manager", "basic_loopback".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                return false;
            }
        };
        if available == 0 {
            emit_json(
                "bloop_create",
                "skipped",
                &[("reason", "transport_unavailable".to_string())],
            );
            return false;
        }

        let Some(definition) = activate_endpoint_definition() else {
            return false;
        };
        let Some(config) = activate_creation_config(&definition) else {
            return false;
        };
        create_transient_loopback(bloop_statics, &config)
    }

    /// `RoActivateInstance` + set/get round-trip for
    /// `MidiBasicLoopbackEndpointDefinition`. The getter read-back
    /// verifies the hand-declared vtable order end to end (a wrong slot
    /// would read a different property or fail outright).
    fn activate_endpoint_definition() -> Option<IMidiBasicLoopbackEndpointDefinition> {
        let object = activate_instance(
            "Windows.Devices.Midi2.Transports.BasicLoopback.MidiBasicLoopbackEndpointDefinition",
        )?;
        let definition: IMidiBasicLoopbackEndpointDefinition = match object.cast() {
            Ok(definition) => definition,
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "bloop_create",
                    "fail",
                    &[
                        ("call", "endpoint_definition_cast".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                return None;
            }
        };

        for (property, value) in [
            ("set_name", BLOOP_ENDPOINT_NAME),
            ("set_unique_id", BLOOP_UNIQUE_ID),
            ("set_description", BLOOP_DESCRIPTION),
        ] {
            // SAFETY: vtable setter on a live definition object;
            // `value` is a valid HSTRING in-parameter.
            let result = unsafe {
                let value = HSTRING::from(value);
                match property {
                    "set_name" => definition.set_name(&value),
                    "set_unique_id" => definition.set_unique_id(&value),
                    _ => definition.set_description(&value),
                }
            };
            if let Err(error) = result {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "bloop_create",
                    "fail",
                    &[
                        ("call", property.to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                return None;
            }
        }

        // SAFETY: vtable getters on the same object — read-back of the
        // values written above. `set_image_file_name` is exercised with
        // the read-back value (a write-back of identical state) so every
        // declared slot is invoked once.
        let readback = unsafe {
            definition.name().and_then(|name| {
                definition.unique_id().and_then(|unique_id| {
                    definition.description().and_then(|description| {
                        definition.image_file_name().and_then(|image| {
                            definition
                                .set_image_file_name(&image)
                                .map(|()| (name, unique_id, description, image))
                        })
                    })
                })
            })
        };
        match readback {
            Ok((name, unique_id, description, image)) => {
                emit_json(
                    "bloop_create",
                    "pass",
                    &[
                        ("call", "endpoint_definition".to_string()),
                        ("name", name.to_string()),
                        ("unique_id", unique_id.to_string()),
                        ("description", description.to_string()),
                        ("image_file_name", image.to_string()),
                    ],
                );
                Some(definition)
            }
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "bloop_create",
                    "fail",
                    &[
                        ("call", "endpoint_definition_readback".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                None
            }
        }
    }

    /// `RoActivateInstance` + `SetEndpointDefinition` +
    /// `AssociationId`/definition read-back for
    /// `MidiBasicLoopbackCreationConfig`.
    fn activate_creation_config(
        definition: &IMidiBasicLoopbackEndpointDefinition,
    ) -> Option<IMidiBasicLoopbackCreationConfig> {
        let object = activate_instance(
            "Windows.Devices.Midi2.Transports.BasicLoopback.MidiBasicLoopbackCreationConfig",
        )?;
        let config: IMidiBasicLoopbackCreationConfig = match object.cast() {
            Ok(config) => config,
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "bloop_create",
                    "fail",
                    &[
                        ("call", "creation_config_cast".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                return None;
            }
        };

        // SAFETY: vtable calls on a live config object; `definition` is
        // a valid borrowed in-parameter.
        let configured = unsafe {
            config.association_id().and_then(|association| {
                config.set_endpoint_definition(definition).and_then(|()| {
                    config
                        .endpoint_definition()
                        .map(|readback| (association, readback))
                })
            })
        };
        match configured {
            Ok((association, readback)) => {
                // SAFETY: vtable getter on the returned definition.
                let readback_name = readback
                    .as_ref()
                    .and_then(|definition| unsafe { definition.name() }.ok());
                emit_json(
                    "bloop_create",
                    "pass",
                    &[
                        ("call", "creation_config".to_string()),
                        ("association_id", guid_string(&association)),
                        (
                            "endpoint_definition_name",
                            readback_name.map_or_else(String::new, |name| name.to_string()),
                        ),
                    ],
                );
                Some(config)
            }
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "bloop_create",
                    "fail",
                    &[
                        ("call", "creation_config".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                None
            }
        }
    }

    /// `CreateTransientLoopback` — the provisioning attempt — followed
    /// by the response fields and (on success) the created entry's
    /// `EndpointDeviceId`.
    fn create_transient_loopback(
        bloop_statics: &IMidiBasicLoopbackManagerStatics,
        config: &IMidiBasicLoopbackCreationConfig,
    ) -> bool {
        // SAFETY: vtable call on a live statics object; `config` is a
        // valid borrowed in-parameter.
        let response = match unsafe { bloop_statics.create_transient_loopback(config) } {
            Ok(Some(response)) => response,
            Ok(None) => {
                emit_json(
                    "bloop_create",
                    "fail",
                    &[(
                        "detail",
                        "CreateTransientLoopback returned Ok with no response".to_string(),
                    )],
                );
                return false;
            }
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "bloop_create",
                    "fail",
                    &[
                        ("call", "CreateTransientLoopback".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                return false;
            }
        };

        // SAFETY: vtable getters on a live response object; each writes
        // its out-parameter on Ok.
        let fields = unsafe {
            response.success().and_then(|success| {
                response.error_code().and_then(|error_code| {
                    response.error_message().and_then(|error_message| {
                        response
                            .created_loopback_entry()
                            .map(|entry| (success, error_code, error_message, entry))
                    })
                })
            })
        };
        let (success, error_code, error_message, entry) = match fields {
            Ok(fields) => fields,
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "bloop_create",
                    "fail",
                    &[
                        ("call", "response_fields".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
                return false;
            }
        };
        emit_json(
            "bloop_create",
            if success != 0 { "pass" } else { "fail" },
            &[
                ("call", "CreateTransientLoopback".to_string()),
                ("success", (success != 0).to_string()),
                ("error_code", format!("0x{error_code:08X}")),
                ("error_message", error_message.to_string()),
            ],
        );
        if success == 0 {
            return false;
        }

        let Some(entry) = entry else { return true };
        // SAFETY: vtable getters on a live entry object; each writes
        // its out-parameter on Ok.
        let entry_fields = unsafe {
            entry.association_id().and_then(|association| {
                entry.endpoint_device_id().and_then(|endpoint_id| {
                    entry.name().map(|name| (association, endpoint_id, name))
                })
            })
        };
        match entry_fields {
            Ok((association, endpoint_id, name)) => emit_json(
                "bloop_create",
                "pass",
                &[
                    ("call", "created_entry".to_string()),
                    ("association_id", guid_string(&association)),
                    ("endpoint_device_id", endpoint_id.to_string()),
                    ("name", name.to_string()),
                ],
            ),
            Err(error) => {
                let (hresult, detail) = error_fields(&error);
                emit_json(
                    "bloop_create",
                    "fail",
                    &[
                        ("call", "entry_fields".to_string()),
                        ("hresult", hresult),
                        ("detail", detail),
                    ],
                );
            }
        }
        true
    }
}
