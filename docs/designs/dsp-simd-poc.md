---
id: "DESIGN-034JcqP2KiWsdes88fgvtt"
type: "design"
status: "approved"
title: "DSP/SIMD 比較ベンチマーク PoC 詳細設計"
created: "2026-10-20"
updated: "2026-10-20"
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
  implements:
    - "../specs/dsp-simd-poc-adr.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ruviform-architecture-design-baseline.md"
    - "../../AGENTS.md"
    - "../discovery/dsp-simd-poc-issue-19.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/dsp-simd-poc-adr.md"
    - "../adr/dsp-simd-poc.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://docs.rs/fearless_simd/1.1.0/fearless_simd/"
    - "https://docs.rs/pulp/0.22.3/pulp/"
    - "https://docs.rs/multiversion/0.9.0/multiversion/"
    - "https://github.com/criterion-rs/criterion.rs/blob/HEAD/CHANGELOG.md"
  defers: []
  deferred-by: []
---

# DSP/SIMD 比較ベンチマーク PoC 詳細設計

## Context

Issue #19 の PoC を実装するための詳細設計。上流は spec [dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md)（成果物・受入基準の正本）と approach ADR [dsp-simd-poc.md](../adr/dsp-simd-poc.md)（ハーネス配置＝`ruviform-math` dev-deps・比較対象＝`fearless_simd` / `pulp` / `multiversion` 3 系統・ベンチ基盤＝criterion の決定）。これらが「どこに・何を・どう測るか」の方針を確定しており、本書はその下位層として、ファイル配置・カーネル API・等価性契約・計測手順・採否 ADR への入力形式を具体的に設計する。

設計の前提となる確定制約（設計ベースライン A.4・AGENTS.md 暫定規約・discovery 調査値）:

- stable Rust のみ・edition 2024・workspace MSRV `rust-version = "1.85"`（mise pin の実働ツールチェーンは 1.99.0）。
- `unsafe` 原則禁止（`unsafe_code = "warn"` + `-D warnings` で実質 deny。例外は SAFETY コメント付き局所許可のみ）。
- 候補クレートは `[dev-dependencies]` に封じ込め、候補クレートの型・API を `ruviform-math` 公開 API に露出しない。
- スカラー参照実装を正とし、カーネルごとに bit 完全一致または ulp 上限の等価性契約を定義する。
- dispatch level は自動検出頼みにせず明示指定で強制実行して「スカラー vs 各 level」の matrix を決定的に検証する。
- 候補クレートの MSRV 超過（`fearless_simd` 1.89 / `multiversion` 1.86 / `criterion` 1.86 / `pulp` 未宣言）は既知であり、MSRV 判断は採否 ADR へ送る。

## Scope

### In Scope

- `crates/ruviform-math` のハーネス構成（`#[doc(hidden)]` PoC モジュール・`tests/`・`benches/`・`[[bench]] harness = false`・dev-dependencies・feature 選定）。
- カーネルセット（add / scale / clamp / fade / dot / fir / biquad / sine）のシグネチャ抽象形と等価性契約。
- 「スカラー参照 × 候補 × dispatch level × 入力セット」の等価性 matrix と入力生成（正常・境界・非アライン・端数長）。
- criterion ベンチマーク methodology（サイズ・入力バリアント・計測対象・判定目安・結果記録形式）。
- 採否 ADR への入力収集方法（性能・コンパイルコスト・unsafe 範囲・MSRV 適合・保守状況）。
- `mise.toml` `bench` タスク記述・`AGENTS.md` 検証入口対応表・`README.md` の整合更新方針。
- MSRV 検証手順（`cargo +1.85 check -p ruviform-math --lib --locked`）と `mise run` 検証経路。

### Out of Scope

- 採否 ADR 本体の結論（二層構造採否・バックエンド選定・MSRV 引上げ・AVX-512・denormal 方針の最終決定）。本書は判定材料の収集形式までを設計する。
- 採用バックエンドの本番 `[dependencies]` 昇格・`ruviform-math` 公開 API 確定 — 採否 ADR 後の後続実装 Issue。
- `ruviform-dsp` 実用カーネル、`std::simd`、`wide`、他 OS/アーキの計測、denormal 対策実装（FTZ/DAZ） — spec の Out of Scope を継承。

## Components and Boundaries

### ハーネス構成（ファイル配置）

```text
crates/ruviform-math/
├── Cargo.toml                  # dev-dependencies + [[bench]] harness = false
├── src/
│   ├── lib.rs                  # `#[doc(hidden)] pub mod simd_poc;` を追加
│   └── simd_poc.rs             # スカラー参照カーネル + 共通型 + 入力生成（pure std のみ）
├── tests/
│   ├── simd_poc/               # 共有サポート mod（tests/*.rs 直下以外はテストターゲットにならない）
│   │   ├── mod.rs              # 再エクスポート + matrix ランナー
│   │   ├── inputs.rs           # 境界入力生成（denormal/±0/NaN/Inf/非アライン/端数長）
│   │   ├── equiv.rs            # 等価性判定（to_bits / ulp・比較器・失敗報告フォーマット）
│   │   ├── fearless_impl.rs    # fearless_simd カーネル群 + level 強制ラッパ
│   │   ├── pulp_impl.rs        # pulp カーネル群 + 強制ラッパ
│   │   └── multiversion_impl.rs# multiversion カーネル群（autovec 本文）
│   └── simd_equiv.rs           # 等価性テストターゲット本体（matrix を実行）
└── benches/
    └── simd_poc.rs             # criterion bench 本体。tests/simd_poc を #[path] で include
```

依存の方向と封じ込め:

- `src/simd_poc.rs` は **pure std** のみ使用（候補クレートを import しない）。スカラー参照カーネル・カーネルシグネチャの共通型（`BiquadCoeffs` 等）・deterministic 入力生成を置き、`#[doc(hidden)] pub mod simd_poc` として lib.rs にぶら下げる。benches（外部クレートとして lib の `pub` API 経由でのみ到達可能）と tests（integration test、同様に外部）の双方から `ruviform_math::simd_poc::*` で到達できる。
- 候補クレート依存のコードは **`tests/simd_poc/` 共有 mod** に集約する。benches/simd_poc.rs は `#[path = "../tests/simd_poc/mod.rs"] mod simd_poc_support;` で同一ファイルを include して再利用する（`tests/` 直下の `*.rs` だけがテストターゲットになるため `tests/simd_poc/` はターゲット化しない）。`#[path]` は両 dev ターゲット間でコードを共有するための最小機構であり、二重コンパイルのみを代償とする。
  - bench 側の `mod` 宣言には `#[allow(dead_code)]` を付ける（`#[allow(dead_code)] #[path = "../tests/simd_poc/mod.rs"] mod simd_poc_support;`）。`mod` 宣言に付けた lint 属性は配下の item 全体に波及するため、これ 1 箇所で共有 mod 内の全 helper をカバーできる。この回避策が必要な機序: `tests/simd_equiv.rs` でのみ使用されるテスト専用 helper（失敗報告フォーマッタ等）は、bench クレート側で `mod` として取り込まれると dead_code 警告となり、`mise run lint`（`clippy -- -D warnings`）で失敗する。`#[allow(dead_code)]` が第一回避策であり、それでも lint を通過できない場合の第二 fallback が後述の独立 PoC support クレート案。
- **アーキテクチャ依存コードの cfg ゲート**: x86 限定の level 強制 API（fearless_simd の `as_sse2` / `as_sse4_2` / `as_avx2` / `as_avx512`、`pulp::x86::V2` / `V3` / `V4`）を参照するコード・等価性 matrix 列・ベンチ識別子は `#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]` でゲートする。aarch64 では portable な `fearless_fallback`（`force_support_fallback`）・`fearless_neon`（aarch64 側の level 強制 API）・`pulp_scalar` / `pulp_dispatched`（`Arch` = Neon 検出経路）のみが検証・計測対象となる。CI の `macos-latest`（aarch64）レーンで `cargo check --workspace --all-targets` が dev ターゲット込みでコンパイルを通すことが、このゲートを必須にする根拠。
- 候補クレートの型が `ruviform-math` の公開 API に出ないことは、シグネチャをすべて std 型（`&[f32]` / `&mut [f32]` / `f32` / `usize`）に揃えることで構造的に保証する。`#[doc(hidden)]` とはいえ `pub` な `simd_poc` モジュール内にも候補クレートの型・トレイト・macro 由来名を置かない。

### PoC カーネル実装の置き場所（決定と代替案の比較）

**決定**: 上記の 2 層配置 — 候補非依存部分は `src/simd_poc.rs`（`#[doc(hidden)] pub`）、候補依存部分は `tests/simd_poc/` 共有 mod + `#[path]`。

理由: dev-dep は `src/` からは一切 import できない（`harness = false` の bench / integration test 等の dev ターゲットからのみ利用可能）という Cargo の構造的制約がある。したがって「候補カーネルを src に置く」には候補クレートを通常 `[dependencies]`（または optional + feature）に昇格させる必要があり、spec の「候補クレートは dev-dependencies として導入し公開 API 汚染を最小化」という要件と衝突する。一方で、スカラー参照・入力生成・契約定数は pure std で書けるため `src/` に置ける — ここに置くと (a) benches/tests から `#[path]` ハックなしで素直に到達できる、(b) `cargo check --lib` / `cargo +1.85 check --lib` の MSRV 検証対象に含まれる、(c) 「dispatch 層を ruviform-math に集約する」構造仮説の着地予定地と同じ層で参照実装を検証できる。

比較した代替案:

| 代替案 | 却下理由 |
| --- | --- |
| 全実装を `src/`（通常 deps または optional+feature） | dev-dep 封じ込めの spec 要件に違反。optional feature は `cargo bench --workspace` で有効化できず bench が壊れる |
| 全実装を `benches/` 内共通 mod | integration test は `benches/` に到達できない（`#[path]` 経由なら可能だが、結局 `#[path]` が必要になり、スカラー参照まで dev 側に閉じ込まり `--lib` MSRV 検証・構造仮説検証の対象から外れる） |
| 全実装を `tests/` 共有 mod のみ | src を完全に無変更にできる利点はあるが、スカラー参照・入力生成・契約定数まで dev 側に閉じ、`--lib` 単独検証と「本実装の着地予定地と同じ場所で参照実装を書く」効を失う。候補依存部に `#[path]` が必須であることは変わらず、封じ込め効果の差は `#[doc(hidden)]` 程度に過ぎない |
| 独立 PoC support クレート（`ruviform-math` の dev-dep として path 依存） | 候補クレートをその support クレートの通常 deps に置けるため `#[path]` 不要になるが、ワークスペース `members` に一時的メンバーを増やし AGENTS.md のクレート構成契約を更新するコストが発生する。ADR が Option B を「クレート境界のずれ・一時メンバー管理」で却下した判断と同型のコスト |

**approach ADR からの deviation 明記**: ADR の Implementation Plan は「スカラー参照カーネルと各バックエンドカーネルを `crates/ruviform-math/src/` に `pub(crate)` 以下の公開度で置く」と記すが、dev-dep は `src/` の非テストコードから import できず、`pub(crate)` な item は別クレートとしてコンパイルされる bench・integration test ターゲットから到達不能なため、その形では成立しない。本設計は `#[doc(hidden)] pub` + `tests/` 側配置への修正を deviation として採用する。ADR 本文は更新せず、判断の記録としてここに残す。

`#[doc(hidden)] pub` を選んだ副作用の明記: `publish = false` の内部クレートのため外部契約にはならないが、機械的に public item は増える。採否 ADR・後続実装 Issue で正式 API 化または除去するまでの暫定措置であり、`simd_poc` モジュール内の item を後続コードから本番用途で参照しない運用とする。

crate doc との整合（実装時の適用条項）: `crates/ruviform-math/src/lib.rs` の crate doc は "All public functions are designed to be RT-suitable: allocation-free and bounded" と宣言するが、`simd_poc` の入力生成 helper は `Vec` を allocate する実験 API でありこの宣言と矛盾する。実装時に `simd_poc` のモジュール doc へ「実験 API・本番 RT 契約の対象外・入力生成 helper は allocate する」旨を明記し、crate doc 側は変更しない（lib.rs の宣言は本番公開 API を対象とする契約であり、`#[doc(hidden)]` 実験モジュールは対象外であることをモジュール側で明示する）。

### dev-dependencies と feature 設計

`crates/ruviform-math/Cargo.toml` に追加する（`[dependencies]` は空のまま維持）:

```toml
[dev-dependencies]
# alloca の非対応ターゲット問題は 0.8.2 で修正済み（"don't build alloca on
# unsupported targets"）。default-features を切って rayon + plotters を外し、
# dev-dep のコンパイルコストを抑える（html_reports / csv_output は未使用でも
# estimates.json は常に出力される）。
criterion = { version = "0.8.2", default-features = false }
# x86_64 では SSE2 が baseline で Fallback バリアント/トークンが cfg 上存在しないため、
# force_support_fallback を有効化して Fallback::new() を構成可能にする。
fearless_simd = { version = "1.1", features = ["force_support_fallback"] }
fearless_simd_macros = "0.1"
# x86-v4 は非 default（bytemuck/avx512_simd を引く）。AVX-512 評価のため有効化。
# macro = #[pulp::with_simd] の評価対象。
pulp = { version = "0.22", features = ["x86-v4", "macro"] }
multiversion = "0.9"
# 等価性判定（UlpsEq）。代替は float_eq — 実装時に API 使い勝手で 1 つに絞る。
approx = "0.5"

[[bench]]
name = "simd_poc"
harness = false
```

feature 選定の根拠と確認事項:

- `force_support_fallback`: docs.rs 確認済みの feature 名（`force_support_fallback`、アンダースコア）。これがないと x86_64 では `fearless_simd::Fallback` 型自体が存在せず、等価性 matrix の Fallback 強制行を埋められない。本番昇格時にこの feature が不要かどうかは採否 ADR の発見事項として記録する。
- `pulp` の `x86-v4`: 非 default。`pulp::x86::V4` / `Arch::V4` を有効化するために必要。CI 的には `bytemuck` の `avx512_simd` を引くだけで追加依存はない。ただし `avx512_simd` は Rust 1.89 で安定化した AVX-512 intrinsics に依存するため、`x86-v4` 有効時の pulp ツリーの推移的 MSRV 下限は ≈1.89 となる（採否 ADR の MSRV 判断材料）。
- `criterion` の feature 切り詰め: `cargo_bench_support`（libtest 互換 API）は `harness = false` + `criterion_main!` では不要。`rayon` は分析並列化のみ・`plotters` は描画のみで、計測値の正しさに影響しない。結果の機械可読出力は `target/criterion/**/new/estimates.json` が常に書かれるため feature 不要。`real_blackbox` feature の有無に関わらず本設計では `std::hint::black_box`（stable 1.66+）を使う。

### `profile.bench` の再現性設定（要否の判断）

**判断: 追加しない**（ルート `Cargo.toml` に `[profile.bench]` を設けない）。

- `profile.bench` の既定値は `release` 継承（opt-level=3・`debug=false`・LTO なし・codegen-units=16）。本 PoC の目的は「同一プロファイル・同一バイナリ内での候補間相対比較」であり、絶対性能の再現性ではない。criterion の統計処理（複数反復・外れ値処理）が run 内ノイズを吸収する。
- `lto`/`codegen-units=1` を設定すると、本番の dev/release プロファイルと乖離した最適化結果になり、dispatch 層オーバーヘッド評価として現実性を損なう。`debug = true` は成果物サイズのみ悪化。
- 計測条件（OS・CPU・rustc 版・profile）は experiment log のメタデータに記録するため、プロファイル差異による再解釈はログ側で担保する。ただし `panic`・`overflow-checks` 等の既定値からの逸脱が必要になった場合は計測前に再評価する（現時点では不要）。

## Data and Control Flow

### カーネル API 抽象形

全候補が揃える共通シグネチャ（`src/simd_poc.rs` で型のみ定義。戻り値・可変借用の形を統一し、候補差を中身に閉じ込める）:

```rust
// スライス長は a.len() == b.len() == dst.len() を契約とし、
// 不一致は debug_assert（PoC 内のためパニック契約は doctest/ユニットテストで担保）。
pub fn add(dst: &mut [f32], a: &[f32], b: &[f32]);                       // dst[i] = a[i] + b[i]
pub fn scale(dst: &mut [f32], src: &[f32], gain: f32);                   // dst[i] = src[i] * gain
pub fn clamp_unit(dst: &mut [f32], src: &[f32]);                         // dst[i] = clamp(src[i], -1, 1)
pub fn fade(dst: &mut [f32], src: &[f32], g0: f32, step: f32);           // dst[i] = src[i] * (g0 + i*step)
pub fn dot(a: &[f32], b: &[f32]) -> f32;                                 // sum(a[i]*b[i])
pub fn fir(dst: &mut [f32], src: &[f32], taps: &[f32]);                  // 64 tap FIR、taps.len() == FIR_TAPS
pub fn biquad(dst: &mut [f32], src: &[f32], c: &BiquadCoeffs,
              state: &mut [f32], channels: usize);                       // planar N ch IIR。state.len() == 2*channels
pub fn sine(dst: &mut [f32], c: &SineCoeffs, state: &mut SineState);     // 2 次再帰発振器

pub const FIR_TAPS: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct BiquadCoeffs { pub b0: f32, pub b1: f32, pub b2: f32,
                          pub a1: f32, pub a2: f32 }

#[derive(Clone, Copy, Debug)]
pub struct SineCoeffs { pub c1: f32, pub c2: f32 }   // c1 = 2*cos(ω), c2 = -1

#[derive(Clone, Copy, Debug)]
pub struct SineState { pub y1: f32, pub y2: f32 }
```

- `biquad` は `src`/`dst` を planar `[ch0: frames][ch1: frames]…` レイアウト、`state` を SoA `[z1×ch][z2×ch]`（len = 2×channels）で受ける。チャンネル間は独立・チャンネル内は逐次 — レーン = チャンネルの並列化形。
- `sine` は `y[n] = c1·y[n-1] + c2·y[n-2]` の純粋再帰（`sin` の初期条件のみ非カーネル部で設定）。ループ依存があるため SIMD 化が構造的に困難なカーネルとする。
- `biquad`/`sine` の状態は呼び出し側が保持し渡す（alloc なし・RT 適合形状の予行演習）。bench では `iter_batched`/`iter_with_setup` で反復ごとに状態を初期化する（methodology 節「状態持ちカーネルの反復構成」参照）。

### カーネルセットと等価性契約

| カーネル | 内容 | 等価性契約 | 選定理由 |
| --- | --- | --- | --- |
| `add` | `dst = a + b` | **bit 完全一致**（非 NaN 入力。NaN 入力は IEEE 加算で NaN 伝播→ `is_nan` 一致） | elementwise 基礎・メモリ帯域律速の代表 |
| `scale` | `dst = src * g` | **bit 完全一致**（同上） | 単一入力 elementwise・autovec との最小差 |
| `clamp_unit` | `dst = clamp(x, -1, 1)` | **非 NaN 入力で bit 完全一致。NaN 入力 → NaN 出力**（payload 不問、`is_nan` 一致） | min/max の NaN 意味論差の実例。fearless_simd の `min`/`max` は NaN・±0 が implementation-defined のため `min_precise_*`/`max_precise_*` 系を使用し NaN 伝播を揃える。スカラー参照は `f32::clamp`（self が NaN → NaN 返却） |
| `fade` | `dst[i] = src[i] * (g0 + i*step)` | **bit 完全一致**（elementwise 構成に両側揃え、fused multiply-add を使わないことを契約として明記。非 NaN 入力で bit 完全一致、NaN 入力は `is_nan` 一致） | iota ベース index 演算を含む ramp。逐次累積形は drift が大きく本番でも不採なため elementwise 構成で統一 |
| `dot` | `Σ a*b` | **ulp ≤ 8**（初期値。実測で見直し可）。入力は有限値を契約 — reduction 順序差で Inf への丸め・NaN 化が変わり得るため、edge 入力の NaN/Inf セルは比較対象から除外する（除外条件はテストコードにコメントで明記） | reduction 順序差・FMA 有無・水平加算の代表 |
| `fir` | 64 tap FIR（直接形） | **ulp ≤ 8**（同上。係数・入力は有限値を契約 — 窓内に Inf/NaN を含む出力は順序非依存で NaN/Inf 伝播するため `is_nan` 一致） | 入れ子ループ＋累積の代表。denormal 入力の性能崖観測ポイント |
| `biquad` | N ch planar IIR（Direct Form II 相当、状態持続） | **ulp ≤ 8**（初期値。FMA 使用差を許容）。再帰状態で丸め差が増幅・累積し得るため、固定上限で正当な超過が出た場合は代替契約形へ切替: 先頭 N フレームのみ ulp 比較＋以降は相対誤差/出力エネルギー差の上限、またはフレーム長依存の上限（切替は実測で見直すプロトコルの範囲内）。NaN/Inf 入力時は再帰状態が NaN 汚染され以降の全出力へ伝播するため `is_nan` 一致とし、edge 適用セルではケースごとに状態をリセットする | チャンネル並列のみ可能・フレーム方向は逐次 — 状態逐次カーネルの並列化限界の実例。denormal 主戦場 |
| `sine` | 2 次再帰発振器 | **bit 完全一致**（全候補が同一逐次コードを dispatch 経由で呼ぶだけの構成のため一致するはず。差が出たら dispatch 層の codegen 差として記録） | 「SIMD が効かない領域」の対照例。性能差 ≈ dispatch オーバーヘッド差のみ |

ulp 上限の初期値 8 の根拠: f32 の dot/FIR でレーン分割 reduction（8〜16 並列 + 水平加算）＋ FMA 縮約で生じる丸め差は経験的に数 ulp 台。上限は `approx::UlpsEq` ベースの実測 max ulp を全 matrix で集計し、余裕を持って採否 ADR で確定値に引き上げ/維持する（観測最大値 > 8 の場合は実装バグか契約不適合として調査してから見直す — 数値を合わせ込むための事後緩和はしない）。再帰系カーネル（`biquad`/`sine`）は丸め差が状態経由で後続フレームへ持ち越され、固定 ulp 上限への適合がフレーム位置依存になり得るため、`biquad` には上表の代替契約形を用意しておく。

### 等価性 matrix

matrix の軸: **カーネル × 実装（dispatch level 強制）× 入力セット × 長さ**。

| 実装列 | 強制方法 | 備考 |
| --- | --- | --- |
| `scalar`（参照） | `src::simd_poc` の関数を直接呼ぶ | 正。全 matrix セルの比較先 |
| `fearless_fallback` | `fearless_simd::Fallback::new()` トークンを `#[simd]` 関数へ直接渡す | `force_support_fallback` feature 前提 |
| `fearless_sse2` / `fearless_sse4_2` / `fearless_avx2` / `fearless_avx512` | `Level::new().as_sse2()` 等（`Option<Token>`）→ `Some` のときのみ実行、`None` は skip 記録 | 検出 level 以下のトークンが得られる。`assume_supported`（unsafe）は使わない。**x86 限定 API のため cfg ゲート対象**（後述） |
| `fearless_dispatched` | `dispatch!(Level::new(), simd => kernel(simd, …))` | 自動検出経路（= 本番想定経路） |
| `pulp_scalar` / `pulp_v2` / `pulp_v3` / `pulp_v4` | `pulp::Scalar::new()`・`pulp::x86::V2/V3/V4::try_new()`（`Option`）→ `Some` のみ実行 | `V4` は `x86-v4` feature。`new_unchecked`（unsafe）は使わない。**`V2`/`V3`/`V4` は x86 限定 API のため cfg ゲート対象**（後述） |
| `pulp_dispatched` | `pulp::Arch::new().dispatch(op)` | 自動検出経路 |
| `multiversion` | `#[multiversion]` 付き関数を普通に呼ぶ | **個別 clone を強制実行する API が存在しない**（clone は private シンボル）。検出環境の最良 clone のみ検証される制限として採否 ADR に記録 |

| 入力セット | 内容 |
| --- | --- |
| `normal` | deterministic PRNG（xorshift64*、固定 seed）の一様乱数 `[-1.0, 1.0)`。`rand` 依存を増やさず再現性を確保 |
| `edge` | 境界値混合: denormal（`f32::from_bits(0x0040_0000)`（≈5.9e-39）や `f32::MIN_POSITIVE * 0.5` 等の非正規化数を含む列 — `MIN_POSITIVE` に小さな係数を掛ける素朴な生成は +0.0 に丸まるため使わない）、`+0.0`/`-0.0`、`NaN`（quiet）、`±Inf`。カーネル契約上 NaN/Inf が意味を持つもののみ適用 |
| `denormal` | `normal` の一部要素を denormal 値に置換したもの（`biquad`/`fir` では出力側 denormal 収束も観測対象） |

| 長さ | 内容 |
| --- | --- |
| 端数系 | `{7, 15, 16, 17, 31, 33}`（最大レーン幅 16=f32x16 想定で ±1・奇数・境界跨ぎ）。biquad は `(channels, frames) ∈ {8}×{…}` で frames 側を変える |
| 実用系 | `{100, 255, 1023, 1024, 4096}`（ベンチと整合する代表サイズ） |
| アライメント | 先頭オフセット `{0, 1, 3}` 要素（`Vec<f32>` を確保して `&v[off..]` で非アライン先頭を構成） |

- テスト本体 `tests/simd_equiv.rs` は各カーネルに対し `(impl, level) × (input, len, offset)` をループし、全セル結果を 1 テスト関数で集約検証する。不一致は `(kernel, impl, level, input, len, offset, index, expected_bits, actual_bits, ulps)` を panic メッセージに含める。
- 強制実行できない環境（例: AVX-512 非搭載 CPU で `as_avx512() → None`）は panic ではなく skip として出力に記録する。matrix は「実行環境で構成可能な level」のみを検証するため CI 環境差に対して決定的 — これは ADR の「dispatch level 明示指定による決定性」要件の解釈であり、level ごとの検証可否自体は結果として記録する。
- x86 限定の実装列（`fearless_sse2`/`sse4_2`/`avx2`/`avx512`・`pulp_v2`/`v3`/`v4`）は Components 節の cfg ゲート方針どおり `#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]` で囲み、aarch64 では matrix に現れない。aarch64 で検証されるのは portable 列のみ（`scalar`・`fearless_fallback`・`fearless_neon`・`fearless_dispatched`・`pulp_scalar`・`pulp_dispatched`・`multiversion`）— CI の `macos-latest` レーンでも `--all-targets` のコンパイルと実行が通る構成とする。
- `edge` 入力に対しカーネル契約が NaN/Inf を扱わない場合（`dot` の有限入力契約等）はセルを除外し、除外条件をテストコードにコメントとして残す。

### ベンチマーク methodology

- **サイズ**: `64 / 256 / 1024 / 4096`（実用帯: RT バッファ 64–256 相当〜オフライン 4096）＋ 端数 `100 / 1000 / 1023`（レーン境界 ±1・非倍数）。biquad は `channels = 8` 固定で frames を同系列、`sine`/`fir`/`dot` も同じ長さ系列。
- **入力**: `normal`（deterministic seed の pseudo-random）と `denormal` 混入バリアントの 2 系統（denormal は `fir`/`biquad`/`sine`/`dot` のような累積系カーネルで性能崖観測用。`add`/`scale`/`clamp`/`fade` は `normal` のみでよい）。`edge` 入力はベンチ対象外（等価性のみ）。
- **計測対象（ベンチ識別子）**: `scalar` / `fearless`（dispatched）/ `fearless_fallback` / `fearless_sse4_2` / `fearless_avx2`（構成可能な level のみ）/ `pulp`（dispatched）/ `pulp_scalar` / `pulp_v3` / `pulp_v4`（同上）/ `multiversion`。`fearless_sse2` / `pulp_v2` は計測対象に含めない — x86_64 では SSE2 が baseline のため、同層は `scalar`（autovec 済み baseline）/ `fearless_fallback` / `pulp_scalar` の実力と近似し冗長となる。v2 tier の代表は `fearless_sse4_2` とする（等価性 matrix では引き続き検証対象）。x86 限定識別子は cfg ゲート方針どおり aarch64 では生成しない。
- **criterion 設定**: カーネルごとに `BenchmarkGroup`、`group.throughput(Throughput::Elements(len as u64))`。ベンチ ID は `BenchmarkId::new(<impl>, <input>/<len>)` — criterion の出力階層 `target/criterion/<group>/<function_id>/<parameter>/new/estimates.json` に `<group>`=カーネル名・`<function_id>`=impl 識別子・`<parameter>`=`{input}/{len}` のスラッシュ区切り encode（入れ子パスになる）を対応させる。例: group=`dot` に `BenchmarkId::new("fearless_avx2", "normal/1024")` → `target/criterion/dot/fearless_avx2/normal/1024/new/estimates.json`。
- **black_box と反復構成**: `b.iter(|| kernel(black_box(&mut *dst), black_box(a), …))` — `black_box` は `std::hint::black_box`。`black_box(dst)` と書くと `&mut [f32]` が move して反復呼び出しで E0507 になるため、再借用した `&mut *dst`（または `dst.as_mut()`）を渡す。`dst` は setup で 1 回確保して反復使用（elementwise カーネルは冪等書き込み）。
- **状態持ちカーネルの反復構成**: `biquad`/`sine` は `iter_batched` / `iter_with_setup` で反復ごとに状態を初期化する。状態を反復横断で持続させると、normal バリアントでもフィルタ出力が denormal 域・0 へ減衰して normal 側のベースライン計測を汚染するため、normal 側は反復ごとに非 denormal の決定的初期状態へリセットする。denormal-cliff 観測側は反復ごとに denormal 状態を維持・再注入する setup とし、崖の有無を安定的に計測する。
- **実行時間予算**: 全 matrix は 8 カーネル × 約 10 impl × 7 サイズ × 入力バリアントで ≈400–560 ベンチケースとなり、criterion 既定設定（warmup・measurement・sample 100）では合計 1 時間超が見込まれる。対策: (a) `warm_up_time` / `measurement_time` / `sample_size` を削減した PoC 用 `BenchmarkGroup` 設定、(b) カーネル・impl・サイズを絞る quick-run filter 規約（`cargo bench -p ruviform-math -- <filter>`、例: `dot`・`1024`）、(c) CI の bench ジョブ（`.github/workflows/ci.yml`・ubuntu-latest・`workflow_dispatch` 専用）はスモーク（`-- --test` または filter 短縮）に留め、全量計測はローカル手動実行に分離する。
- **判定目安の数値化**（spec「目安を後続 PoC 計画で数値化」への対応）:
  - **採用側目安**: elementwise/reduction 系の代表カーネル（`add`/`scale`/`clamp`/`fade`/`dot`/`fir`）で、主要帯 256・1024・4096 におけるスカラー比スループット ≥ **1.5x**（中央値ベース、全対象カーネルの過半数で達成）。
  - **補助条件**: 小サイズ 64・256 で dispatched 経路がスカラー比 < 0.9x の回帰にならないこと（dispatch オーバーヘッド支配が RT バッファ帯で致命的にならないこと）。
  - **dispatch オーバーヘッド**: `dispatched` vs 最良強制 level の中央値差が 1024 で ≤ 5% 程度を健全値の目安とする（実測分布を記録して採否 ADR で判断）。
  - **反証材料**: `sine` は全候補 ≈ 1.0x を期待（dispatch 差のみ）。`biquad` はレーン = チャンネル並列のため 8ch で理論 ~8x だが状態逐次・帯域律速で実効は小さいはず — 実測を記録。
  - 判定数値を下回る場合は「不採用」または「条件付き採用（対象カーネル限定・dispatch 方式限定）」が正当な結論であり、先に決めた結論へ数値を寄せない。
- **実行環境の記録**: OS・CPU 型名・コア数・rustc 版・criterion 版・`Level::new()` 検出結果・`pulp::Arch::new()` 検出結果を run 冒頭で標準出力に出し、experiment log に転記する。

### 計測結果の記録形式（experiment log）

- criterion は各ベンチケースの統計値を `target/criterion/<group>/<function_id>/<parameter>/new/estimates.json` に必ず出力する（`<function_id>`=impl 識別子、`<parameter>`=`{input}/{len}` — methodology 節の BenchmarkId 規則と一致）。これを集約して `docs/impl/exp/dsp-simd-poc-<yyyymmdd>-<seq>.jsonl` としてコミットする。1 行 = 1 計測点:

```json
{"ts":"…","os":"windows","arch":"x86_64","cpu":"…","rustc":"1.99.0","kernel":"dot","impl":"fearless_avx2","input":"normal","len":1024,"median_ns":…,"elems_per_s":…,"source":"target/criterion/dot/fearless_avx2/normal/1024/new/estimates.json"}
```

- 集約手順: `estimates.json` 群から `median`/`mean` を抽出する軽量スクリプト（PoC では `node` ワンショット or 手集計でよい。自動化は本実装 Issue の課題）。生成した jsonl と併せて、環境メタデータ・判定表（カーネル × impl のスカラー比）・観察メモを `docs/impl/exp/` の experiment log 文書に記録し、`docs/impl/ir/` の impl record から参照する。

## 候補別実装パターン

### `fearless_simd`

```rust
// tests/simd_poc/fearless_impl.rs
use fearless_simd::{dispatch, Fallback, Level, Simd};
use fearless_simd_macros::simd;

#[simd]
fn add_impl<S: Simd>(simd: S, dst: &mut [f32], a: &[f32], b: &[f32]) {
    let ((d, a, b)) = …; // chunks_exact_mut(S::f32s::LEN) + remainder スカラー
}

pub fn add(level: Level, dst: &mut [f32], a: &[f32], b: &[f32]) {
    dispatch!(level, simd => add_impl(simd, dst, a, b));
}

// 強制 level: トークンを直接渡す（#[simd] 関数の第 1 引数はトークンキャリア）
//   add_impl(Fallback::new(), dst, a, b);
//   if let Some(t) = Level::new().as_avx2() { add_impl(t, dst, a, b); }
//   ※ as_sse2/as_sse4_2/as_avx2/as_avx512 を参照する強制ラッパは
//     #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] でゲートする
```

- 抽象度は portable `Simd` trait + `#[simd]` macro（自動 vectorize 補助）を使用し、`kernel!` intrinsics 層は使わない（PoC で評価したいのは safe な portable 層）。
- `min`/`max`/`mul_add` は NaN・±0・非 FMA level で implementation-defined → `clamp` は `*_precise` 系、`dot`/`fir`/`biquad` の `mul_add` 使用はカーネル内で明示し等価性契約（ulp ≤ 8）で許容する。`fade` は FMA 非使用構成。

### `pulp`

```rust
// tests/simd_poc/pulp_impl.rs
use pulp::{Arch, Simd, WithSimd};

struct Add<'a> { dst: &'a mut [f32], a: &'a [f32], b: &'a [f32] }
impl WithSimd for Add<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let (head, tail) = S::as_mut_simd_f32s(self.dst);
        // head に SIMD、tail にスカラー
    }
}

pub fn add(dst: &mut [f32], a: &[f32], b: &[f32]) {
    Arch::new().dispatch(Add { dst, a, b });
}
// 強制 level: Add{…}.with_simd(pulp::Scalar::new()) / v3.map(|t| Add{…}.with_simd(t))
//   ※ pulp::x86::V2/V3/V4 を参照する強制ラッパは
//     #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] でゲートする
```

- `#[pulp::with_simd]`（`macro` feature）と手書き `WithSimd` impl は等価 — 実装は `WithSimd` trait を直接 impl する形を基本とし、macro 版の記述性は開発者体験の観察点として採否 ADR に記録する。
- `pulp` の `min`/`max` は x86 の `_mm_min_ps` 系意味論（NaN 時に第 2 オペランド優先等）— `clamp` での NaN 伝播を fearless_simd precise 版と揃えるため、NaN 伝播を明示した実装（比較 + select 等）にするか、pulp 側は NaN 入力セルを「契約差あり」として記録するかを実装時に決める（方向: 両者とも NaN 伝播する形に揃える）。

### `multiversion`

```rust
// tests/simd_poc/multiversion_impl.rs
#[multiversion::multiversion(targets(
    "x86_64+avx2+fma",
    "x86_64+sse4.2",
    "aarch64+neon"
))]
pub fn add(dst: &mut [f32], a: &[f32], b: &[f32]) {
    // autovec スカラー本文 — src::simd_poc::add と同一ループ
}
```

- targets は `x86_64+avx2+fma` / `x86_64+sse4.2` / `aarch64+neon`（`targets = "simd"` ショートハンドは使わず、fearless_simd の level 集合と対応づく明示リストで比較可能性を確保）。
- dispatch は既定（`direct` or `indirect` 自動選択・初回検出を atomic キャッシュ）で、個別 clone の強制実行は API 上不可 → 等価性 matrix は「検出環境での最良 clone のみ」を検証し、この非対称性を発見事項として採否 ADR に記録する（等価性検証の網羅性が構造的に劣る点）。
- 小カーネルでの indirect call オーバーヘッドは discovery の Open Question — `len = 64` での `multiversion` vs `scalar` 直接呼び差がその実測となる。

### `scalar`（参照実装）

- `src::simd_poc` の素朴ループ。**autovec を意図的に抑制しない** — SSE2 baseline でコンパイルされる「現在の配布バイナリ実力」をベースラインとするため（真の逐次スカラーとの比較は本 PoC の問いではない）。
- `#[inline]` 指定は付けない。bench 内で関数ポインタ経由でも直接呼びでもよい（dispatch 層なしの呼び出しコストを測るのが目的）。

## 採否 ADR への入力形式

設計ベースラインの採否基準（性能向上・コンパイルコスト・unsafe カプセル化範囲・MSRV 適合・保守状況）ごとの収集方法:

| 採否基準 | 収集方法 | 記録先 |
| --- | --- | --- |
| 性能向上 | `mise run bench` の matrix 結果（`estimates.json` → jsonl）。各カーネル・サイズのスカラー比中央値を表にする | `docs/impl/exp/` jsonl + experiment log 文書の判定表 |
| コンパイルコスト | `cargo check --workspace --all-targets` の所要時間（dev-dep 導入前後で比較）＋ `cargo check --timings --workspace --all-targets` のユニット別レポートから候補クレート分を抽出（`--all-targets` を付けて dev-dep・bench target も計測対象に含める。`cargo build --timings` では dev 依存が対象外になるため使わない） | impl record（再現コマンド・環境つき） |
| unsafe カプセル化範囲 | PoC コード内の `unsafe` 使用数（目標 0）= `grep -rn "unsafe" crates/ruviform-math`。候補 API が unsafe を要求する経路の有無（`assume_supported` / `new_unchecked` を使わずに済んだか） | impl record・採否 ADR |
| MSRV 適合 | `cargo metadata` / 各候補 Cargo.toml の `rust-version` フィールド値を列挙（fearless_simd 1.89・multiversion 1.86・criterion 1.86・pulp 未宣言→実測。ただし pulp の `x86-v4` feature 有効時は `bytemuck` の `avx512_simd` を経由して Rust 1.89 で安定化した AVX-512 intrinsics に依存するため、推移的下限は ≈1.89）＋ `cargo +1.85 check -p ruviform-math --lib --locked` の可否 | 採否 ADR の MSRV 判断節 |
| 保守状況 | discovery 記録値の更新確認（最新版・最終リリース日・Issue/コミット活動） | 採否 ADR |
| 等価性検証の網羅性 | level 強制可否（fearless/pulp はトークン経由で可・multiversion は不可）と、検出不能 level の skip 記録 | 採否 ADR 発見事項 |
| denormal 影響 | `denormal` 入力バリアントのベンチ差（fir/biquad での性能崖の有無・倍率） | experiment log・採否 ADR の denormal 方針節 |

## Non-Functional Constraints

- **MSRV 検証手順**: dev-dep の MSRV 超過は `cargo +1.85` で `--all-targets` を解決・ビルドできない既知の制限とする。PoC 期間中の MSRV 検証は **`cargo +1.85 check -p ruviform-math --lib --locked`**（本番依存のみ、既存 `Cargo.lock` 前提）とする。`cargo +1.85` は rustup の toolchain 指定構文であり、mise.toml が pin するのは 1.99.0 のみのため、検証前に `rustup toolchain install 1.85`（mise の rust backend は rustup 経由なので管理ツールと競合しない）等で 1.85 ツールチェーンを別途調達する手順を検証手順に含める。また `--locked` を満たすには dev-dep 追加後の `Cargo.lock` が最新である必要があるため、先に pin 済みの 1.99.0 上で `cargo generate-lockfile`（または通常の `mise run check`）で lockfile を再生成してから `+1.85` 側を実行する。`+1.85` での fresh resolve は dev-dep 範囲に 1.85 適合版が存在しないため失敗し得る — これは lockfile を持つワークスペース運用では非問題であり、制限として採否 ADR に記録する。Cargo の `resolver.incompatible-rust-versions` は `rust-version` 宣言パッケージに対し既定で `fallback`（適合版優先・適合版がなければ非適合版も選択可）のため、mise pin の 1.99.0 上での通常解決はブロックされない。
- **unsafe**: PoC コード（`src/simd_poc.rs`・`tests/`・`benches/`）は `unsafe` 0 を目標。候補 API の強制 level 経路は `Option` 返却の safe API（`as_*` / `try_new`）のみ使い、`assume_supported` / `new_unchecked` / `std::arch` は使わない。候補クレート内部の unsafe は対象外だが、公開 API が利用側に unsafe を要求する候補は採否評価の減点要素。
- **公開 enum 規約**: `simd_poc` モジュールには pub enum を置かない（`KernelId` 等は `&'static str` または構造体定数で代替）。将来 pub enum が必要になった場合は `#[non_exhaustive]` 必須。
- **doctest**: `#[doc(hidden)]` モジュールの公開関数に doctest は書かない（公開 API 未確定の実験コードのため）。`cargo test` の doctest 対象にしない。
- **RT 契約と `rt_` 命名の適用判断**: `ruviform-math` の公開関数は責務表上「全公開関数を alloc なし・有界の RT 適合設計」とされるが、PoC カーネルは `#[doc(hidden)]` の実験 API であり本番 RT 契約の対象外とする（この対象外宣言は `simd_poc` のモジュール doc にも明記する — Components 節「crate doc との整合」の適用条項）。したがって `mise run test-rt` が拾う `rt_` プレフィックス命名の RT 契約テストは **本 PoC では追加しない**（`test-rt` は 0 件一致で skip 相当のまま）。alloc 非検査の機械的検証は `assert_no_alloc` 未導入のため実施できず、RT 検証基盤の導入自体は設計ベースライン「RT 違反検出の分担」が指す後続 Issue の範囲 — 本 PoC では「カーネルが事前確保済みスライスのみを操作し alloc を行わないシグネチャ形状」を設計上の予行演習として採用するに留め、その旨を impl record に記録する。

## Risks and Trade-offs

| リスク / トレードオフ | 緩和策 |
| --- | --- |
| dev-dep の MSRV 超過で `cargo +1.85` 系の検証が dev ターゲット込みでは壊れる | `--lib --locked` 検証手順に限定し、制限を採否 ADR の MSRV 判断へ送る。pin or MSRV 引上げは本設計では先取りしない |
| Windows x64 計測ノイズ（turbo/省電力・バックグラウンド負荷・タイマ分解能） | criterion の warmup・複数サイズ・複数 impl の同一 run 内相対比較で緩和。同一マシンで 2–3 回 run して分散を experiment log に記録。省電力モード・負荷状況を環境メタデータに記録 |
| criterion 0.8 の `alloca` 非対応ターゲット問題 | 0.8.2 で修正済み（"don't build alloca on unsupported targets"）のため `0.8.2` 以上を要求。Windows x64 / Linux / macOS は alloca 対応ターゲット |
| `fearless_simd` の新しさ（1.x リリース間もない）・`min`/`max`/`mul_add` の implementation-defined 意味 | `*_precise` 系・NaN 契約・カーネル内コメントで明示。等価性 matrix が実装差の検出網として機能する |
| `pulp` の正式 MSRV 未宣言 | 実測（`cargo +1.85`/`+1.86` での check）で記録し採否 ADR の MSRV 判断材料にする |
| `multiversion` が個別 clone 強制を API 上提供しない | matrix 設計で「dispatched のみ」列として扱い、等価性検証網羅性の構造的差として採否 ADR に記録 |
| AVX-512 環境が手元/CI にない | `Option` ベース強制で skip 記録。採否 ADR では「AVX-512 は将来扱い」とする判断材料（実測不在）を明示 |
| `#[path]` 共有 mod の非標準性（二重コンパイル・lint 通過性） | bench 側 `mod` 宣言への `#[allow(dead_code)]` を第一回避策とし（mod 属性は配下に波及し、テスト専用 helper が bench 側で dead_code 警告→`-D warnings` 失敗するのを防ぐ）、`#[path]` 共有の最小形に留めて `mise run lint` で実際にゲート通過を確認。通らなければ support クレート案へ切替（Deferred Concerns 記録） |
| 全量ベンチの実行時間（≈400–560 ケース・既定設定で 1h 超） | methodology 節の実行時間予算どおり criterion 設定削減・quick-run filter・CI スモーク/手動全量の分離で対処 |
| denormal 収束で biquad/fir のベンチが崖を踏む | denormal バリアントを別途計測して崖を「観測対象」にする。FTZ/DAZ 実装はしない（スコープ外・採否 ADR の denormal 方針節へ） |
| `#[doc(hidden)] pub` による実質公開面の増加 | `publish = false` の内部クレートで外部契約化しない。後続 Issue で正式 API 化または除去するまでの暫定措置として明記 |

## Deferred Design Concerns

| 据え置く項目 | 理由 | 再着手トリガ | 無視した場合のリスク |
| --- | --- | --- | --- |
| 採否 ADR 本体（二層構造採否・バックエンド選定・等価基準確定値・AVX-512・denormal・MSRV 判断） | ベンチ・matrix 結果が入力 | PoC 計測・等価性検証の完了 | 結論を先取りした設計になる |
| 候補クレートの本番 `[dependencies]` 昇格・dispatch 層の公開 API 設計 | 採否 ADR の結論次第 | 採否 ADR 承認 | 実験依存が公開 API に定着する |
| `force_support_fallback` の本番要否 | dev-dep でのみ必要かは採否次第 | 本番昇格 Issue | 不要 feature の持ち越し |
| denormal 対策実装（FTZ/DAZ 設定・カーネル内 flush の API 契約） | spec で実装外・方針のみ | エンジン層 Issue | denormal 性能崖の未対処持ち越し |
| 他 OS/アーキ（aarch64 NEON・AVX-512 実機）での計測 | Windows x64 主計測の spec 範囲 | CI `workflow_dispatch` 手動実行または後続 Issue | 単一環境結果の過剰一般化 |
| `std::simd` 移行判断 | nightly 限定・stable-only 制約 | `std::simd` 安定化 | 将来の最適解への移行パス喪失 |

## Verification Notes

- [ ] `crates/ruviform-math/Cargo.toml` に候補 3 系統・criterion・approx が `[dev-dependencies]` に、`[[bench]] harness = false` が宣言され、`[dependencies]` は空のまま
- [ ] `mise run test`（`cargo test --workspace`）で `tests/simd_equiv.rs` の等価性 matrix がグリーン（denormal/±0/NaN/Inf/非アライン/端数長を含む）
- [ ] `mise run bench`（`cargo bench --workspace`）で criterion bench が実行される。フォールバック: `cargo bench -p ruviform-math`、スモーク: `cargo bench -p ruviform-math -- --test`
- [ ] `src/simd_poc.rs` を含む lib が `cargo +1.85 check -p ruviform-math --lib --locked` でグリーン（dev-dep の MSRV 超過に影響されない）。前提: `rustup toolchain install 1.85` で 1.85 ツールチェーンを調達済み（mise pin は 1.99.0 のみ）・`Cargo.lock` を 1.99.0 側で再生成済み
- [ ] `mise run lint` がグリーン — PoC コードの `unsafe` 0・`#[path]` include・`#[doc(hidden)]` mod が `-D warnings` を通ること
- [ ] 候補クレートの型が `ruviform-math` 公開 API（`#[doc(hidden)]` 含む）に露出していないことのレビュー
- [ ] `mise.toml` の `bench` タスク記述と `AGENTS.md` の対応表・`README.md` が実ベンチ実行に合わせて更新されている
- [ ] `prek run --all-files` がグリーン（新規ファイルの trailing whitespace・EOF・YAML/TOML チェック）
- [ ] `docs/impl/exp/` に計測条件つきの結果（jsonl + 判定表）が記録され、`docs/impl/ir/` の impl record から参照される
