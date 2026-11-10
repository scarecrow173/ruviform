//! `WinMM` (`winmm.dll`) MIDI input backend — Windows only.
//!
//! This module is the single place `WinMM` types, constants and error
//! codes are named; everything crossing the port boundary is normalized
//! onto `DeviceError` and the `midi_input` trait objects. The driver
//! callback ([`midi_in_proc`]) runs on a system thread and only touches
//! the pre-reserved [`Shared`] queue — per the `WinMM` callback contract
//! it performs no allocation and calls no system functions beyond the
//! permitted synchronization primitives; `SysEx` buffer requeue happens
//! on the control thread inside `read`/`close` instead.
//!
//! Semantics verified against Microsoft Learn (mmeapi): `MIM_DATA`
//! packs `dwParam1` as `status | data1<<8 | data2<<16` and `dwParam2`
//! is a millisecond timestamp reset to zero by `midiInStart`; running
//! status is already expanded by the driver.

// The whole module is the FFI boundary: every `unsafe` block below is
// individually justified at its site, and the lint is scoped to this
// file only (workspace policy: local allowance + SAFETY comments).
#![expect(unsafe_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use ruviform_audio_core::device::{DeviceError, DeviceResult};
use ruviform_audio_core::midi_input::{
    MidiInput, MidiInputInfo, MidiInputPort, MidiPortCapabilities, MidiPortOrigin, MidiReadStats,
    TimedMidiInput,
};
use ruviform_media_core::{
    ClockDomainId, InputStamp, MidiEvent, TimeBase, TimedInputEvent, TimestampProvenance,
};
use windows_sys::Win32::Media::Audio::{
    self, CALLBACK_FUNCTION, HMIDIIN, MHDR_DONE, MIDI_IO_STATUS, MIDIHDR, MIDIINCAPSW,
};
use windows_sys::Win32::Media::Multimedia::{
    DRV_QUERYDEVICEINTERFACE, DRV_QUERYDEVICEINTERFACESIZE,
};
use windows_sys::Win32::Media::{
    MM_MIM_CLOSE, MM_MIM_DATA, MM_MIM_LONGDATA, MM_MIM_LONGERROR, MM_MIM_MOREDATA,
    MMSYSERR_ALLOCATED, MMSYSERR_BADDEVICEID, MMSYSERR_NODRIVER, MMSYSERR_NOERROR,
};

use crate::midi;
use crate::queue::Shared;

/// Private clock-domain base for `WinMM` subscriptions. Because the
/// timestamp epoch resets on every `midiInStart`, each open subscription
/// needs its own domain id or stamps from two simultaneously open ports
/// would collide. The base sits far above the well-known ids
/// (`UNKNOWN`/`SAMPLE_CLOCK`/`MONOTONIC` = 0/1/2) so adapter-allocated
/// domains never impersonate them.
const DOMAIN_BASE: u64 = u64::from_be_bytes(*b"RUVMIDI1");
static NEXT_DOMAIN: AtomicU64 = AtomicU64::new(DOMAIN_BASE);

/// Number of `SysEx` buffers registered per input — enough for typical
/// `SysEx` bursts while bounding memory; overflow surfaces as
/// `MIM_LONGERROR` → `dropped`.
const SYSEX_BUFFER_COUNT: usize = 4;
/// Per-buffer `SysEx` capacity in bytes (u32 because `dwBufferLength`
/// is a DWORD).
const SYSEX_BUFFER_LEN: u32 = 1024;
#[expect(clippy::cast_possible_truncation)]
const SIZE_OF_MIDIHDR: u32 = size_of::<MIDIHDR>() as u32;

fn next_domain() -> ClockDomainId {
    ClockDomainId::new(NEXT_DOMAIN.fetch_add(1, Ordering::Relaxed))
}

/// Enumerates the MIDI input ports `WinMM` currently reports.
///
/// **RT contract:** non-RT only — calls OS APIs and allocates.
///
/// Cannot fail at the top level: `midiInGetNumDevs` is infallible and
/// devices that vanish mid-scan are skipped, so the port contract's
/// `DeviceResult` is supplied by `SystemBackend::input_ports`.
pub(crate) fn enumerate_ports() -> Vec<Box<dyn MidiInputPort>> {
    // SAFETY: `midiInGetNumDevs` takes no arguments and cannot fail.
    let count = unsafe { Audio::midiInGetNumDevs() };
    let mut ports = Vec::with_capacity(count as usize);
    for index in 0..count {
        // A device that vanishes between the count query and the caps
        // query is skipped — enumeration reports what it could read.
        if let Ok(port) = WinMmPort::enumerate_at(index) {
            ports.push(Box::new(port) as Box<dyn MidiInputPort>);
        }
    }
    ports
}

/// How an enumerated port is re-resolved to a device index at `open`.
#[derive(Debug)]
enum PortKey {
    /// Stable device-interface id from `DRV_QUERYDEVICEINTERFACE`
    /// (the same mechanism `midir` uses). Re-scanned at open so index
    /// reshuffles cannot open the wrong device.
    Interface(String),
    /// Fallback for devices without an interface id: device index +
    /// name, re-verified at open.
    Index { index: u32, name: String },
}

/// A live handle for one enumerated `WinMM` input port.
struct WinMmPort {
    info: MidiInputInfo,
    key: PortKey,
    domain: ClockDomainId,
    opened: AtomicBool,
}

impl WinMmPort {
    /// Reads caps + interface id for `index` and builds the port.
    fn enumerate_at(index: u32) -> DeviceResult<Self> {
        let caps = device_caps(index)?;
        let name = caps_name(&caps);
        let key = match interface_id(index) {
            Some(id) => PortKey::Interface(id),
            None => PortKey::Index {
                index,
                name: name.clone(),
            },
        };
        let domain = next_domain();
        let info = MidiInputInfo::new(
            port_id(&key),
            name,
            // WinMM exposes no external/virtual/internal distinction —
            // report Unknown rather than guess.
            MidiPortOrigin::Unknown,
            // MIDI 1.0 byte stream only; driver timestamps are carried.
            MidiPortCapabilities::stamped(false, domain),
        );
        Ok(Self {
            info,
            key,
            domain,
            opened: AtomicBool::new(false),
        })
    }

    /// Re-resolves the port to a current device index.
    fn resolve_index(&self) -> DeviceResult<u32> {
        match &self.key {
            PortKey::Interface(id) => find_interface_index(id).ok_or(DeviceError::DeviceNotFound),
            PortKey::Index { index, name } => {
                if index_still_matches(*index, name) {
                    Ok(*index)
                } else {
                    Err(DeviceError::DeviceNotFound)
                }
            }
        }
    }

    fn open_inner(&self) -> DeviceResult<Box<dyn MidiInput>> {
        let index = self.resolve_index()?;
        open_at_index(index, self.domain)
    }
}

impl MidiInputPort for WinMmPort {
    /// The port's identity and declared capabilities.
    ///
    /// **RT contract:** non-RT only — clones owned strings.
    fn info(&self) -> MidiInputInfo {
        self.info.clone()
    }

    /// Opens a `WinMM` subscription on this port.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    fn open(&self) -> DeviceResult<Box<dyn MidiInput>> {
        if self.opened.swap(true, Ordering::AcqRel) {
            return Err(DeviceError::DeviceBusy);
        }
        match self.open_inner() {
            Ok(input) => Ok(input),
            Err(err) => {
                // A failed open never started a subscription, so the
                // one-open rule is not consumed — allow a retry.
                self.opened.store(false, Ordering::Release);
                Err(err)
            }
        }
    }
}

fn port_id(key: &PortKey) -> String {
    match key {
        PortKey::Interface(id) => id.clone(),
        PortKey::Index { index, name } => format!("winmm-fallback:{index}:{name}"),
    }
}

/// `midiInGetDevCapsW` for one device index.
#[expect(clippy::cast_possible_truncation)]
fn device_caps(index: u32) -> DeviceResult<MIDIINCAPSW> {
    let mut caps = MIDIINCAPSW::default();
    // SAFETY: `caps` is a valid out-struct of the declared size.
    let rc = unsafe {
        Audio::midiInGetDevCapsW(
            index as usize,
            &raw mut caps,
            size_of::<MIDIINCAPSW>() as u32,
        )
    };
    if rc == MMSYSERR_NOERROR {
        Ok(caps)
    } else {
        Err(DeviceError::DeviceNotFound)
    }
}

/// Decodes the fixed-width UTF-16 `szPname` field (NUL-terminated,
/// max 32 units).
fn caps_name(caps: &MIDIINCAPSW) -> String {
    // `MIDIINCAPSW` is `repr(packed)` — copy the field to a local before
    // taking any reference (a `&[u16]` into the packed field would be an
    // unaligned reference, which is UB).
    let sz_pname = caps.szPname;
    let end = sz_pname
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(sz_pname.len());
    String::from_utf16_lossy(&sz_pname[..end])
}

/// Queries the stable device-interface id via `midiInMessage`
/// (`DRV_QUERYDEVICEINTERFACE`) — the same driver-verbs mechanism
/// `midir` uses. Returns `None` for legacy devices that do not answer.
fn interface_id(index: u32) -> Option<String> {
    // SAFETY: `size`/`buf` are valid out-parameters for the duration of
    // each call; `index` is passed as the device id handle exactly as
    // the WinMM docs specify for unopened devices.
    unsafe {
        let mut size: u32 = 0;
        let rc = Audio::midiInMessage(
            index as usize as HMIDIIN,
            DRV_QUERYDEVICEINTERFACESIZE,
            (&raw mut size) as usize,
            0,
        );
        if rc != MMSYSERR_NOERROR || size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let rc = Audio::midiInMessage(
            index as usize as HMIDIIN,
            DRV_QUERYDEVICEINTERFACE,
            size as usize,
            buf.as_mut_ptr() as usize,
        );
        if rc != MMSYSERR_NOERROR {
            return None;
        }
        // The interface id is a NUL-terminated UTF-16LE string.
        let units: Vec<u16> = buf
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_ne_bytes(*c))
            .take_while(|&c| c != 0)
            .collect();
        String::from_utf16(&units).ok()
    }
}

/// Finds the current device index carrying `interface_id`.
fn find_interface_index(id: &str) -> Option<u32> {
    // SAFETY: same contract as `enumerate_ports` — count query is
    // infallible; per-index caps/interface queries fail soft.
    let count = unsafe { Audio::midiInGetNumDevs() };
    (0..count).find(|&i| interface_id(i).is_some_and(|candidate| candidate == id))
}

/// Fallback-key check: `index` still exists and still reports `name`.
fn index_still_matches(index: u32, name: &str) -> bool {
    device_caps(index).is_ok_and(|caps| caps_name(&caps) == name)
}

/// One prepared+queued `SysEx` buffer. `header.lpData` points into
/// `_data`; both must stay alive until `midiInUnprepareHeader`.
struct SysexBuffer {
    header: Box<MIDIHDR>,
    /// Owns the byte allocation `header.lpData` points into — held for
    /// its lifetime only (never read directly).
    _data: Box<[u8]>,
}

impl SysexBuffer {
    /// Re-queues a buffer the driver returned (`MHDR_DONE`) so the next
    /// `SysEx` message has somewhere to land. Called from `read` — never
    /// from the callback, which may not call system functions.
    fn requeue_if_done(&mut self, handle: HMIDIIN) {
        if self.header.dwFlags & MHDR_DONE == 0 {
            return;
        }
        // SAFETY: `handle` is open and `header` is a prepared,
        // driver-returned `MIDIHDR` whose `lpData` still points at the
        // live `data` allocation owned alongside it.
        unsafe {
            let _ = Audio::midiInAddBuffer(handle, &raw mut *self.header, SIZE_OF_MIDIHDR);
        }
    }
}

/// Opens a `CALLBACK_FUNCTION | MIDI_IO_STATUS` subscription on the
/// device at `index`.
fn open_at_index(index: u32, domain: ClockDomainId) -> DeviceResult<Box<dyn MidiInput>> {
    let shared = Arc::new(Shared::new(domain));
    let shared_ptr = Arc::as_ptr(&shared);

    let mut handle: HMIDIIN = core::ptr::null_mut();
    // SAFETY: `handle` is a valid out-pointer; `midi_in_proc` is a
    // valid `extern "system"` callback; `shared_ptr` is the
    // `Arc::as_ptr` of the `Shared` the returned `Subscription` owns —
    // the Arc (and thus the pointee) outlives `midiInClose`, and WinMM
    // guarantees no callback fires after close returns.
    let rc = unsafe {
        Audio::midiInOpen(
            &raw mut handle,
            index,
            midi_in_proc as extern "system" fn(HMIDIIN, u32, usize, usize, usize) as usize,
            shared_ptr as usize,
            CALLBACK_FUNCTION | MIDI_IO_STATUS,
        )
    };
    if rc != MMSYSERR_NOERROR {
        return Err(match rc {
            MMSYSERR_BADDEVICEID | MMSYSERR_NODRIVER => DeviceError::DeviceNotFound,
            MMSYSERR_ALLOCATED => DeviceError::DeviceBusy,
            _ => DeviceError::Backend("winmm midiInOpen failed"),
        });
    }

    // Register SysEx buffers so long messages are delivered (and
    // countable) instead of vanishing driver-side. On any failure the
    // partially built subscription is torn down before returning.
    let mut buffers: Vec<SysexBuffer> = Vec::with_capacity(SYSEX_BUFFER_COUNT);
    for _ in 0..SYSEX_BUFFER_COUNT {
        match SysexBuffer::new(handle) {
            Ok(buffer) => buffers.push(buffer),
            Err(err) => {
                abort_open(handle, &mut buffers);
                return Err(err);
            }
        }
    }
    for buffer in &mut buffers {
        // SAFETY: `handle` is open; `buffer.header` is prepared and its
        // `lpData` points at the live `data` allocation.
        let rc =
            unsafe { Audio::midiInAddBuffer(handle, &raw mut *buffer.header, SIZE_OF_MIDIHDR) };
        if rc != MMSYSERR_NOERROR {
            abort_open(handle, &mut buffers);
            return Err(DeviceError::Backend("winmm midiInAddBuffer failed"));
        }
    }
    // SAFETY: `handle` is open with all buffers queued.
    let rc = unsafe { Audio::midiInStart(handle) };
    if rc != MMSYSERR_NOERROR {
        abort_open(handle, &mut buffers);
        return Err(DeviceError::Backend("winmm midiInStart failed"));
    }

    Ok(Box::new(Subscription {
        shared,
        handle,
        buffers,
        closed: false,
    }))
}

impl SysexBuffer {
    /// Allocates the data buffer and prepares the header against
    /// `handle`. The header is *not* queued yet — callers add it after
    /// all buffers are prepared so a mid-way failure unpreparies
    /// cleanly.
    fn new(handle: HMIDIIN) -> DeviceResult<Self> {
        let mut data = vec![0u8; SYSEX_BUFFER_LEN as usize].into_boxed_slice();
        let mut header = Box::new(MIDIHDR::default());
        header.lpData = data.as_mut_ptr().cast();
        header.dwBufferLength = SYSEX_BUFFER_LEN;
        // SAFETY: `header` is a valid `MIDIHDR` whose `lpData`/`dwBufferLength`
        // describe the live `data` allocation; `handle` is open.
        let rc = unsafe { Audio::midiInPrepareHeader(handle, &raw mut *header, SIZE_OF_MIDIHDR) };
        if rc != MMSYSERR_NOERROR {
            return Err(DeviceError::Backend("winmm midiInPrepareHeader failed"));
        }
        Ok(Self {
            header,
            _data: data,
        })
    }
}

/// Tears down a partially opened subscription: unprepare every prepared
/// header, then close the handle. Safe to call with an empty `buffers`.
fn abort_open(handle: HMIDIIN, buffers: &mut [SysexBuffer]) {
    // SAFETY: `handle` is open; each header in `buffers` was prepared by
    // `SysexBuffer::new`. Unpreparing a prepared header is required
    // before its memory is released.
    unsafe {
        for buffer in buffers {
            let _ = Audio::midiInUnprepareHeader(handle, &raw mut *buffer.header, SIZE_OF_MIDIHDR);
        }
        let _ = Audio::midiInClose(handle);
    }
}

/// The `WinMM` driver callback (`CALLBACK_FUNCTION`). Runs on a system
/// thread — must stay minimal: decode, stamp, enqueue, count. No
/// allocation, no OS calls.
///
/// # Safety of the `instance` pointer
///
/// `dwInstance` is the `Arc::as_ptr` of the `Shared` owned by the
/// [`Subscription`]. The Arc is released only when the `Subscription`
/// is dropped, which happens strictly after `midiInClose` returns;
/// `WinMM` guarantees no callback fires after close. The pointer is
/// therefore valid for the callback's entire lifetime.
#[expect(clippy::cast_possible_truncation)]
extern "system" fn midi_in_proc(
    _hmi: HMIDIIN,
    msg: u32,
    instance: usize,
    param1: usize,
    param2: usize,
) {
    // SAFETY: see the doc comment above — `instance` points at a live
    // `Shared` for the handle's whole callback lifetime.
    let shared = unsafe { &*(instance as *const Shared) };
    if shared.closing.load(Ordering::Acquire) {
        return;
    }
    match msg {
        MM_MIM_DATA => {
            // dwParam1: byte0=status, byte1=data1, byte2=data2,
            // byte3 unused (MSDN `MIM_DATA`).
            let dw = param1 as u32;
            let status = (dw & 0xFF) as u8;
            let data1 = ((dw >> 8) & 0xFF) as u8;
            let data2 = ((dw >> 16) & 0xFF) as u8;
            match midi::decode_message(status, data1, data2) {
                Some(event) => shared.push(stamp_event(
                    param2 as u64,
                    shared.domain,
                    shared.stamp_base,
                    event,
                )),
                // SysEx fragments, system common/realtime and malformed
                // bytes are received-but-untranslatable — counted, not
                // silently dropped.
                None => shared.push_unsupported(),
            }
        }
        // Driver-side loss notifications (same packed args, content
        // irrelevant — the message itself was lost).
        MM_MIM_MOREDATA | MM_MIM_LONGERROR => shared.push_dropped(),
        // SysEx delivered via a long-data buffer — outside the
        // channel-voice vocabulary. The returned header is re-queued
        // lazily on `read` (callbacks may not call system functions).
        MM_MIM_LONGDATA => shared.push_unsupported(),
        MM_MIM_CLOSE => shared.mark_dead(),
        _ => {}
    }
}

/// Builds the stamped envelope for one decoded event. `ticks` is the
/// raw `dwParam2` value — milliseconds since `midiInStart` — carried
/// unchanged; conversion to the audio clock is the engine's job.
fn stamp_event(
    ticks: u64,
    domain: ClockDomainId,
    base: TimeBase,
    event: MidiEvent,
) -> TimedMidiInput {
    TimedInputEvent {
        stamp: InputStamp {
            ticks,
            base,
            domain,
            provenance: TimestampProvenance::Carried,
        },
        event,
    }
}

/// An open `WinMM` input subscription.
struct Subscription {
    shared: Arc<Shared>,
    handle: HMIDIIN,
    buffers: Vec<SysexBuffer>,
    closed: bool,
}

impl Subscription {
    /// `close`/`Drop` shared teardown — idempotent via `closed`.
    fn shutdown(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        // Stop new callback enqueues first; `Shared` stays alive via
        // the Arc until this struct drops, so in-flight callbacks
        // remain memory-safe.
        self.shared.closing.store(true, Ordering::Release);
        // SAFETY: `handle` is open and `closed` guarantees this runs at
        // most once; `buffers` outlive the calls (fields drop after
        // `Drop::drop`). Order follows the WinMM contract: stop the
        // stream, reset pending I/O, unprepare headers, then close.
        unsafe {
            let _ = Audio::midiInStop(self.handle);
            let _ = Audio::midiInReset(self.handle);
            for buffer in &mut self.buffers {
                let _ = Audio::midiInUnprepareHeader(
                    self.handle,
                    &raw mut *buffer.header,
                    SIZE_OF_MIDIHDR,
                );
            }
            let _ = Audio::midiInClose(self.handle);
        }
    }
}

impl MidiInput for Subscription {
    /// Drains queued events and reports since-last-read loss counters.
    ///
    /// **RT contract:** non-RT — short internal queue lock plus
    /// `midiInAddBuffer` requeue calls; never blocks on OS resources
    /// beyond those bounded driver calls.
    ///
    /// # Errors
    ///
    /// [`DeviceError::StreamInvalidated`] once the stream is dead and
    /// fully drained (see [`Shared::drain`]).
    fn read(
        &mut self,
        out: &mut Vec<TimedMidiInput>,
        max_events: usize,
    ) -> DeviceResult<MidiReadStats> {
        if self.closed {
            return Err(DeviceError::StreamInvalidated);
        }
        let stats = self.shared.drain(out, max_events)?;
        // Return driver-held SysEx buffers to the queue. This is the
        // only place requeue happens — the callback may not call system
        // functions (WinMM contract).
        for buffer in &mut self.buffers {
            buffer.requeue_if_done(self.handle);
        }
        Ok(stats)
    }

    /// Stops the stream and releases the native subscription.
    /// Idempotent; also runs from `Drop` if the consumer forgets.
    ///
    /// **RT contract:** non-RT only — calls OS APIs.
    fn close(&mut self) {
        self.shutdown();
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_port(key: PortKey) -> WinMmPort {
        WinMmPort {
            info: MidiInputInfo::new(
                port_id(&key),
                "test port".to_string(),
                MidiPortOrigin::Unknown,
                MidiPortCapabilities::stamped(false, ClockDomainId::new(1)),
            ),
            key,
            domain: ClockDomainId::new(1),
            opened: AtomicBool::new(false),
        }
    }

    #[test]
    fn interface_key_with_no_matching_device_is_not_found() {
        // Deterministic on any Windows host: no device carries this id.
        let port = test_port(PortKey::Interface("no-such-interface-id".to_string()));
        assert!(matches!(port.open(), Err(DeviceError::DeviceNotFound)));
        // A failed open does not consume the one-open rule.
        assert!(!port.opened.load(Ordering::Acquire));
    }

    #[test]
    fn fallback_key_with_out_of_range_index_is_not_found() {
        let port = test_port(PortKey::Index {
            index: u32::MAX,
            name: "nonexistent".to_string(),
        });
        assert!(matches!(port.open(), Err(DeviceError::DeviceNotFound)));
    }

    #[test]
    fn enumeration_is_ok_and_ports_declare_honest_capabilities() {
        let ports = enumerate_ports();
        let mut domains = std::collections::HashSet::new();
        for port in &ports {
            let info = port.info();
            assert!(!info.id.is_empty(), "port id must be non-empty");
            assert_eq!(info.origin, MidiPortOrigin::Unknown);
            assert!(!info.capabilities.supports_ump);
            assert!(info.capabilities.provides_timestamps);
            let domain = info
                .capabilities
                .clock_domain
                .expect("stamped ports declare a domain");
            assert!(domains.insert(domain), "each port gets a unique domain");
        }
    }

    #[test]
    fn stamp_event_preserves_raw_ticks_and_declares_carried_provenance() {
        let event = MidiEvent {
            group: ruviform_media_core::MidiGroup::ZERO,
            channel: ruviform_media_core::MidiChannel::new(0).unwrap(),
            kind: ruviform_media_core::MidiKind::NoteOn {
                key: ruviform_media_core::NoteKey::new(60).unwrap(),
                velocity: ruviform_media_core::MidiValue::from_u7(100),
            },
        };
        let stamped = stamp_event(
            1234,
            ClockDomainId::new(42),
            TimeBase::new(1, 1_000).unwrap(),
            event,
        );
        assert_eq!(stamped.stamp.ticks, 1234);
        assert_eq!(stamped.stamp.base, TimeBase::new(1, 1_000).unwrap());
        assert_eq!(stamped.stamp.domain, ClockDomainId::new(42));
        assert_eq!(stamped.stamp.provenance, TimestampProvenance::Carried);
        assert_eq!(stamped.event, event);
    }

    /// Manual real-device check: `cargo test -p ruviform-midi-device
    /// -- --ignored`. Passes trivially with no devices; with devices it
    /// opens the first port, drains, and closes.
    #[test]
    #[ignore = "requires a MIDI input device; manual opt-in check"]
    fn real_device_open_read_close_smoke() {
        let ports = enumerate_ports();
        let Some(port) = ports.into_iter().next() else {
            return;
        };
        let mut input = port.open().unwrap();
        let mut out = Vec::new();
        let _stats = input.read(&mut out, 64).unwrap();
        input.close();
    }
}
