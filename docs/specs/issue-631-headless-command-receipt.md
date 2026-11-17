---
id: "SPEC-Qa82xTzLmV5nWp7rYh3dGf"
type: "spec"
status: "approved"
title: "Issue #631 headless-command-receipt case"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-631-headless-command-receipt.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 headless-command-receipt case

## What Should Be Built

`HEADLESS_MATRIX` の Fake binding に `command-receipt` case を
1 行追加する（catalog 8→9 cell）。`FakeGuiTest` に
`sv_fader_dock_fixture().spec` を mount し、
`parity_gesture_scenario`（Begin→Preview→Commit）で
`fader-ch1` を駆動する。`drain_intents` が返す
receipt-expecting `Commit` intent の minted `command_id` を
`expect_receipt` + gesture id で相関し、`post_event` で
applied `Event::Result` + `Event::Notify` projection と
rejected `Event::result_err` を投入する。
`CommandReceipt::from_result` で decode した receipt を
`ScenarioOutcome.receipts` へ populate し、JSONL の
`receipts` field に非空で記録する — headless lane で初めて
`receipts` が非空になる（AC1 link + AC4 evidence の headless
側）。

real lane の `real-command-receipt`（PR `ruviform#748`）と同一の
semantic flow を Fake driver で再現する counterpart — real lane
の window D mount・close survivor 一般化は本 slice の対象外。

## Why Now

- Issue #725（`tests/plugin_dock.rs` 等の lease）は CLOSED
  （2026-11-17）で解放済み — headless 側の件数追従が可能に
  なった。
- real lane 側の receipt plumbing（`ScenarioOutcome.receipts`→
  JSONL、`CommandReceipt::from_result`、`MintedIntent` 契約）は
  全て着地済み — headless 側は catalog 行 + case body のみで
  完結する。
- Issue #631 tracker の AC1/AC4 を driver-neutral に両 lane で
  示す最後の未着地ピース。

## Acceptance Criteria

- `HEADLESS_MATRIX` に `command-receipt`（driver=`Fake`・
  required=`SemanticAction` のみ）が catalog 順で `view-mount`
  の次に存在する。
- case は `sv_fader_dock_fixture` の `fader-ch1` へ
  Begin→Preview→Commit gesture を駆動し、receipt-expecting
  `Commit` の minted `command_id` を verbatim で
  `Event::Result.id` に使う（位置・表示文字・合成値で推測
  しない）。
- applied 経路: `Event::result_ok`（`status:"applied"`）→
  `CommandPhase::Applied` → `Event::Notify` projection →
  `CommandPhase::Observed` を `settle`/`observe` で観測する。
- rejected 経路: `Event::result_err` → decode 済み receipt が
  `status:"rejected"` + detail を持ち、`CommandPhase::Rejected`
  を観測する。
- decode は配送した `Event` そのものに対して
  `CommandReceipt::from_result` を実行する — receipt を手組み
  しない。`command_id` が minted id と一致することを assert
  する。
- `ScenarioOutcome.receipts` に decode 済み receipt が入り、
  JSONL の `receipts` field に記録される。失敗経路でもそれまで
  に decode した receipt は残る。
- `observed.steps` に step 単位の証拠（gesture intent 数・
  `command_id`/`control_id`・phase 遷移）と
  `semantic_tree` の fader snapshot を記録する。
- environment は `driver: Fake`・`environment_kind:
  deterministic-backend` のみを名乗る — real-window・pixel・
  OS-level・native plugin dock の主張をしない。
- `tests/plugin_dock.rs`・`tests/console.rs`・`README.md` の
  件数記述を 8→9（console は 30→31）へ追従する。
- `evidence/rf631/` の 4 JSONL と `verification.md` を新実測で
  再生成する。

## Non-Goals

- real lane（`real_dock.rs`/`real_catalog.rs`/
  `plugin-dock-real-*.jsonl`）の一切の変更。
- 新規 API・新規依存・feature 追加（`Cargo.toml`/`Cargo.lock`/
  `mise.toml` は変更しない）。
- plugin-core/host/ui 側の変更（FakeGuiTest/CommandReceipt/
  Event 型は実装済み）。
- AC4 残 field（GPU 情報・semantic tree 全量・pixel-diff
  evidence 同梱・window/plugin hash 完全化）。
- DPI-change 駆動・window move、pixel Pass（`ruviform#532`
  gate）、crash/hang out-of-process 化、consumer 投影
  （`synvelis#114`/`synvelis#176`/`audio-lens#127` gate）。
- docs index の再生成（Coordinator が統合時に実施）。
