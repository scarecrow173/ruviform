//! `endpoint_probe` — Issue #629 first-slice capability probe for
//! Windows virtual audio endpoints.
//!
//! Manual diagnostic — never in CI. Run with
//! `cargo run -p ruviform-audio-device --example endpoint_probe`
//! (equivalently `mise exec -- cargo run -p ruviform-audio-device
//! --example endpoint_probe`). The captured output is survey evidence
//! for `docs/discovery/issue-629-virtual-endpoint-survey.md` and
//! `evidence/rf629/`.
//!
//! What it does: reports OS facts, enumerates every WASAPI endpoint in
//! all device states through `IMMDeviceEnumerator`, joins each
//! endpoint with the friendly name the production port surface
//! (`ruviform_audio_device` → cpal) reports, queries the activated
//! `IAudioClient`'s mix format / `IsFormatSupported` spot-checks /
//! device period / stream latency, and applies a clearly labelled
//! name heuristic to flag *candidate* virtual endpoints.
//!
//! What it never does: claim `virtual-os-endpoint` evidence. Per the
//! Issue #627 boundary-A rule, enumeration and format queries are not
//! endpoint streaming proof — the product backend must actually open
//! and stream through the endpoint, which is the next slice. When no
//! virtual endpoint is provisioned, every virtual-endpoint case this
//! probe feeds is reported `NotRun` (fail-closed), never `Pass`.

use std::process::ExitCode;

fn main() -> ExitCode {
    println!("endpoint_probe — Issue #629 virtual-endpoint capability probe (WASAPI/MMDevice)");
    println!("manual diagnostic; the output is survey evidence, not a test verdict");
    println!();
    print_environment();
    #[cfg(windows)]
    win::run();
    #[cfg(not(windows))]
    {
        println!();
        println!("== verdict ==");
        println!("  wasapi enumeration: NotRun — this probe requires Windows");
        println!("  virtual-endpoint cases: NotRun — no Windows WASAPI on this host");
    }
    ExitCode::SUCCESS
}

/// Prints the OS facts every Case record needs (`os`/`arch`/`family`);
/// the Windows build string is added by the platform module.
fn print_environment() {
    println!("== environment ==");
    println!(
        "  os={} arch={} family={}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    );
    #[cfg(windows)]
    win::print_os_build();
}

#[cfg(windows)]
mod win {
    //! Windows/WASAPI probe body — the only place Win32 FFI appears in
    //! this example. Every `unsafe` block is individually justified at
    //! its site (workspace policy: local allowance + SAFETY comments —
    //! precedent `src/wasapi_input.rs`).

    #![expect(unsafe_code)]

    use std::process::Command;

    use ruviform_audio_core::device::DeviceEnumerator;
    use ruviform_audio_device::default_host;
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::Media::Audio::{
        self, AUDCLNT_SHAREMODE_SHARED, DEVICE_STATE, EDataFlow, IAudioClient, IMMDevice,
        IMMDeviceEnumerator, WAVEFORMATEX,
    };
    use windows::Win32::System::Com::{self, CLSCTX_ALL};

    /// `DEVICE_STATE` mask covering every endpoint state — the probe
    /// reports disabled/not-present/unplugged endpoints too, because
    /// virtual-endpoint hotplug testing needs to observe them.
    const ALL_STATES: DEVICE_STATE = DEVICE_STATE(Audio::DEVICE_STATEMASK_ALL);

    /// `IsFormatSupported` spot-checks run against every endpoint: the
    /// mix-format query alone does not prove the endpoint accepts the
    /// F32 formats the adapter negotiates.
    struct FormatProbe {
        label: &'static str,
        tag: u16,
        channels: u16,
        rate_hz: u32,
        bits: u16,
    }

    /// Standard shared-mode candidates: F32 (the adapter's negotiated
    /// format) at 48 kHz / 44.1 kHz, and PCM s16 at 48 kHz as a
    /// non-float control.
    const FORMAT_PROBES: &[FormatProbe] = &[
        FormatProbe {
            label: "f32/2ch/48k",
            tag: 3,
            channels: 2,
            rate_hz: 48_000,
            bits: 32,
        },
        FormatProbe {
            label: "f32/2ch/44.1k",
            tag: 3,
            channels: 2,
            rate_hz: 44_100,
            bits: 32,
        },
        FormatProbe {
            label: "s16/2ch/48k",
            tag: 1,
            channels: 2,
            rate_hz: 48_000,
            bits: 16,
        },
    ];

    /// Name fragments that flag a *candidate* virtual audio endpoint.
    /// This is a detection aid for the survey, not a provisioning
    /// decision — Issue #629 forbids fixing endpoint names/installers
    /// up front, so every hit stays `candidate (unconfirmed)` until a
    /// human confirms the endpoint on the dedicated runner.
    const VIRTUAL_NAME_HINTS: &[&str] = &[
        "vb-cable",
        "vb audio",
        "vb-audio",
        "cable input",
        "cable output",
        "hi-fi cable",
        "hifi-cable",
        "virtual audio cable",
        "voicemeeter",
    ];

    /// One enumerated endpoint plus everything the probe could query.
    struct EndpointRow {
        flow: &'static str,
        state: DEVICE_STATE,
        id: String,
        friendly_name: Option<String>,
        is_virtual_candidate: bool,
        mix_format: Option<String>,
        format_results: Vec<String>,
        device_period: Option<(i64, i64)>,
        stream_latency: Option<i64>,
        notes: Vec<String>,
    }

    /// `(cpal device-id rendering, friendly name, direction)` rows
    /// from the production port surface — used to attach friendly
    /// names to raw `IMMDevice` endpoints.
    type PortNames = Vec<(String, String, &'static str)>;

    /// Runs the whole probe and prints the report.
    pub(super) fn run() {
        println!();
        println!("== WASAPI MMDevice enumeration ==");
        match enumerate_all() {
            Ok(rows) => {
                for row in &rows {
                    print_endpoint(row);
                }
                println!();
                print_verdict(&rows);
            }
            Err(error) => println!("  enumeration failed: {error}"),
        }
        println!();
        println!("== production port view (ruviform_audio_device) ==");
        print_port_view();
    }

    /// Prints the Windows version string via `cmd /c ver` — the raw
    /// output is recorded verbatim for the evidence file.
    /// `GetVersionExW`/`GetVersion` would need
    /// `Win32_System_SystemInformation`, which is outside the crate's
    /// enabled `windows` features — recorded as a probe gap in the
    /// discovery doc rather than worked around silently.
    pub(super) fn print_os_build() {
        match Command::new("cmd").args(["/c", "ver"]).output() {
            Ok(output) => {
                let text = String::from_utf8_lossy(&output.stdout);
                println!("  os_version: {}", text.trim());
            }
            Err(error) => println!("  os_version: NotRun — `cmd /c ver` failed: {error}"),
        }
    }

    /// Enumerates render + capture endpoints in every state and
    /// describes each one.
    fn enumerate_all() -> Result<Vec<EndpointRow>, String> {
        ensure_com()?;
        let enumerator = create_enumerator()?;
        let names = port_names();
        let mut rows = Vec::new();
        for (flow, label) in [(Audio::eRender, "render"), (Audio::eCapture, "capture")] {
            for device in enum_devices(&enumerator, flow)? {
                rows.push(describe_endpoint(&device, label, &names));
            }
        }
        Ok(rows)
    }

    /// Ensures COM is initialised on this thread. Mirrors the adapter's
    /// `ensure_com`: `S_OK`/`S_FALSE` mean usable and
    /// `RPC_E_CHANGED_MODE` (COM already live in another apartment) is
    /// usable through marshalling; anything else is a hard failure.
    fn ensure_com() -> Result<(), String> {
        // SAFETY: standard COM apartment initialisation on the probe's
        // single control thread; the result is checked, not assumed.
        let hr = unsafe { Com::CoInitializeEx(None, Com::COINIT_APARTMENTTHREADED) };
        if hr.is_ok() || hr == RPC_E_CHANGED_MODE {
            Ok(())
        } else {
            Err(format!("CoInitializeEx failed: {hr}"))
        }
    }

    /// Creates the `MMDeviceEnumerator` COM object.
    fn create_enumerator() -> Result<IMMDeviceEnumerator, String> {
        // SAFETY: `CoCreateInstance` on the control path with COM
        // initialised; the enumerator is a fresh COM reference owned
        // by the returned smart pointer.
        unsafe {
            Com::CoCreateInstance::<_, IMMDeviceEnumerator>(
                &Audio::MMDeviceEnumerator,
                None,
                CLSCTX_ALL,
            )
        }
        .map_err(|error| format!("CoCreateInstance(MMDeviceEnumerator) failed: {error}"))
    }

    /// Enumerates every endpoint of `flow` in every state.
    fn enum_devices(
        enumerator: &IMMDeviceEnumerator,
        flow: EDataFlow,
    ) -> Result<Vec<IMMDevice>, String> {
        // SAFETY: `enumerator` is a live COM object; the returned
        // collection is a fresh COM reference.
        let collection = unsafe { enumerator.EnumAudioEndpoints(flow, ALL_STATES) }
            .map_err(|error| format!("EnumAudioEndpoints failed: {error}"))?;
        // SAFETY: `GetCount`/`Item` on a live collection; indices are
        // bounded by the count just read.
        let count = unsafe { collection.GetCount() }
            .map_err(|error| format!("IMMDeviceCollection::GetCount failed: {error}"))?;
        let mut devices = Vec::with_capacity(count as usize);
        for index in 0..count {
            match unsafe { collection.Item(index) } {
                Ok(device) => devices.push(device),
                Err(error) => {
                    println!("  warning: Item({index}) failed: {error}");
                }
            }
        }
        Ok(devices)
    }

    /// Gathers every queryable fact about one endpoint. Per-query
    /// failures are recorded in `notes` — a probe reports what it could
    /// read, it does not abort on the first refused call.
    fn describe_endpoint(device: &IMMDevice, flow: &'static str, names: &PortNames) -> EndpointRow {
        let mut notes = Vec::new();
        let id = endpoint_id(device).unwrap_or_else(|error| {
            notes.push(error);
            String::new()
        });
        // SAFETY: `GetState` on a live IMMDevice.
        let state = unsafe { device.GetState() }.unwrap_or_default();
        let friendly_name = friendly_name(names, &id).map(str::to_owned);
        let is_virtual_candidate = friendly_name.as_deref().is_some_and(is_virtual_name);

        let client = match activate_client(device) {
            Ok(client) => Some(client),
            Err(error) => {
                notes.push(error);
                None
            }
        };

        let (mix_format, format_results, device_period, stream_latency) = match &client {
            Some(client) => (
                mix_format(client, &mut notes),
                format_results(client),
                device_period(client, &mut notes),
                stream_latency(client, &mut notes),
            ),
            None => (None, Vec::new(), None, None),
        };

        EndpointRow {
            flow,
            state,
            id,
            friendly_name,
            is_virtual_candidate,
            mix_format,
            format_results,
            device_period,
            stream_latency,
            notes,
        }
    }

    /// Activates `IAudioClient` on the endpoint — query-only: the
    /// probe never calls `Initialize`, so no stream is opened and no
    /// audio session is created.
    fn activate_client(device: &IMMDevice) -> Result<IAudioClient, String> {
        // SAFETY: `device` is a live IMMDevice; `Activate` returns a
        // new COM reference owned by the smart pointer.
        unsafe { device.Activate::<IAudioClient>(CLSCTX_ALL, None) }
            .map_err(|error| format!("IAudioClient activate failed: {error}"))
    }

    /// `IMMDevice::GetId` → owned `String`; the returned `PWSTR` is
    /// `CoTaskMem`-allocated and freed exactly once.
    fn endpoint_id(device: &IMMDevice) -> Result<String, String> {
        // SAFETY: `device` is a live IMMDevice; the returned PWSTR is
        // valid until freed below.
        let pwstr = unsafe { device.GetId() }.map_err(|error| format!("GetId failed: {error}"))?;
        let com_string = ComString(pwstr);
        // SAFETY: `com_string.0` is a valid NUL-terminated UTF-16
        // string for the duration of the call.
        unsafe { com_string.0.to_string() }.map_err(|error| format!("GetId decode failed: {error}"))
    }

    /// `IAudioClient::GetMixFormat` rendered as one line; the
    /// `CoTaskMem` block is freed exactly once after the fields are
    /// copied out.
    fn mix_format(client: &IAudioClient, notes: &mut Vec<String>) -> Option<String> {
        // SAFETY: `GetMixFormat` returns a `CoTaskMem`-allocated
        // `WAVEFORMATEX` owned by the caller.
        let ptr = match unsafe { client.GetMixFormat() } {
            Ok(ptr) if !ptr.is_null() => ptr,
            Ok(_) => return None,
            Err(error) => {
                notes.push(format!("GetMixFormat failed: {error}"));
                return None;
            }
        };
        // SAFETY: `ptr` points to a valid `WAVEFORMATEX` until freed
        // below; fields are copied to locals before the free.
        // `WAVEFORMATEX` is `repr(packed)` — fields are copied out one
        // by one (taking `&field` would be an unaligned reference).
        let (tag, channels, rate, bits, cb_size) = unsafe {
            let format = &*ptr;
            (
                format.wFormatTag,
                format.nChannels,
                format.nSamplesPerSec,
                format.wBitsPerSample,
                format.cbSize,
            )
        };
        let rendered = format!(
            "tag={tag}({}) ch={channels} rate={rate}Hz bits={bits} cbSize={cb_size}",
            format_tag_name(tag)
        );
        // SAFETY: frees the `CoTaskMem` block `GetMixFormat` allocated.
        unsafe { Com::CoTaskMemFree(Some(ptr.cast())) }
        Some(rendered)
    }

    /// Runs every [`FORMAT_PROBES`] entry through
    /// `IAudioClient::IsFormatSupported` in shared mode.
    fn format_results(client: &IAudioClient) -> Vec<String> {
        FORMAT_PROBES
            .iter()
            .map(|probe| {
                let format = WAVEFORMATEX {
                    wFormatTag: probe.tag,
                    nChannels: probe.channels,
                    nSamplesPerSec: probe.rate_hz,
                    nAvgBytesPerSec: probe
                        .rate_hz
                        .saturating_mul(u32::from(probe.channels))
                        .saturating_mul(u32::from(probe.bits))
                        / 8,
                    nBlockAlign: probe.channels.saturating_mul(probe.bits / 8),
                    wBitsPerSample: probe.bits,
                    cbSize: 0,
                };
                // SAFETY: `format` is a valid `WAVEFORMATEX` for the
                // duration of the call; `client` is a live
                // `IAudioClient`. `closest` receives a `CoTaskMem`
                // `WAVEFORMATEX*` only on `AUDCLNT_E_UNSUPPORTED_FORMAT`
                // — freed below when non-null.
                let mut closest: *mut WAVEFORMATEX = std::ptr::null_mut();
                let hr = unsafe {
                    client.IsFormatSupported(
                        AUDCLNT_SHAREMODE_SHARED,
                        &raw const format,
                        Some(&raw mut closest),
                    )
                };
                let closest_desc = describe_closest(closest);
                let verdict = if hr.is_ok() {
                    "supported".to_owned()
                } else if hr == Audio::AUDCLNT_E_UNSUPPORTED_FORMAT {
                    format!("unsupported{closest_desc}")
                } else {
                    format!("error({hr}){closest_desc}")
                };
                format!("{}: {verdict}", probe.label)
            })
            .collect()
    }

    /// Renders the `ppclosestmatch` out-param of `IsFormatSupported`
    /// (a `CoTaskMem` `WAVEFORMATEX*`) and frees it exactly once.
    fn describe_closest(ptr: *mut WAVEFORMATEX) -> String {
        if ptr.is_null() {
            return String::new();
        }
        // SAFETY: on `AUDCLNT_E_UNSUPPORTED_FORMAT` the OS hands back a
        // `CoTaskMem`-allocated `WAVEFORMATEX` owned by the caller.
        let (tag, channels, rate, bits, cb_size) = unsafe {
            let format = &*ptr;
            (
                format.wFormatTag,
                format.nChannels,
                format.nSamplesPerSec,
                format.wBitsPerSample,
                format.cbSize,
            )
        };
        // SAFETY: frees the `CoTaskMem` block the call allocated.
        unsafe { Com::CoTaskMemFree(Some(ptr.cast())) }
        format!(
            " closest=tag={tag}({}) ch={channels} rate={rate}Hz bits={bits} cbSize={cb_size}",
            format_tag_name(tag)
        )
    }

    /// `IAudioClient::GetDevicePeriod` — default/minimum periodicity
    /// in 100 ns units.
    fn device_period(client: &IAudioClient, notes: &mut Vec<String>) -> Option<(i64, i64)> {
        let mut default_period = 0i64;
        let mut minimum_period = 0i64;
        // SAFETY: both out-pointers reference live `i64` locals for
        // the duration of the call.
        match unsafe {
            client.GetDevicePeriod(Some(&raw mut default_period), Some(&raw mut minimum_period))
        } {
            Ok(()) => Some((default_period, minimum_period)),
            Err(error) => {
                notes.push(format!("GetDevicePeriod failed: {error}"));
                None
            }
        }
    }

    /// `IAudioClient::GetStreamLatency` — 100 ns units. May be
    /// refused before `Initialize`; the refusal is recorded, not
    /// hidden.
    fn stream_latency(client: &IAudioClient, notes: &mut Vec<String>) -> Option<i64> {
        // SAFETY: `client` is a live `IAudioClient`.
        match unsafe { client.GetStreamLatency() } {
            Ok(latency) => Some(latency),
            Err(error) => {
                notes.push(format!("GetStreamLatency failed: {error}"));
                None
            }
        }
    }

    /// Friendly names reported by the production port surface, keyed
    /// indirectly: each cpal `DeviceId` renders as
    /// `wasapi:<endpoint-id>`, so the raw `IMMDevice` id is matched by
    /// substring.
    fn port_names() -> PortNames {
        let mut names = Vec::new();
        let Ok(host) = default_host() else {
            return names;
        };
        for (result, direction) in [
            (host.output_devices(), "output"),
            (host.input_devices(), "input"),
        ] {
            let Ok(devices) = result else {
                continue;
            };
            for device in devices {
                let name = device
                    .name()
                    .unwrap_or_else(|_| "<name unavailable>".to_owned());
                let id = device
                    .id()
                    .ok()
                    .flatten()
                    .map(|device_id| format!("{device_id:?}"));
                if let Some(id) = id {
                    names.push((id, name, direction));
                }
            }
        }
        names
    }

    /// Finds the friendly name whose cpal id rendering contains the
    /// raw endpoint id.
    fn friendly_name<'a>(names: &'a PortNames, endpoint_id: &str) -> Option<&'a str> {
        if endpoint_id.is_empty() {
            return None;
        }
        names
            .iter()
            .find(|(id, _, _)| id.contains(endpoint_id))
            .map(|(_, name, _)| name.as_str())
    }

    /// The case-insensitive name heuristic behind
    /// [`EndpointRow::is_virtual_candidate`].
    fn is_virtual_name(name: &str) -> bool {
        let lower = name.to_lowercase();
        VIRTUAL_NAME_HINTS.iter().any(|hint| lower.contains(hint))
    }

    fn state_name(state: DEVICE_STATE) -> &'static str {
        if state == Audio::DEVICE_STATE_ACTIVE {
            "active"
        } else if state == Audio::DEVICE_STATE_DISABLED {
            "disabled"
        } else if state == Audio::DEVICE_STATE_NOTPRESENT {
            "not-present"
        } else if state == Audio::DEVICE_STATE_UNPLUGGED {
            "unplugged"
        } else {
            "unknown"
        }
    }

    fn format_tag_name(tag: u16) -> &'static str {
        match tag {
            0x0001 => "pcm",
            0x0003 => "ieee-float",
            0xfffe => "extensible",
            _ => "other",
        }
    }

    fn print_endpoint(row: &EndpointRow) {
        let candidate = if row.is_virtual_candidate {
            "  [virtual-candidate: name heuristic, unconfirmed]"
        } else {
            ""
        };
        println!(
            "  [{}] {} state={} id={}{}",
            row.flow,
            row.friendly_name.as_deref().unwrap_or("<unnamed>"),
            state_name(row.state),
            row.id,
            candidate
        );
        if let Some(mix) = &row.mix_format {
            println!("      mix_format: {mix}");
        }
        for result in &row.format_results {
            println!("      is_format_supported(shared): {result}");
        }
        if let Some((default, minimum)) = row.device_period {
            println!("      device_period_100ns: default={default} minimum={minimum}");
        }
        if let Some(latency) = row.stream_latency {
            println!("      stream_latency_100ns: {latency}");
        }
        for note in &row.notes {
            println!("      note: {note}");
        }
    }

    /// The fail-closed summary: what this run observed and which
    /// virtual-endpoint cases stay `NotRun`.
    fn print_verdict(rows: &[EndpointRow]) {
        let candidates = rows.iter().filter(|row| row.is_virtual_candidate).count();
        println!("== verdict ==");
        println!("  endpoints enumerated: {}", rows.len());
        println!("  virtual-endpoint candidates (name heuristic, unconfirmed): {candidates}");
        println!(
            "  driver signature check: NotRun — driver/package signature query needs SetupAPI surfaces outside the enabled `windows` features (discovery doc gap)"
        );
        println!(
            "  provisioning: {}",
            if candidates == 0 {
                "NOT_PROVISIONED"
            } else {
                "CANDIDATES_OBSERVED"
            }
        );
        println!("  capability notes:");
        println!("    mmdevice enumeration (all states) + id/state: observed");
        println!(
            "    mix format / IsFormatSupported / device period / stream latency queries: observed per endpoint"
        );
        println!(
            "    virtual hotplug / device-loss / format-switch / permission-denial / xrun / loopback latency: NotRun — need a provisioned endpoint plus the runtime-driven controller (next slice)"
        );
        println!(
            "  audio virtual-endpoint cases: NotRun — enumeration is not `virtual-os-endpoint` evidence; open/stream proof is the next slice"
        );
    }

    /// Prints the same devices through the production port surface
    /// (`ruviform_audio_device` → cpal) so the raw enumeration can be
    /// cross-checked against what product code actually sees.
    fn print_port_view() {
        let Ok(host) = default_host() else {
            println!("  default_host() failed");
            return;
        };
        println!("  host: {}", host.name());
        for (result, label) in [
            (host.output_devices(), "output"),
            (host.input_devices(), "input"),
        ] {
            match result {
                Ok(devices) => {
                    println!("  {label} devices: {}", devices.len());
                    for device in devices {
                        let name = device.name().unwrap_or_else(|_| "?".to_owned());
                        let id = device.id().ok().flatten();
                        println!("    - {name}  id={id:?}");
                    }
                }
                Err(error) => println!("  {label} devices: query failed: {error}"),
            }
        }
        for (query, result) in [
            ("default_output_device", host.default_output_device()),
            ("default_input_device", host.default_input_device()),
        ] {
            match result {
                Ok(Some(device)) => {
                    let name = device.name().unwrap_or_else(|_| "?".to_owned());
                    println!("  {query}: {name}");
                }
                Ok(None) => println!("  {query}: none"),
                Err(error) => println!("  {query}: query failed: {error}"),
            }
        }
    }

    /// `CoTaskMem`-allocated `PWSTR` freed exactly once on drop — the
    /// same rule as the adapter's `ComString` (`src/wasapi_input.rs`).
    struct ComString(windows::core::PWSTR);

    impl Drop for ComString {
        fn drop(&mut self) {
            // SAFETY: `self.0` came from `IMMDevice::GetId`
            // (CoTaskMemAlloc family) and is freed exactly once here.
            unsafe { Com::CoTaskMemFree(Some(self.0.as_ptr().cast())) }
        }
    }
}
