//! Spawn E2E for the reference L1 video runner
//! `ruviform-plugin-l1-video-runner` (`CARGO_BIN_EXE_*` — stays
//! inside this crate; no cross-package bin resolution).
//!
//! The test plays the *minimal host*, mirrored from
//! `l1_runner_e2e.rs` onto the wire-v3 video contract: it creates
//! the `vshm` segments (host-owned — the `[header][in][out]` layout
//! from `ruviform_plugin_core::wire::vshm`), spawns the runner bare
//! (wire v3 takes no argv), speaks the framed protocol over the
//! child's stdio and verifies the full contract — `Hello` →
//! child-scoped `Scan` → `CreateVideoInstance` (registry mint,
//! stash of the carried `VideoProcessConfig`) → the video lifecycle
//! verbs (`VideoInitialize` replays the stash) → `VideoRender`
//! through the shared `in`/`out` frame regions → `DestroyInstance` →
//! `Shutdown` exit(0) — plus the defined rejections (argv usage,
//! attach-header corruption, illegal transitions, dead/duplicate/
//! full registry handles, unknown `ParamId` pre-checks, plugin
//! panic) and the video-specific transport rules: an inbound
//! `BlockIn` is dropped silently (the video child owns no doorbell
//! data plane) while `BlockDone`/`Reply`/`Hello` inbound stay
//! direction-illegal protocol violations.
//!
//! Frame descriptors are runner-side *post-decode semantic*
//! subjects: canonical placement and region fit are `Err(Ipc)`
//! replies, a config dims/format mismatch is
//! `Err(Prepare(UnsupportedConfig))` — the child survives every
//! one.

use std::io::BufReader;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use ruviform_media_core::{FrameRate, MediaDuration, MediaTime, TimeBase};
use ruviform_plugin_core::wire::{self, Command as WireCommand, Message, Reply as WireReply, vshm};
use ruviform_plugin_core::{
    HostError, PlainParamValue, PluginDescriptor, PrepareError, ProcessResult, VideoProcessConfig,
};
use ruviform_video_core::{ColorInfo, ColorPrimaries, PixelFormat, TransferCharacteristics};
use shared_memory::{Shmem, ShmemConf};

const RUNNER: &str = env!("CARGO_BIN_EXE_ruviform-plugin-l1-video-runner");
const AUDIO_RUNNER: &str = env!("CARGO_BIN_EXE_ruviform-plugin-l1-runner");
const INVERT_ID: &str = "dev.ruviform.mock.video.invert";
const PANIC_ID: &str = "dev.ruviform.mock.video.panic";
const OVERLIMIT_ID: &str = "dev.ruviform.mock.video.overlimit";
const OVERLIMIT_INFO_ID: &str = "dev.ruviform.mock.video.overlimit-info";

const INVERT_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.video.invert",
    name: "Invert",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["video-effect"],
};

const TIMEOUT: Duration = Duration::from_secs(15);

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
    /// Next command-envelope seq.
    next_seq: u64,
}

impl MiniHost {
    fn spawn(binary: &str, args: &[&str]) -> Self {
        Self::spawn_with_env(binary, args, &[])
    }

    /// Spawn with extra environment — the `descriptor()`/`create()`/
    /// `render` panic fixtures are gated on `RUVIFORM_PANIC_*`.
    fn spawn_with_env(binary: &str, args: &[&str], env: &[(&str, &str)]) -> Self {
        let mut command = Command::new(binary);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (name, value) in env {
            command.env(name, value);
        }
        let mut child = command.spawn().expect("spawn video runner");
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

    /// Spawns the reference video runner — wire v3 is spawned bare:
    /// plugin selection, the vshm key and the config ride the
    /// `CreateVideoInstance` payload.
    fn spawn_runner() -> Self {
        Self::spawn(RUNNER, &[])
    }

    /// Spawns the video runner with panic fixtures armed.
    fn spawn_panicking(env: &[(&str, &str)]) -> Self {
        Self::spawn_with_env(RUNNER, &[], env)
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

    /// `CreateVideoInstance` minting `instance` for `plugin_id` on the
    /// segment `key` under `config`.
    fn create_video_instance(
        &mut self,
        instance: u32,
        plugin_id: &str,
        key: &str,
        config: &VideoProcessConfig,
    ) -> Result<WireReply, HostError> {
        self.command(
            instance,
            WireCommand::CreateVideoInstance {
                plugin_id: plugin_id.to_owned(),
                shm_key: key.to_owned(),
                config: *config,
            },
        )
    }

    fn recv(&self) -> Message {
        self.inbound
            .recv_timeout(TIMEOUT)
            .expect("timed out waiting for a child frame")
            .unwrap_or_else(|reason| panic!("child stream failed: {reason}"))
    }

    /// Receives the next frame, asserting it is a `Reply` for `seq`
    /// echoing `instance` (the reply's envelope copies the command's
    /// verbatim).
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

    /// `Shutdown` then expect a clean exit(0).
    fn shutdown_ok(&mut self) {
        self.send(&Message::Shutdown);
        let status = self.wait_exit();
        assert!(status.success(), "Shutdown must exit 0, got {status}");
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
/// defined rejection" oracle the registry contract pins.
fn assert_alive(host: &mut MiniHost) {
    let descriptors = expect_scanned(host.scan());
    assert_eq!(descriptors.len(), 4, "Scan must still enumerate factories");
}

// --- minimal host vshm side -------------------------------------------

fn unique_key(tag: &str) -> String {
    format!("/ruviform-l1-video-e2e-{}-{tag}", std::process::id())
}

/// The test config — 64×64 RGBA8 at 30 fps, 16 KiB per frame.
fn test_config() -> VideoProcessConfig {
    VideoProcessConfig::new(PixelFormat::Rgba8, 64, 64, FrameRate::new(30, 1).unwrap())
        .expect("valid config")
}

/// `frame_bytes`/`segment_len` as a usize — bounded by
/// `WIRE_VIDEO_MAX_FRAME_BYTES`, always fits.
fn frame_len(frame_bytes: u64) -> usize {
    usize::try_from(frame_bytes).expect("frame bytes fit usize")
}

/// Creates a vshm segment sized for `config` with the canonical
/// header — what the real host's `vshm::Segment::create` produces.
fn create_video_segment(key: &str, config: &VideoProcessConfig) -> Shmem {
    let frame_bytes = vshm::frame_bytes(config);
    let header = vshm::header_bytes(
        u32::try_from(frame_bytes).expect("frame bytes fit u32"),
        u32::try_from(frame_bytes).expect("frame bytes fit u32"),
    );
    create_video_segment_raw(key, frame_len(vshm::segment_len(frame_bytes)), &header)
}

/// Creates a segment of arbitrary size carrying an arbitrary header —
/// the attach-rejection matrix overrides one field at a time against
/// otherwise-valid values.
fn create_video_segment_raw(key: &str, size: usize, header: &[u8; vshm::HEADER_LEN]) -> Shmem {
    let mut mem = ShmemConf::new()
        .os_id(key)
        .size(size)
        .create()
        .expect("create vshm segment");
    segment_bytes_mut(&mut mem)[..vshm::HEADER_LEN].copy_from_slice(header);
    mem
}

/// The canonical descriptor a conforming host puts in `VideoRender` —
/// config dims/format, packed canonical plane placement.
fn canonical_desc(
    config: &VideoProcessConfig,
    pts: MediaTime,
    duration: Option<MediaDuration>,
    color: ColorInfo,
) -> wire::VideoFrameDesc {
    let planes = (0..config.format().plane_count())
        .map(|index| {
            let layout =
                vshm::plane_layout(config.format(), config.width(), config.height(), index)
                    .expect("canonical layout exists");
            wire::VideoFramePlane {
                offset: u32::try_from(layout.offset).expect("offset fits u32"),
                len: u32::try_from(layout.len).expect("len fits u32"),
                stride: u32::try_from(layout.stride).expect("stride fits u32"),
            }
        })
        .collect();
    wire::VideoFrameDesc {
        format: config.format(),
        width: config.width(),
        height: config.height(),
        planes,
        pts,
        duration,
        color,
    }
}

/// The `in` region of `mem` for a `frame_bytes`-sized frame.
fn in_region_mut(mem: &mut Shmem, frame_bytes: u64) -> &mut [u8] {
    let len = frame_len(frame_bytes);
    &mut segment_bytes_mut(mem)[vshm::HEADER_LEN..vshm::HEADER_LEN + len]
}

/// The `out` region of `mem` — the second `frame_bytes` half.
fn out_region(mem: &Shmem, frame_bytes: u64) -> &[u8] {
    let len = frame_len(frame_bytes);
    &segment_bytes(mem)[vshm::HEADER_LEN + len..vshm::HEADER_LEN + 2 * len]
}

/// SAFETY: `as_slice` is the crate's read-only view over the mapped
/// region; the test only reads the `out` region after the child's
/// `Rendered` reply serialized the write.
#[expect(unsafe_code)]
fn segment_bytes(mem: &Shmem) -> &[u8] {
    unsafe { mem.as_slice() }
}

/// SAFETY: see `segment_bytes` — additionally the test writes only
/// regions the child is not currently touching (the `in` region
/// before its `VideoRender`, the header before attach), so no
/// concurrent access exists.
#[expect(unsafe_code)]
fn segment_bytes_mut(mem: &mut Shmem) -> &mut [u8] {
    unsafe { mem.as_slice_mut() }
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// Spawns a child, creates its segment and mints instance 0 of
/// `INVERT_ID`, then drives it to `Processing` — the fixture every
/// render/lifecycle test shares.
fn processing_instance(
    host: &mut MiniHost,
    config: &VideoProcessConfig,
    tag: &str,
) -> (u32, Shmem) {
    let key = unique_key(tag);
    let mem = create_video_segment(&key, config);
    host.expect_hello();
    assert_eq!(
        host.create_video_instance(0, INVERT_ID, &key, config),
        Ok(WireReply::Ok)
    );
    for cmd in [
        WireCommand::VideoInitialize,
        WireCommand::VideoActivate,
        WireCommand::VideoStartProcessing,
    ] {
        assert_eq!(
            host.command(0, cmd.clone()),
            Ok(WireReply::Ok),
            "{cmd:?} must succeed"
        );
    }
    (0, mem)
}

/// `VideoRender` on `instance` with `desc` → the reply.
fn render(
    host: &mut MiniHost,
    instance: u32,
    at: MediaTime,
    desc: &wire::VideoFrameDesc,
) -> Result<WireReply, HostError> {
    host.command(
        instance,
        WireCommand::VideoRender {
            at,
            input: desc.clone(),
        },
    )
}

// --- E2E -------------------------------------------------------------

/// Happy path: `Hello` → `Scan` → `CreateVideoInstance` → video
/// lifecycle to `Processing` → `VideoRender` round-trips frame bytes
/// and metadata → `DestroyInstance` → `Shutdown`.
#[test]
fn happy_path_scan_create_render_destroy() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    // `Scan` enumerates every video factory's descriptor.
    let descriptors = expect_scanned(host.scan());
    assert_eq!(descriptors.len(), 4);
    assert!(
        descriptors.iter().any(|d| d.matches(&INVERT_DESCRIPTOR)),
        "invert descriptor scanned"
    );

    let config = test_config();
    let key = unique_key("happy");
    let mut mem = create_video_segment(&key, &config);
    assert_eq!(
        host.create_video_instance(0, INVERT_ID, &key, &config),
        Ok(WireReply::Ok)
    );
    for cmd in [
        WireCommand::VideoInitialize,
        WireCommand::VideoActivate,
        WireCommand::VideoStartProcessing,
    ] {
        assert_eq!(host.command(0, cmd), Ok(WireReply::Ok));
    }

    // Write a deterministic pattern into the `in` region and render.
    let frame_bytes = vshm::frame_bytes(&config);
    for (index, byte) in in_region_mut(&mut mem, frame_bytes).iter_mut().enumerate() {
        *byte = u8::try_from(index % 251).expect("mod 251 fits u8");
    }
    let pts = MediaTime::new(90_000, TimeBase::COMMON_90K);
    let duration = Some(MediaDuration::new(3_000, TimeBase::COMMON_90K));
    let color = ColorInfo {
        primaries: ColorPrimaries::Bt709,
        transfer: TransferCharacteristics::Bt709,
        ..ColorInfo::default()
    };
    let desc = canonical_desc(&config, pts, duration, color);
    match render(&mut host, 0, pts, &desc) {
        Ok(WireReply::Rendered { output }) => {
            // Runner-derived canonical descriptor: config geometry,
            // packed placement, input metadata passed through.
            assert_eq!(output.format, config.format());
            assert_eq!(output.width, 64);
            assert_eq!(output.height, 64);
            assert_eq!(output.planes.len(), 1);
            assert_eq!(output.planes[0].offset, 0);
            assert_eq!(output.planes[0].len, 16_384);
            assert_eq!(output.planes[0].stride, 256);
            assert_eq!(output.pts, pts, "pts passes through");
            assert_eq!(output.duration, duration, "duration passes through");
            assert_eq!(output.color, color, "color passes through");
        }
        other => panic!("expected Rendered, got {other:?}"),
    }
    // The `out` region holds the inverted frame bytes — the pixel
    // oracle the deterministic fixture pins.
    for (index, byte) in out_region(&mem, frame_bytes).iter().enumerate() {
        assert_eq!(
            *byte,
            !u8::try_from(index % 251).expect("mod 251 fits u8"),
            "inverted byte at {index}"
        );
    }

    for cmd in [
        WireCommand::VideoStopProcessing,
        WireCommand::VideoDeactivate,
        WireCommand::DestroyInstance,
    ] {
        assert_eq!(host.command(0, cmd), Ok(WireReply::Ok));
    }
    host.shutdown_ok();
}

/// `argv` rejection — any argument is the defined usage error:
/// stderr diagnostic + exit 2, no `Hello` emitted.
#[test]
fn argv_is_a_usage_rejection() {
    let mut host = MiniHost::spawn(RUNNER, &["--ruviform-plugin-id=x"]);
    // stdout closes without a frame — the parent observes spawn → EOF.
    let status = host.wait_exit();
    assert_eq!(status.code(), Some(2), "argv rejection exits 2");
}

/// `CreateVideoInstance` with an unscanned plugin id →
/// `Err(UnknownPluginId)`, child alive.
#[test]
fn unknown_plugin_id_is_a_defined_rejection() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    let config = test_config();
    let key = unique_key("unknown-id");
    let _mem = create_video_segment(&key, &config);
    assert_eq!(
        host.create_video_instance(0, "dev.ruviform.mock.video.missing", &key, &config),
        Err(HostError::UnknownPluginId)
    );
    assert_alive(&mut host);
    host.shutdown_ok();
}

/// A `CreateVideoInstance` for a missing segment key → `Err(Ipc)`,
/// child alive, no half-registered entry (the handle stays
/// unregistered: a lifecycle verb on it is `InvalidHandle`).
#[test]
fn missing_segment_is_ipc_and_leaves_no_entry() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    let config = test_config();
    assert_eq!(
        host.create_video_instance(0, INVERT_ID, &unique_key("absent"), &config),
        Err(HostError::Ipc)
    );
    assert_eq!(
        host.command(0, WireCommand::VideoInitialize),
        Err(HostError::InvalidHandle),
        "a failed create must leave the handle unregistered"
    );
    assert_alive(&mut host);
    host.shutdown_ok();
}

/// The attach-rejection matrix — every structural or config-agreement
/// mismatch is `Err(Ipc)` on the wire with the child alive.
#[test]
fn attach_rejections_are_ipc_and_keep_the_child_alive() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    let config = test_config();
    let frame_bytes = vshm::frame_bytes(&config);
    let fb32 = u32::try_from(frame_bytes).expect("frame bytes fit u32");
    let segment_size = frame_len(vshm::segment_len(frame_bytes));

    // (tag, key, header, mapped size) — the key rides the tuple so the
    // lookup can never drift from the created segment's name.
    let mut cases: Vec<(String, String, [u8; vshm::HEADER_LEN], usize)> = Vec::new();
    // magic mismatch.
    let mut header = vshm::header_bytes(fb32, fb32);
    header[0..4].copy_from_slice(b"BAD!");
    cases.push((
        "magic".to_owned(),
        unique_key("att-magic"),
        header,
        segment_size,
    ));
    // wire_version mismatch.
    let mut header = vshm::header_bytes(fb32, fb32);
    put_u32(&mut header, vshm::OFF_WIRE_VERSION, 2);
    cases.push((
        "version".to_owned(),
        unique_key("att-version"),
        header,
        segment_size,
    ));
    // in_off != HEADER_LEN.
    let mut header = vshm::header_bytes(fb32, fb32);
    put_u32(&mut header, vshm::OFF_IN_OFF, 0);
    cases.push((
        "in_off".to_owned(),
        unique_key("att-in-off"),
        header,
        segment_size,
    ));
    // out_off != HEADER_LEN + in_bytes.
    let mut header = vshm::header_bytes(fb32, fb32);
    put_u32(
        &mut header,
        vshm::OFF_OUT_OFF,
        u32::try_from(vshm::HEADER_LEN).expect("header len fits u32") + fb32 + 4,
    );
    cases.push((
        "out_off".to_owned(),
        unique_key("att-out-off"),
        header,
        segment_size,
    ));
    // reserved != 0.
    let mut header = vshm::header_bytes(fb32, fb32);
    put_u32(&mut header, vshm::OFF_RESERVED, 1);
    cases.push((
        "reserved".to_owned(),
        unique_key("att-reserved"),
        header,
        segment_size,
    ));
    // mapped len < declared out_off + out_bytes — the OS rounds the
    // mapping up to the allocation granularity, so the violation is
    // the declared layout (out_bytes inflated past any rounding of a
    // valid-size segment), not the requested size.
    cases.push((
        "len".to_owned(),
        unique_key("att-len"),
        vshm::header_bytes(fb32, u32::MAX),
        segment_size,
    ));
    // in_bytes != frame_bytes(config) — structurally consistent, so
    // only the config-agreement layer fires.
    let smaller = fb32 / 2;
    cases.push((
        "in_bytes".to_owned(),
        unique_key("att-in-bytes"),
        vshm::header_bytes(smaller, smaller),
        vshm::HEADER_LEN + 2 * smaller as usize,
    ));

    let mut segments = Vec::new();
    for (index, (tag, key, header, size)) in cases.into_iter().enumerate() {
        segments.push(create_video_segment_raw(&key, size, &header));
        let instance = u32::try_from(index).expect("few cases");
        assert_eq!(
            host.create_video_instance(instance, INVERT_ID, &key, &config),
            Err(HostError::Ipc),
            "{tag} mismatch must be Err(Ipc)"
        );
    }
    drop(segments);

    // `frame_bytes(config) > WIRE_VIDEO_MAX_FRAME_BYTES` — the
    // config-derived bound the child re-checks: the segment is small
    // and valid, the oversized config is the violation.
    let oversized = VideoProcessConfig::new(
        PixelFormat::RgbaF32,
        8_192,
        8_192,
        FrameRate::new(30, 1).unwrap(),
    )
    .expect("valid config");
    assert!(vshm::frame_bytes(&oversized) > wire::WIRE_VIDEO_MAX_FRAME_BYTES);
    let cap_key = unique_key("att-cap");
    let _cap_mem = create_video_segment(&cap_key, &config);
    assert_eq!(
        host.create_video_instance(10, INVERT_ID, &cap_key, &oversized),
        Err(HostError::Ipc),
        "the frame-size bound must be re-checked child-side"
    );

    assert_alive(&mut host);
    host.shutdown_ok();
}

/// `CreateVideoInstance` on an already-registered handle →
/// `Err(Ipc)` — minting a live handle twice is a host contract
/// violation; the existing entry keeps working.
#[test]
fn duplicate_handle_is_ipc() {
    let mut host = MiniHost::spawn_runner();
    let config = test_config();
    let (instance, _mem) = processing_instance(&mut host, &config, "dup-live");
    let key = unique_key("dup-second");
    let _second = create_video_segment(&key, &config);
    assert_eq!(
        host.create_video_instance(instance, INVERT_ID, &key, &config),
        Err(HostError::Ipc),
        "a duplicate live handle is Err(Ipc)"
    );
    // The original entry is untouched — still dispatches.
    assert_eq!(
        host.command(instance, WireCommand::VideoGetParams),
        Ok(WireReply::Params {
            ids: vec![0, 1, 2, 3, 4],
        })
    );
    assert_alive(&mut host);
    host.shutdown_ok();
}

/// Instance commands on unregistered handles →
/// `Err(InvalidHandle)` — including `DestroyInstance` and the
/// cross-kind `CreateInstance` on a video runner.
#[test]
fn unregistered_handles_are_invalid_handle() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    for cmd in [
        WireCommand::VideoInitialize,
        WireCommand::VideoGetParams,
        WireCommand::DestroyInstance,
    ] {
        assert_eq!(
            host.command(7, cmd.clone()),
            Err(HostError::InvalidHandle),
            "{cmd:?} on an unregistered handle"
        );
    }
    assert_alive(&mut host);
    host.shutdown_ok();
}

/// Cross-kind rejection: the audio `CreateInstance` on a video
/// runner falls through to `Err(InvalidHandle)` unregistered /
/// `Err(Unsupported)` registered — and `CreateVideoInstance` on the
/// *audio* runner is `Err(InvalidHandle)` for free, no new arms.
#[test]
fn cross_kind_create_is_a_defined_rejection() {
    let mut host = MiniHost::spawn_runner();
    let config = test_config();
    let (instance, _mem) = processing_instance(&mut host, &config, "cross-live");

    // The audio create on the video runner — the envelope's handle
    // lands in the registry lookup like any verb.
    assert_eq!(
        host.command(
            40,
            WireCommand::CreateInstance {
                plugin_id: "dev.ruviform.mock.gain".to_owned(),
                shm_key: "unused".to_owned(),
                in_depth: 1,
                out_depth: 1,
            },
        ),
        Err(HostError::InvalidHandle),
        "CreateInstance on the video runner is InvalidHandle unregistered"
    );
    // On a registered handle the entry catch-all answers
    // `Unsupported` — an audio verb is not a registry verb.
    assert_eq!(
        host.command(
            instance,
            WireCommand::CreateInstance {
                plugin_id: "dev.ruviform.mock.gain".to_owned(),
                shm_key: "unused".to_owned(),
                in_depth: 1,
                out_depth: 1,
            },
        ),
        Err(HostError::Unsupported),
        "CreateInstance on a live video entry is Unsupported"
    );
    // An audio lifecycle verb on the same entry — same catch-all.
    assert_eq!(
        host.command(instance, WireCommand::Initialize),
        Err(HostError::Unsupported)
    );
    assert_alive(&mut host);
    host.shutdown_ok();

    // The mirror direction: `CreateVideoInstance` on the *audio*
    // runner → `Err(InvalidHandle)` — the same registry-dispatch
    // shape needs no new arms there either.
    let mut audio = MiniHost::spawn(AUDIO_RUNNER, &[]);
    audio.expect_hello();
    assert_eq!(
        audio.command(
            40,
            WireCommand::CreateVideoInstance {
                plugin_id: INVERT_ID.to_owned(),
                shm_key: "unused".to_owned(),
                config,
            },
        ),
        Err(HostError::InvalidHandle),
        "CreateVideoInstance on the audio runner is InvalidHandle"
    );
    audio.shutdown_ok();
}

/// `WireMaxInstances` is the registry bound — `Err(InstanceLimit)`
/// when the registry is full. Every legal wire handle `0..64` is
/// then already registered (a fresh handle cannot even decode), so
/// `InstanceLimit` is reached by re-minting a *live* handle: the
/// capacity check precedes the duplicate check precisely to keep
/// `InstanceLimit` — not `Ipc` — the answer there.
#[test]
fn instance_limit_is_a_defined_rejection() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    // Depth-1 equivalents: 1×1 RGBA8 — 4-byte frames keep 64
    // instances cheap.
    let config = VideoProcessConfig::new(PixelFormat::Rgba8, 1, 1, FrameRate::new(30, 1).unwrap())
        .expect("valid config");
    let mut segments = Vec::new();
    for instance in 0..wire::WIRE_MAX_INSTANCES {
        let key = unique_key(&format!("limit-{instance}"));
        segments.push(create_video_segment(&key, &config));
        assert_eq!(
            host.create_video_instance(instance, INVERT_ID, &key, &config),
            Ok(WireReply::Ok),
            "instance {instance} must mint"
        );
    }
    let key = unique_key("limit-overflow");
    let _overflow = create_video_segment(&key, &config);
    // Capacity precedes the duplicate check — reusing a live handle
    // on the full registry reports `InstanceLimit`, not `Ipc`.
    assert_eq!(
        host.create_video_instance(0, INVERT_ID, &key, &config),
        Err(HostError::InstanceLimit),
        "a full registry is InstanceLimit"
    );
    drop(segments);
    assert_alive(&mut host);
    host.shutdown_ok();
}

/// Lifecycle mirrors: illegal transitions are `Err(Lifecycle)` and
/// legal ones run the hooks — `VideoInitialize` replays the
/// `CreateVideoInstance`-stashed config (observed through the
/// `EchoWidth` parameter).
#[test]
fn lifecycle_transitions_and_stash_replay() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    let config = test_config();
    let key = unique_key("lifecycle");
    let _mem = create_video_segment(&key, &config);
    assert_eq!(
        host.create_video_instance(0, INVERT_ID, &key, &config),
        Ok(WireReply::Ok)
    );

    // Illegal-order rejections are `Err(Lifecycle)`, not panics.
    assert_eq!(
        host.command(0, WireCommand::VideoActivate),
        Err(HostError::Lifecycle(
            ruviform_plugin_core::LifecycleError::IllegalTransition {
                from: ruviform_plugin_core::LifecycleState::Created,
                event: ruviform_plugin_core::LifecycleEvent::Activate,
            }
        )),
        "Activate from Created"
    );
    assert_eq!(
        host.command(
            0,
            WireCommand::VideoRender {
                at: MediaTime::ZERO,
                input: canonical_desc(&config, MediaTime::ZERO, None, ColorInfo::default()),
            }
        ),
        Err(HostError::Lifecycle(
            ruviform_plugin_core::LifecycleError::IllegalTransition {
                from: ruviform_plugin_core::LifecycleState::Created,
                event: ruviform_plugin_core::LifecycleEvent::Process,
            }
        )),
        "Render before Processing"
    );

    assert_eq!(
        host.command(0, WireCommand::VideoInitialize),
        Ok(WireReply::Ok)
    );
    // The stash replay: `EchoWidth` reads back the config width the
    // hook received — 0 would mean `initialize` never saw it.
    assert_eq!(
        host.command(0, WireCommand::VideoGetParamValue { id: 2 }),
        Ok(WireReply::ParamValue {
            value: PlainParamValue::Integer(64),
        }),
        "the stashed config must reach initialize"
    );
    // `VideoInitialize` twice is illegal from `Initialized`.
    assert_eq!(
        host.command(0, WireCommand::VideoInitialize),
        Err(HostError::Lifecycle(
            ruviform_plugin_core::LifecycleError::IllegalTransition {
                from: ruviform_plugin_core::LifecycleState::Initialized,
                event: ruviform_plugin_core::LifecycleEvent::Initialize,
            }
        ))
    );
    // Render from `Activated` (before `StartProcessing`) is illegal.
    assert_eq!(
        host.command(0, WireCommand::VideoActivate),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(
            0,
            WireCommand::VideoRender {
                at: MediaTime::ZERO,
                input: canonical_desc(&config, MediaTime::ZERO, None, ColorInfo::default()),
            }
        ),
        Err(HostError::Lifecycle(
            ruviform_plugin_core::LifecycleError::IllegalTransition {
                from: ruviform_plugin_core::LifecycleState::Activated,
                event: ruviform_plugin_core::LifecycleEvent::Process,
            }
        ))
    );
    host.shutdown_ok();
}

/// `VideoDeactivate` hook error → the reply is the plugin error but
/// the mirror committed back to `Initialized` first
/// (commit-then-advisory) — `VideoActivate` is legal again.
#[test]
fn deactivate_error_is_commit_then_advisory() {
    let mut host = MiniHost::spawn_runner();
    let config = test_config();
    let (instance, _mem) = processing_instance(&mut host, &config, "deactivate");
    assert_eq!(
        host.command(
            instance,
            WireCommand::VideoSetParamValue {
                id: 3,
                value: PlainParamValue::Boolean(true),
            },
        ),
        Ok(WireReply::Ok),
        "arm FailDeactivate"
    );
    assert_eq!(
        host.command(instance, WireCommand::VideoStopProcessing),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::VideoDeactivate),
        Err(HostError::PluginFailed),
        "the hook error is the reply"
    );
    // Commit-then-advisory: the mirror is `Initialized`, so
    // `VideoActivate` is legal again — not `Activated`.
    assert_eq!(
        host.command(instance, WireCommand::VideoActivate),
        Ok(WireReply::Ok),
        "mirror committed Initialized despite the Err reply"
    );
    host.shutdown_ok();
}

/// Parameter dispatch: `id ∈ parameters()` is checked before any
/// hook — `VideoGetParamInfo` answers `ParamInfo{None}` for an
/// unknown id, the value verbs answer `Err(InvalidValue)`; known ids
/// pass through the hooks verbatim.
#[test]
fn parameter_dispatch_prechecks_membership() {
    let mut host = MiniHost::spawn_runner();
    let config = test_config();
    let (instance, _mem) = processing_instance(&mut host, &config, "params");

    assert_eq!(
        host.command(instance, WireCommand::VideoGetParams),
        Ok(WireReply::Params {
            ids: vec![0, 1, 2, 3, 4],
        })
    );
    match host.command(instance, WireCommand::VideoGetParamInfo { id: 0 }) {
        Ok(WireReply::ParamInfo { info: Some(info) }) => {
            assert_eq!(info.name, "Amount");
            assert_eq!(info.min_value, 0.0);
            assert_eq!(info.max_value, 1.0);
        }
        other => panic!("expected ParamInfo(Some), got {other:?}"),
    }
    // Unknown id → the plugin-declared `None` — the hook is never
    // invoked (`parameter(9)` on the fixture would return `None`
    // anyway, but the runner must not even ask).
    assert_eq!(
        host.command(instance, WireCommand::VideoGetParamInfo { id: 9 }),
        Ok(WireReply::ParamInfo { info: None })
    );
    assert_eq!(
        host.command(instance, WireCommand::VideoGetParamValue { id: 9 }),
        Err(HostError::InvalidValue),
        "unknown id is InvalidValue for reads"
    );
    assert_eq!(
        host.command(
            instance,
            WireCommand::VideoSetParamValue {
                id: 9,
                value: PlainParamValue::Double(0.5),
            },
        ),
        Err(HostError::InvalidValue),
        "unknown id is InvalidValue for writes"
    );
    // Known id round-trips through the hook.
    assert_eq!(
        host.command(instance, WireCommand::VideoGetParamValue { id: 0 }),
        Ok(WireReply::ParamValue {
            value: PlainParamValue::Double(1.0),
        })
    );
    assert_eq!(
        host.command(
            instance,
            WireCommand::VideoSetParamValue {
                id: 0,
                value: PlainParamValue::Double(0.25),
            },
        ),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(instance, WireCommand::VideoGetParamValue { id: 0 }),
        Ok(WireReply::ParamValue {
            value: PlainParamValue::Double(0.25),
        })
    );
    host.shutdown_ok();
}

/// `VideoRender`'s post-decode semantic checks: a non-canonical
/// descriptor is `Err(Ipc)`, a config dims/format mismatch is
/// `Err(Prepare(UnsupportedConfig))` — both `Err` replies, child
/// alive, mirror untouched.
#[test]
fn render_descriptor_semantics_are_checked() {
    let mut host = MiniHost::spawn_runner();
    let config = test_config();
    let (instance, _mem) = processing_instance(&mut host, &config, "desc-semantics");
    let canonical = canonical_desc(&config, MediaTime::ZERO, None, ColorInfo::default());

    // Non-canonical variants — stride ≠ row_bytes, offset ≠ Σlen,
    // len ≠ plane size, offset+len past the region.
    for mutate in [
        (|d: &mut wire::VideoFrameDesc| d.planes[0].stride = 300) as fn(&mut wire::VideoFrameDesc),
        (|d: &mut wire::VideoFrameDesc| d.planes[0].offset = 4) as fn(&mut wire::VideoFrameDesc),
        (|d: &mut wire::VideoFrameDesc| d.planes[0].len = 100) as fn(&mut wire::VideoFrameDesc),
        (|d: &mut wire::VideoFrameDesc| d.planes[0].offset = 16_384)
            as fn(&mut wire::VideoFrameDesc),
    ] {
        let mut desc = canonical.clone();
        mutate(&mut desc);
        assert_eq!(
            render(&mut host, instance, MediaTime::ZERO, &desc),
            Err(HostError::Ipc),
            "{desc:?} must be the Ipc rejection"
        );
    }

    // Config mismatch — canonical under its own declared dims (and
    // still inside the 16 KiB region, so the config check is the one
    // that fires), but not the instance config's.
    for (width, height, format) in [(32, 64, PixelFormat::Rgba8), (64, 64, PixelFormat::Yuv420p)] {
        let mismatched =
            VideoProcessConfig::new(format, width, height, FrameRate::new(30, 1).unwrap())
                .expect("valid config");
        let desc = canonical_desc(&mismatched, MediaTime::ZERO, None, ColorInfo::default());
        assert_eq!(
            render(&mut host, instance, MediaTime::ZERO, &desc),
            Err(HostError::Prepare(PrepareError::UnsupportedConfig)),
            "{width}×{height} {format:?} must be the config-mismatch rejection"
        );
    }

    // The instance still renders after every defined rejection.
    assert_eq!(
        render(&mut host, instance, MediaTime::ZERO, &canonical),
        Ok(WireReply::Rendered {
            output: wire::VideoFrameDesc {
                format: PixelFormat::Rgba8,
                width: 64,
                height: 64,
                planes: vec![wire::VideoFramePlane {
                    offset: 0,
                    len: 16_384,
                    stride: 256,
                }],
                pts: MediaTime::ZERO,
                duration: None,
                color: ColorInfo::default(),
            },
        }),
        "a valid render still succeeds"
    );
    host.shutdown_ok();
}

/// Render hook failure and panic: the reply is the plugin error
/// class, the `out` region keeps its previous content (nothing
/// committed), and the child stays alive — the instance mirror
/// stays `Processing` (a defined `Err` reply never folds to
/// `Failed`).
#[test]
fn render_failures_leave_output_uncommitted_and_the_child_alive() {
    // Defined hook error first.
    let mut host = MiniHost::spawn_runner();
    let config = test_config();
    let (instance, mut mem) = processing_instance(&mut host, &config, "render-err");
    let frame_bytes = vshm::frame_bytes(&config);
    in_region_mut(&mut mem, frame_bytes).fill(0xAB);
    let canonical = canonical_desc(&config, MediaTime::ZERO, None, ColorInfo::default());
    assert_eq!(
        host.command(
            instance,
            WireCommand::VideoSetParamValue {
                id: 4,
                value: PlainParamValue::Boolean(true),
            },
        ),
        Ok(WireReply::Ok),
        "arm FailRender"
    );
    assert_eq!(
        render(&mut host, instance, MediaTime::ZERO, &canonical),
        Err(HostError::PluginFailed)
    );
    assert!(
        out_region(&mem, frame_bytes).iter().all(|byte| *byte == 0),
        "a failed render must not commit the out region"
    );
    // Still `Processing` — a healthy render succeeds afterwards.
    assert_eq!(
        host.command(
            instance,
            WireCommand::VideoSetParamValue {
                id: 4,
                value: PlainParamValue::Boolean(false),
            },
        ),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        render(&mut host, instance, MediaTime::ZERO, &canonical),
        Ok(WireReply::Rendered {
            output: wire::VideoFrameDesc {
                format: PixelFormat::Rgba8,
                width: 64,
                height: 64,
                planes: vec![wire::VideoFramePlane {
                    offset: 0,
                    len: 16_384,
                    stride: 256,
                }],
                pts: MediaTime::ZERO,
                duration: None,
                color: ColorInfo::default(),
            },
        })
    );
    host.shutdown_ok();

    // Panic — same normalization class via `RUVIFORM_PANIC_RENDER`.
    let mut host = MiniHost::spawn_panicking(&[("RUVIFORM_PANIC_RENDER", "1")]);
    let (instance, mut mem) = processing_instance(&mut host, &config, "render-panic");
    in_region_mut(&mut mem, frame_bytes).fill(0xCD);
    assert_eq!(
        render(&mut host, instance, MediaTime::ZERO, &canonical),
        Err(HostError::PluginFailed),
        "a hook panic normalizes to PluginFailed"
    );
    assert!(
        out_region(&mem, frame_bytes).iter().all(|byte| *byte == 0),
        "a panicking render must not commit the out region"
    );
    assert_alive(&mut host);
    host.shutdown_ok();
}

/// `PanicVideo`'s `initialize` panic → `Err(PluginFailed)` on the
/// wire, mirror reports `InitFailed` (back at `Created`), and the
/// child keeps serving — including another instance.
#[test]
fn panic_initialize_is_plugin_failed_and_child_survives() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    let config = test_config();
    let key = unique_key("panic-init");
    let _mem = create_video_segment(&key, &config);
    assert_eq!(
        host.create_video_instance(0, PANIC_ID, &key, &config),
        Ok(WireReply::Ok),
        "create does not touch the panicking hook"
    );
    assert_eq!(
        host.command(0, WireCommand::VideoInitialize),
        Err(HostError::PluginFailed)
    );
    // `InitFailed` reported — the mirror is back at `Created`, so a
    // retry reaches the hook again (same panic, same reply).
    assert_eq!(
        host.command(0, WireCommand::VideoInitialize),
        Err(HostError::PluginFailed)
    );
    assert_alive(&mut host);
    host.shutdown_ok();
}

/// Guarded factory calls: `descriptor()` and `create()` panics are
/// `Err(PluginFailed)` on the wire — `Scan` and
/// `CreateVideoInstance` alike, with no half-registered entry.
#[test]
fn factory_panics_are_plugin_failed() {
    let config = test_config();
    for (env, check) in [
        ("RUVIFORM_PANIC_DESCRIPTOR", "scan"),
        ("RUVIFORM_PANIC_CREATE", "create"),
    ] {
        let mut host = MiniHost::spawn_panicking(&[(env, "1")]);
        host.expect_hello();
        if check == "scan" {
            assert_eq!(host.scan(), Err(HostError::PluginFailed));
        } else {
            let key = unique_key("panic-create");
            let _mem = create_video_segment(&key, &config);
            assert_eq!(
                host.create_video_instance(0, PANIC_ID, &key, &config),
                Err(HostError::PluginFailed)
            );
            assert_eq!(
                host.command(0, WireCommand::VideoInitialize),
                Err(HostError::InvalidHandle),
                "a panicking create leaves no registry entry"
            );
        }
        host.shutdown_ok();
    }
}

/// Encode obligation (c): over-bound reports are `Err(PluginFailed)`
/// — never truncated, never a dead child. `parameters()` over the
/// element bound fails the membership pre-check too; `parameter()`
/// over the choice/string bounds fails `VideoGetParamInfo`.
/// Obligation (d): non-finite `display_*` hints arrive clamped into
/// `[min,max]` through `VideoGetParamInfo`.
#[test]
fn over_limit_and_non_finite_reports_are_normalized() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    let config = test_config();

    // `parameters()` > WIRE_MAX_ELEMENTS.
    let key = unique_key("over-ids");
    let _mem = create_video_segment(&key, &config);
    assert_eq!(
        host.create_video_instance(0, OVERLIMIT_ID, &key, &config),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(0, WireCommand::VideoGetParams),
        Err(HostError::PluginFailed),
        "an over-bound id list is PluginFailed, not truncated"
    );
    assert_eq!(
        host.command(0, WireCommand::VideoGetParamValue { id: 0 }),
        Err(HostError::PluginFailed),
        "the membership pre-check hits the same bound"
    );

    // `parameter()` over the choices/string bounds; `display_*` clamp.
    let key = unique_key("over-info");
    let _mem = create_video_segment(&key, &config);
    assert_eq!(
        host.create_video_instance(1, OVERLIMIT_INFO_ID, &key, &config),
        Ok(WireReply::Ok)
    );
    assert_eq!(
        host.command(1, WireCommand::VideoGetParamInfo { id: 0 }),
        Err(HostError::PluginFailed),
        "over-bound choices are PluginFailed"
    );
    assert_eq!(
        host.command(1, WireCommand::VideoGetParamInfo { id: 1 }),
        Err(HostError::PluginFailed),
        "an over-bound name is PluginFailed"
    );
    match host.command(1, WireCommand::VideoGetParamInfo { id: 2 }) {
        Ok(WireReply::ParamInfo { info: Some(info) }) => {
            assert_eq!(info.display_min_value, 0.0, "-inf folds to min");
            assert_eq!(info.display_max_value, 1.0, "+inf folds to max");
        }
        other => panic!("expected ParamInfo(Some), got {other:?}"),
    }
    assert_alive(&mut host);
    host.shutdown_ok();
}

/// An inbound `BlockIn` is direction-legal but unattributable on a
/// video child — dropped silently, no reply, the child keeps
/// answering commands.
#[test]
fn inbound_block_in_is_dropped_silently() {
    let mut host = MiniHost::spawn_runner();
    host.expect_hello();
    host.send(&Message::BlockIn {
        seq: 1 << 63,
        instance: 0,
    });
    // The next inbound frame must be the *Scan* reply — nothing
    // answered the doorbell.
    let descriptors = expect_scanned(host.scan());
    assert_eq!(descriptors.len(), 4);
    host.shutdown_ok();
}

/// `BlockDone`/`Reply`/`Hello` inbound stay direction-illegal:
/// protocol violation → normalize + exit(1), mirroring the audio
/// `serve` contract.
#[test]
fn direction_illegal_inbound_messages_kill_the_child() {
    for (index, message) in [
        Message::BlockDone {
            seq: 1,
            instance: 0,
            result: ProcessResult::CONTINUE,
        },
        Message::Reply {
            seq: 0,
            instance: wire::WIRE_INSTANCE_NONE,
            result: Ok(WireReply::Ok),
        },
        Message::Hello {
            version: wire::WIRE_VERSION,
            capabilities: 0,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let mut host = MiniHost::spawn_runner();
        host.expect_hello();
        host.send(&message);
        let status = host.wait_exit();
        assert!(
            !status.success(),
            "inbound {message:?} (case {index}) must exit with failure"
        );
    }
}

/// `Shutdown` and clean stdin EOF normalize every entry and exit 0.
#[test]
fn shutdown_and_eof_exit_cleanly() {
    // `Shutdown` mid-lifecycle (a live `Processing` instance is
    // normalized best-effort — the process still exits 0).
    let mut host = MiniHost::spawn_runner();
    let config = test_config();
    let (_instance, _mem) = processing_instance(&mut host, &config, "shutdown-live");
    host.shutdown_ok();

    // Clean EOF — dropping stdin ends the loop identically.
    let mut host = MiniHost::spawn_runner();
    let (_instance, _mem) = processing_instance(&mut host, &config, "eof-live");
    drop(host.stdin.take());
    let status = host.wait_exit();
    assert!(status.success(), "clean EOF exits 0");
}
