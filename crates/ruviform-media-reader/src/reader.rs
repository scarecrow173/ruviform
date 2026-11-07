//! [`SourceReader`] — sequential, source-frame-addressed decode over a
//! [`Demuxer`]/[`Decoder`] pair.
//!
//! The reader owns both ports for one selected stream: it feeds the
//! stream's packets to the decoder, pulls decoded frames, and exposes
//! two consumption shapes — [`next_frame`](SourceReader::next_frame)
//! for whole-frame observation (canonical or native per the open
//! [`DecodeMode`]) and [`read_planar`](SourceReader::read_planar) for
//! contiguous canonical `f32` windows.
//!
//! Positions are **source frames** — decoded-sample indices at the
//! stream's rate — anchored by frame timestamps: each emitted frame's
//! pts is converted into the canonical rate's base (`Estimated` is
//! reported when a frame carries no resolvable timestamp). Seeking is
//! the composed operation the boundary deliberately leaves to the
//! caller: demuxer seek → decoder flush → decode-discard forward to
//! the exact target, with the real landing reported by
//! [`SeekReceipt`].
//!
//! **RT contract:** non-RT only — every method may allocate, block on
//! source I/O and run unbounded codec work.

use ruviform_media_core::{Fact, MediaTime, Rounding, StreamId, TimeBase, TimestampProvenance};
use ruviform_media_io::{DecodeMode, Decoder, Demuxer, Error, ErrorKind, Frame, Result};

use crate::mapping::SourceFrame;
use crate::metadata::{SourceMetadata, stream_metadata};

/// The result of a [`SourceReader::read_planar`] window pull.
///
/// `frames` is the count of real decoded samples written per channel;
/// `eof` marks that the stream's end was reached inside this read —
/// a destination region beyond `frames` was left untouched so the
/// caller applies its own bounded fallback. EOF is a state report,
/// never an error and never content.
///
/// **RT contract:** non-RT only — produced by the non-RT pump.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadOutcome {
    /// Decoded samples written per channel.
    pub frames: usize,
    /// The stream's end was reached: no further content exists.
    pub eof: bool,
}

/// What a [`SourceReader::seek_source_frame`] actually landed on.
///
/// `landed` is the pts-derived source frame of the first sample the
/// reader will deliver after the seek. When the demuxer lands *past*
/// the request (`landed > requested`) the reader reports the gap
/// rather than pretending the request landed — positions are never
/// silently fabricated.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SeekReceipt {
    /// The requested target.
    pub requested: SourceFrame,
    /// The first source frame that will be delivered.
    pub landed: SourceFrame,
    /// Where `landed` came from: `Carried`/`Derived` when a frame
    /// timestamp anchored it, `Estimated` when extrapolated or when
    /// the stream ended before any frame landed.
    pub landing_provenance: TimestampProvenance,
}

/// Sequential source-frame-addressed reader over one stream.
///
/// Construct with [`SourceReader::open`]; the demuxer is held so the
/// reader drives the whole packet→frame pipeline itself — `'static`
/// ownership for the common case, or a shorter `'a` bound when the
/// boxed demuxer is a reborrow (e.g. `impl Demuxer for &mut D` inside
/// a scope-bound callback). The selected stream's metadata is
/// available through [`metadata`](Self::metadata) and never goes stale
/// — a [`reopen`](Self::reopen) rebuilds it under the caller's control.
///
/// **RT contract:** non-RT only — every method may allocate, block on
/// source I/O and run unbounded codec work.
pub struct SourceReader<'a> {
    demuxer: Box<dyn Demuxer + 'a>,
    decoder: Box<dyn Decoder>,
    stream: StreamId,
    mode: DecodeMode,
    meta: SourceMetadata,
    /// The currently-open decoded frame plus how much of it was
    /// already consumed — whole frames cross the boundary one at a
    /// time while callers consume sample windows.
    pending: Option<Pending>,
    /// Source frame index of the next sample `read_planar` delivers /
    /// the next pulled frame begins at.
    position: u64,
    /// Whether `position`'s last anchor came from a real timestamp.
    position_provenance: TimestampProvenance,
    /// Decoder was sent `drain` — end of input.
    drained: bool,
    /// Stream fully consumed.
    finished: bool,
}

/// A decoded frame held between `read_planar` calls.
struct Pending {
    frame: Frame,
    /// Source-frame index of the frame's first sample.
    anchor: u64,
    /// Samples already consumed (per-channel frame units).
    cursor: usize,
}

impl<'a> SourceReader<'a> {
    /// Opens a reader over `stream` of an already-probed `demuxer`,
    /// building the decoder in `mode` through
    /// [`Demuxer::make_decoder`].
    ///
    /// `demuxer` is `Box<dyn Demuxer + 'a>` — an owned `'static` box
    /// infers `SourceReader<'static>` as before, while a box built from
    /// a reborrow (`Box::new(&mut *d)` via `impl Demuxer for &mut D`)
    /// yields a `SourceReader` bound to that borrow's lifetime.
    ///
    /// **RT contract:** non-RT only — probes metadata, allocates codec
    /// state.
    ///
    /// # Errors
    ///
    /// * [`ErrorKind::InvalidValue`] — `stream` names no stream, or the
    ///   stream is not decodable (`StreamFacts::decodable`).
    /// * [`ErrorKind::Unsupported`] — the stream is not audio, or the
    ///   backend cannot build the requested decoder.
    pub fn open(
        demuxer: Box<dyn Demuxer + 'a>,
        stream: StreamId,
        mode: DecodeMode,
    ) -> Result<Self> {
        let meta = stream_metadata(demuxer.as_ref(), stream, mode)?;
        if !matches!(meta.stream.facts.decodable, Fact::Known(())) {
            return Err(Error::from_kind(ErrorKind::InvalidValue));
        }
        let decoder = demuxer.make_decoder(stream, mode)?;
        Ok(Self {
            demuxer,
            decoder,
            stream,
            mode,
            meta,
            pending: None,
            position: 0,
            position_provenance: TimestampProvenance::Estimated,
            drained: false,
            finished: false,
        })
    }

    /// The stream's metadata projection — fixed at open, refreshed on
    /// [`reopen`](Self::reopen).
    ///
    /// **RT contract:** non-RT only — borrow.
    #[must_use]
    pub fn metadata(&self) -> &SourceMetadata {
        &self.meta
    }

    /// The source-frame index of the next sample to be delivered —
    /// pts-anchored (`position_provenance` reports the anchor class).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn position(&self) -> SourceFrame {
        SourceFrame(self.position)
    }

    /// Whether the next-position anchor is a real timestamp or an
    /// estimate.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn position_provenance(&self) -> TimestampProvenance {
        self.position_provenance
    }

    /// `true` once the stream's last sample has been delivered — the
    /// distinguishable EOF state.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Pulls the next decoded [`Frame`] in open-mode order —
    /// `Frame::Audio` for [`DecodeMode::Canonical`],
    /// `Frame::NativeAudio` for [`DecodeMode::Native`].
    ///
    /// `Ok(None)` is the normal end-of-stream report; the reader is
    /// then [`finished`](Self::finished) and stays empty until a seek
    /// or reopen.
    ///
    /// **RT contract:** non-RT only — demux + decode work.
    ///
    /// # Errors
    ///
    /// Propagates the boundary's classified errors: truncated input
    /// [`ErrorKind::UnexpectedEos`], corrupt data
    /// [`ErrorKind::InvalidData`], source failures [`ErrorKind::Io`].
    pub fn next_frame(&mut self) -> Result<Option<Frame>> {
        if let Some(pending) = self.pending.take() {
            return Ok(Some(pending.frame));
        }
        loop {
            if let Some(frame) = self.decoder.receive_frame()? {
                self.anchor_frame(&frame);
                return Ok(Some(frame));
            }
            if self.drained {
                self.finished = true;
                return Ok(None);
            }
            match self.demuxer.read_packet()? {
                Some(packet) if packet.stream == self.stream => {
                    self.decoder.send_packet(packet)?;
                }
                Some(_) => {}
                None => {
                    self.decoder.drain()?;
                    self.drained = true;
                }
            }
        }
    }

    /// Reads up to `frames` contiguous canonical `f32` samples into
    /// `dst` (one slice per channel — planar output).
    ///
    /// `dst.len()` must equal the stream's channel count and every
    /// slice must be at least `frames` long; violations are
    /// [`ErrorKind::InvalidValue`]. The call may write fewer samples
    /// than requested only at end of stream — `ReadOutcome.eof` marks
    /// that case, and the unwritten tail of `dst` is left untouched.
    ///
    /// [`DecodeMode::Native`] readers report
    /// [`ErrorKind::Unsupported`] — the planar pull is the canonical
    /// representation only; native consumers pull whole frames.
    ///
    /// **RT contract:** non-RT only — may decode and allocate.
    ///
    /// # Errors
    ///
    /// Boundary errors plus [`ErrorKind::InvalidValue`] (bad `dst`
    /// shape) and [`ErrorKind::Unsupported`] (native mode).
    pub fn read_planar(&mut self, dst: &mut [&mut [f32]], frames: usize) -> Result<ReadOutcome> {
        if !matches!(self.mode, DecodeMode::Canonical) {
            return Err(Error::from_kind(ErrorKind::Unsupported));
        }
        let channels = usize::try_from(self.meta.canonical.layout.channels())
            .map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
        if dst.len() != channels || dst.iter().any(|d| d.len() < frames) {
            return Err(Error::from_kind(ErrorKind::InvalidValue));
        }
        let mut written = 0usize;
        while written < frames {
            let Some(pending) = self.pending.as_mut() else {
                match self.next_frame()? {
                    Some(frame @ Frame::Audio(_)) => {
                        let anchor = self.frame_anchor(&frame).unwrap_or(self.position);
                        self.pending = Some(Pending {
                            frame,
                            anchor,
                            cursor: 0,
                        });
                        continue;
                    }
                    Some(_other) => {
                        // A canonical decoder emitted a non-audio
                        // frame — contract violation, reported not
                        // silently skipped.
                        return Err(Error::from_kind(ErrorKind::InvalidData));
                    }
                    None => break,
                }
            };
            let Frame::Audio(audio) = &pending.frame else {
                return Err(Error::from_kind(ErrorKind::InvalidData));
            };
            let buffer = audio.buffer();
            if buffer.channels() != channels {
                return Err(Error::from_kind(ErrorKind::Unsupported));
            }
            let available = buffer.frames() - pending.cursor;
            let take = available.min(frames - written);
            for (channel, out) in dst.iter_mut().enumerate() {
                let src = buffer
                    .channel(channel)
                    .ok_or_else(|| Error::from_kind(ErrorKind::InvalidData))?;
                out[written..written + take]
                    .copy_from_slice(&src[pending.cursor..pending.cursor + take]);
            }
            pending.cursor += take;
            written += take;
            self.position = pending.anchor + pending.cursor as u64;
            if pending.cursor >= buffer.frames() {
                self.pending = None;
            }
        }
        Ok(ReadOutcome {
            frames: written,
            eof: self.finished && written < frames,
        })
    }

    /// Frame-accurate source-frame seek: demuxer seek → decoder flush
    /// → decode-discard forward to `target`.
    ///
    /// `metadata().seekable == Fact::Unsupported` reports
    /// [`ErrorKind::Unsupported`] without touching the position; a
    /// `NotDeclared` capability is attempted (not declared is not a
    /// refusal). The receipt reports the real landing — including
    /// `landed > requested` when the demuxer could not land early
    /// enough.
    ///
    /// **RT contract:** non-RT only — demuxer seek + decoder flush +
    /// bounded decode-discard.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Unsupported`] (unseekable),
    /// [`ErrorKind::InvalidValue`]/[`ErrorKind::Io`] (demuxer seek),
    /// [`ErrorKind::TimeOverflow`] (frame→time conversion).
    pub fn seek_source_frame(&mut self, target: SourceFrame) -> Result<SeekReceipt> {
        if matches!(self.meta.seekable, Fact::Unsupported) {
            return Err(Error::from_kind(ErrorKind::Unsupported));
        }
        let base = TimeBase::sample_rate(self.meta.canonical.sample_rate.get())
            .map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
        let ticks =
            i64::try_from(target.0).map_err(|_| Error::from_kind(ErrorKind::TimeOverflow))?;
        self.demuxer.seek(MediaTime::new(ticks, base))?;
        self.decoder.flush()?;
        self.pending = None;
        self.drained = false;
        self.finished = false;

        // Pump until the first post-seek frame reports where we
        // actually landed — or the stream ends first.
        if let Some(frame) = self.next_frame()? {
            let anchor = self.frame_anchor(&frame);
            let (landed, provenance) = match anchor {
                Some(a) => (a, Self::frame_provenance(&frame)),
                None => (target.0, TimestampProvenance::Estimated),
            };
            // Canonical windows discard forward inside the
            // landing frame; native whole-frame delivery keeps
            // the frame whole and reports the real landing.
            let skip = if landed <= target.0 && matches!(self.mode, DecodeMode::Canonical) {
                target.0 - landed
            } else {
                0
            };
            self.position = landed + skip;
            self.position_provenance = provenance;
            self.hold_frame(frame, skip);
            Ok(SeekReceipt {
                requested: target,
                landed: SourceFrame(landed),
                landing_provenance: provenance,
            })
        } else {
            // Stream ended before any frame landed — position
            // holds at the request and EOF is reported.
            self.position = target.0;
            self.position_provenance = TimestampProvenance::Estimated;
            Ok(SeekReceipt {
                requested: target,
                landed: target,
                landing_provenance: TimestampProvenance::Estimated,
            })
        }
    }

    /// Re-opens the reader on a freshly-probed demuxer — the reopen
    /// path of the source-lifecycle contract: stream id and decode
    /// mode are retained, metadata is re-derived from the new probe,
    /// position resets to the stream start.
    ///
    /// **RT contract:** non-RT only — decoder rebuild.
    ///
    /// # Errors
    ///
    /// Same classes as [`open`](Self::open).
    pub fn reopen(&mut self, demuxer: Box<dyn Demuxer + 'a>) -> Result<()> {
        let meta = stream_metadata(demuxer.as_ref(), self.stream, self.mode)?;
        if !matches!(meta.stream.facts.decodable, Fact::Known(())) {
            return Err(Error::from_kind(ErrorKind::InvalidValue));
        }
        let decoder = demuxer.make_decoder(self.stream, self.mode)?;
        self.demuxer = demuxer;
        self.decoder = decoder;
        self.meta = meta;
        self.pending = None;
        self.position = 0;
        self.position_provenance = TimestampProvenance::Estimated;
        self.drained = false;
        self.finished = false;
        Ok(())
    }

    /// Keeps a just-pumped frame as pending with `skip` leading samples
    /// already accounted — used by seek to stash the landing frame.
    fn hold_frame(&mut self, frame: Frame, skip: u64) {
        let anchor = self.position.saturating_sub(skip);
        let frames = match &frame {
            Frame::Audio(audio) => audio.buffer().frames(),
            Frame::NativeAudio(native) => native.frames(),
            _ => usize::MAX,
        };
        let cursor = usize::try_from(skip).unwrap_or(usize::MAX).min(frames);
        self.pending = Some(Pending {
            frame,
            anchor,
            cursor,
        });
    }

    /// The pts-derived source-frame index of a frame's first sample —
    /// `None` when the frame carries no resolvable timestamp.
    fn frame_anchor(&self, frame: &Frame) -> Option<u64> {
        let rate_base = TimeBase::sample_rate(self.meta.canonical.sample_rate.get()).ok()?;
        let time = match frame {
            Frame::Audio(audio) => Some(audio.pts().time),
            Frame::NativeAudio(native) => self
                .meta
                .stream
                .stream
                .time_base
                .value()
                .map(|base| native.pts().at(base)),
            _ => None,
        }?;
        let converted = time.convert(rate_base, Rounding::Floor).ok()?;
        u64::try_from(converted.value()).ok()
    }

    /// The provenance of a frame's timestamp.
    fn frame_provenance(frame: &Frame) -> TimestampProvenance {
        match frame {
            Frame::Audio(audio) => audio.pts().provenance,
            Frame::NativeAudio(native) => native.pts().provenance,
            _ => TimestampProvenance::Estimated,
        }
    }

    /// Advances the position anchor when `next_frame` emits a frame —
    /// pts-anchored when possible, sequential estimate otherwise.
    fn anchor_frame(&mut self, frame: &Frame) {
        if let Some(anchor) = self.frame_anchor(frame) {
            self.position = anchor;
            self.position_provenance = Self::frame_provenance(frame);
        }
    }
}
