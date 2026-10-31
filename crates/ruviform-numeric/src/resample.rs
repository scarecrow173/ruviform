//! アンチエイリアス付き、有理レート比の polyphase Blackman-sinc FIR
//! リサンプラ（AudioLens `al-math::resample` の移植）。
//!
//! 対称 FIR の中心を出力時刻に置くことで群遅延を補償する。端点はゼロ拡張、
//! 出力長は `ceil(frames·up/down)`。al-math が呼出し時に `HashMap` で位相
//! 係数を遅延構築するのに対し、本実装は plan 構築時に全 `up` 位相の
//! 正規化済み係数表を確保する — これにより [`ResamplePlan::resample_into`]
//! が alloc なし・有界の **RT-suitable** API になる。
//!
//! # 確保量
//!
//! plan は `up × (2·radius + 1)` 個の係数を保持する。極端な有理比
//! （gcd が小さい）では `up` が大きくなり得るため、確保量は
//! [`ResamplePlan::memory_bytes`] で確認できる。

use crate::{Error, Result};
use std::f64::consts::PI;

/// 係数を完全に再生成できる変換設定。端点はゼロ拡張、長さは ceil。
#[derive(Debug, Clone, PartialEq)]
pub struct ResamplePlan {
    source_rate: u32,
    target_rate: u32,
    up: u32,
    down: u32,
    radius: usize,
    cutoff: f64,
    /// 位相係数表。`phases[p·taps .. (p+1)·taps]` が位相 `p` の
    /// `taps = 2·radius + 1` 個の正規化済み係数。
    phases: Vec<f64>,
}

impl ResamplePlan {
    /// `source_rate` → `target_rate` の plan を構築する。
    /// 有理比は gcd で約分される。
    ///
    /// **RT 契約:** non-RT only — 全位相の係数表を確保する。
    ///
    /// # Errors
    ///
    /// いずれかのレートが 0 なら `Error::InvalidValue("rate")`。
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn new(source_rate: u32, target_rate: u32) -> Result<Self> {
        if source_rate == 0 || target_rate == 0 {
            return Err(Error::InvalidValue("rate"));
        }
        let (mut a, mut b) = (source_rate, target_rate);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        let ratio = (f64::from(target_rate) / f64::from(source_rate)).min(1.0);
        let mut plan = Self {
            source_rate,
            target_rate,
            up: target_rate / a,
            down: source_rate / a,
            radius: (32.0 / ratio).ceil() as usize,
            cutoff: 0.94 * ratio,
            phases: Vec::new(),
        };
        let taps = plan.taps();
        let mut phases = Vec::with_capacity(plan.up as usize * taps);
        for p in 0..plan.up {
            phases.extend(plan.phase_coefficients(p));
        }
        plan.phases = phases;
        Ok(plan)
    }

    /// 変換元サンプルレート。
    #[must_use]
    pub fn source_rate(&self) -> u32 {
        self.source_rate
    }

    /// 変換先サンプルレート。
    #[must_use]
    pub fn target_rate(&self) -> u32 {
        self.target_rate
    }

    /// 約分後の補間比（分子側）。
    #[must_use]
    pub fn up(&self) -> u32 {
        self.up
    }

    /// 約分後の間引き比（分母側）。
    #[must_use]
    pub fn down(&self) -> u32 {
        self.down
    }

    /// FIR の半径（タップ数は `2·radius + 1`）。
    #[must_use]
    pub fn radius(&self) -> usize {
        self.radius
    }

    /// 1 位相あたりのタップ数（`2·radius + 1`）。
    #[must_use]
    pub fn taps(&self) -> usize {
        2 * self.radius + 1
    }

    /// sinc の正規化遮断（`0.94·min(target/source, 1)`）。
    #[must_use]
    pub fn cutoff(&self) -> f64 {
        self.cutoff
    }

    /// 係数表のメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.phases.len() * size_of::<f64>()
    }

    /// `frames` 入力サンプルの出力長（`ceil(frames·up/down)`）。
    ///
    /// # Errors
    ///
    /// 長さが `usize` に収まらなければ `Error::InvalidValue("length")`。
    pub fn output_len(&self, frames: usize) -> Result<usize> {
        usize::try_from((frames as u128 * u128::from(self.up)).div_ceil(u128::from(self.down)))
            .map_err(|_| Error::InvalidValue("length"))
    }

    /// `input` をリサンプルした `Vec` を返す。レート一致ではサンプルを
    /// そのまま複製し、波形を一切変えない。
    ///
    /// **RT 契約:** non-RT only — 出力 `Vec` を確保する。
    ///
    /// # Errors
    ///
    /// [`Self::output_len`] と同じ。
    pub fn resample(&self, input: &[f64]) -> Result<Vec<f64>> {
        let mut out = vec![0.0; self.output_len(input.len())?];
        self.resample_into(input, &mut out)?;
        Ok(out)
    }

    /// `input` をリサンプルして `out` に書き込む。`out` の長さは
    /// `self.output_len(input.len())` に一致しなければならない。
    ///
    /// **RT 契約:** RT-suitable — 事前確保済み係数表のみ使用し、
    /// alloc なし・有界。
    ///
    /// # Errors
    ///
    /// `out.len()` が `output_len(input.len())` と異なれば
    /// `Error::InvalidValue("length")`。
    pub fn resample_into(&self, input: &[f64], out: &mut [f64]) -> Result<()> {
        let length = self.output_len(input.len())?;
        if out.len() != length {
            return Err(Error::InvalidValue("length"));
        }
        if self.source_rate == self.target_rate {
            out.copy_from_slice(input);
            return Ok(());
        }
        let taps = self.taps();
        let radius = self.radius as i128;
        for (n, o) in out.iter_mut().enumerate() {
            let position = n as u128 * u128::from(self.down);
            let center = i128::try_from(position / u128::from(self.up)).unwrap_or(i128::MAX);
            let phase = usize::try_from(position % u128::from(self.up)).unwrap_or(0);
            let coefficients = &self.phases[phase * taps..phase * taps + taps];
            // ISA に依存しない順序で積和する。FMA を使わない。
            let mut acc = 0.0f64;
            for (i, &c) in coefficients.iter().enumerate() {
                let index = center + i as i128 - radius;
                let value = usize::try_from(index)
                    .ok()
                    .and_then(|i| input.get(i))
                    .copied()
                    .unwrap_or(0.0);
                acc += value * c;
            }
            *o = acc;
        }
        Ok(())
    }

    /// 位相 `p` の正規化済み係数（`taps` 要素）。
    #[expect(clippy::cast_precision_loss)]
    fn phase_coefficients(&self, phase: u32) -> Vec<f64> {
        let fraction = f64::from(phase) / f64::from(self.up);
        let radius = isize::try_from(self.radius).unwrap_or(0);
        let mut coefficients: Vec<f64> = (-radius..=radius)
            .map(|k| {
                let x = k as f64 - fraction;
                let w = x / self.radius as f64;
                if w.abs() > 1.0 {
                    return 0.0;
                }
                let window = 0.42 + 0.5 * (PI * w).cos() + 0.08 * (2.0 * PI * w).cos();
                let z = PI * self.cutoff * x;
                let sinc = if z == 0.0 { 1.0 } else { z.sin() / z };
                self.cutoff * sinc * window
            })
            .collect();
        let sum: f64 = coefficients.iter().sum();
        for c in &mut coefficients {
            *c /= sum;
        }
        coefficients
    }
}

#[cfg(test)]
// テストはインデックス→値の変換を多用するため usize→f64 キャストを許容する。
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    /// 有限区間で正弦振幅を推定する（sin/cos への最小二乗投影）。
    fn sine_amplitude(x: &[f64], normalized_freq: f64) -> f64 {
        let mut s_sin = 0.0;
        let mut s_cos = 0.0;
        let mut x_sin = 0.0;
        let mut x_cos = 0.0;
        for (i, &v) in x.iter().enumerate() {
            let s = (2.0 * PI * normalized_freq * i as f64).sin();
            let c = (2.0 * PI * normalized_freq * i as f64).cos();
            s_sin += s * s;
            s_cos += c * c;
            x_sin += v * s;
            x_cos += v * c;
        }
        let a = x_sin / s_sin;
        let b = x_cos / s_cos;
        a.hypot(b)
    }

    #[test]
    fn identity_rate_copies_input_and_is_ratio_reduced() {
        let plan = ResamplePlan::new(48000, 48000).unwrap();
        assert_eq!((plan.up(), plan.down()), (1, 1));
        let input: Vec<f64> = (0..100).map(|i| f64::from(i) * 0.01 - 0.5).collect();
        assert_eq!(plan.resample(&input).unwrap(), input);
        let mut out = vec![0.0; input.len()];
        plan.resample_into(&input, &mut out).unwrap();
        assert_eq!(out, input);
        // 有理比の約分: 48000→96000 は 2x、44100Hz は 147:160。
        let up2 = ResamplePlan::new(48000, 96000).unwrap();
        assert_eq!((up2.up(), up2.down()), (2, 1));
        let to441 = ResamplePlan::new(48000, 44100).unwrap();
        assert_eq!((to441.up(), to441.down()), (147, 160));
    }

    #[test]
    fn output_len_is_ceil_and_overflows_to_error() {
        let plan = ResamplePlan::new(48000, 44100).unwrap();
        // ceil(1000·147/160) = ceil(918.75) = 919。
        assert_eq!(plan.output_len(1000).unwrap(), 919);
        assert_eq!(plan.output_len(0).unwrap(), 0);
        let up2 = ResamplePlan::new(48000, 96000).unwrap();
        // usize::MAX·2 は usize に収まらず "length" エラーになる。
        assert_eq!(
            up2.output_len(usize::MAX),
            Err(Error::InvalidValue("length"))
        );
        assert_eq!(up2.resample(&[]).unwrap(), Vec::<f64>::new());
    }

    #[test]
    fn invalid_parameters_and_lengths_are_errors() {
        assert_eq!(
            ResamplePlan::new(0, 48000).err(),
            Some(Error::InvalidValue("rate"))
        );
        assert_eq!(
            ResamplePlan::new(48000, 0).err(),
            Some(Error::InvalidValue("rate"))
        );
        let plan = ResamplePlan::new(48000, 96000).unwrap();
        let input = vec![0.0; 100];
        let mut out = vec![0.0; 199];
        assert_eq!(
            plan.resample_into(&input, &mut out),
            Err(Error::InvalidValue("length"))
        );
    }

    #[test]
    fn dc_passes_with_unity_gain_in_interior() {
        // 位相係数は正規化済みなので DC は内部で厳密に保存される。
        // 端点はゼロ拡張により減衰する（仕様）。
        let plan = ResamplePlan::new(48000, 96000).unwrap();
        let input = vec![1.0; 200];
        let out = plan.resample(&input).unwrap();
        assert_eq!(out.len(), 400);
        let radius_out = plan.radius() * plan.up() as usize;
        for (i, &v) in out.iter().enumerate().skip(radius_out).take(200) {
            assert!((v - 1.0).abs() < 1e-12, "interior out[{i}] = {v}");
        }
        // 端点は減衰する（ゼロ拡張の影響。末尾サンプルは ≈0.5）。
        assert!(out[0] < 1.0);
        assert!(*out.last().unwrap() < 0.9);
    }

    #[test]
    fn upsampled_sine_tracks_source_phase() {
        // 2x 補間: out[n] ≈ x(n/2)。群遅延 0 なので位相も一致する。
        let plan = ResamplePlan::new(48000, 96000).unwrap();
        let f = 0.1; // 源 Nyquist 正規化周波数（cutoff 0.94 以下）
        let input: Vec<f64> = (0..512)
            .map(|i| (2.0 * PI * f * f64::from(i)).sin())
            .collect();
        let out = plan.resample(&input).unwrap();
        assert_eq!(out.len(), 1024);
        // 内部区間で、期待値 sin(2π·f·n/2) と一致。
        let margin = plan.radius() * plan.up() as usize + 8;
        let mut max_err = 0.0f64;
        for (n, &v) in out
            .iter()
            .enumerate()
            .skip(margin)
            .take(out.len() - 2 * margin)
        {
            let expected = (2.0 * PI * f * n as f64 / 2.0).sin();
            max_err = max_err.max((v - expected).abs());
        }
        assert!(max_err < 1e-3, "max_err = {max_err}");
    }

    #[test]
    fn downsampling_attenuates_alias_but_passes_baseband() {
        // 8:1 間引き。新 Nyquist は源の 1/8 = 0.125、cutoff ≈ 0.1175。
        let plan = ResamplePlan::new(48000, 6000).unwrap();
        assert_eq!((plan.up(), plan.down()), (1, 8));
        let n = 8192;
        // 通過帯域内の低周波（源正規化 0.02 < cutoff）。
        let low: Vec<f64> = (0..n)
            .map(|i| (2.0 * PI * 0.02 * f64::from(i)).sin())
            .collect();
        // エイリアス領域の高周波（源正規化 0.4 > cutoff）。
        let high: Vec<f64> = (0..n)
            .map(|i| (2.0 * PI * 0.4 * f64::from(i)).sin())
            .collect();
        let out_low = plan.resample(&low).unwrap();
        let out_high = plan.resample(&high).unwrap();
        assert_eq!(out_low.len(), 1024);
        let radius_out = plan.radius() * plan.up() as usize;
        let interior_low = &out_low[radius_out / 8 + 4..out_low.len() - radius_out / 8 - 4];
        // 低周波はほぼそのままの振幅で通過（出力側正規化周波数は 0.02·8 = 0.16）。
        let amp_low = sine_amplitude(interior_low, 0.16);
        assert!((amp_low - 1.0).abs() < 0.05, "passband amplitude {amp_low}");
        // 高周波は強く減衰（Blackman-sinc の阻止帯域減衰）。
        let amp_high = sine_amplitude(&out_high[radius_out / 8 + 4..], 0.4 * 8.0 % 1.0);
        assert!(amp_high < 1e-3, "stopband amplitude {amp_high}");
    }

    #[test]
    fn plan_reports_shape_and_memory() {
        let plan = ResamplePlan::new(48000, 96000).unwrap();
        // ratio = 1 → radius = 32、taps = 65、係数表は up·taps。
        assert_eq!(plan.radius(), 32);
        assert_eq!(plan.taps(), 65);
        assert_eq!(plan.memory_bytes(), 2 * 65 * size_of::<f64>());
        assert_eq!(plan.cutoff(), 0.94);
        // 8:1 では radius = 32/0.125 = 256。
        let dec = ResamplePlan::new(48000, 6000).unwrap();
        assert_eq!(dec.radius(), 256);
        assert_eq!(dec.taps(), 513);
    }
}
