//! Internal transfer-function and primaries math shared by the
//! color-management kernels (`ToWorking` / `FromWorking` /
//! `ToneMapSdr`) — `pub(crate)` internals, no public API surface.
//!
//! ## Working-space convention
//!
//! The color pipeline's working space is `RgbaF32` tagged
//! `{primaries: Bt2020, transfer: Linear, matrix: Identity, range:
//! Full}`: channel values are linear light on an **absolute-luminance
//! scale where `1.0` = 100 cd/m²** (the SDR reference white,
//! [`SDR_SCALE`]). The value range is not limited to `[0, 1]` —
//! scene-referred values above `1.0` and wide-gamut-conversion
//! negatives are carried losslessly.
//!
//! Under that convention *decode* maps a normalized signal value `v`
//! to a working-space value, and *encode* is the strict inverse:
//!
//! - `Linear` — identity (the signal already is an absolute
//!   luminance).
//! - `Bt709` / `Smpte170m` — the shared H.273 OETF inverse/forward
//!   pair: linear toe `v / 4.5` for `v < 0.081`, `((v + 0.099) /
//!   1.099)^(1/0.45)` above. The toe maps negative signals to negative
//!   output, so negative input is defined. The display-relative output
//!   is read as absolute luminance (`1.0` = SDR white = 100 nits).
//! - `Pq` — the ST 2084 EOTF and its inverse ([`pq_eotf`] /
//!   [`pq_eotf_inv`] with the `PQ_*` constants), normalized by
//!   [`SDR_SCALE`]. Inputs outside the signal domain `[0, 1]` clamp at
//!   the decode boundary (`v < 0` → `0.0`, `v > 1` → `100.0`); encode
//!   output clamps to `[0, 1]` — out-of-domain luminance saturates.
//! - `Hlg` — the BT.2100 inverse/forward OETF (`S = v²/3` for
//!   `v ≤ 0.5`, `S = (exp((v−c)/a) + b)/12` above, constants
//!   [`HLG_A`]/[`HLG_B`]/[`HLG_C`]) followed by the reference OOTF
//!   `S^1.2` at the nominal [`HLG_NOMINAL_PEAK`] — `display_rel` `1.0`
//!   ↔ working value `10.0` (= 1000 nits, the same absolute scale as
//!   PQ). Signal-domain input/output clamp as for `Pq`.
//!
//! `Bt709`/`Smpte170m`/`Linear` encode are *not* clamped — negative
//! and `> 1` signals pass through.
//!
//! ## Gamut conversion
//!
//! Each `ColorPrimaries` variant's RGB→XYZ normalized matrix (all
//! variants share the D65 white, so no chromatic adaptation is needed)
//! is pre-combined into `src→BT.2020` ([`TO_BT2020`]) and
//! `BT.2020→dst` ([`FROM_BT2020`]) 3×3 matrices — `f64` derivation
//! constants, no run-time inversion. Only the RGB channels transform;
//! alpha passes through unchanged.
//!
//! ## Resolution rules
//!
//! [`primaries_slot`] / [`transfer_slot`] resolve `Unspecified` to the
//! `Bt709` default; unknown future `#[non_exhaustive]` variants are
//! rejected with [`Error::InvalidValue`] through a `_` arm.
//!
//! All math is `f64`: the kernels' reference definition is "the `f64`
//! evaluation rounded to binary32", and optimized backends are bound
//! to a ≤ 4 ulp equivalence contract.
//!
//! Contract source: `docs/designs/color-conversion-kernels.md`
//! (「決定サマリ」・「`RgbaF32` と working-space 定義」) and
//! `docs/test-specs/color-working-space-kernel-tests.md`.

use ruviform_media_core::{Error, Result};
use ruviform_video_core::{ColorPrimaries, TransferCharacteristics};

/// Working-space absolute-luminance scale in cd/m²: the channel value
/// `1.0` corresponds to the SDR reference white of 100 nits.
pub(crate) const SDR_SCALE: f64 = 100.0;

/// BT.2100 nominal peak luminance (cd/m²) of the HLG reference
/// display — `L_W` is not carried by `ColorInfo`/`HdrMetadata`, so the
/// fixed nominal 1000 nits is the convention.
pub(crate) const HLG_NOMINAL_PEAK: f64 = 1000.0;

/// BT.2390 target display luminance (cd/m²) — the fixed SDR endpoint
/// the `ToneMapSdr` EETF maps onto.
// Sole consumer is `ToneMapSdr`, which lands in a later Issue #99
// task — `expect` (not `allow`) so the attribute is flagged for
// removal the moment the constant gains a real user.
#[expect(dead_code)]
pub(crate) const L_DS: f64 = 100.0;

/// Distinct transfer slots — must match [`transfer_slot`] and the
/// [`TRANSFER_DECODE`]/[`TRANSFER_ENCODE`] table order:
/// `0` = `Bt709`, `1` = `Smpte170m`, `2` = `Linear`, `3` = `Pq`,
/// `4` = `Hlg`.
pub(crate) const TRANSFER_SLOTS: usize = 5;

/// Resolves a frame's `transfer` to its decode/encode table slot;
/// `Unspecified` defaults to the `Bt709` slot.
///
/// Unknown future `#[non_exhaustive]` variants are rejected with
/// [`Error::InvalidValue`].
pub(crate) fn transfer_slot(transfer: TransferCharacteristics) -> Result<usize> {
    match transfer {
        TransferCharacteristics::Bt709 | TransferCharacteristics::Unspecified => Ok(0),
        TransferCharacteristics::Smpte170m => Ok(1),
        TransferCharacteristics::Linear => Ok(2),
        TransferCharacteristics::Pq => Ok(3),
        TransferCharacteristics::Hlg => Ok(4),
        // Every future `#[non_exhaustive]` variant.
        _ => Err(Error::InvalidValue("color transfer")),
    }
}

/// The applied transfer per table slot — the inverse of
/// [`transfer_slot`]'s default rule (`Unspecified` resolves to
/// `Bt709`, so the recorded value is never `Unspecified`). Same
/// purpose as `coef.rs`'s `RESOLVED_MATRIX`: kernels that fix the
/// transfer at prepare time report the applied value in an output
/// `ColorInfo`.
pub(crate) const RESOLVED_TRANSFER: [TransferCharacteristics; TRANSFER_SLOTS] = [
    TransferCharacteristics::Bt709,
    TransferCharacteristics::Smpte170m,
    TransferCharacteristics::Linear,
    TransferCharacteristics::Pq,
    TransferCharacteristics::Hlg,
];

/// `Linear` decode — identity: the signal value already is a
/// working-space absolute luminance. Not clamped.
fn decode_linear(v: f64) -> f64 {
    v
}

/// `Linear` encode — identity. Not clamped.
fn encode_linear(l: f64) -> f64 {
    l
}

/// H.273 OETF inverse (`Bt709`/`Smpte170m` share the curve): signal
/// `v` → working-space value. `v / 4.5` linear toe for `v < 0.081`,
/// `((v + 0.099) / 1.099)^(1/0.45)` above; the toe maps negative
/// inputs to negative outputs, so negative input is defined.
fn decode_bt709(v: f64) -> f64 {
    if v < 0.081 {
        v / 4.5
    } else {
        ((v + 0.099) / 1.099).powf(1.0 / 0.45)
    }
}

/// H.273 OETF (`Bt709`/`Smpte170m`): working-space value → signal.
/// `4.5·l` for `l < 0.018` (the toe boundary on the luminance side),
/// `1.099·l^0.45 − 0.099` above. Not clamped — negative and `> 1`
/// signals are preserved.
fn encode_bt709(l: f64) -> f64 {
    if l < 0.018 {
        4.5 * l
    } else {
        1.099f64.mul_add(l.powf(0.45), -0.099)
    }
}

// SMPTE ST 2084 (PQ) constants — the EOTF `m1`/`m2` and the
// `c1`/`c2`/`c3` shape constants.
const PQ_M1: f64 = 2610.0 / 16384.0;
const PQ_M2: f64 = 78.843_75;
const PQ_C1: f64 = 0.835_937_5;
const PQ_C2: f64 = 18.851_562_5;
const PQ_C3: f64 = 18.6875;

/// ST 2084 EOTF: normalized signal `v` → luminance in nits —
/// `N = v^(1/m2)`, `L = 10000·(max(N − c1, 0) / (c2 − c3·N))^(1/m1)`.
/// `v` is assumed to be inside the signal domain `[0, 1]` by the
/// caller ([`decode_pq`] clamps there); `NaN` propagates.
pub(crate) fn pq_eotf(v: f64) -> f64 {
    let n = v.powf(1.0 / PQ_M2);
    // `c2 − c3·N` in mul_add form (pedantic `suboptimal_flops`).
    // `NaN` input reaches the output through this denominator — the
    // numerator's `.max(0.0)` swallows `NaN` (`f64::max` returns the
    // non-NaN operand), but `c2 − c3·NaN` stays `NaN`.
    let denominator = PQ_C3.mul_add(-n, PQ_C2);
    10_000.0 * ((n - PQ_C1).max(0.0) / denominator).powf(1.0 / PQ_M1)
}

/// ST 2084 inverse EOTF: luminance in nits → normalized signal —
/// `((c1 + c2·Y^m1) / (1 + c3·Y^m1))^m2` with `Y = nits/10000`.
/// `nits` is assumed non-negative by the caller ([`encode_pq`]
/// saturates below `0`); `NaN` propagates.
pub(crate) fn pq_eotf_inv(nits: f64) -> f64 {
    let y = (nits / 10_000.0).powf(PQ_M1);
    // `(c1 + c2·Y^m1) / (1 + c3·Y^m1)` in mul_add form.
    let ratio = PQ_C2.mul_add(y, PQ_C1) / PQ_C3.mul_add(y, 1.0);
    ratio.powf(PQ_M2)
}

/// `Pq` decode: normalized signal → working-space value
/// (`nits / SDR_SCALE`). Inputs outside the signal domain `[0, 1]`
/// clamp at the boundary (`v < 0` → `0.0`, `v > 1` → `100.0`);
/// `NaN` propagates.
fn decode_pq(v: f64) -> f64 {
    pq_eotf(v.clamp(0.0, 1.0)) / SDR_SCALE
}

/// `Pq` encode: working-space value → normalized signal, output
/// clamped to `[0, 1]` — out-of-domain absolute luminance saturates
/// (`l ≤ 0` → `0`, `l ≥ 100` → `1`); `NaN` propagates.
fn encode_pq(l: f64) -> f64 {
    if l <= 0.0 {
        // Negative luminance has no signal — saturate to black. The
        // `<=` comparison keeps `NaN` on the formula path, where it
        // propagates.
        return 0.0;
    }
    if l >= SDR_SCALE {
        // `l = 100` ↔ `10_000` nits is the PQ signal peak — saturate
        // to `1.0`. The guard also catches `+∞` before the formula can
        // evaluate `∞/∞ = NaN`; `NaN` itself fails both comparisons
        // and stays on the formula path, where it propagates.
        return 1.0;
    }
    pq_eotf_inv(l * SDR_SCALE).clamp(0.0, 1.0)
}

// BT.2100 HLG OETF shape constants.
const HLG_A: f64 = 0.178_832_77;
const HLG_B: f64 = 0.284_668_92;
const HLG_C: f64 = 0.559_910_73;
/// BT.2100 reference system gamma — the OOTF exponent.
const HLG_GAMMA: f64 = 1.2;

/// `Hlg` decode: normalized signal → working-space value.
/// Scene-relative `S = v²/3` for `v ≤ 0.5`, `S = (exp((v−c)/a) + b)/12`
/// above; the reference OOTF `S^1.2` at the nominal
/// [`HLG_NOMINAL_PEAK`] lands on the working scale as `×10`
/// (`HLG_NOMINAL_PEAK / SDR_SCALE`). Inputs outside the signal domain
/// `[0, 1]` clamp at the boundary; `NaN` propagates.
fn decode_hlg(v: f64) -> f64 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.5 {
        v * v / 3.0
    } else {
        (f64::exp((v - HLG_C) / HLG_A) + HLG_B) / 12.0
    };
    s.powf(HLG_GAMMA) * (HLG_NOMINAL_PEAK / SDR_SCALE)
}

/// `Hlg` encode: working-space value → normalized signal, output
/// clamped to `[0, 1]` — out-of-domain luminance saturates. Inverse
/// of [`decode_hlg`]: display-relative `l·(SDR_SCALE /
/// HLG_NOMINAL_PEAK)`, `S = rel^(1/γ)`, then the BT.2100 OETF
/// `sqrt(3·S)` for `S ≤ 1/12`, `a·ln(12·S − b) + c` above.
fn encode_hlg(l: f64) -> f64 {
    let rel = l * (SDR_SCALE / HLG_NOMINAL_PEAK);
    if rel <= 0.0 {
        // Non-positive display-relative luminance has no signal —
        // saturate to black (`NaN` keeps to the formula path and
        // propagates).
        return 0.0;
    }
    let s = rel.powf(1.0 / HLG_GAMMA);
    let v = if s <= 1.0 / 12.0 {
        (3.0 * s).sqrt()
    } else {
        // `a·ln(12·S − b) + c` in mul_add form.
        HLG_A.mul_add(12.0f64.mul_add(s, -HLG_B).ln(), HLG_C)
    };
    v.clamp(0.0, 1.0)
}

/// Transfer decode (`normalized signal → working-space value`) per
/// [`transfer_slot`] — `Smpte170m` shares the `Bt709` curve.
pub(crate) const TRANSFER_DECODE: [fn(f64) -> f64; TRANSFER_SLOTS] = [
    decode_bt709,
    decode_bt709,
    decode_linear,
    decode_pq,
    decode_hlg,
];

/// Transfer encode (`working-space value → normalized signal`) per
/// [`transfer_slot`] — `Smpte170m` shares the `Bt709` curve.
pub(crate) const TRANSFER_ENCODE: [fn(f64) -> f64; TRANSFER_SLOTS] = [
    encode_bt709,
    encode_bt709,
    encode_linear,
    encode_pq,
    encode_hlg,
];

/// Distinct primaries slots — must match [`primaries_slot`] and the
/// [`TO_BT2020`]/[`FROM_BT2020`] table order: `0` = `Bt709`,
/// `1` = `Smpte170m`, `2` = `Bt470Bg`, `3` = `Bt2020`, `4` = `P3D65`.
pub(crate) const PRIMARIES_SLOTS: usize = 5;

/// Resolves a frame's `primaries` to its matrix-table slot;
/// `Unspecified` defaults to the `Bt709` slot.
///
/// Unknown future `#[non_exhaustive]` variants are rejected with
/// [`Error::InvalidValue`].
pub(crate) fn primaries_slot(primaries: ColorPrimaries) -> Result<usize> {
    match primaries {
        ColorPrimaries::Bt709 | ColorPrimaries::Unspecified => Ok(0),
        ColorPrimaries::Smpte170m => Ok(1),
        ColorPrimaries::Bt470Bg => Ok(2),
        ColorPrimaries::Bt2020 => Ok(3),
        ColorPrimaries::P3D65 => Ok(4),
        // Every future `#[non_exhaustive]` variant.
        _ => Err(Error::InvalidValue("color primaries")),
    }
}

/// The applied primaries per table slot — the inverse of
/// [`primaries_slot`]'s default rule, same purpose as
/// [`RESOLVED_TRANSFER`].
pub(crate) const RESOLVED_PRIMARIES: [ColorPrimaries; PRIMARIES_SLOTS] = [
    ColorPrimaries::Bt709,
    ColorPrimaries::Smpte170m,
    ColorPrimaries::Bt470Bg,
    ColorPrimaries::Bt2020,
    ColorPrimaries::P3D65,
];

/// Row-major 3×3 matrix applied to `[R, G, B]` triples —
/// `out[i] = m[i][0]·R + m[i][1]·G + m[i][2]·B`. `f64` throughout: the
/// reference math the kernels round to binary32.
pub(crate) type Mat3 = [[f64; 3]; 3];

/// Applies `m` to an `[R, G, B]` triple. Alpha is not part of the
/// primaries transform — callers pass it through unchanged.
pub(crate) fn apply(m: &Mat3, rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb;
    std::array::from_fn(|row| m[row][0].mul_add(r, m[row][1].mul_add(g, m[row][2] * b)))
}

/// `src primaries → BT.2020` combined matrices per [`primaries_slot`]
/// — each is `BT.2020→XYZ⁻¹ · src→XYZ` over the variant's normalized
/// D65 RGB→XYZ matrix, derived in `f64` (no run-time inversion;
/// chromatic adaptation is unnecessary because every variant shares
/// the D65 white). The `Bt2020` slot is the exact identity.
pub(crate) const TO_BT2020: [Mat3; PRIMARIES_SLOTS] = [
    // Bt709 → BT.2020.
    [
        [
            0.627_403_895_934_699_1,
            0.329_283_038_377_883_8,
            0.043_313_065_687_417_2,
        ],
        [
            0.069_097_289_358_231_99,
            0.919_540_395_075_458_7,
            0.011_362_315_566_309_171,
        ],
        [
            0.016_391_438_875_150_224,
            0.088_013_307_877_225_78,
            0.895_595_253_247_624,
        ],
    ],
    // Smpte170m → BT.2020.
    [
        [
            0.595_254_206_376_441_6,
            0.349_313_919_749_775_16,
            0.055_431_873_873_783_36,
        ],
        [
            0.081_243_661_805_292_31,
            0.891_503_295_602_217,
            0.027_253_042_592_490_482,
        ],
        [
            0.015_512_341_244_384_942,
            0.081_911_642_143_068_15,
            0.902_576_016_612_546_8,
        ],
    ],
    // Bt470Bg → BT.2020.
    [
        [
            0.655_036_776_701_966_8,
            0.302_160_964_978_907_7,
            0.042_802_258_319_125_54,
        ],
        [
            0.072_140_555_698_382_61,
            0.916_631_128_820_964_2,
            0.011_228_315_480_653_448,
        ],
        [
            0.017_113_370_439_451_71,
            0.097_853_469_923_832_98,
            0.885_033_159_636_715_6,
        ],
    ],
    // Bt2020 → BT.2020 — the identity.
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    // P3D65 → BT.2020 — note the small negative term: wide-gamut
    // primaries legitimately land outside BT.2020 and stay unclamped.
    [
        [
            0.753_833_034_361_721_7,
            0.198_597_369_052_616_52,
            0.047_569_596_585_661_86,
        ],
        [
            0.045_743_848_965_358_27,
            0.941_777_219_811_693_6,
            0.012_478_931_222_948_113,
        ],
        [
            -0.001_210_340_354_518_392_1,
            0.017_601_717_301_089_99,
            0.983_608_623_053_428_2,
        ],
    ],
];

/// `BT.2020 → dst primaries` combined matrices per [`primaries_slot`]
/// — the exact inverses of [`TO_BT2020`] (`(dst→XYZ)⁻¹ ·
/// BT.2020→XYZ`), same `f64` derivation. The `Bt2020` slot is the
/// exact identity.
pub(crate) const FROM_BT2020: [Mat3; PRIMARIES_SLOTS] = [
    // BT.2020 → Bt709.
    [
        [
            1.660_491_002_108_434_3,
            -0.587_641_138_788_549_6,
            -0.072_849_863_319_884_84,
        ],
        [
            -0.124_550_474_521_590_63,
            1.132_899_897_125_96,
            -0.008_349_422_604_369_494,
        ],
        [
            -0.018_150_763_354_905_206,
            -0.100_578_898_008_007_43,
            1.118_729_661_362_912_5,
        ],
    ],
    // BT.2020 → Smpte170m.
    [
        [
            1.776_133_615_647_231_2,
            -0.687_820_599_337_579_2,
            -0.088_313_016_309_652_1,
        ],
        [
            -0.161_375_515_461_484_76,
            1.187_315_276_947_568_8,
            -0.025_939_761_486_083_65,
        ],
        [
            -0.015_880_609_505_007_245,
            -0.095_931_239_730_308_57,
            1.111_811_849_235_315_7,
        ],
    ],
    // BT.2020 → Bt470Bg.
    [
        [
            1.585_188_606_818_974_9,
            -0.515_059_709_788_497_1,
            -0.070_128_897_030_477_5,
        ],
        [
            -0.124_550_474_521_590_68,
            1.132_899_897_125_959_4,
            -0.008_349_422_604_369_487,
        ],
        [
            -0.016_880_976_231_926_147,
            -0.115_299_384_307_778_92,
            1.132_180_360_539_704_7,
        ],
    ],
    // BT.2020 → Bt2020 — the identity.
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    // BT.2020 → P3D65.
    [
        [
            1.343_578_252_584_332_3,
            -0.282_179_670_526_135_96,
            -0.061_398_582_058_196_2,
        ],
        [
            -0.065_297_452_789_119_47,
            1.075_787_915_848_574_1,
            -0.010_490_463_059_454_946,
        ],
        [
            0.002_821_787_261_701_048,
            -0.019_598_494_524_494_192,
            1.016_776_707_262_793_1,
        ],
    ],
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts `|actual − expected| ≤ tol` with a diagnostic message —
    /// tolerance comparison keeps `float_cmp` quiet and matches the
    /// ulp-bound equivalence contract (the f64 reference formulas
    /// invert to within a few ulp away from the knots).
    fn assert_close(actual: f64, expected: f64, tol: f64) {
        assert!(
            (actual - expected).abs() <= tol,
            "expected {expected} ± {tol}, got {actual}",
        );
    }

    #[test]
    fn slot_resolution_applies_default_rules() {
        // `Unspecified` resolves to the `Bt709` slot on both tables.
        assert_eq!(transfer_slot(TransferCharacteristics::Unspecified), Ok(0));
        assert_eq!(transfer_slot(TransferCharacteristics::Bt709), Ok(0));
        assert_eq!(transfer_slot(TransferCharacteristics::Smpte170m), Ok(1));
        assert_eq!(transfer_slot(TransferCharacteristics::Linear), Ok(2));
        assert_eq!(transfer_slot(TransferCharacteristics::Pq), Ok(3));
        assert_eq!(transfer_slot(TransferCharacteristics::Hlg), Ok(4));

        assert_eq!(primaries_slot(ColorPrimaries::Unspecified), Ok(0));
        assert_eq!(primaries_slot(ColorPrimaries::Bt709), Ok(0));
        assert_eq!(primaries_slot(ColorPrimaries::Smpte170m), Ok(1));
        assert_eq!(primaries_slot(ColorPrimaries::Bt470Bg), Ok(2));
        assert_eq!(primaries_slot(ColorPrimaries::Bt2020), Ok(3));
        assert_eq!(primaries_slot(ColorPrimaries::P3D65), Ok(4));
    }

    #[test]
    fn resolved_tables_report_the_applied_value() {
        // `Unspecified` records the resolved `Bt709`, never itself.
        let slot = transfer_slot(TransferCharacteristics::Unspecified).unwrap_or(0);
        assert_eq!(RESOLVED_TRANSFER[slot], TransferCharacteristics::Bt709);
        let slot = primaries_slot(ColorPrimaries::Unspecified).unwrap_or(0);
        assert_eq!(RESOLVED_PRIMARIES[slot], ColorPrimaries::Bt709);
        // Every other variant records itself.
        for variant in [
            TransferCharacteristics::Bt709,
            TransferCharacteristics::Smpte170m,
            TransferCharacteristics::Linear,
            TransferCharacteristics::Pq,
            TransferCharacteristics::Hlg,
        ] {
            let slot = transfer_slot(variant).unwrap_or(0);
            assert_eq!(RESOLVED_TRANSFER[slot], variant);
        }
        for variant in [
            ColorPrimaries::Bt709,
            ColorPrimaries::Smpte170m,
            ColorPrimaries::Bt470Bg,
            ColorPrimaries::Bt2020,
            ColorPrimaries::P3D65,
        ] {
            let slot = primaries_slot(variant).unwrap_or(0);
            assert_eq!(RESOLVED_PRIMARIES[slot], variant);
        }
    }

    #[test]
    fn linear_decode_encode_is_identity() {
        let slot = transfer_slot(TransferCharacteristics::Linear).unwrap_or(0);
        for v in [-1.0, 0.0, 0.5, 1.0, 2.5] {
            assert_eq!(TRANSFER_DECODE[slot](v).to_bits(), v.to_bits());
            assert_eq!(TRANSFER_ENCODE[slot](v).to_bits(), v.to_bits());
        }
    }

    #[test]
    fn bt709_decode_toe_and_power() {
        // Toe boundary: `v = 0.081` takes the power branch — the
        // reference value is ((0.081+0.099)/1.099)^(1/0.45).
        assert_close(decode_bt709(0.081), 0.017_945_023_366_747_79, 1e-15);
        // Linear toe — and negative signals map through it (defined).
        assert_close(decode_bt709(0.0405), 0.009, 1e-15);
        assert_close(decode_bt709(-0.045), -0.01, 1e-15);
        // Power branch endpoints: SDR white and a mid gray.
        assert_close(decode_bt709(1.0), 1.0, 1e-15);
        assert_close(decode_bt709(0.5), 0.259_589_400_506_285_76, 1e-15);
    }

    #[test]
    fn bt709_encode_is_unclamped_inverse() {
        // Luminance-side toe boundary `l = 0.018` takes the power
        // branch (reference: 1.099·0.018^0.45 − 0.099).
        assert_close(encode_bt709(0.018), 0.081_247_944_035_140_46, 1e-15);
        // Negative luminance maps to a negative signal — not clamped.
        assert_close(encode_bt709(-0.01), -0.045, 1e-15);
        // Above `1.0` the signal likewise passes through unclamped.
        assert!(encode_bt709(2.0) > 1.0);
        assert_close(encode_bt709(1.0), 1.0, 1e-15);
    }

    #[test]
    fn pq_decode_clamps_outside_signal_domain() {
        // In-range goldens (ST 2084): 0.5 ≈ 92.2 nits → 0.922 working.
        assert_eq!(decode_pq(0.0).to_bits(), 0.0f64.to_bits());
        assert_close(decode_pq(0.5), 0.922_457_089_940_652_7, 1e-13);
        assert_close(decode_pq(1.0), 100.0, 1e-12);
        // Out-of-signal input saturates at the decode boundary.
        assert_eq!(decode_pq(-0.5).to_bits(), 0.0f64.to_bits());
        assert_close(decode_pq(1.5), 100.0, 1e-12);
        assert!(decode_pq(f64::NAN).is_nan());
    }

    #[test]
    fn pq_encode_clamps_to_signal_domain() {
        assert_eq!(encode_pq(0.0).to_bits(), 0.0f64.to_bits());
        assert_close(encode_pq(0.922_457_089_940_652_7), 0.5, 1e-12);
        assert_close(encode_pq(100.0), 1.0, 1e-12);
        // Out-of-domain luminance saturates — `+∞` lands on `1.0`,
        // not on the `∞/∞` `NaN` the bare formula would produce.
        assert_eq!(encode_pq(-1.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(encode_pq(f64::INFINITY).to_bits(), 1.0f64.to_bits());
        assert_close(encode_pq(200.0), 1.0, 1e-12);
        assert!(encode_pq(f64::NAN).is_nan());
    }

    #[test]
    fn hlg_decode_clamps_outside_signal_domain() {
        // `v = 0.5` is the OETF boundary: `S = (0.5)²/3 = 1/12`,
        // `(1/12)^1.2 × 10` on the working scale.
        assert_eq!(decode_hlg(0.0).to_bits(), 0.0f64.to_bits());
        assert_close(decode_hlg(0.5), 0.506_970_284_911_004_8, 1e-13);
        // `v = 1.0` ↔ `display_rel` 1.0 ↔ 1000 nits = working 10.0 —
        // the same absolute scale as PQ's 1000 nits.
        assert_close(decode_hlg(1.0), 10.0, 1e-6);
        // Out-of-signal input saturates at the decode boundary.
        assert_eq!(decode_hlg(-0.5).to_bits(), 0.0f64.to_bits());
        assert_close(decode_hlg(1.5), decode_hlg(1.0), 1e-12);
        assert!(decode_hlg(f64::NAN).is_nan());
    }

    #[test]
    fn hlg_encode_clamps_to_signal_domain() {
        assert_eq!(encode_hlg(0.0).to_bits(), 0.0f64.to_bits());
        // `S ≤ 1/12` (sqrt branch) and above (exp branch) both invert.
        assert_close(encode_hlg(0.1), 0.254_230_290_683_381_15, 1e-13);
        // Unclamped exp-branch golden: `l = 1.0` → `rel = 0.1` →
        // `S = 0.1^(1/1.2) ≈ 0.14678 > 1/12` → `a·ln(12·S − b) + c`.
        assert_close(encode_hlg(1.0), 0.629_620_321_882_465_1, 1e-13);
        assert_close(encode_hlg(10.0), 1.0, 1e-6);
        // Out-of-domain luminance saturates.
        assert_eq!(encode_hlg(-1.0).to_bits(), 0.0f64.to_bits());
        assert_close(encode_hlg(20.0), 1.0, 1e-12);
        assert!(encode_hlg(f64::NAN).is_nan());
    }

    #[test]
    fn decode_encode_round_trip_through_slot_tables() {
        // Signal → working → signal round-trips on the in-signal
        // domain for every slot (Smpte170m shares the Bt709 pair).
        for slot in 0..TRANSFER_SLOTS {
            for v in [0.1, 0.25, 0.5, 0.75, 1.0] {
                let round_trip = TRANSFER_ENCODE[slot](TRANSFER_DECODE[slot](v));
                // The H.273 toe/power pair is discontinuous by
                // ~2.5·10⁻⁴ around the 0.081/0.018 knot — the wider
                // bound covers slots 0/1 near it.
                assert_close(round_trip, v, 5e-4);
            }
        }
        // Working → signal → working keeps out-of-domain luminance
        // per the encode clamp rules (Pq/Hlg saturate, Bt709/Linear
        // pass through).
        for (slot, l) in [(2_usize, 2.5_f64), (3, 50.0), (4, 5.0)] {
            let round_trip = TRANSFER_DECODE[slot](TRANSFER_ENCODE[slot](l));
            assert_close(round_trip, l, 1e-9);
        }
    }

    #[test]
    fn bt2020_matrices_are_identity() {
        let slot = primaries_slot(ColorPrimaries::Bt2020).unwrap_or(0);
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(TO_BT2020[slot], identity);
        assert_eq!(FROM_BT2020[slot], identity);
    }

    #[test]
    fn combined_matrices_round_trip_and_preserve_white() {
        for slot in 0..PRIMARIES_SLOTS {
            // `TO` then `FROM` inverts to the identity (within f64
            // derivation noise).
            for rgb in [[1.0, 1.0, 1.0], [0.9, 0.2, 0.05], [0.0, 0.5, 1.0]] {
                let back = apply(&FROM_BT2020[slot], apply(&TO_BT2020[slot], rgb));
                for (got, want) in back.into_iter().zip(rgb) {
                    assert_close(got, want, 1e-12);
                }
            }
            // D65-normalized primaries map source white (1, 1, 1) to
            // BT.2020 white — row sums are ≈ 1 in both directions.
            for direction in [&TO_BT2020[slot], &FROM_BT2020[slot]] {
                for row in direction {
                    assert_close(row[0] + row[1] + row[2], 1.0, 1e-12);
                }
            }
        }
        // Wide-gamut stays unclamped: P3D65 red maps to a BT.2020
        // triple with a negative blue-ish component.
        let p3 = primaries_slot(ColorPrimaries::P3D65).unwrap_or(0);
        let [r, g, b] = apply(&TO_BT2020[p3], [1.0, 0.0, 0.0]);
        assert_close(r, 0.753_833_034_361_721_7, 1e-15);
        assert_close(g, 0.045_743_848_965_358_27, 1e-15);
        assert_close(b, -0.001_210_340_354_518_392_1, 1e-15);
        assert!(b < 0.0);
    }
}
