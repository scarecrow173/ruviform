---
id: "DISC-034Jc7DRXwhHU9BTMwYJe2"
type: "discovery"
status: "draft"
title: "DSP/SIMD バックエンド比較 PoC の探索（Issue #19）"
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
    - "https://github.com/scarecrow173/ruviform/issues/19"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# DSP/SIMD バックエンド比較 PoC の探索（Issue #19）

## Exploration Goal

本書は GitHub Issue [#19 [poc] DSP/SIMD 比較ベンチマークと採否 ADR](https://github.com/scarecrow173/ruviform/issues/19) の briefing／探索フェーズの中間成果を記録する canonical ドキュメントである。

本書が回答する問い:

- `fearless_simd` / `pulp` / `multiversion` 等の SIMD ライブラリ・dispatch 方式のどれを比較対象とし、どの基準で採否を判断するか
- スカラー参照実装との等価性基準（ビット完全一致または ULP 上限）をどう定義・検証するか
- 設計ベースライン [ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md)「DSP／SIMD 設計（A.4）」節および分類表の「要PoC」項目（SIMD 二層構造の採否・バックエンド選定・採否基準）を本 PoC でどう解消するか

> **位置づけ**: SIMD 二層構造（スカラー参照実装 + ランタイム分岐 + フォールバック）は現時点では有力仮説であり、本 PoC のベンチマーク比較評価で確定する。採否の最終決定は後続の採否 ADR に委ね、本書は探索の推論経路と一次情報の記録に留める（「調査で有力候補が見つかった」ことと「採用決定」は区別する）。

## Confirmed Requirements and Constraints

以下は設計ベースライン（`docs/designs/ruviform-architecture-design-baseline.md`）および [AGENTS.md](../../AGENTS.md) 暫定開発規約で確定済みの制約であり、PoC の設計・採否判断はこれらを前提とする。

- **stable Rust のみ**: `std::simd`（portable SIMD）は nightly 限定のため使用不可（tracking issue rust-lang/rust#86656 は open のまま）。
- **MSRV**: ワークスペース確定値 `rust-version = "1.85"`（mise pin のツールチェーンは rust 1.99.0。公開 API・依存解決は 1.85 を下限とする）。
- **unsafe 原則禁止**: `workspace.lints` の `unsafe_code = "warn"` が `lint` / `lint-ci` の `-D warnings` ゲートで実質 deny として機能。例外は SAFETY コメント付き局所許可（`#[expect(unsafe_code)]`）のみ。
- **公開 API 非露出**: SIMD バックエンドクレートの型・API を Ruviform 公開 API に露出しない（ruviform-math 責務表。dispatch 層を `ruviform-math` に集約する構造仮説と整合）。
- **スカラー参照必須**: スカラー参照実装を正とし、カーネルごとに等価性契約（ビット完全一致 or ulp 上限）を定義してスカラーを参照として検証する。
- **ランタイム検出方針**: aarch64 NEON は baseline（ランタイム検出不要）。x86_64 は SSE2 baseline → AVX2/FMA（将来 AVX-512）を起動時検出する。
- **採否基準**: スカラー比の性能向上・コンパイルコスト・unsafe カプセル化範囲・MSRV 適合・保守状況。
- **denormal**: エンジン側 FTZ/DAZ 設定を基本案とし、カーネル内 flush は API 契約で定義する方針（具体方針は要 PoC 項目）。

## Key Issues and Assumptions

本 PoC で確定すべき論点（未解決のままでは採否 ADR を作成できないもの）:

- **SIMD 二層構造の採否**: スカラー参照 + ランタイム分岐 + フォールバックという構造仮説そのものが要 PoC。採否はベンチマーク比較評価で確定する。
- **dispatch 層の所在**: dispatch 層を `ruviform-math` に集約する構造仮説の検証（責務表・公開 API 非露出制約との整合）。
- **バックエンド選定**: `fearless_simd` / `pulp` / `multiversion` の採否。`wide` はランタイム dispatch 不可が公式明記されたため事実上脱落（後述）。
- **MSRV 衝突の解消方針**: 主要候補の MSRV が workspace 1.85 を上回る（`fearless_simd` 1.x = 1.89、`wide` = 1.89、`multiversion` = 1.86、`criterion` = 1.86）。MSRV 引上げ or 旧版 pin の選択が必要。
- **AVX-512 の扱い**: 「将来」扱いか PoC 比較対象に含めるか。
- **denormal 具体方針**: FTZ/DAZ 設定の所在・カーネル内 flush の API 契約・ベンチでの扱い。
- **数値等価基準の具体値**: カーネルごとの bit 一致 / ulp 上限の割当て。
- **ベンチ基盤と再現性**: criterion vs divan、`profile.bench` の再現性設定、ハーネス配置（`[[bench]] harness = false` + dev-deps）。

## Alternatives and Comparison

エコシステム調査（2026-10、出典: crates.io / docs.rs / GitHub / Linebender blog 等）を構造化したもの。`Lean` は調査時点の傾向（仮説）であり採否決定ではない。

### SIMD ライブラリ・dispatch 方式

| Option | 概要 | Pros | Cons | Lean |
| --- | --- | --- | --- | --- |
| `fearless_simd` 1.1.0 + `fearless_simd_macros` 0.1.0（2026-10-06、Linebender。1.0 は 2026-09-21） | safe portable SIMD + dispatch 内蔵 | Apache-2.0 OR MIT。依存ゼロ（optional libm）。`Level::new()` が `is_x86_feature_detected!` で初回検出 → atomic キャッシュ。Level enum = Fallback/Sse2/Sse4_2/Avx2/Avx512/Neon/WasmSimd128（`#[non_exhaustive]` — 公開 enum 規約と同型）。3 層の抽象度（autovec `#[simd]` proc-macro / portable `Simd` trait / `kernel!` intrinsics）。公開 API 全て safe。3 年保守公約 | **MSRV 1.89 で workspace MSRV 1.85 と衝突**。新しめで実績はこれから。`min`/`max`/`mul_add` の NaN・±0・非 FMA レベルでの意味が implementation-defined（`*_precise` 版あり） | yes（第一候補） |
| `pulp` 0.22.3（2026-06-20、sarah quiñones） | safe SIMD + `Arch::new()` ランタイム検出 → `WithSimd::with_simd<S: Simd>` dispatch | 実績十分（faer 系列、31M DL）。`#[pulp::with_simd]` マクロあり | MIT のみ。MSRV 未宣言（build.rs は 1.83 分岐のみ — 実測要）。依存約 8 個（bytemuck, cfg-if, libm, num-complex, paste, pulp-wasm-simd-flag, raw-cpuid, reborrow）。x86-v4（AVX-512）は bytemuck avx512_simd feature 経由 | yes（対抗比較） |
| `multiversion` 0.9.0（2026-09-06、calebzulawski） | 関数単位 multiversioning（`#[multiversion(targets(...))]`）、ランタイムで cached fn pointer 選択 | MIT OR Apache-2.0。no_std 可。0.9 で target_feature_11 対応。dispatch のみのため構成が単純 | MSRV 1.86。SIMD 抽象化を持たない → 中身は autovec スカラー or `std::arch`（unsafe カプセル化自前） | maybe（dispatch 層代替の第 3 候補） |
| `wide` 1.7.1（2026-09-14、Lokathor） | portable SIMD 型ライブラリ | Zlib OR Apache-2.0 OR MIT | **ビルド時検出のみ** — 公式 docs が「runtime feature detection via is_x86_feature_detected! or multiversion do not work with wide」と明記（wide 内部の intrinsic 選択が wide 自身の cfg で確定するため）。MSRV 1.89 | no（ランタイム dispatch 不可で脱落。「multiversion + wide」組合せ案も否定。配布バイナリのランタイム dispatch 方針と不整合） |
| `std::simd` | portable SIMD 標準 API | 標準化すれば依存ゼロ・将来の最適解 | 引き続き nightly のみ（tracking issue open、2026-08 最終更新）→ stable-only 方針で使用不可 | no |

### ベンチマーク基盤

| Option | 概要 | Pros | Cons | Lean |
| --- | --- | --- | --- | --- |
| `criterion` 0.8.2 | 統計的ベンチの事実標準 | `Throughput::Elements`・alloca 配置ランダム化等の機能多。設計ベースライン「criterion 等でカーネル別ベンチマーク」と整合 | MSRV 1.86（workspace 1.85 を超過）。依存重い（rayon + plotters） | yes（既定候補） |
| `divan` 0.1.21 | 軽量ベンチ | MSRV 1.80・依存軽。`cargo bench -- --test` で CI 煙試験が容易 | 機能は criterion より少ない | maybe（代替） |

### 等価性テスト基盤

- `approx`（`UlpsEq` / `assert_ulps_eq!` / `assert_relative_eq!`）・`float_eq` 1.0.1（`float_eq!(a,b,ulps<=n)`・`DebugUlpsDiff`）ともに現役。
- ビット完全一致は `to_bits()` 比較 or `f32::total_cmp` で検証可能。

### denormal / FTZ/DAZ

- Rust での FTZ/DAZ 設定は形式的 UB（`std::arch` docs 明記、`_mm_setcsr` は 1.75 で deprecated → inline asm 推奨）。
- `no_denormals` 0.3.0（MIT、RAII で FTZ+DAZ 設定/復元）は "use at your own risk" の unsafe API。
- 実践的代替: (a) ホスト側 FTZ 設定に委ねる、(b) カーネル内ソフトウェア flush（DC offset / 閾値）、(c) ベンチで denormal 入力を排除、(d) PoC で denormal 有無の性能差を観測対象にする。

### dev-dep / ベンチハーネス配置の注意

- `resolver = "2"` で dev-dep feature は test/bench ビルド時のみ unify。`[[bench]] harness = false` + dev-deps が標準配置。
- MSRV 1.85 を超える dev-dep は `cargo test` / `--all-targets` で resolution エラーになりうる（`resolver.incompatible-rust-versions = "fallback"` もあるがベンチ再現性のため pin 推奨）。

## Tentative Conclusions and Hypotheses

以下はすべて仮説であり、最終決定は採否 ADR で行う。

- **仮説 1（バックエンド第一候補）**: `fearless_simd` 1.1.0 + `fearless_simd_macros` が最有力 — safe API・dispatch 内蔵・Fallback レベル内蔵で二層構造と整合・依存ゼロ・Apache-2.0 OR MIT・`#[non_exhaustive]` Level が公開 enum 規約と同型。
- **仮説 2（対抗比較）**: `pulp` を同一カーネルで対抗比較する — dispatch 方式（`Arch`/`WithSimd` vs `Level`/dispatch）と意味論差を直接評価できる。
- **仮説 3（dispatch 層代替）**: `multiversion` は dispatch 層代替の第 3 候補 — 中身は autovec スカラーが最も自前度低い。
- **仮説 4（ベンチ基盤）**: criterion 既定・divan 代替で十分（両方を並行評価する必要なし）。
- **仮説 5（MSRV）**: MSRV 1.89 への引上げを検討課題として切り出すのが現実的（`fearless_simd` 1.x / `wide` が 1.89、`multiversion` / `criterion` が 1.86 で、1.85 維持は旧版 pin を強いる）。

### カーネル案と等価性契約の方向性（仮説）

| カーネル | 等価性契約の方向性 | 備考 |
| --- | --- | --- |
| ベクトル加算/乗算（gain） | ビット完全一致 | メモリ帯域律速 |
| クリッピング | ビット完全一致（要規約） | NaN/±0 規約注意（`*_precise` 版 or NaN 非入力契約） |
| レベルフェード（ramp） | bit 一致 or ulp 上限 | 構成次第 |
| ドット積 / FIR | ulp 上限 | reduction 順序差・FMA 有無 |
| 正弦波生成 | —（SIMD 化困難なカーネルの実例） | 採否判断の反証材料 |
| IIR biquad（N ch 並列） | ulp 上限 + denormal | denormal 主戦場 |

等価性テストは「スカラー vs 各 dispatch level」の matrix とし、入力に denormal / ±0 / NaN / Inf / 非アライン先頭 / レーン幅±1 を含める。

## Open Questions

調査からの未検証引継ぎ事項、および PoC 実装・採否 ADR で残る問い:

- `pulp` の正式 MSRV（未宣言、実測か CI 確認が必要）
- `fearless_simd` の非 FMA レベルでの `mul_add` が真の fused か
- `pulp` の `Arch` が非対応ターゲットで Scalar fallback するか
- `multiversion` の小カーネルでの indirect call オーバーヘッド実測
- `no_denormals` の実装詳細（asm か deprecated intrinsic か）
- criterion の alloca ランダム化の Windows/macOS での実効性
- `float_eq` / `approx` の最新版・MSRV

## Promotion Candidates

- [ ] spec-doc needed: PoC の成果物・受入基準（本 discovery から派生）
- [ ] adr-doc needed: PoC アプローチ決定（比較対象・ベンチ基盤・ハーネス配置）
- [ ] 実装完了時に採否 ADR（SIMD バックエンド・二層構造の採否）を作成

## Sources and Relations

- 一次ソース: GitHub Issue [#19](https://github.com/scarecrow173/ruviform/issues/19)（`relations.source` / `relations.derives-from`）。比較ベンチマーク・等価性基準・採否 ADR 化の要求の正本。
- 上位設計: 設計ベースライン [ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md)「DSP／SIMD 設計（A.4）」節（二層構造の有力仮説・候補一覧・要 PoC 一覧）および分類表の「要PoC」行（`relations.derives-from` / `relations.related`）。
- 親 Issue: GitHub Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（`relations.references`）— 設計ベースラインの発生源。
- 作業契約: リポジトリルートの [AGENTS.md](../../AGENTS.md)（`relations.related`）— stable-only・MSRV 1.85・unsafe 原則禁止・公開 enum 規約の確定制約の所在。
- 調査入力: エコシステム調査レポート（2026-10、出典: crates.io / docs.rs / GitHub / Linebender blog 等 — 未検証点は Open Questions に明記）。`docs/research/` 相当の独立ファイルは存在しないため、本書が調査結果の canonical 記録を兼ねる。
