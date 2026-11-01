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
//!
//! # 安定パラメータ表現（永続・cache 鍵向け）
//!
//! [`ResampleParams`] は係数表を含まない正準パラメータ値で、変換の
//! 意味的同一性を表す。決定的な [`ResampleParams::stable_id`]（正準
//! 直列化文字列）は同一パラメータ → 同一文字列を全プラットフォームで
//! 保証し、[`ResampleParams::from_stable_id`] が逆変換と整合性検証を
//! 担う。永続スキーマ・cache 鍵は内部表現（係数表）ではなくこの
//! 正準表現へ依存させる（Issue #170、audio-lens #102 R4）。

use crate::{Error, Result};
use core::{fmt, str::FromStr};
use std::f64::consts::PI;

/// [`ResamplePlan`] が実行する変換の正準パラメータ表現。
///
/// 係数表などの内部表現を含まない、変換を一意に決める意味的
/// パラメータ集合。`PartialEq`/`Eq`/`Hash` で等値・キー比較でき、
/// [`stable_id`](Self::stable_id) が決定的な識別文字列を返す。
/// 永続スキーマ・cache 鍵はこの正準表現へ依存させる
/// （Issue #170、audio-lens #102 R4）。
///
/// # 安定性契約
///
/// - 同一パラメータ → 同一 stable ID。決定的・プラットフォーム非依存
///   （`cutoff` は IEEE-754 bit 表現で保持・直列化し、浮動小数表記に
///   依存しない）。
/// - 等価なパラメータは係数表レイアウト等の内部表現が異なっても
///   同一 stable ID。係数導出に使わない実装内部表現の変更では ID は
///   変わらない。
/// - パラメータ集合・直列化形式・変換意味論の後方非互換な変更は
///   [`CONTRACT_VERSION`](Self::CONTRACT_VERSION) を増分する。version は
///   stable ID に埋め込まれるため、永続済み ID は新契約と衝突せず
///   自然に無効化される（前方互換の仕組み）。
/// - stable ID 文字列は opaque に扱う。version 以降のフィールド構成は
///   version ごとの契約であり、呼出側で分解・再構成しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResampleParams {
    source_rate: u32,
    target_rate: u32,
    up: u32,
    down: u32,
    radius: usize,
    /// `cutoff` の `f64::to_bits`。値の bit-exact 同一性を Eq/Hash で扱う。
    cutoff_bits: u64,
}

impl ResampleParams {
    /// 安定表現の契約バージョン（stable ID の 2 番目フィールド）。
    ///
    /// パラメータ集合・直列化形式・変換意味論のいずれかを後方非互換に
    /// 変える変更で増分する。同一バージョン内では「同一パラメータ →
    /// 同一 stable ID」が保証される。
    pub const CONTRACT_VERSION: u32 = 1;

    /// provenance 記録用のアルゴリズム識別子（固定トークン）。
    ///
    /// 永続スキーマへアルゴリズム名を記録する用途向け。
    /// `AudioLens` の `normalized_pcm` が記録してきた値と同一。
    pub const ALGORITHM_ID: &str = "polyphase-blackman-sinc";

    /// stable ID の先頭タグ（直列化契約名）。
    const STABLE_ID_TAG: &str = "rvf-resample";

    /// `source_rate` → `target_rate` の正準パラメータを導出する。
    /// 係数表は構築しないため、係数表確保が現実的でない極端な有理比でも
    /// 識別子を得られる。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
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
        Ok(Self {
            source_rate,
            target_rate,
            up: target_rate / a,
            down: source_rate / a,
            radius: (32.0 / ratio).ceil() as usize,
            cutoff_bits: (0.94 * ratio).to_bits(),
        })
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
        f64::from_bits(self.cutoff_bits)
    }

    /// `frames` 入力サンプルの出力長（`ceil(frames·up/down)`）。
    /// [`ResamplePlan::output_len`] と同一規約。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// 長さが `usize` に収まらなければ `Error::InvalidValue("length")`。
    pub fn output_len(&self, frames: usize) -> Result<usize> {
        usize::try_from((frames as u128 * u128::from(self.up)).div_ceil(u128::from(self.down)))
            .map_err(|_| Error::InvalidValue("length"))
    }

    /// 正準直列化（stable ID）。`self.to_string()` と同一。
    ///
    /// 同一バージョン内では決定的・プラットフォーム非依存。
    ///
    /// **RT 契約:** non-RT only — `String` を確保する。
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_numeric::resample::ResampleParams;
    ///
    /// let params = ResampleParams::new(48000, 96000).unwrap();
    /// assert_eq!(
    ///     params.stable_id(),
    ///     "rvf-resample/1/48000/96000/2/1/32/3fee147ae147ae14",
    /// );
    /// // 逆変換は形式・version・フィールド整合性を検証する。
    /// assert_eq!(
    ///     ResampleParams::from_stable_id(&params.stable_id()).unwrap(),
    ///     params,
    /// );
    /// ```
    #[must_use]
    pub fn stable_id(&self) -> String {
        self.to_string()
    }

    /// `stable_id` の逆変換。[`FromStr`] 実装と同一。
    ///
    /// 形式・[`CONTRACT_VERSION`](Self::CONTRACT_VERSION)・全フィールドの
    /// 整合性（`new(source_rate, target_rate)` の再導出と一致）を検証し、
    /// 正準スペルでない・手組みの・version 不明の ID を拒否する。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// 形式不正・未対応 version・フィールド不整合は
    /// `Error::InvalidValue("params")`。レートが 0 なら
    /// [`new`](Self::new) と同じく `Error::InvalidValue("rate")`。
    pub fn from_stable_id(id: &str) -> Result<Self> {
        id.parse()
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
                let z = PI * self.cutoff() * x;
                let sinc = if z == 0.0 { 1.0 } else { z.sin() / z };
                self.cutoff() * sinc * window
            })
            .collect();
        let sum: f64 = coefficients.iter().sum();
        for c in &mut coefficients {
            *c /= sum;
        }
        coefficients
    }
}

/// `stable_id` の正準形式を `Display`/`FromStr` で提供する。
/// 形式:
/// `rvf-resample/<version>/<source>/<target>/<up>/<down>/<radius>/<cutoff-bits-hex16>`。
impl fmt::Display for ResampleParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}/{}/{}/{}/{}/{:016x}",
            Self::STABLE_ID_TAG,
            Self::CONTRACT_VERSION,
            self.source_rate,
            self.target_rate,
            self.up,
            self.down,
            self.radius,
            self.cutoff_bits
        )
    }
}

impl FromStr for ResampleParams {
    type Err = Error;

    /// 正準形式のみ受理する（`stable_id` の逆変換）。
    /// タグ・version・正準スペル・全フィールドの導出整合性を検証する。
    fn from_str(id: &str) -> Result<Self> {
        let invalid = Error::InvalidValue("params");
        let mut fields = id.split('/');
        if fields.next() != Some(Self::STABLE_ID_TAG) {
            return Err(invalid);
        }
        if stable_decimal::<u32>(fields.next())? != Self::CONTRACT_VERSION {
            return Err(invalid);
        }
        let parsed = Self {
            source_rate: stable_decimal(fields.next())?,
            target_rate: stable_decimal(fields.next())?,
            up: stable_decimal(fields.next())?,
            down: stable_decimal(fields.next())?,
            radius: stable_decimal(fields.next())?,
            cutoff_bits: stable_hex16(fields.next())?,
        };
        if fields.next().is_some() || parsed != Self::new(parsed.source_rate, parsed.target_rate)? {
            return Err(invalid);
        }
        Ok(parsed)
    }
}

/// `stable_id` フィールドの正準 10 進表記（先頭ゼロなし・数字のみ）を
/// parse する。非正準スペル・桁外・欠落は `Error::InvalidValue("params")`。
fn stable_decimal<T: FromStr>(field: Option<&str>) -> Result<T> {
    let invalid = || Error::InvalidValue("params");
    let v = field.ok_or_else(invalid)?;
    if v.bytes().any(|b| !b.is_ascii_digit()) || (v.len() > 1 && v.starts_with('0')) {
        return Err(invalid());
    }
    v.parse().map_err(|_| invalid())
}

/// `stable_id` フィールドの正準 16 桁小文字 hex を parse する。
/// 16 桁固定のため `u64` に収まる。
fn stable_hex16(field: Option<&str>) -> Result<u64> {
    let invalid = || Error::InvalidValue("params");
    let v = field.ok_or_else(invalid)?;
    if v.len() != 16 || v.bytes().any(|b| !matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(invalid());
    }
    u64::from_str_radix(v, 16).map_err(|_| invalid())
}

/// 係数を完全に再生成できる変換設定。端点はゼロ拡張、長さは ceil。
///
/// 変換の意味的同一性は [`params`](Self::params)（[`ResampleParams`]）で
/// 判定・直列化する。永続スキーマ・cache 鍵は
/// [`stable_id`](Self::stable_id) を使う。
#[derive(Debug, Clone, PartialEq)]
pub struct ResamplePlan {
    /// 変換の正準パラメータ（係数表以外の全決定要素）。
    params: ResampleParams,
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
    pub fn new(source_rate: u32, target_rate: u32) -> Result<Self> {
        Ok(Self::from_params(ResampleParams::new(
            source_rate,
            target_rate,
        )?))
    }

    /// 正準パラメータから plan を構築する（係数表を確保する）。
    /// `Self::new(params.source_rate(), params.target_rate())` と等価だが、
    /// 検証済みの正準値をそのまま使う。`stable_id` から復元した
    /// [`ResampleParams`] の再構築入口。
    ///
    /// **RT 契約:** non-RT only — 全位相の係数表を確保する。
    ///
    /// # Panics
    ///
    /// `params.up()` がプラットフォームの `usize` に収まらない場合。
    #[must_use]
    pub fn from_params(params: ResampleParams) -> Self {
        // u32 -> usize を明示的に検証し、狭いポインタ幅での切り捨てを防ぐ。
        let phase_count = usize::try_from(params.up()).expect("resample phase count exceeds usize");
        let mut phases = Vec::with_capacity(phase_count * params.taps());
        for p in 0..params.up() {
            phases.extend(params.phase_coefficients(p));
        }
        Self { params, phases }
    }

    /// この plan の正準パラメータ（永続・識別用の安定表現）。
    ///
    /// **RT 契約:** RT-suitable — `Copy` 値を返すのみ。
    #[must_use]
    pub fn params(&self) -> ResampleParams {
        self.params
    }

    /// この plan の変換を一意に識別する stable ID。
    /// `self.params().stable_id()` と同一。
    ///
    /// **RT 契約:** non-RT only — `String` を確保する。
    #[must_use]
    pub fn stable_id(&self) -> String {
        self.params.stable_id()
    }

    /// 変換元サンプルレート。
    #[must_use]
    pub fn source_rate(&self) -> u32 {
        self.params.source_rate()
    }

    /// 変換先サンプルレート。
    #[must_use]
    pub fn target_rate(&self) -> u32 {
        self.params.target_rate()
    }

    /// 約分後の補間比（分子側）。
    #[must_use]
    pub fn up(&self) -> u32 {
        self.params.up()
    }

    /// 約分後の間引き比（分母側）。
    #[must_use]
    pub fn down(&self) -> u32 {
        self.params.down()
    }

    /// FIR の半径（タップ数は `2·radius + 1`）。
    #[must_use]
    pub fn radius(&self) -> usize {
        self.params.radius()
    }

    /// 1 位相あたりのタップ数（`2·radius + 1`）。
    #[must_use]
    pub fn taps(&self) -> usize {
        self.params.taps()
    }

    /// sinc の正規化遮断（`0.94·min(target/source, 1)`）。
    #[must_use]
    pub fn cutoff(&self) -> f64 {
        self.params.cutoff()
    }

    /// 係数表のメモリ量（バイト、構造見積もり）。
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.phases.len() * size_of::<f64>()
    }

    /// `frames` 入力サンプルの出力長（`ceil(frames·up/down)`）。
    ///
    /// **RT 契約:** RT-suitable — alloc なし・有界。
    ///
    /// # Errors
    ///
    /// 長さが `usize` に収まらなければ `Error::InvalidValue("length")`。
    pub fn output_len(&self, frames: usize) -> Result<usize> {
        self.params.output_len(frames)
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
        if self.params.source_rate() == self.params.target_rate() {
            out.copy_from_slice(input);
            return Ok(());
        }
        let taps = self.taps();
        let radius = self.params.radius() as i128;
        for (n, o) in out.iter_mut().enumerate() {
            let position = n as u128 * u128::from(self.params.down());
            let center =
                i128::try_from(position / u128::from(self.params.up())).unwrap_or(i128::MAX);
            let phase = usize::try_from(position % u128::from(self.params.up())).unwrap_or(0);
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

    #[test]
    fn stable_id_is_golden_and_deterministic() {
        let params = ResampleParams::new(48000, 96000).unwrap();
        // 正準形式を golden で固定（形式・導出規則の退行を検出する）。
        assert_eq!(
            params.stable_id(),
            "rvf-resample/1/48000/96000/2/1/32/3fee147ae147ae14"
        );
        // Display と stable_id は同一。
        assert_eq!(params.to_string(), params.stable_id());
        // CONTRACT_VERSION は stable ID の 2 番目フィールドに埋め込まれる。
        assert!(params.stable_id().starts_with(&format!(
            "rvf-resample/{}/",
            ResampleParams::CONTRACT_VERSION
        )));
        // 係数表を持つ plan からも同一 ID が導出される（内部表現の
        // 係数表ではなく意味的パラメータから導出される契約）。
        let plan = ResamplePlan::new(48000, 96000).unwrap();
        assert_eq!(plan.params(), params);
        assert_eq!(plan.stable_id(), params.stable_id());
    }

    #[test]
    fn equivalent_parameters_share_stable_id() {
        // 別々に構築した等価 plan は同一 params/ID を持つ。
        let a = ResamplePlan::new(48000, 44100).unwrap();
        let b = ResamplePlan::new(48000, 44100).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.params(), b.params());
        assert_eq!(a.stable_id(), b.stable_id());
        // from_params で再構築した plan も等価（永続経路の再構築）。
        let rebuilt = ResamplePlan::from_params(a.params());
        assert_eq!(rebuilt, a);
        assert_eq!(rebuilt.stable_id(), a.stable_id());
    }

    #[test]
    fn different_parameters_have_different_ids() {
        let ids: Vec<String> = [
            (48000, 96000),
            (48000, 44100),
            (96000, 48000),
            (48000, 48000),
            (44100, 88200),
        ]
        .iter()
        .map(|&(s, t)| ResampleParams::new(s, t).unwrap().stable_id())
        .collect();
        for (i, a) in ids.iter().enumerate() {
            for b in ids.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn stable_id_round_trips_through_parse() {
        for &(s, t) in &[
            (48000u32, 96000u32),
            (48000, 44100),
            (44100, 48000),
            (48000, 48000),
            (48000, 6000),
        ] {
            let params = ResampleParams::new(s, t).unwrap();
            let id = params.stable_id();
            // from_stable_id と FromStr の両経路で同一値へ戻る。
            assert_eq!(ResampleParams::from_stable_id(&id).unwrap(), params);
            assert_eq!(id.parse::<ResampleParams>().unwrap(), params);
            // plan 経由でも同一 params。
            assert_eq!(ResamplePlan::new(s, t).unwrap().params(), params);
            // 復元した params の派生値も plan と一致する。
            let restored = ResampleParams::from_stable_id(&id).unwrap();
            assert_eq!(restored.output_len(1000), params.output_len(1000));
            assert_eq!(restored.taps(), params.taps());
            assert_eq!(restored.cutoff(), params.cutoff());
        }
    }

    #[test]
    fn from_stable_id_rejects_malformed_and_inconsistent_ids() {
        let invalid = Err(Error::InvalidValue("params"));
        for id in [
            "",
            "rvf-resample",
            "other/1/48000/96000/2/1/32/3fee147ae147ae14",
            // version 不正（未知・非正準スペル・非数値）。
            "rvf-resample/0/48000/96000/2/1/32/3fee147ae147ae14",
            "rvf-resample/2/48000/96000/2/1/32/3fee147ae147ae14",
            "rvf-resample/01/48000/96000/2/1/32/3fee147ae147ae14",
            "rvf-resample/x/48000/96000/2/1/32/3fee147ae147ae14",
            // フィールド欠落・余分。
            "rvf-resample/1/48000/96000/2/1/32",
            "rvf-resample/1/48000/96000/2/1/32/3fee147ae147ae14/x",
            // 非正準スペル（先頭ゼロ・符号・空白・非数字）。
            "rvf-resample/1/048000/96000/2/1/32/3fee147ae147ae14",
            "rvf-resample/1/+48000/96000/2/1/32/3fee147ae147ae14",
            "rvf-resample/1/48000 /96000/2/1/32/3fee147ae147ae14",
            // 桁外れ。
            "rvf-resample/1/48000/96000/2/1/32/99999999999999999999999999",
            // cutoff hex の非正準スペル（大文字・桁数違い・非 hex）。
            "rvf-resample/1/48000/96000/2/1/32/3FEE147AE147AE14",
            "rvf-resample/1/48000/96000/2/1/32/3fe1eb851eb851e",
            "rvf-resample/1/48000/96000/2/1/32/3fee147ae147ae14c",
            "rvf-resample/1/48000/96000/2/1/32/zzzzzzzzzzzzzzzz",
            // フィールド値は parse 可能だが導出規則と非整合
            // （up/down・radius・cutoff の手組み値を拒否する）。
            "rvf-resample/1/48000/96000/1/2/32/3fee147ae147ae14",
            "rvf-resample/1/48000/96000/2/1/33/3fee147ae147ae14",
            "rvf-resample/1/48000/96000/2/1/32/3fee147ae147ae15",
        ] {
            assert_eq!(
                ResampleParams::from_stable_id(id),
                invalid,
                "expected rejection: {id}"
            );
        }
        // レート 0 は new と同じ "rate" エラー。
        assert_eq!(
            ResampleParams::from_stable_id("rvf-resample/1/0/96000/2/1/32/3fee147ae147ae14"),
            Err(Error::InvalidValue("rate"))
        );
    }

    #[test]
    fn stable_id_survives_extreme_ratio_without_table_alloc() {
        // gcd=1 の極端な有理比は係数表が巨大になり得るが、
        // params は表を構築せず ID だけを導出できる。
        let params = ResampleParams::new(u32::MAX, 1).unwrap();
        assert_eq!((params.up(), params.down()), (1, u32::MAX));
        assert_eq!(
            ResampleParams::from_stable_id(&params.stable_id()).unwrap(),
            params
        );
    }
}
