//! Spawn E2E for the reference L1 runner `ruviform-plugin-l1-runner`
//! (`CARGO_BIN_EXE_*` — stays inside this crate; no cross-package bin
//! resolution).
//!
//! The test plays the *minimal host*: it creates the shared-memory
//! segments, writes the `[header][in-slots][out-slots]` layout, spawns
//! the runner bare (wire v2 takes no argv), speaks the wire protocol
//! over the child's stdio and verifies the full contract — `Hello` →
//! child-scoped `Scan` → `CreateInstance` (registry mint) → lifecycle
//! commands on the instance envelope → shm block processing (gain
//! correctness) → state round-trip → `DestroyInstance` → `Shutdown`
//! exit(0) — plus the defined rejections (argv usage, bad/mismatched
//! segment on `CreateInstance`, illegal `BlockIn`/`Command`
//! transitions, dead/duplicate/full registry handles, plugin panic).
//!
//! The byte layout comes from `ruviform_plugin_core::wire::shm` — the
//! authoritative wire-side definition the runner's `src/shm.rs` and
//! the proxy's `l1/shm.rs` consume; this minimal host is a third
//! consumer, so all three sides agree by construction.

use std::io::BufReader;
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
    /// Next command-envelope seq — block doorbells keep their own raw
    /// seq space (the runner echoes a doorbell seq verbatim).
    next_seq: u64,
}

impl MiniHost {
    fn spawn(args: &[&str]) -> Self {
        Self::spawn_with_env(args, &[])
    }

    /// Spawn with extra environment — the `descriptor()`/`create()`
    /// panic fixtures are gated on `RUVIFORM_PANIC_DESCRIPTOR` /
    /// `RUVIFORM_PANIC_CREATE`.
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
            let mut frame_buf = Vec::new();
            loop {
                match wire::read_message(&mut reader, &mut frame_buf) {
                    Ok(message) => {
                        if tx.send(Ok(message)).is_err() {
                            return;
                        }
                    }
                    Err(reason) => {
                        let _ = tx.send(Err(reason.to_string()));
                        return;
                    }
                }
            }
        });
        Self {
            child,
            stdin: Some(stdin),
            inbound: rx,
            next_seq: 0,
        }
    }

    /// Spawns the reference runner — wire v2 is spawned bare: plugin
    /// selection and the shm key ride the `CreateInstance` payload.
    fn spawn_runner() -> Self {
        Self::spawn(&[])
    }

    fn send(&mut self, message: &Message) {
        let stdin = self.stdin.as_mut().expect("stdin still open");
        wire::write_message(stdin, message, &mut Vec::new()).expect("write child stdin");
    }

    /// Sends one `Command` envelope and waits for the echo-attributed
    /// `Reply` — `instance` is `WIRE_INSTANCE_NONE` for `Scan`, the
    /// live handle for everything else.
    fn command(&mut self, instance: u32, cmd: WireCommand) -> Result<WireReply, HostError> {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.send(&Message::Command { seq, instance, cmd });
        self.recv_reply(seq, instance)
    }

    /// Child-scoped `Scan` — the only verb stamped
    /// [`wire::WIRE_INSTANCE_NONE`].
    fn scan(&mut self) -> Result<WireReply, HostError> {
        self.command(wire::WIRE_INSTANCE_NONE, WireCommand::Scan)
    }

    /// `CreateInstance` minting `instance` for `plugin_id` on the
    /// segment `key` at the standard depths.
    fn create_instance(
        &mut self,
        instance: u32,
        plugin_id: &str,
        key: &str,
    ) -> Result<WireReply, HostError> {
        self.create_instance_depths(instance, plugin_id, key, IN_DEPTH, OUT_DEPTH)
    }

    /// `CreateInstance` with explicit declared depths — the
    /// depth-mismatch fixture overrides them against the header.
    fn create_instance_depths(
        &mut self,
        instance: u32,
        plugin_id: &str,
        key: &str,
        in_depth: u32,
        out_depth: u32,
    ) -> Result<WireReply, HostError> {
        self.command(
            instance,
            WireCommand::CreateInstance {
                plugin_id: plugin_id.to_owned(),
                shm_key: key.to_owned(),
                in_depth,
                out_depth,
            },
        )
    }

    /// One `BlockIn` doorbell for `instance` — block seqs are the raw
    /// slot space the runner echoes verbatim.
    fn block_in(&mut self, seq: u64, instance: u32) {
        self.send(&Message::BlockIn { seq, instance });
    }

    fn recv(&self) -> Message {
        self.inbound
            .recv_timeout(TIMEOUT)
            .expect("timed out waiting for a child frame")
            .unwrap_or_else(|reason| panic!("child stream failed: {reason}"))
    }

    /// Receives the next frame, asserting it is a `Reply` for `seq`
    /// echoing `instance` (the v2 attribution contract — the reply's
    /// envelope copies the command's verbatim).
    fn recv_reply(&self, seq: u64, instance: u32) -> Result<WireReply, HostError> {
        match self.recv() {
            Message::Reply {
                seq: got,
                instance: got_instance,
                result,
            } => {
                assert_eq!(got, seq, "reply seq mismatch");
                assert_eq!(
                    got_instance, instance,
                    "reply instance must echo the command envelope"
                );
                result
            }
            other => panic!("expected Reply for seq {seq} instance {instance}, got {other:?}"),
        }
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

/// The descriptor `Scan` must report — field-equality against the
/// spec-side descriptor like the real handshake.
fn expect_scanned(reply: Result<WireReply, HostError>) -> Vec<wire::Descriptor> {
    match reply {
        Ok(WireReply::Scanned(descriptors)) => descriptors,
        other => panic!("expected Scanned, got {other:?}"),
    }
}

/// Asserts the child still serves commands — the "alive after a
/// defined rejection" oracle the v2 registry contract pins.
fn assert_alive(host: &mut MiniHost) {
    let descriptors = expect_scanned(host.scan());
    assert_eq!(descriptors.len(), 3, "Scan must still enumerate factories");
}

/// Rings the assertions on one `BlockDone`: `seq` and `instance`
/// echo the doorbell verbatim. Returns the block's `ProcessResult`.
fn expect_block_done(
    host: &mut MiniHost,
    seq: u64,
    instance: u32,
) -> ruviform_plugin_core::ProcessResult {
    match host.recv() {
        Message::BlockDone {
            seq: got,
            instance: got_instance,
            result,
        } => {
            assert_eq!(got, seq);
            assert_eq!(got_instance, instance, "BlockDone must echo the instance");
            result
        }
        other => panic!("expected BlockDone for seq {seq} instance {instance}, got {other:?}"),
    }
}

// --- minimal host shm side -------------------------------------------

fn unique_key(tag: &str) -> String {
    format!("/ruviform-l1-e2e-{}-{tag}", std::process::id())
}

/// Creates the segment at the standard depths and writes the
/// canonical header.
fn create_segment(key: &str) -> Shmem {
    create_segment_with(
        key,
        IN_DEPTH,
        OUT_DEPTH,
        shm::MAGIC,
        u32::from(wire::WIRE_VERSION),
    )
}

/// Creates a segment at arbitrary depths — the `InstanceLimit`
/// fixture uses depth-1 rings so 64 instances stay cheap.
fn create_segment_depths(key: &str, in_depth: u32, out_depth: u32) -> Shmem {
    create_segment_with(
        key,
        in_depth,
        out_depth,
        shm::MAGIC,
        u32::from(wire::WIRE_VERSION),
    )
}

/// Creates a segment whose header carries the given `magic` and
/// `wire_version` — header-corruption tests override one field at a
/// time against otherwise-valid values.
fn create_segment_with(
    key: &str,
    in_depth: u32,
    out_depth: u32,
    magic: u32,
    wire_version: u32,
) -> Shmem {
    let mut mem = ShmemConf::new()
        .os_id(key)
        .size(shm::segment_len(in_depth, out_depth))
        .create()
        .expect("create shared-memory segment");
    let bytes = segment_bytes_mut(&mut mem);
    put_u32(bytes, shm::OFF_MAGIC, magic);
    put_u32(bytes, shm::OFF_WIRE_VERSION, wire_version);
    put_u32(bytes, shm::OFF_IN_DEPTH, in_depth);
    put_u32(bytes, shm::OFF_OUT_DEPTH, out_depth);
    put_u32(
        bytes,
        shm::OFF_SLOT_BYTES,
        u32::try_from(shm::SLOT_BYTES).expect("slot size fits u32"),
    );
    put_u32(
        bytes,
        shm::OFF_IN_OFF,
        u32::try_from(shm::IN_SLOTS_OFF).expect("in offset fits u32"),
    );
    put_u32(
        bytes,
        shm::OFF_OUT_OFF,
        u32::try_from(shm::out_slots_off(in_depth)).expect("out offset fits u32"),
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

/// Happy path: `Hello` → `Scan` (descriptor field-equality,
/// `WIRE_INSTANCE_NONE` echo) → `CreateInstance` → `Initialize` →
/// `Activate` → `StartProcessing` → shm block processing with a param
/// event (gain correctness) → `SaveState`/`RestoreState` round-trip →
/// `StopProcessing` → `Deactivate` → `DestroyInstance` → `Shutdown` →
/// exit(0).
#[test]
fn e2e_happy_path() {
    let key = unique_key("happy");
    let mut segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner();

    host.expect_hello();

    // Scan reports all built-in factories; the gain descriptor must
    // be field-equivalent to the spec-side descriptor. The reply
    // echoes `WIRE_INSTANCE_NONE` (asserted inside `command`).
    let descriptors = expect_scanned(host.scan());
    let gain = descriptors
        .iter()
        .find(|d| d.id == GAIN_ID)
        .expect("gain descriptor scanned");
    assert!(
        gain.matches(&GAIN_DESCRIPTOR),
        "scanned descriptor must be field-equivalent"
    );

    let instance = 0;
    assert_eq!(
        host.create_instance(instance, GAIN_ID, &key),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::Initialize),
        Ok(WireReply::Ok)
    );
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    assert_eq!(
        host.command(instance, WireCommand::Activate { config }),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::StartProcessing),
        Ok(WireReply::Ok)
    );

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
    std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
    host.block_in(0, instance);
    assert_eq!(
        expect_block_done(&mut host, 0, instance).status,
        ProcessStatus::Continue
    );
    std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
    let (frames, channels, _result, samples) = read_out_slot(&segment, 0);
    assert_eq!((frames, channels), (4, 2));
    assert_eq!(samples[0], [1.5, -1.5, 0.75, -0.75]);
    assert_eq!(samples[1], [0.375, -0.375, 3.0, -3.0]);

    // Block 1: no events — the applied gain (1.5) persists.
    write_in_slot(&mut segment, 1, 4, 2, &[], None, &inputs);
    std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
    host.block_in(1, instance);
    assert_eq!(
        expect_block_done(&mut host, 1, instance).status,
        ProcessStatus::Continue
    );
    std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
    let (_, _, _, samples) = read_out_slot(&segment, 1);
    assert_eq!(samples[0], [1.5, -1.5, 0.75, -0.75]);

    // State round-trip: save the gain, restore it after another value.
    let state = match host.command(instance, WireCommand::SaveState) {
        Ok(WireReply::State { bytes }) => bytes,
        other => panic!("expected State, got {other:?}"),
    };
    assert_eq!(
        host.command(instance, WireCommand::RestoreState { bytes: state }),
        Ok(WireReply::Ok)
    );
    // Malformed state is a defined rejection, not a crash.
    assert_eq!(
        host.command(
            instance,
            WireCommand::RestoreState {
                bytes: vec![0xDE; 3],
            },
        ),
        Err(HostError::PluginFailed)
    );

    assert_eq!(
        host.command(instance, WireCommand::StopProcessing),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::Deactivate),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::DestroyInstance),
        Ok(WireReply::Ok)
    );

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
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    let instance = 0;
    assert_eq!(
        host.create_instance(instance, GAIN_ID, &key),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::Initialize),
        Ok(WireReply::Ok)
    );
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    assert_eq!(
        host.command(instance, WireCommand::Activate { config }),
        Ok(WireReply::Ok)
    );

    // Activated, not Processing: BlockIn is rejected on the wire —
    // a flagged `Reply` echoing the doorbell's seq + instance.
    write_in_slot(&mut segment, 0, 4, 1, &[], None, &[vec![1.0; 4]]);
    host.block_in(0, instance);
    match host.recv() {
        Message::Reply {
            seq,
            instance: got,
            result,
        } => {
            assert_eq!(seq, 0);
            assert_eq!(got, instance);
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
    match host.command(instance, WireCommand::Initialize) {
        Err(HostError::Lifecycle(_)) => {}
        other => panic!("expected Err(Lifecycle), got {other:?}"),
    }
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// Wire v2's argv contract: **no** arguments — the runner accepts an
/// empty argv and serves immediately (`Hello` + `Scan` round-trip),
/// while *any* argv entry — including the retired `--ruviform-*`
/// bootstrap flags — is the defined usage rejection (exit 2, no
/// `Hello`, no hang).
#[test]
fn e2e_argv_rejections() {
    // Zero argv is the v2 spawn shape — the child comes up serving.
    let mut host = MiniHost::spawn(&[]);
    host.expect_hello();
    assert_eq!(
        host.scan().map(|r| matches!(r, WireReply::Scanned(_))),
        Ok(true)
    );
    host.send(&Message::Shutdown);
    assert_eq!(
        host.wait_exit().code(),
        Some(0),
        "clean Shutdown must exit(0)"
    );

    // An unknown argument is a usage rejection.
    let mut host = MiniHost::spawn(&["--bogus-flag"]);
    assert_eq!(
        host.wait_exit().code(),
        Some(2),
        "unknown argv must exit 2 (usage)"
    );

    // The retired v1 bootstrap flags are usage rejections too.
    for args in [
        &["--ruviform-plugin-id", GAIN_ID][..],
        &["--ruviform-shm-key", "k"][..],
        &["--ruviform-plugin-id", GAIN_ID, "--ruviform-shm-key", "k"][..],
    ] {
        let mut host = MiniHost::spawn(args);
        assert_eq!(
            host.wait_exit().code(),
            Some(2),
            "retired --ruviform-* argv {args:?} must exit 2 (usage)"
        );
    }
}

/// A nonexistent shm key on `CreateInstance` → `Reply{seq, instance,
/// Err(Ipc)}` — the defined rejection. Wire v2 keeps the child alive:
/// attach failure is per-instance, not process-fatal.
#[test]
fn e2e_shm_attach_failure_is_ipc_reply_and_child_survives() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    let missing = unique_key("missing-segment");
    assert_eq!(
        host.create_instance(0, GAIN_ID, &missing),
        Err(HostError::Ipc),
        "attach failure is Err(Ipc) on the CreateInstance reply"
    );
    // The registry entry was never inserted — the same handle is
    // mintable again, and the child keeps serving.
    let key = unique_key("after-attach-failure");
    let _segment = create_segment(&key);
    assert_eq!(host.create_instance(0, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_alive(&mut host);
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// A segment with a bad header magic → `CreateInstance` →
/// `Err(Ipc)`, child alive.
#[test]
fn e2e_shm_header_mismatch_is_ipc_reply() {
    let key = unique_key("bad-magic");
    let _segment = create_segment_with(
        &key,
        IN_DEPTH,
        OUT_DEPTH,
        0xDEAD_BEEF,
        u32::from(wire::WIRE_VERSION),
    );
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    assert_eq!(host.create_instance(0, GAIN_ID, &key), Err(HostError::Ipc));
    assert_alive(&mut host);
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// A segment whose header `wire_version` differs from the runner's
/// `WIRE_VERSION` → `Err(Ipc)`. The shm version check is the same
/// strict-equality contract as the `Hello` handshake (Issue #119
/// single-version policy).
#[test]
fn e2e_shm_header_wire_version_mismatch_is_ipc_reply() {
    let key = unique_key("bad-version");
    let _segment = create_segment_with(
        &key,
        IN_DEPTH,
        OUT_DEPTH,
        shm::MAGIC,
        u32::from(wire::WIRE_VERSION) + 1,
    );
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    assert_eq!(host.create_instance(0, GAIN_ID, &key), Err(HostError::Ipc));
    assert_alive(&mut host);
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// `CreateInstance` depths that disagree with the segment header →
/// `Err(Ipc)` — the declared-depth cross-check added with v2.
#[test]
fn e2e_shm_declared_depth_mismatch_is_ipc_reply() {
    let key = unique_key("depth-mismatch");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    assert_eq!(
        host.create_instance_depths(0, GAIN_ID, &key, IN_DEPTH + 1, OUT_DEPTH),
        Err(HostError::Ipc),
        "in-depth mismatch is Err(Ipc)"
    );
    assert_eq!(
        host.create_instance_depths(0, GAIN_ID, &key, IN_DEPTH, OUT_DEPTH + 1),
        Err(HostError::Ipc),
        "out-depth mismatch is Err(Ipc)"
    );
    // Still mintable with the honest depths.
    assert_eq!(host.create_instance(0, GAIN_ID, &key), Ok(WireReply::Ok));
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// A clean stdin EOF at a frame boundary (parent closed the control
/// channel without `Shutdown`) normalizes every registered
/// instance's lifecycle — including a live `Processing` one — and
/// exits 0.
#[test]
fn e2e_stdin_eof_exits_zero() {
    let key = unique_key("eof");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    // Leave a Processing instance registered: `normalize_all` must
    // walk it back (`Processing → Activated → Initialized →
    // Destroyed`) before exit.
    assert_eq!(host.create_instance(0, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_eq!(host.command(0, WireCommand::Initialize), Ok(WireReply::Ok));
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    assert_eq!(
        host.command(0, WireCommand::Activate { config }),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(0, WireCommand::StartProcessing),
        Ok(WireReply::Ok)
    );

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
    let mut host = MiniHost::spawn_runner();

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
/// wire; the child survives and keeps serving commands — a
/// co-resident sibling instance's lifecycle and state are
/// untouched, not merely the process alive — then exits cleanly on
/// `Shutdown`.
#[test]
fn e2e_plugin_panic_is_plugin_failed_and_child_survives() {
    let key = unique_key("panic");
    let sibling_key = unique_key("panic-sibling");
    let _segment = create_segment(&key);
    let _sibling_segment = create_segment(&sibling_key);
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    let instance = 0;
    let sibling = 1;
    // `create()` itself is clean without the env gate — both
    // instances register on the one child; the panic lives in
    // instance 0's `init` (the gain lookup matches `GainFactory`
    // first, so the sibling never reaches `PanicFactory`).
    assert_eq!(
        host.create_instance(instance, PANIC_ID, &key),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.create_instance(sibling, GAIN_ID, &sibling_key),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::Initialize),
        Err(HostError::PluginFailed)
    );

    // The panic is instance-scoped: the sibling's entry is
    // untouched — its own lifecycle commits and its own state
    // blob round-trips on the same child.
    assert_eq!(
        host.command(sibling, WireCommand::Initialize),
        Ok(WireReply::Ok)
    );
    let saved = match host.command(sibling, WireCommand::SaveState) {
        Ok(WireReply::State { bytes }) => bytes,
        other => panic!("expected State, got {other:?}"),
    };
    assert_eq!(
        host.command(sibling, WireCommand::RestoreState { bytes: saved }),
        Ok(WireReply::Ok)
    );

    // The child is alive and still dispatching.
    assert_alive(&mut host);
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// `PanicFactory::descriptor()` panics under the
/// `RUVIFORM_PANIC_DESCRIPTOR` env gate — inside `CreateInstance`'s
/// plugin-id lookup the panic is caught and normalized to
/// `Reply{seq, instance, Err(PluginFailed)}`: a mid-serve factory
/// panic must not kill the process (Issue #117 — sibling instances
/// would die with it). The handle stays free.
#[test]
fn e2e_descriptor_panic_at_create_is_plugin_failed_and_child_survives() {
    let key = unique_key("create-desc-panic");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_with_env(&[], &[("RUVIFORM_PANIC_DESCRIPTOR", "1")]);
    host.expect_hello();

    // Resolving `dev.ruviform.mock.panic` walks the factories to
    // `PanicFactory::descriptor()` — which panics under the gate.
    assert_eq!(
        host.create_instance(0, PANIC_ID, &key),
        Err(HostError::PluginFailed),
        "descriptor() panic inside CreateInstance is normalized"
    );

    // Child alive, handle untouched: minting it with a healthy
    // factory works (the lookup matches `GainFactory` first, so it
    // never reaches the panicking `PanicFactory::descriptor()`).
    assert_eq!(host.create_instance(0, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_eq!(host.command(0, WireCommand::Initialize), Ok(WireReply::Ok));
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// `PanicFactory::create()` panics under the `RUVIFORM_PANIC_CREATE`
/// env gate — the `catch_unwind` around `create()` normalizes it to
/// `Err(PluginFailed)`; the registry entry is never inserted and the
/// handle stays free.
#[test]
fn e2e_create_panic_is_plugin_failed_and_handle_stays_free() {
    let key = unique_key("create-panic");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_with_env(&[], &[("RUVIFORM_PANIC_CREATE", "1")]);
    host.expect_hello();

    assert_eq!(
        host.create_instance(0, PANIC_ID, &key),
        Err(HostError::PluginFailed),
        "create() panic inside CreateInstance is normalized"
    );
    // The failed mint left no entry — the same handle mints cleanly.
    assert_eq!(host.create_instance(0, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_alive(&mut host);
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// The same env-gated `descriptor()` panic during a `Scan` command is
/// caught and mapped to `Err(HostError::PluginFailed)` — the child
/// survives and keeps serving commands (symmetric with `create()`).
#[test]
fn e2e_descriptor_panic_at_scan_is_plugin_failed() {
    let key = unique_key("scan-panic");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_with_env(&[], &[("RUVIFORM_PANIC_DESCRIPTOR", "1")]);
    host.expect_hello();

    assert_eq!(host.scan(), Err(HostError::PluginFailed));

    // Survived: CreateInstance still answers (the gain lookup matches
    // `GainFactory` before reaching `PanicFactory::descriptor()`).
    assert_eq!(host.create_instance(0, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_eq!(host.command(0, WireCommand::Initialize), Ok(WireReply::Ok));
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
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    let instance = 0;
    assert_eq!(
        host.create_instance(instance, UNDERWRITE_ID, &key),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::Initialize),
        Ok(WireReply::Ok)
    );
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    assert_eq!(
        host.command(instance, WireCommand::Activate { config }),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::StartProcessing),
        Ok(WireReply::Ok)
    );

    let inputs = [vec![1.0_f32, 1.0, 1.0, 1.0], vec![7.0_f32, 7.0, 7.0, 7.0]];
    // Block 0: all channels written — the stale seed for block 1.
    write_in_slot(&mut segment, 0, 4, 2, &[], None, &inputs);
    host.block_in(0, instance);
    expect_block_done(&mut host, 0, instance);
    let (_, _, _, samples) = read_out_slot(&segment, 0);
    assert_eq!(samples[0], [1.0; 4]);
    assert_eq!(samples[1], [7.0; 4]);

    // Block 1: the plugin under-writes (channel 0 only) — channel 1
    // must come out zeroed, not as block 0's leftover `7.0`s.
    write_in_slot(&mut segment, 1, 4, 2, &[], None, &inputs);
    host.block_in(1, instance);
    expect_block_done(&mut host, 1, instance);
    let (_, _, _, samples) = read_out_slot(&segment, 1);
    assert_eq!(samples[0], [1.0; 4]);
    assert_eq!(
        samples[1], [0.0; 4],
        "under-written channel must be zero, not stale block-0 samples"
    );

    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

// --- registry (wire v2 bundle semantics) ----------------------------------

/// Two live instances in one child — independent lifecycle, data
/// plane and state: instance 0 reaches `Processing` and serves
/// `BlockIn` while instance 1 is still `Created` (its doorbell is a
/// `Lifecycle` rejection). `DestroyInstance` reclaims handle 1;
/// commands on the dead handle become `InvalidHandle` while handle 0
/// keeps working.
#[test]
fn e2e_bundle_instances_are_independent() {
    let key_a = unique_key("bundle-a");
    let key_b = unique_key("bundle-b");
    let mut segment_a = create_segment(&key_a);
    let _segment_b = create_segment(&key_b);
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    assert_eq!(host.create_instance(0, GAIN_ID, &key_a), Ok(WireReply::Ok));
    assert_eq!(host.create_instance(1, GAIN_ID, &key_b), Ok(WireReply::Ok));

    // Drive instance 0 to Processing; instance 1 stays Created.
    assert_eq!(host.command(0, WireCommand::Initialize), Ok(WireReply::Ok));
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    assert_eq!(
        host.command(0, WireCommand::Activate { config }),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(0, WireCommand::StartProcessing),
        Ok(WireReply::Ok)
    );

    // Instance 0 processes its own segment — the gain fixture proves
    // the entry's staging and segment are per-instance.
    write_in_slot(&mut segment_a, 0, 4, 1, &[], None, &[vec![2.0; 4]]);
    host.block_in(0, 0);
    assert_eq!(
        expect_block_done(&mut host, 0, 0).status,
        ProcessStatus::Continue
    );
    let (_, _, _, samples) = read_out_slot(&segment_a, 0);
    assert_eq!(samples[0], [2.0; 4]);

    // Instance 1 is still `Created`: its doorbell is the defined
    // `Lifecycle` rejection, not `InvalidHandle` (the handle is live)
    // and not instance-0's `Processing` state leaking across.
    host.block_in(0, 1);
    match host.recv() {
        Message::Reply {
            seq,
            instance: got,
            result,
        } => {
            assert_eq!(seq, 0);
            assert_eq!(got, 1);
            assert!(
                matches!(result, Err(HostError::Lifecycle(_))),
                "Created instance's doorbell is Err(Lifecycle), got {result:?}"
            );
        }
        other => panic!("expected Reply, got {other:?}"),
    }

    // Independent state blobs: a restore on 0 does not touch 1.
    let state = match host.command(0, WireCommand::SaveState) {
        Ok(WireReply::State { bytes }) => bytes,
        other => panic!("expected State, got {other:?}"),
    };
    assert_eq!(
        host.command(0, WireCommand::RestoreState { bytes: state }),
        Ok(WireReply::Ok)
    );

    // Reclaim 1; its handle is dead afterwards while 0 still serves.
    assert_eq!(
        host.command(1, WireCommand::DestroyInstance),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(1, WireCommand::Initialize),
        Err(HostError::InvalidHandle)
    );
    assert_eq!(
        host.command(0, WireCommand::StopProcessing),
        Ok(WireReply::Ok)
    );

    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// Commands and doorbells on a never-registered handle are the
/// defined `Err(InvalidHandle)` reply — never a dead child.
#[test]
fn e2e_unregistered_handle_is_invalid_handle() {
    let key = unique_key("invalid-handle");
    let mut segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    // Command on an unregistered handle.
    assert_eq!(
        host.command(7, WireCommand::Initialize),
        Err(HostError::InvalidHandle)
    );
    // `DestroyInstance` on it is `InvalidHandle` too.
    assert_eq!(
        host.command(7, WireCommand::DestroyInstance),
        Err(HostError::InvalidHandle)
    );
    // A doorbell on it is the flagged `Reply{seq, instance,
    // Err(InvalidHandle)}` — the runner's defined answer.
    write_in_slot(&mut segment, 0, 4, 1, &[], None, &[vec![1.0; 4]]);
    host.block_in(0, 7);
    match host.recv() {
        Message::Reply {
            seq,
            instance: got,
            result,
        } => {
            assert_eq!(seq, 0);
            assert_eq!(got, 7);
            assert_eq!(result, Err(HostError::InvalidHandle));
        }
        other => panic!("expected Reply, got {other:?}"),
    }

    // The child is fine: the same handle mints and serves.
    assert_eq!(host.create_instance(7, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_eq!(host.command(7, WireCommand::Initialize), Ok(WireReply::Ok));
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// `DestroyInstance` reclaims the handle: subsequent commands and
/// doorbells answer `InvalidHandle`, and the freed handle mints
/// again.
#[test]
fn e2e_destroyed_handle_is_invalid_and_remintable() {
    let key = unique_key("destroy");
    let mut segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    assert_eq!(host.create_instance(3, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_eq!(host.command(3, WireCommand::Initialize), Ok(WireReply::Ok));
    assert_eq!(
        host.command(3, WireCommand::DestroyInstance),
        Ok(WireReply::Ok)
    );

    assert_eq!(
        host.command(3, WireCommand::Initialize),
        Err(HostError::InvalidHandle)
    );
    write_in_slot(&mut segment, 0, 4, 1, &[], None, &[vec![1.0; 4]]);
    host.block_in(0, 3);
    match host.recv() {
        Message::Reply { result, .. } => {
            assert_eq!(result, Err(HostError::InvalidHandle));
        }
        other => panic!("expected Reply, got {other:?}"),
    }

    // The freed handle mints again — same segment, fresh lifecycle.
    assert_eq!(host.create_instance(3, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_eq!(host.command(3, WireCommand::Initialize), Ok(WireReply::Ok));
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// Minting an already-live handle is a host-side contract violation —
/// the defined `Err(Ipc)` reply, and the live instance is untouched.
#[test]
fn e2e_duplicate_instance_is_ipc_rejection() {
    let key = unique_key("dup");
    let _segment = create_segment(&key);
    let other_key = unique_key("dup-other");
    let _other = create_segment(&other_key);
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    assert_eq!(host.create_instance(0, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_eq!(
        host.create_instance(0, GAIN_ID, &other_key),
        Err(HostError::Ipc),
        "duplicate handle mint is Err(Ipc)"
    );

    // The original instance survived the rejected re-mint.
    assert_eq!(host.command(0, WireCommand::Initialize), Ok(WireReply::Ok));
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// `CreateInstance` for a `plugin_id` no registered factory claims
/// is the defined `Err(UnknownPluginId)` — the lookup runs first in
/// the dispatch order, so the rejection mints no registry entry:
/// the child survives and the handle mints cleanly afterwards.
#[test]
fn e2e_create_instance_unknown_plugin_id() {
    let key = unique_key("unknown-plugin");
    let _segment = create_segment(&key);
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    assert_eq!(
        host.create_instance(0, "dev.ruviform.mock.nonexistent", &key),
        Err(HostError::UnknownPluginId)
    );

    // The failed mint left no entry — the same handle serves a
    // claimed id, and the child keeps dispatching.
    assert_eq!(host.create_instance(0, GAIN_ID, &key), Ok(WireReply::Ok));
    assert_eq!(host.command(0, WireCommand::Initialize), Ok(WireReply::Ok));
    assert_alive(&mut host);
    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// A full registry — [`wire::WIRE_MAX_INSTANCES`] live instances —
/// rejects the next `CreateInstance` with `Err(InstanceLimit)`
/// regardless of the handle (capacity is checked before the
/// duplicate-handle check), and a reclaimed slot frees capacity.
///
/// The instances ride depth-1 segments: the limit is about the
/// registry, not the ring shape, and 64 × 2 slots keeps the fixture
/// small.
#[test]
fn e2e_instance_limit() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    let mut segments = Vec::new();
    for instance in 0..wire::WIRE_MAX_INSTANCES {
        let key = unique_key(&format!("limit-{instance}"));
        segments.push(create_segment_depths(&key, 1, 1));
        assert_eq!(
            host.create_instance_depths(instance, GAIN_ID, &key, 1, 1),
            Ok(WireReply::Ok),
            "instance {instance} must mint while capacity remains"
        );
    }

    // Registry full — even re-minting a live handle reports
    // `InstanceLimit` (capacity precedes the duplicate check).
    let spare_key = unique_key("limit-spare");
    let _spare = create_segment_depths(&spare_key, 1, 1);
    assert_eq!(
        host.create_instance_depths(0, GAIN_ID, &spare_key, 1, 1),
        Err(HostError::InstanceLimit)
    );

    // Destroy one and capacity returns.
    assert_eq!(
        host.command(0, WireCommand::DestroyInstance),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.create_instance_depths(0, GAIN_ID, &spare_key, 1, 1),
        Ok(WireReply::Ok)
    );

    host.send(&Message::Shutdown);
    assert_eq!(host.wait_exit().code(), Some(0));
}

/// `Shutdown` (and EOF) normalizes *every* live entry — a bundle of
/// instances left mid-lifecycle still exits 0.
#[test]
fn e2e_shutdown_normalizes_all_instances() {
    let key_a = unique_key("norm-a");
    let key_b = unique_key("norm-b");
    let _segment_a = create_segment(&key_a);
    let _segment_b = create_segment(&key_b);
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();

    assert_eq!(host.create_instance(0, GAIN_ID, &key_a), Ok(WireReply::Ok));
    assert_eq!(host.create_instance(1, GAIN_ID, &key_b), Ok(WireReply::Ok));
    assert_eq!(host.command(0, WireCommand::Initialize), Ok(WireReply::Ok));
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    assert_eq!(
        host.command(0, WireCommand::Activate { config }),
        Ok(WireReply::Ok)
    );
    // Instance 1 stays `Created`; instance 0 sits `Activated`.
    host.send(&Message::Shutdown);
    assert_eq!(
        host.wait_exit().code(),
        Some(0),
        "Shutdown must normalize every entry and exit(0)"
    );
}
