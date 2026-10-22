---
id: "DESIGN-034JQxPUJjVzr5aekaU1B8"
type: "design"
status: "approved"
title: "SIMD dispatch 層設計（トークン繰り上げ + ルーティング + 公開 API 形）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-math/src/dsp.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-math/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-math/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-math/src/simd_poc.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_equiv.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/simd-dispatch-layer.md"
  implemented-by: []
  depends-on:
    - "../adr/dsp-simd-backend-adoption.md"
    - "../adr/simd-ulp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
  refines:
    - "ruviform-architecture-design-baseline.md"
  refined-by: []
  derives-from:
    - "docs/specs/simd-dispatch-layer.md"
    - "../adr/simd-ulp.md"
  derived-by:
    - "../plans/implement-issue-53-simd-dispatch-layer.md"
    - "docs/designs/ruviform-dsp.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# SIMD dispatch 層設計（トークン繰り上げ + ルーティング + 公開 API 形）

## Context

採否 ADR 決定 1–3 で確定した「スカラー参照 + ランタイム分岐 + フォールバック」の二層構造を、`ruviform-math` の本番公開 API として実装する設計。設計ベースライン A.4 の構造仮説（起動時検出・トークンキャッシュ・dispatch 層の `ruviform-math` 集約）を具体化する。

#56 の計測結果が設計を直接規定する:

- `dispatch!(Level::new(), …)` の per-call 全経路コスト = **1.29ns**（プローブ安定値）
- 実カーネル経路の固定コストは **数 ns**（add 小 N で +6–7ns）— `dispatch!` 選択自体より `#[simd]`（`#[target_feature]` 境界）経由の非インライン呼出しが支配的
- したがって設計は「トークンを prepare 境界に繰り上げて dispatch 経路全体を除去」+「固定コストが埋まらない小 N 帯をスカラーへルーティング」の二本立て

## Scope

### In Scope

- `ruviform_math::dsp` 公開モジュール: `SimdToken` 型 + 8 カーネル（`add`/`scale`/`clamp_unit`/`fade`/`dot`/`fir`/`biquad`/`sine`）
- トークン内部での確定済み ISA 選択（AVX2 / SSE4.2 / SSE2 / NEON / Fallback）
- カーネル・サイズ別スカラールーティング（閾値は本書 §ルーティングで確定値化）
- 公開 doc への精度契約明記（`simd-ulp.md` ADR の Option A: ulp 固定 + 有限入力前提）
- `fearless_simd` の `[dependencies]` 昇格

### Out of Scope

- AVX-512 arm（未計測 — Issue #55 で実機計測後に arm 追加）
- denormal 性能対策（Issue #54 — エンジン層責務）
- engine 層への接続、`pulp`/`multiversion` 再評価
- `simd_poc` モジュールの再編（実験資産として温存。`dsp` がスカラー参照実装・係数型を内部再利用するのみ）

## Components and Boundaries

### `SimdToken`（公開・不透明）

```rust
#[derive(Clone, Copy, Debug)]
pub struct SimdToken { sel: Sel }
```

- `Sel` はクレート内部 enum: `#[cfg]` ゲートつき `Avx2(Avx2)` / `Sse4_2(Sse4_2)` / `Sse2(Sse2)`（x86 系）・`Neon(Neon)`（aarch64）・常設の終端 `Fallback(Fallback)`。バックエンド型は variant payload としてのみ存在し公開 API に露出しない
- `SimdToken::detect()`: `Level::new()` から `as_avx2() → as_sse4_2() → as_sse2()`（x86 系）/ `as_neon()`（aarch64）の順に試行、いずれも不可なら `Fallback(Fallback::new())`。AVX-512 マシンでは `as_avx2()` が superset 降格で `Some` を返すため Avx512 arm 不設置でも最適経路に乗る
- `Copy + Clone + Debug + Send + Sync`（トークンはすべて ZST）
- `#[doc(hidden)] pub fn force_*` コンストラクタ群（`force_fallback` / `force_sse2` / `force_sse4_2` / `force_avx2` / `force_neon` — `Option` 返し）: 等価性 matrix と bench が強制経路を構築するために必要。公開だが doc 非表示（実験・検証専用と明記）
- `pub fn level(&self) -> &'static str`: 診断・テスト用の ISA 名（`"avx2"` 等）

### カーネル関数群（`dsp` モジュール直下、トークン第 1 引数）

```rust
pub fn add(simd: SimdToken, dst: &mut [f32], a: &[f32], b: &[f32]);
pub fn scale(simd: SimdToken, dst: &mut [f32], src: &[f32], gain: f32);
pub fn clamp_unit(simd: SimdToken, dst: &mut [f32], src: &[f32]);
pub fn fade(simd: SimdToken, dst: &mut [f32], src: &[f32], g0: f32, step: f32);
pub fn dot(simd: SimdToken, a: &[f32], b: &[f32]) -> f32;
pub fn fir(simd: SimdToken, dst: &mut [f32], src: &[f32], taps: &[f32]);
pub fn biquad(simd: SimdToken, dst: &mut [f32], src: &[f32], c: &BiquadCoeffs, state: &mut [f32], channels: usize);
pub fn sine(simd: SimdToken, dst: &mut [f32], c: &SineCoeffs, state: &mut SineState);
```

- 各関数は①ルーティング閾値判定 → ②`match simd.sel` で確定済み concrete token → `#[simd]` モノモーフ化カーネル呼出し。`dispatch!` も `Level::new()` も per-call では実行しない
- `BiquadCoeffs` / `SineCoeffs` / `SineState` は `simd_poc` の既存型を `dsp` から `pub use` で昇格（PoC 側の参照はそのまま機能）
- panic-free 契約: 長さ不一致・`channels == 0` 等の契約違反は `debug_assert` で捕捉し、release では共通プレフィックス処理/`channels` の `state.len()/2` クランプで非 panic に統一（PoC の `add` はこの形式、fir/biquad は release panic — 本番面では統一して非 panic とする。契約外入力の silently-partial 処理は「0 を書き込む」より悪くない副作用なしとして採用）
- `unsafe` 不使用: `as_*()` / `Fallback::new()` / `#[simd]` 経由のみ（`assume_supported` 等は呼ばない — 降格判定の内部 unsafe は fearless_simd 側の責務）

### `#[simd]` カーネル本体（`dsp` 内部 `fn *_impl<S: Simd>`）

PoC の `tests/simd_poc/fearless_impl.rs` 8 カーネルを本番面へ移植。PoC 側は凍結された実験資産として残す（matrix の `fearless_*` 列は PoC 実装を測っている記録であり、本番 `dsp` とは意図的に別実装 — 等価性 matrix が両者をスカラー参照へ照合して乖離を検出する）。

### スカラー参照経路

`crate::simd_poc::{add, scale, …}` の既存スカラー実装を契約の「正」として内部呼出し（移動・複製しない）。ルーティング先・等価性検証の参照・`Fallback` variant の実体として共用。

## Data and Control Flow

```
engine prepare        process（RT、per buffer）
    │                      │
SimdToken::detect() ──► dsp::add(tok, dst, a, b)
  Level::new()             │ n < threshold? ──yes──► simd_poc::add (scalar, 契約の正)
  as_avx2()/as_*()         │ no
  → Sel::Avx2(t)           ▼ match sel { Avx2(t) => add_impl::<Avx2>(t, …) }
  （以後再利用）            （単一の予測可能タグ分岐 + 確定済みトークン呼出し）
```

## ルーティング規則（確定値）

根拠データ: `dispatched-overhead-20261021` 3-run 中央値（i9-9900K・低負荷）。「scalar/avx2 比」列で scalar が劣後しない帯を scalar ルートにする。

| カーネル | 閾値 | 根拠（scalar/avx2 中央値比） |
|---|---|---|
| `add` | `n < 128` → scalar | n=16: 0.62x・32: 0.79x・64: 1.03x・100: 1.00x（scalar が負けない）。256 で avx2 が 1.36x に転じる |
| `scale` | `n < 64` → scalar | 計測最小 n=64 で avx2 1.17x と既に勝つ。固定呼出しコスト ~2-3ns を考慮した保守的下限（未計測帯の小さい方のみ保護） |
| `clamp_unit` | `n < 256` → scalar | n=64: 0.79x・100: 0.76x（scalar 勝）、256 以降 parity（メモリ帯域律速で SIMD が伸びない） |
| `fade`/`dot`/`fir`/`biquad`/`sine` | 常に token 経路 | fade: avx2 が 3.9–6.7x で全域支配。dot/fir/biquad: 演算律速で採否 ADR の実測（dot 7.9–8.3x・fir 3.6–5.8x・biquad ~2.8x）。sine は逐次再帰で全経路同一ループ |

- 閾値は `const` として `dsp` 内に定義し、上表の計測根拠を doc コメントに転記（マジックナンバー化しない）
- 閾値は ISA・CPU 依存のヒューリスティックであり「性能上の保守的選択」であって契約ではない — 公開 doc には精度契約のみ記載し、性能ルーティングは実装詳細として扱う

## 精度契約（`simd-ulp.md` ADR Option A の適用）

各公開関数の doc に以下を明記:

| カーネル | 契約 |
|---|---|
| `add`/`scale`/`clamp_unit`/`fade` | スカラー参照と **bit 完全一致**（NaN は両側 `is_nan` 一致） |
| `dot` | ulp ≤ **256**（有限入力・通常の打ち消しなし入力クラス） |
| `fir`/`biquad` | ulp ≤ **8**（同上） |
| `sine` | bit 完全一致（逐次再帰 — dispatch 層の codegen 差異が出れば finding） |

前提条件（全カーネル共通）: 入力は finite f32。NaN/Inf は契約外（結果は伝播するが精度保証の対象外）。強打ち消し入力では固定 ulp 上限を超え得る — その入力域の保証が必要になった時点で契約形を再評価する（事後緩和せず不具合/契約不適合として調査するプロトコル）。

## `fearless_simd` 昇格

- `[dependencies]` に `fearless_simd = "1.1"`（feature なし）
- `[dev-dependencies]` の既存 `fearless_simd = { version = "1.1", features = ["force_support_fallback"] }` + `fearless_simd_macros = "0.1"` は維持（bench が `force_support_fallback` と macro に依存。cargo が feature を統合し、dev ビルドでは fallback 強制が有効、本番ビルドでは無効）
- 前提: workspace MSRV 1.89（#52 — 本ブランチのベース）。`Cargo.lock` 更新を含む

## Verification

- [x] `cargo check -p ruviform-math` で `fearless_simd` が `[dependencies]` 解決され、公開 API に fearless 型が露出しない（`cargo doc` / 公開シグネチャ目視）
- [x] `SimdToken::detect()` が AVX2 マシンで `level() == "avx2"` を返す
- [x] forced-token 単体テストで各 ISA 経路が全サイズ帯で契約適合（ルーティングのマスクを受けない内部 `*_impl` 直接検証）
- [x] `tests/simd_equiv.rs` に本番経路列（`prod_detected` + forced 各列）を追加してグリーン
- [x] `rt_` プレフィックステストで `assert_no_alloc` 測定窓内のカーネル呼出しが 0 allocation（#22 レーンと同機構）
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke` パス
- [x] doc 監査で本チェーン起因 findings 0

## Alternatives Considered

- **per-call `dispatch!` 維持**: #56 で固定コスト実在が確定したため不採用
- **engine 側ジェネリクス化**（`Processor<S: Simd>`）: 呼出し境界の完全インライン化は可能だが、backend 型を公開 API に露出させる crate 契約違反 + engine 層への型汚染。不採用
- **block 単位 dispatch**（process 全体を一つの `dispatch!` 内に）: 本 API 粒度では実現不可。spec の Deferred として記録済み
- **スカラールーティングなし**（常に token 経路）: add/clamp の小 N で scalar が実測勝つため不採用
- **Level enum の直接 match**（`match Level::new()`）: `#[non_exhaustive]` で wildcard arm が必須 + Avx512 を落とすために結局 `as_*` 判定が必要になる。`as_*` 逐次試行の方が単純
