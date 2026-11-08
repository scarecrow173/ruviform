---
id: "IMPL-034UaLZbuMa8o4uov8RY26"
type: "impl"
status: "completed"
title: "Issue #436 実装記録 — GpuiObserver drawn の consumer 側検証（ruviform 内 consumer 型テスト）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-436-observer-drawn.md"
    - "docs/plans/issue-436-observer-drawn.md"
    - "docs/specs/issue-436-observer-drawn.md"
    - "https://github.com/scarecrow173/ruviform/issues/436"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-436-drawn-consumer-verification.md"
    modified:
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-436-observer-drawn.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #436 実装記録 — GpuiObserver drawn の consumer 側検証（ruviform 内 consumer 型テスト）

## Summary

Issue #436 の残受入「`new-kinds.json` の `drawnBars` / `drawnMarks` を consumer 側の test から観測して一致する」を、ruviform リポジトリ内で完結する consumer 型テストで検証した記録。

実装本体（`GpuiObserver::drawn` と driver-neutral DTO、Spectrogram / Label / Table への拡張）は PR #439（Issue #436 部分達成）と PR #444（Issue #442 部分達成）で main にマージ済みであり、本タスクではコード変更を行わず、現行 main における受入条件の充足を検証 lane の実行結果で裏付けた。

## Context

- 対象 Issue: https://github.com/scarecrow173/ruviform/issues/436
- 文書 chain: `docs/specs|designs|plans|tasks|test-specs/issue-436-observer-drawn.md`（T6「検証と PR」が本タスクに相当）
- 前提: #429 の `GpuiObserver`（observe / intents / dispatch_input / settle / inbox_len）と #421 の plan 関数（`meter_bar_plan` / `axis_mark_plan`）は main に存在する。

## Implementation

### Changes

コード変更なし。検証の裏付けとして本 Implementation Record と `docs/impl/ir/README.md` の index 行のみを追加した。

### Decisions During Implementation

- Issue 本文の受入条件は「consumer 側（AudioLens）の test」を要求するが、AudioLens での実観測は consumer repo の pin 更新を伴う別作業である（タスク注記・Issue #166 系の作業）。本タスクでは ruviform 内の consumer 型テスト `crates/ruviform-ui-gpui/tests/consumer_observation.rs` で代替検証する。同ファイルは外部 test crate として公開 `GpuiObserver` seam と `ruviform-ui` 契約型のみを使い、crate 内 hook や `gpui` 型を一切参照しない（ファイル冒頭 doc 明記）。これは AudioLens 型 consumer test が観測できる面をそのまま再現する。
- `crates/ruviform-ui-test` への GPUI 依存追加は行わない。同クレートは driverless build（UI-DVR-01）と MSRV 1.89 lane（`test-ui-test-msrv`）の対象であり、`ruviform-ui-gpui` への dev-dep 追加は GPUI stack を引き込み両者を壊す。consumer 型テストの置き場は `ruviform-ui-gpui/tests/` の外部 crate test が既存の境界規約に適合する。
- `cases.json` / `stories.json` への binding 追加は行わない。driver-lab manifest の case は driver 契約の semantic（mount/reconcile・gesture・theme 等）を対象としており、描画 plan parity を宣言する既存 case がない。新 case 追加は manifest の `requiredCases` 完全性 pin に波及するため、本タスクの範囲外と判断した（Coordinator 確認済み: 追加の binding 登録・コード変更は期待されない）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実行環境: Windows 11 x64 / Git Bash、mise 管理 toolchain（`mise.toml` の pin）。検証は `scarecrow173/rf436-observer-drawn` ブランチ（`origin/main` = `9b13ddba` と同一内容）で実施。

| タスク | 結果 |
| --- | --- |
| `mise run test-ui-consumer-observation` | pass（11 件。`observes_drawn_meter_and_axis_plans_through_seam` を含む） |
| `mise run test-ui-gpui` | pass（72 件 = unit 61 + consumer_observation 11） |
| `mise run check-ui-driver-boundary` | pass（dep-graph / signature / allowlist すべて pass） |
| `mise run check-ui-gpui` | pass |
| `mise run lint-ui-gpui` | pass（`-D warnings`） |
| `mise run fmt-check` | pass |
| `mise run check` | pass（workspace） |
| `mise run lint` | pass（workspace、`-D warnings`） |
| `mise run test` | pass（workspace、3088 件・失敗 0） |
| `mise run test-ui-parity` | pass（11 件） |
| `mise run test-ui-driver-contract` | pass（7 件） |
| `mise run check-ui-headless` | pass（`ui/core` tsc --noEmit） |
| `mise run test-ui-headless` | pass（`ui/core` node --test 67 件） |
| `mise run test-ui-component-view` | pass（17 件） |
| `mise run test-ui-native-slot` | pass（13 件） |
| `mise run test-ui-test-msrv`（rust 1.89.0） | pass（121 件） |

### consumer 側 drawnBars / drawnMarks 検証の根拠

- `consumer_observation.rs::observes_drawn_meter_and_axis_plans_through_seam` が `parity/new-kinds.json` の `expect.meter.drawnBars`（3 bar）と `expect.timeAxis.drawnMarks`（3 mark、viewport 外 tick 除外）を件数・順序・各値（許容誤差 `1e-3`）で比較する。
- 同 test は `DrawnPlan::Meter` / `DrawnPlan::TimeAxis` のみを bind し、`gpui` 型・crate 内 hook を使わない公開 seam 経由の観測である。
- `check-ui-driver-boundary`（`gui-boundary-guard`）が公開 signature の gpui 型非露出を検査する。

## Risks

- `drawn` の観測値は spec からの純粋計算であり、実 pixel 出力ではない（設計書の限界どおり）。実 OS window での描画確認は manual verification。
- AudioLens 側の実 consumer test は未実施（NotRun）。consumer repo の ruviform pin 更新後に audio-lens#166 系の作業で検証される。

## Follow-ups

- AudioLens 側で `GpuiObserver::drawn` を使った drawn parity test を実装し、pin 更新後に `new-kinds.json` の `drawnBars` / `drawnMarks` を実観測する（Issue #436 の残受入、consumer repo 側作業）。
