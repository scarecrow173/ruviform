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

/// The validated scalar products of the stream-declaration gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WireScalars {
    /// Channel count (`layout.channels()`), u16-safe.
    channels: u16,
    /// `channels * bits_per_sample / 8`.
    block_align: u16,
    /// `rate * block_align`.
    byte_rate: u32,
}

/// The rejection class of the stream-declaration gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WireRejection {
    /// The sample format is outside [`WIRE_FORMATS`].
    Unsupported,
    /// A RIFF integer field cannot represent the declaration.
    InvalidValue,
}

/// The authoritative scalar decision behind [`Wire::new`]: sample-format
/// membership, channel-count width, block-align width, and byte-rate
/// overflow. Positive channel counts and rates are the input domain — the
/// production caller derives them from [`ChannelLayout`] and
/// `SampleRate` — while overflow classes are decided here, not assumed.
///
/// This core deliberately carries no `Error`, `io::Error`, diagnostic
/// payload, allocation, header serialization, `SampleRate` normalization,
/// or environment access. The shell attaches diagnostics only when the
/// decision rejects a declaration.
fn wire_declaration_decision(
    sample_format: SampleFormat,
    channels: u32,
    rate: u32,
) -> std::result::Result<WireScalars, WireRejection> {
    if format_tag(sample_format).is_none() {
        return Err(WireRejection::Unsupported);
    }
    let channels = u16::try_from(channels).map_err(|_| WireRejection::InvalidValue)?;
    let bytes_per_sample = u64::from(sample_format.bits_per_sample()) / 8;
    let block_align = u16::try_from(u64::from(channels) * bytes_per_sample)
        .map_err(|_| WireRejection::InvalidValue)?;
    let byte_rate = u32::try_from(u64::from(rate) * u64::from(block_align))
        .map_err(|_| WireRejection::InvalidValue)?;
    Ok(WireScalars {
        channels,
        block_align,
        byte_rate,
    })
}

impl Wire {
    /// Builds wire parameters from a declared wire [`AudioFormat`];
    /// rejects sample formats outside the advertised set and channel
    /// counts/byte rates that overflow the RIFF fields.
    pub(crate) fn new(format: &AudioFormat) -> Result<Self> {
        let sample_format = format.sample_format;
        let rate = format.sample_rate.get();
        let scalars = match wire_declaration_decision(sample_format, format.layout.channels(), rate)
        {
            Ok(scalars) => scalars,
            Err(WireRejection::Unsupported) => {
                return Err(unsupported_sample_format(sample_format));
            }
            Err(WireRejection::InvalidValue) => {
                return Err(Error::from_kind(ErrorKind::InvalidValue));
            }
        };
        Ok(Self {
            format: sample_format,
            channels: scalars.channels,
            rate,
            block_align: scalars.block_align,
            byte_rate: scalars.byte_rate,
            channel_mask: format.layout.speaker_mask().bits(),
            extensible: use_extensible(format.layout),
        })
    }
}

/// The `Unsupported` rejection for a non-wire [`SampleFormat`].
///
/// This is the lazy diagnostic shell half of the declaration boundary:
/// it runs only after the pure decision core rejects the format, so the
/// success path never pays for the `io::Error` payload. The payload is a
/// diagnostic, not part of the verified wire contract; its stable meaning
/// is that the rejected format is outside `wav_capabilities`'s wire set.
fn unsupported_sample_format(sample_format: SampleFormat) -> Error {
    Error::with_source(
        ErrorKind::Unsupported,
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("wire sample format {sample_format:?} is not in wav_capabilities"),
        ),
    )
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

/// Kani proofs of the production wire-declaration decision core and the
/// `fmt `/header serialization contract — private items, so the harnesses
/// live in-source; `cargo kani` covers lib-target proofs (run via
/// `mise run test-formal-kani`).
///
/// rf#1015 slice-18 boundary: the gate proofs target
/// `wire_declaration_decision`, not a model, test double, or a
/// `cfg(kani)`-substituted production error. The core is scalar-only and
/// carries no diagnostic construction, `SampleRate::new` normalization,
/// allocation, or header serialization. `Wire::new` remains the thin
/// shell that extracts those scalars and lazily attaches diagnostics; the
/// shell-core connection and diagnostic meaning are deterministic/proptest
/// coverage, not part of these Kani verdicts. Symbolic-rate interiors stay
/// in proptest, while the boundary matrix keeps the exact machine-word
/// comparisons that define the gate. The small unwind bounds cover every
/// real loop with headroom; unwinding assertions stay fail-closed.
#[cfg(kani)]
mod proofs {
    use super::*;
    use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleRate, SpeakerMask};

    /// The non-wire `SampleFormat` variants — `WIRE_FORMATS`'s
    /// complement as of this proof. This manual inventory is the known
    /// D-5 limitation of a `#[non_exhaustive]` enum, not a permanent
    /// exhaustive proof over future variants.
    const NON_WIRE_FORMATS: &[SampleFormat] = &[
        SampleFormat::F64,
        SampleFormat::S8,
        SampleFormat::U16,
        SampleFormat::U24,
        SampleFormat::U32,
    ];

    /// The known `SampleFormat` inventory used by the membership proof.
    /// Adding a variant without updating this map makes the proof
    /// inventory stale; it is not absorbed into the verified domain.
    fn any_known_sample_format() -> SampleFormat {
        let sel: u8 = kani::any();
        match sel % 10 {
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
        }
    }

    /// A known wire format for scalar-width proofs. The `% 5` selector is
    /// exhaustive for the fixed wire set; future wire-set changes must
    /// update this harness with the inventory they advertise.
    fn any_wire_format() -> SampleFormat {
        let sel: u8 = kani::any();
        match sel % 5 {
            0 => SampleFormat::U8,
            1 => SampleFormat::S16,
            2 => SampleFormat::S24,
            3 => SampleFormat::S32,
            _ => SampleFormat::F32,
        }
    }

    /// Membership exactness of the production decision core over the
    /// known `SampleFormat` inventory: accepted exactly for
    /// `WIRE_FORMATS`, rejected `Unsupported` exactly for its documented
    /// complement. Fixed valid scalars keep the numeric gates accepted so
    /// this proof isolates membership. It also preserves the old
    /// `format_tag` oracle equality, because `format_tag` remains the
    /// serialization helper the core consults.
    #[kani::proof]
    #[kani::unwind(8)]
    fn wav_decl_gate_membership_exactness() {
        let format = any_known_sample_format();
        let decision = wire_declaration_decision(format, 2, 48_000);
        let is_wire = WIRE_FORMATS.contains(&format);
        let is_non_wire = NON_WIRE_FORMATS.contains(&format);
        assert!(is_wire != is_non_wire);
        assert!(format_tag(format).is_some() == is_wire);
        assert!(format_tag(format).is_none() == is_non_wire);
        assert!(!matches!(decision, Err(WireRejection::InvalidValue)));
        match decision {
            Ok(scalars) => {
                assert!(is_wire);
                assert!(scalars.channels == 2);
                assert!(
                    u64::from(scalars.block_align) == 2 * u64::from(format.bits_per_sample()) / 8
                );
                assert!(u64::from(scalars.byte_rate) == 48_000 * u64::from(scalars.block_align));
            }
            Err(WireRejection::Unsupported) => assert!(is_non_wire),
            Err(WireRejection::InvalidValue) => assert!(false),
        }
        kani::cover!(
            matches!(decision, Ok(_)),
            "supported format acceptance is reachable"
        );
        kani::cover!(
            matches!(decision, Err(WireRejection::Unsupported)),
            "non-wire unsupported rejection is reachable"
        );
    }

    /// Width-gate exactness over every positive `u32` channel count and
    /// every known wire format. `rate = 1` makes byte-rate overflow
    /// impossible after the u16 block-align gate, isolating the channel
    /// and block-align decisions. Production layouts currently reach only
    /// 1..=32 channels, so the widened scalar domain proves the core's
    /// machine-word boundary rather than claiming a production layout
    /// defect path.
    #[kani::proof]
    #[kani::unwind(4)]
    fn wav_decl_gate_width_exactness() {
        let format = any_wire_format();
        let channels: u32 = kani::any();
        kani::assume(channels > 0);
        let decision = wire_declaration_decision(format, channels, 1);
        let bytes_per_sample = u64::from(format.bits_per_sample()) / 8;
        let channels64 = u64::from(channels);
        let align64 = channels64 * bytes_per_sample;
        let channels_fit = channels64 <= u64::from(u16::MAX);
        let align_fits = align64 <= u64::from(u16::MAX);

        assert!(!matches!(decision, Err(WireRejection::Unsupported)));
        if !channels_fit || !align_fits {
            assert!(matches!(decision, Err(WireRejection::InvalidValue)));
        } else {
            match decision {
                Ok(scalars) => {
                    assert!(u32::from(scalars.channels) == channels);
                    assert!(u64::from(scalars.block_align) == align64);
                    assert!(u64::from(scalars.byte_rate) == align64);
                }
                Err(_) => assert!(false),
            }
        }

        kani::cover!(
            format == SampleFormat::U8
                && channels == u32::from(u16::MAX)
                && matches!(decision, Ok(_)),
            "u16 channel boundary acceptance is reachable"
        );
        kani::cover!(
            format == SampleFormat::U8
                && channels == u32::from(u16::MAX) + 1
                && matches!(decision, Err(WireRejection::InvalidValue)),
            "u16 channel boundary+1 rejection is reachable"
        );
        kani::cover!(
            format == SampleFormat::S32 && channels == 16_383 && matches!(decision, Ok(_)),
            "u16 block-align near-boundary acceptance is reachable"
        );
        kani::cover!(
            format == SampleFormat::S32
                && channels == 16_385
                && matches!(decision, Err(WireRejection::InvalidValue)),
            "u16 block-align boundary+1 rejection is reachable"
        );
    }

    /// Byte-rate acceptance over the wire formats is exactly
    /// "`rate * block_align <= u32::MAX`" — proven at the boundary:
    /// one (channel-count, format) rep per *distinct `block_align`*,
    /// rates straddling `u32::MAX / block_align`, accept iff the product
    /// fits, reject `InvalidValue` iff it does not. The core input is
    /// scalar, so this matrix no longer unrolls `SampleRate::new`'s
    /// Euclid loop or the diagnostic shell. The proof is split over three
    /// disjoint align subsets for solver RAM, not semantics. The interior
    /// `u32` domain remains proptest-covered
    /// (`wire_gate_is_exact_membership_plus_byte_rate` generates the same
    /// ±4 boundary band plus uniform rates).
    fn check_wav_decl_gate_byte_rate(cases: &[(u32, SampleFormat)]) {
        for &(channels, sample_format) in cases {
            let block_align = u64::from(channels) * u64::from(sample_format.bits_per_sample()) / 8;
            let boundary =
                u32::try_from(u64::from(u32::MAX) / block_align).expect("boundary fits u32");
            for rate in [
                boundary.saturating_sub(1),
                boundary,
                boundary.saturating_add(1),
            ] {
                let decision = wire_declaration_decision(sample_format, channels, rate);
                let product = u64::from(rate) * block_align;
                match decision {
                    Ok(scalars) => {
                        assert!(product <= u64::from(u32::MAX));
                        assert!(u32::from(scalars.channels) == channels);
                        assert!(u64::from(scalars.block_align) == block_align);
                        assert!(u64::from(scalars.byte_rate) == product);
                    }
                    Err(WireRejection::InvalidValue) => {
                        assert!(product > u64::from(u32::MAX));
                    }
                    Err(WireRejection::Unsupported) => assert!(false),
                }
                kani::cover!(
                    matches!(decision, Err(WireRejection::InvalidValue))
                        && product == u64::from(u32::MAX) + 1,
                    "byte-rate boundary+1 rejection is reachable"
                );
            }
        }
    }

    /// Small `block_align` reps — {1,2,3,4,6} through one/two-channel
    /// integer formats. `unwind(8)` covers the outer/inner loops with
    /// headroom.
    #[kani::proof]
    #[kani::unwind(8)]
    fn wav_decl_gate_byte_rate_boundary_small_align() {
        let exact = wire_declaration_decision(SampleFormat::U8, 1, u32::MAX);
        assert!(matches!(
            exact,
            Ok(scalars) if scalars.byte_rate == u32::MAX
        ));
        kani::cover!(
            matches!(exact, Ok(_)),
            "byte-rate exact boundary acceptance is reachable"
        );
        check_wav_decl_gate_byte_rate(&[
            (1, SampleFormat::U8),
            (1, SampleFormat::S16),
            (1, SampleFormat::S24),
            (1, SampleFormat::S32),
            (2, SampleFormat::S24),
        ]);
    }

    /// Mid `block_align` reps — {8,12,16,24} through stereo/quad/5.1
    /// channel counts. `unwind(8)` as above.
    #[kani::proof]
    #[kani::unwind(8)]
    fn wav_decl_gate_byte_rate_boundary_mid_align() {
        check_wav_decl_gate_byte_rate(&[
            (2, SampleFormat::F32),
            (4, SampleFormat::S24),
            (4, SampleFormat::F32),
            (6, SampleFormat::S32),
        ]);
    }

    /// Large `block_align` reps — {32,64,96,128} through 7.1 and
    /// 16/24/32-channel custom-layout widths. `unwind(8)` as above.
    #[kani::proof]
    #[kani::unwind(8)]
    fn wav_decl_gate_byte_rate_boundary_large_align() {
        check_wav_decl_gate_byte_rate(&[
            (8, SampleFormat::F32),
            (16, SampleFormat::S32),
            (24, SampleFormat::F32),
            (32, SampleFormat::F32),
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
    /// domain on this environment (see the byte-rate boundary proofs).
    /// `unwind(64)` covers the builtin `memcmp` loop inside the
    /// slice-equality asserts (largest compare: 40 bytes).
    #[kani::proof]
    #[kani::unwind(64)]
    fn wav_decl_gate_fmt_and_header_layout() {
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
