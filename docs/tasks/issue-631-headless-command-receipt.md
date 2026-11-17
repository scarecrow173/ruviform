---
id: "TASK-Km62sWdPfC8vNx4rTq9yHb"
type: "task"
status: "done"
title: "Issue #631 T1 — headless-command-receipt case"
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
  implements:
    - "docs/plans/issue-631-headless-command-receipt.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-headless-command-receipt.md"
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

# Issue #631 T1 — headless-command-receipt case

## Work

`HEADLESS_MATRIX` に `command-receipt` case を追加し、Fake
driver 上の Command→Receipt link と `receipts` populate を
headless lane の初の非空 receipts evidence として記録する
slice（real lane counterpart `real-command-receipt` の headless
版 — Issue #725 lease 解放後の後続 slice）。

1. `scenario.rs`: module doc「eight」→「nine」+ table row、
   `ruviform_ui::behavior`/`ipc`/`view` と
   `ruviform_ui_test`（`MintedIntent`/`ScenarioError`/
   `parity_gesture_scenario`/`sv_fader_dock_fixture`）の import
   追加、`command_receipt_extras`（claim
   `semantic-command-receipt`）、Fake binding に
   `command-receipt` entry（required=`SemanticAction` のみ）。
2. `scenario.rs` case 実装: `command_receipt_body` —
   `mount_view(sv_fader_dock_fixture().spec)` →
   `resolve_target(fader-ch1)` → gesture `"rc-apply"`/`"rc-reject"`
   → `commit_for_gesture` で receipt-expecting commit を
   `command_id` 相関 → `post_event` で applied
   `Event::result_ok` + `Event::Notify` projection / rejected
   `Event::result_err` → `settle`/`observe` で
   `Applied`→`Observed`/`Rejected` を観測 →
   `CommandReceipt::from_result` decode を `outcome.receipts`
   へ populate。`step`/`steps_outcome`/`component_snapshot_json`
   helper を局所実装。
3. `tests/plugin_dock.rs`: cells/projections/reports 8→9、
   command-receipt Pass + receipts 2 件（applied/rejected
   status）の assert。
4. `tests/console.rs`: `HEADLESS_CASES` 9 件化（catalog 順）、
   `--list` rows 8→9、`--include-real` rows 30→31、
   `--driver fake` 2 cell、`--scenario` 9 件。
5. `README.md`: case table row、「eight」→「nine」、
   「twenty-one」→「twenty-two」。
6. evidence: `test-ui-plugin-dock`・`test-ui-plugin-dock-
   console` lane を実走して 4 JSONL を再生成、
   `verification.md` に本 slice の記録を追記。
7. doc chain（discovery→spec→design→plan→task→test-spec→
   adr→impl/ir→impl/exp + exp.jsonl）を作成。
8. review-protocol レビュー → commit → push → PR（`Refs #631`）。

## Done

- `HEADLESS_MATRIX` が 9 cell（Fake 2 + probe 7）を宣言し、
  `command-receipt` が `Pass` + receipts 2 件で JSONL に記録
  される。
- 全 headless lane の検証コマンドが green、evidence が新実測で
  再生成済み。
- real lane・manifest・他 crate・docs index は無変更。
