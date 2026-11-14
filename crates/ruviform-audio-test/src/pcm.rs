//! Deterministic PCM fixtures (Issue #628).
//!
//! [`PcmPattern`] is a *spec*: identical spec + identical absolute frame
//! always produces bit-identical `f32` samples on every platform. All
//! synthesis uses deterministic arithmetic only — `u64`/`u128` integer
//! math, `f64` IEEE-754 `+ - * /` (no libm transcendentals such as
//! `sin`/`cos`/`powf`), and a splitmix64-style integer hash for noise.
//! Every pattern is a pure function of the **absolute frame index**, so
//! [`PcmFixture::fill`] may start at any frame and still produce the
//! same values (position independence).
//!
//! Multichannel content is differentiated deterministically: `Noise`
//! uses an independent per-channel stream, `Sine`/`Sweep` add a
//! `channel / 16`-turn phase offset, and the remaining patterns are
//! identical across channels (still valid multichannel data).
//!
//! **RT contract:** [`PcmFixture::fill`], [`PcmFixture::fill_channel`]
//! and [`PcmFixture::fill_view`] are allocation-free, non-blocking and
//! I/O-free — proven by `rt_`-prefixed `assert_no_alloc` tests.
//! [`PcmFixture::block`] allocates and is non-RT.

use ruviform_audio_core::{AudioBufferViewMut, SampleRate};
use ruviform_media_core::{Error, Result};

/// A deterministic PCM waveform specification.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new patterns may be added without breaking downstream
/// `match` arms.
///
/// Construct fixtures through [`PcmFixture::new`] for validation; the
/// fields are public so scenarios can be inspected/serialized as data.
/// Mutating fields after construction bypasses validation but remains
/// deterministic (invalid parameters produce deterministic output, not
/// undefined behavior — e.g. `period_frames == 0` yields silence).
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PcmPattern {
    /// Digital silence — all zeros.
    Silence,
    /// Constant DC value on every sample.
    Constant {
        /// The constant sample value.
        value: f32,
    },
    /// Periodic impulse: `amplitude` at `frame % period_frames == 0`,
    /// `0.0` elsewhere.
    Impulse {
        /// Frames between impulses; must be `> 0`.
        period_frames: u32,
        /// Impulse amplitude.
        amplitude: f32,
    },
    /// Sawtooth ramp `-amplitude..+amplitude` over `period_frames`,
    /// wrapping each period.
    Ramp {
        /// Peak amplitude (symmetric around zero).
        amplitude: f32,
        /// Frames per ramp cycle; must be `> 0`.
        period_frames: u32,
    },
    /// 50% duty square wave `+amplitude`/`-amplitude` over
    /// `period_frames`.
    Square {
        /// Peak amplitude (symmetric around zero).
        amplitude: f32,
        /// Frames per square cycle; must be `> 0`.
        period_frames: u32,
    },
    /// Sine wave at `freq_hz` with peak `amplitude`, synthesized via a
    /// fixed-degree polynomial approximation (libm-free, bit-deterministic).
    Sine {
        /// Frequency in Hz; `0 <= freq_hz < sample_rate`.
        freq_hz: f64,
        /// Peak amplitude.
        amplitude: f32,
    },
    /// Linear chirp from `f0_hz` to `f1_hz` over `duration_frames`,
    /// then holding `f1_hz`. Phase is the closed-form integral of the
    /// instantaneous frequency.
    Sweep {
        /// Start frequency in Hz (`>= 0`, finite).
        f0_hz: f64,
        /// End frequency in Hz (`>= 0`, finite).
        f1_hz: f64,
        /// Frames over which the sweep interpolates; `0` switches to
        /// `f1_hz` immediately.
        duration_frames: u64,
        /// Peak amplitude.
        amplitude: f32,
    },
    /// Seeded pseudo-random noise: a stateless per-frame hash of
    /// `(seed, channel, absolute_frame)` — independent of fill position.
    Noise {
        /// Mix seed.
        seed: u64,
        /// Peak amplitude (values span `[-amplitude, +amplitude)`).
        amplitude: f32,
    },
}

/// A validated deterministic PCM fixture: sample rate + channel count
/// + pattern spec.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PcmFixture {
    /// Output sample rate.
    pub sample_rate: SampleRate,
    /// Channel count (`>= 1`).
    pub channels: u16,
    /// Waveform specification.
    pub pattern: PcmPattern,
}

impl PcmFixture {
    /// Creates a validated fixture.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `channels == 0` or the pattern
    /// parameters are out of range (zero period, negative/non-finite or
    /// `>= sample_rate` sine frequency, negative/non-finite sweep
    /// frequencies).
    pub fn new(sample_rate: SampleRate, channels: u16, pattern: PcmPattern) -> Result<Self> {
        if channels == 0 {
            return Err(Error::InvalidValue("channels"));
        }
        Self::validate_pattern(&pattern, sample_rate)?;
        Ok(Self {
            sample_rate,
            channels,
            pattern,
        })
    }

    /// Re-validates `pattern` against `sample_rate` — for callers that
    /// mutated public fields after construction.
    ///
    /// # Errors
    ///
    /// Same as [`Self::new`].
    pub fn validate(&self) -> Result<()> {
        Self::validate_pattern(&self.pattern, self.sample_rate)
    }

    fn validate_pattern(pattern: &PcmPattern, sample_rate: SampleRate) -> Result<()> {
        let rate = f64::from(sample_rate.get());
        let field = match *pattern {
            PcmPattern::Impulse { period_frames, .. }
            | PcmPattern::Ramp { period_frames, .. }
            | PcmPattern::Square { period_frames, .. }
                if period_frames == 0 =>
            {
                Some("period_frames")
            }
            PcmPattern::Sine { freq_hz, .. } if !(freq_hz >= 0.0 && freq_hz < rate) => {
                Some("freq_hz")
            }
            PcmPattern::Sweep { f0_hz, f1_hz, .. }
                if !(f0_hz >= 0.0 && f0_hz.is_finite() && f1_hz >= 0.0 && f1_hz.is_finite()) =>
            {
                Some("sweep_frequency")
            }
            _ => None,
        };
        if let Some(field) = field {
            return Err(Error::InvalidValue(field));
        }
        Ok(())
    }

    /// The deterministic sample for `channel` at absolute `frame`.
    ///
    /// **RT contract:** RT-suitable — pure arithmetic, allocation-free.
    ///
    /// The `u64`/`f64` → `f32` casts are the deliberate IEEE-754
    /// narrowing of the deterministic spec.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "deterministic IEEE-754 narrowing is the spec"
    )]
    pub fn sample_at(&self, channel: u16, frame: u64) -> f32 {
        match self.pattern {
            PcmPattern::Silence => 0.0,
            PcmPattern::Constant { value } => value,
            PcmPattern::Impulse {
                period_frames,
                amplitude,
            } => {
                let p = u64::from(period_frames);
                if p != 0 && frame.is_multiple_of(p) {
                    amplitude
                } else {
                    0.0
                }
            }
            PcmPattern::Ramp {
                amplitude,
                period_frames,
            } => {
                let p = u64::from(period_frames);
                if p == 0 {
                    return 0.0;
                }
                let phase = (frame % p) as f64 / p as f64;
                (f64::from(amplitude) * (2.0 * phase - 1.0)) as f32
            }
            PcmPattern::Square {
                amplitude,
                period_frames,
            } => {
                let p = u64::from(period_frames);
                if p == 0 {
                    return 0.0;
                }
                let phase = (frame % p) as f64 / p as f64;
                if phase < 0.5 { amplitude } else { -amplitude }
            }
            PcmPattern::Sine { freq_hz, amplitude } => {
                let phase01 = sine_phase01(freq_hz, self.sample_rate.get(), channel, frame);
                (sine_poly(phase01) * f64::from(amplitude)) as f32
            }
            PcmPattern::Sweep {
                f0_hz,
                f1_hz,
                duration_frames,
                amplitude,
            } => {
                let phase01 = sweep_phase01(
                    f0_hz,
                    f1_hz,
                    duration_frames,
                    self.sample_rate.get(),
                    channel,
                    frame,
                );
                (sine_poly(phase01) * f64::from(amplitude)) as f32
            }
            PcmPattern::Noise { seed, amplitude } => {
                (noise_unit(seed, channel, frame) * f64::from(amplitude)) as f32
            }
        }
    }

    /// Fills `frames` frames of planar `[channel][frame]` data starting
    /// at absolute `start_frame`.
    ///
    /// `out.len()` must equal `channels * frames` exactly.
    ///
    /// **RT contract:** RT-suitable — allocation-free, non-blocking,
    /// I/O-free.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::SampleRate;
    /// use ruviform_audio_test::pcm::{PcmFixture, PcmPattern};
    ///
    /// let fixture = PcmFixture::new(
    ///     SampleRate::new(48_000).unwrap(),
    ///     1,
    ///     PcmPattern::Impulse { period_frames: 4, amplitude: 0.5 },
    /// ).unwrap();
    /// let mut buf = vec![0.0f32; 8];
    /// fixture.fill(0, 8, &mut buf).unwrap();
    /// assert_eq!(buf[0], 0.5);
    /// assert_eq!(buf[4], 0.5);
    /// assert_eq!(buf[1], 0.0);
    /// ```
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `out.len() != channels * frames`.
    pub fn fill(&self, start_frame: u64, frames: u32, out: &mut [f32]) -> Result<()> {
        let expected = usize::from(self.channels).checked_mul(frames as usize);
        if self.channels == 0 || expected != Some(out.len()) {
            return Err(Error::InvalidValue("out"));
        }
        let frame_len = out.len() / usize::from(self.channels);
        for ch in 0..self.channels {
            let base = usize::from(ch) * frame_len;
            self.fill_channel(ch, start_frame, &mut out[base..base + frame_len]);
        }
        Ok(())
    }

    /// Fills one channel's `frames`-long slice starting at absolute
    /// `start_frame`.
    ///
    /// **RT contract:** RT-suitable — allocation-free.
    pub fn fill_channel(&self, channel: u16, start_frame: u64, out: &mut [f32]) {
        for (i, s) in out.iter_mut().enumerate() {
            *s = self.sample_at(channel, start_frame.wrapping_add(i as u64));
        }
    }

    /// Fills a production [`AudioBufferViewMut`] planar buffer starting
    /// at absolute `start_frame` — the same contract a device callback
    /// receives.
    ///
    /// **RT contract:** RT-suitable — allocation-free.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when the view's channel count differs
    /// from the fixture's.
    pub fn fill_view(
        &self,
        start_frame: u64,
        view: &mut AudioBufferViewMut<'_, f32>,
    ) -> Result<()> {
        if view.channels() != usize::from(self.channels) {
            return Err(Error::InvalidValue("channels"));
        }
        for ch in 0..self.channels {
            let dst = view
                .channel_mut(usize::from(ch))
                .ok_or(Error::InvalidValue("channel"))?;
            self.fill_channel(ch, start_frame, dst);
        }
        Ok(())
    }

    /// Generates an owned block of `frames` frames starting at absolute
    /// `start_frame`.
    ///
    /// **RT contract:** non-RT — allocates the returned `Vec`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `channels * frames` overflows
    /// `usize`.
    pub fn block(&self, start_frame: u64, frames: u32) -> Result<PcmBlock> {
        let len = usize::from(self.channels)
            .checked_mul(frames as usize)
            .ok_or(Error::InvalidValue("frames"))?;
        let mut data = vec![0.0f32; len];
        self.fill(start_frame, frames, &mut data)?;
        Ok(PcmBlock {
            start_frame,
            frames,
            channels: self.channels,
            sample_rate: self.sample_rate,
            data,
        })
    }
}

/// One generated PCM block: owned planar `[channel][frame]` `f32` data
/// plus the absolute frame position and format it was produced under.
#[derive(Clone, Debug, PartialEq)]
pub struct PcmBlock {
    /// Absolute frame index of `data[..]` frame 0.
    pub start_frame: u64,
    /// Frames per channel.
    pub frames: u32,
    /// Channel count.
    pub channels: u16,
    /// Sample rate the block was generated at.
    pub sample_rate: SampleRate,
    /// Planar channel-major `f32` samples (`channels * frames`).
    pub data: Vec<f32>,
}

/// A format/channel segment of a [`PcmScenario`] — the unit at which
/// format, channel count or buffer size changes mid-stream. Gaps
/// between segments model dropped/missing frames.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PcmSegment {
    /// Absolute stream frame where this segment starts.
    pub frame_start: u64,
    /// Segment length in frames (`> 0`).
    pub frame_len: u64,
    /// The fixture producing this segment's samples.
    pub fixture: PcmFixture,
}

/// A deterministic multi-segment PCM scenario: ordered, non-overlapping
/// [`PcmSegment`]s covering (possibly with gaps) an absolute frame
/// range. Gaps between consecutive segments are the dropped/missing
/// frames a consumer must observe.
#[derive(Clone, Debug, PartialEq)]
pub struct PcmScenario {
    segments: Vec<PcmSegment>,
}

impl PcmScenario {
    /// Creates a scenario from segments sorted by `frame_start`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when a segment has `frame_len == 0`,
    /// `frame_start + frame_len` overflows, segments are unsorted or
    /// overlapping.
    pub fn new(mut segments: Vec<PcmSegment>) -> Result<Self> {
        segments.sort_by_key(|s| s.frame_start);
        let mut prev_end: Option<u64> = None;
        for seg in &segments {
            if seg.frame_len == 0 {
                return Err(Error::InvalidValue("frame_len"));
            }
            let end = seg
                .frame_start
                .checked_add(seg.frame_len)
                .ok_or(Error::InvalidValue("frame_start"))?;
            if prev_end.is_some_and(|pe| seg.frame_start < pe) {
                return Err(Error::InvalidValue("frame_start"));
            }
            prev_end = Some(end);
        }
        Ok(Self { segments })
    }

    /// The validated segments in `frame_start` order.
    #[must_use]
    pub fn segments(&self) -> &[PcmSegment] {
        &self.segments
    }

    /// Whether the segments tile their frame range contiguously
    /// (`false` = gaps exist — dropped/missing frames).
    #[must_use]
    pub fn is_contiguous(&self) -> bool {
        self.segments
            .windows(2)
            .all(|w| w[0].frame_start + w[0].frame_len == w[1].frame_start)
    }

    /// The segment covering absolute `frame`, if any.
    #[must_use]
    pub fn segment_at(&self, frame: u64) -> Option<&PcmSegment> {
        let idx = self.segments.partition_point(|s| s.frame_start <= frame);
        idx.checked_sub(1).and_then(|i| {
            let seg = &self.segments[i];
            (frame < seg.frame_start + seg.frame_len).then_some(seg)
        })
    }
}

/// `splitmix64`-style integer mixer — pure `u64` arithmetic.
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Deterministic noise unit value in `[-1.0, 1.0)` for
/// `(seed, channel, absolute_frame)` — a stateless per-frame hash, so
/// results do not depend on where a fill starts.
#[expect(
    clippy::cast_precision_loss,
    reason = "top-24-bit hash → f64 is exact enough for the fixture spec"
)]
fn noise_unit(seed: u64, channel: u16, frame: u64) -> f64 {
    let h = mix64(
        seed ^ frame.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ u64::from(channel).wrapping_mul(0xBF58_476D_1CE4_E5B9),
    );
    // Top 24 bits -> [0, 2^24) -> [0, 2) -> [-1, 1); power-of-two
    // scaling is exact in f64.
    ((h >> 40) as f64) * (1.0 / 8_388_608.0) - 1.0
}

/// Phase in `[0, 1)` turns for a sine at `freq_hz`, as a pure function
/// of absolute `frame`: `step = round(freq/rate * 2^32)` fixed-point,
/// `phase = (frame * step + channel * 2^28) mod 2^32`.
///
/// The `u64` wraparound is exact modulo `2^32`, so arbitrarily large
/// frame indices stay position-independent.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "fixed-point phase math; f64→u64 is guarded and saturating"
)]
fn sine_phase01(freq_hz: f64, sample_rate: u32, channel: u16, frame: u64) -> f64 {
    let step = (freq_hz / f64::from(sample_rate) * 4_294_967_296.0).round();
    let step_u = if step >= 0.0 { step as u64 } else { 0 };
    let frame_phase = frame.wrapping_mul(step_u) & 0xFFFF_FFFF;
    // channel/16 turn = channel * 2^28 in 2^32-fixed-point units.
    let ch_phase = u64::from(channel) << 28;
    let phase32 = frame_phase.wrapping_add(ch_phase) & 0xFFFF_FFFF;
    phase32 as f64 / 4_294_967_296.0
}

/// Phase in `[0, 1)` turns for a linear sweep, as the closed-form
/// integral of `freq(t)/sample_rate` in turns. `f(t)` ramps linearly
/// `f0 -> f1` over `duration_frames` then holds `f1`.
#[expect(
    clippy::cast_precision_loss,
    reason = "frame indices beyond 2^53 lose sub-frame precision; fixture spec accepts it"
)]
fn sweep_phase01(
    f0_hz: f64,
    f1_hz: f64,
    duration_frames: u64,
    sample_rate: u32,
    channel: u16,
    frame: u64,
) -> f64 {
    let rate = f64::from(sample_rate);
    let d = duration_frames as f64;
    let t = frame as f64;
    // Phase in turns = integral of freq(t)/rate dt.
    let base_turns = if duration_frames == 0 || t <= d {
        // freq(t) = f0 + (f1-f0)*t/D ; integral = f0*t + (f1-f0)*t^2/(2D)
        let k = if duration_frames == 0 {
            0.0
        } else {
            (f1_hz - f0_hz) / (2.0 * d)
        };
        (f0_hz * t + k * t * t) / rate
    } else {
        // Full sweep phase + f1 for the remaining frames.
        let sweep_turns = (f0_hz * d + (f1_hz - f0_hz) * d / 2.0) / rate;
        sweep_turns + f1_hz * (t - d) / rate
    };
    // channel/16 turn offset, then wrap into [0, 1).
    let with_ch = base_turns + f64::from(channel) / 16.0;
    with_ch - with_ch.floor()
}

/// Fixed-degree polynomial approximation of `sin(2π·phase01 - π)` —
/// i.e. `sin` on `[-π, π)` mapped from phase `[0,1)`. Taylor series
/// through `x^17` evaluated in `f64`; all operations are IEEE-754
/// `+ - * /`, bit-identical across platforms. No libm.
fn sine_poly(phase01: f64) -> f64 {
    const TAU: f64 = std::f64::consts::TAU;
    const PI: f64 = std::f64::consts::PI;
    let x = phase01 * TAU - PI;
    let x2 = x * x;
    // sin(x) = x - x^3/3! + x^5/5! - x^7/7! + x^9/9! - x^11/11!
    //        + x^13/13! - x^15/15! + x^17/17!  (Horner form).
    x * (1.0
        + x2 * (-1.0 / 6.0
            + x2 * (1.0 / 120.0
                + x2 * (-1.0 / 5_040.0
                    + x2 * (1.0 / 362_880.0
                        + x2 * (-1.0 / 39_916_800.0
                            + x2 * (1.0 / 6_227_020_800.0
                                + x2 * (-1.0 / 1_307_674_368_000.0
                                    + x2 * (1.0 / 355_687_428_096_000.0)))))))))
}
