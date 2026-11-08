//! `DiffPolicy` / `diff` — RGBA8 画像の決定的 pixel 比較
//! （Issue #314 slice 1・設計 D4）。
//!
//! 比較規則は明示的で決定的:
//!
//! - 画素一致判定は **per-channel**。`|actual - baseline| >
//!   tolerance[channel]` の channel が 1 つでもあればその画素は
//!   mismatch。tolerance は caller が policy として明示する —
//!   本 module は「環境差を吸収するのに十分な値」を推測しない
//!   （anti-aliasing/font/GPU 差を隠す universal threshold は
//!   存在しない）。v1 は mismatch 0 画素のみを許容し、画素数・
//!   面積比の許容は持たない（backend 別 baseline policy の責務）。
//! - **alpha=0 正規化**: baseline・actual 双方の alpha が 0 の画素は
//!   RGB が未定義になりうるため差分とみなさない（比較対象には
//!   数える）。片方だけ alpha=0 なら通常どおり全 channel を評価する
//!   （alpha 差分として検出される）。
//! - `mask` は axis-aligned `MaskRect` の列。mask 内画素は比較対象
//!   から除外される（`compared_pixels` に数えない）。mask は
//!   baseline/actual の**両方**に同じ座標で適用し、画像端で clip
//!   される。
//! - `compared_pixels == 0`（全面 mask 等）は「何も検査していない」
//!   ため `within_policy == false` — vacuous pass を出さない
//!   （verdict は `blocked`）。
//!
//! 差分画像（`diff_image`）は決定的な可視化: actual を基底に、
//! mismatch 画素を不透明赤 `(255,0,0,255)`、masked 画素を 50% 減光
//! で描画する。証拠 artifact としての保存は caller の責務。
//!
//! **RT contract:** non-RT only.

use serde::{Deserialize, Serialize};

use crate::golden::image::RgbaImage;

/// channel 毎の許容差（0..=255、per-channel `u8` 値）。
/// 既定は全 channel 0（完全一致のみ許容）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelTolerance {
    /// Red channel の許容差。
    pub red: u8,
    /// Green channel の許容差。
    pub green: u8,
    /// Blue channel の許容差。
    pub blue: u8,
    /// Alpha channel の許容差。
    pub alpha: u8,
}

impl ChannelTolerance {
    /// 全 channel 同一値の tolerance を作る。
    #[must_use]
    pub const fn uniform(v: u8) -> Self {
        Self {
            red: v,
            green: v,
            blue: v,
            alpha: v,
        }
    }

    /// channel index（0=R,1=G,2=B,3=A）に対応する許容差を返す。
    const fn channel(self, i: usize) -> u8 {
        match i {
            0 => self.red,
            1 => self.green,
            2 => self.blue,
            _ => self.alpha,
        }
    }
}

impl Default for ChannelTolerance {
    fn default() -> Self {
        Self::uniform(0)
    }
}

/// axis-aligned な矩形（画素座標、`[x, x+width) × [y, y+height)`）。
/// mask 入力と差分 bbox 出力で共用する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskRect {
    /// 左上 x 座標。
    pub x: u32,
    /// 左上 y 座標。
    pub y: u32,
    /// 幅。
    pub width: u32,
    /// 高さ。
    pub height: u32,
}

impl MaskRect {
    /// `(px, py)` が矩形内か（矩形自体の範囲で判定。画像端での
    /// clip は `diff` 側が行う）。
    #[must_use]
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x.saturating_add(self.width)
            && py < self.y.saturating_add(self.height)
    }
}

/// pixel 比較 policy。caller が明示する — tolerance/mask の値を
/// 本 module が推測・既定補完することはない（`Default` は
/// 「完全一致・mask 無し」= 最も厳しい policy）。baseline 承認時に
/// sidecar へ同梱され、比較は常に承認済み policy で行われる
/// （`store` 参照）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DiffPolicy {
    /// per-channel 許容差。
    pub channel_tolerance: ChannelTolerance,
    /// 比較から除外する領域。画像外の矩形は clip される。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mask: Vec<MaskRect>,
}

/// channel 別の観測最大差分（0..=255、順序は R,G,B,A）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelDelta {
    /// Red channel の最大差分。
    pub red: u32,
    /// Green channel の最大差分。
    pub green: u32,
    /// Blue channel の最大差分。
    pub blue: u32,
    /// Alpha channel の最大差分。
    pub alpha: u32,
}

impl ChannelDelta {
    fn channel_mut(&mut self, i: usize) -> &mut u32 {
        match i {
            0 => &mut self.red,
            1 => &mut self.green,
            2 => &mut self.blue,
            _ => &mut self.alpha,
        }
    }
}

/// 比較統計（serialize 可能・画像を含まない証拠用サマリ）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffStats {
    /// 比較画像の幅。
    pub width: u32,
    /// 比較画像の高さ。
    pub height: u32,
    /// 比較対象画素数（mask 適用後。双方 alpha=0 の画素を含む）。
    pub compared_pixels: u64,
    /// mask で除外された画素数。
    pub masked_pixels: u64,
    /// tolerance を超えた画素数。
    pub mismatched_pixels: u64,
    /// `mismatched_pixels / compared_pixels`。`compared_pixels == 0`
    /// のとき `0.0`（NaN を出さない — 比較不能は
    /// `compared_pixels` で判定する）。
    pub diff_ratio: f64,
    /// channel 別の観測最大差分。
    pub max_deltas: ChannelDelta,
    /// mismatch 画素の bounding box。mismatch 0 のとき `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_bbox: Option<MaskRect>,
}

/// `diff` の成功結果 — 統計 + policy 判定 + 決定的な差分画像。
#[derive(Debug, Clone, PartialEq)]
pub struct DiffResult {
    /// 比較統計。
    pub stats: DiffStats,
    /// `stats` が `diff` に渡された `policy` の許容内か
    /// （`compared_pixels > 0 && mismatched_pixels == 0`）。
    /// `compared_pixels == 0`（全面 mask 等の比較不能）は常に
    /// `false` — 比較不能を pass にしない。
    pub within_policy: bool,
    /// 差分可視化画像（baseline と同サイズの RGBA8）。
    pub diff_image: RgbaImage,
}

/// `diff` の結果。サイズ不一致は比較不能ではなく**測定された
/// 不一致**として扱う（`fail` 側の情報を失わない）。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum DiffOutcome {
    /// 同サイズで比較できた。
    Compared(DiffResult),
    /// baseline と actual のサイズが不一致。
    SizeMismatch {
        /// baseline の `(width, height)`。
        baseline: (u32, u32),
        /// actual の `(width, height)`。
        actual: (u32, u32),
    },
}

/// baseline と actual を `policy` で比較する。
///
/// mask は画像端で clip される。サイズ不一致は
/// [`DiffOutcome::SizeMismatch`] を返し、差分画像は作らない。
/// 本関数は失敗しない — 不正な policy という概念は v1 に存在しない
/// （policy は常に有限な u8 許容差と矩形列）。
///
/// # Panics
/// 内部不変条件（diff buffer 長 = `width * height * 4`）が壊れた
/// 場合のみ panic する — `RgbaImage` の構築時検証により通常は
/// 到達不能。
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "pixel counts are bounded by image dimensions (<= u32::MAX^2 px) and f64 mantissa (2^53) is far larger; ratio precision loss is not observable"
)]
pub fn diff(baseline: &RgbaImage, actual: &RgbaImage, policy: &DiffPolicy) -> DiffOutcome {
    if baseline.width() != actual.width() || baseline.height() != actual.height() {
        return DiffOutcome::SizeMismatch {
            baseline: (baseline.width(), baseline.height()),
            actual: (actual.width(), actual.height()),
        };
    }

    let (w, _h) = (baseline.width() as usize, baseline.height() as usize);
    let pixel_count = u64::from(baseline.width()) * u64::from(baseline.height());
    let mut stats = DiffStats {
        width: baseline.width(),
        height: baseline.height(),
        compared_pixels: 0,
        masked_pixels: 0,
        mismatched_pixels: 0,
        diff_ratio: 0.0,
        max_deltas: ChannelDelta {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        },
        diff_bbox: None,
    };
    // bbox accumulators: (min_x, min_y, max_x, max_y) inclusive
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    let mut diff_pixels = actual.pixels().to_vec();
    let bp = baseline.pixels();
    let ap = actual.pixels();

    for y in 0..baseline.height() {
        for x in 0..baseline.width() {
            let idx = ((y as usize * w) + x as usize) * 4;
            let (b, a) = (&bp[idx..idx + 4], &ap[idx..idx + 4]);
            let out = &mut diff_pixels[idx..idx + 4];
            if policy.mask.iter().any(|m| m.contains(x, y)) {
                stats.masked_pixels += 1;
                // masked → 50% 減光
                out[0] /= 2;
                out[1] /= 2;
                out[2] /= 2;
                out[3] /= 2;
                continue;
            }
            stats.compared_pixels += 1;
            // alpha=0 正規化: 双方 alpha=0 は RGB 未定義のため一致扱い
            if b[3] == 0 && a[3] == 0 {
                continue;
            }
            let mut mismatch = false;
            for c in 0..4 {
                let delta = u32::from(b[c].abs_diff(a[c]));
                let slot = stats.max_deltas.channel_mut(c);
                *slot = (*slot).max(delta);
                if delta > u32::from(policy.channel_tolerance.channel(c)) {
                    mismatch = true;
                }
            }
            if mismatch {
                stats.mismatched_pixels += 1;
                bbox = Some(match bbox {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
                // mismatch → 不透明赤
                out.copy_from_slice(&[255, 0, 0, 255]);
            }
        }
    }

    if stats.compared_pixels > 0 {
        stats.diff_ratio = stats.mismatched_pixels as f64 / stats.compared_pixels as f64;
    }
    stats.diff_bbox = bbox.map(|(x0, y0, x1, y1)| MaskRect {
        x: x0,
        y: y0,
        width: x1 - x0 + 1,
        height: y1 - y0 + 1,
    });
    debug_assert_eq!(stats.compared_pixels + stats.masked_pixels, pixel_count);

    let within_policy = stats.compared_pixels > 0 && stats.mismatched_pixels == 0;
    let diff_image = RgbaImage::new(baseline.width(), baseline.height(), diff_pixels)
        .expect("diff buffer length matches dimensions");
    DiffOutcome::Compared(DiffResult {
        stats,
        within_policy,
        diff_image,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(px: &[[u8; 4]]) -> RgbaImage {
        let w = u32::try_from(px.len()).unwrap();
        let mut buf = Vec::with_capacity(px.len() * 4);
        for p in px {
            buf.extend_from_slice(p);
        }
        RgbaImage::new(w, 1, buf).unwrap()
    }

    fn compared(outcome: DiffOutcome) -> DiffResult {
        let DiffOutcome::Compared(r) = outcome else {
            panic!("expected Compared");
        };
        r
    }

    #[test]
    fn exact_match_passes_with_zero_stats() {
        let a = img(&[[1, 2, 3, 4], [5, 6, 7, 8]]);
        let b = a.clone();
        let r = compared(diff(&a, &b, &DiffPolicy::default()));
        assert_eq!(r.stats.compared_pixels, 2);
        assert_eq!(r.stats.mismatched_pixels, 0);
        assert_eq!(r.stats.diff_ratio, 0.0);
        assert!(r.stats.diff_bbox.is_none());
        assert!(r.within_policy);
    }

    #[test]
    fn single_channel_delta_detected() {
        let a = img(&[[100, 0, 0, 255]]);
        let mut b = a.clone();
        b.set_pixel(0, 0, [101, 0, 0, 255]);
        let strict = DiffPolicy {
            channel_tolerance: ChannelTolerance::uniform(0),
            ..DiffPolicy::default()
        };
        let r = compared(diff(&a, &b, &strict));
        assert_eq!(r.stats.mismatched_pixels, 1);
        assert_eq!(r.stats.max_deltas.red, 1);
        assert_eq!(r.stats.max_deltas.green, 0);
        assert!(!r.within_policy);
        // tolerance 1 なら pass（policy は diff 呼出し時に適用される）
        let tolerant = DiffPolicy {
            channel_tolerance: ChannelTolerance::uniform(1),
            ..DiffPolicy::default()
        };
        let r2 = compared(diff(&a, &b, &tolerant));
        assert_eq!(r2.stats.mismatched_pixels, 0);
        assert!(r2.within_policy);
    }

    #[test]
    fn tolerance_boundary_is_exclusive() {
        // alpha=255 で alpha=0 正規化の影響を除く
        let a = img(&[[0, 0, 0, 255]]);
        let b = img(&[[10, 0, 0, 255]]);
        // delta == tolerance は一致扱い（> tolerance で mismatch）
        let at_limit = DiffPolicy {
            channel_tolerance: ChannelTolerance::uniform(10),
            ..DiffPolicy::default()
        };
        let r = compared(diff(&a, &b, &at_limit));
        assert_eq!(r.stats.mismatched_pixels, 0);
        assert!(r.within_policy);
        let over = DiffPolicy {
            channel_tolerance: ChannelTolerance::uniform(9),
            ..DiffPolicy::default()
        };
        let r2 = compared(diff(&a, &b, &over));
        assert_eq!(r2.stats.mismatched_pixels, 1);
        assert!(!r2.within_policy);
    }

    #[test]
    fn alpha_channel_difference_counts() {
        let a = img(&[[0, 0, 0, 255]]);
        let b = img(&[[0, 0, 0, 254]]);
        let r = compared(diff(&a, &b, &DiffPolicy::default()));
        assert_eq!(r.stats.mismatched_pixels, 1);
        assert_eq!(r.stats.max_deltas.alpha, 1);
    }

    #[test]
    fn both_transparent_pixels_match_regardless_of_rgb() {
        // alpha=0 正規化: 双方 alpha=0 は RGB が違っても差分にしない
        let pa = img(&[[0, 0, 0, 0], [9, 9, 9, 0]]);
        let pb = img(&[[255, 255, 255, 0], [1, 2, 3, 0]]);
        let res = compared(diff(&pa, &pb, &DiffPolicy::default()));
        assert_eq!(res.stats.compared_pixels, 2);
        assert_eq!(res.stats.mismatched_pixels, 0);
        assert!(res.within_policy);
        // 片方だけ alpha=0 → alpha 差分として mismatch
        let pc = img(&[[0, 0, 0, 0]]);
        let pd = img(&[[0, 0, 0, 255]]);
        let res2 = compared(diff(&pc, &pd, &DiffPolicy::default()));
        assert_eq!(res2.stats.mismatched_pixels, 1);
    }

    #[test]
    fn mask_excludes_pixels_from_comparison() {
        let a = RgbaImage::solid(4, 1, [0, 0, 0, 255]);
        let mut b = a.clone();
        b.set_pixel(1, 0, [255, 255, 255, 255]);
        b.set_pixel(3, 0, [255, 255, 255, 255]);
        let policy = DiffPolicy {
            mask: vec![MaskRect {
                x: 1,
                y: 0,
                width: 3,
                height: 1,
            }],
            ..DiffPolicy::default()
        };
        let r = compared(diff(&a, &b, &policy));
        assert_eq!(r.stats.compared_pixels, 1);
        assert_eq!(r.stats.masked_pixels, 3);
        assert_eq!(r.stats.mismatched_pixels, 0);
        assert!(r.within_policy);
        // 差分画像: masked は actual の 50% 減光
        assert_eq!(r.diff_image.pixel(1, 0), Some([127, 127, 127, 127]));
    }

    #[test]
    fn full_mask_blocks_comparison() {
        let a = RgbaImage::solid(2, 2, [0, 0, 0, 255]);
        let b = a.clone();
        let policy = DiffPolicy {
            mask: vec![MaskRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            }],
            ..DiffPolicy::default()
        };
        let r = compared(diff(&a, &b, &policy));
        assert_eq!(r.stats.compared_pixels, 0);
        assert_eq!(r.stats.masked_pixels, 4);
        assert!(!r.within_policy);
    }

    #[test]
    fn mismatch_pixels_fail_without_ratio_allowance() {
        // v1 は画素数・面積比の許容を持たない — 1 画素でも超過で fail
        let a = RgbaImage::solid(4, 1, [0, 0, 0, 255]);
        let mut b = a.clone();
        b.set_pixel(2, 0, [255, 255, 255, 255]);
        let r = compared(diff(&a, &b, &DiffPolicy::default()));
        assert_eq!(r.stats.mismatched_pixels, 1);
        assert!((r.stats.diff_ratio - 0.25).abs() < f64::EPSILON);
        assert!(!r.within_policy);
    }

    #[test]
    fn size_mismatch_is_explicit_outcome() {
        let a = RgbaImage::solid(2, 2, [0, 0, 0, 255]);
        let b = RgbaImage::solid(2, 3, [0, 0, 0, 255]);
        let outcome = diff(&a, &b, &DiffPolicy::default());
        assert_eq!(
            outcome,
            DiffOutcome::SizeMismatch {
                baseline: (2, 2),
                actual: (2, 3),
            }
        );
    }

    #[test]
    fn diff_image_marks_mismatch_red_on_actual_base() {
        let a = img(&[[200, 100, 50, 255], [10, 20, 30, 255]]);
        let b = img(&[[200, 100, 50, 255], [77, 88, 99, 255]]);
        let r = compared(diff(&a, &b, &DiffPolicy::default()));
        // match → actual そのまま
        assert_eq!(r.diff_image.pixel(0, 0), Some([200, 100, 50, 255]));
        // mismatch → 赤
        assert_eq!(r.diff_image.pixel(1, 0), Some([255, 0, 0, 255]));
    }

    #[test]
    fn diff_bbox_covers_all_mismatches() {
        let mut a = RgbaImage::solid(8, 4, [0, 0, 0, 255]);
        let b = a.clone();
        a.set_pixel(1, 1, [255, 0, 0, 255]);
        a.set_pixel(5, 2, [255, 0, 0, 255]);
        a.set_pixel(6, 3, [255, 0, 0, 255]);
        let r = compared(diff(&a, &b, &DiffPolicy::default()));
        assert_eq!(r.stats.mismatched_pixels, 3);
        assert_eq!(
            r.stats.diff_bbox,
            Some(MaskRect {
                x: 1,
                y: 1,
                width: 6,
                height: 3,
            })
        );
    }

    #[test]
    fn diff_stats_serde_round_trip() {
        let a = RgbaImage::solid(2, 2, [1, 2, 3, 255]);
        let b = a.clone();
        let r = compared(diff(&a, &b, &DiffPolicy::default()));
        let json = serde_json::to_string(&r.stats).unwrap();
        let back: DiffStats = serde_json::from_str(&json).unwrap();
        assert_eq!(r.stats, back);
    }

    #[test]
    fn policy_serde_round_trip() {
        let policy = DiffPolicy {
            channel_tolerance: ChannelTolerance {
                red: 1,
                green: 2,
                blue: 3,
                alpha: 4,
            },
            mask: vec![MaskRect {
                x: 0,
                y: 0,
                width: 4,
                height: 4,
            }],
        };
        let json = serde_json::to_string(&policy).unwrap();
        let back: DiffPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, back);
        // 空 mask は serialize で省略される
        let p2 = DiffPolicy::default();
        let j2 = serde_json::to_string(&p2).unwrap();
        assert!(!j2.contains("mask"));
    }
}
