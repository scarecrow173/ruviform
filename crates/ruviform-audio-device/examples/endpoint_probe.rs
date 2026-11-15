//! `endpoint_probe` — Issue #629 capability probe for Windows
//! virtual audio endpoints (extended in the fourth slice).
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
//! device period / stream latency, applies a clearly labelled
//! name heuristic to flag *candidate* virtual endpoints, prints the
//! render↔capture pair-candidate table the streaming verifier
//! (`tests/virtual_endpoint.rs`) consumes, and makes best-effort
//! driver-signature and microphone-permission observations through
//! PowerShell/registry queries.
//!
//! What it never does: claim `virtual-os-endpoint` evidence. Per the
//! Issue #627 boundary-A rule, enumeration, format queries, name
//! matches and pair candidates are not endpoint streaming proof —
//! the product backend must actually open and stream through the
//! endpoint, which is `tests/virtual_endpoint.rs`'s job (a
//! `#[ignore]`-gated manual lane, fail-closed `NotRun` without a
//! provisioned pair). When no virtual endpoint is provisioned, every
//! virtual-endpoint case this probe feeds is reported `NotRun`
//! (fail-closed), never `Pass`.

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
        println!(
            "  pair candidates / driver signature / permission observations: NotRun — Windows-only"
        );
        println!("  cleanup: no persistent state — the probe is read-only on every platform");
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

    /// Direction words stripped before stem matching — kept in sync
    /// with `tests/virtual_endpoint.rs` (`DIRECTION_TOKENS`) by hand:
    /// test files cannot be imported into examples.
    const DIRECTION_TOKENS: &[&str] = &["input", "output", "render", "capture", "in", "out"];

    /// One production-port device reduced to what pair matching needs.
    struct PortDevice {
        name: String,
        direction: &'static str,
    }

    /// Runs the whole probe and prints the report.
    pub(super) fn run() {
        println!();
        println!("== WASAPI MMDevice enumeration ==");
        let (rows, enumerated) = match enumerate_all() {
            Ok(rows) => {
                for row in &rows {
                    print_endpoint(row);
                }
                (rows, true)
            }
            Err(error) => {
                println!("  enumeration failed: {error}");
                (Vec::new(), false)
            }
        };
        println!();
        println!("== production port view (ruviform_audio_device) ==");
        let port_devices = print_port_view();
        println!();
        let pair_count = print_pair_candidates(&port_devices);
        println!();
        let driver_status = print_driver_signature();
        println!();
        let permission_status = print_capture_permission();
        println!();
        print_verdict(
            &rows,
            enumerated,
            pair_count,
            &driver_status,
            &permission_status,
        );
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
                // `ppclosestmatch` is only populated on
                // `AUDCLNT_E_UNSUPPORTED_FORMAT` — anything else leaves
                // `closest` null, and freeing a pointer the API did not
                // allocate would be unsound.
                let closest_desc = if hr == Audio::AUDCLNT_E_UNSUPPORTED_FORMAT {
                    describe_closest(closest)
                } else {
                    String::new()
                };
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

    /// Normalized product stem for pair matching — identical rule to
    /// `tests/virtual_endpoint.rs`'s `virtual_stem`: lowercase,
    /// non-alphanumeric split, direction tokens removed. `None` for
    /// non-virtual names or an empty stem.
    fn virtual_stem(name: &str) -> Option<String> {
        if !is_virtual_name(name) {
            return None;
        }
        let stem = name
            .to_lowercase()
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|token| !token.is_empty())
            .filter(|token| !DIRECTION_TOKENS.contains(token))
            .collect::<Vec<_>>()
            .join(" ");
        if stem.is_empty() { None } else { Some(stem) }
    }

    /// Render↔capture pair candidates: each output-direction device
    /// whose virtual stem matches an unclaimed input-direction
    /// device's stem — the same selection rule
    /// `tests/virtual_endpoint.rs` applies. A match is a *candidate*,
    /// never proof of provisioning.
    fn pair_candidates(devices: &[PortDevice]) -> Vec<(String, String, String)> {
        let mut free: Vec<(&PortDevice, String)> = devices
            .iter()
            .filter(|device| device.direction == "input")
            .filter_map(|device| virtual_stem(&device.name).map(|stem| (device, stem)))
            .collect();
        let mut pairs = Vec::new();
        for render in devices.iter().filter(|device| device.direction == "output") {
            let Some(stem) = virtual_stem(&render.name) else {
                continue;
            };
            let Some(index) = free.iter().position(|(_, free_stem)| free_stem == &stem) else {
                continue;
            };
            let (capture, _) = free.remove(index);
            pairs.push((render.name.clone(), capture.name.clone(), stem));
        }
        pairs
    }

    /// Prints the pair-candidate table and returns the pair count
    /// for the verdict. Unpaired virtual candidates are listed too —
    /// they are the "half-provisioned" evidence a reviewer needs.
    fn print_pair_candidates(devices: &[PortDevice]) -> usize {
        println!("== pair candidates (virtual name-stem match) ==");
        let pairs = pair_candidates(devices);
        if pairs.is_empty() {
            println!("  none — NOT_PROVISIONED");
            return 0;
        }
        let mut paired_names: Vec<&str> = Vec::new();
        for (render, capture, stem) in &pairs {
            println!("  render \"{render}\" ↔ capture \"{capture}\"  stem=\"{stem}\"");
            paired_names.push(render.as_str());
            paired_names.push(capture.as_str());
        }
        for device in devices {
            if virtual_stem(&device.name).is_some() && !paired_names.contains(&device.name.as_str())
            {
                println!(
                    "  unpaired virtual candidate: {} \"{}\"",
                    device.direction, device.name
                );
            }
        }
        pairs.len()
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
    fn print_verdict(
        rows: &[EndpointRow],
        enumerated: bool,
        pair_count: usize,
        driver_status: &str,
        permission_status: &str,
    ) {
        let candidates = rows.iter().filter(|row| row.is_virtual_candidate).count();
        println!("== verdict ==");
        println!("  endpoints enumerated: {}", rows.len());
        println!("  virtual-endpoint candidates (name heuristic, unconfirmed): {candidates}");
        println!("  render↔capture pair candidates (name-stem match, unconfirmed): {pair_count}");
        println!("  driver signature / driver metadata: {driver_status}");
        println!("  microphone/capture permission (HKCU privacy keys): {permission_status}");
        println!(
            "  provisioning: {}",
            if !enumerated {
                "UNKNOWN — MMDevice enumeration failed"
            } else if candidates == 0 {
                "NOT_PROVISIONED"
            } else {
                "CANDIDATES_OBSERVED"
            }
        );
        println!("  capability notes:");
        println!("    mmdevice enumeration (all states) + id/state: observed");
        println!(
            "    mix format / IsFormatSupported / device period / stream latency queries: attempted per endpoint (refusals recorded in notes)"
        );
        println!(
            "    pair candidate table + driver signature + permission observations: best-effort this slice (see sections above; failures are NotRun notes, never probe failures)"
        );
        println!(
            "    streaming proof: tests/virtual_endpoint.rs — `cargo test -p ruviform-audio-device --features real-device --test virtual_endpoint -- --ignored --test-threads=1` (auxiliary lane; no mise task is registered yet)"
        );
        println!(
            "    virtual hotplug / device-loss / format-switch / permission-denial / xrun / loopback latency: NotRun — need a provisioned endpoint plus the runtime-driven controller (next slice)"
        );
        println!(
            "  cleanup: no persistent state — COM references are dropped at scope end; `IAudioClient` is activated for queries only and never `Initialize()`d, so no audio session or stream is created; PowerShell/registry queries are read-only"
        );
        println!(
            "  audio virtual-endpoint cases: NotRun — enumeration/pairing is not `virtual-os-endpoint` evidence; the streaming verifier (`tests/virtual_endpoint.rs`) reports its own fail-closed verdict"
        );
    }

    /// Best-effort driver signature/metadata observation: `PnP` signed
    /// drivers whose device name matches the virtual-endpoint hints,
    /// with an Authenticode check on the staged driver catalog
    /// (`C:\Windows\INF\<inf>.cat`) when it exists. Every failure —
    /// missing PowerShell, WMI refusal, absent catalog — becomes a
    /// `NotRun` note; the probe never fails on this observation.
    /// Returns the one-line status the verdict prints.
    fn print_driver_signature() -> String {
        println!("== driver signature observations (best-effort) ==");
        // `$rx` is a deliberately broader superset of `VIRTUAL_NAME_HINTS`
        // — driver records use product names ("NVIDIA Virtual Audio
        // Device"), not endpoint names, so bare 'virtual audio'/'cable'
        // also match. A hit is metadata evidence only, never a pairing.
        let script = r"
$rx = 'cable|voicemeeter|vb-?audio|virtual audio';
$found = $false;
Get-CimInstance Win32_PnPSignedDriver -ErrorAction SilentlyContinue |
  Where-Object { $_.DeviceName -match $rx } |
  ForEach-Object {
    $found = $true;
    $cat = 'C:\Windows\INF\' + ($_.InfName -replace '\.inf$','.cat');
    $sig = 'catalog:absent';
    if (Test-Path $cat) {
      $s = Get-AuthenticodeSignature $cat -ErrorAction SilentlyContinue;
      if ($s) { $sig = 'catalog:' + $s.Status.ToString() + ' signer=' + $s.SignerCertificate.Subject }
    }
    '{0} | provider={1} | version={2} | date={3} | inf={4} | {5}' -f $_.DeviceName, $_.DriverProviderName, $_.DriverVersion, $_.DriverDate, $_.InfName, $sig
  };
if (-not $found) { 'no PnP signed-driver records matched virtual-endpoint name hints' }
";
        match run_powershell(script) {
            Ok(lines) => {
                for line in &lines {
                    println!("  {line}");
                }
                "observed".to_owned()
            }
            Err(reason) => {
                println!("  driver signature check: NotRun — {reason}");
                format!("NotRun — {reason}")
            }
        }
    }

    /// Best-effort microphone/capture permission observation: the
    /// HKCU privacy keys Windows uses for "Microphone access" (the
    /// per-capability `DeviceAccess` switch) and "Let apps access
    /// your microphone" (`ConsentStore`). Missing keys are reported
    /// as `absent`, query failures as `NotRun` — never fatal.
    /// Returns the one-line status the verdict prints.
    fn print_capture_permission() -> String {
        println!("== microphone/capture permission (best-effort) ==");
        let script = r"
$keys = @(
  'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\DeviceAccess\Global\{2EEF81BE-33FA-4800-9670-1FEE788A9517}',
  'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone',
  'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone\NonPackaged'
);
foreach ($k in $keys) {
  $p = Get-ItemProperty -Path $k -Name Value -ErrorAction SilentlyContinue;
  if ($p) { '{0} => {1}' -f $k, $p.Value } else { '{0} => absent' -f $k }
}
";
        match run_powershell(script) {
            Ok(lines) => {
                for line in &lines {
                    println!("  {line}");
                }
                "observed".to_owned()
            }
            Err(reason) => {
                println!("  permission check: NotRun — {reason}");
                format!("NotRun — {reason}")
            }
        }
    }

    /// Runs a read-only PowerShell snippet and returns its trimmed
    /// non-empty stdout lines. Spawn failure or a non-zero exit is an
    /// `Err` reason — callers turn it into a `NotRun` note.
    fn run_powershell(script: &str) -> Result<Vec<String>, String> {
        match Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let err = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("powershell exit {}: {}", output.status, err.trim()));
                }
                let text = String::from_utf8_lossy(&output.stdout);
                Ok(text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect())
            }
            Err(error) => Err(format!("powershell failed: {error}")),
        }
    }

    /// Prints the same devices through the production port surface
    /// (`ruviform_audio_device` → cpal) so the raw enumeration can be
    /// cross-checked against what product code actually sees. Returns
    /// the `(name, direction)` rows the pair-candidate table consumes.
    fn print_port_view() -> Vec<PortDevice> {
        let mut port_devices = Vec::new();
        let Ok(host) = default_host() else {
            println!("  default_host() failed");
            return port_devices;
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
                        port_devices.push(PortDevice {
                            name,
                            direction: label,
                        });
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
        port_devices
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
