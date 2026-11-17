---
id: "DISC-Kd82mQxVnR4tYw6pLc3sJa"
type: "discovery"
status: "draft"
title: "Issue #631 real-command-receipt case"
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
  implements: []
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
---

# Issue #631 real-command-receipt case

## Summary

Issue #631 AC1 残 link「OS Input Injection→Command→Applied/Rejected
Receipt→同一 Case ID 関連付け」と AC4「Command+Receipt evidence
populate」を担う `real-command-receipt` case を real lane に追加する
slice の探索記録。全 22 real row のうち `receipts` が非空になるのは
本 case のみ — 既存 driver-neutral 契約
（`SemanticGuiTest::dispatch_input`/`drain_intents`/`post_event`/
`observe`/`settle`）と `CommandReceipt::from_result` decoder、
`ScenarioOutcome.receipts`→`jsonl.rs` の配線（catalog-flip slice で
実装済み）をそのまま使う。結論: 新規 API/依存なしで実装可能
（confirmed — 実装・実走済み）。

## Exploration Goal

`rf631.plugin-dock-real` catalog に Command→Receipt 相関 case を
1 行追加し、実 Window 上の gesture が mint した receipt-expecting
intent の `command_id` に対し、driver-neutral `post_event` 経由で
実 `Event::Result` を投入して `CommandPhase` の
Applied→Observed / Rejected 遷移を観測、decode 済み
`CommandReceipt` を report に populate する。

## Key Issues and Assumptions

- 既存 `plugin_dock_view_spec`（NativeSlot のみ）は command binding
  を持たず receipt-expecting intent を mint しない — gesture 対象は
  command-bound fixture が必要（confirmed: `real_dock.rs` の mount
  構成と `lab_story::sv_fader_dock_fixture` の spec を精読）。
- `sv_fader_dock_fixture`（`ruviform-ui-test`）は fader control を
  持ち、Begin→Preview→Commit gesture が `MintedIntent`（
  `command_id`・`expect_receipt`・decode 済み `CommandIntent`）を
  mint する — `tests/scenario_contract.rs` の
  `post_event_routes_receipt_and_projection_to_binding` が同型の
  先例（confirmed）。
- `RealOsGuiTest` は `SemanticGuiTest` を実装済みで
  `drain_intents`/`post_event`/`observe`/`settle`/`dispatch_input`
  が公開面 — 新規 API 不要（confirmed: `real_window.rs` の impl）。
- `Event::Result` は intent id で相関し、`CommandReceipt::
  from_result` が `command_id`/`status`/`revision`/`detail` を
  decode する — receipt は view へ配送したものと同一 event から
  収集する（confirmed: `behavior.rs`・`scenario_contract.rs`）。
- `apply_projection`（`Event::Notify` の `control_id`/`revision`/
  `value`）は `Applied` とは別経路で `Observed` へ settle する —
  AC1 の「Applied/Rejected」と AC4 の独立 state 照合を分離できる
  （confirmed: `ViewHost::apply_receipt`/`apply_projection`）。
- window 追加は sentinel 契約に抵触しない — window A は生存必須、
  B/C は close case で破壊される close target。fader fixture は
  専用 window D（`ViewInstanceId` 634）として第 4 mount にするのが
  既存 window の spec を壊さない最小構成（confirmed: mount 順・
  `windows()` 契約・case 実行順で command-receipt は close より先）。
- `real-window-close` の終端 assert は「sentinel A のみ残存」を
  仮定していた — window D 追加で `[A]` ではなく `[A, D]` が正しい
  終端。close case を「非対象 window が全て生存」へ一般化する必要
  がある（confirmed: 初回実走で `real-window-close` fail を観測・
  原因特定済み）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 既存 window A の spec を一時差し替え | window 追加なし | `update_view` は identity を保つが spec 差し替えは他 case へ波及リスク・復元責務が新規発生 | no |
| window B/C の spec を fader 化 | mount 追加なし | close case の target を破壊・multiwindow-focus の前提を壊す | no |
| 第 4 window（D）に fader fixture を mount | 既存 window/view に非干渉・case 順で安全・sentinel 契約維持 | session の window が 1 つ増える（paint/mount コストのみ） | yes |
| HEADLESS_MATRIX 側へ同 case を追加 | headless でも receipts 検証可 | `tests/plugin_dock.rs`・`Cargo.toml` は ruviform#725 lease 占有中・本 slice 範囲外 | no（後続 slice） |
| receipt を合成値でなく `from_result` decode | 実 event と receipt の同一性を証明 | なし | yes |

## Tentative Conclusions and Hypotheses

- confirmed: `real-command-receipt` は window D の fader fixture で
  gesture を駆動し、receipt-expecting commit intent を
  `drain_intents` から採取、`post_event` で applied result →
  `CommandPhase::Applied` → `Event::Notify` projection →
  `CommandPhase::Observed`、別 gesture の rejected result →
  `CommandPhase::Rejected` を観測する設計で実装できる。
- confirmed: `ScenarioOutcome.receipts` に decode 済み
  `CommandReceipt` を入れれば `jsonl.rs` 既存配線で JSONL の
  `receipts` field に非空で出る（AC4）。
- confirmed: `required` は `SemanticAction` が正直な宣言
  （gesture 駆動 + semantic observe が前提）。
- confirmed: close case の終端 assert は「mount 時の window 集合
  から close target B/C を除いた全 window が生存」へ一般化する
  — sentinel A だけでなく fixture window も非対象として検証する
  方が invariant として正しい。

## Open Questions

- なし — gesture 駆動・receipt 相関・phase 遷移・evidence 配線・
  window 構成は全て既存契約の範囲で確定。残る不確実性は実走時の
  OS/GPUI 挙動のみで honest Fail/NotRun で記録する設計。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-631-command-receipt.md`
  — case の verdict contract・window 構成・evidence 要件を確定する。
- [ ] adr-doc needed: なし — 既存 `SemanticGuiTest`/`ViewHost`/
  `RealOsGuiTest` 契約の範囲内で、新規の architecture decision は
  発生しない（mount 形態は design で確定する実装詳細）。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- `docs/discovery/issue-631-real-window-catalog-flip.md` — 前 slice
  の探索記録（receipts/case_seed 配線の由来）。
- `docs/specs/issue-631-real-window-catalog-flip.md` 他
  `issue-631-*` 系列 — 前 slice の spec/design/plan/task/test-spec。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` — real lane
  session 構成と case 実装の追加先。
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs` —
  `REAL_CATALOG` 22 行目の宣言先。
- `crates/ruviform-ui-test/src/scenario/plugin_dock.rs`・
  `lab_story.rs` — `sv_fader_dock_fixture` の定義（変更なし・
  既存 fixture 再利用）。
- `crates/ruviform-ui-test/tests/scenario_contract.rs` —
  `post_event_routes_receipt_and_projection_to_binding` の先例。
- `crates/ruviform-ui/src/behavior.rs` — `CommandReceipt::
  from_result`・`CommandPhase`・binding 契約。
- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs` —
  `RealOsGuiTest` の `SemanticGuiTest` 実装。
- `POLICIES.md` — GUI-L2・Pass/Fail/NotRun/Unsupported・fail-closed
  規範。

## Evidence

### 2026-11-17 — real-command-receipt の実装と実走

- `REAL_CATALOG` に `real-command-receipt`（required=
  `SemanticAction`）を追加し、`real_entries`/`real_dock.rs` に
  実行 case を協調追加（catalog 整合 test が両者を検査）。
- session に window D（`ViewInstanceId` 634・title
  `ruviform fader-dock D`・position (4350,80)）を第 4 mount と
  して追加し、`sv_fader_dock_fixture` を host。
- case は Begin→Preview→Commit gesture で receipt-expecting
  commit intent（`command_id` 記録）を mint し、`post_event` で
  `status:"applied"`+revision の `Event::Result` を投入 →
  `CommandPhase::Applied` を settle/observe、`Event::Notify`
  projection（`control_id`/`revision`/`value`）で `Observed` へ。
  第 2 gesture の rejected result で `Rejected` を観測。
  `CommandReceipt::from_result` で decode した 2 receipt を
  `ScenarioOutcome.receipts` へ populate。
- `real-window-close` の終端 assert を「非対象 window 全生存」へ
  一般化（window D 追加による `[A]`→`[A,D]` 修正 — 初回実走の
  fail を観測して対応）。
- 実走結果は `evidence/rf631/plugin-dock-real-scenario.jsonl`・
  `plugin-dock-real-headless.jsonl`・`verification.md` を参照。
