//! The bounded prefetch pipeline: non-RT decode into a fixed
//! [`PcmBlock`] pool, single-owner handoff through two SPSC rings, and
//! an RT [`PrefetchConsumer`] an audio callback fills from without
//! allocating, locking or touching the OS.
//!
//! ## Structure
//!
//! ```text
//! PrefetchHandle (non-RT owner)            PrefetchProducer (non-RT thread / manual pump)
//!   seek / reopen / close ── mpsc cmd ──▶   reads cmd, fills PcmBlocks via SourceReader,
//!                                           pushes PrefetchItem into the data ring
//!   PrefetchTelemetry ◀─ relaxed atomics ──  (underrun/fault/stale/position counters)
//!
//! data ring: Block(PcmBlock) | Boundary{gen,first} | Fault{gen,kind,first,frames} | End{gen}
//! free ring: PcmBlock returns from consumer to producer (capacity == pool size)
//!
//! PrefetchConsumer (RT endpoint): fill(&mut [&mut [f32]], expected_first, &RtContext)
//! ```
//!
//! ## Contract
//!
//! * **Generation.** `u64`, bumps on every seek/reopen; every item is
//!   tagged. The consumer adopts the newest `Boundary`'s generation and
//!   returns any item tagged otherwise to the free ring — stale
//!   prefetch results can never cross a seek.
//! * **Position.** Every stream element advances `next_frame`
//!   deterministically: a `Block` covers `[first_frame, +valid)`,
//!   a `Fault` covers `[first_frame, +frames)`, `Boundary` resets it,
//!   `End` marks exhaustion. Gaps are structural, never guessed.
//!   `fill`'s `expected_first` is a *stream-position assertion* in
//!   source frames: the caller declares which `next_frame` it
//!   believes the stream is at, and the fill is refused as a
//!   `mismatch` when the assertion is wrong. It is not an
//!   engine-frame or output-frame expectation — engine-side mapping
//!   stays with the caller.
//! * **Faults.** A span that cannot be served as decoded content is
//!   fault-marked and zero-filled — the declared bounded fallback,
//!   always counted in `FillReport`/`PrefetchTelemetry`, never
//!   reported as content. `End` (EOF) is distinct from underrun: past
//!   the end there is nothing left to produce.
//! * **Drive.** One `pump()` call drains every queued command, then
//!   performs at most one sequenced item of work — push one pending
//!   marker, emit `End`, or fill and push one block. `Backpressured`
//!   means work exists but a ring has no slot: markers may wait in
//!   the producer `pending` queue, so a manual driver iterates
//!   `pump()` until `Idle`/`Ended` rather than assuming one call
//!   drains the pipeline.
//! * **Single owner.** Exactly `block_count` `PcmBlock`s exist. Pool →
//!   free ring → producer → data ring → consumer → free ring. The
//!   free ring's capacity equals the pool size, so the consumer's
//!   return push can never fail — it never deallocates on RT.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use ruviform_media_core::Fact;
use ruviform_media_io::{Demuxer, Error, ErrorKind, MediaSource, Result};
use ruviform_rt::RtContext;
use ruviform_rt::{Consumer, Producer, channel};

use crate::mapping::SourceFrame;
use crate::reader::SourceReader;

/// Marker slack beyond the pool size in the data ring, so `Boundary` /
/// `Fault` / `End` markers never stall behind a ring full of blocks.
const MARKER_SLACK: usize = 8;

/// Which stream fault a marker reports — the pipeline-level
/// classification every non-content span maps onto.
///
/// Mapped from [`ErrorKind`] at the boundary (`UnexpectedEos` →
/// `Truncated`, `InvalidData` → `Corrupt`, `Io` → `Io`,
/// `Unsupported` → `Unsupported`); `InvalidSample` and `Unseekable`
/// are pipeline-detected classes with no boundary kind.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum SourceFault {
    /// Input ended before a complete unit — media-io `UnexpectedEos`.
    Truncated,
    /// Malformed container/packet data — media-io `InvalidData`.
    Corrupt,
    /// Underlying source read/seek failure — media-io `Io`.
    Io,
    /// A decoded sample was not a finite number — pipeline audit.
    InvalidSample,
    /// A capability gap (codec, layout, decode mode) — media-io
    /// `Unsupported`.
    #[default]
    Unsupported,
    /// A seek request the source could not honour.
    Unseekable,
}

impl SourceFault {
    /// Maps a boundary error class onto the pipeline taxonomy.
    fn of_kind(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::UnexpectedEos => Self::Truncated,
            ErrorKind::InvalidData => Self::Corrupt,
            ErrorKind::Io => Self::Io,
            _ => Self::Unsupported,
        }
    }
}

/// Per-kind fault-event counts in a [`FillReport`].
///
/// **RT contract:** RT-suitable — `Copy` plain data.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FaultCounts {
    /// Truncated-input events.
    pub truncated: u64,
    /// Corrupt-data events.
    pub corrupt: u64,
    /// Source-failure events.
    pub io: u64,
    /// Invalid-sample events.
    pub invalid_sample: u64,
    /// Unsupported-capability events.
    pub unsupported: u64,
    /// Failed-seek events.
    pub unseekable: u64,
}

impl FaultCounts {
    fn bump(&mut self, kind: SourceFault) {
        match kind {
            SourceFault::Truncated => self.truncated += 1,
            SourceFault::Corrupt => self.corrupt += 1,
            SourceFault::Io => self.io += 1,
            SourceFault::InvalidSample => self.invalid_sample += 1,
            SourceFault::Unsupported => self.unsupported += 1,
            SourceFault::Unseekable => self.unseekable += 1,
        }
    }
}

/// What one [`PrefetchConsumer::fill`] call observed — the honest
/// accounting of every requested frame.
///
/// `written` is real decoded content. `underrun` is frames the
/// producer had not delivered in time. `faulted` is frames covered by
/// fault markers (zeroed). `eof` means the stream's `End` marker was
/// consumed — the destination tail beyond delivered content is
/// silence by *exhaustion*, not underrun. `mismatch` means the
/// caller's `expected_first` assertion disagreed with the stream's
/// `next_frame` — nothing was consumed and the destination was
/// zero-filled.
///
/// Every non-`written` span is zero-filled: the declared bounded
/// fallback, visible here and in [`PrefetchTelemetry`] — never
/// "verified content".
///
/// **RT contract:** RT-suitable — `Copy` plain data.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FillReport {
    /// Frames the caller asked for.
    pub requested: usize,
    /// Real decoded frames written.
    pub written: usize,
    /// Frames the pipeline could not supply in time (starvation).
    pub underrun: usize,
    /// Frames covered by fault markers (zeroed).
    pub faulted: usize,
    /// The stream's end was reached inside this fill.
    pub eof: bool,
    /// The caller's `expected_first` stream-position assertion
    /// mismatched `next_frame` — nothing was consumed; the caller
    /// must seek or resynchronize.
    pub mismatch: bool,
    /// Per-kind fault events consumed in this fill.
    pub faults: FaultCounts,
}

/// One allocation-unit of prefetched PCM — a fixed channel-major
/// planar slab owned by exactly one pipeline stage at a time.
///
/// Layout: `data[c * block_frames .. c * block_frames + valid_frames)`
/// holds channel `c`'s valid samples. A block also records which
/// source frames it covers and which generation produced it — the
/// consumer's sequencing and stale-drop evidence.
///
/// **RT contract:** the block *handle* is RT-passable by move; only
/// its construction (in the pool, non-RT) allocates.
#[derive(Debug)]
pub struct PcmBlock {
    data: Vec<f32>,
    channels: usize,
    block_frames: usize,
    first_frame: u64,
    valid_frames: usize,
    generation: u64,
    fault: Option<BlockFault>,
}

/// A faulted sub-span inside a [`PcmBlock`] — positions
/// `[first, first + frames)` carry zeros instead of content.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockFault {
    /// The fault class.
    pub kind: SourceFault,
    /// First faulted frame inside the block (block-relative).
    pub first: u32,
    /// Faulted frame count.
    pub frames: u32,
}

impl PcmBlock {
    /// Pool construction — allocates the channel-major slab.
    ///
    /// **RT contract:** non-RT only — allocates.
    fn new(channels: usize, block_frames: usize) -> Self {
        Self {
            data: vec![0.0; channels * block_frames],
            channels,
            block_frames,
            first_frame: 0,
            valid_frames: 0,
            generation: 0,
            fault: None,
        }
    }

    /// Channel `c`'s valid samples — `[0, valid_frames)`.
    ///
    /// **RT contract:** RT-suitable — bounds-checked slice access.
    #[must_use]
    pub fn channel(&self, index: usize) -> Option<&[f32]> {
        if index >= self.channels {
            return None;
        }
        let start = index * self.block_frames;
        Some(&self.data[start..start + self.valid_frames])
    }

    /// Channel count, first covered source frame, valid frame count,
    /// producing generation — the sequencing accessors.
    ///
    /// **RT contract:** RT-suitable.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.channels
    }
    /// First source frame this block covers.
    #[must_use]
    pub fn first_frame(&self) -> u64 {
        self.first_frame
    }
    /// Valid samples per channel.
    #[must_use]
    pub fn valid_frames(&self) -> usize {
        self.valid_frames
    }
    /// The producing generation.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// The faulted sub-span, when this block carries one.
    #[must_use]
    pub fn fault(&self) -> Option<BlockFault> {
        self.fault
    }

    /// Borrowed per-channel mutable views for a producer fill —
    /// allocates the slice vector (non-RT side only).
    fn channel_slices_mut(&mut self) -> Vec<&mut [f32]> {
        self.data.chunks_exact_mut(self.block_frames).collect()
    }
}

/// The sequencing unit of the prefetch data ring.
///
/// Every variant is generation-tagged; `Block`/`Fault` advance the
/// consumer's `next_frame` by a known span, `Boundary` resets it and
/// adopts a new generation, `End` marks stream exhaustion.
///
/// **RT contract:** the *item* is RT-passable by move; producing
/// items is non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum PrefetchItem {
    /// A filled PCM block — carries its own span + generation.
    Block(PcmBlock),
    /// Stream position reset under a new generation (seek/reopen).
    Boundary {
        /// The generation the stream continues under.
        generation: u64,
        /// The source frame the stream resumes at.
        first_frame: u64,
    },
    /// A faulted span — covers `[first_frame, +frames)` with no
    /// content (zeroed by the consumer, counted as faulted); a
    /// zero-frame marker records a point fault such as a fatal
    /// decode error or a failed seek.
    Fault {
        /// The producing generation.
        generation: u64,
        /// The fault class.
        kind: SourceFault,
        /// First faulted source frame.
        first_frame: u64,
        /// Faulted span length — `0` for point events.
        frames: u64,
    },
    /// Normal end of stream for its generation — exhaustion, not a
    /// fault.
    End {
        /// The generation that ended.
        generation: u64,
    },
}

/// Non-RT observability of the pipeline — relaxed-atomic counters the
/// consumer publishes every `fill` and the producer on drops.
///
/// **RT contract:** read access is RT-suitable (relaxed loads); the
/// handle side reads it non-RT for telemetry.
#[derive(Clone, Debug, Default)]
pub struct PrefetchTelemetry {
    inner: Arc<TelemetryInner>,
}

#[derive(Debug, Default)]
struct TelemetryInner {
    underrun_frames: AtomicU64,
    underrun_events: AtomicU64,
    fault_frames: AtomicU64,
    fault_events: AtomicU64,
    stale_drops: AtomicU64,
    eof_seen: AtomicU64,
    position: AtomicU64,
    generation: AtomicU64,
}

impl PrefetchTelemetry {
    /// Total frames reported as underrun across all fills.
    #[must_use]
    pub fn underrun_frames(&self) -> u64 {
        self.inner.underrun_frames.load(Ordering::Relaxed)
    }

    /// Fills that observed at least one underrun frame.
    #[must_use]
    pub fn underrun_events(&self) -> u64 {
        self.inner.underrun_events.load(Ordering::Relaxed)
    }

    /// Total fault-covered frames.
    #[must_use]
    pub fn fault_frames(&self) -> u64 {
        self.inner.fault_frames.load(Ordering::Relaxed)
    }

    /// Total fault events consumed.
    #[must_use]
    pub fn fault_events(&self) -> u64 {
        self.inner.fault_events.load(Ordering::Relaxed)
    }

    /// Items dropped because their generation was stale.
    #[must_use]
    pub fn stale_drops(&self) -> u64 {
        self.inner.stale_drops.load(Ordering::Relaxed)
    }

    /// Whether the stream's `End` marker has been consumed.
    #[must_use]
    pub fn eof_seen(&self) -> bool {
        self.inner.eof_seen.load(Ordering::Relaxed) != 0
    }

    /// The consumer's stream position (next source frame).
    #[must_use]
    pub fn position(&self) -> SourceFrame {
        SourceFrame(self.inner.position.load(Ordering::Relaxed))
    }

    /// The generation the consumer currently consumes under.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Relaxed)
    }
}

/// Non-RT control verbs for the producer.
enum PrefetchCmd {
    Seek {
        generation: u64,
        first: u64,
        /// Purge queued markers ahead of the new boundary.
        discard_pending: bool,
    },
    Reopen {
        generation: u64,
        source: Box<dyn MediaSource>,
        open: OpenFn,
        /// Seek target applied after the reopen; `None` anchors at
        /// frame 0.
        first: Option<u64>,
    },
    Quit,
}

/// Caller-supplied re-open function: fresh `MediaSource` → probed
/// `Demuxer`. Never a path — provider interchange stays caller-side.
type OpenFn = Box<dyn FnOnce(Box<dyn MediaSource>) -> Result<Box<dyn Demuxer>> + Send>;

/// What one [`PrefetchProducer::pump`] did — the drive loop's
/// observable step result.
///
/// One `pump()` call drains every queued command and then performs at
/// most one sequenced item of work, so a single call never drains the
/// pipeline: [`Pushed`](Self::Pushed) covers exactly one item reaching
/// the data ring, [`Backpressured`](Self::Backpressured) means work
/// exists but a ring had no slot — the item stays queued (markers in
/// `pending`, the block re-parked) and the caller must iterate.
/// [`Idle`](Self::Idle)/[`Ended`](Self::Ended) are the terminal
/// outcomes a manual drive loop iterates towards.
///
/// **RT contract:** non-RT only — produced by the non-RT pump.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PumpOutcome {
    /// Nothing to do (ended, waiting for commands).
    Idle,
    /// An item was pushed into the data ring — at most one per call.
    Pushed,
    /// Work exists but a ring has no slot right now — bounded wait,
    /// retry; the item was re-queued, not dropped.
    Backpressured,
    /// The stream's `End` marker was emitted; the producer idles until
    /// a seek/reopen arrives.
    Ended,
    /// `Quit` was received — the drive loop must stop.
    Quit,
}

/// The producing side of the pipeline — owns the `SourceReader`, the
/// free-block inbox, the data-ring outbox and the command inbox.
///
/// Drive it either manually ([`pump`](Self::pump) — the deterministic
/// path tests use) or by a spawned worker
/// ([`run`](Self::run)/[`Prefetch::spawn`]).
///
/// **RT contract:** non-RT only — decodes, allocates, may block on the
/// command channel in [`run`](Self::run).
pub struct PrefetchProducer {
    reader: SourceReader,
    data_tx: Producer<PrefetchItem>,
    free_rx: Consumer<PcmBlock>,
    cmd_rx: mpsc::Receiver<PrefetchCmd>,
    telemetry: PrefetchTelemetry,
    generation: u64,
    /// Markers awaiting a data-ring slot (full-ring retry queue —
    /// bounded by construction: at most a few markers ever queue).
    pending: VecDeque<PrefetchItem>,
    /// A popped-but-unfilled block kept between EOF and the next
    /// generation — the producer owns it so it never has to drop a
    /// pool block.
    parked: Option<PcmBlock>,
    stream_ended: bool,
    /// The `End` marker for the current generation was emitted —
    /// the producer idles until a new generation arrives.
    end_emitted: bool,
    quit: bool,
}

/// The producer's teardown accounting — how many pool blocks the
/// producer side reclaimed on close.
///
/// **RT contract:** non-RT only.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ProducerClose {
    /// Blocks reclaimed from the free inbox plus the parked block.
    pub free_reclaimed: usize,
}

impl PrefetchProducer {
    /// One non-blocking work step: drain every queued command, then
    /// perform at most one sequenced item — emit one queued marker,
    /// emit `End`, or fill and push one block.
    ///
    /// Command execution is unbounded per call (every queued command
    /// is applied); item production is not — [`PumpOutcome::Pushed`]
    /// covers exactly one item. [`PumpOutcome::Backpressured`] means
    /// work exists but a ring has no slot: the marker is re-queued to
    /// `pending` or the block re-parked, so `Boundary`/`Fault`/`End`
    /// markers may remain in `pending` under backpressure. A manual
    /// driver iterates `pump()` until [`PumpOutcome::Idle`]/
    /// [`PumpOutcome::Ended`] rather than assuming one call drains
    /// the pipeline.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// Propagates only command-channel failures; decode errors are
    /// converted to `Fault`/`End` markers per contract.
    pub fn pump(&mut self) -> Result<PumpOutcome> {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            self.exec_cmd(cmd);
            if self.quit {
                return Ok(PumpOutcome::Quit);
            }
        }
        // 1. Queued markers take priority — they are already
        //    sequenced work awaiting a ring slot.
        if let Some(item) = self.pending.pop_front() {
            match self.data_tx.try_push(item) {
                Ok(()) => return Ok(PumpOutcome::Pushed),
                Err(full) => {
                    self.pending.push_front(full.into_inner());
                    return Ok(PumpOutcome::Backpressured);
                }
            }
        }
        // 2. Stream ended → emit End once, then idle.
        if self.stream_ended && !self.end_emitted {
            let end = PrefetchItem::End {
                generation: self.generation,
            };
            match self.data_tx.try_push(end) {
                Ok(()) => {
                    self.end_emitted = true;
                    return Ok(PumpOutcome::Ended);
                }
                Err(full) => {
                    self.pending.push_front(full.into_inner());
                    return Ok(PumpOutcome::Backpressured);
                }
            }
        }
        if self.end_emitted {
            return Ok(PumpOutcome::Idle);
        }
        Ok(self.fill_block())
    }

    /// The blocking drive loop: wait on the command channel with a
    /// cadence, pump between commands, return the teardown accounting
    /// after `Quit`/disconnect.
    ///
    /// **RT contract:** non-RT only — `recv_timeout` blocking is the
    /// documented best-effort cadence (~1 ms).
    pub fn run(mut self) -> ProducerClose {
        loop {
            match self.cmd_rx.recv_timeout(Duration::from_millis(1)) {
                Ok(cmd) => {
                    self.exec_cmd(cmd);
                    if self.quit {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            match self.pump() {
                Ok(PumpOutcome::Quit) | Err(_) => break,
                Ok(_) => {}
            }
        }
        self.close_audit()
    }

    /// Teardown: drain the free inbox and reclaim the parked block.
    pub fn close_audit(mut self) -> ProducerClose {
        let mut free_reclaimed = 0;
        while self.free_rx.try_pop().is_ok() {
            free_reclaimed += 1;
        }
        if self.parked.take().is_some() {
            free_reclaimed += 1;
        }
        ProducerClose { free_reclaimed }
    }

    /// Executes one control command (seek/reopen/quit).
    fn exec_cmd(&mut self, cmd: PrefetchCmd) {
        match cmd {
            PrefetchCmd::Quit => self.quit = true,
            PrefetchCmd::Seek {
                generation,
                first,
                discard_pending,
            } => {
                self.generation = generation;
                self.stream_ended = false;
                self.end_emitted = false;
                if discard_pending {
                    // Opt-in purge: stale queued markers never reach
                    // the ring behind the new boundary.
                    self.pending.clear();
                }
                match self.reader.seek_source_frame(SourceFrame(first)) {
                    Ok(_) => {
                        self.pending.push_back(PrefetchItem::Boundary {
                            generation,
                            first_frame: self.reader.position().0,
                        });
                    }
                    Err(_error) => {
                        // A rejected seek keeps the old position; the
                        // new generation still re-anchors the stream so
                        // stale items die and the fault is reported.
                        self.pending.push_back(PrefetchItem::Fault {
                            generation,
                            kind: SourceFault::Unseekable,
                            first_frame: first,
                            frames: 0,
                        });
                        self.pending.push_back(PrefetchItem::Boundary {
                            generation,
                            first_frame: self.reader.position().0,
                        });
                    }
                }
            }
            PrefetchCmd::Reopen {
                generation,
                source,
                open,
                first,
            } => {
                self.generation = generation;
                self.end_emitted = false;
                match open(source).and_then(|demuxer| self.reader.reopen(demuxer)) {
                    Ok(()) => {
                        self.stream_ended = false;
                        match first {
                            None => {
                                self.pending.push_back(PrefetchItem::Boundary {
                                    generation,
                                    first_frame: 0,
                                });
                            }
                            Some(target) => {
                                match self.reader.seek_source_frame(SourceFrame(target)) {
                                    Ok(_) => {
                                        self.pending.push_back(PrefetchItem::Boundary {
                                            generation,
                                            first_frame: self.reader.position().0,
                                        });
                                    }
                                    Err(_error) => {
                                        self.pending.push_back(PrefetchItem::Fault {
                                            generation,
                                            kind: SourceFault::Unseekable,
                                            first_frame: target,
                                            frames: 0,
                                        });
                                        self.pending.push_back(PrefetchItem::Boundary {
                                            generation,
                                            first_frame: self.reader.position().0,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        self.pending.push_back(PrefetchItem::Fault {
                            generation,
                            kind: SourceFault::of_kind(error.kind()),
                            first_frame: 0,
                            frames: 0,
                        });
                        self.stream_ended = true;
                    }
                }
            }
        }
    }

    /// Pops a free block, fills it through the reader, audits it and
    /// pushes it — or records why it could not.
    fn fill_block(&mut self) -> PumpOutcome {
        if self.parked.is_none() {
            match self.free_rx.try_pop() {
                Ok(block) => self.parked = Some(block),
                Err(_) => return PumpOutcome::Backpressured,
            }
        }
        let mut block = self.parked.take().expect("parked just filled");
        block.first_frame = self.reader.position().0;
        block.generation = self.generation;
        block.fault = None;
        let block_frames = block.block_frames;
        let outcome = {
            let mut slices = block.channel_slices_mut();
            self.reader.read_planar(&mut slices, block_frames)
        };
        match outcome {
            Ok(read) => {
                block.valid_frames = read.frames;
                if read.frames == 0 {
                    // End of stream — park the unused block and let
                    // the End marker sequence out next pump.
                    self.parked = Some(block);
                    self.stream_ended = true;
                    return PumpOutcome::Ended;
                }
                audit_block(&mut block);
                self.telemetry
                    .inner
                    .position
                    .store(self.reader.position().0, Ordering::Relaxed);
                match self.data_tx.try_push(PrefetchItem::Block(block)) {
                    Ok(()) => PumpOutcome::Pushed,
                    Err(full) => {
                        let PrefetchItem::Block(block) = full.into_inner() else {
                            unreachable!("block item pushed is a block")
                        };
                        self.parked = Some(block);
                        PumpOutcome::Backpressured
                    }
                }
            }
            Err(error) => {
                // Fatal stream error — the partially-filled block is
                // ambiguous, so it is parked for reuse and the fault
                // + end sequence out.
                self.parked = Some(block);
                self.pending.push_back(PrefetchItem::Fault {
                    generation: self.generation,
                    kind: SourceFault::of_kind(error.kind()),
                    first_frame: self.reader.position().0,
                    frames: 0,
                });
                self.stream_ended = true;
                PumpOutcome::Pushed
            }
        }
    }
}

/// Detects non-finite samples in a filled block, zeroes the faulted
/// envelope and records it — the producer-side `InvalidSample` audit.
///
/// The span is a single `[first, first + frames)` envelope covering
/// the first-to-last invalid frame across channels; merging disjoint
/// runs into one envelope slightly over-marks but never fabricates.
fn audit_block(block: &mut PcmBlock) {
    let mut first = u32::MAX;
    let mut last = 0_u32;
    for channel in 0..block.channels {
        let start = channel * block.block_frames;
        for (frame, sample) in block.data[start..start + block.valid_frames]
            .iter()
            .enumerate()
        {
            if !sample.is_finite() {
                let Ok(frame) = u32::try_from(frame) else {
                    continue;
                };
                first = first.min(frame);
                last = last.max(frame + 1);
            }
        }
    }
    if first <= last {
        // Zero the faulted envelope in every channel.
        for channel in 0..block.channels {
            let start = channel * block.block_frames;
            for sample in &mut block.data[start + first as usize..start + last as usize] {
                *sample = 0.0;
            }
        }
        block.fault = Some(BlockFault {
            kind: SourceFault::InvalidSample,
            first,
            frames: last - first,
        });
    }
}

/// The RT endpoint — consumes the `PrefetchItem` stream into caller
/// destination slices.
///
/// Single-owner, `!Sync`, meant to live on the audio thread. Every
/// method is RT-suitable under the documented preconditions: the pool
/// is pre-allocated, rings are bounded, and `fill` never allocates,
/// locks, blocks or performs I/O.
///
/// **RT contract:** RT-suitable — see per-method labels.
pub struct PrefetchConsumer {
    data_rx: Consumer<PrefetchItem>,
    free_tx: Producer<PcmBlock>,
    telemetry: PrefetchTelemetry,
    channels: usize,
    /// The block currently being consumed, and how much of it is done.
    current: Option<PcmBlock>,
    cursor: usize,
    /// Unconsumed remainder of a fault span (zeros + faulted frames).
    fault_remaining: u64,
    /// The next source frame the stream will deliver.
    next_frame: u64,
    /// The generation this consumer serves.
    generation: u64,
    /// The stream's `End` marker was consumed.
    ended: bool,
}

/// The consumer's teardown accounting on [`close`](PrefetchConsumer::close).
///
/// **RT contract:** non-RT only — consumed at pipeline teardown.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsumerClose {
    /// Blocks returned to the free ring during the drain.
    pub recycled: usize,
    /// Pool blocks that could not be returned (always 0 by the pool
    /// invariant — a nonzero value is a leak report, not silent loss).
    pub dropped: usize,
}

/// What one [`PrefetchConsumer::discard_pending`] call drained — the
/// bounded discard's accounting.
///
/// `items` counts ring items processed (blocks and markers).
/// `blocks` counts `PcmBlock`s returned to the free ring, including
/// a partially-consumed in-flight block. `boundary` is the landed
/// `first_frame` of the first `Boundary` consumed: the consumer
/// adopted that generation and re-anchored `next_frame` there, and
/// items behind the boundary stayed queued. `None` means no
/// boundary was reached — the drain consumed the backlog only.
///
/// **RT contract:** RT-suitable — `Copy` plain data.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DiscardReport {
    /// Ring items processed (blocks and markers).
    pub items: usize,
    /// Pool blocks returned to the free ring, including an in-flight
    /// block.
    pub blocks: usize,
    /// Landed boundary frame when a `Boundary` was consumed.
    pub boundary: Option<u64>,
}

impl PrefetchConsumer {
    /// Fills `dst` with up to `dst.len()`-per-channel frames starting
    /// at `expected_first`, classifying every span per the fault
    /// contract.
    ///
    /// `expected_first` is a *stream-position assertion* in source
    /// frames: the caller declares the `next_frame` it believes the
    /// stream is at — typically the position it last observed plus
    /// the frames it has since consumed. It is not an engine-frame
    /// or output-frame expectation; engine-side mapping stays with
    /// the caller. When the assertion does not match the stream's
    /// `next_frame` the call is a `mismatch`: nothing is consumed
    /// and the destination is zero-filled (a stale-plan read is
    /// refused, not served); the caller resynchronizes via
    /// `seek`/`discard_pending` or drops the call.
    ///
    /// `dst` must have exactly the configured channel count; every
    /// slice's length is the requested frame count. Violating the
    /// channel count is a contract bug and panics — it is checked in
    /// constant time, never a silent remap.
    ///
    /// **RT contract:** RT-suitable — no allocation, no locks, no
    /// blocking, no OS I/O. `dst` zero-fill is the declared bounded
    /// fallback for every non-content span.
    ///
    /// # Panics
    ///
    /// `dst.len()` differs from the configured channel count.
    pub fn fill(
        &mut self,
        dst: &mut [&mut [f32]],
        expected_first: SourceFrame,
        _ctx: &RtContext,
    ) -> FillReport {
        assert_eq!(
            dst.len(),
            self.channels,
            "prefetch fill: channel count mismatch"
        );
        let frames = dst.first().map_or(0, |d| d.len());
        // The declared bounded fallback: every span starts as silence
        // and only content overwrites it.
        for channel in dst.iter_mut() {
            channel.fill(0.0);
        }
        let mut report = FillReport {
            requested: frames,
            ..FillReport::default()
        };
        if expected_first.0 != self.next_frame {
            report.mismatch = true;
            self.publish(&report);
            return report;
        }
        let mut done = 0usize;
        while done < frames {
            // Consume a pending fault span first — it covers
            // positions just like a block does.
            if self.fault_remaining > 0 {
                let take = usize::try_from(self.fault_remaining)
                    .unwrap_or(usize::MAX)
                    .min(frames - done);
                self.fault_remaining -= take as u64;
                self.next_frame += take as u64;
                done += take;
                report.faulted += take;
                continue;
            }
            if self.current.is_none() {
                match self.step() {
                    Step::Block => {}
                    Step::Faulted(kind) => {
                        report.faults.bump(kind);
                        continue;
                    }
                    Step::Underrun => {
                        report.underrun += frames - done;
                        break;
                    }
                    Step::Ended => {
                        report.eof = true;
                        break;
                    }
                }
            }
            if let Some(block) = self.current.take() {
                done += self.copy_block(&block, dst, done, frames, &mut report);
                if self.cursor < block.valid_frames {
                    self.current = Some(block);
                } else {
                    self.return_block(block);
                }
            }
        }
        self.publish(&report);
        report
    }

    /// Tears down the consumer: drains the data ring, returning every
    /// block to the pool (free-ring capacity equals the pool size, so
    /// a full ring cannot strand a block).
    ///
    /// **RT contract:** non-RT only — teardown.
    #[must_use]
    pub fn close(mut self) -> ConsumerClose {
        let mut close = ConsumerClose::default();
        if let Some(block) = self.current.take() {
            if self.free_tx.try_push(block).is_err() {
                close.dropped += 1;
            } else {
                close.recycled += 1;
            }
        }
        while let Ok(item) = self.data_rx.try_pop() {
            if let PrefetchItem::Block(block) = item {
                if self.free_tx.try_push(block).is_err() {
                    close.dropped += 1;
                } else {
                    close.recycled += 1;
                }
            }
        }
        close
    }

    /// Discards queued stream elements up to and including the first
    /// `Boundary` — the RT-safe drain half of sample-exact
    /// re-anchoring, paired with
    /// [`PrefetchHandle::seek_discard_pending`].
    ///
    /// Every popped item is processed by the same sequencing rules
    /// [`fill`](Self::fill) applies: blocks return to the free ring
    /// (generation-mismatched ones count as stale drops), matching
    /// `Fault`/`End` markers update fault/end state, and the first
    /// `Boundary` is *consumed* — the consumer adopts its generation
    /// and `first_frame`, clears fault/end state, and stops. Items
    /// behind that boundary stay queued for the next `fill`.
    ///
    /// A partially-consumed in-flight block is discarded too. Every
    /// iteration is bounded O(1) with no allocation, locks or
    /// blocking; the work per call is bounded by ring occupancy —
    /// data-ring capacity plus one in-flight block, plus any items a
    /// running producer feeds during the drain itself.
    ///
    /// Residual limit: elements that arrive *after* this call but
    /// *before* the producer's boundary still play through — the
    /// generation model is boundary-driven, so a drain cannot
    /// discard items it has not seen. For strict re-anchoring call
    /// it once the boundary is known queued (e.g. after
    /// `seek_discard_pending` has been pumped).
    ///
    /// **RT contract:** RT-suitable.
    #[must_use]
    pub fn discard_pending(&mut self, _ctx: &RtContext) -> DiscardReport {
        let mut report = DiscardReport::default();
        if let Some(block) = self.current.take() {
            if block.generation != self.generation {
                self.stale();
            }
            self.return_block(block);
            report.blocks += 1;
        }
        loop {
            let Ok(item) = self.data_rx.try_pop() else {
                return report;
            };
            report.items += 1;
            match item {
                PrefetchItem::Block(block) => {
                    if block.generation != self.generation {
                        self.stale();
                    }
                    self.return_block(block);
                    report.blocks += 1;
                }
                PrefetchItem::Boundary {
                    generation,
                    first_frame,
                } => {
                    self.generation = generation;
                    self.next_frame = first_frame;
                    self.fault_remaining = 0;
                    self.ended = false;
                    self.cursor = 0;
                    report.boundary = Some(first_frame);
                    self.telemetry
                        .inner
                        .position
                        .store(self.next_frame, Ordering::Relaxed);
                    self.telemetry
                        .inner
                        .generation
                        .store(self.generation, Ordering::Relaxed);
                    return report;
                }
                PrefetchItem::Fault {
                    generation,
                    kind: _,
                    first_frame,
                    frames,
                } => {
                    if generation != self.generation {
                        self.stale();
                        continue;
                    }
                    self.telemetry
                        .inner
                        .fault_events
                        .fetch_add(1, Ordering::Relaxed);
                    if frames > 0 {
                        if first_frame == self.next_frame {
                            self.fault_remaining += frames;
                        } else {
                            self.stale();
                        }
                    }
                }
                PrefetchItem::End { generation } => {
                    if generation == self.generation {
                        self.ended = true;
                        self.telemetry.inner.eof_seen.store(1, Ordering::Relaxed);
                    } else {
                        self.stale();
                    }
                }
            }
        }
    }

    /// The current stream position — the next source frame a
    /// non-mismatch `fill` would serve.
    ///
    /// **RT contract:** RT-suitable.
    #[must_use]
    pub fn position(&self) -> SourceFrame {
        SourceFrame(self.next_frame)
    }

    /// The generation the consumer currently serves.
    ///
    /// **RT contract:** RT-suitable.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Pulls the next sequencing step: pops the data ring until a
    /// usable block arrives, updating state on every marker.
    fn step(&mut self) -> Step {
        loop {
            match self.data_rx.try_pop() {
                Err(_) => {
                    return if self.ended {
                        Step::Ended
                    } else {
                        Step::Underrun
                    };
                }
                Ok(PrefetchItem::Block(block)) => {
                    if block.generation != self.generation {
                        self.stale();
                        self.return_block(block);
                        continue;
                    }
                    if block.first_frame > self.next_frame {
                        // A sequence gap — missing positions are
                        // covered as a fault span, never skipped
                        // silently.
                        self.fault_remaining += block.first_frame - self.next_frame;
                        self.telemetry
                            .inner
                            .fault_events
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    let skip = self.next_frame.saturating_sub(block.first_frame);
                    if skip >= block.valid_frames as u64 {
                        // Fully-stale block.
                        self.stale();
                        self.return_block(block);
                        continue;
                    }
                    self.cursor = usize::try_from(skip).unwrap_or(usize::MAX);
                    self.current = Some(block);
                    return Step::Block;
                }
                Ok(PrefetchItem::Boundary {
                    generation,
                    first_frame,
                }) => {
                    self.generation = generation;
                    self.next_frame = first_frame;
                    self.fault_remaining = 0;
                    self.ended = false;
                }
                Ok(PrefetchItem::Fault {
                    generation,
                    kind,
                    first_frame,
                    frames,
                }) => {
                    if generation != self.generation {
                        self.stale();
                        continue;
                    }
                    self.telemetry
                        .inner
                        .fault_events
                        .fetch_add(1, Ordering::Relaxed);
                    if frames > 0 {
                        if first_frame == self.next_frame {
                            self.fault_remaining += frames;
                        } else {
                            // Mis-sequenced fault — report as a point
                            // event rather than corrupting position.
                            self.stale();
                        }
                    }
                    return Step::Faulted(kind);
                }
                Ok(PrefetchItem::End { generation }) => {
                    if generation == self.generation {
                        self.ended = true;
                    } else {
                        self.stale();
                    }
                }
            }
        }
    }

    /// Copies up to `frames - done` valid frames from `block` into
    /// `dst` at `done`, splitting around the block's fault span.
    /// Returns frames consumed.
    fn copy_block(
        &mut self,
        block: &PcmBlock,
        dst: &mut [&mut [f32]],
        done: usize,
        frames: usize,
        report: &mut FillReport,
    ) -> usize {
        let available = block.valid_frames - self.cursor;
        let take = available.min(frames - done);
        let mut i = 0usize;
        while i < take {
            let pos = self.cursor + i;
            let (is_fault, run) = match block.fault {
                Some(fault)
                    if pos >= fault.first as usize
                        && pos < (fault.first + fault.frames) as usize =>
                {
                    (
                        true,
                        ((fault.first + fault.frames) as usize - pos).min(take - i),
                    )
                }
                Some(fault) if pos < fault.first as usize => {
                    (false, (fault.first as usize - pos).min(take - i))
                }
                _ => (false, take - i),
            };
            if is_fault {
                if pos == block.fault.map_or(usize::MAX, |f| f.first as usize) {
                    report
                        .faults
                        .bump(block.fault.map_or(SourceFault::Unsupported, |f| f.kind));
                }
                report.faulted += run;
            } else {
                for (channel, out) in dst.iter_mut().enumerate() {
                    let start = channel * block.block_frames;
                    out[done + i..done + i + run]
                        .copy_from_slice(&block.data[start + pos..start + pos + run]);
                }
                report.written += run;
            }
            i += run;
        }
        self.cursor += take;
        self.next_frame += take as u64;
        take
    }

    /// Hands a consumed block back to the pool — infallible by the
    /// pool-size == ring-capacity invariant; a failure is a leak
    /// report, never a panic on RT.
    fn return_block(&mut self, block: PcmBlock) {
        if self.free_tx.try_push(block).is_err() {
            // Impossible by the pool invariant (free capacity equals
            // the pool size, so every outstanding block always has a
            // slot); a failure is recorded, not silently dropped.
            self.stale();
        }
    }

    fn stale(&mut self) {
        self.telemetry
            .inner
            .stale_drops
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Publishes a fill's counters to telemetry.
    fn publish(&mut self, report: &FillReport) {
        let inner = &self.telemetry.inner;
        inner
            .underrun_frames
            .fetch_add(report.underrun as u64, Ordering::Relaxed);
        if report.underrun > 0 {
            inner.underrun_events.fetch_add(1, Ordering::Relaxed);
        }
        inner
            .fault_frames
            .fetch_add(report.faulted as u64, Ordering::Relaxed);
        if report.eof {
            inner.eof_seen.store(1, Ordering::Relaxed);
        }
        inner.position.store(self.next_frame, Ordering::Relaxed);
        inner.generation.store(self.generation, Ordering::Relaxed);
    }
}

enum Step {
    Block,
    /// A `Fault` marker was consumed — the event kind is returned so
    /// the fill can attribute it; any span it carries is consumed
    /// through `fault_remaining`.
    Faulted(SourceFault),
    Underrun,
    Ended,
}

/// Pool/ring configuration for [`Prefetch::new`].
///
/// `channels`/`sample_rate` must equal the reader stream's canonical
/// format — validated at construction. `block_count` fixes the pool
/// (and therefore the free-ring capacity); `block_frames` fixes each
/// block's per-channel capacity.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PrefetchConfig {
    /// Per-channel samples per block.
    pub block_frames: usize,
    /// Total pool size (max blocks in flight + held + queued).
    pub block_count: usize,
    /// Channel count — must match the stream's canonical layout.
    pub channels: usize,
    /// Sample rate — must match the stream's canonical rate.
    pub sample_rate: u32,
}

/// The pipeline owner pair: the non-RT control [`PrefetchHandle`]
/// plus the RT [`PrefetchConsumer`].
///
/// **RT contract:** `handle` is non-RT only; `consumer` is the RT
/// endpoint.
pub struct Prefetch {
    /// Seek/reopen/close + telemetry — the non-RT control surface.
    pub handle: PrefetchHandle,
    /// The RT fill endpoint — move it to the audio thread.
    pub consumer: PrefetchConsumer,
}

impl Prefetch {
    /// Builds the pipeline without a worker — the manual-drive
    /// constructor: the returned `PrefetchProducer` is pumped by the
    /// caller (deterministic tests, or an external scheduler).
    ///
    /// **RT contract:** non-RT only — allocates the pool and rings.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::InvalidValue`] — `config` disagrees with the
    /// reader's canonical format, or a zero dimension was supplied.
    #[allow(clippy::type_complexity)]
    pub fn manual(
        reader: SourceReader,
        config: PrefetchConfig,
    ) -> Result<(PrefetchHandle, PrefetchProducer, PrefetchConsumer)> {
        validate_config(&reader, config)?;
        let (data_tx, data_rx) = channel(config.block_count + MARKER_SLACK);
        let (free_tx, free_rx) = channel(config.block_count);
        let mut free_tx = free_tx;
        for _ in 0..config.block_count {
            let block = PcmBlock::new(config.channels, config.block_frames);
            debug_assert!(free_tx.try_push(block).is_ok());
        }
        let telemetry = PrefetchTelemetry::default();
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let producer = PrefetchProducer {
            reader,
            data_tx,
            free_rx,
            cmd_rx,
            telemetry: telemetry.clone(),
            generation: 1,
            pending: VecDeque::new(),
            parked: None,
            stream_ended: false,
            end_emitted: false,
            quit: false,
        };
        let handle = PrefetchHandle {
            cmd_tx,
            telemetry: telemetry.clone(),
            next_gen: 1,
            worker: None,
            seekable: producer_seekable(&producer),
            block_count: config.block_count,
        };
        let consumer = PrefetchConsumer {
            data_rx,
            free_tx,
            telemetry,
            channels: config.channels,
            current: None,
            cursor: 0,
            fault_remaining: 0,
            next_frame: 0,
            generation: 1,
            ended: false,
        };
        Ok((handle, producer, consumer))
    }

    /// Builds the pipeline and spawns the producer worker thread —
    /// the standard construction.
    ///
    /// **RT contract:** non-RT only — allocates and spawns a thread.
    ///
    /// # Errors
    ///
    /// Same as [`manual`](Self::manual), plus
    /// [`ErrorKind::Io`] when the thread cannot be spawned.
    pub fn spawn(reader: SourceReader, config: PrefetchConfig) -> Result<Prefetch> {
        let (mut handle, producer, consumer) = Self::manual(reader, config)?;
        let worker = std::thread::Builder::new()
            .name("ruviform-prefetch".to_owned())
            .spawn(move || producer.run())
            .map_err(|_| Error::from_kind(ErrorKind::Io))?;
        handle.worker = Some(worker);
        Ok(Prefetch { handle, consumer })
    }
}

fn validate_config(reader: &SourceReader, config: PrefetchConfig) -> Result<()> {
    let meta = reader.metadata();
    let channels = usize::try_from(meta.canonical.layout.channels())
        .map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
    if config.block_frames == 0
        || config.block_count == 0
        || config.channels != channels
        || config.sample_rate != meta.canonical.sample_rate.get()
    {
        return Err(Error::from_kind(ErrorKind::InvalidValue));
    }
    Ok(())
}

fn producer_seekable(producer: &PrefetchProducer) -> Fact<()> {
    producer.reader.metadata().seekable
}

/// The non-RT control surface: seek/reopen/close commands plus
/// telemetry reads.
///
/// **RT contract:** non-RT only — sends may allocate-free but the
/// handle is an owner object for the control thread, not the audio
/// thread.
pub struct PrefetchHandle {
    cmd_tx: mpsc::Sender<PrefetchCmd>,
    telemetry: PrefetchTelemetry,
    /// Last assigned generation — bumps on every seek/reopen.
    next_gen: u64,
    worker: Option<JoinHandle<ProducerClose>>,
    /// The declared seek capability captured at pipeline build —
    /// reopened sources re-declare through telemetry only.
    seekable: Fact<()>,
    /// The pool size — the audit's expected reclaim count.
    block_count: usize,
}

/// Teardown accounting for [`PrefetchHandle::close`].
///
/// **RT contract:** non-RT only.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct PrefetchAudit {
    /// Total pool blocks allocated at construction.
    pub block_count: usize,
    /// Blocks reclaimed to the pool on teardown (producer free inbox
    /// + consumer recycle).
    pub reclaimed: usize,
    /// Pool blocks that could not be returned — nonzero is a leak
    /// report, never silent.
    pub dropped: usize,
    /// Whether the worker thread was joined.
    pub worker_joined: bool,
}

impl PrefetchHandle {
    /// Requests a source-frame seek: bumps the generation and queues
    /// the command — the producer re-anchors the stream
    /// asynchronously; watch [`PrefetchTelemetry::generation`]/
    /// [`position`](PrefetchTelemetry::position) for convergence.
    ///
    /// A `Fact::Unsupported` seekable reports
    /// [`ErrorKind::Unsupported`] immediately, without consuming a
    /// generation.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Unsupported`] — the source cannot seek.
    /// [`ErrorKind::Io`] — the control channel is gone (producer
    /// stopped).
    pub fn seek(&mut self, first: SourceFrame) -> Result<()> {
        self.seek_cmd(first, false)
    }

    /// Requests a source-frame seek that also purges the producer's
    /// queued markers: the seek boundary sequences out without stale
    /// `Boundary`/`Fault`/`End` markers from earlier commands
    /// preceding it.
    ///
    /// Only the producer `pending` queue is purged — items already
    /// pushed into the data ring (blocks *and* markers from earlier
    /// commands) still sequence ahead of the boundary: blocks play
    /// through, ring-resident markers are consumed or stale-dropped
    /// per the usual rules. Pair this with
    /// [`PrefetchConsumer::discard_pending`] to drain the consumer's
    /// backlog when sample-exact re-anchoring must not wait for
    /// play-through.
    ///
    /// A `Fact::Unsupported` seekable reports
    /// [`ErrorKind::Unsupported`] immediately, without consuming a
    /// generation.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Unsupported`] — the source cannot seek.
    /// [`ErrorKind::Io`] — the control channel is gone (producer
    /// stopped).
    pub fn seek_discard_pending(&mut self, first: SourceFrame) -> Result<()> {
        self.seek_cmd(first, true)
    }

    fn seek_cmd(&mut self, first: SourceFrame, discard_pending: bool) -> Result<()> {
        if matches!(self.seekable, Fact::Unsupported) {
            return Err(Error::from_kind(ErrorKind::Unsupported));
        }
        self.next_gen += 1;
        self.cmd_tx
            .send(PrefetchCmd::Seek {
                generation: self.next_gen,
                first: first.0,
                discard_pending,
            })
            .map_err(|_| Error::from_kind(ErrorKind::Io))
    }

    /// Requests a source reopen: the producer builds a fresh
    /// `SourceReader` on `open(source)` under a new generation.
    ///
    /// `open` is the caller's own construction — a `Box<dyn
    /// MediaSource>` in, a probed `Box<dyn Demuxer>` out; the API
    /// never names files, paths or providers.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] — the control channel is gone.
    pub fn reopen(&mut self, source: Box<dyn MediaSource>, open: OpenFn) -> Result<()> {
        self.reopen_cmd(source, open, None)
    }

    /// Requests a source reopen that lands directly on `first`: the
    /// producer reopens, seeks to `first`, and emits a single
    /// `Boundary` at the actual landed position — the fused form of
    /// `reopen` + `seek` for mid-stream source replacement.
    ///
    /// If the post-reopen seek fails, an [`SourceFault::Unseekable`]
    /// fault is queued ahead of a `Boundary` at the reopened
    /// position; per the generation-adoption model the consumer
    /// drops the new-generation fault ahead of that boundary as
    /// stale, so the observable outcome is the re-anchor plus a
    /// stale-drop count — the same reporting shape as `seek` on a
    /// rejected seek. Whether the replacement source is seekable is
    /// not pre-checked.
    ///
    /// If `open`/`reopen` itself fails, the existing `reopen`
    /// failure path applies unchanged: a `Fault` marker and stream
    /// end — no `Boundary` is emitted, so the generation is never
    /// adopted.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] — the control channel is gone.
    pub fn reopen_at(
        &mut self,
        source: Box<dyn MediaSource>,
        open: OpenFn,
        first: SourceFrame,
    ) -> Result<()> {
        self.reopen_cmd(source, open, Some(first.0))
    }

    fn reopen_cmd(
        &mut self,
        source: Box<dyn MediaSource>,
        open: OpenFn,
        first: Option<u64>,
    ) -> Result<()> {
        self.next_gen += 1;
        self.cmd_tx
            .send(PrefetchCmd::Reopen {
                generation: self.next_gen,
                source,
                open,
                first,
            })
            .map_err(|_| Error::from_kind(ErrorKind::Io))
    }

    /// Tears the pipeline down: sends `Quit`, joins the worker (when
    /// spawned), drains the consumer's ring back to the pool, and
    /// reports the block accounting — the leak audit of the
    /// source-lifecycle contract.
    ///
    /// **RT contract:** non-RT only — joins the worker.
    #[must_use]
    pub fn close(mut self, consumer: PrefetchConsumer) -> PrefetchAudit {
        let _ = self.cmd_tx.send(PrefetchCmd::Quit);
        let mut audit = PrefetchAudit {
            block_count: self.block_count,
            worker_joined: false,
            ..PrefetchAudit::default()
        };
        if let Some(worker) = self.worker.take()
            && let Ok(close) = worker.join()
        {
            audit.worker_joined = true;
            audit.reclaimed += close.free_reclaimed;
        }
        let close = consumer.close();
        audit.reclaimed += close.recycled;
        audit.dropped += close.dropped;
        audit
    }

    /// The telemetry counters — pipeline observability for the
    /// non-RT side.
    ///
    /// **RT contract:** non-RT only — relaxed reads.
    #[must_use]
    pub fn telemetry(&self) -> &PrefetchTelemetry {
        &self.telemetry
    }
}
