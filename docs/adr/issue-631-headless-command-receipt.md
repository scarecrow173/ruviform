---
id: "ADR-Ns84uYadQF1xOk7tWx9jMe"
type: "adr"
status: "accepted"
title: "issue-631-headless-command-receipt fixture・body 配置"
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
    - "../adr/issue-631-command-receipt.md"
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

# issue-631-headless-command-receipt fixture・body 配置

## Status

accepted（2026-11-18）

## Context

headless `command-receipt` case は receipt-expecting intent を
mint する command-bound fixture が必要。real lane は第 4
window D に `sv_fader_dock_fixture` を mount する形態を採用
した（`docs/adr/issue-631-command-receipt.md`）。headless lane
では `FakeGuiTest` が case ごとに fresh 生成され、
`view-mount` が `plugin_dock_view_spec`（`NativeSlot` のみ・
command binding なし）を mount している。

## Decision

1. **fixture は既存 `sv_fader_dock_fixture` をそのまま使う**
   — `fader-ch1` の command binding・`expect_receipt` mint・
   `control_id` decode は全て fixture 側に実装済み。fixture・
   `plugin_dock_view_spec` は変更しない。
2. **body は `scenario.rs` に局所実装する**
   （`command_receipt_body` + `step`/`steps_outcome`/
   `component_snapshot_json`/`commit_for_gesture`/
   `commit_control_id`）。real lane の同名 helper は
   `#[cfg(feature = "real-dock")]` 配下で headless から参照
   不可のため、責務の同じ最小 helper を複製する。共有化のため
   に real_dock.rs を編集することは本 slice の範囲外。
3. **case は `view-mount` の直後に catalog 配置** — Fake
   binding の `cases` 配列で `view-mount` → `command-receipt`
   の順。driver・scenario・required の宣言は既存 entry と同型。

## Consequences

- `plugin_dock_view_spec`・`sv_fader_dock_fixture`・
  driver-neutral API・JSONL 配線は無変更 — 変更面は
  `scenario.rs` 1 file + 件数追従 + evidence/docs のみ。
- helper の lane 間複製（`step`/`steps_outcome`/
  `component_snapshot_json`/`commit_for_gesture`/
  `commit_control_id`）は cfg 境界由来の許容コスト — 将来
  cfg 非依存 module への集約は別 slice の検討事項。
- `command-receipt` は artifact 非依存の deterministic case —
  artifact 未解決環境でも `Pass` し、`receipts` は常に 2 件
  （applied + rejected）を記録する。
