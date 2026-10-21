---
id: "TSPEC-034JenWCuRngWvxWm00Ls3"
type: "test-spec"
status: "approved"
title: "SIMD 等価性 matrix の契約保証"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/dsp-simd-poc.md"
  derived-by: []
  verifies:
    - "../specs/dsp-simd-poc-adr.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# SIMD 等価性 matrix の契約保証

## Purpose

Issue #19 PoC の等価性検証が保証すべき意図——「スカラー参照を正として、候補バックエンドの各 dispatch level がカーネルごとの等価性契約（bit 完全一致 / ulp 上限 / NaN 条項）を満たす」こと——を、[設計書](../designs/dsp-simd-poc.md)の等価性契約表・matrix 設計から検証意図の正本として固定する。[spec](../specs/dsp-simd-poc-adr.md) の受け入れ基準「カーネルごとの等価性契約（bit 完全一致 or ulp 上限値）が定義され、『スカラー vs 各 dispatch level』の等価性テストが `mise run test` でグリーン」を分解し、将来のバックエンド差し替え・新規カーネル追加・等価基準確定値の引上げ/維持の際に「どの振る舞いを守るべきか」「このテストはまだ必要か」を判断できる形にする。廃止してよいのは、採否 ADR が等価基準確定値を定めたうえで本番カーネルの等価性検証を規定する test-spec / spec に置き換わったとき、または PoC ハーネス自体が除去されたとき。

## Feature

SIMD 等価性 matrix: `crates/ruviform-math` のスカラー参照カーネル 8 本（`add` / `scale` / `clamp_unit` / `fade` / `dot` / `fir` / `biquad` / `sine`）に対し、候補バックエンド実装（`fearless_simd` / `pulp` / `multiversion`）を dispatch level の明示強制で実行し、カーネル × 実装列 × 入力セット × 長さ × 先頭オフセットの全セルでカーネル固有の等価性契約を検証すること。

## Rules

### カーネル別等価性契約（設計書「カーネルセットと等価性契約」表の正本化）

- **`add` / `scale` — bit 完全一致**。非 NaN 入力では出力ビットがスカラー参照と完全一致（`f32::to_bits` 比較）。NaN 入力は `is_nan` 一致（payload 不問）。
- **`clamp_unit` — 非 NaN 入力で bit 完全一致、NaN 入力 → NaN 出力**（`is_nan` 一致）。min/max の NaN 意味論差（fearless_simd の `min`/`max` は NaN・±0 が implementation-defined、pulp は `_mm_min_ps` 系）を吸収するため、`min_precise_*` / `max_precise_*` 系または比較 + select で NaN 伝播をスカラー `f32::clamp`（self が NaN → NaN 返却）に揃える。
- **`fade` — bit 完全一致**。elementwise 構成（`dst[i] = src[i] * (g0 + i*step)`）に両側揃え、fused multiply-add を使わないことを契約として明記する。逐次累積形は drift が大きく本番でも不採なため採用しない。
- **`dot` — ulp ≤ 8（初期値）**。入力は有限値契約: reduction 順序差で Inf への丸め・NaN 化が変わり得るため、edge 入力の NaN/Inf セルは比較対象から除外する（除外条件はテストコードにコメントで明記）。
- **`fir`（64 tap 直接形）— ulp ≤ 8（初期値）**。係数・入力は有限値契約。窓内に Inf/NaN を含む出力は順序非依存で NaN/Inf 伝播するため `is_nan` 一致で判定する。
- **`biquad`（N ch planar IIR・状態持続）— ulp ≤ 8（初期値、FMA 使用差を許容）**。再帰状態で丸め差が増幅・累積し得るため、固定上限で正当な超過が出た場合のみ代替契約形（先頭 N フレームのみ ulp 比較＋以降は相対誤差/出力エネルギー差上限、またはフレーム長依存上限）へ切替える（切替は実測に基づく判断・採否 ADR に記録）。NaN/Inf 入力時は再帰状態が NaN 汚染され以降の全出力へ伝播するため `is_nan` 一致とし、edge 適用セルではケースごとに状態をリセットする。
- **`sine`（2 次再帰発振器）— bit 完全一致**。全候補が同一逐次コードを dispatch 経由で呼ぶだけの構成のため一致するはず。差が出たら dispatch 層の codegen 差として記録する。

### matrix 構造と実行規約

- **実装列**: `scalar`（参照・全 matrix セルの比較先）/ `fearless_fallback` / `fearless_sse2` / `fearless_sse4_2` / `fearless_avx2` / `fearless_avx512` / `fearless_dispatched` / `pulp_scalar` / `pulp_v2` / `pulp_v3` / `pulp_v4` / `pulp_dispatched` / `multiversion`。
- **cfg ゲート**: x86 限定の実装列（`fearless_sse2` / `fearless_sse4_2` / `fearless_avx2` / `fearless_avx512`、`pulp_v2` / `pulp_v3` / `pulp_v4`）は `#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]` で囲み、aarch64 では matrix に現れない。aarch64 では `fearless_neon` 列（aarch64 側の level 強制 API）を持ち、portable 列（`scalar` / `fearless_fallback` / `fearless_neon` / `fearless_dispatched` / `pulp_scalar` / `pulp_dispatched` / `multiversion`）のみが検証される。CI `macos-latest`（aarch64）レーンで `--all-targets` のコンパイルと実行が通る構成を維持する。
- **構成不能 level は panic ではなく skip**: `Option` ベースの強制 API（`Level::new().as_*()` / `pulp::x86::V*.try_new()`）が `None` を返す環境（例: AVX-512 非搭載 CPU）では、そのセルを panic させず skip として記録する。`assume_supported` / `new_unchecked`（unsafe）は使用しない。matrix は「実行環境で構成可能な level」のみを検証するため CI 環境差に対して決定的であり、level ごとの検証可否自体は結果として記録する。
- **`multiversion` 列は dispatched のみ**: 個別 clone を強制実行する API が存在しない（clone は private シンボル）ため、検出環境の最良 clone のみが検証される制限を許容し、この検証網羅性の構造的差を採否 ADR の発見事項として記録する。
- **入力セット**: `normal`（deterministic PRNG・xorshift64*・固定 seed・`[-1.0, 1.0)` 一様乱数）、`edge`（denormal・`+0.0` / `-0.0`・quiet NaN・`±Inf` の混合。カーネル契約上 NaN/Inf が意味を持つもののみ適用）、`denormal`（`normal` の一部要素を denormal 値に置換。`biquad` / `fir` では出力側 denormal 収束も観測対象）。
- **denormal 値はビット構成で生成する**: `f32::from_bits(0x0040_0000)`（≈5.9e-39）や `f32::MIN_POSITIVE * 0.5` で非正規化数を構成する。`MIN_POSITIVE` に小さな係数を掛ける素朴な生成は +0.0 に丸まるため使わない。
- **長さ・オフセット**: 端数系 `{7, 15, 16, 17, 31, 33}`（最大レーン幅 16 = f32x16 想定の ±1・奇数・境界跨ぎ）と実用系 `{100, 255, 1023, 1024, 4096}`。`biquad` は `channels = 8` 固定で frames 側を同系列に変える。先頭オフセット `{0, 1, 3}` 要素（`Vec<f32>` を確保して `&v[off..]` で非アライン先頭を構成）。
- **集約検証と失敗報告**: 各カーネルに対し `(impl, level) × (input, len, offset)` をループし、全セル結果を 1 テスト関数で集約検証する。不一致は `(kernel, impl, level, input, len, offset, index, expected_bits, actual_bits, ulps)` を panic メッセージに含める。
- **観測 max ulp の集計**: ulp 上限契約のカーネルは全 matrix で実測 max ulp を集計する。観測値 > 8 は実装バグか契約不適合として先に調査し、数値を合わせ込むための事後緩和はしない（`biquad` のみ上記の代替契約形への切替を許容）。

## Examples

- Given: AVX-512 非搭載の Windows x64 マシン。When: `fearless_avx512` 列を強制構成する。Then: `Level::new().as_avx512()` が `None` を返し、そのセルは panic せず skip として記録される。
- Given: `macos-latest`（aarch64）CI レーン。When: `mise run test` で matrix を実行する。Then: `fearless_sse2` 等の x86 限定列は matrix に現れず、`scalar` / `fearless_fallback` / `fearless_neon` / `fearless_dispatched` / `pulp_scalar` / `pulp_dispatched` / `multiversion` のみが検証される。
- Given: `edge` 入力と `dot` カーネル。When: NaN/Inf を含むセルを評価する。Then: `dot` は有限値契約のため当該セルは比較対象外となり、除外条件がテストコードのコメントに明記される。`fir` は窓内 NaN/Inf で `is_nan` 一致。
- Given: denormal 入力の生成。When: `f32::from_bits(0x0040_0000)` または `f32::MIN_POSITIVE * 0.5` で値を構成する。Then: 実際に非正規化数（`is_normal() == false`）が生成される。`MIN_POSITIVE` に小さな係数を掛ける素朴な生成は +0.0 に丸まり denormal を生成しない。
- Given: `clamp_unit` に NaN 入力。When: 各実装列で実行する。Then: 全実装で出力が NaN（`is_nan` 一致）。`min_precise_*` / `max_precise_*` 系または等価の NaN 伝播構成が使われている。
- Given: `biquad` に NaN を含む edge 入力。When: 1 ケース実行する。Then: 再帰状態が NaN 汚染され以降の出力すべてが NaN となり `is_nan` 一致で判定。次の edge ケース前に状態がリセットされる。
- Given: `dot` の `normal/1024` セル。When: `pulp_v3` で実行する。Then: スカラー参照との差が ulp ≤ 8 で、実測 ulp が集計対象に記録される。
- Given: `sine` を全実装列で実行。When: bit 完全一致を検証する。Then: 差が出た場合はカーネル実装差ではなく dispatch 層 codegen 差として記録される。

## Guarantees

- スカラー参照を正として、全候補・全構成可能 level の数値振る舞いがカーネル契約内に収まることが機械的に検証される（SIMD 化による暗黙の数値差の混入を防ぐ）。
- NaN / ±0 / Inf / denormal・非アライン先頭・レーン境界 ±1 の端数長という境界入力での挙動差が顕在化する。
- 環境差（CPU 機能の有無・アーキテクチャ）に対し決定的な検証結果が得られる（unavailable level は skip 記録・cfg ゲートで列を制御）。
- 検証網羅性の構造的差（`multiversion` の clone 強制不可）が skip / 記録として可視化され、採否 ADR の判断材料になる。
- 実測 max ulp の集計により、採否 ADR で等価基準確定値を余裕を持って定める根拠データが残る。

## Non-goals

- 性能計測値・スループットの正しさ（ベンチ methodology と CI スモーク運用は [bench-tasks-and-ci-smoke](bench-tasks-and-ci-smoke.md) の責務）。
- 本番公開 API の等価性契約（PoC は `#[doc(hidden)]` 実験 API・dev-dep 封じ込め。確定値の本番適用は採否 ADR 後の後続実装 Issue）。
- `multiversion` の clone 単位網羅検証（API が提供しない構造的制限。採否 ADR の発見事項）。
- RT 契約・alloc 検証（PoC カーネルは RT 契約対象外・`rt_` テスト追加なし — 設計書 Non-Functional Constraints どおり）。
- denormal 対策（FTZ/DAZ）の実装（方針記録までが PoC 範囲、実装はエンジン層 Issue）。
- OS レベルのハードリアルタイム保証。

## Risk

- 等価性契約を失うと、SIMD バックエンドの数値差（reduction 順序・FMA 有無・NaN 意味論差）が黙って本番カーネルに混入し、スカラー参照との乖離が後工程で発覚する。
- skip 規約がなければ環境差（AVX-512 非搭載・aarch64）で matrix が不安定化し、環境由来の失敗と契約違反を区別できなくなる。
- denormal を正しく生成できない（+0.0 への丸め込み）と、境界入力の検証が実質素通りになり denormal 崖・丸め差を見逃す。
- ulp 上限の事後緩和を許すと、実装バグを契約緩和で隠蔽し等価性検証の意味を失う。
- cfg ゲートを欠くと aarch64 でコンパイル・実行が壊れ、CI macOS レーンが常時失敗する。
