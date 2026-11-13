---
id: "EXP-issue-275-obs-owner-map"
title: "Issue #275 OBS owner map 監査メモ — Issue 状態・crate/feature・CI lane の棚卸し"
created: "2026-11-14"
updated: "2026-11-14"
---

# Issue #275 OBS owner map 監査メモ(2026-11-14)

- 計測契約: `docs/specs/issue-275-obs-owner-map.md` +
  `docs/designs/issue-275-obs-owner-map-design.md`
- 正本証拠: `evidence/rf275/obs-owner-map-audit.md`

## 1. 問い

Issue #275 残件の対応付けに必要な実測値 — OBS-01〜11 の
実装 Owner Issue 状態、`ruviform-observability` の
module/feature/公開 API、既存 CI lane の到達範囲 — を
どう固定するか。

## 2. 調査と実測

### Issue 状態（`gh issue view`、2026-11-14）

- 実装 Issue: #276/#277/#278/#279/#280・#289/#290/#291/
  #292/#293 は全て CLOSED。OBS-06 #281 は OPEN（検証
  Gate）。#294・#219 は OPEN。AL #127・SV #28 は OPEN、
  AL #138・SV #81 は CLOSED（履歴）。
- 結論: 未着手の実装 Issue は存在しない。残件は全て
  検証/Gate 領域であり、新規実装 Issue の起票は不要。

### crate/feature（`Cargo.toml`・`src/`・`cargo tree`）

- `default = []`。optional feature は `logging`・
  `tracing-export`・`allocation-probe`・`integration` の
  4 つ。`collector`/`diagnostics`/`debug-tools` は feature
  として未採用で無条件モジュール。
- feature unify の実測（`cargo tree --workspace -e
  features -i ruviform-observability`、補助証拠）:
  `integration` のみ `ruviform-audio` dev-dep 経由で
  `cargo test --workspace`/`check --all-targets` 系に
  unify される。`logging`/`tracing-export`/
  `allocation-probe` を有効化する workspace member は
  なし。
- 帰結: feature 付き module の compile+test は既存では
  `msrv-verify`/`test-observability-msrv`
  （`--all-features`・Linux・rust@1.89.0）のみ。Win/macOS
  native の feature 付き build/link/test は `NotRun`。

### CI lane（`.github/workflows/`・`mise.toml`）

- `ci.yml` は `workflow_dispatch` のみ（daily-ci.yml が
  main SHA で起動）。lane: `verify`（ubuntu+windows）・
  `verify-macos`・`rt-verify`（ubuntu、`test-rt` =
  `cargo test --workspace rt_`）・`msrv-verify`。
- `consumer-verify.yml`: `verify-audiolens`（test-rt lane
  なし）・`verify-synvelis`（test-rt あり）+ fail-closed
  `summary`。
- 帰結: 対応表の「既存 CI Case」列は lane 定義と到達範囲
  の対応付けであり、個別 SHA の Pass を意味しない。

## 3. 採用した判定

- 対応表は「実行し得る lane」の対応付け + 要件状態
  （Required/Deferred/NotApplicable）を記し、実行結果は
  `NotRun` を維持して各 run の artifact に委ねる。
- Deferred 4 項目（実外部 Provider・実 OS GUI・実 Device・
  長時間 Profiler）に再開 Owner と Activation Trigger を
  記載。feature compile matrix は Deferred ではなく
  #281 G0 の継続 gate 項目（Required・NotRun）。

## 4. 棄却した選択肢

- 対応表で per-module の公開 API を全列挙する案 →
  per-issue 正本との drift リスクがあるため代表型のみに
  留め、詳細は正本参照とした。
- lane 名から個別 Case の Pass を推論して記す案 →
  POLICIES の非昇格原則に反するため棄却。
