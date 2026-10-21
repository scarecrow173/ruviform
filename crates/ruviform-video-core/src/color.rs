//! Color space description and HDR metadata: [`ColorInfo`] bundles the
//! primaries, transfer characteristics, matrix coefficients and color
//! range of a frame, with optional [`HdrMetadata`] carrying
//! [`MasteringDisplayInfo`] and [`ContentLightLevel`].
//!
//! These types describe color; they perform no conversion. The decided
//! scope of that description — ADR
//! `docs/adr/video-proc-kernel-api-and-color-hdr-scope.md`, D3 — is HDR
//! as *metadata carriage plus non-destructive handling*: [`ColorInfo`] /
//! [`HdrMetadata`] travel with a frame, conversion kernels
//! (`ruviform-video-proc`) apply `matrix`/`range` code-value math only
//! and propagate `primaries`/`transfer`/`hdr` unchanged — no transfer
//! function application (e.g. PQ/HLG linearization) and no tone
//! mapping. Conversions interpret an `Unspecified` matrix as Bt601 and
//! an `Unspecified` range as Limited, and update the output
//! `matrix`/`range` to the applied values. Broader color management
//! stays deferred to a later design target.

/// Color primaries of a video signal (ITU-T H.273-style labeling).
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorPrimaries {
    /// Not specified — the interpretation is left to downstream
    /// processing.
    #[default]
    Unspecified,
    /// ITU-R BT.709 (HDTV).
    Bt709,
    /// SMPTE 170M (NTSC SDTV).
    Smpte170m,
    /// ITU-R BT.470 BG (PAL SDTV).
    Bt470Bg,
    /// ITU-R BT.2020 (UHDTV).
    Bt2020,
    /// DCI-P3 with D65 white point.
    P3D65,
}

/// Transfer (electro-optical) characteristics of a video signal
/// (ITU-T H.273-style labeling).
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum TransferCharacteristics {
    /// Not specified — the interpretation is left to downstream
    /// processing.
    #[default]
    Unspecified,
    /// ITU-R BT.709 / BT.601 / BT.2020 SDR gamma curve.
    Bt709,
    /// SMPTE 170M gamma curve.
    Smpte170m,
    /// Linear light — no transfer function applied.
    Linear,
    /// Perceptual Quantizer — SMPTE ST 2084 / ITU-R BT.2100 HDR.
    Pq,
    /// Hybrid Log-Gamma — ITU-R BT.2100 HDR.
    Hlg,
}

/// Matrix coefficients used to convert between RGB and YCbCr
/// (ITU-T H.273-style labeling).
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum MatrixCoefficients {
    /// Not specified — the interpretation is left to downstream
    /// processing.
    #[default]
    Unspecified,
    /// Identity — no matrixing (typically RGB or monochrome content).
    Identity,
    /// ITU-R BT.601 (SDTV).
    Bt601,
    /// ITU-R BT.709 (HDTV).
    Bt709,
    /// ITU-R BT.2020 (UHDTV).
    Bt2020,
}

/// Quantization range of a video signal.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorRange {
    /// Not specified — the interpretation is left to downstream
    /// processing.
    #[default]
    Unspecified,
    /// Limited (studio) range — headroom and footroom reserved.
    Limited,
    /// Full range — the whole code space is signal.
    Full,
}

/// One chromaticity coordinate pair.
///
/// Coordinates are stored in CTA-861 units of 1/50 000 — i.e. the stored
/// integers equal the CIE 1931 x/y coordinates multiplied by 50 000.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Chromaticity {
    /// x coordinate in units of 1/50 000.
    pub x: u16,
    /// y coordinate in units of 1/50 000.
    pub y: u16,
}

/// Mastering display color volume metadata (SMPTE ST 2086 / CTA-861).
///
/// `primaries` holds the display's R, G, B chromaticities in that order.
/// Luminance values are in units of 1/10 000 cd/m².
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MasteringDisplayInfo {
    /// R, G, B primaries of the mastering display.
    pub primaries: [Chromaticity; 3],
    /// White point of the mastering display.
    pub white_point: Chromaticity,
    /// Maximum mastering display luminance in units of 1/10 000 cd/m².
    pub max_luminance: u32,
    /// Minimum mastering display luminance in units of 1/10 000 cd/m².
    pub min_luminance: u32,
}

/// Content light level metadata (CTA-861).
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ContentLightLevel {
    /// Maximum content light level (`MaxCLL`) in cd/m².
    pub max_cll: u16,
    /// Maximum frame-average light level (`MaxFALL`) in cd/m².
    pub max_fall: u16,
}

/// HDR side metadata attached to a frame or stream.
///
/// Both fields are optional and independent: a stream may carry mastering
/// display info, content light levels, both or neither.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct HdrMetadata {
    /// Mastering display color volume, when signaled.
    pub mastering_display: Option<MasteringDisplayInfo>,
    /// Content light level, when signaled.
    pub content_light_level: Option<ContentLightLevel>,
}

/// Complete color description of a [`VideoFrame`](crate::VideoFrame).
///
/// The default is "all unspecified": every enumeration sits at
/// `Unspecified` and `hdr` is `None`.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_video_core::{ColorInfo, ColorPrimaries, ColorRange, TransferCharacteristics};
///
/// let mut color = ColorInfo::default();
/// assert_eq!(color.primaries, ColorPrimaries::Unspecified);
/// assert_eq!(color.range, ColorRange::Unspecified);
/// assert!(color.hdr.is_none());
///
/// color.primaries = ColorPrimaries::Bt709;
/// color.transfer = TransferCharacteristics::Bt709;
/// assert_ne!(color, ColorInfo::default());
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ColorInfo {
    /// Color primaries.
    pub primaries: ColorPrimaries,
    /// Transfer characteristics.
    pub transfer: TransferCharacteristics,
    /// RGB↔YCbCr matrix coefficients.
    pub matrix: MatrixCoefficients,
    /// Quantization range.
    pub range: ColorRange,
    /// Optional HDR side metadata.
    pub hdr: Option<HdrMetadata>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_all_unspecified() {
        assert_eq!(ColorPrimaries::default(), ColorPrimaries::Unspecified);
        assert_eq!(
            TransferCharacteristics::default(),
            TransferCharacteristics::Unspecified
        );
        assert_eq!(
            MatrixCoefficients::default(),
            MatrixCoefficients::Unspecified
        );
        assert_eq!(ColorRange::default(), ColorRange::Unspecified);

        let color = ColorInfo::default();
        assert_eq!(color.primaries, ColorPrimaries::Unspecified);
        assert_eq!(color.transfer, TransferCharacteristics::Unspecified);
        assert_eq!(color.matrix, MatrixCoefficients::Unspecified);
        assert_eq!(color.range, ColorRange::Unspecified);
        assert_eq!(color.hdr, None);
    }

    #[test]
    fn hdr_metadata_is_copy_and_optional() {
        let hdr = HdrMetadata {
            mastering_display: Some(MasteringDisplayInfo {
                primaries: [
                    Chromaticity {
                        x: 34_000,
                        y: 16_000,
                    },
                    Chromaticity {
                        x: 7_500,
                        y: 30_000,
                    },
                    Chromaticity {
                        x: 15_000,
                        y: 6_000,
                    },
                ],
                white_point: Chromaticity {
                    x: 15_635,
                    y: 16_450,
                },
                max_luminance: 10_000_000, // 1000 cd/m²
                min_luminance: 50,         // 0.005 cd/m²
            }),
            content_light_level: Some(ContentLightLevel {
                max_cll: 1_000,
                max_fall: 400,
            }),
        };
        let copy = hdr; // `Copy` — the original stays usable.
        assert_eq!(hdr, copy);
        assert_eq!(copy.mastering_display.unwrap().max_luminance, 10_000_000);
        assert_eq!(copy.content_light_level.unwrap().max_fall, 400);
    }
}
