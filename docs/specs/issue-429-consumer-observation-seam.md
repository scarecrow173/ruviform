---
id: "SPEC-Wb3KqT8nLf2RvXc6HmPd9Y"
type: "spec"
status: "proposed"
title: "Issue #429 — Consumer 観測 seam・残 Fake/GPUI 検証の公開"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/429"
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/audio-lens/pull/169"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lifecycle_fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
    - "docs/specs/issue-390-driver-contract-lab.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-387-component-view-specs.md"
---

# Issue #429 — Consumer 観測 seam・残 Fake/GPUI 検証の公開

## 背景

#421（PR #428）は Meter/TimeAxis の GPUI 描画と Fake/GPUI parity の中核を main へ統合した。その際、AudioLens PR #169 が「`settle_for_test` / `inbox_len_for_test` が `pub(crate)` のため、consumer の test から GPUI driver の projection と intent を観測できない」と依存を記録し、設計判断（#421 design D5）は本 Issue へ移管された。

現状の問題は次のとおり。

- `ruviform-ui-gpui` の観測フックは `#[cfg(all(test, feature = "gpui-test"))]` の `pub(crate)` で、外部 crate からは到達できない。
- 観測対象の `ViewHost` は `RvfView` 内部に閉じており、consumer は mount 後の projection 状態を読めない。
- 仮に公開しても、`gpui::TestAppContext` を引数に取る形では consumer に gpui の test-support 依存と gpui 型を強制する。

## 目的

- consumer crate の test から、GPUI Driver の projection（mount された view の component 状態）と intent（`Action` 経路）を、gpui 型を含まない driver-neutral な公開 API で取得できること。
- 公開 seam を置いたままでも `check-ui-driver-boundary` が pass すること。
- seam を使う consumer 相当の parity 検証を、外部 crate の test として mise タスクに記録すること。

## 受入条件（Issue #429 本文に対応）

| # | Issue #429 受入 | 本 PR での扱い |
| --- | --- | --- |
| A1 | consumer crate から GPUI Driver の projection/intent を driver-neutral な観測 API で取得できる（または等価の公開 seam） | 達成。`GpuiObserver`（`observe` / `intents` / `dispatch_input` / `settle`）。外部 crate の `tests/consumer_observation.rs` が parity fixture を mount して観測する |
| A2 | boundary guard（`check-ui-driver-boundary`）が pass したまま seam が公開されている | 達成。公開 signature に `gpui` 型なし。guard の `dep-graph` / `signature` / `allowlist` が pass |
| A3 | 各項目の検証が mise タスクとして記録されている | 部分達成。観測 seam は `test-ui-consumer-observation`（新設）と `test-ui-gpui` で記録。残項目は下表の未達を参照 |

## 残 DoD の扱い（未達は明示する）

Issue #429 のスコープのうち、本 PR で実装しないものを次に明示する。いずれも完了と偽らず、PR 本文と worker_done に同じ内容を書く。

| 項目 | 状態 | 理由 |
| --- | --- | --- |
| `NumericInput` / `Transport` Control | 未達（NotRun） | #387 の Non-goals。Consumer 必要性の再評価が前提。観測 seam とは独立 |
| Waveform/Meter/Spectrum の長尺・RT snapshot・gap/NaN precision・bounded memory/cancel/stale・GPU release の GPUI 検証 | 未達（NotRun） | 範囲が大きく、本 PR の seam 公開を優先した |
| driver-lab manifest の残 lane（UI-DVR-03 GPUI の Cancel・遅延/順序逆転、UI-DVR-07/08 GPUI） | 未達（not_run のまま） | Cancel 等の lab binding が未接続。`cases.json` の status は実行結果と一致させ、変更しない |
| `LAB-SV-FADER-DOCK` / `LAB-AL-WAVEFORM-SEEK` の 2 scale factor × 2 theme × 状態遷移の GPUI 実行 | 未達（not_run のまま） | story の GPUI 実行 lab が未実装。`stories.json` の status は変更しない |

## スコープ（本 PR）

- `ruviform-ui-gpui`: 観測 seam（`gpui_driver/observe.rs`、`gpui-test` 境界）、window slot への view 参照の保持、外部 crate test。
- 既存の内部フック（`settle_for_test` / `inbox_len_for_test`）の撤去と、lifecycle fixture の seam への移行。
- raw parent 捕捉の gate を `cfg(test)` から `feature = "gpui-test"` へ修正（外部 crate test で gpui test platform の `window_handle` が panic していた欠陥）。
- `mise.toml` の `test-ui-consumer-observation` 新設、`docs/development-environment.md` の表。
- 文書 chain（spec → design → plan → task → test-spec）。

## 非スコープ

- 上記未達表の項目すべて。
- 実 OS window 上での観測（gpui test platform 上の観測に限る。実機の挙動は manual verification）。
- `ruviform-ui`（共通層）・`ruviform-ui-theme`・`ruviform-ui-webview`・plugin 系への変更。

## 不変条件

- 共通層（`ruviform-ui` / `ruviform-ui-theme`）と公開 seam に `gpui::*` 型を出さない。観測 DTO は `ruviform-ui` の契約型（`ComponentKey` / `ComponentSnapshot` / `Intent` / `WindowId`）だけで構成する。
- 公開 struct は `#[non_exhaustive]`、MSRV 1.89、workspace lint（`-D warnings`）準拠。
- seam は gpui test platform 上でのみ動作する（`gpui-test` は本番向けではない）。

## 検証の方針

- `test-ui-consumer-observation`（外部 crate test）と `test-ui-gpui`（crate 内 gpui-test 全体）で seam の振る舞いを検査する。
- `check-ui-driver-boundary` で公開面の gpui 非露出を機械検査する。
- 詳細は [設計](../designs/issue-429-consumer-observation-seam.md) と [test-spec](../test-specs/issue-429-consumer-observation-seam.md)。
