---
id: "TASK-U2Bnd3KffOanQDe08MNWxo"
type: "task"
status: "done"
title: "Task 1 Ruviform SDK epoch採用 — ViewLink.attach(windowId, generation?)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "docs/plans/viewlink-authoritative-generation.md"
    - "docs/superpowers/plans/2026-11-07-viewlink-authoritative-generation.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/binding/src/link.ts"
      - type: "file"
        path: "ui/binding/test/link.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/viewlink-authoritative-generation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/viewlink-authoritative-generation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/viewlink-authoritative-generation-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/viewlink-authoritative-generation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/synvelis/pull/171"
  defers: []
  deferred-by: []
---

# Task 1: Ruviform SDK epoch採用

## Work

`ViewLink.attach(windowId: string, generation?: number): void` にauthoritative generation seamを追加。指定値は非負safe integerのみ受理し（無効値は `RangeError`・状態不変）、指定時は `generation_` をその値へ、省略時は既存incrementを維持。同window同値は冪等no-op、同window異epochは `unmount` detach→再attach、異windowは `dock-transfer` detach→指定epoch採用。CommandBinding/QueryBindingの `attach(epoch?)` とsubscription contextは既存経路で伝搬（#379）。Issue #377/#378は混ぜない。

## Classification

- `normal-plan-task`

## Acceptance

- [x] RED確認後の恒久テストgreen（test-specの全ケース）— RED 8 fail → GREEN 87/0
- [x] `mise run check-ui-binding` / `test-ui-binding` / `build-ui-binding` green
- [x] `mise run check-ui-guitest` / `test-ui-guitest` green（oracle変更なしで成立を確認 — 135/0）
- [x] 独立review-protocol通過（Reviewer A/B → Validator → Judge: A1 P1 blocking→修正→ACCEPT WITH NON-BLOCKING NOTES）、自branch push + 日本語PR + 候補SHA manual Full CI結果報告
