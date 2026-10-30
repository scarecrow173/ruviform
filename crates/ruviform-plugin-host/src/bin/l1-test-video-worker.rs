//! `l1-test-video-worker` — the deterministic wire-conformant L1
//! **video** child fixture for `tests/l1_video_e2e.rs` (spawned via
//! `CARGO_BIN_EXE_l1-test-video-worker`; it stays inside this crate
//! — no cross-package bin resolution).
//!
//! It speaks the `ruviform-plugin-core` wire protocol exactly as a
//! real `ruviform-plugin-l1-video-runner` child does: `Hello` on
//! startup, then a single-threaded frame loop — `Command` → `Reply`
//! (a registry keyed by the wire `instance` envelope — `Scan` is
//! child-scoped, `CreateVideoInstance`/`DestroyInstance` mint and
//! reclaim entries, and every other verb resolves the handle,
//! `InvalidHandle` on a miss — with a `Lifecycle` state machine per
//! entry deciding transition legality, matching the child-side
//! semantics of the design), inbound `BlockIn` silently dropped
//! (the video child owns no doorbell data plane — direction-legal
//! but unattributable foreign traffic, inert per the wire
//! contract), direction-illegal child→parent kinds
//! (`Reply`/`BlockDone`/`Hello` inbound) folded into the
//! `run_video` protocol-violation exit, `Shutdown` → `exit(0)`,
//! stdin EOF → `exit(0)`.
//!
//! This is a *hand-rolled* child, deliberately **not** `run_video`:
//! the E2E needs wire-level fault injection the runner's erased
//! surface cannot produce (`Rendered` replies carrying a
//! noncanonical `output` descriptor, plus the argv-injected crash/
//! hang/over-limit matrix below). It depends only on the
//! core wire/video/shared-memory layers — no `ruviform-plugin-export`
//! dependency (the host crate must not grow a host→export edge).
//!
//! ## argv
//!
//! | flag | effect |
//! | --- | --- |
//! | `--crash-after-msgs=N` | `process::exit` upon *receiving* the Nth parent→child message — before dispatch, so the Nth command's waiter stays pending and is woken by EOF/`try_wait` |
//! | `--crash-exit-code=C` | exit code used by `--crash-after-msgs` (default 3; `0` simulates a voluntary `exit(0)`, which is still a `Failed` violation) |
//! | `--hang-after-msgs=N` | after the Nth received message, keep reading but stop answering commands (`0` hangs even the handshake `Scan`); `Shutdown` is still honored so teardown stays deterministic |
//! | `--stop-reading-after-msgs=N` | after the Nth received message is fully answered, never read stdin again — a wedged child that stays alive but never drains the pipe; only a kill ends it |
//! | `--leave-heir-after-msgs=N` | after the Nth received message is fully answered, spawn a copy of this binary with `--sleep-ms=15000` (inheriting stdio — the grandchild holds the parent's stdout pipe open) then `exit(0)`; EOF never reaches the reader thread |
//! | `--sleep-ms=M` | sleep `M` ms then exit 0 — the heir mode used by `--leave-heir-after-msgs`; not meant to be spawned directly by tests |
//! | `--hello-version=V` | `Hello.version` override (default [`wire::WIRE_VERSION`]) for the version-mismatch handshake test |
//! | `--fail-initialize` | `VideoInitialize` replies `Err(HostError::PluginFailed)` without committing — the entry stays `Created` (the real runner's `InitFailed` destination), so `Destroy` stays legal |
//! | `--fail-start-processing` | `VideoStartProcessing` replies `Err(HostError::PluginFailed)` without committing — the entry stays `Activated` (the real runner's hook-failure rollback destination), so cleanup unwinds `Deactivate` → `Destroy` |
//! | `--fail-stop-processing` | `VideoStopProcessing` commits `Processing → Activated` then replies `Err(HostError::PluginFailed)` — the real runner's commit-then-advisory shape (a hook error arrives post-commit), exercising the host-side mirror fix |
//! | `--fail-deactivate` | `VideoDeactivate` commits `Activated → Initialized` then replies `Err(HostError::PluginFailed)` — the same commit-then-advisory shape as `--fail-stop-processing` |
//! | `--fail-render` | `VideoRender` replies `Err(HostError::PluginFailed)` without committing — the `out` region is never written, so the host-observable output keeps its previous contents |
//! | `--noncanonical-output` | a successful `VideoRender` answers `Rendered{output}` whose plane-0 `stride` is bumped past canonical (`row_bytes + 4`) — decode-legal but semantically noncanonical, so the host's semantic gate (not the decoder) fires: that instance converges `Failed` and the caller sees `Err(Ipc)` |
//! | `--fail-param-value` | `VideoGetParamValue`/`VideoSetParamValue` reply `Err(HostError::PluginFailed)` on a *known* id — the host-observable shape of encode obligation (b) (a plugin whose `parameter_value`/`set_parameter` reports a wart its adapter would normalize); an unknown id still answers `Err(InvalidValue)` — the membership check precedes the hook |
//! | `--overlimit-params` | `VideoGetParams`/`VideoGetParamInfo` reply `Err(HostError::PluginFailed)` — the adapter-normalized answer of encode obligation (c), emulated fixture-side: an over-limit `parameters()`/`parameter()` report can never reach the wire, so its defined answer is `Err(PluginFailed)` with the child left alive |
//! | `--stderr-bytes=N` | emit `N` stderr bytes at startup, before `Hello` — the drain's pre-handshake capture oracle |
//! | `--heir-stderr-bytes=N` / `--heir-stderr-delay-ms=M` | heir mode: emit `N` stderr bytes after `M` ms — forwarded into the heir's argv by the parent's `--leave-heir-after-msgs` spawn, exercising the drain's deadline flags |
//!
//! A `--ruviform-*` argument is a defined usage rejection: stderr +
//! `exit(2)`, no `Hello`.
//!
//! Message counting covers parent→child frames (`Command`, `BlockIn`,
//! `Shutdown`) — the handshake `Scan` the proxy sends counts as
//! message 1 and its `CreateVideoInstance` as message 2.

use std::collections::HashMap;
use std::io;
use std::process::ExitCode;
use std::sync::atomic::{Ordering, fence};

use ruviform_plugin_core::wire::{self, vshm};
use ruviform_plugin_core::{
    HostError, Lifecycle, LifecycleEvent, ParamFlags, ParamId, PlainParamInfoOwned,
    PlainParamValue, PluginDescriptor, PrepareError, VideoProcessConfig,
};
use shared_memory::{Shmem, ShmemConf};

/// The single built-in descriptor this worker reports for `Scan` and
/// validates nothing else against — the E2E's
/// `VideoOutOfProcessSpec.descriptor` must match it field-for-field.
const WORKER_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l1-test-video-worker",
    name: "L1 Test Video Worker",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &["video-effect"],
};

/// `parameters()` — the fixture's two-param surface. Id 0 is a
/// double `Amount` (settable, render-transform scale); id 1 is a
/// choice `Mode` (`Invert` = per-byte `!b`, `Fill` = constant
/// `0x80`) so a `set_parameter` round-trip is observable in the
/// next rendered frame's bytes.
const PARAM_AMOUNT: u32 = 0;
const PARAM_MODE: u32 = 1;

/// The mode-1 fill byte — `0x80` is visually mid-gray for 8-bit
/// planes and deterministic for the byte oracle.
const FILL_BYTE: u8 = 0x80;

/// Parsed CLI flags.
///
/// The injected-failure knobs are flat bools by design — argv is a
/// flat string list, so a nested state model would only shuffle the
/// same flags around; `struct_excessive_bools` is expected here.
#[expect(clippy::struct_excessive_bools)]
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
    /// `VideoInitialize` replies `Err(PluginFailed)` without
    /// committing — the entry stays `Created` (the runner's
    /// `InitFailed` destination), so `Destroy` stays legal.
    fail_initialize: bool,
    /// `VideoStartProcessing` replies `Err(PluginFailed)` without
    /// committing — the entry stays `Activated` (the real runner's
    /// hook-failure rollback destination), so cleanup unwinds via
    /// `Deactivate`.
    fail_start_processing: bool,
    /// `VideoStopProcessing` commits the transition then replies
    /// `Err(PluginFailed)` — the real runner's advisory-error shape.
    fail_stop_processing: bool,
    /// `VideoDeactivate` commits the transition then replies
    /// `Err(PluginFailed)` — the same commit-then-advisory shape as
    /// `fail_stop_processing`.
    fail_deactivate: bool,
    /// `VideoRender` replies `Err(PluginFailed)` without writing the
    /// `out` region — the defined hook failure.
    fail_render: bool,
    /// A successful `VideoRender` answers `Rendered{output}` with a
    /// noncanonical plane-0 stride — a semantic protocol violation
    /// the host converges per-instance, not child-scope.
    noncanonical_output: bool,
    /// `VideoGetParamValue`/`VideoSetParamValue` reply
    /// `Err(PluginFailed)` on a known id — obligation (b)'s
    /// host-observable shape.
    fail_param_value: bool,
    /// `VideoGetParams`/`VideoGetParamInfo` reply
    /// `Err(PluginFailed)` — obligation (c)'s adapter-normalized
    /// answer, emulated fixture-side.
    overlimit_params: bool,
    /// Emit this many stderr bytes immediately at startup — before
    /// `Hello` — so the host's drain captures pre-handshake output.
    stderr_bytes: Option<u64>,
    /// Heir mode: emit this many stderr bytes after
    /// `heir_stderr_delay_ms`. The parent's `--leave-heir-after-msgs`
    /// spawn forwards both `--heir-stderr-*` values into the heir's
    /// argv — the heir parses the same flag names.
    heir_stderr_bytes: Option<u64>,
    /// Heir mode: delay before `heir_stderr_bytes` is emitted.
    heir_stderr_delay_ms: Option<u64>,
}

/// Writes `n` bytes to stderr in bounded chunks — the drain-capture
/// oracle for `--stderr-bytes`/`--heir-stderr-bytes`.
fn emit_stderr(mut n: u64) {
    use std::io::Write as _;
    let chunk = [b'!'; 4096];
    let mut stderr = io::stderr();
    while n > 0 {
        let take = usize::try_from(n).unwrap_or(usize::MAX).min(chunk.len());
        if stderr.write_all(&chunk[..take]).is_err() {
            return;
        }
        n -= take as u64;
    }
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
        fail_initialize: false,
        fail_start_processing: false,
        fail_stop_processing: false,
        fail_deactivate: false,
        fail_render: false,
        noncanonical_output: false,
        fail_param_value: false,
        overlimit_params: false,
        stderr_bytes: None,
        heir_stderr_bytes: None,
        heir_stderr_delay_ms: None,
    };
    for arg in std::env::args_os().skip(1) {
        let arg = arg.to_string_lossy();
        if arg.starts_with("--ruviform-") {
            // A real runner answers a bootstrap-flag argument the same
            // way — the defined usage rejection.
            eprintln!("l1-test-video-worker: unexpected argument `{arg}`");
            std::process::exit(2);
        }
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
        } else if arg == "--fail-initialize" {
            flags.fail_initialize = true;
        } else if arg == "--fail-start-processing" {
            flags.fail_start_processing = true;
        } else if arg == "--fail-stop-processing" {
            flags.fail_stop_processing = true;
        } else if arg == "--fail-deactivate" {
            flags.fail_deactivate = true;
        } else if arg == "--fail-render" {
            flags.fail_render = true;
        } else if arg == "--noncanonical-output" {
            flags.noncanonical_output = true;
        } else if arg == "--fail-param-value" {
            flags.fail_param_value = true;
        } else if let Some(value) = arg.strip_prefix("--stderr-bytes=") {
            flags.stderr_bytes = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--heir-stderr-bytes=") {
            flags.heir_stderr_bytes = value.parse().ok();
        } else if let Some(value) = arg.strip_prefix("--heir-stderr-delay-ms=") {
            flags.heir_stderr_delay_ms = value.parse().ok();
        } else if arg == "--overlimit-params" {
            flags.overlimit_params = true;
        }
    }
    flags
}

fn transition(lifecycle: &mut Lifecycle, event: LifecycleEvent) -> Result<(), HostError> {
    lifecycle
        .transition(event)
        .map(|_| ())
        .map_err(HostError::Lifecycle)
}

/// One registry slot — the whole per-instance state a
/// `CreateVideoInstance` mints and a `DestroyInstance` reclaims:
/// lifecycle legality, the attached `vshm` segment, the minted
/// config, and the fixture's small parameter store.
struct VideoEntry {
    /// The instance's `Created → …` legality state machine.
    lifecycle: Lifecycle,
    /// The minted `VideoProcessConfig` — the shape every render
    /// descriptor must agree with.
    config: VideoProcessConfig,
    /// The instance's shared-memory view — `CreateVideoInstance`'s
    /// `shm_key` attached with its declared region sizes.
    segment: Segment,
    /// `Amount` (id 0) — the double surface.
    amount: f64,
    /// `Mode` (id 1) — `0` = invert, `1` = fill; a `set_parameter`
    /// round-trip the pixel oracle can observe.
    mode: u32,
}

impl VideoEntry {
    /// The child-side `Lifecycle` semantics of the design: transition
    /// legality is decided here, plugin-hook failures are reported as
    /// `HostError`s — mirroring what the real runner's
    /// `erase_video` boundary does. Instance-scoped verbs only; the
    /// registry-level dispatch owns `Scan`/`CreateVideoInstance`/
    /// `DestroyInstance`.
    ///
    /// `flags` carries the injection knobs the whole child answers
    /// to — a struct reference, not positional bools, so wiring a new
    /// knob cannot silently swap two flags.
    fn dispatch(&mut self, cmd: &wire::Command, flags: &Flags) -> Result<wire::Reply, HostError> {
        match cmd {
            wire::Command::VideoInitialize => {
                // Legality before the injected hook failure — the
                // real runner validates the transition first, so an
                // illegal `Initialize` is `Lifecycle`, not
                // `PluginFailed`. The transition is never committed:
                // the entry stays `Created` (the runner's `InitFailed`
                // destination) and `Destroy` stays legal.
                self.lifecycle
                    .state()
                    .transition(LifecycleEvent::Initialize)
                    .map_err(HostError::Lifecycle)?;
                if flags.fail_initialize {
                    return Err(HostError::PluginFailed);
                }
                transition(&mut self.lifecycle, LifecycleEvent::Initialize)
                    .map(|()| wire::Reply::Ok)
            }
            wire::Command::VideoActivate => {
                transition(&mut self.lifecycle, LifecycleEvent::Activate).map(|()| wire::Reply::Ok)
            }
            wire::Command::VideoStartProcessing => {
                // Same non-commit shape as `--fail-initialize`:
                // legality first, then the injected hook failure —
                // the entry stays `Activated` (the real runner's
                // `start_processing` rollback destination), so the
                // unwind path starts at `Deactivate`.
                self.lifecycle
                    .state()
                    .transition(LifecycleEvent::StartProcessing)
                    .map_err(HostError::Lifecycle)?;
                if flags.fail_start_processing {
                    return Err(HostError::PluginFailed);
                }
                transition(&mut self.lifecycle, LifecycleEvent::StartProcessing)
                    .map(|()| wire::Reply::Ok)
            }
            wire::Command::VideoRender { at, input } => self.render(*at, input, flags),
            wire::Command::VideoStopProcessing => {
                // Commit first, like the real runner: the transition
                // lands before the hook's `Err` reply — the
                // commit-then-advisory shape.
                transition(&mut self.lifecycle, LifecycleEvent::StopProcessing)?;
                if flags.fail_stop_processing {
                    Err(HostError::PluginFailed)
                } else {
                    Ok(wire::Reply::Ok)
                }
            }
            wire::Command::VideoDeactivate => {
                // Same commit-then-advisory shape as
                // `VideoStopProcessing` — `Activated → Initialized`
                // lands before the advisory `Err`.
                transition(&mut self.lifecycle, LifecycleEvent::Deactivate)?;
                if flags.fail_deactivate {
                    Err(HostError::PluginFailed)
                } else {
                    Ok(wire::Reply::Ok)
                }
            }
            wire::Command::VideoGetParams => {
                if flags.overlimit_params {
                    // Obligation (c): an over-limit `parameters()`
                    // report can never reach the wire — its defined
                    // answer is `Err(PluginFailed)`, child alive.
                    return Err(HostError::PluginFailed);
                }
                Ok(wire::Reply::Params {
                    ids: vec![PARAM_AMOUNT, PARAM_MODE],
                })
            }
            wire::Command::VideoGetParamInfo { id } => {
                if flags.overlimit_params {
                    // Same obligation-(c) shape as `VideoGetParams`
                    // — the adapter-normalized answer for an
                    // over-limit `parameter()` report.
                    return Err(HostError::PluginFailed);
                }
                match *id {
                    PARAM_AMOUNT => Ok(wire::Reply::ParamInfo {
                        info: Some(amount_info()),
                    }),
                    PARAM_MODE => Ok(wire::Reply::ParamInfo {
                        info: Some(mode_info()),
                    }),
                    // Unknown id — the plugin-declared `None`, never
                    // an `Err`.
                    _ => Ok(wire::Reply::ParamInfo { info: None }),
                }
            }
            wire::Command::VideoGetParamValue { id } => {
                // Membership before the hook — an unknown id is
                // `Err(InvalidValue)`, the value-domain defined
                // rejection, never folded into `PluginFailed`.
                if !Self::has_param(*id) {
                    return Err(HostError::InvalidValue);
                }
                if flags.fail_param_value {
                    // Obligation (b): a plugin `parameter_value`
                    // report whose scalar is a wart is normalized to
                    // `Err(PluginFailed)` — child alive.
                    return Err(HostError::PluginFailed);
                }
                Ok(wire::Reply::ParamValue {
                    value: self.param_value(*id),
                })
            }
            wire::Command::VideoSetParamValue { id, value } => {
                if !Self::has_param(*id) {
                    return Err(HostError::InvalidValue);
                }
                if flags.fail_param_value {
                    return Err(HostError::PluginFailed);
                }
                self.set_param_value(*id, *value)
            }
            // Audio verbs — and `Scan`/`CreateInstance`/`DestroyInstance`
            // reaching an entry — are the defined rejection
            // `Err(Unsupported)` (the real runner's catch-all arm).
            _ => Err(HostError::Unsupported),
        }
    }

    /// `id ∈ parameters()` — the membership check every parameter
    /// verb shares (a fixture-level constant, hence `Self::`).
    fn has_param(id: u32) -> bool {
        matches!(id, PARAM_AMOUNT | PARAM_MODE)
    }

    /// The stored value for a known id.
    fn param_value(&self, id: u32) -> PlainParamValue {
        match id {
            PARAM_AMOUNT => PlainParamValue::Double(self.amount),
            _ => PlainParamValue::Choice(self.mode),
        }
    }

    /// `set_parameter` — a type-mismatched write is the same
    /// `Err(PluginFailed)` the real runner maps its fixture's
    /// parameter/type error to.
    fn set_param_value(
        &mut self,
        id: u32,
        value: PlainParamValue,
    ) -> Result<wire::Reply, HostError> {
        match (id, value) {
            (PARAM_AMOUNT, PlainParamValue::Double(v)) => self.amount = v,
            (PARAM_MODE, PlainParamValue::Choice(v)) => self.mode = v,
            _ => return Err(HostError::PluginFailed),
        }
        Ok(wire::Reply::Ok)
    }

    /// `VideoRender`: `Processing` gate first, then the descriptor's
    /// post-decode semantic check (mirroring the real runner's
    /// `check_input_desc`), the in-region transform into `out`, and
    /// `Rendered` with the runner-derived canonical descriptor —
    /// unless a flag injects the wart. A hook failure commits
    /// nothing to `out`.
    fn render(
        &mut self,
        _at: ruviform_media_core::MediaTime,
        desc: &wire::VideoFrameDesc,
        flags: &Flags,
    ) -> Result<wire::Reply, HostError> {
        transition(&mut self.lifecycle, LifecycleEvent::Process)?;
        check_input_desc(desc, &self.config, self.segment.in_bytes())?;
        if flags.fail_render {
            // The defined hook error — the `out` region stays
            // uncommitted and the child survives.
            return Err(HostError::PluginFailed);
        }
        // The doorbell-free equivalent of the audio `BlockIn` acquire
        // edge: the host's release side is its pipe write, so a real
        // fence plus the pipe read orders the frame bytes about to be
        // read; the Release side publishes `out` before the
        // `Rendered` reply can reach the pipe.
        fence(Ordering::Acquire);
        self.segment.render_frame(self.mode);
        fence(Ordering::Release);
        Ok(wire::Reply::Rendered {
            output: output_desc(&self.config, desc, flags.noncanonical_output),
        })
    }
}

/// The id-0 `Amount` metadata — a double in `[0, 1]` defaulting to
/// `1.0`, automatable.
fn amount_info() -> PlainParamInfoOwned {
    PlainParamInfoOwned {
        id: ParamId(PARAM_AMOUNT),
        name: "Amount".to_owned(),
        unit: String::new(),
        min_value: 0.0,
        max_value: 1.0,
        display_min_value: 0.0,
        display_max_value: 1.0,
        default_value: PlainParamValue::Double(1.0),
        flags: ParamFlags::AUTOMATABLE,
        choices: Vec::new(),
    }
}

/// The id-1 `Mode` metadata — a two-entry `Choice` list whose
/// default index is in bounds (the `Choice ⇔ choices non-empty`
/// codec contract).
fn mode_info() -> PlainParamInfoOwned {
    PlainParamInfoOwned {
        id: ParamId(PARAM_MODE),
        name: "Mode".to_owned(),
        unit: String::new(),
        min_value: 0.0,
        max_value: 1.0,
        display_min_value: 0.0,
        display_max_value: 1.0,
        default_value: PlainParamValue::Choice(0),
        flags: ParamFlags::AUTOMATABLE.union(ParamFlags::STEPPED),
        choices: vec!["Invert".to_owned(), "Fill".to_owned()],
    }
}

/// `VideoRender.input`'s post-decode semantic check — the checks
/// the codec bounds layer cannot express: every declared plane must
/// sit in canonical placement (`stride == row_bytes`, `offset(p) ==
/// Σlen(0..p)`, `len` the canonical plane size) inside the `in`
/// region, and the descriptor's dims/format must equal the
/// instance config's. A noncanonical or overflowing descriptor is
/// `Err(HostError::Ipc)` — a host contract violation in the same
/// class as a duplicate handle — while a config dims/format
/// mismatch is `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`,
/// mirroring the real runner's `check_input_desc` byte for byte.
fn check_input_desc(
    desc: &wire::VideoFrameDesc,
    config: &VideoProcessConfig,
    region_bytes: usize,
) -> Result<(), HostError> {
    for (index, declared) in desc.planes.iter().enumerate() {
        let layout = vshm::plane_layout(desc.format, desc.width, desc.height, index)
            .ok_or(HostError::Ipc)?;
        if u64::from(declared.stride) != layout.stride
            || u64::from(declared.offset) != layout.offset
            || u64::from(declared.len) != layout.len
            || u64::from(declared.offset) + u64::from(declared.len) > region_bytes as u64
        {
            return Err(HostError::Ipc);
        }
    }
    if desc.format != config.format()
        || desc.width != config.width()
        || desc.height != config.height()
    {
        return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
    }
    Ok(())
}

/// The canonical `Rendered.output` descriptor the runner reports —
/// runner-derived from the instance config (never the plugin's own
/// report): config dims/format, canonical plane placement, and
/// metadata seeded from the *input* descriptor so
/// `pts`/`duration`/`color` pass through unchanged.
///
/// `noncanonical` fabricates the wire-level wart the task's fault
/// matrix needs: plane 0's `stride` is bumped past `row_bytes` —
/// still inside the codec's `WIRE_VIDEO_MAX_STRIDE` bound (so the
/// reply decodes) but semantically noncanonical, so the host's
/// post-decode semantic gate is what converges the instance.
fn output_desc(
    config: &VideoProcessConfig,
    input: &wire::VideoFrameDesc,
    noncanonical: bool,
) -> wire::VideoFrameDesc {
    let mut planes: Vec<wire::VideoFramePlane> = (0..config.format().plane_count())
        .map(|index| {
            let layout =
                vshm::plane_layout(config.format(), config.width(), config.height(), index)
                    .expect("canonical layout exists for every plane");
            wire::VideoFramePlane {
                offset: u32::try_from(layout.offset).expect("canonical offset fits u32"),
                len: u32::try_from(layout.len).expect("canonical len fits u32"),
                stride: u32::try_from(layout.stride).expect("canonical stride fits u32"),
            }
        })
        .collect();
    if noncanonical {
        // `stride != row_bytes` — a semantic violation the decoder
        // cannot see (still under `WIRE_VIDEO_MAX_STRIDE`), so the
        // host-side `validate_rendered_desc` gate is what fires.
        planes[0].stride += 4;
    }
    wire::VideoFrameDesc {
        format: config.format(),
        width: config.width(),
        height: config.height(),
        planes,
        pts: input.pts,
        duration: input.duration,
        color: input.color,
    }
}

/// The registry-level dispatch: `Scan` is child-scoped (the wire
/// decoder already required `WIRE_INSTANCE_NONE`),
/// `CreateVideoInstance`/`DestroyInstance` mint and reclaim slots by
/// the envelope `instance`, and every other verb resolves the
/// handle — `InvalidHandle` on a miss, exactly like the real
/// runner.
fn dispatch(
    instances: &mut HashMap<u32, VideoEntry>,
    instance: u32,
    cmd: &wire::Command,
    flags: &Flags,
) -> Result<wire::Reply, HostError> {
    match cmd {
        wire::Command::Scan => Ok(wire::Reply::Scanned(vec![wire::Descriptor::from(
            WORKER_DESCRIPTOR,
        )])),
        wire::Command::CreateVideoInstance {
            plugin_id,
            shm_key,
            config,
        } => {
            // The real runner's ordering: plugin selection → slot
            // budget → duplicate handle → attach → insert (a failed
            // step leaves the registry untouched).
            if *plugin_id != WORKER_DESCRIPTOR.id {
                return Err(HostError::UnknownPluginId);
            }
            if instances.len() >= usize::try_from(wire::WIRE_MAX_INSTANCES).unwrap_or(usize::MAX) {
                return Err(HostError::InstanceLimit);
            }
            if instances.contains_key(&instance) {
                // The host mints handles — a reuse while live is a
                // protocol violation, not a lifecycle error.
                return Err(HostError::Ipc);
            }
            let segment = Segment::attach(shm_key, config).ok_or(HostError::Ipc)?;
            instances.insert(
                instance,
                VideoEntry {
                    lifecycle: Lifecycle::new(),
                    config: *config,
                    segment,
                    amount: 1.0,
                    mode: 0,
                },
            );
            Ok(wire::Reply::Ok)
        }
        wire::Command::DestroyInstance => match instances.remove(&instance) {
            Some(_) => Ok(wire::Reply::Ok),
            None => Err(HostError::InvalidHandle),
        },
        _ => match instances.get_mut(&instance) {
            Some(entry) => entry.dispatch(cmd, flags),
            None => Err(HostError::InvalidHandle),
        },
    }
}

/// The worker's frame segment: a non-owning attach to the
/// `CreateVideoInstance` `shm_key` object, header-validated and
/// config-checked exactly like the real runner's
/// `ruviform_plugin_export::vshm::Segment` — `in`/`out` regions are
/// the two equal canonical frame windows the header declares.
struct Segment {
    mem: Shmem,
    in_off: usize,
    in_bytes: usize,
    out_off: usize,
    out_bytes: usize,
}

impl Segment {
    /// Attaches to `key` and validates it end to end —
    /// structurally through `wire::vshm::validate_header`, then for
    /// config agreement (both regions must be exactly the
    /// `frame_bytes(config)` the create command carried, bounded by
    /// `WIRE_VIDEO_MAX_FRAME_BYTES`). `None` on any failure: the
    /// `CreateVideoInstance` replies `Err(Ipc)`, exactly like the
    /// real runner.
    fn attach(key: &str, config: &VideoProcessConfig) -> Option<Self> {
        let mem = ShmemConf::new().os_id(key).open().ok()?;
        let bytes = bytes(&mem);
        let (in_bytes, out_bytes) = vshm::validate_header(bytes)?;
        let frame_bytes = vshm::frame_bytes(config);
        if frame_bytes > wire::WIRE_VIDEO_MAX_FRAME_BYTES {
            return None;
        }
        if u64::from(in_bytes) != frame_bytes || u64::from(out_bytes) != frame_bytes {
            return None;
        }
        Some(Self {
            mem,
            in_off: vshm::HEADER_LEN,
            in_bytes: in_bytes as usize,
            out_off: vshm::HEADER_LEN + in_bytes as usize,
            out_bytes: out_bytes as usize,
        })
    }

    /// The declared `in`-region capacity — the bound a `VideoRender`
    /// input descriptor is region-fit checked against.
    fn in_bytes(&self) -> usize {
        self.in_bytes
    }

    /// The video data-plane step: read the `in` region and write
    /// `out` — mode `0` inverts every byte (the invert oracle),
    /// mode `1` fills with [`FILL_BYTE`]. The regions are the two
    /// disjoint canonical windows `in_off`/`out_off` declare, so
    /// one `split_at_mut` hands back a shared `in` and an
    /// exclusive `out` without aliasing.
    fn render_frame(&mut self, mode: u32) {
        let mem = bytes_mut(&mut self.mem);
        let (before_out, out_region) = mem.split_at_mut(self.out_off);
        let in_region = &before_out[self.in_off..self.in_off + self.in_bytes];
        let out = &mut out_region[..self.out_bytes];
        for (src, dst) in in_region.iter().zip(out.iter_mut()) {
            *dst = if mode == 0 { !*src } else { FILL_BYTE };
        }
    }
}

/// The segment as a read-only byte slice.
fn bytes(mem: &Shmem) -> &[u8] {
    // SAFETY: `Shmem::as_slice` is `unsafe` because the crate cannot
    // prove the mapped range immutable — another process maps the
    // same pages. The L1 vshm protocol supplies that guarantee: the
    // header is owner-written before spawn and only read here, and
    // the in/out regions are touched by exactly one side at a time
    // under the command/reply round trip. Same documented deviation
    // as `ruviform-plugin-export::vshm` and `l1-test-worker`.
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice()
    }
}

/// The segment as a mutable byte slice.
fn bytes_mut(mem: &mut Shmem) -> &mut [u8] {
    // SAFETY: same protocol argument as `bytes` — the `out` region
    // written through `render_frame` is owned by this child between
    // reading `VideoRender` and writing `Rendered` under the
    // serialized wire protocol, and the `&mut Shmem` borrow keeps
    // the slice exclusive within the call.
    #[expect(unsafe_code)]
    unsafe {
        mem.as_slice_mut()
    }
}

fn main() -> ExitCode {
    let flags = parse_flags();
    if let Some(n) = flags.stderr_bytes {
        // Startup stderr — emitted before `Hello` so the host drain
        // captures pre-handshake output.
        emit_stderr(n);
    }
    if let Some(ms) = flags.sleep_ms {
        // Heir mode — a spawned grandchild that just holds the
        // inherited pipes open, then exits cleanly. The optional
        // delayed stderr write exercises the drain's deadline flags.
        if let Some(delay) = flags.heir_stderr_delay_ms {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
        if let Some(n) = flags.heir_stderr_bytes {
            emit_stderr(n);
        }
        std::thread::sleep(std::time::Duration::from_millis(ms));
        return ExitCode::SUCCESS;
    }
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    if wire::write_message(
        &mut stdout,
        &wire::Message::Hello {
            version: flags.hello_version,
            capabilities: 0,
        },
        &mut Vec::new(),
    )
    .is_err()
    {
        return ExitCode::FAILURE;
    }

    // The registry — `CreateVideoInstance` mints an entry keyed by
    // the envelope `instance` (its `shm_key` attaches there — no
    // segment exists before the first `CreateVideoInstance`
    // arrives).
    let mut instances: HashMap<u32, VideoEntry> = HashMap::new();
    let mut received: u64 = 0;

    loop {
        if flags
            .stop_reading_after_msgs
            .is_some_and(|limit| received >= limit)
        {
            // Wedged-child simulation: stay alive but never drain
            // stdin again. Only `child.kill()` ends this; `Shutdown`
            // is never seen.
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
            // reaped, so the reader thread never sees EOF. (The
            // running heir briefly locks this exe image on Windows —
            // its lifetime stays short so rebuilds are not blocked
            // for long.)
            let mut heir =
                std::process::Command::new(std::env::current_exe().expect("heir needs this exe"));
            heir.arg("--sleep-ms=15000");
            // Forward the heir's own stderr knobs — the heir parses
            // the same flag names, so without this wiring the parsed
            // values would never reach it.
            if let Some(n) = flags.heir_stderr_bytes {
                heir.arg(format!("--heir-stderr-bytes={n}"));
            }
            if let Some(m) = flags.heir_stderr_delay_ms {
                heir.arg(format!("--heir-stderr-delay-ms={m}"));
            }
            let _ = heir.spawn();
            std::process::exit(0);
        }
        let Ok(message) = wire::read_message(&mut stdin, &mut Vec::new()) else {
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
            wire::Message::Command { seq, instance, cmd } => {
                if !hung {
                    let result = dispatch(&mut instances, instance, &cmd, &flags);
                    if wire::write_message(
                        &mut stdout,
                        &wire::Message::Reply {
                            seq,
                            instance,
                            result,
                        },
                        &mut Vec::new(),
                    )
                    .is_err()
                    {
                        return ExitCode::FAILURE;
                    }
                }
            }
            // The video child owns no doorbell data plane — an
            // inbound `BlockIn` is direction-legal but unattributable
            // foreign traffic: drop it, never a reply, never a
            // violation (the wire contract's inert rule — the video
            // asymmetry against the direction-illegal kinds below).
            wire::Message::BlockIn { .. } => {}
            wire::Message::Shutdown => return ExitCode::SUCCESS,
            // Direction-illegal frames arriving child→parent this way
            // — `Reply`/`BlockDone`/`Hello` — are the protocol
            // violation `serve`/`run_video` folds into `normalize_all`
            // + failure.
            _ => return ExitCode::FAILURE,
        }
    }
}
