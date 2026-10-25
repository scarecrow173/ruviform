//! Spawn E2E for the reference L1 runner `ruviform-plugin-l1-runner`
//! (`CARGO_BIN_EXE_*` — stays inside this crate; no cross-package bin
//! resolution).
//!
//! The test plays the *minimal host*: it creates the shared-memory
//! segment, writes the `[header][in-slots][out-slots]` layout, spawns
//! the runner, speaks the wire protocol over the child's stdio and
//! verifies the full contract — `Hello` → `Scan` → lifecycle commands
//! → shm block processing (gain correctness) → state round-trip →
//! `Shutdown` exit(0) — plus the defined rejections (bad argv, bad
//! segment, illegal `BlockIn`/`Command` transitions, plugin panic).
//!
//! The byte layout comes from `ruviform_plugin_core::wire::shm` — the
//! authoritative wire-side definition the runner's `src/shm.rs` and
//! the proxy's `l1/shm.rs` consume; this minimal host is a third
//! consumer, so all three sides agree by construction.

use std::io::{BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use ruviform_audio_core::SampleRate;
use ruviform_media_core::{MediaTime, TimeBase, TransportStatus};
use ruviform_plugin_core::wire::{self, Command as WireCommand, Message, Reply as WireReply, shm};
use ruviform_plugin_core::{
    Event, EventKind, HostError, ParamEvent, ParamId, ParamValue, PluginDescriptor, ProcessConfig,
    ProcessStatus, TransportInfo,
};
use shared_memory::{Shmem, ShmemConf};

const RUNNER: &str = env!("CARGO_BIN_EXE_ruviform-plugin-l1-runner");
const GAIN_ID: &str = "dev.ruviform.mock.gain";
const PANIC_ID: &str = "dev.ruviform.mock.panic";
const UNDERWRITE_ID: &str = "dev.ruviform.mock.underwrite";

const GAIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.gain",
    name: "Gain",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["audio-effect"],
};

const TIMEOUT: Duration = Duration::from_secs(15);

// --- shm layout (ruviform_plugin_core::wire::shm) -----------------

const IN_DEPTH: u32 = 3;
const OUT_DEPTH: u32 = 3;
const SEGMENT_LEN: usize = shm::segment_len(IN_DEPTH, OUT_DEPTH);
const IN_OFF: usize = shm::IN_SLOTS_OFF;
const OUT_OFF: usize = shm::out_slots_off(IN_DEPTH);

// --- minimal host ---------------------------------------------------

/// A spawned runner child: stdin handle plus a background reader
/// thread that demuxes child→parent frames onto a channel so the test
/// can wait with a timeout instead of hanging on a dead child.
struct MiniHost {
    child: Child,
    /// `Option` so a test can drop the control channel to simulate a
    /// clean stdin EOF.
    stdin: Option<ChildStdin>,
    inbound: Receiver<Result<Message, String>>,
}

impl MiniHost {
    fn spawn(args: &[&str]) -> Self {
        Self::spawn_with_env(args, &[])
    }

    /// Spawn with extra environment — the `descriptor()`-panic fixture
    /// is gated on `RUVIFORM_PANIC_DESCRIPTOR`.
    fn spawn_with_env(args: &[&str], env: &[(&str, &str)]) -> Self {
        let mut command = Command::new(RUNNER);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (name, value) in env {
            command.env(name, value);
        }
        let mut child = command.spawn().expect("spawn ruviform-plugin-l1-runner");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = child.stdout.take().expect("child stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_message(&mut reader) {
                    Ok(message) => {
                        if tx.send(Ok(message)).is_err() {
                            return;
                        }
                    }
                    Err(reason) => {
                        let _ = tx.send(Err(reason));
                        return;
                    }
                }
            }
        });
        Self {
            child,
            stdin: Some(stdin),
            inbound: rx,
        }
    }

    /// Spawns the reference runner with the standard argv pair.
    fn spawn_runner(plugin_id: &str, shm_key: &str) -> Self {
        Self::spawn(&[
            "--ruviform-plugin-id",
            plugin_id,
            "--ruviform-shm-key",
            shm_key,
        ])
    }

    fn send(&mut self, message: &Message) {
        let stdin = self.stdin.as_mut().expect("stdin still open");
        let mut frame = Vec::new();
        wire::encode(message, &mut frame);
        stdin.write_all(&frame).expect("write child stdin");
        stdin.flush().expect("flush child stdin");
    }

    fn send_command(&mut self, seq: u64, cmd: WireCommand) {
        self.send(&Message::Command { seq, cmd });
    }

    fn recv(&self) -> Message {
        self.inbound
            .recv_timeout(TIMEOUT)
            .expect("timed out waiting for a child frame")
            .unwrap_or_else(|reason| panic!("child stream failed: {reason}"))
    }

    /// Receives the next frame, asserting it is a `Reply` for `seq`.
    fn recv_reply(&self, seq: u64) -> Result<WireReply, HostError> {
        match self.recv() {
            Message::Reply { seq: got, result } => {
                assert_eq!(got, seq, "reply seq mismatch");
                result
            }
            other => panic!("expected Reply for seq {seq}, got {other:?}"),
        }
    }

    fn recv_ok(&self, seq: u64) {
        assert_eq!(self.recv_reply(seq), Ok(WireReply::Ok));
    }

    /// `Hello` must be the first frame, at `WIRE_VERSION`.
    fn expect_hello(&self) {
        match self.recv() {
            Message::Hello {
                version,
                capabilities,
            } => {
                assert_eq!(version, wire::WIRE_VERSION, "Hello wire version");
                assert_eq!(capabilities, 0);
            }
            other => panic!("expected Hello, got {other:?}"),
        }
    }

    /// Waits for the child to exit within `TIMEOUT`; kills + panics
    /// on timeout so the test can never hang.
    fn wait_exit(&mut self) -> ExitStatus {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().expect("try_wait") {
                return status;
            }
            if Instant::now() > deadline {
                let _ = self.child.kill();
                panic!("child did not exit within {TIMEOUT:?}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn read_message(reader: &mut impl Read) -> Result<Message, String> {
    let mut header = [0_u8; 4];
    reader
        .read_exact(&mut header)
        .map_err(|e| format!("frame header read: {e}"))?;
    let payload_len = u32::from_le_bytes(header) as usize;
    if payload_len > wire::WIRE_MAX_PAYLOAD as usize {
        return Err("declared payload over WIRE_MAX_PAYLOAD".to_owned());
    }
    let mut frame = Vec::with_capacity(wire::WIRE_FRAME_HEADER_LEN + payload_len);
    frame.extend_from_slice(&header);
    frame.resize(wire::WIRE_FRAME_HEADER_LEN + payload_len, 0);
    reader
        .read_exact(&mut frame[4..])
        .map_err(|e| format!("frame body read: {e}"))?;
    wire::decode(&frame).map_err(|e| format!("frame decode: {e}"))
}

// --- minimal host shm side -------------------------------------------

fn unique_key(tag: &str) -> String {
    format!("/ruviform-l1-e2e-{}-{tag}", std::process::id())
}

/// Creates the segment and writes the canonical header.
fn create_segment(key: &str) -> Shmem {
    create_segment_with(key, shm::MAGIC)
}

fn create_segment_with(key: &str, magic: u32) -> Shmem {
    let mut mem = ShmemConf::new()
        .os_id(key)
        .size(SEGMENT_LEN)
        .create()
        .expect("create shared-memory segment");
    let bytes = segment_bytes_mut(&mut mem);
    put_u32(bytes, shm::OFF_MAGIC, magic);
    put_u32(bytes, shm::OFF_WIRE_VERSION, u32::from(wire::WIRE_VERSION));
    put_u32(bytes, shm::OFF_IN_DEPTH, IN_DEPTH);
    put_u32(bytes, shm::OFF_OUT_DEPTH, OUT_DEPTH);
    put_u32(
        bytes,
        shm::OFF_SLOT_BYTES,
        u32::try_from(shm::SLOT_BYTES).expect("slot size fits u32"),
    );
    put_u32(
        bytes,
        shm::OFF_IN_OFF,
        u32::try_from(IN_OFF).expect("in offset fits u32"),
    );
    put_u32(
        bytes,
        shm::OFF_OUT_OFF,
        u32::try_from(OUT_OFF).expect("out offset fits u32"),
    );
    mem
}

/// Writes an in-slot for doorbell `seq`: scalars, optional wire
/// transport, wire events and planar LE `f32` samples
/// (`samples[channel][frame]`).
#[allow(clippy::too_many_arguments)]
fn write_in_slot(
    mem: &mut Shmem,
    seq: u64,
    frames: u32,
    channels: u32,
    events: &[Event],
    transport: Option<&TransportInfo>,
    samples: &[Vec<f32>],
) {
    let index = usize::try_from(seq % u64::from(IN_DEPTH)).unwrap();
    let start = IN_OFF + index * shm::SLOT_BYTES;
    let slot = &mut segment_bytes_mut(mem)[start..start + shm::SLOT_BYTES];
    put_u64(slot, shm::SLOT_OFF_SEQ, seq);
    put_u32(slot, shm::SLOT_OFF_FRAMES, frames);
    put_u32(slot, shm::SLOT_OFF_CHANNELS, channels);
    put_u32(
        slot,
        shm::SLOT_OFF_EVENT_COUNT,
        u32::try_from(events.len()).expect("event count fits u32"),
    );
    put_u32(
        slot,
        shm::SLOT_OFF_FLAGS,
        if transport.is_some() {
            shm::FLAG_TRANSPORT
        } else {
            0
        },
    );
    slot[shm::TRANSPORT_OFF..shm::TRANSPORT_OFF + shm::TRANSPORT_LEN].fill(0);
    if let Some(info) = transport {
        let mut encoded = Vec::new();
        wire::encode_transport(info, &mut encoded).expect("transport encodes");
        slot[shm::TRANSPORT_OFF..shm::TRANSPORT_OFF + encoded.len()].copy_from_slice(&encoded);
    }
    slot[shm::RESULT_OFF..shm::RESULT_OFF + shm::RESULT_LEN].fill(0);
    let mut encoded = Vec::new();
    for event in events {
        wire::encode_event(event, &mut encoded);
    }
    slot[shm::EVENTS_OFF..shm::EVENTS_OFF + encoded.len()].copy_from_slice(&encoded);
    for (channel, channel_samples) in samples.iter().enumerate() {
        let base = shm::SAMPLES_OFF + channel * shm::SAMPLE_STRIDE;
        for (frame, &sample) in channel_samples.iter().enumerate() {
            slot[base + frame * 4..base + frame * 4 + 4].copy_from_slice(&sample.to_le_bytes());
        }
    }
}

/// Reads back the out-slot for doorbell `seq`:
/// `(frames, channels, result, samples[channel][frame])`.
fn read_out_slot(
    mem: &Shmem,
    seq: u64,
) -> (u32, u32, ruviform_plugin_core::ProcessResult, Vec<Vec<f32>>) {
    let index = usize::try_from(seq % u64::from(OUT_DEPTH)).unwrap();
    let start = OUT_OFF + index * shm::SLOT_BYTES;
    let slot = &segment_bytes(mem)[start..start + shm::SLOT_BYTES];
    assert_eq!(
        get_u64(slot, shm::SLOT_OFF_SEQ),
        seq,
        "out-slot seq must echo the doorbell"
    );
    let frames = get_u32(slot, shm::SLOT_OFF_FRAMES);
    let channels = get_u32(slot, shm::SLOT_OFF_CHANNELS);
    let result = wire::decode_process_result(&mut wire::Reader::new(
        &slot[shm::RESULT_OFF..shm::RESULT_OFF + shm::RESULT_LEN],
    ))
    .expect("out-slot result decodes");
    let mut samples = Vec::with_capacity(channels as usize);
    for channel in 0..channels as usize {
        let base = shm::SAMPLES_OFF + channel * shm::SAMPLE_STRIDE;
        let mut data = Vec::with_capacity(frames as usize);
        for frame in 0..frames as usize {
            data.push(f32::from_le_bytes(
                slot[base + frame * 4..base + frame * 4 + 4]
                    .try_into()
                    .expect("f32 is 4 bytes"),
            ));
        }
        samples.push(data);
    }
    (frames, channels, result, samples)
}

/// The segment as bytes — test side of the host.
///
/// SAFETY: the test owns the segment; the child only touches a slot
/// between its `BlockIn` doorbell and the matching `BlockDone` (strict
/// alternation), and the header/in-slot writes happen before the
/// doorbell is sent. No other handle writes the region while this
/// slice is live.
#[expect(unsafe_code)]
fn segment_bytes(mem: &Shmem) -> &[u8] {
    unsafe { mem.as_slice() }
}

/// SAFETY: see `segment_bytes` — additionally the test writes only
/// slots the child is not currently processing (in-slots before their
/// doorbell), so no concurrent access exists.
#[expect(unsafe_code)]
fn segment_bytes_mut(mem: &mut Shmem) -> &mut [u8] {
    unsafe { mem.as_slice_mut() }
}

fn get_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32 field"))
}

fn get_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64 field"))
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

// --- E2E -------------------------------------------------------------

/// Happy path: `Hello` → `Scan` (descriptor field-equality) →
/// `Initialize` → `Activate` → `StartProcessing` → shm block
/// processing with a param event (gain correctness) →
/// `SaveState`/`RestoreState` round-trip → `StopProcessing` →
/// `Deactivate` → `Shutdown` → exit(0).
#[test]
fn e2e_happy_path() {
    let key = unique_key("happy");
    let mut segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner(GAIN_ID, &key);

    host.expect_hello();

    // Scan reports all built-in factories; the gain descriptor must
    // be field-equivalent to the spec-side descriptor.
    host.send_command(0, WireCommand::Scan);
    match host.recv_reply(0) {
        Ok(WireReply::Scanned(descriptors)) => {
            assert_eq!(descriptors.len(), 3);
            let gain = descriptors
                .iter()
                .find(|d| d.id == GAIN_ID)
                .expect("gain descriptor scanned");
            assert!(
                gain.matches(&GAIN_DESCRIPTOR),
                "scanned descriptor must be field-equivalent"
            );
        }
        other => panic!("expected Scanned, got {other:?}"),
    }

    host.send_command(1, WireCommand::Initialize);
    host.recv_ok(1);
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    host.send_command(2, WireCommand::Activate { config });
    host.recv_ok(2);
    host.send_command(3, WireCommand::StartProcessing);
    host.recv_ok(3);

    // Block 0: 2ch × 4 frames + a gain event (normalized 0.75 → plain
    // 1.5) + a Playing transport snapshot.
    let inputs = [
        vec![1.0_f32, -1.0, 0.5, -0.5],
        vec![0.25_f32, -0.25, 2.0, -2.0],
    ];
    let events = [Event::new(
        0,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.75).unwrap(),
        }),
    )];
    let transport = TransportInfo::new(
        TransportStatus::Playing,
        MediaTime::new(0, TimeBase::sample_rate(48_000).unwrap()),
    );
    write_in_slot(&mut segment, 0, 4, 2, &events, Some(&transport), &inputs);
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Release);
    host.send(&Message::BlockIn { seq: 0 });
    match host.recv() {
        Message::BlockDone { seq, result } => {
            assert_eq!(seq, 0);
            assert_eq!(result.status, ProcessStatus::Continue);
        }
        other => panic!("expected BlockDone, got {other:?}"),
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Acquire);
    let (frames, channels, _result, samples) = read_out_slot(&segment, 0);
    assert_eq!((frames, channels), (4, 2));
    assert_eq!(samples[0], [1.5, -1.5, 0.75, -0.75]);
    assert_eq!(samples[1], [0.375, -0.375, 3.0, -3.0]);

    // Block 1: no events — the applied gain (1.5) persists.
    write_in_slot(&mut segment, 1, 4, 2, &[], None, &inputs);
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Release);
    host.send(&Message::BlockIn { seq: 1 });
    match host.recv() {
        Message::BlockDone { seq, result } => {
            assert_eq!(seq, 1);
            assert_eq!(result.status, ProcessStatus::Continue);
        }
        other => panic!("expected BlockDone, got {other:?}"),
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Acquire);
    let (_, _, _, samples) = read_out_slot(&segment, 1);
    assert_eq!(samples[0], [1.5, -1.5, 0.75, -0.75]);

    // State round-trip: save the gain, restore it after another value.
    host.send_command(4, WireCommand::SaveState);
    let state = match host.recv_reply(4) {
        Ok(WireReply::State { bytes }) => bytes,
        other => panic!("expected State, got {other:?}"),
    };
    host.send_command(5, WireCommand::RestoreState { bytes: state });
    host.recv_ok(5);
    // Malformed state is a defined rejection, not a crash.
    host.send_command(
        6,
        WireCommand::RestoreState {
            bytes: vec![0xDE; 3],
        },
    );
    assert_eq!(host.recv_reply(6), Err(HostError::PluginFailed));

    host.send_command(7, WireCommand::StopProcessing);
    host.recv_ok(7);
    host.send_command(8, WireCommand::Deactivate);
    host.recv_ok(8);

    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0), "Shutdown must exit(0)");
}

/// `BlockIn` while not `Processing` → `Reply` with
/// `Err(HostError::Lifecycle(_))` — the L0 `process` check, on the
/// wire. The child stays alive afterwards.
#[test]
fn e2e_blockin_requires_processing() {
    let key = unique_key("not-processing");
    let mut segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner(GAIN_ID, &key);
    host.expect_hello();

    host.send_command(0, WireCommand::Initialize);
    host.recv_ok(0);
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    host.send_command(1, WireCommand::Activate { config });
    host.recv_ok(1);

    // Activated, not Processing: BlockIn is rejected on the wire.
    write_in_slot(&mut segment, 0, 4, 1, &[], None, &[vec![1.0; 4]]);
    host.send(&Message::BlockIn { seq: 0 });
    match host.recv() {
        Message::Reply { seq, result } => {
            assert_eq!(seq, 0);
            assert!(
                matches!(result, Err(HostError::Lifecycle(_))),
                "expected Err(Lifecycle), got {result:?}"
            );
        }
        other => panic!("expected Reply, got {other:?}"),
    }

    // The child is still functional: an illegal command transition is
    // likewise a wire `Err(Lifecycle)` (`Initialize` is only legal
    // from `Created`), and Shutdown exits cleanly.
    host.send_command(2, WireCommand::Initialize);
    match host.recv_reply(2) {
        Err(HostError::Lifecycle(_)) => {}
        other => panic!("expected Err(Lifecycle), got {other:?}"),
    }
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// Missing/unknown argv → the documented `EXIT_USAGE` (2) rejection
/// without a `Hello`, no hang.
#[test]
fn e2e_argv_rejections() {
    // No arguments at all.
    let mut host = MiniHost::spawn(&[]);
    assert_eq!(
        host.wait_exit().code(),
        Some(2),
        "missing argv must exit 2 (usage)"
    );

    // Unknown argument.
    let mut host = MiniHost::spawn(&["--bogus-flag"]);
    assert_eq!(
        host.wait_exit().code(),
        Some(2),
        "unknown argv must exit 2 (usage)"
    );

    // Missing shm key only.
    let mut host = MiniHost::spawn(&["--ruviform-plugin-id", GAIN_ID]);
    assert_eq!(
        host.wait_exit().code(),
        Some(2),
        "missing shm key must exit 2 (usage)"
    );
}

/// A nonexistent shm key → attach fails → non-zero exit (after the
/// already-sent `Hello`).
#[test]
fn e2e_shm_attach_failure() {
    let key = unique_key("missing-segment");
    let mut host = MiniHost::spawn_runner(GAIN_ID, &key);
    host.expect_hello();
    assert!(
        !host.wait_exit().success(),
        "attach failure must exit non-zero"
    );
}

/// A segment with a bad header magic → header validation fails →
/// non-zero exit.
#[test]
fn e2e_shm_header_mismatch() {
    let key = unique_key("bad-magic");
    let _segment = create_segment_with(&key, 0xDEAD_BEEF);
    let mut host = MiniHost::spawn_runner(GAIN_ID, &key);
    host.expect_hello();
    assert!(
        !host.wait_exit().success(),
        "header mismatch must exit non-zero"
    );
}

/// A clean stdin EOF at a frame boundary (parent closed the control
/// channel without `Shutdown`) normalizes the lifecycle and exits 0.
#[test]
fn e2e_stdin_eof_exits_zero() {
    let key = unique_key("eof");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner(GAIN_ID, &key);
    host.expect_hello();

    drop(host.stdin.take());
    assert_eq!(host.wait_exit().code(), Some(0), "clean EOF must exit(0)");
}

/// `Hello.version != WIRE_VERSION` is a *parent-side* handshake
/// failure — the runner just reports its version; the minimal host
/// here demonstrates the documented reaction: handshake fails and the
/// parent kills the child (non-zero termination, no hang).
#[test]
fn e2e_version_mismatch_is_a_parent_side_handshake_failure() {
    // The runner reports the shared wire version verbatim; a parent
    // expecting anything else fails the handshake and kills.
    const PARENT_EXPECTED: u16 = wire::WIRE_VERSION + 1;
    let key = unique_key("version");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner(GAIN_ID, &key);

    match host.recv() {
        Message::Hello { version, .. } => {
            assert_ne!(
                version, PARENT_EXPECTED,
                "fixture: runner must not accidentally match the wrong expectation"
            );
        }
        other => panic!("expected Hello, got {other:?}"),
    }
    host.child.kill().expect("kill child");
    assert!(!host.wait_exit().success(), "killed child must not exit 0");
}

/// `PanicPlugin::init` panics → `Err(HostError::PluginFailed)` on the
/// wire; the child survives and keeps serving commands, then exits
/// cleanly on `Shutdown`.
#[test]
fn e2e_plugin_panic_is_plugin_failed_and_child_survives() {
    let key = unique_key("panic");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner(PANIC_ID, &key);
    host.expect_hello();

    host.send_command(0, WireCommand::Initialize);
    assert_eq!(host.recv_reply(0), Err(HostError::PluginFailed));

    // The child is alive and still dispatching: Scan still answers.
    host.send_command(1, WireCommand::Scan);
    match host.recv_reply(1) {
        Ok(WireReply::Scanned(descriptors)) => assert_eq!(descriptors.len(), 3),
        other => panic!("expected Scanned, got {other:?}"),
    }

    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// `PanicFactory::descriptor()` panics under the
/// `RUVIFORM_PANIC_DESCRIPTOR` env gate — during `argv` resolution
/// the panic is caught and reported as exit 1 (a plugin defect, not
/// an abort-signature 101 and not a usage exit 2).
#[test]
fn e2e_descriptor_panic_at_argv_exits_failure() {
    let key = unique_key("argv-panic");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_with_env(
        &["--ruviform-plugin-id", PANIC_ID, "--ruviform-shm-key", &key],
        &[("RUVIFORM_PANIC_DESCRIPTOR", "1")],
    );
    assert_eq!(
        host.wait_exit().code(),
        Some(1),
        "descriptor() panic during argv must exit 1, not abort"
    );
}

/// The same env-gated `descriptor()` panic during a `Scan` command is
/// caught and mapped to `Err(HostError::PluginFailed)` — the child
/// survives and keeps serving commands (symmetric with `create()`).
///
/// Selecting `GAIN_ID` is deliberate: `argv` resolution stops at the
/// first matching factory, so `PanicFactory::descriptor()` is never
/// invoked before `Scan` runs it.
#[test]
fn e2e_descriptor_panic_at_scan_is_plugin_failed() {
    let key = unique_key("scan-panic");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_with_env(
        &["--ruviform-plugin-id", GAIN_ID, "--ruviform-shm-key", &key],
        &[("RUVIFORM_PANIC_DESCRIPTOR", "1")],
    );
    host.expect_hello();

    host.send_command(0, WireCommand::Scan);
    assert_eq!(host.recv_reply(0), Err(HostError::PluginFailed));

    // Survived: a normal command still answers.
    host.send_command(1, WireCommand::Initialize);
    host.recv_ok(1);
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// `UnderwritePlugin` writes all channels on its first `process` call
/// but only channel 0 afterwards — a plugin may legally under-write
/// the out-view, so the runner must re-zero `staging_out` per block
/// rather than leak the previous block's samples into the out-slot.
#[test]
fn e2e_underwritten_channels_are_zero_filled() {
    let key = unique_key("underwrite");
    let mut segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner(UNDERWRITE_ID, &key);
    host.expect_hello();

    host.send_command(0, WireCommand::Initialize);
    host.recv_ok(0);
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    host.send_command(1, WireCommand::Activate { config });
    host.recv_ok(1);
    host.send_command(2, WireCommand::StartProcessing);
    host.recv_ok(2);

    let inputs = [vec![1.0_f32, 1.0, 1.0, 1.0], vec![7.0_f32, 7.0, 7.0, 7.0]];
    // Block 0: all channels written — the stale seed for block 1.
    write_in_slot(&mut segment, 0, 4, 2, &[], None, &inputs);
    host.send(&Message::BlockIn { seq: 0 });
    match host.recv() {
        Message::BlockDone { seq, .. } => assert_eq!(seq, 0),
        other => panic!("expected BlockDone, got {other:?}"),
    }
    let (_, _, _, samples) = read_out_slot(&segment, 0);
    assert_eq!(samples[0], [1.0; 4]);
    assert_eq!(samples[1], [7.0; 4]);

    // Block 1: the plugin under-writes (channel 0 only) — channel 1
    // must come out zeroed, not as block 0's leftover `7.0`s.
    write_in_slot(&mut segment, 1, 4, 2, &[], None, &inputs);
    host.send(&Message::BlockIn { seq: 1 });
    match host.recv() {
        Message::BlockDone { seq, .. } => assert_eq!(seq, 1),
        other => panic!("expected BlockDone, got {other:?}"),
    }
    let (_, _, _, samples) = read_out_slot(&segment, 1);
    assert_eq!(samples[0], [1.0; 4]);
    assert_eq!(
        samples[1], [0.0; 4],
        "under-written channel must be zero, not stale block-0 samples"
    );

    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}
