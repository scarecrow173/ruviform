//! `WinMM` (`winmm.dll`) MIDI output backend — Windows only.
//!
//! The send-direction counterpart of `os/windows.rs`: this module is
//! the single place `midiOut*` types, constants and error codes are
//! named; everything crossing the port boundary is normalized onto
//! `DeviceError` and the `midi_output` trait objects. Channel-voice
//! events encode through [`crate::midi::encode_message`] and go out
//! via `midiOutShortMsg`; `SysEx` byte strings go out via a prepared
//! `MIDIHDR` + `midiOutLongMsg`, waited on `MM_MOM_DONE` so the
//! buffer is unprepared only after the driver released it.
//!
//! Port identity is index + name only: `midiOutMessage` with
//! `DRV_QUERYDEVICEINTERFACE` is deliberately never called — the
//! 1st-slice probe recorded a crash on Microsoft GS Wavetable Synth
//! (`docs/impl/exp` E2 for the issue-629 survey). Enumeration is
//! infallible at the top level (`midiOutGetNumDevs` cannot fail;
//! devices vanishing mid-scan are skipped).
//!
//! **RT contract:** non-RT — every method calls OS APIs and may
//! allocate; `send_sysex` additionally blocks on the driver completion
//! callback (control path only, never the audio thread).

// The whole module is the FFI boundary: every `unsafe` block below is
// individually justified at its site, and the lint is scoped to this
// file only (workspace policy: local allowance + SAFETY comments).
#![expect(unsafe_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use ruviform_audio_core::device::{DeviceError, DeviceResult};
use ruviform_audio_core::midi_output::{
    MidiOutput, MidiOutputInfo, MidiOutputPort, MidiPortCapabilities, MidiPortOrigin,
};
use windows_sys::Win32::Media::Audio::{
    self, CALLBACK_FUNCTION, HMIDIOUT, MIDI_IO_STATUS, MIDIHDR, MIDIOUTCAPSW,
};
use windows_sys::Win32::Media::{
    MM_MOM_DONE, MMSYSERR_ALLOCATED, MMSYSERR_BADDEVICEID, MMSYSERR_NODRIVER, MMSYSERR_NOERROR,
};

use crate::midi;

#[expect(clippy::cast_possible_truncation)]
const SIZE_OF_MIDIHDR: u32 = size_of::<MIDIHDR>() as u32;

/// State shared between an open subscription and its driver callback.
/// The `Arc` is owned by the subscription and outlives `midiOutClose`,
/// so the callback's raw `dwInstance` pointer stays valid for the
/// handle's whole callback lifetime (same argument as the input side).
struct OutShared {
    /// Set by the callback on `MM_MOM_DONE`; `send_sysex` waits on it.
    /// `Mutex` + `Condvar` because the callback may not allocate.
    sysex_done: Mutex<bool>,
    sysex_cv: Condvar,
    /// Set once `close` begins; the callback checks it first so a late
    /// `MM_MOM_DONE` does not touch state mid-teardown.
    closing: AtomicBool,
}

impl OutShared {
    fn new() -> Self {
        Self {
            sysex_done: Mutex::new(false),
            sysex_cv: Condvar::new(),
            closing: AtomicBool::new(false),
        }
    }
}

/// Enumerates the MIDI output ports `WinMM` currently reports.
///
/// **RT contract:** non-RT only — calls OS APIs and allocates.
///
/// Cannot fail at the top level: `midiOutGetNumDevs` is infallible and
/// devices that vanish mid-scan are skipped, so the port contract's
/// `DeviceResult` is supplied by `SystemBackend::output_ports`.
pub(crate) fn enumerate_output_ports() -> Vec<Box<dyn MidiOutputPort>> {
    // SAFETY: `midiOutGetNumDevs` takes no arguments and cannot fail.
    let count = unsafe { Audio::midiOutGetNumDevs() };
    let mut ports = Vec::with_capacity(count as usize);
    for index in 0..count {
        // A device that vanishes between the count query and the caps
        // query is skipped — enumeration reports what it could read.
        if let Ok(port) = WinMmOutputPort::enumerate_at(index) {
            ports.push(Box::new(port) as Box<dyn MidiOutputPort>);
        }
    }
    ports
}

/// A live handle for one enumerated `WinMM` output port.
struct WinMmOutputPort {
    info: MidiOutputInfo,
    /// Device index captured at enumeration, re-verified against the
    /// reported name at `open` (index + name fallback — the only
    /// identity mechanism allowed for output).
    index: u32,
    name: String,
    opened: AtomicBool,
}

impl WinMmOutputPort {
    /// Reads caps for `index` and builds the port.
    fn enumerate_at(index: u32) -> DeviceResult<Self> {
        let caps = output_caps(index)?;
        let name = caps_name(&caps);
        let info = MidiOutputInfo::new(
            format!("winmm-out:{index}:{name}"),
            name.clone(),
            // WinMM exposes no external/virtual/internal distinction —
            // report Unknown rather than guess.
            MidiPortOrigin::Unknown,
            // MIDI 1.0 byte stream only; the driver stamps nothing on
            // the send direction, so no timestamps and no clock domain.
            MidiPortCapabilities::midi1_unstamped(),
        );
        Ok(Self {
            info,
            index,
            name,
            opened: AtomicBool::new(false),
        })
    }

    /// `index` still exists and still reports this port's name.
    fn resolve_index(&self) -> DeviceResult<u32> {
        if index_still_matches(self.index, &self.name) {
            Ok(self.index)
        } else {
            Err(DeviceError::DeviceNotFound)
        }
    }

    fn open_inner(&self) -> DeviceResult<Box<dyn MidiOutput>> {
        let index = self.resolve_index()?;
        open_at_index(index)
    }
}

impl MidiOutputPort for WinMmOutputPort {
    /// The port's identity and declared capabilities.
    ///
    /// **RT contract:** non-RT only — clones owned strings.
    fn info(&self) -> MidiOutputInfo {
        self.info.clone()
    }

    /// Opens a `WinMM` output subscription on this port.
    ///
    /// **RT contract:** non-RT only — calls OS APIs and allocates.
    fn open(&self) -> DeviceResult<Box<dyn MidiOutput>> {
        if self.opened.swap(true, Ordering::AcqRel) {
            return Err(DeviceError::DeviceBusy);
        }
        match self.open_inner() {
            Ok(output) => Ok(output),
            Err(err) => {
                // A failed open never started a subscription, so the
                // one-open rule is not consumed — allow a retry.
                self.opened.store(false, Ordering::Release);
                Err(err)
            }
        }
    }
}

/// `midiOutGetDevCapsW` for one device index.
#[expect(clippy::cast_possible_truncation)]
fn output_caps(index: u32) -> DeviceResult<MIDIOUTCAPSW> {
    let mut caps = MIDIOUTCAPSW::default();
    // SAFETY: `caps` is a valid out-struct of the declared size.
    let rc = unsafe {
        Audio::midiOutGetDevCapsW(
            index as usize,
            &raw mut caps,
            size_of::<MIDIOUTCAPSW>() as u32,
        )
    };
    if rc == MMSYSERR_NOERROR {
        Ok(caps)
    } else {
        Err(DeviceError::DeviceNotFound)
    }
}

/// Decodes the fixed-width UTF-16 `szPname` field (NUL-terminated,
/// max 32 units). `MIDIOUTCAPSW` is `repr(packed)` — copy the field to
/// a local before taking any reference (a `&[u16]` into the packed
/// field would be an unaligned reference, which is UB).
fn caps_name(caps: &MIDIOUTCAPSW) -> String {
    let sz_pname = caps.szPname;
    let end = sz_pname
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(sz_pname.len());
    String::from_utf16_lossy(&sz_pname[..end])
}

/// Fallback-key check: `index` still exists and still reports `name`.
fn index_still_matches(index: u32, name: &str) -> bool {
    output_caps(index).is_ok_and(|caps| caps_name(&caps) == name)
}

/// Opens a `CALLBACK_FUNCTION | MIDI_IO_STATUS` subscription on the
/// device at `index`.
fn open_at_index(index: u32) -> DeviceResult<Box<dyn MidiOutput>> {
    let shared = Arc::new(OutShared::new());
    let shared_ptr = Arc::as_ptr(&shared);

    let mut handle: HMIDIOUT = core::ptr::null_mut();
    // SAFETY: `handle` is a valid out-pointer; `midi_out_proc` is a
    // valid `extern "system"` callback; `shared_ptr` is the
    // `Arc::as_ptr` of the `OutShared` the returned `Subscription`
    // owns — the Arc (and thus the pointee) outlives `midiOutClose`,
    // and WinMM guarantees no callback fires after close returns.
    let rc = unsafe {
        Audio::midiOutOpen(
            &raw mut handle,
            index,
            midi_out_proc as extern "system" fn(HMIDIOUT, u32, usize, usize, usize) as usize,
            shared_ptr as usize,
            CALLBACK_FUNCTION | MIDI_IO_STATUS,
        )
    };
    if rc != MMSYSERR_NOERROR {
        return Err(match rc {
            MMSYSERR_BADDEVICEID | MMSYSERR_NODRIVER => DeviceError::DeviceNotFound,
            MMSYSERR_ALLOCATED => DeviceError::DeviceBusy,
            _ => DeviceError::Backend("winmm midiOutOpen failed"),
        });
    }

    Ok(Box::new(Subscription {
        shared,
        handle,
        closed: false,
    }))
}

/// The `WinMM` driver callback (`CALLBACK_FUNCTION`). Runs on a system
/// thread — must stay minimal: flag the `SysEx` completion. No
/// allocation, no OS calls.
///
/// # Safety of the `instance` pointer
///
/// `dwInstance` is the `Arc::as_ptr` of the `OutShared` owned by the
/// [`Subscription`]. The Arc is released only when the `Subscription`
/// is dropped, which happens strictly after `midiOutClose` returns;
/// `WinMM` guarantees no callback fires after close. The pointer is
/// therefore valid for the callback's entire lifetime.
extern "system" fn midi_out_proc(
    _hmo: HMIDIOUT,
    msg: u32,
    instance: usize,
    _param1: usize,
    _param2: usize,
) {
    // SAFETY: see the doc comment above — `instance` points at a live
    // `OutShared` for the handle's whole callback lifetime.
    let shared = unsafe { &*(instance as *const OutShared) };
    if shared.closing.load(Ordering::Acquire) {
        return;
    }
    if msg == MM_MOM_DONE {
        // A prepared long-message buffer finished — wake the
        // `send_sysex` waiter so it can unprepare. `MM_MOM_DONE` is
        // the only completion signal the send path needs; open/close
        // notifications carry no payload we use.
        let mut done = shared
            .sysex_done
            .lock()
            .expect("winmm output shared state poisoned");
        *done = true;
        shared.sysex_cv.notify_one();
    }
}

/// An open `WinMM` output subscription.
struct Subscription {
    shared: Arc<OutShared>,
    handle: HMIDIOUT,
    closed: bool,
}

impl Subscription {
    /// `close`/`Drop` shared teardown — idempotent via `closed`.
    fn shutdown(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        // Stop new callback work first; `OutShared` stays alive via
        // the Arc until this struct drops, so in-flight callbacks
        // remain memory-safe.
        self.shared.closing.store(true, Ordering::Release);
        // SAFETY: `handle` is open and `closed` guarantees this runs at
        // most once; `Drop` order keeps `shared` alive through the
        // call. `midiOutReset` flushes pending long messages (each
        // completion still lands on the live `OutShared`), then close.
        unsafe {
            let _ = Audio::midiOutReset(self.handle);
            let _ = Audio::midiOutClose(self.handle);
        }
    }
}

/// Maps a `midiOut*` driver error onto `DeviceError`. `what` is the
/// static classification name carried by [`DeviceError::Backend`].
fn send_error(rc: u32, what: &'static str) -> DeviceError {
    match rc {
        // The device vanished or the handle died — the subscription is
        // dead, not merely busy.
        MMSYSERR_BADDEVICEID | MMSYSERR_NODRIVER => DeviceError::StreamInvalidated,
        MMSYSERR_ALLOCATED => DeviceError::DeviceBusy,
        _ => DeviceError::Backend(what),
    }
}

impl MidiOutput for Subscription {
    /// Sends one channel-voice event through `midiOutShortMsg`.
    ///
    /// **RT contract:** non-RT — one bounded driver call; must not
    /// block on realtime resources.
    ///
    /// # Errors
    ///
    /// [`DeviceError::Unsupported`] when the event is outside the
    /// MIDI 1.0 channel-voice vocabulary this port can carry (e.g. a
    /// non-zero UMP group); driver errors normalized via
    /// [`send_error`]; [`DeviceError::StreamInvalidated`] after close.
    fn send(&mut self, event: ruviform_media_core::MidiEvent) -> DeviceResult<()> {
        if self.closed {
            return Err(DeviceError::StreamInvalidated);
        }
        let Some((status, data1, data2)) = midi::encode_message(event) else {
            return Err(DeviceError::Unsupported);
        };
        // Same packing `MIM_DATA` uses on the input side:
        // status | data1<<8 | data2<<16.
        let dwmsg = u32::from(status) | (u32::from(data1) << 8) | (u32::from(data2) << 16);
        // SAFETY: `handle` is open for the subscription's lifetime;
        // `dwmsg` is a pure value argument.
        let rc = unsafe { Audio::midiOutShortMsg(self.handle, dwmsg) };
        if rc == MMSYSERR_NOERROR {
            Ok(())
        } else {
            Err(send_error(rc, "winmm midiOutShortMsg failed"))
        }
    }

    /// Sends one complete `SysEx` byte string (`0xF0` … `0xF7`) through a
    /// prepared `MIDIHDR` + `midiOutLongMsg`, blocking until the driver
    /// reports `MM_MOM_DONE` so the buffer is unprepared only after
    /// the driver released it.
    ///
    /// **RT contract:** non-RT — calls OS APIs, allocates the message
    /// copy, and waits on the driver completion callback; never call
    /// from the audio thread.
    ///
    /// # Errors
    ///
    /// [`DeviceError::Unsupported`] when `bytes` is not a complete
    /// `SysEx` message or exceeds `u32` length; driver errors normalized
    /// via [`send_error`]; [`DeviceError::StreamInvalidated`] after
    /// close. A stalled device stalls this call — the control path
    /// owns that risk, mirroring every synchronous driver send.
    fn send_sysex(&mut self, bytes: &[u8]) -> DeviceResult<()> {
        if self.closed {
            return Err(DeviceError::StreamInvalidated);
        }
        // Boundary validation: the port contract requires a complete
        // SysEx (`0xF0` … `0xF7`); anything else cannot be carried.
        if bytes.len() < 2 || bytes[0] != 0xF0 || bytes[bytes.len() - 1] != 0xF7 {
            return Err(DeviceError::Unsupported);
        }
        let len = u32::try_from(bytes.len()).map_err(|_| DeviceError::Unsupported)?;

        // The driver reads `lpData` until `MM_MOM_DONE`; both the
        // header and the byte copy stay alive on this stack frame for
        // the whole prepare → longmsg → done → unprepare sequence.
        let mut data = bytes.to_vec().into_boxed_slice();
        let mut header = Box::new(MIDIHDR::default());
        header.lpData = data.as_mut_ptr().cast();
        header.dwBufferLength = len;
        // SAFETY: `header` is a valid `MIDIHDR` whose `lpData`/
        // `dwBufferLength` describe the live `data` allocation;
        // `handle` is open.
        let rc =
            unsafe { Audio::midiOutPrepareHeader(self.handle, &raw mut *header, SIZE_OF_MIDIHDR) };
        if rc != MMSYSERR_NOERROR {
            return Err(send_error(rc, "winmm midiOutPrepareHeader failed"));
        }

        // Arm the completion flag before queueing so a fast driver
        // cannot complete before we start waiting. Sends serialize on
        // `&mut self`, and the previous send's `MM_MOM_DONE` was
        // already consumed — no stale flag can leak into this wait.
        {
            let mut done = self
                .shared
                .sysex_done
                .lock()
                .expect("winmm output shared state poisoned");
            *done = false;
        }
        // SAFETY: `header` is prepared against `handle`; `data` outlives
        // the call and the wait below.
        let rc = unsafe { Audio::midiOutLongMsg(self.handle, &raw const *header, SIZE_OF_MIDIHDR) };
        if rc != MMSYSERR_NOERROR {
            // SAFETY: `header` is prepared; unpreparing releases the
            // driver-side reference before `data`/`header` drop.
            unsafe {
                let _ =
                    Audio::midiOutUnprepareHeader(self.handle, &raw mut *header, SIZE_OF_MIDIHDR);
            }
            return Err(send_error(rc, "winmm midiOutLongMsg failed"));
        }

        // Wait for the driver's `MM_MOM_DONE`. The callback takes the
        // same mutex, so the flag cannot be missed between check and
        // wait.
        let mut done = self
            .shared
            .sysex_done
            .lock()
            .expect("winmm output shared state poisoned");
        while !*done {
            done = self
                .shared
                .sysex_cv
                .wait(done)
                .expect("winmm output shared state poisoned");
        }
        drop(done);

        // SAFETY: `MM_MOM_DONE` means the driver finished reading
        // `lpData`; unpreparing now is the required release order.
        unsafe {
            let _ = Audio::midiOutUnprepareHeader(self.handle, &raw mut *header, SIZE_OF_MIDIHDR);
        }
        Ok(())
    }

    /// Resets pending output and releases the native subscription.
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
    use ruviform_media_core::{MidiChannel, MidiGroup, MidiKind, MidiValue, NoteKey};

    fn test_port(index: u32, name: &str) -> WinMmOutputPort {
        WinMmOutputPort {
            info: MidiOutputInfo::new(
                format!("winmm-out:{index}:{name}"),
                name.to_string(),
                MidiPortOrigin::Unknown,
                MidiPortCapabilities::midi1_unstamped(),
            ),
            index,
            name: name.to_string(),
            opened: AtomicBool::new(false),
        }
    }

    #[test]
    fn fallback_key_with_out_of_range_index_is_not_found() {
        let port = test_port(u32::MAX, "nonexistent");
        assert!(matches!(port.open(), Err(DeviceError::DeviceNotFound)));
        // A failed open does not consume the one-open rule.
        assert!(!port.opened.load(Ordering::Acquire));
    }

    #[test]
    fn enumeration_is_ok_and_ports_declare_honest_capabilities() {
        let ports = enumerate_output_ports();
        for port in &ports {
            let info = port.info();
            assert!(!info.id.is_empty(), "port id must be non-empty");
            assert_eq!(info.origin, MidiPortOrigin::Unknown);
            assert!(!info.capabilities.supports_ump);
            assert!(!info.capabilities.provides_timestamps);
            assert_eq!(info.capabilities.clock_domain, None);
        }
    }

    /// Manual real-device check: `cargo test -p ruviform-midi-device
    /// -- --ignored`. Passes trivially with no devices; with devices it
    /// opens the first port, sends channel-voice + `SysEx`, and closes.
    #[test]
    #[ignore = "requires a MIDI output device; manual opt-in check"]
    fn real_device_send_smoke() {
        let ports = enumerate_output_ports();
        let Some(port) = ports.into_iter().next() else {
            // No output device on this host — record honestly, not a
            // failure (the `test-midi-real-device` lane owns real
            // evidence).
            return;
        };
        let mut output = port.open().unwrap();
        let note_on = ruviform_media_core::MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::NoteOn {
                key: NoteKey::new(60).unwrap(),
                velocity: MidiValue::from_u7(100),
            },
        };
        output.send(note_on).unwrap();
        output.send_sysex(&[0xF0, 0x7E, 0x00, 0xF7]).unwrap();
        output.close();
    }
}
