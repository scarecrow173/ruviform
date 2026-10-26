//! `l1-test-worker` — the deterministic wire-conformant L1 child
//! fixture for `tests/l1_proxy_e2e.rs` (spawned via
//! `CARGO_BIN_EXE_l1-test-worker`; it stays inside this crate — no
//! cross-package bin resolution).
//!
//! It speaks the `ruviform-plugin-core` wire protocol exactly as a
//! real runner child does: `Hello` on startup, then a single-threaded
//! frame loop — `Command` → `Reply` (a `Lifecycle` state machine
//! decides transition legality, matching the child-side semantics of
//! the design), `Shutdown` → `exit(0)`, stdin EOF → `exit(0)`.
//!
//! ## argv
//!
//! | flag | effect |
//! | --- | --- |
//! | `--ruviform-plugin-id <id>` | parsed and ignored — the worker serves its single built-in descriptor regardless |
//! | `--ruviform-shm-key <key>` | the proxy's shm segment `os_id` — attached after `Hello`; `BlockIn` reads the in-slot, applies [`WORKER_GAIN`], writes the out-slot and answers `BlockDone` (a hung worker answers nothing) |
//! | `--crash-after-msgs=N` | `process::exit` upon *receiving* the Nth parent→child message — before dispatch, so the Nth command's waiter stays pending and is woken by EOF/`try_wait` |
//! | `--crash-exit-code=C` | exit code used by `--crash-after-msgs` (default 3; `0` simulates a voluntary `exit(0)`, which is still a `Failed` violation) |
//! | `--hang-after-msgs=N` | after the Nth received message, keep reading but stop answering commands (`0` hangs even the handshake `Scan`); `Shutdown` is still honored so teardown stays deterministic |
//! | `--reject-blocks-after-msgs=N` | after the Nth parsed `BlockIn` doorbell (a doorbell-only counter — `Command`/`Shutdown` messages do not count), answer every later `BlockIn` with the flagged `Reply{seq, Err(HostError::PluginFailed)}` — the real rejection shape — instead of processing |
//! | `--stop-reading-after-msgs=N` | after the Nth received message is fully answered, never read stdin again — a wedged child that stays alive but never drains the pipe, so a large parent frame blocks the proxy's `write_all`; only a kill ends it |
//! | `--leave-heir-after-msgs=N` | after the Nth received message is fully answered, spawn a copy of this binary with `--sleep-ms=15000` (inheriting stdio — the grandchild holds the parent's stdout pipe open) then `exit(0)`; EOF never reaches the proxy's reader thread |
//! | `--sleep-ms=M` | sleep `M` ms then exit 0 — the heir mode used by `--leave-heir-after-msgs`; not meant to be spawned directly by tests |
//! | `--hello-version=V` | `Hello.version` override (default [`wire::WIRE_VERSION`]) for the version-mismatch handshake test |
//! | `--fail-stop-processing` | `StopProcessing` commits `Processing → Activated` then replies `Err(HostError::PluginFailed)` — the real runner's commit-then-advisory shape (a hook error arrives post-commit), exercising the host-side mirror fix |
//!
//! Message counting covers parent→child frames (`Command`, `BlockIn`,
//! `Shutdown`) — the handshake `Scan` the proxy sends itself counts
//! as message 1 — except `--reject-blocks-after-msgs`, which counts
//! `BlockIn` doorbells only.

use std::io::{self, Read, Write};
use std::process::ExitCode;
use std::sync::atomic::{Ordering, fence};

use ruviform_plugin_core::wire::{self, WIRE_FRAME_HEADER_LEN, WIRE_MAX_PAYLOAD, shm};
use ruviform_plugin_core::{
    HostError, Lifecycle, LifecycleEvent, PluginDescriptor, PrepareError, ProcessResult,
    ProcessStatus,
};
use shared_memory::{Shmem, ShmemConf};

/// The single built-in descriptor this worker reports for `Scan` and
/// validates nothing else against — the E2E's `SpawnSpec.descriptor`
/// must match it field-for-field.
const WORKER_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l1-test-worker",
    name: "L1 Test Worker",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// The fixed gain the data-plane path applies — `out = in * 0.5`,
/// so the E2E can tell real processed audio from silence and
/// pass-through copies.
const WORKER_GAIN: f32 = 0.5;

/// The `BlockDone` result marker for a degenerate block
/// (`frames == 0 || channels == 0`): a legitimately staged block
/// always carries real dims, so a zero-dim in-slot can only come
/// from a doorbell the proxy rang without staging — the
/// re-activation stale-submit case. Answering with an
/// unmistakable marker (instead of `CONTINUE`) lets the E2E
/// distinguish "the child actually processed this seq" from "the
/// seq was never doorbell'd".
const ZERO_DIM_RESULT: ProcessResult = ProcessResult::new(ProcessStatus::Sleep, 0xDEAD);

/// Parsed CLI flags.
struct Flags {
    /// Die upon receiving message N (before dispatch).
    crash_after_msgs: Option<u64>,
    /// Exit code used with `crash_after_msgs` (default 3).
    crash_exit_code: i32,
    /// Stop answering commands after message N.
    hang_after_msgs: Option<u64>,
    /// Answer `BlockIn` doorbells with a flagged `Err` `Reply`
    /// after the Nth doorbell (doorbell-only counter).
    reject_blocks_after_msgs: Option<u64>,
    /// Never read stdin again after message N.
    stop_reading_after_msgs: Option<u64>,
    /// Spawn a stdio-inheriting heir then exit after message N.
    leave_heir_after_msgs: Option<u64>,
    /// Heir mode: sleep this many ms then exit (`None` = normal serve).
    sleep_ms: Option<u64>,
    /// `Hello.version` override.
    hello_version: u16,
    /// `StopProcessing` commits the transition then replies
    /// `Err(PluginFailed)` — the real runner's advisory-error shape.
    fail_stop_processing: bool,
    /// `--ruviform-shm-key` value — attached after `Hello`.
    shm_key: Option<String>,
}

fn parse_flags() -> Flags {
    let mut flags = Flags {
        crash_after_msgs: None,
        crash_exit_code: 3,
        hang_after_msgs: None,
        reject_blocks_after_msgs: None,
        stop_reading_after_msgs: None,
        leave_heir_after_msgs: None,
        sleep_ms: None,
        hello_version: wire::WIRE_VERSION,
        fail_stop_processing: false,
        shm_key: None,
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        if let Some(value) = arg.strip_prefix("--crash-after-msgs=") {
            flags.crash_after_msgs = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--crash-exit-code=") {
            flags.crash_exit_code = value.parse().unwrap_or(3);
        } else if let Some(value) = arg.strip_prefix("--hang-after-msgs=") {
            flags.hang_after_msgs = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--reject-blocks-after-msgs=") {
            flags.reject_blocks_after_msgs = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--stop-reading-after-msgs=") {
            flags.stop_reading_after_msgs = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--leave-heir-after-msgs=") {
            flags.leave_heir_after_msgs = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--sleep-ms=") {
            flags.sleep_ms = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--hello-version=") {
            flags.hello_version = value.parse().unwrap_or(wire::WIRE_VERSION);
        } else if arg == "--fail-stop-processing" {
            flags.fail_stop_processing = true;
        } else if arg == "--ruviform-plugin-id" {
            // Value-carrying bootstrap flag — consumed and ignored.
            let _ = args.next();
        } else if arg == "--ruviform-shm-key" {
            flags.shm_key = args.next().map(|v| v.to_string_lossy().into_owned());
        }
    }
    flags
}

fn read_message(reader: &mut impl Read) -> io::Result<wire::Message> {
    let mut header = [0_u8; WIRE_FRAME_HEADER_LEN];
    reader.read_exact(&mut header)?;
    let payload_len =
        u32::from_le_bytes(header[..4].try_into().expect("u32 length is 4 bytes")) as usize;
    if payload_len > WIRE_MAX_PAYLOAD as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "declared payload_len exceeds WIRE_MAX_PAYLOAD",
        ));
    }
    let mut frame = Vec::with_capacity(WIRE_FRAME_HEADER_LEN + payload_len);
    frame.extend_from_slice(&header);
    frame.resize(WIRE_FRAME_HEADER_LEN + payload_len, 0);
    reader.read_exact(&mut frame[WIRE_FRAME_HEADER_LEN..])?;
    wire::decode(&frame).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn write_message(writer: &mut impl Write, message: &wire::Message) -> io::Result<()> {
    let mut frame = Vec::new();
    wire::encode(message, &mut frame).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writer.write_all(&frame)?;
    writer.flush()
}

fn transition(lifecycle: &mut Lifecycle, event: LifecycleEvent) -> Result<(), HostError> {
    lifecycle
        .transition(event)
        .map(|_| ())
        .map_err(HostError::Lifecycle)
}

/// The child-side `Lifecycle` semantics of the design: transition
/// legality is decided here, plugin-hook failures are reported as
/// `HostError`s — mirroring what `InProcessHost` does in-process.
fn dispatch(
    lifecycle: &mut Lifecycle,
    state: &mut Vec<u8>,
    cmd: &wire::Command,
    fail_stop_processing: bool,
) -> Result<wire::Reply, HostError> {
    match cmd {
        wire::Command::Scan => Ok(wire::Reply::Scanned(vec![wire::Descriptor::from(
            WORKER_DESCRIPTOR,
        )])),
        wire::Command::Initialize => {
            transition(lifecycle, LifecycleEvent::Initialize).map(|()| wire::Reply::Ok)
        }
        wire::Command::Activate { config } => {
            // Lifecycle legality before the config bound — the real
            // runner (`Runner::activate`) and L0 validate the
            // transition first, so the proxy tests observe production
            // ordering (an `Activate` from `Created` is `Lifecycle`,
            // not `Prepare`).
            lifecycle
                .state()
                .transition(LifecycleEvent::Activate)
                .map_err(HostError::Lifecycle)?;
            if config.max_frames() > wire::WIRE_MAX_FRAMES {
                // Defined rejection, same shape the proxy applies:
                // reported failure destination stays `Initialized`.
                let _ = lifecycle.transition(LifecycleEvent::ActivateFailed);
                return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
            }
            transition(lifecycle, LifecycleEvent::Activate).map(|()| wire::Reply::Ok)
        }
        wire::Command::StartProcessing => {
            transition(lifecycle, LifecycleEvent::StartProcessing).map(|()| wire::Reply::Ok)
        }
        wire::Command::StopProcessing => {
            // Commit first, like the real runner
            // (`Runner::stop_processing` applies the transition
            // before the hook runs): `--fail-stop-processing` then
            // reports the advisory `Err` *after* the commit.
            transition(lifecycle, LifecycleEvent::StopProcessing)?;
            if fail_stop_processing {
                Err(HostError::PluginFailed)
            } else {
                Ok(wire::Reply::Ok)
            }
        }
        wire::Command::Deactivate => {
            transition(lifecycle, LifecycleEvent::Deactivate).map(|()| wire::Reply::Ok)
        }
        wire::Command::SaveState => Ok(wire::Reply::State {
            bytes: state.clone(),
        }),
        wire::Command::RestoreState { bytes } => {
            state.clone_from(bytes);
            Ok(wire::Reply::Ok)
        }
        // GUI is `Unsupported` — the proxy never forwards these, so
        // reaching them here means the proxy violated the contract;
        // the reply still matches the documented rejection.
        _ => Err(HostError::Unsupported),
    }
}

/// The worker's data-plane segment: a non-owning attach to the
/// proxy's `--ruviform-shm-key` object, header-validated exactly like
/// the real runner's `ruviform_plugin_export::shm::Segment`. Slot
/// selection is the doorbell `seq % depth` under strict alternation —
/// no control state lives in the segment.
struct Segment {
    mem: Shmem,
    in_depth: u64,
    out_depth: u64,
    in_off: usize,
    out_off: usize,
}

impl Segment {
    /// Attaches to `key` and validates the fixed header — mirrors the
    /// runner's `attach` byte for byte. `None` on any failure: the
    /// worker exits non-zero, exactly like the real runner.
    fn attach(key: &str) -> Option<Self> {
        let mem = ShmemConf::new().os_id(key).open().ok()?;
        let bytes = bytes(&mem);
        if bytes.len() < shm::HEADER_LEN {
            return None;
        }
        if get_u32(bytes, shm::OFF_MAGIC) != shm::MAGIC
            || get_u32(bytes, shm::OFF_WIRE_VERSION) != u32::from(wire::WIRE_VERSION)
        {
            return None;
        }
        let in_depth = get_u32(bytes, shm::OFF_IN_DEPTH);
        let out_depth = get_u32(bytes, shm::OFF_OUT_DEPTH);
        if in_depth == 0 || out_depth == 0 {
            return None;
        }
        if get_u32(bytes, shm::OFF_SLOT_BYTES) as usize != shm::SLOT_BYTES {
            return None;
        }
        let in_off = get_u32(bytes, shm::OFF_IN_OFF) as usize;
        if in_off != shm::IN_SLOTS_OFF {
            return None;
        }
        let out_off = get_u32(bytes, shm::OFF_OUT_OFF) as usize;
        if out_off != shm::out_slots_off(in_depth) {
            return None;
        }
        if bytes.len() < out_off + out_depth as usize * shm::SLOT_BYTES {
            return None;
        }
        Some(Self {
            mem,
            in_depth: u64::from(in_depth),
            out_depth: u64::from(out_depth),
            in_off,
            out_off,
        })
    }

    /// Reads the in-slot for doorbell `seq`, applies [`WORKER_GAIN`]
    /// and writes the out-slot — the child's whole data-plane step.
    /// `Err(())` is the runner's slot-parse failure (the caller
    /// answers `Reply{seq, Err(Ipc)}` instead of `BlockDone`).
    /// `activated_max_frames` mirrors the real runner's
    /// `Runner::activated_max_frames` enforcement: a block over the
    /// *activated* bound — not just over the wire max — is rejected.
    /// A degenerate (`frames == 0 || channels == 0`) block answers
    /// [`ZERO_DIM_RESULT`] instead of `CONTINUE` — see its doc.
    ///
    /// Every offset comes from [`wire::shm`] consts — the worker
    /// reproduces the layout contract, never a copy of it.
    fn process_block(
        &mut self,
        seq: u64,
        activated_max_frames: Option<u32>,
    ) -> Result<ProcessResult, ()> {
        let in_index = usize::try_from(seq % self.in_depth).map_err(|_| ())?;
        let in_start = self.in_off + in_index * shm::SLOT_BYTES;
        let out_index =
            usize::try_from(seq % self.out_depth).expect("out-slot index always fits usize");
        let out_start = self.out_off + out_index * shm::SLOT_BYTES;
        // The in-slot array precedes the out-slot array, so one
        // `split_at_mut` hands back a shared in-slot and an exclusive
        // out-slot without aliasing.
        let mem = bytes_mut(&mut self.mem);
        let (in_region, out_region) = mem.split_at_mut(out_start);
        let in_slot = &in_region[in_start..in_start + shm::SLOT_BYTES];
        let out_slot = &mut out_region[..shm::SLOT_BYTES];
        if get_u64(in_slot, shm::SLOT_OFF_SEQ) != seq {
            return Err(());
        }
        let frames = get_u32(in_slot, shm::SLOT_OFF_FRAMES);
        let channels = get_u32(in_slot, shm::SLOT_OFF_CHANNELS);
        let event_count = get_u32(in_slot, shm::SLOT_OFF_EVENT_COUNT);
        let flags = get_u32(in_slot, shm::SLOT_OFF_FLAGS);
        if frames > wire::WIRE_MAX_FRAMES
            || channels > wire::WIRE_MAX_CHANNELS
            || event_count > wire::WIRE_MAX_EVENTS
            || flags & !shm::FLAG_TRANSPORT != 0
        {
            return Err(());
        }
        // The activated bound — same enforcement order as the real
        // runner (wire bounds first, then the negotiated config).
        if activated_max_frames.is_some_and(|max| frames > max) {
            return Err(());
        }
        let result = if frames == 0 || channels == 0 {
            ZERO_DIM_RESULT
        } else {
            ProcessResult::CONTINUE
        };
        put_u64(out_slot, shm::SLOT_OFF_SEQ, seq);
        put_u32(out_slot, shm::SLOT_OFF_FRAMES, frames);
        put_u32(out_slot, shm::SLOT_OFF_CHANNELS, channels);
        put_u32(out_slot, shm::SLOT_OFF_EVENT_COUNT, 0);
        put_u32(out_slot, shm::SLOT_OFF_FLAGS, 0);
        out_slot[shm::TRANSPORT_OFF..shm::TRANSPORT_OFF + shm::TRANSPORT_LEN].fill(0);
        let mut encoded = Vec::new();
        wire::encode_process_result(&result, &mut encoded);
        out_slot[shm::RESULT_OFF..shm::RESULT_OFF + encoded.len()].copy_from_slice(&encoded);
        out_slot[shm::RESULT_OFF + encoded.len()..shm::RESULT_OFF + shm::RESULT_LEN].fill(0);
        for channel in 0..channels as usize {
            let base = shm::SAMPLES_OFF + channel * shm::SAMPLE_STRIDE;
            for frame in 0..frames as usize {
                let sample = f32::from_le_bytes(
                    in_slot[base + frame * 4..base + frame * 4 + 4]
                        .try_into()
                        .expect("f32 sample is 4 bytes"),
                ) * WORKER_GAIN;
                out_slot[base + frame * 4..base + frame * 4 + 4]
                    .copy_from_slice(&sample.to_le_bytes());
            }
        }
        Ok(result)
    }
}

/// The segment as a read-only byte slice.
fn bytes(mem: &Shmem) -> &[u8] {
    // SAFETY: `Shmem::as_slice` is `unsafe` because the crate cannot
    // prove the mapped range immutable — another process maps the same
    // pages. The L1 slot protocol supplies that guarantee: header
    // bytes are owner-written before spawn and only read here, and
    // each slot is touched by exactly one side at a time under the
    // strict-alternation doorbell rule. Same documented deviation as
    // `ruviform-plugin-export::shm` and `ruviform-plugin-host::l1::shm`.
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice()
    }
}

/// The segment as a mutable byte slice.
fn bytes_mut(mem: &mut Shmem) -> &mut [u8] {
    // SAFETY: same protocol argument as `bytes` — the out-slot written
    // through `process_block` is owned by this child between doorbells
    // under the strict-alternation rule, and the `&mut Shmem` borrow
    // keeps the slice exclusive within the call.
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice_mut()
    }
}

fn get_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("u32 field is 4 bytes"),
    )
}

fn get_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("u64 field is 8 bytes"),
    )
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

/// One `BlockIn` doorbell: a lifecycle rejection answers the runner's
/// defined flagged `Reply{seq, Err}`; otherwise the shm protocol runs
/// Acquire → slot read → gain → slot write → Release → `BlockDone`
/// (a slot-parse failure — including dims over the *activated*
/// `max_frames`, not just the wire max — answers
/// `Reply{seq, Err(Ipc)}` instead). The Acquire/Release edges are
/// real atomic fences (`std::sync::atomic::fence`) — ordering across
/// the process boundary rests on them plus the pipe syscalls.
/// An `Err` means the reply write itself failed — the caller exits.
fn block_in(
    lifecycle: &mut Lifecycle,
    segment: Option<&mut Segment>,
    seq: u64,
    activated_max_frames: Option<u32>,
    stdout: &mut impl Write,
) -> io::Result<()> {
    if let Err(error) = transition(lifecycle, LifecycleEvent::Process) {
        return write_message(
            stdout,
            &wire::Message::Reply {
                seq,
                result: Err(error),
            },
        );
    }
    fence(Ordering::Acquire);
    let outcome = segment.map_or(Err(()), |segment| {
        segment.process_block(seq, activated_max_frames)
    });
    fence(Ordering::Release);
    let message = match outcome {
        Ok(result) => wire::Message::BlockDone { seq, result },
        Err(()) => wire::Message::Reply {
            seq,
            result: Err(HostError::Ipc),
        },
    };
    write_message(stdout, &message)
}

/// One `BlockIn` dispatch — `hung` answers nothing (the wedged data
/// plane); past `--reject-blocks-after-msgs` doorbells the worker
/// sends the real block-rejection shape — a flagged `Reply{seq,
/// Err(PluginFailed)}` instead of `BlockDone`; otherwise the doorbell
/// runs through [`block_in`]. `Err` means a reply write failed.
#[allow(clippy::too_many_arguments)]
fn block_in_dispatch(
    lifecycle: &mut Lifecycle,
    segment: Option<&mut Segment>,
    seq: u64,
    blocks_received: u64,
    hung: bool,
    reject_blocks_after_msgs: Option<u64>,
    activated_max_frames: Option<u32>,
    stdout: &mut impl Write,
) -> io::Result<()> {
    if hung {
        // A wedged data plane answers nothing — the proxy's RT leg
        // must pass through, never block.
        return Ok(());
    }
    if reject_blocks_after_msgs.is_some_and(|limit| blocks_received > limit) {
        return write_message(
            stdout,
            &wire::Message::Reply {
                seq,
                result: Err(HostError::PluginFailed),
            },
        );
    }
    block_in(lifecycle, segment, seq, activated_max_frames, stdout)
}

fn main() -> ExitCode {
    let flags = parse_flags();
    if let Some(ms) = flags.sleep_ms {
        // Heir mode — a spawned grandchild that just holds the
        // inherited pipes open, then exits cleanly.
        std::thread::sleep(std::time::Duration::from_millis(ms));
        return ExitCode::SUCCESS;
    }
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    if write_message(
        &mut stdout,
        &wire::Message::Hello {
            version: flags.hello_version,
            capabilities: 0,
        },
    )
    .is_err()
    {
        return ExitCode::FAILURE;
    }

    let mut lifecycle = Lifecycle::new();
    let mut state: Vec<u8> = vec![0x11, 0x22, 0x33, 0x44];
    let mut received: u64 = 0;
    // Doorbells parsed so far — the `--reject-blocks-after-msgs`
    // counter (deliberately separate from `received`, which counts
    // every parent→child message).
    let mut blocks_received: u64 = 0;
    // The activated `max_frames` — the real runner's
    // `activated_max_frames` enforcement, set on a successful
    // `Activate` and cleared on `Deactivate`.
    let mut activated_max_frames: Option<u32> = None;

    // Attach the proxy's segment the way the real runner does — after
    // `Hello`. A missing/invalid segment is a hard failure (non-zero
    // exit) just like `Runner::run`.
    let mut segment = match flags.shm_key.as_deref() {
        Some(key) => match Segment::attach(key) {
            Some(segment) => Some(segment),
            None => return ExitCode::FAILURE,
        },
        None => None,
    };

    loop {
        if flags
            .stop_reading_after_msgs
            .is_some_and(|limit| received >= limit)
        {
            // Wedged-child simulation: stay alive but never drain
            // stdin again — a large parent frame then blocks the
            // proxy driver's `write_all` once the pipe fills. Only
            // `child.kill()` ends this; `Shutdown` is never seen.
            loop {
                std::thread::park();
            }
        }
        if flags
            .leave_heir_after_msgs
            .is_some_and(|limit| received >= limit)
        {
            // Spawn a stdio-inheriting heir then exit: the heir keeps
            // the parent's stdout pipe open after this process is
            // reaped, so the proxy's reader thread never sees EOF.
            // (The running heir briefly locks this exe image on
            // Windows — its lifetime stays short so rebuilds are not
            // blocked for long.)
            let _ =
                std::process::Command::new(std::env::current_exe().expect("heir needs this exe"))
                    .arg("--sleep-ms=15000")
                    .spawn();
            std::process::exit(0);
        }
        let Ok(message) = read_message(&mut stdin) else {
            // stdin EOF or a broken/malformed frame — clean teardown,
            // same as `Shutdown`.
            return ExitCode::SUCCESS;
        };
        received += 1;
        if let Some(limit) = flags.crash_after_msgs
            && received >= limit
        {
            std::process::exit(flags.crash_exit_code);
        }
        let hung = flags.hang_after_msgs.is_some_and(|limit| received > limit);
        match message {
            wire::Message::Command { seq, cmd } => {
                if !hung {
                    let result =
                        dispatch(&mut lifecycle, &mut state, &cmd, flags.fail_stop_processing);
                    // Track the activated bound like the real runner
                    // (`Runner::activated_max_frames`): set on a
                    // successful `Activate`, cleared on `Deactivate`.
                    if result.is_ok() {
                        match &cmd {
                            wire::Command::Activate { config } => {
                                activated_max_frames = Some(config.max_frames());
                            }
                            wire::Command::Deactivate => activated_max_frames = None,
                            _ => {}
                        }
                    }
                    if write_message(&mut stdout, &wire::Message::Reply { seq, result }).is_err() {
                        return ExitCode::FAILURE;
                    }
                }
            }
            wire::Message::BlockIn { seq } => {
                blocks_received += 1;
                if block_in_dispatch(
                    &mut lifecycle,
                    segment.as_mut(),
                    seq,
                    blocks_received,
                    hung,
                    flags.reject_blocks_after_msgs,
                    activated_max_frames,
                    &mut stdout,
                )
                .is_err()
                {
                    return ExitCode::FAILURE;
                }
            }
            wire::Message::Shutdown => return ExitCode::SUCCESS,
            // Child→parent kinds arriving this direction are ignored.
            _ => {}
        }
    }
}
