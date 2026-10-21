---
id: "SPEC-034Jci19siJZexLs53Xh5d"
type: "spec"
status: "implemented"
title: "DSP/SIMD 比較ベンチマーク PoC と採否 ADR"
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
  implemented-by:
    - "../designs/dsp-simd-poc.md"
    - "../plans/dsp-simd-poc.md"
  depends-on:
    - "establish-rust-dev-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/dsp-simd-poc-issue-19.md"
  derived-by:
    - "../adr/dsp-simd-backend-adoption.md"
    - "../designs/dsp-simd-poc.md"
    - "../plans/dsp-simd-poc.md"
  verifies: []
  verified-by:
    - "../test-specs/simd-equivalence-matrix-contract.md"
    - "../test-specs/bench-tasks-and-ci-smoke.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# DSP/SIMD 比較ベンチマーク PoC と採否 ADR

## What Should Be Built

GitHub Issue #19 で定義された PoC の成果物として、`ruviform-math` クレート内にスカラー参照実装と複数 SIMD バックエンド候補を同一 API で比較するベンチマークハーネスを構築し、等価性テストと採否 ADR までを一連の成果として出す。

- **比較ベンチマークハーネス**（`crates/ruviform-math`）: `[[bench]] harness = false` + dev-dependencies 構成のベンチ基盤。候補バックエンドを切り替えて同一ワークロードを計測できる構成とし、SIMD バックエンドクレートの型・API を `ruviform-math` の公開 API に露出しない（PoC コードは内部モジュール・dev-dep に封じ込める）。
- **スカラー参照実装カーネル群**: DSP 代表ワークロードとして、ベクトル加算・ゲイン乗算・クリッピング・レベルフェード（ramp）・ドット積・FIR・IIR biquad（チャンネル並列）・正弦波生成などからカーネルを選定し、正となるスカラー実装を用意する。SIMD 化が困難なカーネル（例: 正弦波生成）も採否判断の反証材料として含める。
- **候補バックエンドの同等実装**: discovery で絞られた `fearless_simd`（第一候補）・`pulp`（対抗比較）・`multiversion` + autovec スカラー（dispatch 層代替の第 3 候補）を同一カーネル API で実装し、スカラーと並べて計測可能にする。
- **等価性テスト matrix**: 「スカラー vs 各 dispatch level」の組合せを網羅し、入力に denormal / ±0 / NaN / Inf / 非アライン先頭 / レーン幅 ±1 の端数長を含める。カーネルごとの等価性契約（ビット完全一致 or ulp 上限）をテストで検証する。
- **採否 ADR**（最終成果物、`docs/adr/`）: SIMD 二層構造（ランタイム分岐 + フォールバック・dispatch 層の `ruviform-math` 集約）の採否、バックエンド選定、等価基準の具体値、AVX-512 の扱い、denormal 方針、MSRV 判断を記録する。
- **ベンチマーク結果の記録**: impl record / experiment log として計測条件（OS・CPU・ツールチェーン・プロファイル）と結果を残し、採否 ADR の根拠とする。

## Why Now

- 設計ベースライン「DSP／SIMD 設計（A.4）」および分類表で **要PoC** とされた項目（SIMD 二層構造の採否・バックエンド選定・採否基準・AVX-512 扱い・denormal 具体方針・数値等価基準の具体値）を確定させるための PoC が本 Issue。これらが未確定のままでは、後続の DSP 実装 Issue（`ruviform-dsp`・`ruviform-math` のカーネル API 確定）が着手できない。
- SIMD 二層構造は現時点では「有力仮説」であり、ベンチマーク比較評価による確定が設計ベースラインの採否条件として明示されている。実測なしに構造を採用・拒否のどちらに決めても根拠を欠く。
- MSRV 1.85 と主要候補クレートの MSRV 要求（`fearless_simd` 1.89 / `multiversion` 1.86 / `criterion` 1.86）の衝突が discovery で判明済みであり、MSRV 引上げの要否を判断するにも実測データが必要。後送りすると後からアーキテクチャ前提を破壊するリスクが大きい。

## Users and Value

- **DSP・基盤の実装者（後続 Issue の作業者）**: 採否 ADR により SIMD 二層構造の採否とバックエンドが確定し、カーネル実装・dispatch 層設計を確定的な方針の下で進められる。
- **レビュアー・意思決定者**: スカラー対比の計測データと等価性テスト結果に基づく採否判断が可能になり、「調査で有力候補が見つかった」ことと「採用決定」を区別できる。
- **将来のメンテナー**: 等価性契約（bit 完全一致 / ulp 上限）と境界入力テストが残るため、バックエンド差し替えや新規カーネル追加時の検証基準が再利用できる。

## Scope

### In Scope

- `crates/ruviform-math` への dev-dependencies ベースの PoC 実装（ベンチハーネス・スカラー参照カーネル・候補バックエンド実装。公開 API は増やさないか最小限に留める）
- カーネルごとの等価性契約定義と、「スカラー vs 各 dispatch level」の等価性テスト（denormal / ±0 / NaN / Inf / 非アライン / 端数長入力を含む）
- Windows x64 を主計測環境とするベンチ実行（複数サイズ・端数長を含む）と結果の記録（impl record / experiment log）
- 採否 ADR の作成（SIMD 二層構造の採否・バックエンド選定・等価基準具体値・AVX-512 扱い・denormal 方針・MSRV 判断）
- 必要に応じた `profile.bench` の再現性設定（ルート `Cargo.toml`）
- README / AGENTS.md / `mise.toml` の `bench` タスク記述と実態の整合（bench が no-op でなくなる点を含む）

### Out of Scope

- 採用バックエンドの本番 `[dependencies]` 昇格と `ruviform-math` 公開 API としての確定 — 採否 ADR の結論を受けた後続実装 Issue の責務
- `ruviform-dsp` クレートの新設・フィルタ/リサンプリング等の実用 DSP カーネル — 後続実装 Issue
- `std::simd`（nightly 限定）の利用と、その安定化後の移行判断 — 設計ベースラインで stable-only が確定制約
- `wide` をランタイム dispatch で利用する案 — 公式 docs でランタイム検出非対応が明記され discovery 時点で脱落済み
- マルチ OS / マルチアーキでのベンチ比較 — Windows x64 が主計測。他 OS は CI `workflow_dispatch` 手動トリガー等で補足するに留める
- denormal 対策の実装そのもの（エンジン側 FTZ/DAZ 設定。Rust では形式的 UB）— 方針を ADR に記録するまでが本 PoC の範囲。実装はエンジン層 Issue へ

## Requirements

- Rust edition 2024・ワークスペース MSRV `rust-version = "1.85"`・stable ツールチェーンのみ。`unsafe` は原則禁止。必要な場合のみ SAFETY コメント付きの局所許可（`#[expect(unsafe_code)]`、未対応なら `#[allow(unsafe_code)]`）で例外的に認める。候補クレート内部の `unsafe` は対象外とするが、その公開 API が利用側に `unsafe` を要求する場合は採否評価の減点要素とする
- SIMD バックエンドクレートの型・API を `ruviform-math` の公開 API に露出しない。PoC 用の候補クレートは `[dev-dependencies]` として導入し、公開 API 汚染を最小化する（設計ベースライン ruviform-math 責務表・公開 API 境界と整合）
- スカラー参照実装を必須とし、カーネルごとに「ビット完全一致」または「ulp 上限（具体値）」の等価性契約を定義して、スカラーを参照としてテストで検証する
- ベンチマークは複数サイズと端数長（レーン幅の倍数 ±1）を含むワークロードで実行する（設計ベースラインの性能検証計画に整合）
- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench` と `prek run --all-files` がグリーンであること
- 採否 ADR は設計ベースラインの採否基準（スカラー比の性能向上・コンパイルコスト・`unsafe` カプセル化範囲・MSRV 適合・保守状況）に沿って結論を出すこと。不採用・二層構造否決も正当な結論とする

## Acceptance Criteria

- [x] スカラー参照カーネルと各候補バックエンド実装が同一 API で比較可能なベンチハーネスが `crates/ruviform-math` に存在する（`[[bench]] harness = false` + dev-deps 構成）
- [x] `mise run bench`（または `cargo bench -p ruviform-math`）で候補別・サイズ別の計測が実行できる
- [x] カーネルごとの等価性契約（bit 完全一致 or ulp 上限値）が定義され、「スカラー vs 各 dispatch level」の等価性テストが `mise run test` でグリーン（denormal / ±0 / NaN / Inf / 非アライン / 端数長入力を含む）
- [x] 採否 ADR が `docs/adr/` に存在し、SIMD 二層構造の採否・バックエンド選定・MSRV 判断・等価基準の具体値・denormal / AVX-512 方針を記録している
- [x] ベンチマーク結果が impl record / experiment log として計測条件つきで記録されている
- [x] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench` と `prek run --all-files` がグリーン
- [x] README / AGENTS.md / `mise.toml` の記述が実態と整合している（`bench` タスクが no-op ではなくなった点を含む）

充足実績: 採否 ADR は [dsp-simd-backend-adoption.md](../adr/dsp-simd-backend-adoption.md)（`ADR-034JEe7SAbksaycyrum16R`・accepted）、ベンチ結果は experiment log [dsp-simd-poc-20261021.md](../impl/exp/dsp-simd-poc-20261021.md)（2 run × 756 ケース・環境メタデータ付き）と impl record [dsp-simd-poc-benchmark.md](../impl/ir/dsp-simd-poc-benchmark.md) に記録済み。AC-6 の `mise run bench` グリーン要求は Phase F（T6）の全量計測実績（上記 experiment log の 2 run 完走）で充足済みであり、Phase H の最終検証では `mise run bench-smoke` でハーネス健全性を再確認する運用（plan V-8 の充足経路どおり）。

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| MSRV 1.85 vs 候補クレート要求（`fearless_simd` 1.89 / `multiversion` 1.86 / `criterion` 1.86） | PoC 内では dev-dep として導入し必要に応じて旧版 pin or toolchain 解決で回避。MSRV 引上げ自体は本 spec で決定しない | 採否 ADR で MSRV 判断を記録し、引上げが必要なら別途 MSRV 引上げ Issue（または dev-dep 昇格時の後続実装 Issue）へ | workspace `rust-version` を暗黙に破壊し、CI・依存解決が分断される |
| 採用バックエンドの本番 `[dependencies]` 昇格・公開 API 確定（dispatch 層の `ruviform-math` 集約の実装） | dev-dep + 内部モジュールに封じ込め、公開 API は確定しない | 採否 ADR の結論を受けた後続実装 Issue | PoC の実験的依存が公開 API・依存木に定着する |
| 他 OS / アーキでのベンチ再現（aarch64 NEON・AVX-512 環境等） | Windows x64 を主計測とし他環境は検証しない | CI `workflow_dispatch` 手動トリガー等で補足。必要なら後続 Issue | 単一環境の計測結果を全プラットフォームの採否根拠に誤用する |
| denormal 対策の実装（FTZ/DAZ 設定、カーネル内 flush） | 方針のみ ADR に記録。Rust での FTZ/DAZ は形式的 UB のため実装は行わない | エンジン層 Issue（ホスト側 FTZ/DAZ 設定 or カーネル内ソフトウェア flush の API 契約） | denormal 入力での性能崖を未計測のまま見逃す |
| `std::simd` 安定化後の移行判断 | nightly 限定のため評価対象外 | 設計ベースラインの「保留」項目。stable 化をトリガに再評価 | 将来の最適解への移行パスを設計に織り込めない |

## Sources and Relations

- 一次ソース: GitHub Issue [#19 [poc] DSP/SIMD 比較ベンチマークと採否 ADR](https://github.com/scarecrow173/ruviform/issues/19)（`relations.source`）。Issue #1 設計ベースライン「後続実装 Issue の提案一覧」から起票された。
- 派生元（`relations.derives-from`）: [discovery ドキュメント](../discovery/dsp-simd-poc-issue-19.md)（Issue #19 の briefing／探索フェーズ成果。エコシステム調査・候補比較・仮説・Open Questions の canonical 記録）。
- 前提（`relations.depends-on`）: spec [establish-rust-dev-foundation.md](establish-rust-dev-foundation.md)（`SPEC-034IcwhwIFSNUNKtX7KK0Q`）が確立した Rust 開発基盤（workspace・lint・mise 検証タスク）を本 spec の検証基盤として利用する。
- 関連（`relations.related`）: リポジトリルートの [`AGENTS.md`](../../AGENTS.md)（stable-only・MSRV 1.85・`unsafe` 原則禁止・公開 enum 規約の確定制約）、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（「DSP／SIMD 設計（A.4）」節の要 PoC 項目・ruviform-math 責務表・分類表）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（RT 経路の制約前提）、[platform ADR](../adr/platform-and-toolchain-targets.md)（x86_64 / aarch64 のランタイム検出方針）、[公開 enum ADR](../adr/public-enum-non-exhaustive-policy.md)（`#[non_exhaustive]` 統一方針）。
- 参考（`relations.references`）: 親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（設計ベースラインの一次ソース）。
