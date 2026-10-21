---
id: "IMPL-034JEzb6zrstxQuNrV0fmM"
type: "impl"
status: "completed"
title: "DSP/SIMD PoC benchmark"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-math/benches/simd_poc.rs"
      - type: "file"
        path: "crates/ruviform-math/src/simd_poc.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_equiv.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/mod.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/equiv.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/inputs.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/fearless_impl.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/pulp_impl.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/multiversion_impl.rs"
      - type: "file"
        path: "docs/adr/dsp-simd-backend-adoption.md"
      - type: "file"
        path: "docs/impl/exp/dsp-simd-poc-20261021.md"
      - type: "file"
        path: "docs/impl/exp/dsp-simd-poc-benchmark-comparison.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/dsp-simd-poc-20261021-01.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/dsp-simd-poc-20261021-02.jsonl"
    modified:
      - type: "file"
        path: "crates/ruviform-math/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-math/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../plans/dsp-simd-poc.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../exp/dsp-simd-poc-20261021.md"
    - "../exp/dsp-simd-poc-benchmark-comparison.jsonl"
    - "../../adr/dsp-simd-backend-adoption.md"
    - "../../adr/dsp-simd-poc.md"
    - "../../specs/dsp-simd-poc-adr.md"
    - "../../designs/dsp-simd-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/simd-equivalence-matrix-contract.md"
    - "../../test-specs/bench-tasks-and-ci-smoke.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "../exp/dsp-simd-poc-benchmark-comparison.jsonl"
    rejected: []
---

# DSP/SIMD PoC benchmark

## Summary

plan `PLAN-034JdOWI3vblJW11kIueY0`（Issue #19、単一実装ストリーム T1〜T7）として、DSP/SIMD 比較ベンチマーク PoC の実装・計測・採否判断を完了した。`crates/ruviform-math` に `#[doc(hidden)] pub mod simd_poc`（`src/simd_poc.rs` — スカラー参照カーネル 8 種: add / scale / clamp_unit / fade / dot / fir / biquad / sine）を配置し、候補バックエンド `fearless_simd` / `pulp` / `multiversion`(+autovec) の同等実装と等価性契約を `tests/simd_poc/` 共有モジュールに、全組合せ matrix を `tests/simd_equiv.rs` に、criterion 計測 matrix を `benches/simd_poc.rs` に実装した。2 run × 756 ケースの全量計測を experiment log に記録し、採否 ADR `ADR-034JEe7SAbksaycyrum16R`（accepted）で「二層構造採用（カーネル別適用）・`fearless_simd` 条件付き採用・等価基準確定値・AVX-512 将来扱い・denormal policy-only・MSRV 1.85 維持（dev-dep 封じ込め）」の 6 論点を確定した。

## Context

- 実装対象: [plan](../../plans/dsp-simd-poc.md)（`PLAN-034JdOWI3vblJW11kIueY0`）。本 IR は同 plan の T1〜T7（ハーネス骨格 → スカラー参照 → 候補バックエンド → 等価性 matrix → criterion ベンチ → 計測収集 → 採否 ADR）を単一実装ストリームとして記録する。T8（文書整合）の記録タスクは [record-impl-and-sync-docs.md](../../tasks/record-impl-and-sync-docs.md)
- 上流: [spec](../../specs/dsp-simd-poc-adr.md)（受け入れ基準の正本）、[design](../../designs/dsp-simd-poc.md)（カーネル API 抽象形・等価性契約・ベンチ methodology の正本）、[評価アプローチ ADR](../../adr/dsp-simd-poc.md)（ハーネス配置 = ruviform-math dev-deps・比較 3 系統・criterion 基盤の決定）
- 前提制約: edition 2024・workspace MSRV `rust-version = "1.85"`・stable のみ・`unsafe` 原則禁止・候補クレート型の公開 API 非露出（`[dev-dependencies]` + `#[doc(hidden)]` 封じ込め）・スカラー参照必須・カーネル別等価性契約（bit 完全一致 or ulp 上限）

## Implementation

### Changes

| 領域 | ファイル | 内容 |
| --- | --- | --- |
| スカラー参照 | `crates/ruviform-math/src/simd_poc.rs` | 8 カーネルのスカラー参照実装（`add`/`scale`/`clamp_unit`/`fade`/`dot`/`fir`/`biquad`/`sine`）+ 係数・状態型（`BiquadCoeffs`/`SineCoeffs`/`SineState`/`fir_taps`）と決定的入力生成（`Xorshift64Star`/`uniform_f32`/`denormal_mix` 等）。`#[doc(hidden)] pub` で lib から露出し tests/benches 共用。公開 API 契約外の実験ブリッジ |
| 候補実装 | `tests/simd_poc/{fearless_impl,pulp_impl,multiversion_impl}.rs` | 3 系統の同等実装。fearless は `Level` トークン強制指定（fallback/sse2/sse4_2/avx2/avx512）、pulp は `Arch` 強制指定（scalar/v2/v3/v4）、multiversion は dispatched 列のみ |
| 等価性基盤 | `tests/simd_poc/{mod,equiv,inputs}.rs` | 等価性契約（bit 完全一致 = `to_bits`+NaN 一致 / ulp 上限 `MAX_ULPS_{DOT,FIR,BIQUAD}`）・入力生成（denormal/±0/NaN/Inf/非アライン/端数長）・probe skip 機構 |
| matrix テスト | `tests/simd_equiv.rs` | 「スカラー vs 各構成可能列」の全組合せ等価性検証（構成不能列は probe skip 記録） |
| ベンチ | `benches/simd_poc.rs` + `[[bench]] harness = false` | 8 カーネル × 9 計測列 × 7 長さ（端数長含む）× 入力 variant の criterion matrix |
| 構成 | `crates/ruviform-math/Cargo.toml` / `Cargo.lock` | dev-deps 追加: `fearless_simd` 1.1.0（`force_support_fallback`）+ `fearless_simd_macros`、`pulp` 0.22.3（`x86-v4` + `macro`）、`multiversion` 0.9.0、`criterion` 0.8.2（`default-features = false`）、`approx` |
| 検証入口 | `mise.toml` / `.github/workflows/ci.yml` | `bench` タスク実装（no-op 解消）+ `bench-smoke` 追加（criterion `--test` で各ベンチ 1 回実行）・CI `bench` ジョブは `mise run bench-smoke` を呼ぶ smoke 構成 |
| 結果記録 | `docs/impl/exp/dsp-simd-poc-20261021.md` + `data/*.jsonl` + `dsp-simd-poc-benchmark-comparison.jsonl` / `docs/adr/dsp-simd-backend-adoption.md` | 計測レポート（環境メタデータ・判定表・denormal 崖・dispatch オーバーヘッド）・生データ・イベントログ・採否 ADR |

### Decisions During Implementation

- **PoC コードの配置**: 評価アプローチ ADR 当初案の `pub(crate)` 配置は dev-dep 制約で成立しないため、設計書どおり `#[doc(hidden)] pub mod simd_poc` + `tests/` 共有 mod の形を採用（候補クレート型を公開 API から構造的に隔離）
- **等価性契約の確定値**: `dot` ulp ≤ 256（実測 max 46、f64 真値照合で「実装バグではなく契約不適合」と診断した上で ~5.5x マージン）、`fir`/`biquad` ulp ≤ 8（実測 max 0。FMA 版 fir は ~3994 ulp の打ち消しドリフトを実測したため非融合 multiply-add を採用）、elementwise/再帰一致系 5 カーネルは bit 完全一致。確定値は採否 ADR で裁定
- **`rt_` テストは追加しない**: 本 PoC は RT 経路の対象ではない（PoC コードは dev-dep 封じ込めの実験ブリッジであり、`Processor`/`RtContext` のランタイム API は未実装）。`rt_` プレフィックスの RT 契約テストを PoC に新設する契約根拠がないため未追加とし、`mise run test-rt` は 0 件一致のスキップ相当でグリーン維持
- **`unsafe` 不使用**: PoC コードの `unsafe` は 0（`assume_supported`/`new_unchecked`/`std::arch` 不使用）。`unsafe_code` lint ゲート通過で検証済み
- **構成不能列の扱い**: AVX-512（`fearless_avx512`/`pulp_v4`）・SSE2（`fearless_sse2`/`pulp_v2` はベースラインのため bench 対象外）は probe skip 記録とし、matrix/ベンチ両方で「列は存在するが skip」として可視化
- **全量計測の分割実行**: `mise run bench` 単一プロセス全量は並行エージェント多数稼働下の Windows コミット上限逼迫で 2 回 abort したため、カーネル単位 8 回分割で完走（experiment log「実施記録」。産物は全量実行と同一）

## Related Experiments

### Adopted

- [dsp-simd-poc-benchmark-comparison.jsonl](../exp/dsp-simd-poc-benchmark-comparison.jsonl) — 計測実行のイベントログ（start/observation/decision/validation/summary）。レポート文書 [dsp-simd-poc-20261021.md](../exp/dsp-simd-poc-20261021.md)（判定表・denormal 崖表・dispatch オーバーヘッド・補助データ）と生データ `data/dsp-simd-poc-20261021-01.jsonl` / `-02.jsonl`（756 行/run）が対応する一次証拠として採用

### Rejected

None.

## Validation

実施環境: Windows 11 x64・i9-9900K（AVX2 まで、AVX-512 非搭載）・rustc 1.99.0・criterion 0.8.2。

| Check | 実績 | 証拠 |
| --- | --- | --- |
| `mise run test` | グリーン — 等価性 matrix 全構成可能列クリア（bit 完全一致 5 カーネル、ulp: dot max 46≤256・fir/biquad max 0≤8） | T4 実績・採否 ADR Verification |
| `mise run check` / `lint` | グリーン・0 warnings — PoC `unsafe` 0 で `unsafe_code` ゲート通過 | 採否 ADR Verification |
| `mise run test-rt` | グリーン（`rt_` 0 件一致のスキップ相当 — RT テスト未追加は上記判断どおり意図的） | mise タスク定義 |
| `mise run bench` | 2 run × 756 ケース完走（各 ~25–27 分）。単一プロセス全量はメモリコミット逼迫で abort したためカーネル別 8 分割で実施 — 採否 ADR が要求する全量計測として充足 | experiment log・レポート |
| `mise run bench-smoke` | CI `bench` ジョブの呼ぶ入口として導入・各ベンチ 1 回実行でハーネス健全性を確認 | ci.yml・test-spec |
| MSRV 検証 | `rustup run 1.85 cargo check -p ruviform-math --lib --locked` グリーン（dev-dep 封じ込め下で `--lib` 範囲は 1.85 適合） | 採否 ADR Verification・レポート補助データ |
| doc 監査 | `audit_docs.js --type adr` 等、T7 時点でクリーン | T7 実績 |

計測結果の要点（スカラー比・2 run 平均 @256/1024/4096、>1 で高速）: `fearless_avx2` dot 7.9–8.3x / fir 3.6–5.8x / biquad ~2.8x / fade 4.1–6.7x、`pulp_v3` dot 8.1–8.9x / fir 2.6–3.7x / fade 3.9–6.6x。elementwise 小バッファでは dispatched 経路がスカラーに劣後（`fearless_dispatched` add/64=0.58x 等）→ 二層構造は「カーネル別適用 + サイズ応答ルーティング」で採用。`multiversion` は dot ~1.0x・fir 0.69–0.73x で不採用。`sine` は全 impl ≈1.00x（SIMD 非適用領域の反証材料として設計どおり）。

## Risks

- `fearless_dispatched` の軽量カーネル劣後（@1024 で add 0.62x・scale 0.66x vs 強制 avx2）が per-call dispatch コストかノイズか未分離 — 計測は ~350 プロセス並行稼働下で実施され、run 間 |Δ| p90=25.7% のノイズを含む（run 内比率評価で相殺済みだが、低負荷環境での再計測を推奨）
- AVX-512・aarch64 NEON は未計測（本機に AVX-512 非搭載、aarch64 は CI `--all-targets` コンパイル確認のみ）。`dot` ulp ≤256 の確定値は NEON 4-lane/AVX-512 16-lane のヘッドルーム込みだが実測未確認
- denormal 崖（fir で最大 ~31.8x）は性能のみで等価性は維持されるが、対策未実装。FTZ/DAZ は Rust 上で形式的 UB のためカーネル側では対策しない方針（採否 ADR 決定 5）
- 候補 dev-dep のコールド `cargo check --workspace --all-targets` 増分 ~30s（criterion/pulp 系の推移的依存を含む）は残存コスト
- `cargo +1.85` での fresh resolve は dev-dep 範囲に適合版がなく失敗し得る（lockfile 運用では非問題として ADR に記録済み）

## Follow-ups

- **MSRV 引上げ Issue の起票**（未着手）: `fearless_simd` の本番 `[dependencies]` 昇格には workspace `rust-version` ≥1.89 への引上げが前提 — spec「Deferred Design Concerns」どおり別 Issue で判断（採否 ADR 決定 6）
- **AVX-512 / aarch64 実測**: 実機または CI `workflow_dispatch` 環境で matrix/bench 再実行し、16 レーン `dot` が ulp 256 に収まることを確認してから有効化（未検証のまま本番 dispatch に含めない）
- **dispatch 層の本番設計**: per-call `dispatch!` を軽量カーネルに使わず、`Level`/`Simd` トークンを prepare/ブロック境界で保持・繰り上げる設計（採否 ADR 決定 1/2 の条件）— 後続 DSP 実装 Issue
- **denormal 対策の実装**: エンジン層 Issue へ移管（ホスト側 FTZ/DAZ 設定 or カーネル内ソフトウェア flush の API 契約）
- **検証済み代替の再評価経路**: `pulp` 実装は PoC ハーネスに残置 — fearless_simd の dispatch が本番統合で構造的問題を示した場合の再評価対象（採否 ADR 決定 2）
