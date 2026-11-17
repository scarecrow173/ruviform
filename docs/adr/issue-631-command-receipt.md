---
id: "ADR-Km83rTzcPE9wLj6uVx7yHq"
type: "adr"
status: "accepted"
title: "issue-631-command-receipt fixture mount 形態"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added:
      - type: "file"
        path: "docs/adr/issue-631-command-receipt.md"
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
    - "docs/specs/issue-631-command-receipt.md"
    - "docs/designs/issue-631-command-receipt.md"
    - "docs/discovery/issue-631-command-receipt.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-command-receipt.md"
    - "docs/designs/issue-631-command-receipt.md"
  derived-by:
    - "docs/plans/issue-631-command-receipt.md"
    - "docs/impl/ir/issue-631-command-receipt.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# command-receipt fixture は第 4 window mount として追加する

## Context and Problem Statement

`real-command-receipt` は receipt-expecting intent を mint する
command-bound control を必要とするが、既存 session の
`plugin_dock_view_spec`（NativeSlot のみ）は command binding を
持たない。fixture をどこへ mount するか — (a) 既存 window A の
spec を `update_view` で一時差し替え、(b) 既存 window B/C の
spec を fader 化、(c) 第 4 window D として新規 mount — の 3 案が
あった。

## Decision Drivers

- 既存 case への非干渉: window A は sentinel（GPUI platform loop
  は最終 window close で終了）であり、B/C は
  `real-window-close`/`real-multiwindow-focus` の前提。
- view identity の保全: `update_view` は view instance を保つが
  spec 差し替えは同 window を使う全 case の前提を変え、復元責務
  が新規発生する。
- session コスト: window 追加は mount/paint コストのみで、
  catalog 順（command-receipt は close より先）で安全。
- honest evidence: fixture window が実 Window として観測対象に
  なること自体が real-lane の証拠価値。

## Considered Options

- (a) window A の spec 一時差し替え — mount 追加なしだが、
  sentinel window の spec を case 中に書き換える復元責務と、
  A を使う他 case への波及リスクがある。
- (b) window B/C の spec を fader 化 — mount 追加なしだが、
  close target・focus target の前提を壊す。
- (c) 第 4 window D（`ViewInstanceId` 634・
  `sv_fader_dock_fixture`・position (4350,80)）— 既存 window/
  view/spec に一切触れず、専用 fixture window として分離。

## Decision Outcome

Chosen option: "(c) 第 4 window D として新規 mount", because 既存
window/view/spec への非干渉が最も確実で、fixture 専用 window と
して責務が明確。session の window が 4 つになるコストは mount/
paint のみで、case 順で command-receipt は close より先に走る
ため close の破壊 target と衝突しない。

### Consequences

- `real-window-close` の終端 assert は「sentinel A のみ残存」
  ではなく「mount 時 window 集合から close target B/C を除く
  全 window が生存」へ一般化が必要 — window D も sentinel A と
  同列の非対象 survivor として検証する（実走で `[A]`→`[A,D]`
  の fail を観測し修正）。
- `RealDockContext` に `window_d`/`view_d` が増える — session
  構成の説明責任が 1 window 分増えるが、各 field の doc comment
  で用途を明記済み。
- 将来 fixture window が増える場合も、close の survivor assert
  は「非対象全員」で一般化済みのため追加変更なしで追従できる。

### Confirmation

- `real_catalog_matches_entries`・`blocked_reports_fail_closed`
  等の feature-gated 整合 test が green。
- `test-ui-plugin-dock-real` 実走で `real-command-receipt` pass
  + `real-window-close` pass（survivor 一般化後）を確認。

## Pros and Cons of the Options

### (a) window A の spec 一時差し替え

- Good: window 数が増えない。
- Bad: sentinel window の spec を case 中に書き換える復元責務、
  `update_view` の semantics への依存、他 case への波及リスク。

### (b) window B/C の spec を fader 化

- Good: window 数が増えない。
- Bad: close/focus case の前提を直接壊す — 却下。

### (c) 第 4 window D（採用）

- Good: 既存 window/view/spec に非干渉、責務分離が明確、
  survivor assert の一般化で将来の window 追加にも追従可能。
- Bad: session の window が 1 つ増える（mount/paint コスト）。

## More Information

- `crates/ruviform-plugin-dock-test/src/real_dock.rs` —
  `run_all_with_pixel_artifacts` の 4 mount 構成と
  `command_receipt_body`/`close_body`。
- `crates/ruviform-ui-test/src/lab_story.rs` —
  `sv_fader_dock_fixture`。
- `docs/designs/issue-631-command-receipt.md` — session 構成の
  設計正本。
