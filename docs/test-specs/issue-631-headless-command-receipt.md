---
id: "TSPEC-Lp73tXeQgD9wMz5sUv8iNc"
type: "test-spec"
status: "approved"
title: "Issue #631 headless-command-receipt case のテスト仕様"
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
    - "../designs/issue-631-headless-command-receipt.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-headless-command-receipt.md"
    - "docs/designs/issue-631-headless-command-receipt.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 headless-command-receipt case のテスト仕様

## Feature: command-receipt case（HEADLESS_MATRIX・Fake binding）

`rf631.plugin-dock` scenario の `command-receipt` cell が、
Fake driver 上で Command→Receipt link を正直に実行し、
decode 済み `CommandReceipt` を report/evidence に残すことを
保証する。

## Rules

### Rule: catalog 宣言

- `HEADLESS_MATRIX.cells()` は 9 cell を返し、`command-receipt`
  は `view-mount` の直後・driver=`Fake`・required=
  `{semantic-action}`・scenario=`rf631.plugin-dock`。
- `HEADLESS_MATRIX.project()` の全 cell が `runnable`（Fake
  binding は `SemanticAction` を claim 済み）。

### Rule: receipt-expecting intent の相関

- gesture が mint する `MintedIntent` のうち
  `expect_receipt == true` かつ `CommandIntent::Commit` の
  `gesture_id` が一致するものだけを command として採用する。
- `command_id` は minted 値を verbatim で `Event::Result.id`
  に使う — 配列位置・display text・合成値から推測しない。
- decode した `CommandReceipt.command_id` が minted id と
  一致しない場合は `Fail`。

### Rule: applied→observed 経路

- `Event::result_ok(command_id, {"status":"applied",
  "revision":1})` を `post_event` → `settle` → `observe` で
  `phase == CommandPhase::Applied` を確認する。
- 続けて `Event::Notify { control_id, revision:2, value }` を
  `post_event` → `settle` → `observe` で
  `phase == CommandPhase::Observed` かつ `observed.is_some()`
  を確認する — revision guard を bypass しない（実 revision
  より新しい値を送る）。

### Rule: rejected 経路

- 第 2 gesture の commit に対し `Event::result_err(command_id2,
  ErrorBody)` を `post_event` → `settle` → `observe` で
  `phase == CommandPhase::Rejected` を確認する。
- decode 済み receipt は `status == ReceiptStatus::Rejected`
  で `detail`（error message）を持つ。

### Rule: receipts の記録

- `CommandReceipt::from_result` を配送した `Event` そのものに
  適用して decode する — receipt を手組みしない。
- `ScenarioOutcome.receipts` に applied + rejected の 2 件が
  残り、`ScenarioReport.receipts` → JSONL `receipts` field に
  記録される。
- 途中 step が失敗しても、それまでに decode した receipt は
  `outcome.receipts` に残る（fail-closed でも evidence を
  失わない）。

### Rule: environment honesty

- report の `driver` は `fake`、`environment_kind` は
  `deterministic-backend`、`environment.claim` は
  `semantic-command-receipt`。
- `real-window`・pixel・OS-level・native plugin dock の主張を
  一切含まない（`environment_kind != "real-window"` を全
  report で assert）。

### Rule: artifact 非依存

- `command-receipt` は Fake driver の semantic case で artifact
  に依存しない — artifact 未解決環境でも `Pass`。
  `missing_artifact_fails_closed_as_not_run` で
  `ScenarioVerdict::Pass` を assert する。

## Examples

### Example: applied + rejected の receipts populate

`fixture_run_reports_honest_verdicts` で
`verdict_of(&reports, "command-receipt")` が
`verdict == Pass`・`receipts.len() == 2`・
`ReceiptStatus::Applied` と `ReceiptStatus::Rejected` の両方を
含むことを assert する。

### Example: console catalog 投影

`plugin-dock-console --list` が 9 row（`command-receipt` を
含む catalog 順）を `type:"catalog"` で出し、verdict 語彙を
含まないことを `list_emits_headless_projection_without_verdicts`
が保証する。`--include-real` は 31 row（9 headless + 22 real）。

### Example: selector 次元

`--run --driver fake` は `view-mount` + `command-receipt` の
2 cell を catalog 順で実行する
（`run_driver_and_scenario_selectors`）。
