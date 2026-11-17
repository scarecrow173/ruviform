---
id: "TASK-Gh04pUabNC7uKg9tWx5zOf"
type: "task"
status: "done"
title: "Issue #631 T1 — real-command-receipt case"
created: "2026-11-17"
updated: "2026-11-17"
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
  implements:
    - "docs/plans/issue-631-command-receipt.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-command-receipt.md"
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
---

# Issue #631 T1 — real-command-receipt case

## Work

`rf631.plugin-dock-real` catalog に `real-command-receipt` case を
追加し、AC1 の Command→Receipt link と AC4 の receipts populate
を実走 evidence として記録する slice。

1. `real_catalog.rs`: `REAL_CATALOG` に `real-command-receipt`
   （driver=`RealOs`・required=`SemanticAction`）を追加。
2. `real_dock.rs` session 構成: window D（`ViewInstanceId` 634・
   `sv_fader_dock_fixture`・`WindowSpec` position (4350,80)）を
   第 4 mount として追加し、`RealDockContext` に `window_d`・
   `view_d` を追加。
3. `real_dock.rs` case 実装: `command_receipt_body` —
   `resolve_target`（window D/`fader-ch1`）→ Begin→Preview→
   Commit gesture → `drain_intents` で receipt-expecting commit
   intent を `command_id` で採取 → `post_event` で applied
   `Event::Result` → `CommandPhase::Applied` settle/observe →
   `Event::Notify` projection → `CommandPhase::Observed` →
   第 2 gesture の rejected result → `CommandPhase::Rejected`。
   `CommandReceipt::from_result` で decode した receipt を
   `ScenarioOutcome.receipts` へ populate。observed に step
   観測・semantic tree を記録。
4. `real_dock.rs` `real-window-close`: 終端 assert を「mount 時
   window 集合から close target B/C を除く全 window が生存」へ
   一般化（window D 追加で `[A]`→`[A,D]` — 実走 fail を観測して
   修正）。survivor 全員の close lifecycle 非観測を検証。
5. `tests/console.rs`: `--list` 29→30 行・real-dock 系 assertion
   21→22。`real_dock.rs` module doc「twenty-two cases」表更新。
6. 検証 lane 実行 → `test-ui-plugin-dock-real` 実走で
   `plugin-dock-real-scenario.jsonl` 再生成、
   `test-ui-plugin-dock-real-headless` で
   `plugin-dock-real-headless.jsonl` 再生成、
   `verification.md` へ slice 記録追記。
7. doc chain（discovery/spec/design/plan/task/test-spec/adr/ir/exp）
   を完成させる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-631-command-receipt.md`
- `docs/specs/issue-631-command-receipt.md`
- `docs/designs/issue-631-command-receipt.md`
- `docs/discovery/issue-631-command-receipt.md`
- `docs/test-specs/issue-631-command-receipt.md`

## Dependencies

- `SemanticGuiTest` の `dispatch_input`/`drain_intents`/
  `post_event`/`observe`/`settle`（`RealOsGuiTest` 実装済み）。
- `MintedIntent`（`command_id`/`expect_receipt`/`CommandIntent`）。
- `CommandReceipt::from_result`・`CommandPhase`・
  `ViewHost::apply_receipt`/`apply_projection`（変更なし）。
- `sv_fader_dock_fixture`（`ruviform-ui-test` 既存 fixture）。
- `ScenarioOutcome.receipts`→`ScenarioReport.receipts`→
  `jsonl.rs` 配線（catalog-flip slice で実装済み）。
- `settle_until`/`run_scenario`/`steps_outcome`/`step`/
  `finalize_case`/`component_snapshot_json` 既存 helper。

## Affected Files

- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/tests/console.rs`
- `evidence/rf631/plugin-dock-real-scenario.jsonl`（再生成）
- `evidence/rf631/plugin-dock-real-headless.jsonl`（再生成）
- `evidence/rf631/verification.md`（slice 記録追記）
- `docs/{discovery,specs,designs,plans,tasks,test-specs,adr,impl/ir,impl/exp}/issue-631-command-receipt.*`

## Done When

- [x] `real-command-receipt` が実 Window で gesture→intent→
  receipt/projection→phase 遷移を実観測して判定する。
- [x] 同 case の `receipts` が JSONL に非空で出る（applied +
  rejected の decode 済み `CommandReceipt`）。
- [x] `REAL_CATALOG`/`real_entries` の required が一致し、
  catalog 整合 test が green。
- [x] headless lane が全行 `NotRun`+`environment_blocked` を
  exit 0 で維持する（22 case 行）。
- [x] `real-window-close` が window D 追加後も正しい終端で判定
  する。
- [x] 非回帰 lane が green、`verification.md` に記録が追記される。

## Verification

- [x] `mise run fmt-check` — pass
- [x] `mise run check/lint/test ruviform-plugin-dock-test`
  （default/`real-dock`/`lab-gui`）— pass
- [x] `mise run test-ui-plugin-dock-console` — pass
- [x] `mise run test-ui-plugin-dock` — pass
- [x] `mise run test-ui-plugin-dock-real-headless` — pass（exit 0）
- [x] `mise run test-ui-plugin-dock-real` — 実走、
  `real-command-receipt` pass + receipts 2 件、
  `real-window-close` pass、`real-dcomp-pixel` 既知 Fail のみで
  exit 1 は設計どおり
- [x] `mise run test-ui-scenario-contract` — pass
- [x] `mise run check-ui-driver-boundary` — pass
- [ ] `mise exec -- prek run --all-files` — 実行予定
- [ ] `test-ui-plugin-dock-lab` — interactive operator 操作が
  必要（computer-use/UIA 不可のため window 起動→clean exit の
  み確認。catalog 投影は console `--list` + unit test で検証）

## Notes

- `real-dcomp-pixel` は `fail` のまま（#532 gate — 既知 DComp
  欠陥を正直に保持）。
- `real-window-ime`・editor verbs・crash/hang は `Unsupported`
  維持（#389/#630 領域）。
- HEADLESS_MATRIX 側 command-receipt 行は ruviform#725 lease
  解除後の後続 slice。
- docs index（`docs/**/README.md`）は Coordinator が統合時に
  再生成 — 本 slice では編集しない。
- PR 本文は `Refs #631` のみ（auto-close 禁止）。
