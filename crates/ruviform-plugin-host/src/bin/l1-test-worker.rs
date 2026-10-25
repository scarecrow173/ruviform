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
//! | `--ruviform-shm-key <key>` | parsed and ignored — shm attach arrives with the data-plane task |
//! | `--crash-after-msgs=N` | `process::exit` upon *receiving* the Nth parent→child message — before dispatch, so the Nth command's waiter stays pending and is woken by EOF/`try_wait` |
//! | `--crash-exit-code=C` | exit code used by `--crash-after-msgs` (default 3; `0` simulates a voluntary `exit(0)`, which is still a `Failed` violation) |
//! | `--hang-after-msgs=N` | after the Nth received message, keep reading but stop answering commands (`0` hangs even the handshake `Scan`); `Shutdown` is still honored so teardown stays deterministic |
//! | `--stop-reading-after-msgs=N` | after the Nth received message is fully answered, never read stdin again — a wedged child that stays alive but never drains the pipe, so a large parent frame blocks the proxy's `write_all`; only a kill ends it |
//! | `--leave-heir-after-msgs=N` | after the Nth received message is fully answered, spawn a copy of this binary with `--sleep-ms=15000` (inheriting stdio — the grandchild holds the parent's stdout pipe open) then `exit(0)`; EOF never reaches the proxy's reader thread |
//! | `--sleep-ms=M` | sleep `M` ms then exit 0 — the heir mode used by `--leave-heir-after-msgs`; not meant to be spawned directly by tests |
//! | `--hello-version=V` | `Hello.version` override (default [`wire::WIRE_VERSION`]) for the version-mismatch handshake test |
//!
//! Message counting covers parent→child frames (`Command`, `BlockIn`,
//! `Shutdown`) — the handshake `Scan` the proxy sends itself counts
//! as message 1.

use std::io::{self, Read, Write};
use std::process::ExitCode;

use ruviform_plugin_core::wire::{self, WIRE_FRAME_HEADER_LEN, WIRE_MAX_PAYLOAD};
use ruviform_plugin_core::{HostError, Lifecycle, LifecycleEvent, PluginDescriptor, PrepareError};

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

/// Parsed CLI flags.
struct Flags {
    /// Die upon receiving message N (before dispatch).
    crash_after_msgs: Option<u64>,
    /// Exit code used with `crash_after_msgs` (default 3).
    crash_exit_code: i32,
    /// Stop answering commands after message N.
    hang_after_msgs: Option<u64>,
    /// Never read stdin again after message N.
    stop_reading_after_msgs: Option<u64>,
    /// Spawn a stdio-inheriting heir then exit after message N.
    leave_heir_after_msgs: Option<u64>,
    /// Heir mode: sleep this many ms then exit (`None` = normal serve).
    sleep_ms: Option<u64>,
    /// `Hello.version` override.
    hello_version: u16,
}

fn parse_flags() -> Flags {
    let mut flags = Flags {
        crash_after_msgs: None,
        crash_exit_code: 3,
        hang_after_msgs: None,
        stop_reading_after_msgs: None,
        leave_heir_after_msgs: None,
        sleep_ms: None,
        hello_version: wire::WIRE_VERSION,
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
        } else if let Some(value) = arg.strip_prefix("--stop-reading-after-msgs=") {
            flags.stop_reading_after_msgs = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--leave-heir-after-msgs=") {
            flags.leave_heir_after_msgs = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--sleep-ms=") {
            flags.sleep_ms = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--hello-version=") {
            flags.hello_version = value.parse().unwrap_or(wire::WIRE_VERSION);
        } else if arg == "--ruviform-plugin-id" || arg == "--ruviform-shm-key" {
            // Value-carrying bootstrap flags — consumed and ignored.
            let _ = args.next();
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
    wire::encode(message, &mut frame);
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
            transition(lifecycle, LifecycleEvent::StopProcessing).map(|()| wire::Reply::Ok)
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
                    let result = dispatch(&mut lifecycle, &mut state, &cmd);
                    if write_message(&mut stdout, &wire::Message::Reply { seq, result }).is_err() {
                        return ExitCode::FAILURE;
                    }
                }
            }
            wire::Message::Shutdown => return ExitCode::SUCCESS,
            // `BlockIn` (data plane) and child→parent kinds arriving
            // this direction are ignored — the data-plane task wires
            // slot processing.
            _ => {}
        }
    }
}
