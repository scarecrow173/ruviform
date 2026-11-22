//! RIFF/WAVE serialization primitives (little-endian) — private to the
//! backend.
//!
//! Classic RIFF/WAVE only: 32-bit chunk sizes, `WAVE` form type, `fmt`
//! and `data` chunks. No `RF64`/`ds64`, no `bext`/BWF metadata — both
//! are out of the declared capability set
//! ([`crate::wav_capabilities`]).

use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat};
use ruviform_media_io::{Error, ErrorKind, Result};

/// RIFF `WAVE_FORMAT_PCM` — integer PCM.
const TAG_PCM: u16 = 0x0001;
/// RIFF `WAVE_FORMAT_IEEE_FLOAT` — IEEE-754 float PCM.
const TAG_IEEE_FLOAT: u16 = 0x0003;
/// RIFF `WAVE_FORMAT_EXTENSIBLE` — subformat + channel mask form.
const TAG_EXTENSIBLE: u16 = 0xFFFE;

/// `KSDATAFORMAT_SUBTYPE_PCM` — `{00000001-0000-0010-8000-00AA00389B71}`.
const SUBTYPE_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];
/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT` —
/// `{00000003-0000-0010-8000-00AA00389B71}`.
const SUBTYPE_IEEE_FLOAT: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// The wire sample formats this backend emits — advertised verbatim in
/// [`crate::WavCapabilities::sample_formats`].
pub(crate) const WIRE_FORMATS: &[SampleFormat] = &[
    SampleFormat::U8,
    SampleFormat::S16,
    SampleFormat::S24,
    SampleFormat::S32,
    SampleFormat::F32,
];

/// Whether `layout` is emitted as plain `WAVEFORMATEX` (16-byte `fmt`)
/// or `WAVEFORMATEXTENSIBLE` (40-byte `fmt` with channel mask).
///
/// Mono and Stereo take the classic form — every RIFF reader handles
/// it — while any other layout (including a 2-channel `Custom` mask
/// that is not plain stereo) needs the mask-carrying extensible form.
pub(crate) fn use_extensible(layout: ChannelLayout) -> bool {
    !matches!(layout, ChannelLayout::Mono | ChannelLayout::Stereo)
}

/// The `wFormatTag` for a wire sample format — `None` when the format
/// is outside [`WIRE_FORMATS`].
pub(crate) fn format_tag(format: SampleFormat) -> Option<u16> {
    match format {
        SampleFormat::U8 | SampleFormat::S16 | SampleFormat::S24 | SampleFormat::S32 => {
            Some(TAG_PCM)
        }
        SampleFormat::F32 => Some(TAG_IEEE_FLOAT),
        _ => None,
    }
}

/// The `KSDATAFORMAT_SUBTYPE_*` GUID for a wire sample format.
fn subtype(format: SampleFormat) -> [u8; 16] {
    match format_tag(format) {
        Some(TAG_IEEE_FLOAT) => SUBTYPE_IEEE_FLOAT,
        _ => SUBTYPE_PCM,
    }
}

/// Validated wire parameters shared by encoder and muxer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Wire {
    /// Wire sample encoding (one of [`WIRE_FORMATS`]).
    pub format: SampleFormat,
    /// Channel count (`layout.channels()`), u16-safe.
    pub channels: u16,
    /// Samples per second.
    pub rate: u32,
    /// `channels * bits_per_sample / 8`.
    pub block_align: u16,
    /// `rate * block_align`.
    pub byte_rate: u32,
    /// Speaker mask for `WAVEFORMATEXTENSIBLE` (`dwChannelMask`).
    pub channel_mask: u32,
    /// Whether the `fmt` chunk is the extensible form.
    pub extensible: bool,
}

impl Wire {
    /// Builds wire parameters from a declared wire [`AudioFormat`];
    /// rejects sample formats outside the advertised set and channel
    /// counts/byte rates that overflow the RIFF fields.
    pub(crate) fn new(format: &AudioFormat) -> Result<Self> {
        let sample_format = format.sample_format;
        if format_tag(sample_format).is_none() {
            return Err(unsupported_sample_format(sample_format));
        }
        let channels = u16::try_from(format.layout.channels())
            .map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
        let bytes_per_sample = u64::from(sample_format.bits_per_sample()) / 8;
        let block_align = u16::try_from(u64::from(channels) * bytes_per_sample)
            .map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
        let byte_rate = u32::try_from(u64::from(format.sample_rate.get()) * u64::from(block_align))
            .map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
        Ok(Self {
            format: sample_format,
            channels,
            rate: format.sample_rate.get(),
            block_align,
            byte_rate,
            channel_mask: format.layout.speaker_mask().bits(),
            extensible: use_extensible(format.layout),
        })
    }
}

/// The `Unsupported` rejection for a non-wire [`SampleFormat`].
///
/// The production form carries an `io::Error` source with a `{:?}`
/// diagnostic. CBMC encodes that `format!`/`io::Error::new`
/// construction into the SSA for *every* `Wire::new` unroll —
/// measured at ~0.3 GiB per call on the slice-4 reference WSL2 box,
/// which made any proof unrolling `Wire::new` more than a handful of
/// times infeasible (`Wsl/Service/E_UNEXPECTED` at ~4 GiB RSS). The
/// `kani` form keeps the verified contract — `ErrorKind::Unsupported`
/// — while dropping the diagnostic payload, which is not part of the
/// wire contract (the proptest gate likewise asserts the kind only).
#[cfg(not(kani))]
fn unsupported_sample_format(sample_format: SampleFormat) -> Error {
    Error::with_source(
        ErrorKind::Unsupported,
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("wire sample format {sample_format:?} is not in wav_capabilities"),
        ),
    )
}

/// See the `not(kani)` variant — same contract, source-less error.
#[cfg(kani)]
fn unsupported_sample_format(_sample_format: SampleFormat) -> Error {
    Error::from_kind(ErrorKind::Unsupported)
}

/// Bytes of a 16-bit little-endian value.
fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Bytes of a 32-bit little-endian value.
fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// The complete `fmt ` chunk body (`WAVEFORMATEX` 16 bytes or
/// `WAVEFORMATEXTENSIBLE` 40 bytes).
pub(crate) fn fmt_chunk_body(wire: &Wire) -> Vec<u8> {
    let mut body = Vec::with_capacity(40);
    if wire.extensible {
        push_u16(&mut body, TAG_EXTENSIBLE);
    } else {
        push_u16(&mut body, format_tag(wire.format).unwrap_or(TAG_PCM));
    }
    push_u16(&mut body, wire.channels);
    push_u32(&mut body, wire.rate);
    push_u32(&mut body, wire.byte_rate);
    push_u16(&mut body, wire.block_align);
    push_u16(&mut body, wire.format.bits_per_sample());
    if wire.extensible {
        push_u16(&mut body, 22); // cbSize
        push_u16(&mut body, wire.format.bits_per_sample()); // wValidBitsPerSample
        push_u32(&mut body, wire.channel_mask);
        body.extend_from_slice(&subtype(wire.format));
    }
    body
}

/// The complete file header — `RIFF` + size placeholder + `WAVE` +
/// `fmt ` chunk + `data` + size placeholder. Returns the byte offset
/// of the `data` chunk's size field for later backpatching.
pub(crate) fn header(wire: &Wire) -> (Vec<u8>, u64) {
    let body = fmt_chunk_body(wire);
    let mut header = Vec::with_capacity(12 + 8 + body.len() + 8);
    header.extend_from_slice(b"RIFF");
    push_u32(&mut header, 0); // backpatched: riff size = file len - 8
    header.extend_from_slice(b"WAVE");
    header.extend_from_slice(b"fmt ");
    push_u32(&mut header, u32::try_from(body.len()).unwrap_or(u32::MAX));
    header.extend_from_slice(&body);
    header.extend_from_slice(b"data");
    let data_size_offset = header.len() as u64;
    push_u32(&mut header, 0); // backpatched: data chunk byte count
    (header, data_size_offset)
}

/// Byte offset of the `RIFF` chunk size field (always 4).
pub(crate) const RIFF_SIZE_OFFSET: u64 = 4;

/// Kani proofs of the `Wire` gate and `fmt `/header serialization
/// contract — `pub(crate)` items, so the harnesses live in-source;
/// `cargo kani --tests` covers lib-target proofs (run via
/// `mise run test-formal-kani`).
///
/// Slice-1 lesson applied: symbolic scalars (rate bits, a layout
/// selector plus mask bits, a format selector) on a concrete
/// serialization skeleton — no symbolic byte streams. Two bounded
/// costs shaped the harnesses:
///
/// - `Wire::new`'s `Unsupported` arm builds `std::io::Error::new`
///   + a `format!` diagnostic eagerly, which exhausted the slice-4
///   WSL2 box even over a concrete 5-iteration loop — the boxed
///   `io::Error` repr, not the state space, is the formula bomb.
///   `unsupported_sample_format` isolates that construction and
///   drops the payload under `cfg(kani)`, which keeps every
///   `Wire::new` unroll small enough to verify: the membership
///   oracle is proven exhaustively over a symbolic `SampleFormat`
///   (`format_tag_membership_is_wire_formats`), the `Unsupported`
///   kind itself is proven on the concrete non-wire enumeration
///   (`wire_new_non_wire_is_unsupported`), and the wire-format
///   gate proofs only ever reach `InvalidValue`
///   (`Error::from_kind` — no boxed source).
/// - `SampleRate::new(rate)` normalizes via `TimeBase::new`'s
///   Euclid `while` loop — `gcd(1, rate)` exits in <= 3 iterations,
///   but each unrolled iteration nests another symbolic `%`, so
///   the unwind bound (not the state space) sets the formula size.
///   The small bounds below cover every real loop with headroom;
///   the unwinding assertion stays fail-closed.
#[cfg(kani)]
mod proofs {
    use super::*;
    use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleRate, SpeakerMask};

    /// The non-wire `SampleFormat` variants — `WIRE_FORMATS`'s
    /// complement as of this proof; `format_tag_membership_is_wire_formats`
    /// keeps the two sets in sync.
    const NON_WIRE_FORMATS: &[SampleFormat] = &[
        SampleFormat::F64,
        SampleFormat::S8,
        SampleFormat::U16,
        SampleFormat::U24,
        SampleFormat::U32,
    ];

    /// The membership oracle `Wire::new` consults: for a fully
    /// symbolic `SampleFormat`, `Some(_)` iff the format is in the
    /// advertised `WIRE_FORMATS` set. Pure `match` — no diagnostic
    /// construction — so the whole enum is symbolic here.
    #[kani::proof]
    #[kani::unwind(8)]
    fn format_tag_membership_is_wire_formats() {
        let sel: u8 = kani::any();
        let format = match sel % 10 {
            0 => SampleFormat::F32,
            1 => SampleFormat::F64,
            2 => SampleFormat::U8,
            3 => SampleFormat::S8,
            4 => SampleFormat::U16,
            5 => SampleFormat::S16,
            6 => SampleFormat::U24,
            7 => SampleFormat::S24,
            8 => SampleFormat::U32,
            _ => SampleFormat::S32,
        };
        assert!(format_tag(format).is_some() == WIRE_FORMATS.contains(&format));
        assert!(format_tag(format).is_none() == NON_WIRE_FORMATS.contains(&format));
    }

    /// Every non-wire `SampleFormat` reaches `Wire::new`'s
    /// `Unsupported` rejection — concrete enumeration (no symbolic
    /// rate, so no GCD unroll). Feasible only because
    /// `unsupported_sample_format` drops the `format!`/`io::Error`
    /// payload under `cfg(kani)`; the verified contract is the kind.
    #[kani::proof]
    #[kani::unwind(8)]
    fn wire_new_non_wire_is_unsupported() {
        for &sample_format in NON_WIRE_FORMATS {
            let format = AudioFormat {
                sample_rate: SampleRate::new(48_000).expect("nonzero rate"),
                layout: ChannelLayout::Stereo,
                sample_format,
            };
            let error = Wire::new(&format).expect_err("non-wire format must be rejected");
            assert!(error.kind() == ErrorKind::Unsupported);
        }
    }

    /// `Wire::new` acceptance over the wire formats is exactly
    /// "`rate * block_align <= u32::MAX`" — proven at the boundary:
    /// one (layout, format) rep per *distinct `block_align`*, rates
    /// straddling `u32::MAX / block_align`, accept iff the product
    /// fits, reject `InvalidValue` iff it does not. The gate's whole
    /// contract is that comparison, so per-align reps lose no
    /// coverage. The proof is split over three disjoint align
    /// subsets — not for semantics (each rep is independent) but for
    /// solver RAM: CBMC's formula grows ~linearly with the unrolled
    /// `Wire::new` count and a single 39-case matrix pushed the
    /// reference WSL2 box past the point where its service crashes
    /// (`Wsl/Service/E_UNEXPECTED` — see
    /// `docs/specs/formal-verification.md`). Everything is concrete —
    /// `SampleRate::new` runs Euclid's `while` loop, whose nested
    /// symbolic `%` unrolls are likewise infeasible here. The
    /// interior `u32` domain is proptest-covered
    /// (`wire_gate_is_exact_membership_plus_byte_rate` generates the
    /// same ±4 boundary band plus uniform rates). The
    /// channel-count (<= 32) / block-align (<= 256) `u16` gates can
    /// never trip here — a future wider layout that could trip them
    /// changes the proof, not just the odds.
    fn check_wire_gate(cases: &[(ChannelLayout, SampleFormat)]) {
        for &(layout, sample_format) in cases {
            let block_align =
                u64::from(layout.channels()) * u64::from(sample_format.bits_per_sample()) / 8;
            let boundary =
                u32::try_from(u64::from(u32::MAX) / block_align).expect("boundary fits u32");
            for rate in [
                boundary.saturating_sub(1),
                boundary,
                boundary.saturating_add(1),
            ] {
                let format = AudioFormat {
                    sample_rate: SampleRate::new(rate).expect("nonzero rate"),
                    layout,
                    sample_format,
                };
                match Wire::new(&format) {
                    Ok(wire) => {
                        assert!(u64::from(rate) * block_align <= u64::from(u32::MAX));
                        assert!(wire.format == sample_format);
                        assert!(u32::from(wire.channels) == layout.channels());
                        assert!(wire.rate == rate);
                        assert!(u64::from(wire.block_align) == block_align);
                        assert!(u64::from(wire.byte_rate) == u64::from(rate) * block_align);
                        assert!(wire.channel_mask == layout.speaker_mask().bits());
                        assert!(wire.extensible == use_extensible(layout));
                    }
                    Err(error) => {
                        assert!(error.kind() == ErrorKind::InvalidValue);
                        assert!(u64::from(rate) * block_align > u64::from(u32::MAX));
                    }
                }
            }
        }
    }

    /// Small `block_align` reps — {1,2,3,4,6} via Mono{U8,S16,S24,S32}
    /// and Stereo S24. `unwind(8)` covers the outer/inner loops and
    /// the concrete GCD unrolls.
    #[kani::proof]
    #[kani::unwind(8)]
    fn wire_new_gate_is_exact_small_align() {
        check_wire_gate(&[
            (ChannelLayout::Mono, SampleFormat::U8),
            (ChannelLayout::Mono, SampleFormat::S16),
            (ChannelLayout::Mono, SampleFormat::S24),
            (ChannelLayout::Mono, SampleFormat::S32),
            (ChannelLayout::Stereo, SampleFormat::S24),
        ]);
    }

    /// Mid `block_align` reps — {8,12,16,24} via Stereo F32, Quad
    /// S24, Quad F32, 5.1 S32. `unwind(8)` as above.
    #[kani::proof]
    #[kani::unwind(8)]
    fn wire_new_gate_is_exact_mid_align() {
        check_wire_gate(&[
            (ChannelLayout::Stereo, SampleFormat::F32),
            (ChannelLayout::Quad, SampleFormat::S24),
            (ChannelLayout::Quad, SampleFormat::F32),
            (ChannelLayout::FivePointOne, SampleFormat::S32),
        ]);
    }

    /// Large `block_align` reps — {32,64,96,128} via 7.1 F32 and
    /// arbitrary-mask custom layouts (16/24/32 speakers). Covers the
    /// extensible `fmt ` path. `unwind(8)` as above.
    #[kani::proof]
    #[kani::unwind(8)]
    fn wire_new_gate_is_exact_large_align() {
        check_wire_gate(&[
            (ChannelLayout::SevenPointOne, SampleFormat::F32),
            (
                ChannelLayout::custom(SpeakerMask::from_bits(0x0000_FFFF)).expect("non-empty mask"),
                SampleFormat::S32,
            ),
            (
                ChannelLayout::custom(SpeakerMask::from_bits(0x00FF_FFFF)).expect("non-empty mask"),
                SampleFormat::F32,
            ),
            (
                ChannelLayout::custom(SpeakerMask::from_bits(0xFFFF_FFFF)).expect("non-empty mask"),
                SampleFormat::F32,
            ),
        ]);
    }

    /// `fmt_chunk_body`/`header` serialize a validated `Wire` to the
    /// exact RIFF layout: 16- vs 40-byte `fmt ` body by
    /// extensibility, `cbSize` 22 + valid bits + speaker mask +
    /// subtype GUID on the extensible form, then
    /// `RIFF`/`WAVE`/`fmt `/`data` with zeroed size placeholders and
    /// the data-size offset immediately after `data`. Concrete
    /// (layout, format) matrix — `SampleRate::new`'s Euclid loop and
    /// the `Vec` write path are both formula-heavy under a symbolic
    /// domain on this environment (see `wire_new_gate_is_exact`).
    /// `unwind(64)` covers the builtin `memcmp` loop inside the
    /// slice-equality asserts (largest compare: 40 bytes).
    #[kani::proof]
    #[kani::unwind(64)]
    fn fmt_and_header_layout() {
        for format in [
            AudioFormat {
                sample_rate: SampleRate::new(48_000).expect("nonzero rate"),
                layout: ChannelLayout::Mono,
                sample_format: SampleFormat::S16,
            },
            AudioFormat {
                sample_rate: SampleRate::new(44_100).expect("nonzero rate"),
                layout: ChannelLayout::Stereo,
                sample_format: SampleFormat::F32,
            },
            AudioFormat {
                sample_rate: SampleRate::new(96_000).expect("nonzero rate"),
                layout: ChannelLayout::FivePointOne,
                sample_format: SampleFormat::S24,
            },
            AudioFormat {
                sample_rate: SampleRate::new(192_000).expect("nonzero rate"),
                layout: ChannelLayout::SevenPointOne,
                sample_format: SampleFormat::F32,
            },
            AudioFormat {
                sample_rate: SampleRate::new(8_000).expect("nonzero rate"),
                layout: ChannelLayout::custom(SpeakerMask::from_bits(0x0607))
                    .expect("non-empty mask"),
                sample_format: SampleFormat::S32,
            },
        ] {
            let wire = Wire::new(&format).expect("matrix spec is wire-legal");
            check_serialized_wire(&format, &wire);
        }
    }

    /// Field-level oracle for `fmt_and_header_layout` — asserts the
    /// serialized layout against the declared format.
    fn check_serialized_wire(format: &AudioFormat, wire: &Wire) {
        let extensible = use_extensible(format.layout);
        let body = fmt_chunk_body(wire);
        let body_len = if extensible { 40 } else { 16 };
        assert!(body.len() == body_len);
        let tag = u16::from_le_bytes([body[0], body[1]]);
        assert!(
            tag == if extensible {
                TAG_EXTENSIBLE
            } else {
                format_tag(format.sample_format).unwrap_or(TAG_PCM)
            }
        );
        assert!(u32::from(u16::from_le_bytes([body[2], body[3]])) == format.layout.channels());
        assert!(
            u32::from_le_bytes([body[4], body[5], body[6], body[7]]) == format.sample_rate.get()
        );
        assert!(u64::from(u16::from_le_bytes([body[12], body[13]])) == u64::from(wire.block_align));
        assert!(u16::from_le_bytes([body[14], body[15]]) == format.sample_format.bits_per_sample());
        if extensible {
            assert!(u16::from_le_bytes([body[16], body[17]]) == 22);
            assert!(
                u16::from_le_bytes([body[18], body[19]]) == format.sample_format.bits_per_sample()
            );
            assert!(
                u32::from_le_bytes([body[20], body[21], body[22], body[23]])
                    == format.layout.speaker_mask().bits()
            );
            let want: &[u8] = if format.sample_format == SampleFormat::F32 {
                &SUBTYPE_IEEE_FLOAT
            } else {
                &SUBTYPE_PCM
            };
            assert!(body[24..40] == *want);
        }

        let (header, data_size_offset) = header(wire);
        assert!(header.len() == 28 + body_len);
        assert!(&header[0..4] == b"RIFF");
        assert!(&header[8..12] == b"WAVE");
        assert!(&header[12..16] == b"fmt ");
        assert!(
            u32::from_le_bytes([header[16], header[17], header[18], header[19]]) == body_len as u32
        );
        assert!(header[20..20 + body_len] == body[..]);
        assert!(&header[20 + body_len..24 + body_len] == b"data");
        assert!(data_size_offset == (24 + body_len) as u64);
        // Both size fields stay zeroed placeholders at open time.
        assert!(u32::from_le_bytes([header[4], header[5], header[6], header[7]]) == 0);
        assert!(
            u32::from_le_bytes([
                header[24 + body_len],
                header[25 + body_len],
                header[26 + body_len],
                header[27 + body_len]
            ]) == 0
        );
    }
}
