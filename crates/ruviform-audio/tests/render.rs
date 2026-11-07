//! Issue #198 sequential offline render — OF-01..OF-06 acceptance
//! evidence. OF-07 (command evidence) is recorded in the task doc.
//!
//! Test doubles: `CaptureEncoder` records emitted `AudioFrame`s
//! instead of writing a container, `NullMuxer` accepts any packet,
//! `FailSink` injects write/flush failures inside the muxer's sink.

mod common;

use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use common::{Chain, MockDsp, MockFactory, chain, config, validate};
use ruviform_audio::render::{
    FeedContext, InputFeeder, OutputFacts, PreflightError, PreparedRender, RenderError,
    RenderOutput, RenderReceipt, RenderSpec, RenderTransport, SilentFeeder, StagedCommand,
    TailEvidence, TailSpec, render,
};
use ruviform_audio::typed::{
    Compiler, GraphProcessor, GraphRuntime, NodeEvent, NodeIo, OfflineRenderSupport, Slot,
};
use ruviform_audio_core::{ChannelLayout, SampleFormat, SampleRate};
use ruviform_exec::{CancellationToken, Progress};
use ruviform_graph::PortId;
use ruviform_io_file::{
    Durability, DurabilityEvidence, PublishEffect, PublishFailure, PublishMode, PublishOptions,
    PublishReceipt, SealedFile, StagedFile, TempState,
};
use ruviform_io_wav::WavOutputSpec;
use ruviform_media_core::{
    BeatPosition, BlockContract, DeliveryPolicy, EngineFrame, InBlockTransportSupport,
    MediaDuration, MediaRange, MediaTime, PlanGeneration, ScheduleBudget, ScheduledEvent,
    TempoEvent, TempoMapBuilder, TempoMapRevision, TimeBase, TransportCommand, TransportPlan,
    TransportPlanBuilder, TransportScheduler,
};
use ruviform_media_io::{
    CodecFormat, Decoder, Demuxer, Encoder, Error as MediaError, ErrorKind, Frame, Muxer, Packet,
    StreamInfo,
};
use ruviform_rt::RtContext;

/// The no-overwrite publish contract: refuse an existing destination,
/// require atomic visibility, and require file + directory durability.
const NO_OVERWRITE: PublishOptions =
    PublishOptions::new(PublishMode::CreateNew, Durability::FileAndDirectorySync);

const RATE: u32 = 48_000;

fn rate() -> SampleRate {
    SampleRate::new(RATE).unwrap()
}

fn base() -> TimeBase {
    rate().time_base()
}

fn mt(ticks: i64) -> MediaTime {
    MediaTime::new(ticks, base())
}

/// Deterministic boundary-input signal: channel `c`, position `p` →
/// a bounded repeating pattern (values in (-0.75, 0.75)).
#[allow(clippy::cast_precision_loss)]
fn signal(channels: usize, len: usize) -> Vec<Vec<f32>> {
    (0..channels)
        .map(|c| {
            (0..len)
                .map(|p| (((p * 7 + c * 31) % 97) as f32 / 97.0) * 1.5 - 0.75)
                .collect()
        })
        .collect()
}

/// Block-partition-invariant stateful DSP: `y[n] = 0.5·y[n-1] + x[n]`
/// per channel — output depends on full input history yet is identical
/// for any legal block split. `latency` is *declared* to the plan
/// (the PDC offset the driver's `position - latency` emit math
/// implements) without itself shifting produced samples.
struct OnePole {
    latency: u32,
    support: OfflineRenderSupport,
    state: [f32; 8],
}

impl OnePole {
    fn new(latency: u32) -> Self {
        Self {
            latency,
            support: OfflineRenderSupport::Verified,
            state: [0.0; 8],
        }
    }
}

impl GraphProcessor for OnePole {
    fn latency(&self) -> u32 {
        self.latency
    }

    fn offline_support(&self) -> OfflineRenderSupport {
        self.support
    }

    fn process(&mut self, io: &mut NodeIo<'_>, _ctx: &RtContext) {
        let input = io.input(0);
        for o in 0..io.outputs() {
            let Some(mut out) = io.output(o) else {
                continue;
            };
            for ch in 0..out.channels().min(8) {
                let Some(plane) = out.channel_mut(ch) else {
                    continue;
                };
                let src = input.as_ref().and_then(|v| v.channel(ch));
                let state = &mut self.state[ch];
                for (i, d) in plane.iter_mut().enumerate() {
                    *state = *state * 0.5 + src.map_or(0.0, |s| s[i]);
                    *d = *state;
                }
            }
        }
    }
}

/// Feeds `signal[c][position + j]` into the boundary input slot.
struct SignalFeeder {
    data: Vec<Vec<f32>>,
}

impl InputFeeder for SignalFeeder {
    fn feed(&mut self, _port: PortId, ctx: &FeedContext, slot: &mut Slot) {
        let pos = usize::try_from(ctx.position_ticks.max(0)).unwrap_or(0);
        for ch in 0..usize::from(slot.channels) {
            let Some(plane) = slot.data.channel_mut(ch) else {
                continue;
            };
            let src = &self.data[ch.min(self.data.len() - 1)];
            for (j, d) in plane[..ctx.frames].iter_mut().enumerate() {
                *d = src.get(pos + j).copied().unwrap_or(0.0);
            }
        }
    }
}

/// Captured emission evidence: appended channel planes and the pts of
/// each emitted run.
#[derive(Default)]
struct Captured {
    planes: Vec<Vec<f32>>,
    runs: Vec<(i64, usize)>,
}

/// Encoder that copies `AudioFrame` planes into `out` and yields no
/// packets — the semantic-path sink for OF-01..OF-04.
struct CaptureEncoder {
    out: Arc<Mutex<Captured>>,
}

impl Encoder for CaptureEncoder {
    fn send_frame(&mut self, frame: Frame) -> ruviform_media_io::Result<()> {
        if let Frame::Audio(audio) = frame {
            let mut out = self.out.lock().unwrap();
            let buffer = audio.buffer();
            if out.planes.is_empty() {
                out.planes = vec![Vec::new(); buffer.channels()];
            }
            out.runs.push((audio.pts().time.value(), buffer.frames()));
            for (c, plane) in out.planes.iter_mut().enumerate() {
                plane.extend_from_slice(buffer.channel(c).unwrap());
            }
        }
        Ok(())
    }

    fn receive_packet(&mut self) -> ruviform_media_io::Result<Option<Packet>> {
        Ok(None)
    }
}

/// Muxer that accepts every packet and finishes cleanly.
struct NullMuxer {
    streams: Vec<StreamInfo>,
}

impl Muxer for NullMuxer {
    fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }

    fn write_packet(&mut self, _packet: Packet) -> ruviform_media_io::Result<()> {
        Ok(())
    }

    fn finish(&mut self) -> ruviform_media_io::Result<()> {
        Ok(())
    }
}

/// `null_muxer` as a plain `fn` — a function item satisfies the
/// driver's `for<'a> FnOnce(&'a mut StagedFile) -> Result<Box<dyn
/// Muxer + 'a>, _>` bound at any call site without closure inference.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the driver bound requires a fallible constructor"
)]
fn null_muxer(_staged: &mut StagedFile) -> Result<Box<dyn Muxer + '_>, MediaError> {
    Ok(Box::new(NullMuxer {
        streams: Vec::new(),
    }))
}

/// Sink wrapper that fails `write` once `budget` bytes have been
/// accepted, and/or `flush` unconditionally — deterministic
/// `Output`/`Finalize` injection. It borrows the staged file, so
/// bytes accepted before the failure still land in the staged temp.
struct FailSink<'a> {
    inner: &'a mut StagedFile,
    written: u64,
    budget: u64,
    fail_flush: bool,
}

impl Write for FailSink<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.written + buf.len() as u64 > self.budget {
            return Err(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "injected disk full",
            ));
        }
        let n = self.inner.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.fail_flush {
            return Err(std::io::Error::other("injected flush failure"));
        }
        self.inner.flush()
    }
}

impl Seek for FailSink<'_> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

fn prepared(c: &Chain, dsp: impl GraphProcessor + 'static) -> PreparedRender {
    let g = validate(&c.spec);
    let mut factory = MockFactory::default();
    factory.pending.insert(c.proc, Box::new(dsp));
    PreparedRender::compile(&g, &mut factory, config(768), 8, 64).expect("compile")
}

fn transport<P>(max_frames: u32) -> RenderTransport<P> {
    RenderTransport {
        scheduler: TransportScheduler::new(
            base(),
            BlockContract::new(1, max_frames, InBlockTransportSupport::Unknown),
            DeliveryPolicy::Exact,
            ScheduleBudget::default(),
        ),
        commands: Vec::new(),
        plans: Vec::new(),
    }
}

/// Stages `Play` at `frame` (position anchor starts at 0).
fn play_at<P>(t: &mut RenderTransport<P>, frame: u64) {
    let epoch = t.scheduler.epoch();
    t.commands.push(StagedCommand {
        command: TransportCommand::Play,
        due: EngineFrame::new(epoch, frame),
    });
}

fn spec(c: &Chain, range: (u64, u64), tail: TailSpec, job: (u64, u64), chunk: u32) -> RenderSpec {
    RenderSpec {
        sample_rate: rate(),
        layout: ChannelLayout::Stereo,
        output_port: c.sink_in,
        range_start: range.0,
        range_end: range.1,
        tail,
        job_start: job.0,
        job_end: job.1,
        callback_chunk: chunk,
    }
}

type VerifyFn = fn(&mut SealedFile, &OutputFacts) -> Result<(), MediaError>;
type MuxerCtor = for<'a> fn(&'a mut StagedFile) -> Result<Box<dyn Muxer + 'a>, MediaError>;
type PublishFn = fn(SealedFile, PublishOptions) -> Result<PublishReceipt, PublishFailure>;

/// A render bundle whose output lands in `captured` via the
/// `CaptureEncoder`/`NullMuxer` pair; the staged file still exists on
/// disk (a real `StagedFile` over a real temp file).
fn capture_output(
    dest: &Path,
    captured: Arc<Mutex<Captured>>,
    options: PublishOptions,
) -> RenderOutput<MuxerCtor, VerifyFn, PublishFn> {
    RenderOutput {
        staged: StagedFile::create(dest).expect("staged file"),
        make_muxer: null_muxer,
        encoder: Box::new(CaptureEncoder { out: captured }),
        publish_options: options,
        verify: None,
        publish: SealedFile::publish,
    }
}

fn no_events(_: &ScheduledEvent<u32>, _: u64) -> Option<NodeEvent> {
    None
}

/// Unique scratch dir under the OS temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> (Self, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "rvf-render-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("out.wav");
        (Self(dir), dest)
    }

    /// Leftover `.rvf-tmp-*` files — must be zero after every outcome.
    fn leftover_temps(&self) -> usize {
        std::fs::read_dir(&self.0).map_or(0, |it| {
            it.filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().contains(".rvf-tmp-"))
                .count()
        })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ------------------------------------------------------------------
// OF-01: stateful fixture, continuous vs 64/128/256 block sizes.
// ------------------------------------------------------------------

#[test]
fn of01_block_size_invariance() {
    let c = chain();
    let len = 768usize;
    let feeder_signal = || SignalFeeder {
        data: signal(2, len),
    };

    let mut references: Vec<Vec<Vec<f32>>> = Vec::new();
    for max in [768u32, 64, 128, 256] {
        let (_scratch, dest) = Scratch::new("of01");
        let mut p = prepared(&c, OnePole::new(0));
        let mut t = transport::<u32>(max);
        play_at(&mut t, 0);
        let sp = spec(&c, (0, len as u64), TailSpec::None, (0, len as u64), 0);
        let captured = Arc::new(Mutex::new(Captured::default()));
        let progress = Progress::new();
        let receipt = render(
            &mut p,
            &mut t,
            &sp,
            &mut feeder_signal(),
            capture_output(&dest, Arc::clone(&captured), NO_OVERWRITE),
            &progress,
            &CancellationToken::new(),
            &mut no_events,
        )
        .expect("render succeeds");
        assert_eq!(receipt.frames_emitted, len as u64);
        assert_eq!(receipt.tail_evidence, TailEvidence::None);
        assert_eq!(progress.completed(), len as u64);
        let planes = captured.lock().unwrap().planes.clone();
        references.push(planes);
    }
    for (i, planes) in references.iter().enumerate().skip(1) {
        assert_eq!(
            planes, &references[0],
            "block variant {i} diverges from the continuous render"
        );
    }
}

// ------------------------------------------------------------------
// OF-02: mid-stream start, preroll, PDC, and known tail select the
// exact emitted frame window.
// ------------------------------------------------------------------

#[test]
fn of02_midstream_preroll_pdc_tail() {
    let c = chain();
    let len = 1_024usize;
    let feeder_signal = || SignalFeeder {
        data: signal(2, len + 256),
    };

    // Reference: full-range latency-0 render — emitted[p] = dsp(p).
    let reference = {
        let (_scratch, dest) = Scratch::new("of02-ref");
        let mut p = prepared(&c, OnePole::new(0));
        let mut t = transport::<u32>(128);
        play_at(&mut t, 0);
        let sp = spec(&c, (0, len as u64), TailSpec::None, (0, len as u64), 0);
        let captured = Arc::new(Mutex::new(Captured::default()));
        render(
            &mut p,
            &mut t,
            &sp,
            &mut feeder_signal(),
            capture_output(&dest, Arc::clone(&captured), NO_OVERWRITE),
            &Progress::new(),
            &CancellationToken::new(),
            &mut no_events,
        )
        .unwrap();
        let captured = Arc::into_inner(captured).unwrap();
        captured.into_inner().unwrap().planes
    };

    // (a) Mid-stream range [200, 600): positions 0..200 are processed
    // as warm-up but never emitted — emitted samples equal the
    // reference's [200, 600) window exactly (preroll preserves state).
    {
        let (_scratch, dest) = Scratch::new("of02-mid");
        let mut p = prepared(&c, OnePole::new(0));
        let mut t = transport::<u32>(128);
        play_at(&mut t, 0);
        let sp = spec(&c, (200, 600), TailSpec::None, (0, 600), 0);
        let captured = Arc::new(Mutex::new(Captured::default()));
        let receipt = render(
            &mut p,
            &mut t,
            &sp,
            &mut feeder_signal(),
            capture_output(&dest, Arc::clone(&captured), NO_OVERWRITE),
            &Progress::new(),
            &CancellationToken::new(),
            &mut no_events,
        )
        .unwrap();
        assert_eq!(receipt.frames_emitted, 400);
        let got = captured.lock().unwrap();
        assert_eq!(got.runs.first().unwrap().0, 200, "first emitted pts");
        for (got_plane, ref_plane) in got.planes.iter().zip(&reference) {
            assert_eq!(*got_plane, ref_plane[200..600].to_vec());
        }
    }

    // (b) PDC + tail: declared latency L = 96 → the first L processed
    // frames are warm-up (positions −L..0 never emitted) and
    // emitted[p] = dsp(p + L); a known 64-frame tail extends the
    // emitted window to range_end + 64.
    {
        const L: u64 = 96;
        let (_scratch, dest) = Scratch::new("of02-pdc");
        let mut p = prepared(&c, OnePole::new(u32::try_from(L).unwrap()));
        let mut t = transport::<u32>(128);
        play_at(&mut t, 0);
        let sp = spec(&c, (0, 512), TailSpec::Known(64), (0, 512 + L + 64), 0);
        let captured = Arc::new(Mutex::new(Captured::default()));
        let receipt = render(
            &mut p,
            &mut t,
            &sp,
            &mut feeder_signal(),
            capture_output(&dest, Arc::clone(&captured), NO_OVERWRITE),
            &Progress::new(),
            &CancellationToken::new(),
            &mut no_events,
        )
        .unwrap();
        assert_eq!(receipt.frames_emitted, 512 + 64);
        assert_eq!(receipt.tail_evidence, TailEvidence::Known);
        assert_eq!(receipt.range_end_with_tail, 512 + 64);
        let got = captured.lock().unwrap();
        assert_eq!(got.runs.first().unwrap().0, 0);
        let l = usize::try_from(L).unwrap();
        for (got_plane, ref_plane) in got.planes.iter().zip(&reference) {
            assert_eq!(
                *got_plane,
                ref_plane[l..l + 512 + 64].to_vec(),
                "PDC-aligned emission"
            );
        }
    }
}

// ------------------------------------------------------------------
// OF-03: plan install (tempo map), loop boundary, and same-frame
// events process in transport order.
// ------------------------------------------------------------------

#[test]
fn of03_events_loop_transport_order() {
    let c = chain();
    let len = 768usize;
    let feeder_signal = || SignalFeeder {
        data: signal(2, len + 512),
    };
    let render_log = Arc::new(Mutex::new(Vec::<String>::new()));

    let mut tempo = TempoMapBuilder::new();
    tempo.add_tempo_event(TempoEvent::at(BeatPosition::ZERO, 120_000));
    let tempo_map = Arc::new(tempo.build().expect("tempo map"));

    let mut builder = TransportPlanBuilder::<f32>::new(base(), PlanGeneration::new(1))
        .with_tempo_map(Arc::clone(&tempo_map), TempoMapRevision::new(1));
    // Same-position events, source_sequence 0 then 1 → sequential.
    builder.event(mt(192), 0, 3.0).unwrap();
    builder.event(mt(192), 1, 7.0).unwrap();
    // Inside the loop region → fires once per loop occurrence.
    builder.event(mt(300), 2, 5.0).unwrap();
    let plan: Arc<TransportPlan<f32>> = Arc::new(builder.build());

    let (_scratch, dest) = Scratch::new("of03");
    let mut p = prepared(
        &c,
        MockDsp::new(0, 1.0, Arc::clone(&render_log)).with_support(OfflineRenderSupport::Verified),
    );
    let mut t = transport::<f32>(128);
    let epoch = t.scheduler.epoch();
    t.plans.push((plan, EngineFrame::new(epoch, 0)));
    play_at(&mut t, 0);
    t.commands.push(StagedCommand {
        command: TransportCommand::SetLoop(MediaRange::new(
            mt(256),
            MediaDuration::new(256, base()),
        )),
        due: EngineFrame::new(epoch, 0),
    });
    let sp = spec(&c, (0, len as u64), TailSpec::None, (0, len as u64), 0);
    let node = c.proc;
    let mut map_event = move |ev: &ScheduledEvent<f32>, generation: u64| {
        Some(NodeEvent {
            generation,
            node,
            opcode: 0,
            value: ev.payload,
        })
    };
    render(
        &mut p,
        &mut t,
        &sp,
        &mut feeder_signal(),
        capture_output(
            &dest,
            Arc::new(Mutex::new(Captured::default())),
            NO_OVERWRITE,
        ),
        &Progress::new(),
        &CancellationToken::new(),
        &mut map_event,
    )
    .unwrap();

    let log = render_log.lock().unwrap();
    let events: Vec<&str> = log
        .iter()
        .filter_map(|l| l.strip_prefix("event:"))
        .collect();
    assert_eq!(
        events,
        ["gen=1:v=3", "gen=1:v=7", "gen=1:v=5", "gen=1:v=5"],
        "same-frame order, then loop-occurrence repeats"
    );
}

// ------------------------------------------------------------------
// OF-04: render instance independence + capability preflight rejects
// before any output is created.
// ------------------------------------------------------------------

#[test]
fn of04_live_independence_and_capability_gate() {
    let c = chain();
    let g = validate(&c.spec);

    // A "live" runtime on the same graph — its own instance + log.
    let live_log = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut live_factory = MockFactory::default();
    live_factory.pending.insert(
        c.proc,
        Box::new(MockDsp::new(0, 1.0, Arc::clone(&live_log))),
    );
    let (mut live_graph, mut live_rt) = GraphRuntime::new(&config(768), 8, 64);
    let compiled = Compiler::new(config(768))
        .compile(
            &g,
            &mut live_factory,
            live_graph.registry(),
            live_graph.allocator(),
            live_graph.generation() + 1,
        )
        .unwrap();
    live_graph.publish(compiled).unwrap();
    let ctx = RtContext::mint();
    let warmup = [vec![0.1f32; 64], vec![0.1f32; 64]];
    let refs: Vec<&[f32]> = warmup.iter().map(Vec::as_slice).collect();
    common::feed(&mut live_rt, c.src_out, &refs);
    live_rt.process(64, &ctx).unwrap();
    let live_len = live_log.lock().unwrap().len();

    // Render against a different instance of the same node.
    let render_log = Arc::new(Mutex::new(Vec::<String>::new()));
    let (_scratch, dest) = Scratch::new("of04");
    let mut prep = prepared(
        &c,
        MockDsp::new(0, 2.0, Arc::clone(&render_log)).with_support(OfflineRenderSupport::Verified),
    );
    let mut tr = transport::<u32>(128);
    play_at(&mut tr, 0);
    let sp = spec(&c, (0, 256), TailSpec::None, (0, 256), 0);
    let mut feeder = SilentFeeder;
    render(
        &mut prep,
        &mut tr,
        &sp,
        &mut feeder,
        capture_output(
            &dest,
            Arc::new(Mutex::new(Captured::default())),
            NO_OVERWRITE,
        ),
        &Progress::new(),
        &CancellationToken::new(),
        &mut map_noop_u32(),
    )
    .unwrap();

    // Live instance untouched: its log is exactly as before the render
    // and it still processes afterwards.
    assert_eq!(live_log.lock().unwrap().len(), live_len);
    live_rt.process(64, &ctx).unwrap();
    assert!(render_log.lock().unwrap()[0].starts_with("activate:state=0"));

    // Capability gate: Unknown and Declared reject at preflight —
    // the destination must not exist and no temp may be left.
    for support in [
        OfflineRenderSupport::Unknown,
        OfflineRenderSupport::Declared,
    ] {
        let (scratch, dest) = Scratch::new("of04-rej");
        let mut prep = prepared(
            &c,
            MockDsp::new(0, 1.0, Arc::new(Mutex::new(Vec::new()))).with_support(support),
        );
        let mut tr = transport::<u32>(128);
        play_at(&mut tr, 0);
        let sp = spec(&c, (0, 256), TailSpec::None, (0, 256), 0);
        let mut feeder = SilentFeeder;
        let err = render(
            &mut prep,
            &mut tr,
            &sp,
            &mut feeder,
            capture_output(
                &dest,
                Arc::new(Mutex::new(Captured::default())),
                NO_OVERWRITE,
            ),
            &Progress::new(),
            &CancellationToken::new(),
            &mut map_noop_u32(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            RenderError::Preflight(PreflightError::OfflineUnsupported { node, support: s })
                if node == c.proc && s == support
        ));
        assert!(!dest.exists(), "no destination on preflight rejection");
        assert_eq!(scratch.leftover_temps(), 0, "staged temp cleaned");
    }
}

fn map_noop_u32() -> impl FnMut(&ScheduledEvent<u32>, u64) -> Option<NodeEvent> {
    |_, _| None
}

// ------------------------------------------------------------------
// OF-05: real WAV output reopens through an independent reader with
// matching frame count, channels, rate, and PCM content.
// ------------------------------------------------------------------

/// Verifies the sealed staged file through `open_readback` and an
/// independent symphonia decode — the readback-verification half of
/// the output contract. The bytes come from the sealed object itself
/// (never a reopened path); they are copied into a `Cursor` because
/// `open_demuxer` requires a `'static` `MediaSource`.
fn verify_wav(
    expected: Vec<Vec<f32>>,
) -> impl Fn(&mut SealedFile, &OutputFacts) -> Result<(), MediaError> {
    move |sealed, facts| {
        let mut bytes = Vec::new();
        {
            let mut readback = sealed
                .open_readback()
                .map_err(|e| MediaError::with_source(ErrorKind::Io, e))?;
            readback
                .read_to_end(&mut bytes)
                .map_err(|e| MediaError::with_source(ErrorKind::Io, e))?;
        }
        let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(bytes))?;
        let stream = {
            let streams = demuxer.streams();
            if streams.len() != 1 {
                return Err(MediaError::from_kind(ErrorKind::InvalidData));
            }
            streams[0].clone()
        };
        let CodecFormat::Audio(af) = &stream.codec.format else {
            return Err(MediaError::from_kind(ErrorKind::InvalidData));
        };
        if af.sample_rate != facts.sample_rate || af.layout != facts.layout {
            return Err(MediaError::from_kind(ErrorKind::InvalidData));
        }
        let mut decoder = demuxer
            .make_decoder(stream.stream.id)
            .map_err(|e| MediaError::with_source(ErrorKind::InvalidData, e))?;
        let mut planes: Vec<Vec<f32>> = Vec::new();
        let mut frames = 0u64;
        let mut take = |frame: &Frame| {
            if let Frame::Audio(audio) = frame {
                let buffer = audio.buffer();
                if planes.is_empty() {
                    planes = vec![Vec::new(); buffer.channels()];
                }
                for (c, plane) in planes.iter_mut().enumerate() {
                    plane.extend_from_slice(buffer.channel(c).unwrap());
                }
                frames += buffer.frames() as u64;
            }
        };
        while let Some(packet) = demuxer.read_packet()? {
            decoder.send_packet(packet)?;
            while let Some(frame) = decoder.receive_frame()? {
                take(&frame);
            }
        }
        decoder.drain()?;
        while let Some(frame) = decoder.receive_frame()? {
            take(&frame);
        }
        if frames != facts.frames_emitted {
            return Err(MediaError::from_kind(ErrorKind::InvalidData));
        }
        // PCM content must match the analytic prediction.
        for (ch, plane) in planes.iter().enumerate() {
            for (i, &v) in plane.iter().enumerate() {
                if (v - expected[ch][i]).abs() > 1e-6 {
                    return Err(MediaError::from_kind(ErrorKind::InvalidData));
                }
            }
        }
        Ok(())
    }
}

#[test]
fn of05_wav_reopen_verify() {
    let c = chain();
    let len = 480usize;
    let input = signal(2, len);
    // Analytic one-pole prediction of the rendered PCM.
    let expected: Vec<Vec<f32>> = input
        .iter()
        .map(|plane| {
            let mut state = 0.0f32;
            plane
                .iter()
                .map(|&x| {
                    state = state * 0.5 + x;
                    state
                })
                .collect()
        })
        .collect();

    let wire = WavOutputSpec {
        sample_rate: rate(),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F32,
    };
    let info = wire.stream_info();
    let info_mux = info.clone();

    let (_scratch, dest) = Scratch::new("of05");
    let mut p = prepared(&c, OnePole::new(0));
    let mut t = transport::<u32>(128);
    play_at(&mut t, 0);
    let sp = spec(&c, (0, len as u64), TailSpec::None, (0, len as u64), 0);
    let mut feeder = SignalFeeder { data: input };
    let verify = verify_wav(expected);

    let receipt = render(
        &mut p,
        &mut t,
        &sp,
        &mut feeder,
        RenderOutput {
            staged: StagedFile::create(&dest).unwrap(),
            make_muxer: wav_muxer_ctor(info_mux),
            encoder: Box::new(ruviform_io_wav::make_encoder(&info).expect("wav encoder")),
            publish_options: NO_OVERWRITE,
            verify: Some(verify),
            publish: SealedFile::publish,
        },
        &Progress::new(),
        &CancellationToken::new(),
        &mut map_noop_u32(),
    )
    .expect("wav render");
    assert_eq!(receipt.frames_emitted, len as u64);
    assert!(dest.exists(), "published destination exists");
    // Publish evidence: full durability was requested and achieved,
    // the staged temp is gone.
    assert_eq!(
        receipt.publish.durability(),
        DurabilityEvidence::FileAndDirectorySynced
    );
    assert_eq!(receipt.publish.temp_state(), TempState::Absent);
}

// ------------------------------------------------------------------
// OF-06: injected failures — cancel, write/disk-full, flush,
// publish, destination-exists, verify — all typed, all atomic.
// ------------------------------------------------------------------

/// Drives one standard 128-block render for failure injection:
/// `OnePole`, `Play` at 0, range = job = `[0, len)`.
fn run_render<Mk, V, Pb>(
    c: &Chain,
    len: u64,
    feeder: &mut impl InputFeeder,
    output: RenderOutput<Mk, V, Pb>,
    cancel: &CancellationToken,
) -> Result<RenderReceipt, RenderError>
where
    Mk: for<'a> FnOnce(&'a mut StagedFile) -> Result<Box<dyn Muxer + 'a>, MediaError>,
    V: FnOnce(&mut SealedFile, &OutputFacts) -> Result<(), MediaError>,
    Pb: FnOnce(SealedFile, PublishOptions) -> Result<PublishReceipt, PublishFailure>,
{
    let mut prep = prepared(c, OnePole::new(0));
    let mut tr = transport::<u32>(128);
    play_at(&mut tr, 0);
    let sp = spec(c, (0, len), TailSpec::None, (0, len), 0);
    render(
        &mut prep,
        &mut tr,
        &sp,
        feeder,
        output,
        &Progress::new(),
        cancel,
        &mut map_noop_u32(),
    )
}

/// `RenderOutput` writing through `FailSink` — injected disk-full at
/// `budget` bytes and/or unconditional flush failure.
#[expect(
    clippy::type_complexity,
    reason = "the capturing muxer ctor is opaque by construction"
)]
fn fail_output(
    dest: &Path,
    info_mux: StreamInfo,
    budget: u64,
    fail_flush: bool,
) -> RenderOutput<
    impl for<'a> FnOnce(&'a mut StagedFile) -> Result<Box<dyn Muxer + 'a>, MediaError>,
    VerifyFn,
    PublishFn,
> {
    RenderOutput {
        staged: StagedFile::create(dest).unwrap(),
        make_muxer: fail_muxer_ctor(info_mux, budget, fail_flush),
        encoder: Box::new(ruviform_io_wav::make_encoder(&wire_spec()).expect("enc")),
        publish_options: NO_OVERWRITE,
        verify: None,
        publish: SealedFile::publish,
    }
}

fn wire_spec() -> StreamInfo {
    WavOutputSpec {
        sample_rate: rate(),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F32,
    }
    .stream_info()
}

/// `open_muxer` over the staged borrow as a named `fn` — a declared
/// `<'a>` signature generalizes where a closure's `as _` cast would
/// collapse the muxer to `Box<dyn Muxer + 'static>`.
fn wav_muxer<'a>(
    staged: &'a mut StagedFile,
    info: &StreamInfo,
) -> Result<Box<dyn Muxer + 'a>, MediaError> {
    ruviform_io_wav::open_muxer(staged, std::slice::from_ref(info)).map(|m| Box::new(m) as _)
}

/// `wav_muxer` behind an HRTB-annotated constructor — the `impl
/// for<'a> FnOnce` return type propagates the expected signature into
/// the closure, which plain closures cannot express on their own.
fn wav_muxer_ctor(
    info: StreamInfo,
) -> impl for<'a> FnOnce(&'a mut StagedFile) -> Result<Box<dyn Muxer + 'a>, MediaError> {
    move |staged: &mut StagedFile| wav_muxer(staged, &info)
}

/// `FailSink` variant of [`wav_muxer`] — same lifetime reasoning.
fn fail_muxer<'a>(
    staged: &'a mut StagedFile,
    info: &StreamInfo,
    budget: u64,
    fail_flush: bool,
) -> Result<Box<dyn Muxer + 'a>, MediaError> {
    ruviform_io_wav::open_muxer(
        FailSink {
            inner: staged,
            written: 0,
            budget,
            fail_flush,
        },
        std::slice::from_ref(info),
    )
    .map(|m| Box::new(m) as _)
}

fn fail_muxer_ctor(
    info: StreamInfo,
    budget: u64,
    fail_flush: bool,
) -> impl for<'a> FnOnce(&'a mut StagedFile) -> Result<Box<dyn Muxer + 'a>, MediaError> {
    move |staged: &mut StagedFile| fail_muxer(staged, &info, budget, fail_flush)
}

/// `RenderOutput` over `CaptureEncoder`/`NullMuxer` with caller-chosen
/// options/verify/publish hooks — the "raw" counterpart of
/// `capture_output`. `Pb` is generic so failure-injecting closures
/// (which cannot coerce to the `PublishFn` pointer) fit too.
fn raw_output<Pb>(
    dest: &Path,
    options: PublishOptions,
    verify: Option<VerifyFn>,
    publish: Pb,
) -> RenderOutput<MuxerCtor, VerifyFn, Pb>
where
    Pb: FnOnce(SealedFile, PublishOptions) -> Result<PublishReceipt, PublishFailure>,
{
    RenderOutput {
        staged: StagedFile::create(dest).unwrap(),
        make_muxer: null_muxer,
        encoder: Box::new(CaptureEncoder {
            out: Arc::new(Mutex::new(Captured::default())),
        }),
        publish_options: options,
        verify,
        publish,
    }
}

fn assert_no_output(scratch: &Scratch, dest: &Path) {
    assert!(!dest.exists());
    assert_eq!(scratch.leftover_temps(), 0);
}

#[test]
fn of06_failure_injection() {
    let c = chain();
    let feeder_signal = || SignalFeeder {
        data: signal(2, 1_024),
    };
    let len = 512u64;
    let fresh = CancellationToken::new();

    // (a) Pre-cancelled token → Cancelled{process}, nothing emitted.
    {
        let (scratch, dest) = Scratch::new("of06-cancel");
        let cancel = CancellationToken::new();
        cancel.cancel();
        let err = run_render(
            &c,
            len,
            &mut feeder_signal(),
            capture_output(
                &dest,
                Arc::new(Mutex::new(Captured::default())),
                NO_OVERWRITE,
            ),
            &cancel,
        )
        .unwrap_err();
        assert!(matches!(err, RenderError::Cancelled { phase: "process" }));
        assert_no_output(&scratch, &dest);
    }

    // (b) Sink write failure mid-stream (disk-full) → typed output
    // failure, staged temp cleaned.
    {
        let (scratch, dest) = Scratch::new("of06-write");
        let err = run_render(
            &c,
            len,
            &mut feeder_signal(),
            fail_output(&dest, wire_spec(), 4_096, false),
            &fresh,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                RenderError::Output { .. } | RenderError::Finalize { .. }
            ),
            "write failure surfaces as Output/Finalize, got {err:?}"
        );
        assert_no_output(&scratch, &dest);
    }

    // (c) Sink flush failure at finalize (close/sync) → typed failure.
    {
        let (scratch, dest) = Scratch::new("of06-flush");
        let err = run_render(
            &c,
            len,
            &mut feeder_signal(),
            fail_output(&dest, wire_spec(), u64::MAX, true),
            &fresh,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                RenderError::Finalize { .. } | RenderError::Output { .. }
            ),
            "flush failure surfaces as Finalize/Output, got {err:?}"
        );
        assert_no_output(&scratch, &dest);
    }
}

#[test]
fn of06_staging_policy() {
    let c = chain();
    let feeder_signal = || SignalFeeder {
        data: signal(2, 1_024),
    };
    let len = 512u64;
    let fresh = CancellationToken::new();

    // (d) Publish failure → Publish carrying the real PublishFailure
    // (effect NotPublished); staged temp cleaned through the recovery
    // handle, destination untouched.
    {
        let (scratch, dest) = Scratch::new("of06-pub");
        // Renaming onto an existing *directory* fails on every
        // platform — a deterministic real publish failure.
        std::fs::create_dir(&dest).unwrap();
        let err = run_render(
            &c,
            len,
            &mut feeder_signal(),
            raw_output(
                &dest,
                PublishOptions::new(PublishMode::Replace, Durability::FileSync),
                None,
                SealedFile::publish,
            ),
            &fresh,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                RenderError::Publish {
                    ref source,
                    ref cleanup,
                } if source.effect() == PublishEffect::NotPublished
                    && cleanup.is_none()
            ),
            "publish failure keeps NotPublished effect, got {err:?}"
        );
        assert!(dest.is_dir(), "existing directory untouched");
        assert_eq!(scratch.leftover_temps(), 0, "staged temp cleaned");
    }

    // (e) Destination exists → DestinationExists before rendering;
    // existing destination byte-for-byte untouched.
    {
        let (scratch, dest) = Scratch::new("of06-dest");
        std::fs::write(&dest, b"pre-existing").unwrap();
        let err = run_render(
            &c,
            len,
            &mut feeder_signal(),
            capture_output(
                &dest,
                Arc::new(Mutex::new(Captured::default())),
                NO_OVERWRITE,
            ),
            &fresh,
        )
        .unwrap_err();
        assert!(matches!(err, RenderError::DestinationExists { .. }));
        assert_eq!(std::fs::read(&dest).unwrap(), b"pre-existing");
        assert_eq!(scratch.leftover_temps(), 0);
    }

    // (f) Verify hook rejection → Verify; nothing published.
    {
        let (scratch, dest) = Scratch::new("of06-verify");
        let err = run_render(
            &c,
            len,
            &mut feeder_signal(),
            raw_output(
                &dest,
                NO_OVERWRITE,
                Some(|_p, _f| Err(MediaError::from_kind(ErrorKind::InvalidData))),
                SealedFile::publish,
            ),
            &fresh,
        )
        .unwrap_err();
        assert!(matches!(err, RenderError::Verify { .. }));
        assert_no_output(&scratch, &dest);
    }
}

#[test]
fn of06_publish_lifecycle() {
    let c = chain();
    let feeder_signal = || SignalFeeder {
        data: signal(2, 1_024),
    };
    let len = 512u64;
    let fresh = CancellationToken::new();

    // (g) Publish-time DestinationExists — a destination appearing
    // after the render-entry check is refused by the atomic CreateNew
    // at publish and maps to the dedicated variant. The real staged
    // temp is cleaned via the failure's recovery handle.
    {
        let (scratch, dest) = Scratch::new("of06-destpub");
        let dest_inject = dest.clone();
        let err = run_render(
            &c,
            len,
            &mut feeder_signal(),
            raw_output(&dest, NO_OVERWRITE, None, move |sealed, opts| {
                // Destination appears between the early check and
                // the atomic publish — a real DestinationExists
                // PublishFailure (its temp survives inside the
                // failure's recovery handle).
                std::fs::write(&dest_inject, b"snuck in").unwrap();
                sealed.publish(opts)
            }),
            &fresh,
        )
        .unwrap_err();
        assert!(
            matches!(err, RenderError::DestinationExists { .. }),
            "publish-time refusal maps to DestinationExists, got {err:?}"
        );
        assert_eq!(std::fs::read(&dest).unwrap(), b"snuck in");
        assert_eq!(scratch.leftover_temps(), 0, "staged temp cleaned");
    }

    // (h) Cancel between verify and publish → Cancelled{publish};
    // the sealed temp is dropped and cleaned.
    {
        let (scratch, dest) = Scratch::new("of06-pubcancel");
        let cancel = CancellationToken::new();
        let cancel_in_verify = cancel.clone();
        let err = run_render(
            &c,
            len,
            &mut feeder_signal(),
            RenderOutput {
                staged: StagedFile::create(&dest).unwrap(),
                make_muxer: null_muxer,
                encoder: Box::new(CaptureEncoder {
                    out: Arc::new(Mutex::new(Captured::default())),
                }),
                publish_options: NO_OVERWRITE,
                verify: Some(move |_sealed: &mut SealedFile, _f: &OutputFacts| {
                    cancel_in_verify.cancel();
                    Ok(())
                }),
                publish: SealedFile::publish,
            },
            &cancel,
        )
        .unwrap_err();
        assert!(
            matches!(err, RenderError::Cancelled { phase: "publish" }),
            "cancel after verify hits the publish phase, got {err:?}"
        );
        assert_no_output(&scratch, &dest);
    }
}
