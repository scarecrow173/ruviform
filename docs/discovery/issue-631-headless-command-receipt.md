---
id: "DISC-Mx41kYpNvT8wQc6rLd9sJh"
type: "discovery"
status: "draft"
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
  related:
    - "../specs/issue-631-headless-command-receipt.md"
    - "../impl/ir/issue-631-headless-command-receipt.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 headless-command-receipt case

## Summary

Issue #631 AC1「Command→Applied/Rejected Receipt link」と AC4
「Command+Receipt evidence populate」の headless counterpart —
`HEADLESS_MATRIX` の Fake binding に `command-receipt` case を
追加し、headless lane の `receipts` を初めて非空化する slice の
探索記録。real lane 側の `real-command-receipt`（PR
`ruviform#748`・doc chain slug `issue-631-command-receipt`）が
先行着地済みで、本 slice はその headless 版。全ての driver-neutral
primitive（`dispatch_input`/`drain_intents`/`post_event`/`observe`/
`settle`）、`MintedIntent` 契約、`CommandReceipt::from_result`、
`ScenarioOutcome.receipts`→JSONL 配線は実装済み — 新規 API・新規
依存・feature 追加は不要（confirmed — 実装・検証済み）。

## Exploration Goal

`rf631.plugin-dock` の headless catalog に Command→Receipt 相関
case を 1 行追加し、Fake driver 上で gesture→intent mint→
`post_event` による applied/rejected `Event::Result` +
`Event::Notify` projection の往復、`CommandPhase` 遷移
（Applied→Observed / Rejected）の観測、decode 済み
`CommandReceipt` の `ScenarioOutcome.receipts` populate を
実走 evidence として記録する。

## Key Issues and Assumptions

- fixture: `sv_fader_dock_fixture`（`ruviform-ui-test` の
  `lab_story`）は command-minting fader `fader-ch1` を持つ
  既存 fixture — real lane と同じ fixture を Fake driver で
  再利用し、fixture 自体は変更しない。
- capability: Fake binding は `SemanticAction`/`MultiWindow` を
  claim 済み。新 case の `required` は `SemanticAction` のみ —
  preflight は既存の `require_capabilities` で自動判定される。
- 相関: `expect_receipt` + `CommandIntent::Commit` の gesture id
  で receipt-expecting commit を識別し、minted `command_id` を
  `Event::Result.id` に verbatim で使う。display text・配列位置・
  合成値での推測はしない。
- decode: `CommandReceipt::from_result` は view の
  `deliver_event` と同じ decoder — 記録する receipt は view が
  実際に適用した event の decode 結果と byte 一致する。
- receipts の生存: 途中 step が失敗しても、それまでに decode
  した receipt は `ScenarioOutcome.receipts` に残す（fail-closed
  でも evidence を失わない）。
- evidence honesty: `driver: Fake` + `environment_kind:
  deterministic-backend` のみを名乗る。real-window・pixel・
  OS-level・native plugin dock の主張は一切しない。

## Existing Pieces（変更不要と確認済み）

- `SemanticGuiTest`（`ruviform-ui-test/src/scenario.rs`）:
  `dispatch_input`/`settle`/`drain_intents`/`observe`/`post_event`
  の driver-neutral port。
- `parity_gesture_scenario`: Begin→Preview→Commit→settle→
  drain_intents→observe の共有 parity scenario。
- `FakeGuiTest`/`FakeNativeSlot`: semantic adapter — 実装済み。
- `sv_fader_dock_fixture`（`ruviform-ui-test/src/lab_story.rs`）。
- `CommandReceipt::from_result`（`ruviform-ui/src/behavior.rs`）。
- `ScenarioOutcome.receipts` → `ScenarioReport.receipts` →
  `jsonl.rs` case_line の `receipts` field 配線（catalog-flip
  slice で実装済み）。
- `tests/scenario_contract.rs` の
  `post_event_routes_receipt_and_projection_to_binding`:
  Fake adapter での同一 flow 先例。
- real lane 参照実装 `real_dock.rs::command_receipt_body`
  （`#[cfg(feature = "real-dock")]` — headless からは参照不可の
  ため scenario.rs に同型の body を実装）。

## Constraints

- 編集範囲: `crates/ruviform-plugin-dock-test/src/scenario.rs`、
  `tests/plugin_dock.rs`、`tests/console.rs`、`README.md`、
  `evidence/rf631/` の 4 JSONL + `verification.md`、本 doc chain。
- real lane 関連ファイル（`real_dock.rs`/`real_catalog.rs`/
  `plugin-dock-real-*.jsonl`）は対象外 — real lane は変更しない。
- `Cargo.toml`/`Cargo.lock`/`mise.toml`/`.github/**`/他 crate は
  変更しない。docs index（`docs/**/README.md`）は Worker が
  編集・再生成しない（Coordinator が統合時に再生成）。
- Issue #631 は tracker — `Refs #631` のみ、auto-close 禁止。
- interactive Windows session 不要 — headless row のため全検証が
  headless で完結する。

## Open Questions

なし — 全ての前提 API が実装済みで、real lane の先行 slice が
同一 flow の real 側実績を提供済み。
