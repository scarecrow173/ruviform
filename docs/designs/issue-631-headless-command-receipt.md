---
id: "DESIGN-Rt91uVbKxW4mJq8sZc6fDh"
type: "design"
status: "approved"
title: "Issue #631 headless-command-receipt case 設計"
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
    - "docs/specs/issue-631-headless-command-receipt.md"
    - "docs/discovery/issue-631-headless-command-receipt.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 headless-command-receipt case 設計

## Context

`HEADLESS_MATRIX`（`crates/ruviform-plugin-dock-test/src/
scenario.rs`）は driver 列ごとの `DriverBinding` に case を
並べる catalog。Fake binding は `view-mount` 1 件のみで、
`SemanticAction`/`MultiWindow` を claim している。headless lane
の `receipts` は catalog-flip slice で JSONL 配線済みだが、
receipt を生成する case が存在せず常に空だった。

Command→Receipt 契約（driver-neutral、変更なし）:

- gesture/semantic input → view が `MintedIntent`（`command_id`・
  `expect_receipt`・decode 済み `CommandIntent`）を mint →
  `drain_intents` で採取。
- `post_event(target, Event)` で `Event::Result`（intent id
  相関）と `Event::Notify`（projection:
  `control_id`/`revision`/`value`）を view へ配送。
- view の `deliver_event` が `command_id` で binding を routing
  し `CommandPhase` を遷移（`Applied`/`Observed`/`Rejected`）。
- `ScenarioOutcome.receipts` → `ScenarioReport.receipts` →
  `jsonl.rs` の case line `receipts` field。

real lane の `command_receipt_body`（`real_dock.rs`）が同一 flow
の参照実装だが `#[cfg(feature = "real-dock")]` 配下のため
headless module からは参照できない — 同型の body を
`scenario.rs` に実装する。

## Design

### Catalog 行

Fake binding の `cases` に `view-mount` の直後へ追加:

- `case: "command-receipt"`
- `scenario: PLUGIN_DOCK_SCENARIO`
- `required: &[ScenarioCapability::SemanticAction]` のみ
- `spec_extras: command_receipt_extras`（`claim_extras!` で
  `"semantic-command-receipt"` を宣言 — finalize が report
  environment を実測値で上書きする既存パターンと同じ）
- `run: command_receipt_case`

### Case 構成

`command_receipt_case` は `view_mount_case` と同じ外形:
`FakeGuiTest::new()` → `run_scenario(&mut gui, spec, body)` →
`finalize(report, env, DeterministicBackend, None, {"claim":
"semantic-command-receipt"})`。

body は `command_receipt_body(gui) -> ScenarioOutcome` で、
real lane と同じ step 記録パターン（`step`/`steps_outcome`
helper を scenario.rs に局所実装 — real_dock の private helper
は cfg で参照不可のため同型を複製。責務は「step 証拠の記録」
で lane 固有の知識を持たない）:

1. `open_window` + `mount_view(sv_fader_dock_fixture().spec)`
   （`ViewInstanceId` 1 — view-mount case と同じ採番だが case
   ごとに新しい FakeGuiTest のため衝突しない）。
2. `resolve_target` — `TargetQuery::new(window, view)
   .component(ComponentKey("fader-ch1"))`。
3. gesture 1 `"rc-apply"`（value 0.7）→ `drain_intents` →
   `commit_for_gesture` で `expect_receipt` +
   `CommandIntent::Commit{gesture_id:"rc-apply"}` を識別 →
   minted `command_id`/`control_id` を step 証拠に記録。
4. `applied_event = Event::result_ok(command_id,
   {"status":"applied","revision":1})` →
   `CommandReceipt::from_result` で decode（`command_id` 一致を
   assert）→ `receipts` に push → `post_event` → `settle` →
   `observe` で `phase == Applied` を assert。
5. `Event::Notify { control_id, revision:2, value:{normalized:
   0.7} }` → `post_event` → `settle` → `observe` で
   `phase == Observed && observed.is_some()` を assert
   （revision guard を bypass しない — 実 revision+1 を送る）。
6. gesture 2 `"rc-reject"`（value 0.4）→ 同じ相関 →
   `rejected_event = Event::result_err(command_id2,
   ErrorBody::new("command_rejected", "consumer refused the
   commit"))` → decode → `receipts` に push → `post_event` →
   `settle` → `observe` で `phase == Rejected` を assert。
7. `observed` に `steps` + `window`/`view`/`component`/
   `semantic_tree`（最終 snapshot の `component_snapshot_json`）。

失敗時も `receipts` は `outcome.receipts` に残る
（`steps_outcome` の Fail 経路でも代入は body 終了後に行う）。

### Evidence honesty

- `driver: Fake`・`capabilities: {semantic-action, multi-window}`
  は report が自動記録。
- `environment_kind: deterministic-backend`、`claim:
  semantic-command-receipt`、`scale_factor: null`。
- `evidence` tag は `fake:command-receipt:fader-ch1` — real-window
  系の tag を名乗らない。

## Alternatives Considered

- **`plugin_dock_view_spec` への command binding 追加**: 既存
  fixture は `NativeSlot` のみで command binding を持たず、
  変更すると `view-mount` 等の既存 case の前提を壊す — 不採用。
- **case 内で `dispatch_input` を直接呼ぶ**: `parity_gesture_
  scenario` が Begin→Preview→Commit→settle→drain→observe を
  共有化済みで、real lane と同じ helper を使う方が parity が
  保たれる — 不採用。
- **receipt を手組みして post する**: `CommandReceipt::from_result`
  で decode したものを記録する契約（配送 event と byte 一致）
  に反する — 不採用。

## Risks

- Fake adapter の intent mint が `expect_receipt` を立てない
  可能性 — `scenario_contract.rs` の先例と real lane の実績で
  否定済み。万が一立たなければ case は `Fail`（honest）で、
  `commit_for_gesture` が `None` を返す detail が残る。
- `component_snapshot_json`/`step`/`steps_outcome` の
  scenario.rs への複製は real_dock との重複 — cfg 境界で
  共有不可のため許容（各 ~20-40 行、責務は lane 非依存）。
