---
id: "SPEC-Wb3KqT8nLf2RvXc6HmPd9Y"
type: "spec"
status: "proposed"
title: "Issue #429 — Consumer 観測 seam・残 Fake/GPUI 検証の公開"
created: "2026-11-08"
updated: "2026-11-09"
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
| A3 | 各項目の検証が mise タスクとして記録されている | 達成。観測 seam は `test-ui-consumer-observation`（新設）と `test-ui-gpui` で記録。2026-11-09 のフォローアップで残項目の検証・判断も同タスクに記録した（下表参照） |

## 残 DoD の扱い（2026-11-09 フォローアップで再評価）

Issue #429 のスコープのうち、本 chain の初回 PR で実装しなかったものの現在の状態を次に示す。完了と偽らず、PR 本文と worker_done に同じ内容を書く。

| 項目 | 状態 | 根拠 |
| --- | --- | --- |
| `NumericInput` / `Transport` Control | 再評価済み — 新 kind 不要（composition で充足） | consumer 棚卸し（synvelis の transport 合成・audio-lens `gpui_parity` の利用 kind・`ValueDescriptor`/`ControlInput` の既存契約）で共通 semantic gap なし。[task doc の再評価記録](../tasks/issue-429-consumer-observation-seam.md)参照 |
| Waveform/Meter/Spectrum の長尺・RT snapshot・gap/NaN precision・bounded memory/cancel/stale・GPU release の GPUI 検証 | 一部達成 | bound 近傍 payload（`MAX_WAVEFORM_BINS`/`MAX_METER_BINS`/`MAX_SPECTROGRAM_CELLS` ちょうど）・多チャンネル・viewport culling・gesture 中 projection・`update_view` spec 差し替え・f64 precision を `consumer_observation.rs` の seam test で検証。gap/NaN・bound 拒否・cancel/stale/out-of-order・teardown は既存 lane でカバー済み。実 GPU release は platform 制約で NotRun（#313） |
| driver-lab manifest の UI-DVR-03/07/08 GPUI | 達成（実測一致） | PR #432/#433/#444 で統合済み。`mise run test-ui-gpui` の実測 pass と `cases.json` の gpui `pass` が一致 |
| `LAB-SV-FADER-DOCK` / `LAB-AL-WAVEFORM-SEEK` の 2 scale factor × 2 theme × 状態遷移の GPUI 実行 | 未達（not_run のまま） | story の GPUI 実行 lab は owner = #426。`stories.json` の status は変更しない |
| 実 OS window・real GPU release（#313）・headless-ts（#294）・pixel golden（#314）・共通 Scenario 契約（#464） | 未達（NotRun・他 Issue 帰属） | platform/owner 制約により本 chain の範囲外 |

## スコープ（本 PR）

- `ruviform-ui-gpui`: 観測 seam（`gpui_driver/observe.rs`、`gpui-test` 境界）、mount 済み view の引き当て（`view_of`）、外部 crate test。
- 既存の内部フック（`settle_for_test` / `inbox_len_for_test` / `view_for_test`）は seam と同じ helper への wrapper に一本化し、lifecycle fixture を seam へ移行する。
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
