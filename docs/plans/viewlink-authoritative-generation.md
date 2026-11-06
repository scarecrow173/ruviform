---
id: "PLAN-kreAOLAGHhliWIVO5VQM6G"
type: "plan"
status: "approved"
title: "ViewLink authoritative generation 実装計画"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "docs/superpowers/plans/2026-11-07-viewlink-authoritative-generation.md"
    - "docs/specs/viewlink-authoritative-generation.md"
    - "docs/designs/viewlink-authoritative-generation.md"
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
    - "docs/specs/viewlink-authoritative-generation.md"
    - "docs/designs/viewlink-authoritative-generation.md"
  implemented-by:
    - "docs/tasks/viewlink-authoritative-generation-t1-sdk-epoch.md"
  depends-on:
    - "docs/designs/viewlink-authoritative-generation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/viewlink-authoritative-generation.md"
    - "docs/designs/viewlink-authoritative-generation.md"
    - "docs/superpowers/plans/2026-11-07-viewlink-authoritative-generation.md"
  derived-by:
    - "docs/tasks/viewlink-authoritative-generation-t1-sdk-epoch.md"
  verifies: []
  verified-by:
    - "docs/test-specs/viewlink-authoritative-generation-tests.md"
  references:
    - "https://github.com/scarecrow173/synvelis/pull/171"
  defers: []
  deferred-by: []
---

# ViewLink authoritative generation 実装計画

正本の実行計画は `docs/superpowers/plans/2026-11-07-viewlink-authoritative-generation.md`。本書はcanonical doc graph上の計画ノードとして、タスク分解と検証入口を記す。

## 手順（Task 1: Ruviform SDK epoch採用 — 本タスクのみ対象）

1. `ui/binding/test/link.test.ts` に恒久テストを先に追加し、`mise run test-ui-binding` で期待するREDを確認する。
2. `ui/binding/src/link.ts` の `ViewLink.attach(windowId, generation?)` を実装（design参照: 検証→冪等判定→必要時detach→採用/既存increment→伝搬）。
3. `mise run check-ui-binding` / `test-ui-binding` / `build-ui-binding` / `check-ui-guitest` / `test-ui-guitest` を実行。
4. 最新main取込・整合確認・論理単位commit → 独立review-protocol → push・日本語PR → 候補SHAのmanual Full CI。

Task 2（Synvelis consumer取り込み）・Task 3（coordinator統合）は別dispatchの後続作業。

## Verification

- [ ] link.test.ts: generation=7採用（既存+後発binding・wire ctx・subscription ctx・query generation）、detach/reattach generation=11再採用
- [ ] 同window同epoch冪等（再購読なし）、同window異epochで旧subscription/gesture/query失効+再購読
- [ ] 無効epoch（負数・非整数・NaN・非safe integer）で状態不変
- [ ] 省略時local increment維持、late-created bindingがauthoritative epoch採用
- [ ] `mise run` の上記gateがgreen
