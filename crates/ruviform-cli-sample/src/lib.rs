//! `ruviform-cli-sample` — the Issue #155 usage example: a minimal,
//! GUI-independent CLI application built on the *shared* Ruviform
//! headless service surface.
//!
//! ## What this crate demonstrates (and what it deliberately is not)
//!
//! The crate is an *application adapter*: it composes
//! [`ruviform_exec::OfflineJob`] / [`ruviform_exec::OfflineDriver`],
//! the [`ruviform_media_io`] boundary and the `ruviform-io-*`
//! pure-Rust backends into one offline audio analysis (`probe` +
//! `run`). A GUI adapter would call the same service functions —
//! [`probe`], [`spawn_analyze`] — and render the same
//! [`AnalysisReport`]; only the outer shell differs.
//!
//! Staying on the application side of the Issue #155 responsibility
//! boundary, this crate owns: the command names (`probe`/`run`), the
//! JSON document schema, stdout/stderr policy (in `src/main.rs`), exit
//! codes, argument parsing and the deterministic `--cancel-after`
//! test hook. Ruviform owns everything underneath: demux/decode,
//! the `Processor`/`RtProcessor` kernel contract, offline driving,
//! cancellation and progress.
//!
//! ## Pipeline
//!
//! ```text
//! MediaSource ──sniff──► open_demuxer ──► select audio stream ──► make_decoder
//!        │                                                        │ packets
//!        ▼                                                        ▼
//!   probe report                                          decoded frames
//!        │                                                        │ (progress
//!        ▼                                                        ▼  bump per frame)
//!   JSON on stdout                                  Vec<AudioChunk> ──► OfflineDriver
//!                                                         │ (chunk-boundary cancel)
//!                                                         ▼
//!                                                  AnalysisReport ──► JSON on stdout
//! ```
//!
//! The whole `run` body executes inside one [`OfflineJob`]: the decode
//! phase reports indeterminate progress (`total = 0`), the driver phase
//! reports `completed/total` per chunk, and cancellation — `Ctrl-C`,
//! `cancel()`, `--cancel-after` — unwinds either phase through the same
//! [`CancellationToken`].
//!
//! **RT contract:** non-RT only — everything here is I/O, decode and
//! offline processing.

use std::convert::Infallible;
use std::io::SeekFrom;

use ruviform_audio_core::{AudioBuffer, AudioFormat};
use ruviform_exec::{JobScope, OfflineConfig, OfflineDriver, OfflineError, OfflineJob};
use ruviform_media_core::{Fact, StreamId, StreamKind, TimeBase, TimestampProvenance};
use ruviform_media_io::{
    BackendInfo, CodecFormat, CodecTag, ContainerInfo, Decoder, Demuxer, Error, ErrorKind, Frame,
    MediaSource, PcmTransform, StreamInfo,
};
use ruviform_rt::{Processor, RtContext, RtProcessor};

/// Container signature used to route between the backend adapters.
///
/// The sample's routing rule: `RIFF....AVI ` magic goes to
/// `ruviform-io-avi` (the AVI owner); everything else is handed to
/// `ruviform-io-symphonia`, whose probe decides acceptance — WAV,
/// FLAC, Ogg, Matroska and (opt-in) MP4 all reach it this way. An AVI
/// holding a codec `ruviform-io-avi` cannot decode surfaces the
/// backend's honest `Unsupported` fact/error rather than a guess.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum BackendKind {
    Avi,
    Symphonia,
}

/// Reads the leading bytes of `source` (then rewinds to position 0)
/// and picks the backend.
fn sniff_backend(source: &mut impl MediaSource) -> Result<BackendKind, Error> {
    let mut head = [0_u8; 12];
    let read = source
        .read(&mut head)
        .map_err(|e| Error::with_source(ErrorKind::Io, e))?;
    source
        .seek(SeekFrom::Start(0))
        .map_err(|e| Error::with_source(ErrorKind::Io, e))?;
    if read >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"AVI " {
        Ok(BackendKind::Avi)
    } else {
        Ok(BackendKind::Symphonia)
    }
}

/// Aggregated failure of a [`probe`] or `run` operation — the error
/// half the app maps onto its own diagnostics and exit codes.
///
/// Each variant keeps the underlying Ruviform error intact:
/// [`Media`](Self::Media) carries the normalized media-I/O
/// classification (reachable via [`error_kind`](Self::error_kind)),
/// [`Offline`](Self::Offline) the driver failure,
/// [`Usage`](Self::Usage) a request that cannot be satisfied by the
/// input (unknown stream id, non-audio stream selection), and
/// [`Cancelled`](Self::Cancelled) the cooperative-stop marker returned
/// when the job's token fires outside the driver itself.
#[non_exhaustive]
#[derive(Debug)]
pub enum PipelineError {
    /// A media-I/O boundary failure — open, demux or decode. The
    /// normalized [`ErrorKind`] is preserved.
    Media(Error),
    /// An offline-driver failure (`prepare` error, worker panic).
    /// `Cancelled` inside the driver is reported as
    /// [`Cancelled`](Self::Cancelled) instead.
    Offline(OfflineError),
    /// The request cannot be satisfied by this input — e.g. an unknown
    /// `--stream` id or a stream the backend cannot decode. Carries a
    /// static reason label plus the rendered detail.
    Usage(&'static str),
    /// The shared cancellation token was observed outside the driver
    /// (decode loop); the driver-side cancellation arrives as
    /// [`OfflineError::Cancelled`], which `run` maps here as well.
    Cancelled,
}

impl PipelineError {
    /// Stable machine-readable classification of the error — the
    /// `"kind"` field of the JSON error document. Media errors report
    /// their normalized [`ErrorKind`] name; the rest map to
    /// `"offline"`, `"usage"` or `"cancelled"`.
    #[must_use]
    pub fn error_kind(&self) -> &'static str {
        match self {
            Self::Media(e) => error_kind_name(e.kind()),
            Self::Offline(_) => "offline",
            Self::Usage(_) => "usage",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Media(e) => write!(
                f,
                "media error ({error_kind}): {e}",
                error_kind = error_kind_name(e.kind())
            ),
            Self::Offline(e) => write!(f, "offline driver error: {e}"),
            Self::Usage(reason) => write!(f, "usage error: {reason}"),
            Self::Cancelled => f.write_str("operation cancelled"),
        }
    }
}

impl std::error::Error for PipelineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Media(e) => Some(e),
            Self::Offline(e) => Some(e),
            _ => None,
        }
    }
}

/// The stable, matchable name of a normalized [`ErrorKind`].
///
/// `ErrorKind` is `#[non_exhaustive]`; a future variant reports
/// `"other"` until this mapping learns its name.
#[must_use]
pub fn error_kind_name(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Io => "io",
        ErrorKind::Unsupported => "unsupported",
        ErrorKind::InvalidData => "invalid_data",
        ErrorKind::InvalidValue => "invalid_value",
        ErrorKind::TimeOverflow => "time_overflow",
        ErrorKind::UnexpectedEos => "unexpected_eos",
        _ => "other",
    }
}

/// What a [`probe`] observed: container identification, backend
/// provenance and the full per-stream fact list — verbatim
/// `ruviform-media-io` values, classified (never defaulted) by the
/// backend.
///
/// **RT contract:** non-RT only — owns allocated stream facts.
#[derive(Debug)]
pub struct ProbeReport {
    /// Container identification.
    pub container: ContainerInfo,
    /// Backend provenance (`name` + `version`).
    pub backend: BackendInfo,
    /// Every stream the backend enumerated, in container order.
    pub streams: Vec<StreamInfo>,
}

/// Opens `source`, runs the backend's construction-time probe and
/// returns everything the demuxer reports — the `probe` half of the
/// sample.
///
/// This is the function a GUI "inspect" panel would call identically:
/// one [`MediaSource`] in, one owned report out, no I/O policy of its
/// own.
///
/// # Errors
///
/// [`PipelineError::Media`] on open/probe failure — the normalized
/// media-I/O classification is preserved.
///
/// **RT contract:** non-RT only — opens and scans the source.
pub fn probe(source: impl MediaSource + 'static) -> Result<ProbeReport, PipelineError> {
    let mut source = source;
    let kind = sniff_backend(&mut source).map_err(PipelineError::Media)?;
    match kind {
        BackendKind::Avi => {
            let demuxer = ruviform_io_avi::open_demuxer(source).map_err(PipelineError::Media)?;
            Ok(ProbeReport {
                container: demuxer.container().clone(),
                backend: demuxer.backend_info(),
                streams: demuxer.streams().to_vec(),
            })
        }
        BackendKind::Symphonia => {
            let demuxer =
                ruviform_io_symphonia::open_demuxer(source).map_err(PipelineError::Media)?;
            Ok(ProbeReport {
                container: demuxer.container().clone(),
                backend: demuxer.backend_info(),
                streams: demuxer.streams().to_vec(),
            })
        }
    }
}

/// Which scale the `run` analysis operated in — the report's honesty
/// label for the meaning of every `level` value.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SampleScale {
    /// Canonical path: planar `f32` normalized to `[-1, 1]` (the
    /// boundary's standard [`ruviform_media_io::AudioFrame`]
    /// representation).
    CanonicalF32,
    /// Native path: interleaved `f64` in the decoder's native PCM
    /// scale ([`ruviform_media_io::NativeAudioFrame`] — e.g. `±32768`
    /// for S16), un-normalized. `transforms` records the conversion
    /// history.
    NativeF64,
}

impl SampleScale {
    /// Stable label used in the JSON report's `"scale"` field.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CanonicalF32 => "canonical-f32",
            Self::NativeF64 => "native-f64",
        }
    }
}

/// Per-channel level measurement of one processed chunk — peak
/// magnitude and root-mean-square over the chunk's samples, in the
/// run's [`SampleScale`].
///
/// `f64` fields keep the native path lossless (the whole point of the
/// non-destructive contract); canonical `f32` values widen exactly.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Level {
    /// `max |sample|` over the chunk.
    pub peak: f64,
    /// `sqrt(mean(sample²))` over the chunk.
    pub rms: f64,
}

/// Per-chunk measurement record inside [`AnalysisReport`].
#[derive(Debug)]
pub struct ChunkReport {
    /// Decode order index (0-based among frames of the selected
    /// stream).
    pub index: u64,
    /// Frame count (samples per channel) carried by this chunk.
    pub frames: usize,
    /// Presentation timestamp — raw ticks with their own base and
    /// provenance. `None` only when the frame carried no time
    /// information at all.
    pub pts: Option<ChunkTimestamp>,
    /// Per-channel measurements, filled by [`MeterStage`].
    pub levels: Vec<Level>,
}

/// Counts a `run` accumulated — the operational numbers a diagnostics
/// consumer wants alongside the measurements.
#[derive(Debug, Default)]
pub struct RunStats {
    /// Encoded units the demuxer delivered, all streams.
    pub packets: u64,
    /// Packets forwarded to the decoder (selected stream only).
    pub forwarded: u64,
    /// Packets of other streams skipped without decoding.
    pub skipped: u64,
    /// Decoded frames / processed chunks.
    pub frames: u64,
    /// Total sample frames (per-channel sample count) processed.
    pub samples: u64,
}

/// The `run` operation's success value — the report a GUI panel and
/// the CLI serialize identically.
///
/// Stream/container/backend facts are kept as the verbatim
/// `ruviform-media-io`/`ruviform-media-core` types; [`report_json`]
/// renders the app-owned document shape on top.
#[derive(Debug)]
pub struct AnalysisReport {
    /// Container identification.
    pub container: ContainerInfo,
    /// Backend provenance.
    pub backend: BackendInfo,
    /// The stream that was decoded (full fact set).
    pub stream: StreamInfo,
    /// Scale the measurements are expressed in.
    pub scale: SampleScale,
    /// Native mode only: union of every chunk's recorded transform
    /// steps (empty for the canonical path).
    pub transforms: Vec<PcmTransform>,
    /// Native mode only: `true` when any recorded transform step can
    /// lose precision (`false` for the canonical path).
    pub lossy: bool,
    /// Per-chunk measurements in decode order — the deterministic
    /// output (identical for sequential and parallel driver runs).
    pub chunks: Vec<ChunkReport>,
    /// Aggregated per-channel levels over all chunks.
    pub summary: Vec<Level>,
    /// Operational counters.
    pub stats: RunStats,
    /// Worker count the driver ran with (`1` = sequential reference
    /// path).
    pub workers: usize,
}

/// Knobs of a `run` operation — the request half. Everything
/// operational lives here; I/O policy (stdout, exit codes) does not.
#[derive(Copy, Clone, Debug)]
pub struct AnalyzeOptions {
    /// Explicit stream id to decode (`--stream`). `None` selects the
    /// first audio stream the backend marks decodable.
    pub stream: Option<u32>,
    /// Driver worker count (`--workers`); `1` is the sequential
    /// determinism-reference path.
    pub workers: usize,
    /// `true` selects the native-scale (non-destructive) decoder —
    /// `Frame::NativeAudio`, `f64` samples in the source PCM scale
    /// (`--native`).
    pub native: bool,
    /// Deterministic cancellation hook (`--cancel-after`): `Some(n)`
    /// cancels the job once `n` units of progress have completed —
    /// decoded frames in the decode phase, then processed chunks. The
    /// trigger is armed inside the job itself
    /// ([`OfflineJob::spawn_with_cancel_after`]), so the request lands
    /// worker-side even when the run finishes faster than an `events()`
    /// consumer could observe it.
    pub cancel_after: Option<u64>,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            stream: None,
            workers: 1,
            native: false,
            cancel_after: None,
        }
    }
}

/// A chunk presentation timestamp — raw ticks plus the time base they
/// are expressed in and their provenance. `base` is a [`Fact`] because
/// native frames can exist when the stream's declared base is
/// unavailable (the ticks stay observable regardless).
///
/// **RT contract:** non-RT only — `Copy` value.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChunkTimestamp {
    /// Tick count in `base` units.
    pub ticks: i64,
    /// The base `ticks` is expressed in — the frame's own base for the
    /// canonical path, the stream's declared base for native.
    pub base: Fact<TimeBase>,
    /// Where the value came from.
    pub provenance: TimestampProvenance,
}

/// Decoded-audio block — the `Io` handed through the driver.
///
/// The meter kernel fills [`levels`](Self::levels) in place; the
/// sample data itself is either a canonical planar `f32`
/// [`AudioBuffer`] or native-scale interleaved `f64` plus the channel
/// count.
#[derive(Debug)]
pub struct AudioChunk {
    /// Decode order index.
    pub index: u64,
    /// Presentation timestamp (`None` when the frame carries none).
    pub pts: Option<ChunkTimestamp>,
    /// Sample storage.
    pub samples: ChunkSamples,
    /// Per-channel measurement slots — pre-sized at decode time,
    /// filled by [`Meter::process`].
    pub levels: Vec<Level>,
    /// Native mode: this chunk's conversion history (empty on the
    /// canonical path).
    pub transforms: Vec<PcmTransform>,
}

impl AudioChunk {
    /// Frames (samples per channel) carried by this chunk.
    #[must_use]
    pub fn frames(&self) -> usize {
        match &self.samples {
            ChunkSamples::Canonical(buffer) => buffer.frames(),
            ChunkSamples::Native { samples, channels } => {
                if *channels == 0 {
                    0
                } else {
                    samples.len() / channels
                }
            }
        }
    }
}

/// Sample storage of an [`AudioChunk`].
#[non_exhaustive]
#[derive(Debug)]
pub enum ChunkSamples {
    /// Canonical planar `f32` ([`AudioFrame`](ruviform_media_io::AudioFrame)).
    Canonical(AudioBuffer),
    /// Native-scale interleaved `f64`
    /// ([`NativeAudioFrame`](ruviform_media_io::NativeAudioFrame)) —
    /// `samples[i * channels + c]`.
    Native {
        /// Interleaved native-scale values.
        samples: Vec<f64>,
        /// Channel count of the interleave.
        channels: usize,
    },
}

/// The sample's offline processing kernel: per-channel peak/RMS
/// metering over each chunk — a deliberately small `Processor`
/// demonstrating how an app's kernel wires into
/// [`OfflineDriver`] (and, by extension, any other RT-context driver:
/// the same `Processor` impl is what a GUI engine would `prepare`).
///
/// The `Rt` half is allocation- and blocking-free, honoring the
/// `RtProcessor` contract; `prepare` is a no-op because metering needs
/// no configuration.
#[derive(Clone, Debug, Default)]
pub struct MeterStage;

impl Processor for MeterStage {
    type Setup = ();
    type Rt = Meter;
    type Error = Infallible;

    fn prepare(&mut self, _setup: Self::Setup) -> Result<Self::Rt, Self::Error> {
        Ok(Meter)
    }
}

/// RT-half of [`MeterStage`]: writes `peak`/`rms` into
/// [`AudioChunk::levels`] — in-place, allocation-free, bounded by the
/// chunk length (`RtProcessor`-contract honest).
#[derive(Debug)]
pub struct Meter;

impl RtProcessor for Meter {
    type Io = AudioChunk;

    #[expect(
        clippy::cast_precision_loss,
        reason = "sample counts are far below the f64 exact-integer range"
    )]
    fn process(&mut self, io: &mut Self::Io, _ctx: &RtContext) {
        for (channel, level) in io.levels.iter_mut().enumerate() {
            let (mut peak, mut sum_sq, mut n) = (0.0_f64, 0.0_f64, 0_u64);
            match &io.samples {
                ChunkSamples::Canonical(buffer) => {
                    if let Some(samples) = buffer.channel(channel) {
                        for &x in samples {
                            let v = f64::from(x.abs());
                            peak = peak.max(v);
                            sum_sq += v * v;
                            n += 1;
                        }
                    }
                }
                ChunkSamples::Native { samples, channels } => {
                    for frame in samples.chunks(*channels) {
                        // `get` keeps a short tail (a malformed
                        // interleave) from becoming a panic.
                        if let Some(&x) = frame.get(channel) {
                            let v = x.abs();
                            peak = peak.max(v);
                            sum_sq += v * v;
                            n += 1;
                        }
                    }
                }
            }
            level.peak = peak;
            level.rms = if n == 0 {
                0.0
            } else {
                (sum_sq / n as f64).sqrt()
            };
        }
    }
}

/// Spawns the `run` operation as an [`OfflineJob`]: decode the
/// selected audio stream, then drive [`MeterStage`] over the decoded
/// chunks via [`OfflineDriver`].
///
/// Returns the handle immediately — the caller observes
/// [`OfflineJob::events`]/[`OfflineJob::progress`]/[`OfflineJob::state`]
/// and collects the report via [`OfflineJob::wait`]. This is the exact
/// entry point a GUI adapter would call (its event loop renders the
/// same [`JobEvent`](ruviform_exec::JobEvent) stream the CLI prints as
/// JSONL).
///
/// **Cancellation:** [`OfflineJob::cancel`] (or a cloned
/// [`CancellationToken`]) stops the decode loop at the next packet and
/// the driver at the next chunk boundary.
/// [`AnalyzeOptions::cancel_after`] arms the same path at spawn, via
/// [`OfflineJob::spawn_with_cancel_after`].
///
/// **RT contract:** non-RT only — spawns a thread.
#[must_use]
pub fn spawn_analyze(
    source: impl MediaSource + 'static,
    options: AnalyzeOptions,
) -> OfflineJob<AnalysisReport, PipelineError> {
    let body = move |scope: &JobScope| analyze(source, options, scope);
    match options.cancel_after {
        Some(n) => OfflineJob::spawn_with_cancel_after(body, n),
        None => OfflineJob::spawn(body),
    }
}

/// The synchronous body of `run` — executes entirely inside the job's
/// worker thread. `scope` supplies the shared token/progress the job
/// wired up.
///
/// **RT contract:** non-RT only.
fn analyze(
    mut source: impl MediaSource + 'static,
    options: AnalyzeOptions,
    scope: &JobScope,
) -> Result<AnalysisReport, PipelineError> {
    let kind = sniff_backend(&mut source).map_err(PipelineError::Media)?;
    match kind {
        BackendKind::Avi => {
            let demuxer = ruviform_io_avi::open_demuxer(source).map_err(PipelineError::Media)?;
            let stream = select_stream(&demuxer, options.stream)?;
            let decoder = if options.native {
                demuxer.make_native_decoder(stream.stream.id)
            } else {
                demuxer.make_decoder(stream.stream.id)
            }
            .map_err(PipelineError::Media)?;
            decode_and_analyze(demuxer, decoder, stream, options, scope)
        }
        BackendKind::Symphonia => {
            let demuxer =
                ruviform_io_symphonia::open_demuxer(source).map_err(PipelineError::Media)?;
            let stream = select_stream(&demuxer, options.stream)?;
            let decoder = if options.native {
                demuxer.make_native_decoder(stream.stream.id)
            } else {
                demuxer.make_decoder(stream.stream.id)
            }
            .map_err(PipelineError::Media)?;
            decode_and_analyze(demuxer, decoder, stream, options, scope)
        }
    }
}

/// Picks the stream to decode: an explicit `--stream` id, else the
/// first audio stream the backend marks decodable.
fn select_stream(
    demuxer: &dyn Demuxer,
    requested: Option<u32>,
) -> Result<StreamInfo, PipelineError> {
    match requested {
        Some(id) => {
            let info = demuxer
                .stream(StreamId(id))
                .ok_or(PipelineError::Usage("stream id not found"))?;
            if info.stream.kind != StreamKind::Audio {
                return Err(PipelineError::Usage("selected stream is not audio"));
            }
            if !matches!(info.facts.decodable, Fact::Known(())) {
                return Err(PipelineError::Usage(
                    "selected stream is not decodable by this backend",
                ));
            }
            Ok(info.clone())
        }
        None => demuxer
            .streams()
            .iter()
            .find(|info| {
                info.stream.kind == StreamKind::Audio
                    && matches!(info.facts.decodable, Fact::Known(()))
            })
            .cloned()
            .ok_or(PipelineError::Usage("no decodable audio stream")),
    }
}

/// Demux → decode → meter the selected stream — the shared pipeline
/// core, generic over the concrete backend pair (`make_decoder` is an
/// inherent method per backend, so dispatch happens once at open).
fn decode_and_analyze<D: Demuxer, C: Decoder>(
    mut demuxer: D,
    mut decoder: C,
    stream: StreamInfo,
    options: AnalyzeOptions,
    scope: &JobScope,
) -> Result<AnalysisReport, PipelineError> {
    let container = demuxer.container().clone();
    let backend = demuxer.backend_info();
    let stream_id = stream.stream.id;

    // Decode phase: indeterminate progress — the packet/frame count is
    // not known up-front (a `declared_units` fact, when present, counts
    // stream units, not demuxer packets). `total = 0` keeps the
    // counters honest.
    scope.progress().reset(0);
    let mut chunks: Vec<AudioChunk> = Vec::new();
    let mut stats = RunStats::default();
    let mut transforms: Vec<PcmTransform> = Vec::new();
    let mut index = 0_u64;
    let mut drained = false;
    loop {
        if scope.is_cancelled() {
            return Err(PipelineError::Cancelled);
        }
        if let Some(packet) = demuxer.read_packet().map_err(PipelineError::Media)? {
            stats.packets += 1;
            if packet.stream != stream_id {
                stats.skipped += 1;
                continue;
            }
            stats.forwarded += 1;
            decoder.send_packet(packet).map_err(PipelineError::Media)?;
        } else {
            // EOS: signal end of input, then pump `receive_frame`
            // once more — delayed codecs flush held frames here.
            decoder.drain().map_err(PipelineError::Media)?;
            drained = true;
        }
        while let Some(frame) = decoder.receive_frame().map_err(PipelineError::Media)? {
            collect_frame(
                frame,
                &mut chunks,
                &mut transforms,
                &mut index,
                options.native,
                stream.stream.time_base,
            )?;
            scope.progress().bump();
        }
        if drained {
            break;
        }
    }
    stats.frames = index;
    stats.samples = chunks.iter().map(AudioChunk::frames).sum::<usize>() as u64;

    // Process phase: the driver owns `completed/total` from here on.
    let mut stage = MeterStage;
    let config = OfflineConfig {
        workers: options.workers,
        queue_capacity: 0,
    };
    let cancel = scope.cancel_token().clone();
    let progress = scope.progress().clone();
    let chunks = if config.workers <= 1 {
        OfflineDriver::run(&mut stage, (), chunks, &cancel, &progress)
    } else {
        OfflineDriver::run_parallel(config, &stage, (), chunks, &cancel, &progress)
    }
    .map_err(|e| match e {
        OfflineError::Cancelled => PipelineError::Cancelled,
        other => PipelineError::Offline(other),
    })?;

    let summary = summarize(&chunks);
    let lossy = transforms.iter().any(|t| t.is_lossy());
    Ok(AnalysisReport {
        container,
        backend,
        stream,
        scale: if options.native {
            SampleScale::NativeF64
        } else {
            SampleScale::CanonicalF32
        },
        transforms,
        lossy,
        chunks: chunks
            .into_iter()
            .map(|chunk| ChunkReport {
                index: chunk.index,
                frames: chunk.frames(),
                pts: chunk.pts,
                levels: chunk.levels,
            })
            .collect(),
        summary,
        stats,
        workers: options.workers.max(1),
    })
}

/// Aggregates per-chunk [`Level`]s into run-wide per-channel
/// `peak`/`rms` — peak is the max, rms the root of the frame-weighted
/// mean of per-chunk mean squares.
#[expect(
    clippy::cast_precision_loss,
    reason = "sample counts are far below the f64 exact-integer range"
)]
fn summarize(chunks: &[AudioChunk]) -> Vec<Level> {
    let mut summary = Vec::<Level>::new();
    let mut total_frames = 0_u64;
    for chunk in chunks {
        let n = chunk.frames() as u64;
        total_frames += n;
        for (ch, level) in chunk.levels.iter().enumerate() {
            if summary.len() <= ch {
                summary.resize(ch + 1, Level::default());
            }
            let slot = &mut summary[ch];
            slot.peak = slot.peak.max(level.peak);
            // Recombine rms² × frames into a global mean of squares.
            slot.rms += level.rms * level.rms * n as f64;
        }
    }
    for slot in &mut summary {
        slot.rms = if total_frames == 0 {
            0.0
        } else {
            (slot.rms / total_frames as f64).sqrt()
        };
    }
    summary
}

/// Appends one decoded frame as an [`AudioChunk`] — or rejects a frame
/// kind that cannot satisfy the request (usage-half error).
fn collect_frame(
    frame: Frame,
    chunks: &mut Vec<AudioChunk>,
    transforms: &mut Vec<PcmTransform>,
    index: &mut u64,
    native: bool,
    stream_base: Fact<TimeBase>,
) -> Result<(), PipelineError> {
    match (frame, native) {
        (Frame::Audio(frame), false) => {
            let pts = frame.pts();
            let buffer = frame.into_buffer();
            let channels = buffer.channels();
            chunks.push(AudioChunk {
                index: *index,
                pts: Some(ChunkTimestamp {
                    ticks: pts.time.value(),
                    base: Fact::Known(pts.time.base()),
                    provenance: pts.provenance,
                }),
                samples: ChunkSamples::Canonical(buffer),
                levels: vec![Level::default(); channels],
                transforms: Vec::new(),
            });
            *index += 1;
            Ok(())
        }
        (Frame::NativeAudio(frame), true) => {
            let format = frame.format();
            let channels = format.layout.channels() as usize;
            let pts = frame.pts();
            for step in frame.transforms() {
                if !transforms.contains(step) {
                    transforms.push(*step);
                }
            }
            chunks.push(AudioChunk {
                index: *index,
                pts: Some(ChunkTimestamp {
                    ticks: pts.ticks,
                    // Native ticks live in the owning stream's base;
                    // the frame itself carries none.
                    base: stream_base,
                    provenance: pts.provenance,
                }),
                samples: ChunkSamples::Native {
                    samples: frame.samples().to_vec(),
                    channels,
                },
                levels: vec![Level::default(); channels],
                transforms: frame.transforms().to_vec(),
            });
            *index += 1;
            Ok(())
        }
        (Frame::Audio(_), true) | (Frame::NativeAudio(_), false) => Err(PipelineError::Usage(
            "decoder emitted a frame kind outside the requested scale",
        )),
        (Frame::Video(_), _) => Err(PipelineError::Usage("selected stream emitted video frames")),
        _ => Err(PipelineError::Usage(
            "decoder emitted an unhandled frame kind",
        )),
    }
}

// --- JSON rendering (the app-owned document schema) ----------------

/// Renders a [`ProbeReport`] as the sample's JSON document —
/// `{"status":"probe", ...}` facts verbatim from the boundary types.
///
/// Every `Fact`/`Option` keeps its classified absence:
/// `"not_declared"` / `"unsupported"` / `"malformed"` strings instead
/// of fabricated values.
#[must_use]
pub fn probe_json(report: &ProbeReport) -> serde_json::Value {
    serde_json::json!({
        "status": "ok",
        "operation": "probe",
        "container": container_json(&report.container),
        "backend": backend_json(&report.backend),
        "streams": report.streams.iter().map(stream_json).collect::<Vec<_>>(),
    })
}

/// Renders an [`AnalysisReport`] as the sample's `run` JSON document —
/// `{"status":"completed", ...}`.
#[must_use]
pub fn report_json(report: &AnalysisReport) -> serde_json::Value {
    serde_json::json!({
        "status": "completed",
        "operation": "run",
        "container": container_json(&report.container),
        "backend": backend_json(&report.backend),
        "stream": stream_json(&report.stream),
        "processing": {
            "kernel": "peak-rms-meter",
            "workers": report.workers,
            "scale": report.scale.name(),
            "lossy": report.lossy,
            "transforms": report.transforms.iter().copied().map(transform_name).collect::<Vec<_>>(),
        },
        "stats": {
            "packets": report.stats.packets,
            "forwarded": report.stats.forwarded,
            "skipped": report.stats.skipped,
            "frames": report.stats.frames,
            "samples": report.stats.samples,
        },
        "summary": report.summary.iter().map(level_json).collect::<Vec<_>>(),
        "chunks": report.chunks.iter().map(chunk_json).collect::<Vec<_>>(),
    })
}

/// Renders a failure as the sample's error document —
/// `{"status":"failed"|"cancelled", "error":{kind,message}}`. The app
/// maps the outcome to its exit code; the JSON kind stays
/// machine-readable either way. `operation` is the issuing command
/// name (`"probe"`/`"run"`).
#[must_use]
pub fn error_json(operation: &str, error: &PipelineError) -> serde_json::Value {
    let status = if matches!(error, PipelineError::Cancelled) {
        "cancelled"
    } else {
        "failed"
    };
    serde_json::json!({
        "status": status,
        "operation": operation,
        "error": {
            "kind": error.error_kind(),
            "message": error.to_string(),
        },
    })
}

/// Renders one [`JobEvent`](ruviform_exec::JobEvent) as a JSONL
/// progress line (stderr side channel).
#[must_use]
pub fn event_json(event: &ruviform_exec::JobEvent) -> serde_json::Value {
    use ruviform_exec::JobEvent;
    match *event {
        JobEvent::Started => serde_json::json!({"event": "started"}),
        JobEvent::Progress { completed, total } => {
            serde_json::json!({"event": "progress", "completed": completed, "total": total})
        }
        JobEvent::Finished { outcome } => {
            let name = match outcome {
                ruviform_exec::JobOutcome::Completed => "completed",
                ruviform_exec::JobOutcome::Cancelled => "cancelled",
                ruviform_exec::JobOutcome::Failed => "failed",
                _ => "other",
            };
            serde_json::json!({"event": "finished", "outcome": name})
        }
        _ => serde_json::json!({"event": "other"}),
    }
}

fn container_json(container: &ContainerInfo) -> serde_json::Value {
    serde_json::json!({
        "format": container.format.name(),
        "detail": container.detail.is_some(),
    })
}

fn backend_json(backend: &BackendInfo) -> serde_json::Value {
    serde_json::json!({
        "name": backend.name,
        "version": backend.version,
    })
}

fn stream_json(info: &StreamInfo) -> serde_json::Value {
    let mut object = serde_json::json!({
        "id": info.stream.id.0,
        "index": info.facts.index,
        "kind": stream_kind_name(info.stream.kind),
        "codec": info.codec.codec.name(),
        "time_base": fact_json(&info.stream.time_base, |base| {
            serde_json::json!({
                "numerator": base.numerator(),
                "denominator": base.denominator(),
            })
        }),
        "duration_ticks": info.stream.duration.map(ruviform_media_core::MediaDuration::value),
        "decodable": fact_name(&info.facts.decodable),
    });
    if let Some(tag) = info.facts.codec_tag {
        object["codec_tag"] = match tag {
            CodecTag::FourCc(code) => serde_json::json!({
                "kind": "fourcc",
                "bytes": code,
                "text": String::from_utf8_lossy(&code),
            }),
            CodecTag::Numeric(tag) => serde_json::json!({"kind": "numeric", "value": tag}),
            _ => serde_json::json!({"kind": "other"}),
        };
    }
    if let CodecFormat::Audio(format) = info.codec.format {
        object["audio_format"] = audio_format_json(format);
    }
    if let Fact::Known(CodecFormat::Audio(native)) = info.codec.native_format {
        object["native_format"] = audio_format_json(native);
    }
    object
}

fn chunk_json(chunk: &ChunkReport) -> serde_json::Value {
    serde_json::json!({
        "index": chunk.index,
        "frames": chunk.frames,
        "pts": chunk.pts.map(|pts| serde_json::json!({
            "ticks": pts.ticks,
            "base": fact_json(&pts.base, |base| serde_json::json!({
                "numerator": base.numerator(),
                "denominator": base.denominator(),
            })),
            "provenance": provenance_name(pts.provenance),
        })),
        "levels": chunk.levels.iter().map(level_json).collect::<Vec<_>>(),
    })
}

fn level_json(level: &Level) -> serde_json::Value {
    serde_json::json!({"peak": level.peak, "rms": level.rms})
}

fn audio_format_json(format: AudioFormat) -> serde_json::Value {
    serde_json::json!({
        "sample_rate": format.sample_rate.get(),
        "channels": format.layout.channels(),
        "layout": layout_name(format.layout),
        "sample_format": sample_format_name(format.sample_format),
    })
}

/// Renders a `Fact<T>` as `{"fact": "<classification>", "value": …}`
/// — the classified-absence contract in JSON form.
fn fact_json<T>(fact: &Fact<T>, render: impl FnOnce(&T) -> serde_json::Value) -> serde_json::Value {
    match fact {
        Fact::Known(value) => serde_json::json!({"fact": "known", "value": render(value)}),
        other => serde_json::json!({"fact": fact_name(other)}),
    }
}

/// The classification label of a `Fact` — mirrors
/// `Display` but is the stable JSON vocabulary.
fn fact_name<T>(fact: &Fact<T>) -> &'static str {
    match fact {
        Fact::Known(_) => "known",
        Fact::NotDeclared => "not_declared",
        Fact::Unsupported => "unsupported",
        Fact::Malformed => "malformed",
        _ => "other",
    }
}

fn stream_kind_name(kind: StreamKind) -> &'static str {
    match kind {
        StreamKind::Audio => "audio",
        StreamKind::Video => "video",
        StreamKind::Data => "data",
        _ => "other",
    }
}

fn provenance_name(provenance: TimestampProvenance) -> &'static str {
    match provenance {
        TimestampProvenance::Carried => "carried",
        TimestampProvenance::Derived => "derived",
        TimestampProvenance::Estimated => "estimated",
        _ => "other",
    }
}

fn layout_name(layout: ruviform_audio_core::ChannelLayout) -> &'static str {
    use ruviform_audio_core::ChannelLayout;
    match layout {
        ChannelLayout::Mono => "mono",
        ChannelLayout::Stereo => "stereo",
        ChannelLayout::Quad => "quad",
        ChannelLayout::FivePointOne => "5.1",
        ChannelLayout::SevenPointOne => "7.1",
        ChannelLayout::Custom(_) => "custom",
        _ => "other",
    }
}

fn sample_format_name(format: ruviform_audio_core::SampleFormat) -> &'static str {
    use ruviform_audio_core::SampleFormat;
    match format {
        SampleFormat::F32 => "f32",
        SampleFormat::F64 => "f64",
        SampleFormat::U8 => "u8",
        SampleFormat::S8 => "s8",
        SampleFormat::U16 => "u16",
        SampleFormat::S16 => "s16",
        SampleFormat::U24 => "u24",
        SampleFormat::S24 => "s24",
        SampleFormat::U32 => "u32",
        SampleFormat::S32 => "s32",
        _ => "other",
    }
}

fn transform_name(transform: PcmTransform) -> &'static str {
    match transform {
        PcmTransform::IntegerWidenedToF64 => "integer_widened_to_f64",
        PcmTransform::Float32WidenedToF64 => "float32_widened_to_f64",
        PcmTransform::Normalized => "normalized",
        PcmTransform::Quantized => "quantized",
        PcmTransform::Resampled => "resampled",
        _ => "other",
    }
}
